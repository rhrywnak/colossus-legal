//! Migrations DEV has already applied are FROZEN (ruled 2026-09-23).
//!
//! ## Why a test and not a note
//!
//! On 2026-09-22 a mis-derived database URL let a local boot apply six of this
//! branch's migrations to DEV's `colossus_legal_v2` (see BUILD L3 §0). Roman's
//! ruling was to leave them — and with it came a rule: those files are frozen
//! byte-for-byte, and anything further is a NEW migration file.
//!
//! The rule needs a guard because `sqlx` enforces it the hard way. It records a
//! checksum per applied version, and a file edited after it was applied fails
//! `validate_applied_migrations` at BOOT — which is a service that will not
//! start, discovered on a deploy. `rollback_tolerant` does not help: it relaxes
//! "the store is ahead of me", never a checksum.
//!
//! So this pins the six sha384 digests, MEASURED FROM DEV's own
//! `_sqlx_migrations.checksum` column on 2026-09-23 and compared byte for byte
//! with the files on disk. A change to any of these files fails here, in a
//! second, naming the file and the rule — instead of at the next deploy.
//!
//! ## Rust Learning: `sha2` rather than a shell-out
//!
//! The digest has to be computed the way sqlx computes it (SHA-384 over the raw
//! file bytes). The crate is already a dependency of this workspace, so the
//! test computes it in-process — a `shasum` subprocess would make the guard
//! depend on which machine runs it.

use sha2::{Digest, Sha384};

/// The six versions DEV recorded on 2026-09-23, with their digests.
///
/// Measured, not assumed: `SELECT version, encode(checksum,'hex') FROM
/// _sqlx_migrations` on `colossus_legal_v2`, and every one matched the file.
// STRUCTURAL: the identity of shipped artifacts, not configuration. A digest
// here can only change by a NEW migration replacing what an old one did.
const FROZEN: &[(&str, &str)] = &[
    ("20260922135353", "6b9af350defd9df96615ace31a3ef768f2a2baef5d520f773861c6d10dc35b99a0586fc353ea6bca7e51982bfe9d7ffa"),
    ("20260922142803", "a092221c488204800c08afc2c8939c0e8821986234ceca5ea0e4345ba44871f89e7d6af58f0289269fe985c219f8a39b"),
    ("20260922153654", "296c20a197795cce3f608c0dcb462554c43e840de212ed1db9e8a2706fe6f48f6adb5cc4c3443c8721259e4ab29153a7"),
    ("20260922165053", "1cb777b7f5de2cfa5402262a39cc7218fe59cf16472b66aaeb636fab6121ef25b0d93219acf626ba8fcee1f2f31001c4"),
    ("20260922223758", "5e6d1dbb8c3c84aa1681abe1a2a5e8daee3d32d6962974f1400816158bdccc05f9598dc170bd36b80ee458fa520c131f"),
    ("20260922231242", "81af08a21169bac158abdab8c2c3414a599432bc53bf8ad1b76f659c12d551efb0811da92387e4d7afd6616bff128e57"),
];

/// Every frozen migration still hashes to what DEV recorded.
#[test]
fn the_migrations_dev_has_applied_are_unchanged() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("pipeline_migrations");
    for (version, expected) in FROZEN {
        let path = std::fs::read_dir(&dir)
            .expect("pipeline_migrations is readable")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(version))
            })
            .unwrap_or_else(|| panic!("migration {version} is gone from pipeline_migrations — DEV has applied it, so it may not be deleted"));
        let bytes = std::fs::read(&path).expect("the migration is readable");
        let digest = hex(&Sha384::digest(&bytes));
        assert!(
            digest.starts_with(expected),
            "{} changed after DEV applied it.\n  expected sha384 to start {expected}\n  found            {digest}\nDEV records the OLD digest, and sqlx refuses to boot against a file whose checksum moved. Write a NEW migration instead (ruled 2026-09-23).",
            path.display()
        );
    }
}

/// Lower-case hex, the way `encode(checksum,'hex')` prints it in Postgres.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
