//! Activity tracking for the status bar.
//!
//! Tracks the currently in-progress task and a rolling tail of recently
//! completed tasks. Updates flow from task-status transitions — initiated
//! either by the TUI itself or, in a later phase, by an external file
//! watcher. See `docs/live-tui-updates.md`.
//!
//! `ActivityState` is pure data with no I/O; the rendering side lives in
//! `crate::ui::status_bar`.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use lash_types::TaskStatus;

use lash_db::repository::tasks::TaskRepository;
use rusqlite::Connection;

/// Max entries kept in the recently-completed tail.
pub const RECENT_COMPLETED_CAP: usize = 3;

/// Entries older than this are pruned from the recently-completed tail.
pub const RECENT_COMPLETED_TTL: Duration = Duration::from_secs(5 * 60);

/// One entry in either activity section.
#[derive(Debug, Clone)]
pub struct ActivityEntry {
    /// Fully-qualified task id (`file#task`), used to deduplicate.
    pub full_id: String,
    /// Task title at the time of the transition.
    pub title: String,
    /// When the transition occurred.
    pub at: Instant,
}

impl PartialEq for ActivityEntry {
    fn eq(&self, other: &Self) -> bool {
        self.full_id == other.full_id
    }
}

/// State for the activity sections of the status bar.
#[derive(Debug, Default)]
pub struct ActivityState {
    /// The single in-progress entry, if any. Replaced when a new task moves to
    /// `InProgress`; cleared when the current one transitions out of `InProgress`.
    pub in_progress: Option<ActivityEntry>,
    /// Recently completed entries, newest first.
    pub recently_completed: VecDeque<ActivityEntry>,
}

impl ActivityState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed `in_progress` directly (used at startup from DB scan).
    pub fn set_in_progress(&mut self, entry: ActivityEntry) {
        self.in_progress = Some(entry);
    }

    /// Record a task status transition.
    ///
    /// Encodes the rules described in `tasks/tasks.status-bar-activity.md`:
    /// - moving into `InProgress` → set `in_progress`
    /// - moving out of `InProgress` → clear `in_progress` if it matches
    /// - moving into Done/Waived → push to `recently_completed`
    /// - moving from Done/Waived back to an open state → remove from
    ///   `recently_completed` (re-opened tasks shouldn't still show as "recent")
    pub fn record_transition(
        &mut self,
        full_id: &str,
        title: &str,
        old: TaskStatus,
        new: TaskStatus,
        at: Instant,
    ) {
        let was_in_progress = matches!(old, TaskStatus::InProgress);
        let now_in_progress = matches!(new, TaskStatus::InProgress);
        let was_complete = old.is_complete();
        let now_complete = new.is_complete();

        if now_in_progress && !was_in_progress {
            self.in_progress = Some(ActivityEntry {
                full_id: full_id.to_string(),
                title: title.to_string(),
                at,
            });
        } else if was_in_progress
            && !now_in_progress
            && self
                .in_progress
                .as_ref()
                .is_some_and(|e| e.full_id == full_id)
        {
            self.in_progress = None;
        }

        if now_complete && !was_complete {
            self.recently_completed.retain(|e| e.full_id != full_id);
            self.recently_completed.push_front(ActivityEntry {
                full_id: full_id.to_string(),
                title: title.to_string(),
                at,
            });
            while self.recently_completed.len() > RECENT_COMPLETED_CAP {
                self.recently_completed.pop_back();
            }
        } else if was_complete && !now_complete {
            self.recently_completed.retain(|e| e.full_id != full_id);
        }
    }

    /// Seed both activity sections from the DB at startup.
    ///
    /// - `in_progress`: takes the lexicographically-first task currently in
    ///   `InProgress` status (matches the deterministic ordering of
    ///   `TaskRepository::find_by_status`).
    /// - `recently_completed`: takes up to `RECENT_COMPLETED_CAP` done/waived
    ///   tasks from files modified within `RECENT_COMPLETED_TTL`. The DB
    ///   tracks file mtime, not per-task completion time, so a file recently
    ///   touched will surface its done tasks here — close enough for the
    ///   "what changed recently?" framing of the activity bar.
    ///
    /// Failures (DB errors, missing tasks) are swallowed silently: the
    /// activity bar is a UI nicety, not a correctness path.
    pub fn seed_from_db(&mut self, conn: &Connection, now: Instant) {
        let task_repo = TaskRepository::new(conn);

        // In-progress: first by deterministic ordering.
        if let Ok(mut in_progress) = task_repo.find_by_status(TaskStatus::InProgress) {
            if let Some(first) = in_progress.drain(..).next() {
                self.in_progress = Some(ActivityEntry {
                    full_id: first.full_id,
                    title: first.title,
                    at: now,
                });
            }
        }

        // Recently completed: query done/waived tasks from files modified
        // within the TTL. The query returns newest-first by file mtime;
        // push_front of each (in iteration order) would put oldest at the
        // front, so we push_back to preserve the newest-first VecDeque
        // contract used elsewhere.
        let since_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map_or(0, |d| {
                d.as_secs().saturating_sub(RECENT_COMPLETED_TTL.as_secs())
            });
        #[allow(clippy::cast_possible_wrap)]
        let since_i64 = since_secs as i64;
        if let Ok(recents) = task_repo.find_recently_completed(since_i64, RECENT_COMPLETED_CAP) {
            for task in recents {
                if self.recently_completed.len() >= RECENT_COMPLETED_CAP {
                    break;
                }
                self.recently_completed.push_back(ActivityEntry {
                    full_id: task.full_id,
                    title: task.title,
                    at: now,
                });
            }
        }
    }

    /// Drop recently-completed entries older than the TTL and enforce the cap.
    /// Called periodically from the TUI tick.
    pub fn prune(&mut self, now: Instant) {
        self.recently_completed.retain(|e| {
            now.checked_duration_since(e.at)
                .map_or(true, |d| d <= RECENT_COMPLETED_TTL)
        });
        while self.recently_completed.len() > RECENT_COMPLETED_CAP {
            self.recently_completed.pop_back();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> Instant {
        Instant::now()
    }

    #[test]
    fn open_to_in_progress_sets_in_progress_slot() {
        let mut a = ActivityState::new();
        a.record_transition(
            "f#t1",
            "First",
            TaskStatus::Open,
            TaskStatus::InProgress,
            now(),
        );
        let e = a.in_progress.expect("expected an in-progress entry");
        assert_eq!(e.full_id, "f#t1");
        assert_eq!(e.title, "First");
        assert!(a.recently_completed.is_empty());
    }

    #[test]
    fn second_in_progress_replaces_first() {
        let mut a = ActivityState::new();
        a.record_transition("f#a", "A", TaskStatus::Open, TaskStatus::InProgress, now());
        a.record_transition("f#b", "B", TaskStatus::Open, TaskStatus::InProgress, now());
        assert_eq!(a.in_progress.as_ref().unwrap().full_id, "f#b");
    }

    #[test]
    fn in_progress_to_done_clears_slot_and_pushes_to_recent() {
        let mut a = ActivityState::new();
        a.record_transition("f#a", "A", TaskStatus::Open, TaskStatus::InProgress, now());
        a.record_transition("f#a", "A", TaskStatus::InProgress, TaskStatus::Done, now());
        assert!(a.in_progress.is_none());
        assert_eq!(a.recently_completed.len(), 1);
        assert_eq!(a.recently_completed[0].full_id, "f#a");
    }

    #[test]
    fn in_progress_to_open_clears_slot_but_no_recent_push() {
        let mut a = ActivityState::new();
        a.record_transition("f#a", "A", TaskStatus::Open, TaskStatus::InProgress, now());
        a.record_transition("f#a", "A", TaskStatus::InProgress, TaskStatus::Open, now());
        assert!(a.in_progress.is_none());
        assert!(a.recently_completed.is_empty());
    }

    #[test]
    fn in_progress_to_done_for_different_task_does_not_clear_slot() {
        let mut a = ActivityState::new();
        a.record_transition("f#a", "A", TaskStatus::Open, TaskStatus::InProgress, now());
        a.record_transition("f#b", "B", TaskStatus::Open, TaskStatus::Done, now());
        assert_eq!(a.in_progress.as_ref().unwrap().full_id, "f#a");
        assert_eq!(a.recently_completed.len(), 1);
        assert_eq!(a.recently_completed[0].full_id, "f#b");
    }

    #[test]
    fn open_to_done_directly_pushes_to_recent() {
        let mut a = ActivityState::new();
        a.record_transition("f#x", "X", TaskStatus::Open, TaskStatus::Done, now());
        assert!(a.in_progress.is_none());
        assert_eq!(a.recently_completed.len(), 1);
    }

    #[test]
    fn recent_is_newest_first_and_bounded() {
        let mut a = ActivityState::new();
        for i in 0..(RECENT_COMPLETED_CAP + 2) {
            let id = format!("f#t{i}");
            let title = format!("Task {i}");
            a.record_transition(&id, &title, TaskStatus::Open, TaskStatus::Done, now());
        }
        assert_eq!(a.recently_completed.len(), RECENT_COMPLETED_CAP);
        assert_eq!(
            a.recently_completed[0].full_id,
            format!("f#t{}", RECENT_COMPLETED_CAP + 1)
        );
    }

    #[test]
    fn reopening_a_recent_removes_it_from_recent() {
        let mut a = ActivityState::new();
        a.record_transition("f#a", "A", TaskStatus::Open, TaskStatus::Done, now());
        assert_eq!(a.recently_completed.len(), 1);
        a.record_transition("f#a", "A", TaskStatus::Done, TaskStatus::Open, now());
        assert!(a.recently_completed.is_empty());
    }

    #[test]
    fn re_completing_same_task_dedups_to_one_entry() {
        let mut a = ActivityState::new();
        a.record_transition("f#a", "A", TaskStatus::Open, TaskStatus::Done, now());
        a.record_transition("f#a", "A", TaskStatus::Done, TaskStatus::Open, now());
        a.record_transition("f#a", "A", TaskStatus::Open, TaskStatus::Done, now());
        assert_eq!(a.recently_completed.len(), 1);
    }

    #[test]
    fn prune_drops_entries_older_than_ttl() {
        let mut a = ActivityState::new();
        let long_ago = Instant::now()
            .checked_sub(RECENT_COMPLETED_TTL + Duration::from_secs(1))
            .expect("checked_sub of small duration should succeed");
        a.recently_completed.push_back(ActivityEntry {
            full_id: "f#old".into(),
            title: "old".into(),
            at: long_ago,
        });
        a.recently_completed.push_front(ActivityEntry {
            full_id: "f#new".into(),
            title: "new".into(),
            at: Instant::now(),
        });
        a.prune(Instant::now());
        assert_eq!(a.recently_completed.len(), 1);
        assert_eq!(a.recently_completed[0].full_id, "f#new");
    }

    #[test]
    fn prune_enforces_cap_even_if_all_fresh() {
        let mut a = ActivityState::new();
        for i in 0..(RECENT_COMPLETED_CAP + 2) {
            a.recently_completed.push_front(ActivityEntry {
                full_id: format!("f#{i}"),
                title: format!("t{i}"),
                at: Instant::now(),
            });
        }
        a.prune(Instant::now());
        assert_eq!(a.recently_completed.len(), RECENT_COMPLETED_CAP);
    }

    #[test]
    fn waived_counts_as_complete() {
        let mut a = ActivityState::new();
        a.record_transition("f#w", "W", TaskStatus::Open, TaskStatus::Waived, now());
        assert_eq!(a.recently_completed.len(), 1);
    }

    #[test]
    fn set_in_progress_seeds_directly() {
        let mut a = ActivityState::new();
        a.set_in_progress(ActivityEntry {
            full_id: "f#seed".into(),
            title: "seed".into(),
            at: Instant::now(),
        });
        assert_eq!(a.in_progress.unwrap().full_id, "f#seed");
    }
}
