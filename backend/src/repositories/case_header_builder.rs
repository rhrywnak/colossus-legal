//! Pure shaping logic for the case header: turn raw DB rows into the response
//! DTO. No database access here — that lives in
//! [`super::case_header_repository`] — so this bucketing/sorting logic is fully
//! unit-testable without a connection (the pattern used by
//! `case_summary_elements::group_and_sort_elements`).

use super::case_header_repository::{CaseRow, CounselRow, PartyRow};
use crate::dto::case_header::{
    CaseHeaderResponse, CounselContact, CourtInfo, DroppedDefendant, HeaderParty, PartiesGroups,
};

// CONST: these mirror the CHECK-constraint vocabulary in migration
// 20260524095049_case_metadata_tables.sql (parties.role / parties.status).
// They are schema identifiers, not tunable configuration — changing one
// requires a DB schema migration, not an env/YAML edit, so an external
// override would silently diverge from the constraint. Hence compiled
// constants, referenced by name rather than as bare string literals
// (Standing Rule 2).
const ROLE_PLAINTIFF: &str = "Plaintiff";
const ROLE_DEFENDANT: &str = "Defendant";
const STATUS_ACTIVE: &str = "active";

/// Raised when a party row carries a `role` outside the CHECK-constrained set.
/// The constraint makes this practically impossible, but per Standing Rule 1 we
/// fail loudly rather than silently dropping or misfiling the party.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CaseShapeError {
    #[error("party '{party_id}' has unexpected role '{role}' (expected Plaintiff or Defendant)")]
    UnexpectedRole { party_id: String, role: String },
}

/// Build the case-header response from the three raw row sets.
///
/// Bucketing is **role-first, status-second**:
/// - `Plaintiff` → `plaintiffs` (regardless of status — status only triages
///   defendants, since the design shows the DROPPED subheader only there).
/// - `Defendant` + status `"active"` → `active_defendants`.
/// - `Defendant` + any other status → `dropped_defendants`. This is
///   deliberately permissive (`!= "active"` rather than `== "dropped"`) so a
///   new defendant lifecycle state can't silently route to the wrong bucket.
///
/// Each group is sorted by `sort_order` ascending here (not relying on SQL
/// order), so the ordering is verifiable in a unit test.
pub(crate) fn build_case_header(
    case: CaseRow,
    parties: Vec<PartyRow>,
    counsel: Vec<CounselRow>,
) -> Result<CaseHeaderResponse, CaseShapeError> {
    let (plaintiffs, active_defendants, dropped_defendants) = bucket_parties(parties)?;

    let mut counsel: Vec<CounselContact> = counsel.into_iter().map(to_counsel_contact).collect();
    counsel.sort_by_key(|c| c.sort_order);

    Ok(CaseHeaderResponse {
        case_id: case.case_id,
        case_slug: case.case_slug,
        display_title: case.display_title,
        display_title_full: case.display_title_full,
        court: CourtInfo {
            name: case.court_name,
            jurisdiction: case.jurisdiction,
            case_number: normalize_case_number(case.case_number),
            filed_date: case.filed_date,
            transferred_from: case.transferred_from,
            transfer_date: case.transfer_date,
        },
        status: case.status,
        complaint_document_id: case.complaint_document_id,
        parties: PartiesGroups {
            plaintiffs,
            active_defendants,
            dropped_defendants,
        },
        counsel,
    })
}

/// (plaintiffs, active_defendants, dropped_defendants) — the three Home page
/// party groups, each owned and sorted.
type PartyBuckets = (Vec<HeaderParty>, Vec<HeaderParty>, Vec<DroppedDefendant>);

/// Split parties into the three groups, each sorted by `sort_order` ascending.
/// Role-first, status-second (see [`build_case_header`]). An unrecognized role
/// is a hard error rather than a silent drop.
fn bucket_parties(parties: Vec<PartyRow>) -> Result<PartyBuckets, CaseShapeError> {
    let mut plaintiffs = Vec::new();
    let mut active_defendants = Vec::new();
    let mut dropped_defendants = Vec::new();

    for p in parties {
        if p.role == ROLE_PLAINTIFF {
            plaintiffs.push(to_header_party(p));
        } else if p.role == ROLE_DEFENDANT {
            if p.status == STATUS_ACTIVE {
                active_defendants.push(to_header_party(p));
            } else {
                dropped_defendants.push(to_dropped_defendant(p));
            }
        } else {
            // Construct the error before moving party_id out of `p`.
            return Err(CaseShapeError::UnexpectedRole {
                role: p.role.clone(),
                party_id: p.party_id,
            });
        }
    }

    plaintiffs.sort_by_key(|p| p.sort_order);
    active_defendants.sort_by_key(|p| p.sort_order);
    dropped_defendants.sort_by_key(|d| d.sort_order);
    Ok((plaintiffs, active_defendants, dropped_defendants))
}

/// Collapse a blank docket number to `None`.
///
/// Domain note: the seed stores "not yet assigned" as `''`. Unlike other text
/// columns (where `""` vs `null` is preserved), an empty case number carries no
/// header information, so both `NULL` and `""` become JSON `null`. This special
/// case applies to `case_number` only.
fn normalize_case_number(raw: Option<String>) -> Option<String> {
    match raw {
        Some(s) if s.is_empty() => None,
        other => other,
    }
}

fn to_header_party(p: PartyRow) -> HeaderParty {
    HeaderParty {
        party_id: p.party_id,
        name: p.name,
        entity_type: p.entity_type,
        notes: p.notes,
        sort_order: p.sort_order,
    }
}

fn to_dropped_defendant(p: PartyRow) -> DroppedDefendant {
    DroppedDefendant {
        party_id: p.party_id,
        name: p.name,
        entity_type: p.entity_type,
        status: p.status,
        dismissal_date: p.dismissal_date,
        dismissal_basis: p.dismissal_basis,
        notes: p.notes,
        sort_order: p.sort_order,
    }
}

fn to_counsel_contact(c: CounselRow) -> CounselContact {
    CounselContact {
        counsel_id: c.counsel_id,
        represents_role: c.represents_role,
        firm_name: c.firm_name,
        attorney_name: c.attorney_name,
        bar_number: c.bar_number,
        address: c.address,
        phone: c.phone,
        email: c.email,
        sort_order: c.sort_order,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case_row() -> CaseRow {
        CaseRow {
            case_id: "case-1".into(),
            case_slug: "case_1".into(),
            display_title: "Title".into(),
            display_title_full: None,
            court_name: None,
            jurisdiction: None,
            case_number: None,
            filed_date: None,
            transferred_from: None,
            transfer_date: None,
            status: "active".into(),
            complaint_document_id: None,
        }
    }

    fn party(id: &str, name: &str, role: &str, status: &str, sort: i32) -> PartyRow {
        PartyRow {
            party_id: id.into(),
            name: name.into(),
            role: role.into(),
            entity_type: Some("individual".into()),
            status: status.into(),
            dismissal_date: None,
            dismissal_basis: None,
            notes: None,
            sort_order: sort,
        }
    }

    fn counsel(id: &str, sort: i32) -> CounselRow {
        CounselRow {
            counsel_id: id.into(),
            represents_role: "Plaintiff".into(),
            firm_name: None,
            attorney_name: "Attorney".into(),
            bar_number: None,
            address: None,
            phone: None,
            email: None,
            sort_order: sort,
        }
    }

    #[test]
    fn parties_with_status_dropped_appear_in_dropped_bucket_not_active() {
        let parties = vec![
            party("p1", "Plaintiff One", ROLE_PLAINTIFF, "active", 10),
            party("d1", "Def One", ROLE_DEFENDANT, "active", 20),
            party("d2", "Def Two", ROLE_DEFENDANT, "active", 21),
            party("d3", "Dropped Def", ROLE_DEFENDANT, "dropped", 30),
        ];
        let r = build_case_header(case_row(), parties, vec![]).unwrap();

        assert_eq!(r.parties.dropped_defendants.len(), 1);
        assert_eq!(r.parties.active_defendants.len(), 2);
        assert_eq!(r.parties.dropped_defendants[0].name, "Dropped Def");
        assert!(!r
            .parties
            .active_defendants
            .iter()
            .any(|d| d.name == "Dropped Def"));
    }

    #[test]
    fn parties_within_a_group_are_sorted_by_sort_order_ascending() {
        // Two plaintiffs supplied out of order (20 before 10).
        let parties = vec![
            party("p2", "Second", ROLE_PLAINTIFF, "active", 20),
            party("p1", "First", ROLE_PLAINTIFF, "active", 10),
        ];
        let r = build_case_header(case_row(), parties, vec![]).unwrap();

        assert_eq!(r.parties.plaintiffs[0].sort_order, 10);
        assert_eq!(r.parties.plaintiffs[1].sort_order, 20);
        assert_eq!(r.parties.plaintiffs[0].name, "First");
    }

    #[test]
    fn empty_counsel_returns_empty_array_not_null() {
        let r = build_case_header(
            case_row(),
            vec![party("p1", "P", ROLE_PLAINTIFF, "active", 10)],
            vec![],
        )
        .unwrap();
        // A Vec serializes to `[]`, never null; assert it is present and empty.
        assert!(r.counsel.is_empty());
    }

    #[test]
    fn dismissed_and_settled_defendants_also_go_in_dropped_bucket() {
        // Guards the bug of bucketing by `status == "dropped"` instead of
        // `status != "active"`.
        let parties = vec![
            party("d1", "Dismissed", ROLE_DEFENDANT, "dismissed", 10),
            party("d2", "Settled", ROLE_DEFENDANT, "settled", 20),
        ];
        let r = build_case_header(case_row(), parties, vec![]).unwrap();

        assert_eq!(r.parties.dropped_defendants.len(), 2);
        assert!(r.parties.active_defendants.is_empty());
        let names: Vec<&str> = r
            .parties
            .dropped_defendants
            .iter()
            .map(|d| d.name.as_str())
            .collect();
        assert_eq!(names, vec!["Dismissed", "Settled"]);
    }

    #[test]
    fn plaintiff_with_status_other_than_active_still_appears_in_plaintiffs() {
        // Role triages first: a non-active plaintiff is still a plaintiff.
        let parties = vec![party(
            "p1",
            "Dropped Plaintiff",
            ROLE_PLAINTIFF,
            "dropped",
            10,
        )];
        let r = build_case_header(case_row(), parties, vec![]).unwrap();

        assert_eq!(r.parties.plaintiffs.len(), 1);
        assert_eq!(r.parties.plaintiffs[0].name, "Dropped Plaintiff");
        assert!(r.parties.active_defendants.is_empty());
        assert!(r.parties.dropped_defendants.is_empty());
    }

    #[test]
    fn counsel_is_sorted_by_sort_order_ascending() {
        let r = build_case_header(
            case_row(),
            vec![],
            vec![counsel("c2", 20), counsel("c1", 10)],
        )
        .unwrap();
        assert_eq!(r.counsel[0].counsel_id, "c1");
        assert_eq!(r.counsel[1].counsel_id, "c2");
    }

    #[test]
    fn unexpected_party_role_is_a_hard_error_not_a_silent_drop() {
        // Standing Rule 1/§5: a role outside the CHECK set must fail loudly.
        let parties = vec![party("px", "Mystery", "Witness", "active", 10)];
        let err = build_case_header(case_row(), parties, vec![]).unwrap_err();
        // The Display message is the operator-facing text — pin it exactly.
        assert_eq!(
            err.to_string(),
            "party 'px' has unexpected role 'Witness' (expected Plaintiff or Defendant)"
        );
        let CaseShapeError::UnexpectedRole { party_id, role } = err;
        assert_eq!(party_id, "px");
        assert_eq!(role, "Witness");
    }

    #[test]
    fn active_defendants_are_sorted_by_sort_order_ascending() {
        // Supplied out of order (21 before 20) → must come back ascending.
        let parties = vec![
            party("d2", "Second", ROLE_DEFENDANT, "active", 21),
            party("d1", "First", ROLE_DEFENDANT, "active", 20),
        ];
        let r = build_case_header(case_row(), parties, vec![]).unwrap();
        assert_eq!(r.parties.active_defendants[0].sort_order, 20);
        assert_eq!(r.parties.active_defendants[1].sort_order, 21);
        assert_eq!(r.parties.active_defendants[0].name, "First");
    }

    #[test]
    fn blank_case_number_becomes_null_other_text_preserved() {
        // case_number "" → None (Q3 special case).
        assert_eq!(normalize_case_number(Some(String::new())), None);
        assert_eq!(normalize_case_number(None), None);
        assert_eq!(
            normalize_case_number(Some("13-12345".into())),
            Some("13-12345".into())
        );
    }
}
