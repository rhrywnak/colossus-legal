//! The human-readable change report.
//!
//! [`ChangeReport`] owns the [`LoadPlan`] and renders it via `Display`. The
//! same plan drives both dry-run and real-run output — the only difference is
//! the `[DRY-RUN] ` line prefix and the trailing "no changes written" note —
//! so an operator sees identical numbers whether previewing or applying.

use super::authored::AuthoredCounts;
use super::plan::{CountPlan, LoadPlan, NodePlan, Tally};
use super::schema::DeclarationDef;
use chrono::Utc;
use std::fmt;

/// A rendered summary of a [`LoadPlan`], with dry-run and color toggles.
#[derive(Debug)]
pub struct ChangeReport {
    plan: LoadPlan,
    dry_run: bool,
    no_color: bool,
    generated_at: String,
    /// Tier-1 Postgres write counts. `None` when the loader ran Neo4j-only
    /// (no `--database-url`/`--case-slug`); `Some` reflects the rows
    /// written (or, in dry-run, the rows that would be written).
    authored: Option<AuthoredCounts>,
}

impl ChangeReport {
    /// Build a report for an already-computed plan. `dry_run` only affects
    /// presentation here; the loader is responsible for actually skipping
    /// writes. `authored` is the Tier-1 Postgres count (or `None` when
    /// Postgres was not configured).
    pub fn new(
        plan: LoadPlan,
        dry_run: bool,
        no_color: bool,
        authored: Option<AuthoredCounts>,
    ) -> Self {
        Self {
            plan,
            dry_run,
            no_color,
            generated_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            authored,
        }
    }

    /// Borrow the underlying plan (used by tests to assert on classifications).
    pub fn plan(&self) -> &LoadPlan {
        &self.plan
    }

    // --- color helpers (no-ops when `no_color`) ---

    fn paint(&self, code: &str, s: &str) -> String {
        if self.no_color {
            s.to_string()
        } else {
            format!("\x1b[{code}m{s}\x1b[0m")
        }
    }
    fn header(&self, s: &str) -> String {
        self.paint("1;36", s) // bold cyan
    }
    fn green(&self, s: &str) -> String {
        self.paint("32", s)
    }
    /// Render a deletion count red when non-zero, plain (and uncolored) at zero.
    fn deleted(&self, n: u64) -> String {
        if n > 0 {
            self.paint("31", &n.to_string())
        } else {
            n.to_string()
        }
    }

    // --- line builders ---

    fn line_prefix(&self) -> &'static str {
        if self.dry_run {
            "[DRY-RUN] "
        } else {
            ""
        }
    }

    fn tally_line(t: Tally) -> String {
        format!(
            "{} created, {} updated, {} unchanged",
            t.created, t.updated, t.unchanged
        )
    }

    fn element_line(&self, t: Tally, deleted: u64) -> String {
        format!(
            "{}, {} deleted (orphans)",
            Self::tally_line(t),
            self.deleted(deleted)
        )
    }

    fn declaration_line(decls: &[NodePlan<DeclarationDef>]) -> String {
        let t = Tally::of(decls);
        let (op, inop) = created_operative_split(decls);
        format!(
            "{} created ({op} operative, {inop} non-operative), {} updated, {} unchanged",
            t.created, t.updated, t.unchanged
        )
    }

    fn write_count(&self, f: &mut fmt::Formatter<'_>, c: &CountPlan) -> fmt::Result {
        let p = self.line_prefix();
        writeln!(
            f,
            "{p}{}",
            self.header(&format!(
                "LegalCount {} ({})",
                c.meta.count_number, c.meta.count_name
            ))
        )?;

        let props = if c.changed_legal_count_props.is_empty() {
            "unchanged".to_string()
        } else {
            format!("updated ({})", c.changed_legal_count_props.join(", "))
        };
        writeln!(f, "{p}  Properties:            {props}")?;
        writeln!(
            f,
            "{p}  Elements:              {}",
            self.element_line(Tally::of(&c.elements), c.orphan_elements)
        )?;
        if c.orphan_elements > 0 || c.orphan_proves_edges > 0 {
            writeln!(
                f,
                "{p}  PROVES_ELEMENT edges deleted (orphans): {}",
                self.deleted(c.orphan_proves_edges)
            )?;
        }
        self.write_child_sections(f, c)
    }

    /// Write the theory/declaration lines that only apply to some Counts.
    fn write_child_sections(&self, f: &mut fmt::Formatter<'_>, c: &CountPlan) -> fmt::Result {
        let p = self.line_prefix();
        if !c.breach_theories.is_empty() {
            writeln!(
                f,
                "{p}  Breach Theories:       {}",
                Self::tally_line(Tally::of(&c.breach_theories))
            )?;
        }
        if !c.improper_act_theories.is_empty() {
            writeln!(
                f,
                "{p}  Improper Act Theories: {}",
                Self::tally_line(Tally::of(&c.improper_act_theories))
            )?;
        }
        if !c.declarations.is_empty() {
            writeln!(
                f,
                "{p}  Declarations:          {}",
                Self::declaration_line(&c.declarations)
            )?;
        }
        Ok(())
    }

    fn write_total(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = self.line_prefix();
        let t = Totals::from_plan(&self.plan);
        writeln!(f, "{p}{}", self.header("TOTAL"))?;
        writeln!(
            f,
            "{p}  Elements: {}, {} deleted",
            Self::tally_line(t.elements),
            self.deleted(t.orphan_elements)
        )?;
        writeln!(
            f,
            "{p}  Breach Theories: {}, {} deleted",
            Self::tally_line(t.breach),
            self.deleted(self.plan.orphan_breach_theories)
        )?;
        writeln!(
            f,
            "{p}  Improper Act Theories: {}, {} deleted",
            Self::tally_line(t.improper),
            self.deleted(self.plan.orphan_improper_act_theories)
        )?;
        writeln!(
            f,
            "{p}  Declarations: {} created ({} operative, {} non-operative), {} updated, {} unchanged, {} deleted",
            t.declarations.created,
            t.decl_created_operative,
            t.decl_created_inoperative,
            t.declarations.updated,
            t.declarations.unchanged,
            self.deleted(self.plan.orphan_declarations)
        )?;
        writeln!(
            f,
            "{p}  PROVES_ELEMENT orphan edges deleted: {}",
            self.deleted(t.orphan_proves)
        )?;
        if self.plan.unattributed_orphan_elements > 0
            || self.plan.unattributed_orphan_proves_edges > 0
        {
            writeln!(
                f,
                "{p}  (unattributed orphans: {} Elements, {} PROVES_ELEMENT edges)",
                self.plan.unattributed_orphan_elements, self.plan.unattributed_orphan_proves_edges
            )?;
        }
        if let Some(a) = &self.authored {
            writeln!(
                f,
                "{p}  Authored (Postgres): {} entities upserted, {} HAS_ELEMENT relationships upserted",
                a.entities, a.relationships
            )?;
        }
        Ok(())
    }
}

impl fmt::Display for ChangeReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = self.line_prefix();
        writeln!(
            f,
            "{p}{}",
            self.header(&format!("Canonical Element Loader — {}", self.generated_at))
        )?;
        writeln!(f, "{p}{}", "=".repeat(48))?;
        for count in &self.plan.counts {
            writeln!(f)?;
            self.write_count(f, count)?;
        }
        writeln!(f)?;
        self.write_total(f)?;
        if self.dry_run {
            writeln!(f, "{p}{}", self.green("No changes written to Neo4j."))?;
        }
        Ok(())
    }
}

/// Grand totals across all Counts.
struct Totals {
    elements: Tally,
    breach: Tally,
    improper: Tally,
    declarations: Tally,
    decl_created_operative: usize,
    decl_created_inoperative: usize,
    orphan_elements: u64,
    orphan_proves: u64,
}

impl Totals {
    fn from_plan(plan: &LoadPlan) -> Self {
        let mut t = Totals {
            elements: Tally::default(),
            breach: Tally::default(),
            improper: Tally::default(),
            declarations: Tally::default(),
            decl_created_operative: 0,
            decl_created_inoperative: 0,
            orphan_elements: plan.unattributed_orphan_elements,
            orphan_proves: plan.unattributed_orphan_proves_edges,
        };
        for c in &plan.counts {
            t.elements += Tally::of(&c.elements);
            t.breach += Tally::of(&c.breach_theories);
            t.improper += Tally::of(&c.improper_act_theories);
            t.declarations += Tally::of(&c.declarations);
            let (op, inop) = created_operative_split(&c.declarations);
            t.decl_created_operative += op;
            t.decl_created_inoperative += inop;
            t.orphan_elements += c.orphan_elements;
            t.orphan_proves += c.orphan_proves_edges;
        }
        t
    }
}

/// Split *created* declarations into (operative, non-operative) counts.
fn created_operative_split(decls: &[NodePlan<DeclarationDef>]) -> (usize, usize) {
    use super::plan::ChangeKind;
    let operative = decls
        .iter()
        .filter(|d| d.kind == ChangeKind::Created && d.def.operative)
        .count();
    let inoperative = decls
        .iter()
        .filter(|d| d.kind == ChangeKind::Created && !d.def.operative)
        .count();
    (operative, inoperative)
}
