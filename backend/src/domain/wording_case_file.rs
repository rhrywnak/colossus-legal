// =============================================================================
// backend/src/domain/wording_case_file.rs — the Chat case file box's automatic line
// =============================================================================
//
// CC_TASK_CACHE_KEEPWARM_v1 R3. The one line the Admin → Overview "Chat case
// file" box shows under "Loaded until", saying whether the automatic ping is on.
// Seeded by `pipeline_migrations/20260925081941_chat_keepwarm_settings.sql`. The
// backend fills `{start}` and `{end}` with `domain::wording_templates::render`
// (UNBRACED keys) and the browser draws the finished sentence (Rule 12).
//
// The box's other sentences are still constants in `chatCaseFileText.ts`; these
// three are rows because R3 §5 ruled this line into the store.

/// The three forms of the box's automatic line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseFileWording {
    /// On, with the window. `{start}` and `{end}` are its times.
    pub automatic_on: String,
    /// On, but stopped for the rest of the day (cap, a reload, an unpriced model,
    /// or the window has closed). The reason is in the log, not on the page.
    pub automatic_paused: String,
    /// Switched off.
    pub automatic_off: String,
}

// KEYS: the stable identifiers. Renaming one is a migration.
pub(crate) const KEY_CASE_FILE_AUTOMATIC_ON: &str = "chat_case_file_automatic_on";
pub(crate) const KEY_CASE_FILE_AUTOMATIC_PAUSED: &str = "chat_case_file_automatic_paused";
pub(crate) const KEY_CASE_FILE_AUTOMATIC_OFF: &str = "chat_case_file_automatic_off";

/// Every key this block reads, so a missing one is caught at boot BY NAME.
pub const CASE_FILE_WORDING_KEYS: &[&str] = &[
    KEY_CASE_FILE_AUTOMATIC_ON,
    KEY_CASE_FILE_AUTOMATIC_PAUSED,
    KEY_CASE_FILE_AUTOMATIC_OFF,
];

/// Build a [`CaseFileWording`] from the stored rows, or say which key is wrong.
///
/// # Errors
/// Whatever `read` returns for the first key that is missing, mis-kinded or blank.
pub fn build_case_file_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<CaseFileWording, E> {
    Ok(CaseFileWording {
        automatic_on: read(KEY_CASE_FILE_AUTOMATIC_ON)?,
        automatic_paused: read(KEY_CASE_FILE_AUTOMATIC_PAUSED)?,
        automatic_off: read(KEY_CASE_FILE_AUTOMATIC_OFF)?,
    })
}

#[cfg(test)]
#[path = "wording_case_file_tests.rs"]
pub(crate) mod tests;
