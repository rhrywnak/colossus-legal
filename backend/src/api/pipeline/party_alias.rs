//! Alias-aware party resolution: the index, the normalization, and the two
//! guards that decide when an alias is allowed to bind a mention to a node.
//!
//! ## What this is for
//!
//! `merge_parties --apply` collapses duplicate Person/Organization nodes and
//! records every merged spelling as an alias on the survivor — Tighe now carries
//! "Karen A. Tighe", "Judge Tighe", "Circuit Judge" and nine more. Until this
//! module existed, ingest matched a new mention against the canonical `name`
//! only, so the next document re-spawned `person-tighe`, `person-jeff`,
//! `person-judy` and the People page fragmented again.
//!
//! **Measured, and it is not hypothetical.** This morning's merge deleted
//! `person-mr-dalek` and `person-judy` at 12:30 UTC; the Morris re-ingest at
//! 12:47 UTC re-created both, because Morris says "Mr. Dalek" and "Judy" and
//! nothing consulted the aliases the merge had just written. The merge held for
//! seventeen minutes.
//!
//! ## The two guards, and why a miss is better than a wrong hit
//!
//! Domain note: attaching a sworn statement to the WRONG person is far worse
//! than leaving a duplicate node for a human to merge. A duplicate is visible on
//! the People page and fixable with `merge_parties`; a mis-attribution puts one
//! person's words in another person's mouth, and P3 pairs a witness against
//! "their own sworn words". So both guards fail toward "no match":
//!
//! * **Ambiguity** — a normalized string claimed by two or more nodes never
//!   matches. Measured on the live graph: `awad` is an alias of BOTH Emil and
//!   Marie Awad, `court` of both Judge Tighe and Judge Murphy, `j` of both
//!   appellate judges. Any of those three could have silently mis-attributed
//!   testimony.
//! * **Generic-role stoplist** — role words ("Plaintiff", "the Court",
//!   "decedent") are aliases for DISPLAY and SEARCH. They stay on the node; they
//!   simply never drive a match, because the same role belongs to a different
//!   person in the next case and to nobody in particular in this one.
//!
//! ## No fuzzy matching, ever
//!
//! Nothing here computes an edit distance. "Handley" does not become "Hanley".
//! Anything not equal after normalization is a new node, which Roman merges by
//! hand — that is the design, not a limitation of it (ruling 2026-07-20: a false
//! merge is worse than a duplicate).

use std::collections::{BTreeMap, BTreeSet};

use unicode_normalization::UnicodeNormalization;

/// Corporate suffixes stripped before comparison.
///
/// Deliberately the same list, in the same order, that
/// `colossus_extract::resolver::normalize_name` uses. Two normalizations that
/// disagree about "Penzien & McBride, PLLC" would resolve a mention one way at
/// the name stage and another at the alias stage — the kind of split-brain that
/// is invisible until it mis-files a statement.
// CONST: a structural property of English corporate naming, shared verbatim with
// the upstream normalizer. Not deployment config — a deployment that stripped a
// different suffix set would disagree with `colossus-extract` about identity.
const CORPORATE_SUFFIXES: &[&str] = &[
    ", inc.", ", inc", " inc.", " inc", ", llc", " llc", ", pllc", " pllc", ", ltd.", " ltd.",
    ", ltd", " ltd", ", p.c.", " p.c.",
];

/// Honorifics and professional titles stripped from the FRONT of a name.
///
/// Domain note: these are the forms a legal document actually alternates between
/// — "Judge Tighe" / "Tighe", "Dr. Armaly" / "Armaly", "Mr. Dalek" / "Dalek" —
/// and every one of those pairs is a live alias pair in this graph. Stripped
/// only from the front, and only as whole words, so "Attorney General" keeps its
/// second word and a surname that happens to be "Judge" survives as itself.
// CONST: English honorifics. Identical in every case and every deployment; an
// env var that changed them would change who resolves to whom, silently.
const LEADING_HONORIFICS: &[&str] = &[
    "mr",
    "mrs",
    "ms",
    "miss",
    "dr",
    "doctor",
    "judge",
    "justice",
    "hon",
    "honorable",
    "attorney",
    "atty",
    "prof",
    "professor",
    "rev",
    "messrs",
];

/// Role words that are aliases for DISPLAY only and never resolve a mention.
///
/// ## What may and may not live here
///
/// Every entry must be a phrase whose meaning is a ROLE in English legal prose
/// and nothing else — true in this case, in the next one, and in colossus-ai.
/// "The plaintiff is not a name" is a property of the language; it does not vary
/// by deployment, which is why this is a `const` and not YAML.
///
/// **Six entries were removed on review, and the reason is the rule itself.**
/// The first draft included "social worker from Heartland", "regional manager
/// for NCB", "regional manager for National City Bank", "authorized examiner by
/// the state of Michigan", "one of three heirs to the estate" and "the
/// attorney/guardian ad litem on the prior case". Each is a live alias in THIS
/// graph and each names an organization, a jurisdiction, a prior proceeding or a
/// fact about this estate — case data compiled into shared code, which Standing
/// Rule 2 forbids in as many words ("person aliases — all in config, never
/// compiled in"). They are unambiguous single-node aliases, so leaving them
/// resolvable costs nothing and mis-attributes nobody. If a case ever needs its
/// own stoplist, that is a YAML surface and a ruling, not a longer `const`.
///
/// Note what is also deliberately ABSENT: "Defendant CFS", "Defendant Phillips",
/// "cousin Mike", "her son James". Those carry a name inside the role and are
/// unambiguous, so they resolve — which is exactly what we want them to do.
// CONST: role vocabulary of English legal prose, deliberately free of any name,
// organization, jurisdiction or fact from this case — see the doc comment above
// for the six entries removed to keep it that way (Standing Rule 2).
pub const GENERIC_ROLE_STOPLIST: &[&str] = &[
    // Party-role words.
    "Plaintiff",
    "Defendant",
    "Defendants",
    "Appellant",
    "Appellee",
    "Interested Person",
    "Interested Person-Appellant",
    "interested person and appellant",
    // The bench. "the Court" is on Tighe AND Murphy today — ambiguous as well as
    // generic, and blocked twice over.
    "the Court",
    "THE COURT",
    "Probate Judge",
    "Circuit Judge",
    "Judge of Family Division",
    "Family Division",
    "C.J.",
    "J.",
    // Counsel and court staff.
    "counsel",
    "staff",
    "Attorney for Plaintiff",
    "Affiant",
    "Personal Representative",
    "the personal representative",
    "its public guardian",
    // First-person and kinship — these belong to whoever is speaking, which is a
    // different person in every document.
    "I",
    "myself",
    "my father",
    "dad",
    "their father",
    "my boss",
    "my nephew",
    "the heirs' cousin",
    "decedent",
    "the decedent",
    "primary caregiver",
    // Occupations and credentials, not names.
    "that firm",
    "the auctioneer",
    "potential auctioneer",
    "tax preparer",
    "LLMSW",
    "PLLC",
];

/// Normalize a name or alias into the key both stages compare on.
///
/// Steps, in order: NFC · lowercase · trim · strip one corporate suffix · strip
/// punctuation · collapse whitespace · strip a leading "the " · strip leading
/// honorifics · drop single-letter tokens (middle initials).
///
/// ## Rust Learning: `.nfc()` is an iterator adapter from a trait
///
/// `UnicodeNormalization` is an extension trait on `&str`; importing it is what
/// puts `.nfc()` in scope. It yields `char`s lazily, so nothing is allocated
/// until `collect()`. Same reasoning as `evidence_key::normalize` — a name whose
/// accents arrive decomposed one run and composed the next must not become two
/// people.
///
/// ## The empty-key guard
///
/// Dropping single letters would turn "J." into "" and "I" into "", and an empty
/// key would collide with every other empty key — welding two appellate judges
/// together, which is the precise failure this module exists to prevent. So if
/// the middle-initial pass empties the string, the pre-pass words are kept
/// instead. A key that is STILL empty is refused by the index rather than stored.
pub fn normalize_party_key(text: &str) -> String {
    let composed: String = text.nfc().collect();
    let mut s = composed.to_lowercase().trim().to_string();

    for suffix in CORPORATE_SUFFIXES {
        if s.ends_with(suffix) {
            s.truncate(s.len() - suffix.len());
            break; // only ever strip one, exactly like the upstream normalizer
        }
    }

    // Punctuation to spaces rather than to nothing: "Camille Hanley--Hanley"
    // must not become one run-together word.
    s = s
        .chars()
        .map(|c| if is_name_punctuation(c) { ' ' } else { c })
        .collect();
    s = s.split_whitespace().collect::<Vec<_>>().join(" ");

    if let Some(rest) = s.strip_prefix("the ") {
        s = rest.to_string();
    }

    let all_words: Vec<&str> = s.split_whitespace().collect();

    // Strip leading honorifics — but if that would leave NOTHING, the honorific
    // was the whole name ("Judge", "Dr."). Keep it: a party actually recorded as
    // "Judge" is a bad extraction, and it must stay distinguishable from a party
    // recorded as "Doctor" rather than both collapsing to the empty key.
    let mut words: &[&str] = &all_words;
    while let Some(first) = words.first() {
        if LEADING_HONORIFICS.contains(first) && words.len() > 1 {
            words = &words[1..];
        } else {
            break;
        }
    }

    // Drop middle initials, with the same guard for the same reason: "J." and
    // "I" must keep a key of their own.
    let without_initials: Vec<&str> = words
        .iter()
        .copied()
        .filter(|w| !(w.chars().count() == 1 && w.chars().all(|c| c.is_alphabetic())))
        .collect();

    if without_initials.is_empty() {
        words.join(" ")
    } else {
        without_initials.join(" ")
    }
}

/// Punctuation removed before comparison. `&` survives — it is part of a firm's
/// name ("Penzien & McBride"), not decoration.
fn is_name_punctuation(c: char) -> bool {
    matches!(
        c,
        '.' | ',' | ';' | ':' | '!' | '?' | '"' | '\'' | '(' | ')' | '[' | ']' | '-' | '/'
    )
}

/// Why an alias lookup did or did not bind.
///
/// ## Rust Learning: an enum instead of `Option<String>`
///
/// Three of these four outcomes mean "no match", and they mean very different
/// things to an operator: a stoplisted role word is *working as designed*, an
/// ambiguous string is *a pending merge*, and no-match is *a genuinely new
/// party*. `Option` would collapse all three into `None` and the caller could
/// only log one message for all of them — the exact silence Standing Rule 1
/// forbids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AliasLookup {
    /// Exactly one node claims this key. Bind to it.
    Matched(String),
    /// Two or more nodes claim it. Never binds; the ids are carried so the
    /// caller can name them in the warning.
    Ambiguous(Vec<String>),
    /// A generic role word. Never binds, and this is not a defect.
    Stoplisted,
    /// Nobody claims it — a genuinely new party.
    NoMatch,
}

/// Every normalized name and alias in the graph, grouped by what claims it.
///
/// ## Why names are indexed alongside aliases
///
/// The ambiguity rule is "claimed by two or more NODES", and a string can be the
/// canonical name of one node and an alias of another — `dalek` is exactly that
/// today (`person-mr-dalek`'s name, `person-gerald-dalek`'s alias), which is the
/// signature of a merge that has not happened yet. Indexing only aliases would
/// call that unambiguous and bind a mention to whichever node happened to be
/// listed, silently picking a side in a pending merge.
#[derive(Debug, Clone, Default)]
pub struct PartyAliasIndex {
    /// `(entity_type, normalized key)` → the node ids claiming it, sorted.
    ///
    /// `BTreeMap`/`BTreeSet` rather than the hash variants so the ambiguity
    /// report is in a stable order — two runs over the same graph produce
    /// byte-identical warnings, which is what makes them diffable.
    claims: BTreeMap<(String, String), BTreeSet<String>>,
}

/// One string claimed by more than one node, for the caller to log once per run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmbiguousKey {
    pub entity_type: String,
    pub key: String,
    pub node_ids: Vec<String>,
}

impl PartyAliasIndex {
    /// Build the index from `(entity_type, node_id, surface_form)` triples.
    ///
    /// Feed it every node's `name` AND every entry of its `aliases`. Surface
    /// forms that normalize to nothing are skipped rather than stored under an
    /// empty key.
    pub fn build<I, S>(rows: I) -> Self
    where
        I: IntoIterator<Item = (S, S, S)>,
        S: AsRef<str>,
    {
        let mut claims: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
        for (entity_type, node_id, surface) in rows {
            let key = normalize_party_key(surface.as_ref());
            if key.is_empty() {
                continue;
            }
            claims
                .entry((entity_type.as_ref().to_string(), key))
                .or_default()
                .insert(node_id.as_ref().to_string());
        }
        PartyAliasIndex { claims }
    }

    /// Build the index from what `fetch_existing_parties` returned.
    ///
    /// The single constructor for a `KnownEntity` list, so the ingest path and
    /// the read-only verifier cannot drift apart about what gets indexed. It was
    /// two byte-identical private functions until review caught it.
    ///
    /// Feeds BOTH the canonical name and every alias in, because the ambiguity
    /// rule counts claiming NODES, not claiming aliases: a string that is one
    /// node's name and another's alias is a merge that has not happened, and it
    /// must block rather than silently pick a side.
    pub fn from_known_entities(existing: &[colossus_extract::KnownEntity]) -> Self {
        let mut rows: Vec<(String, String, String)> = Vec::new();
        for k in existing {
            rows.push((k.entity_type.clone(), k.id.clone(), k.label.clone()));
            // A node predating the alias writer has no `aliases` key at all, and
            // one written by a future template could carry the wrong JSON shape.
            // Both yield "no aliases" rather than an error: the name is still
            // indexed, so resolution degrades to today's behaviour.
            if let Some(aliases) = k.properties.get("aliases").and_then(|v| v.as_array()) {
                for alias in aliases.iter().filter_map(|a| a.as_str()) {
                    rows.push((k.entity_type.clone(), k.id.clone(), alias.to_string()));
                }
            }
        }
        Self::build(rows)
    }

    /// Every key claimed by two or more nodes — the pending-merge report.
    pub fn ambiguous_keys(&self) -> Vec<AmbiguousKey> {
        self.claims
            .iter()
            .filter(|(_, ids)| ids.len() > 1)
            .map(|((entity_type, key), ids)| AmbiguousKey {
                entity_type: entity_type.clone(),
                key: key.clone(),
                node_ids: ids.iter().cloned().collect(),
            })
            .collect()
    }

    /// Resolve one surface form against the index.
    ///
    /// Stoplist is checked BEFORE ambiguity: "the Court" is both, and "it is a
    /// role word" is the more useful thing to tell an operator than "two nodes
    /// claim it" — the first is permanent, the second is a merge away from
    /// resolving.
    pub fn lookup(&self, entity_type: &str, surface: &str) -> AliasLookup {
        let key = normalize_party_key(surface);
        if key.is_empty() {
            return AliasLookup::NoMatch;
        }
        if is_stoplisted(&key) {
            return AliasLookup::Stoplisted;
        }
        match self.claims.get(&(entity_type.to_string(), key)) {
            None => AliasLookup::NoMatch,
            Some(ids) if ids.len() == 1 => {
                // `next()` on a one-element set: the length was just checked, so
                // this cannot be None — but it is expressed as a match rather
                // than an `.unwrap()` (Standing Rule 1: no unwrap in production).
                match ids.iter().next() {
                    Some(id) => AliasLookup::Matched(id.clone()),
                    None => AliasLookup::NoMatch,
                }
            }
            Some(ids) => AliasLookup::Ambiguous(ids.iter().cloned().collect()),
        }
    }
}

/// Whether a normalized key is a generic role word.
///
/// The stoplist is stored in display form and normalized on every call rather
/// than at startup. It is 45 short strings compared once per unresolved party
/// mention — a handful of times per document — and keeping one representation
/// means the list a human reads and the list the code compares can never drift.
pub fn is_stoplisted(normalized_key: &str) -> bool {
    GENERIC_ROLE_STOPLIST
        .iter()
        .any(|entry| normalize_party_key(entry) == normalized_key)
}

#[cfg(test)]
#[path = "party_alias_tests.rs"]
mod tests;
