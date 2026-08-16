//! The rulings file: the only thing that can make this tool merge anything.
//!
//! ## Format
//!
//! ```text
//! # Anything after a hash is a comment. Blank lines are ignored.
//!
//! CLUSTER Tighe — the judge, split across the transcript and the opinion
//! SURVIVOR person-karen-a-tighe
//! MERGE person-tighe
//! END
//!
//! CLUSTER Humphrey
//! SKIP "Jeff" could equally be Jeff Sharp; not merging on a first name
//! END
//! ```
//!
//! A block is a `CLUSTER` line, then either one `SURVIVOR` and one or more
//! `MERGE` lines, or exactly one `SKIP` line, then `END`.
//!
//! ## Why a hand-written format and not JSON or YAML
//!
//! Roman fills this in during a merge session, one cluster at a time, reading
//! facts and typing a decision. A format with quoting rules and indentation
//! significance turns a typo into a parse error he has to debug mid-session. This
//! one has four keywords and no punctuation, and every error names the line
//! number and what was expected.
//!
//! ## Every parse failure is fatal
//!
//! There is no lenient mode and no "skip the bad block". A file this tool cannot
//! read completely is a file whose intent it does not know, and guessing at half
//! of a merge instruction is the exact failure the rulings file exists to
//! prevent. Standing Rule 1, at its sharpest.

use std::collections::HashMap;
use std::fmt::Write as _;

/// What Roman decided about one cluster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ruling {
    /// Merge every `members` node into `survivor`.
    Merge {
        survivor: String,
        members: Vec<String>,
    },
    /// Leave this cluster alone, for the stated reason.
    ///
    /// The reason is required. A bare `SKIP` would leave no record of WHY a
    /// cluster was left split, and the next merge session would have to
    /// re-derive it — which for the two do-not-auto-merge clusters means
    /// re-discovering that "Jeff" is ambiguous.
    Skip { reason: String },
}

/// One ruled cluster, with the label Roman wrote so the report can name it the
/// way he does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterRuling {
    pub label: String,
    /// The line the `CLUSTER` keyword was on, for error messages.
    pub line: usize,
    pub ruling: Ruling,
}

/// A parsed rulings file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulingsFile {
    pub clusters: Vec<ClusterRuling>,
}

/// Why a rulings file could not be used.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RulingsError {
    #[error("line {line}: {message}")]
    Syntax { line: usize, message: String },

    #[error(
        "the rulings file contains no cluster blocks. An empty file cannot be \
         distinguished from a file that was never filled in, so it is refused \
         rather than treated as 'merge nothing'"
    )]
    Empty,

    #[error(
        "node '{node}' appears in more than one cluster (lines {first} and \
         {second}). A node cannot merge into two survivors"
    )]
    DuplicateNode {
        node: String,
        first: usize,
        second: usize,
    },
}

/// Parse a rulings file.
///
/// ## Rust Learning: why this takes `&str` and not a path
///
/// Reading a file is I/O and belongs to the binary; deciding what its contents
/// mean is a pure function of those contents. Splitting them that way is what
/// lets every rule below be unit-tested against a string literal, with no
/// fixture files and no temp directories.
pub fn parse(text: &str) -> Result<RulingsFile, RulingsError> {
    let mut clusters = Vec::new();
    let mut open: Option<OpenBlock> = None;

    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let content = strip_comment(raw);
        if content.is_empty() {
            continue;
        }
        let (keyword, rest) = split_keyword(content);
        match keyword {
            "CLUSTER" => open = Some(start_cluster(line, rest, &open)?),
            "END" => {
                let block = open.take().ok_or_else(|| RulingsError::Syntax {
                    line,
                    message: "END without a matching CLUSTER".to_string(),
                })?;
                clusters.push(block.close(line)?);
            }
            other => {
                let block = open.as_mut().ok_or_else(|| RulingsError::Syntax {
                    line,
                    message: format!("'{other}' outside any CLUSTER block"),
                })?;
                block.directive(line, other, rest)?;
            }
        }
    }

    if let Some(block) = open {
        return Err(RulingsError::Syntax {
            line: block.line,
            message: format!("CLUSTER '{}' is never closed with END", block.label),
        });
    }
    if clusters.is_empty() {
        return Err(RulingsError::Empty);
    }
    check_for_duplicate_nodes(&clusters)?;
    Ok(RulingsFile { clusters })
}

impl RulingsFile {
    /// Clusters that will actually merge.
    pub fn merges(&self) -> impl Iterator<Item = &ClusterRuling> {
        self.clusters
            .iter()
            .filter(|c| matches!(c.ruling, Ruling::Merge { .. }))
    }

    /// Clusters Roman explicitly left alone.
    pub fn skips(&self) -> impl Iterator<Item = &ClusterRuling> {
        self.clusters
            .iter()
            .filter(|c| matches!(c.ruling, Ruling::Skip { .. }))
    }

    /// Every node id the file names, survivors included.
    pub fn all_nodes(&self) -> Vec<String> {
        self.clusters
            .iter()
            .flat_map(|c| match &c.ruling {
                Ruling::Merge { survivor, members } => std::iter::once(survivor.clone())
                    .chain(members.iter().cloned())
                    .collect::<Vec<_>>(),
                Ruling::Skip { .. } => Vec::new(),
            })
            .collect()
    }

    /// A one-paragraph summary for the report header.
    pub fn summary(&self) -> String {
        let merges = self.merges().count();
        let members: usize = self
            .merges()
            .map(|c| match &c.ruling {
                Ruling::Merge { members, .. } => members.len(),
                Ruling::Skip { .. } => 0,
            })
            .sum();
        let mut out = String::new();
        let _ = write!(
            out,
            "{} cluster(s) ruled: {merges} to merge ({members} node(s) merging in), \
             {} skipped",
            self.clusters.len(),
            self.skips().count()
        );
        out
    }
}

/// A block being read.
struct OpenBlock {
    label: String,
    line: usize,
    survivor: Option<String>,
    members: Vec<String>,
    skip: Option<String>,
}

impl OpenBlock {
    /// Apply one directive line inside the block.
    fn directive(&mut self, line: usize, keyword: &str, rest: &str) -> Result<(), RulingsError> {
        let value = rest.trim();
        match keyword {
            "SURVIVOR" => {
                require_value(line, "SURVIVOR", value)?;
                if let Some(existing) = &self.survivor {
                    return Err(RulingsError::Syntax {
                        line,
                        message: format!(
                            "a second SURVIVOR ('{value}'); this cluster already \
                             names '{existing}'"
                        ),
                    });
                }
                self.survivor = Some(value.to_string());
            }
            "MERGE" => {
                require_value(line, "MERGE", value)?;
                self.members.push(value.to_string());
            }
            "SKIP" => {
                require_value(line, "SKIP", value)?;
                self.skip = Some(value.to_string());
            }
            other => {
                return Err(RulingsError::Syntax {
                    line,
                    message: format!(
                        "unknown directive '{other}' (expected SURVIVOR, MERGE, SKIP or END)"
                    ),
                })
            }
        }
        Ok(())
    }

    /// Turn a closed block into a ruling, refusing every ambiguous shape.
    fn close(self, end_line: usize) -> Result<ClusterRuling, RulingsError> {
        let syntax = |message: String| RulingsError::Syntax {
            line: end_line,
            message,
        };
        let ruling = match (self.skip, self.survivor, self.members) {
            (Some(reason), None, members) if members.is_empty() => Ruling::Skip { reason },
            (Some(_), _, _) => {
                return Err(syntax(format!(
                    "cluster '{}' mixes SKIP with SURVIVOR/MERGE; it must be one \
                     or the other",
                    self.label
                )))
            }
            (None, Some(survivor), members) if !members.is_empty() => {
                if members.contains(&survivor) {
                    return Err(syntax(format!(
                        "cluster '{}' lists its survivor '{survivor}' as a MERGE \
                         member too",
                        self.label
                    )));
                }
                Ruling::Merge { survivor, members }
            }
            (None, Some(_), _) => {
                return Err(syntax(format!(
                    "cluster '{}' has a SURVIVOR but no MERGE lines; use SKIP if \
                     nothing should merge",
                    self.label
                )))
            }
            (None, None, members) if !members.is_empty() => {
                return Err(syntax(format!(
                    "cluster '{}' has MERGE lines but no SURVIVOR",
                    self.label
                )))
            }
            (None, None, _) => {
                return Err(syntax(format!(
                    "cluster '{}' is empty; every cluster needs a SURVIVOR plus \
                     MERGE lines, or a SKIP with a reason",
                    self.label
                )))
            }
        };
        Ok(ClusterRuling {
            label: self.label,
            line: self.line,
            ruling,
        })
    }
}

/// Begin a block, refusing a `CLUSTER` inside an unclosed one.
fn start_cluster(
    line: usize,
    label: &str,
    open: &Option<OpenBlock>,
) -> Result<OpenBlock, RulingsError> {
    if let Some(previous) = open {
        return Err(RulingsError::Syntax {
            line,
            message: format!(
                "CLUSTER inside the still-open cluster '{}' from line {}; add an \
                 END first",
                previous.label, previous.line
            ),
        });
    }
    let label = label.trim();
    require_value(line, "CLUSTER", label)?;
    Ok(OpenBlock {
        label: label.to_string(),
        line,
        survivor: None,
        members: Vec::new(),
        skip: None,
    })
}

/// No node may appear in two blocks — it cannot merge into two survivors, and a
/// survivor that is elsewhere a member would be deleted out from under its own
/// cluster.
fn check_for_duplicate_nodes(clusters: &[ClusterRuling]) -> Result<(), RulingsError> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for cluster in clusters {
        let Ruling::Merge { survivor, members } = &cluster.ruling else {
            continue;
        };
        for node in std::iter::once(survivor).chain(members.iter()) {
            if let Some(first) = seen.get(node) {
                return Err(RulingsError::DuplicateNode {
                    node: node.clone(),
                    first: *first,
                    second: cluster.line,
                });
            }
            seen.insert(node.clone(), cluster.line);
        }
    }
    Ok(())
}

/// Drop a trailing comment and surrounding whitespace.
fn strip_comment(raw: &str) -> &str {
    raw.split('#').next().unwrap_or("").trim()
}

/// Split `KEYWORD rest of line` into its two halves.
fn split_keyword(content: &str) -> (&str, &str) {
    match content.split_once(char::is_whitespace) {
        Some((keyword, rest)) => (keyword, rest),
        None => (content, ""),
    }
}

/// A directive with no value is a typo, not an instruction.
fn require_value(line: usize, keyword: &str, value: &str) -> Result<(), RulingsError> {
    if value.is_empty() {
        return Err(RulingsError::Syntax {
            line,
            message: format!("{keyword} needs a value"),
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "rulings_tests.rs"]
mod tests;
