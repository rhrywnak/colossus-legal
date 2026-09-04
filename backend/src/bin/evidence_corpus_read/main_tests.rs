//! Tests for `main`, in their own file so the module stays inside
//! Rule 17's 300-line ceiling.

use super::*;

/// The redaction, tested because it is the only thing standing between a
/// failed connection and a database password in a pasted report.
#[test]
fn redact_keeps_the_host_and_database_and_drops_every_credential() {
    assert_eq!(
        redact("postgres://colossus:s3cr3t@10.10.100.200:5432/colossus_legal_v2"),
        "10.10.100.200:5432/colossus_legal_v2"
    );
    // No credentials at all is the common local case.
    assert_eq!(
        redact("postgres://10.10.100.200:5432/colossus_legal_v2"),
        "10.10.100.200:5432/colossus_legal_v2"
    );
    // A query string can carry a password too (`?password=…`), so it goes.
    assert_eq!(
        redact("postgres://u:p@host/db?sslmode=require&password=s3cr3t"),
        "host/db"
    );
    // An `@` inside the password must not fool the split: the LAST `@`
    // separates userinfo from host, and `rsplit_once` is what guarantees it.
    assert_eq!(redact("postgres://u:p@ss@host/db"), "host/db");
}

/// The property that matters, stated as a property rather than an example.
#[test]
fn no_secret_survives_redaction() {
    for url in [
        "postgres://colossus:s3cr3t@10.10.100.200:5432/colossus_legal_v2",
        "postgresql://admin:hunter2@db.internal:5432/x?password=hunter2",
        "postgres://u:p@ss@host/db",
    ] {
        let out = redact(url);
        for secret in ["s3cr3t", "hunter2", "colossus:", "admin", "p@ss"] {
            assert!(
                !out.contains(secret),
                "redact({url}) leaked {secret:?}: {out}"
            );
        }
    }
}
