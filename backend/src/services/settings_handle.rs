//! The shared, swappable configuration snapshot (task 1.6, v2 §2b).
//!
//! Split from `settings_store` because the two answer different questions. That
//! module knows what a parameter MEANS — how to read it, what bounds it has, when
//! to refuse it. This one knows only how one snapshot is shared across requests
//! and replaced without stopping the world. Keeping them apart also keeps each
//! under the module-size limit.

use std::sync::{Arc, RwLock};

use crate::domain::settings::Settings;

/// The shared, swappable configuration snapshot.
///
/// ## Why a handle and not a plain `Arc<Settings>` on `AppState`
///
/// `AppState` is CLONED for every request. A bare `Arc<Settings>` field would
/// give each clone its own pointer, so replacing it inside one handler would be
/// invisible to every request that had already cloned the state — "edits take
/// effect on next read" would quietly become "edits take effect on next restart".
/// One `Arc` around the lock, cloned by every `AppState`, means all of them see
/// the same cell.
///
/// ## Rust Learning: `Arc<RwLock<Arc<T>>>`, and why the inner `Arc`
///
/// The outer `Arc` shares the cell; the `RwLock` guards the swap; the INNER `Arc`
/// is what lets a reader take the current snapshot and let go of the lock
/// immediately. Without it, a reader would have to hold the guard for as long as
/// it used the settings — across `.await` points, in an async handler, which is
/// how a std lock deadlocks a runtime. Cloning an `Arc` is a counter bump, so the
/// lock is held for nanoseconds and only ever uncontended.
#[derive(Clone)]
pub struct SettingsHandle(Arc<RwLock<Arc<Settings>>>);

impl SettingsHandle {
    /// Wrap the boot snapshot.
    pub fn new(settings: Settings) -> Self {
        SettingsHandle(Arc::new(RwLock::new(Arc::new(settings))))
    }

    /// The snapshot as it stands right now.
    ///
    /// Take this ONCE per request and pass the result down: a payload banded by
    /// two different snapshots because a human edited a cutoff halfway through
    /// would be internally inconsistent for no benefit.
    ///
    /// ## Why a poisoned lock degrades to the last-known snapshot
    ///
    /// A `RwLock` is poisoned only if a thread panicked while HOLDING it. The only
    /// code inside these guards is an `Arc` clone and an assignment, neither of
    /// which can panic — so poisoning here means something has gone wrong that
    /// this function cannot fix. Returning the value anyway (via `into_inner`)
    /// keeps the API serving the configuration it already had, and the alternative
    /// — propagating a lock error into every card, cap and cutoff read in the
    /// product — would turn an impossible condition into a fleet of 500s.
    pub fn current(&self) -> Arc<Settings> {
        match self.0.read() {
            Ok(guard) => Arc::clone(&guard),
            Err(poisoned) => {
                tracing::error!(
                    "the configuration lock is poisoned — serving the last known \
                     snapshot. A thread panicked while holding it; the settings \
                     themselves are unaffected."
                );
                Arc::clone(&poisoned.into_inner())
            }
        }
    }

    /// Install a new snapshot. Every later `current()` sees it.
    ///
    /// `pub(crate)` rather than `pub`: the ONLY legitimate caller is
    /// `settings_store::set_setting`, which swaps only after the store has been
    /// re-read. A handler installing a snapshot it built itself would put the
    /// running configuration out of step with the database.
    pub(crate) fn replace(&self, settings: Arc<Settings>) {
        match self.0.write() {
            Ok(mut guard) => *guard = settings,
            Err(poisoned) => {
                tracing::error!("the configuration lock is poisoned — installing anyway");
                *poisoned.into_inner() = settings;
            }
        }
    }
}

impl std::fmt::Debug for SettingsHandle {
    /// ## Rust Learning: a manual `Debug` for a lock
    ///
    /// `AppState` derives `Debug`, so every field needs it. Deriving it here would
    /// print the lock's internals; showing the snapshot is what a reader of a log
    /// line actually wants.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SettingsHandle")
            .field(&self.current())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The freshness law's pure half: what goes into the handle comes back out.
    ///
    /// `set_setting`'s swap is DEV-verified (it needs a database), but the read
    /// side is plain synchronous code and is what every request depends on. If
    /// `current()` returned anything but the installed snapshot, every card in
    /// the product would band by the wrong numbers with nothing failing.
    #[test]
    fn the_handle_returns_the_snapshot_it_was_given() {
        let handle = SettingsHandle::new(Settings::for_test());
        assert_eq!(*handle.current(), Settings::for_test());
    }

    /// A replacement is visible to every later read, through a CLONE of the
    /// handle — which is what `AppState` hands each request.
    ///
    /// This is the freshness law itself, minus the database: if cloning the
    /// handle copied the snapshot instead of sharing the cell, an edit would be
    /// invisible to every request already in flight and "edits take effect on
    /// next read" would quietly mean "on next restart".
    #[test]
    fn a_replacement_is_visible_through_a_clone_of_the_handle() {
        let handle = SettingsHandle::new(Settings::for_test());
        let cloned = handle.clone();

        let mut raised = Settings::for_test();
        raised.talking_points_cap = 7;
        handle.replace(Arc::new(raised));

        assert_eq!(
            cloned.current().talking_points_cap,
            7,
            "a clone taken BEFORE the swap must still see the new value"
        );
    }

    /// Two reads of an unchanged handle agree.
    ///
    /// Pins that `current()` hands out a snapshot rather than rebuilding one: a
    /// payload assembled from two `current()` calls must not be able to see two
    /// different configurations.
    #[test]
    fn repeated_reads_of_an_unchanged_handle_agree() {
        let handle = SettingsHandle::new(Settings::for_test());
        assert_eq!(*handle.current(), *handle.current());
    }

    /// The handle's `Debug` shows the settings, not the lock's internals.
    ///
    /// `AppState` derives `Debug`, so this renders in any state dump. A reader
    /// wants the cutoffs, not a `RwLock { data: ... }`.
    #[test]
    fn the_handle_debug_shows_the_parameters() {
        let rendered = format!("{:?}", SettingsHandle::new(Settings::for_test()));
        assert!(rendered.contains("talking_points_cap"), "{rendered}");
        assert!(!rendered.contains("RwLock"), "{rendered}");
    }
}
