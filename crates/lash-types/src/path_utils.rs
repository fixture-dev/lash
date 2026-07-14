//! Path-related utilities shared across crates.
//!
//! This module is the single source of truth for two questions every layer
//! needs to answer:
//!
//! 1. *Is this directory the root of a Lash project?* — `is_project_root_marker`
//! 2. *Walking upward from here, where does the project live?* —
//!    `find_project_root_from`
//!
//! Each consumer crate (lash-cli, lash-db, lash-types config) wraps these
//! with its own error/fallback semantics (anyhow vs `LashError` vs
//! `DbError`, return-self-on-miss vs hard error), but the walk and the
//! marker-check live exactly once, here.
//!
//! The walk is capped at the enclosing git repository's root (see
//! `find_git_root`) so leftover state in a user's home directory
//! (`~/.lash/`, a stray `lash.index.md`, etc.) can't hijack commands
//! invoked inside an unrelated project.

use std::path::{Path, PathBuf};

/// Walk upward from `start_dir` until a `.git` entry (directory or file) is
/// found, and return the directory containing it. Returns `None` if no git
/// repository contains `start_dir`.
///
/// `.git` is checked with `exists()` so worktrees and submodules (where
/// `.git` is a file pointing to the real gitdir) are detected too.
///
/// The returned path is canonicalized.
#[must_use]
pub fn find_git_root(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.canonicalize().ok()?;
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        current = current.parent()?.to_path_buf();
    }
}

/// Names of the files / directories that mark a Lash project root.
///
/// Kept as a public constant so external tools and tests can introspect the
/// list without parsing source code.
pub const PROJECT_MARKER_NAMES: &[&str] = &["lash.index.md", "index.lash.md", ".lash"];

/// Returns true if `dir` itself contains any Lash project marker:
/// `lash.index.md`, `index.lash.md`, or a `.lash/` directory.
///
/// The `.lash` check requires the entry to be a directory; the two markdown
/// markers only require the path to exist (to keep behaviour predictable
/// when running against a checked-out repository where mode bits or
/// follow-symlinks-style fs quirks could otherwise hide a regular file).
#[must_use]
pub fn is_project_root_marker(dir: &Path) -> bool {
    if dir.join(".lash").is_dir() {
        return true;
    }
    if dir.join("lash.index.md").exists() {
        return true;
    }
    if dir.join("index.lash.md").exists() {
        return true;
    }
    false
}

/// Walk upward from `start_dir` looking for a directory that satisfies
/// `is_project_root_marker`. Returns `Some(canonical_path)` for the first
/// match, or `None` if no marker is found within bounds.
///
/// Bounds:
/// - If `start_dir` is inside a git repository, the search is capped at the
///   git root (inclusive). Markers strictly above the git root are never
///   accepted — this is the guard that prevents `~/.lash/` (or any stray
///   marker in ancestor directories) from hijacking commands invoked
///   inside an unrelated project.
/// - If `start_dir` is *not* inside a git repository, the search is
///   uncapped and walks to the filesystem root.
///
/// The returned path is canonicalized. Returns `None` (rather than
/// erroring) if `start_dir` itself cannot be canonicalized — callers that
/// want a distinct error for that case should check beforehand.
#[must_use]
pub fn find_project_root_from(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.canonicalize().ok()?;
    let git_root = find_git_root(&current);

    loop {
        if is_project_root_marker(&current) {
            return Some(current);
        }
        if let Some(ref gr) = git_root {
            if current == *gr {
                return None;
            }
        }
        current = current.parent()?.to_path_buf();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn finds_dot_git_directory() {
        let temp = TempDir::new().unwrap();
        let nested = temp.path().join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir(temp.path().join(".git")).unwrap();

        let found = find_git_root(&nested).expect("expected to find git root");
        assert_eq!(
            found.canonicalize().unwrap(),
            temp.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn finds_dot_git_file_for_worktrees() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join(".git"), "gitdir: /elsewhere").unwrap();

        let found = find_git_root(temp.path()).expect("expected to find git root via file");
        assert_eq!(
            found.canonicalize().unwrap(),
            temp.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn returns_none_for_nonexistent_path() {
        let result = find_git_root(Path::new("/this/path/should/not/exist/xyz123"));
        assert!(result.is_none());
    }

    #[test]
    fn marker_detects_lash_index_md() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("lash.index.md"), "# x").unwrap();
        assert!(is_project_root_marker(temp.path()));
    }

    #[test]
    fn marker_detects_index_lash_md() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("index.lash.md"), "# x").unwrap();
        assert!(is_project_root_marker(temp.path()));
    }

    #[test]
    fn marker_detects_dot_lash_dir() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join(".lash")).unwrap();
        assert!(is_project_root_marker(temp.path()));
    }

    #[test]
    fn marker_rejects_bare_directory() {
        let temp = TempDir::new().unwrap();
        assert!(!is_project_root_marker(temp.path()));
    }

    #[test]
    fn marker_rejects_dot_lash_when_its_a_file() {
        let temp = TempDir::new().unwrap();
        // A file named `.lash` is *not* a project marker; the marker is the
        // directory specifically.
        fs::write(temp.path().join(".lash"), "wat").unwrap();
        assert!(!is_project_root_marker(temp.path()));
    }

    #[test]
    fn find_project_root_finds_self() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("lash.index.md"), "# x").unwrap();
        let found = find_project_root_from(temp.path()).expect("expected to find self");
        assert_eq!(
            found.canonicalize().unwrap(),
            temp.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn find_project_root_walks_up_to_ancestor() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("lash.index.md"), "# x").unwrap();
        let nested = temp.path().join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        let found = find_project_root_from(&nested).expect("expected to find ancestor");
        assert_eq!(
            found.canonicalize().unwrap(),
            temp.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn find_project_root_refuses_to_cross_git_root() {
        // Stray marker above the git root must be ignored.
        let temp = TempDir::new().unwrap();
        let leftover = temp.path().join("leftover");
        fs::create_dir(&leftover).unwrap();
        fs::write(leftover.join("lash.index.md"), "stray").unwrap();

        let repo = leftover.join("repo");
        fs::create_dir(&repo).unwrap();
        fs::create_dir(repo.join(".git")).unwrap();
        let inner = repo.join("crate").join("src");
        fs::create_dir_all(&inner).unwrap();

        assert!(
            find_project_root_from(&inner).is_none(),
            "marker above git root must be ignored"
        );
    }

    #[test]
    fn find_project_root_accepts_marker_at_git_root_itself() {
        let temp = TempDir::new().unwrap();
        let repo = temp.path();
        fs::create_dir(repo.join(".git")).unwrap();
        fs::write(repo.join("lash.index.md"), "# x").unwrap();
        let inner = repo.join("a").join("b");
        fs::create_dir_all(&inner).unwrap();

        let found = find_project_root_from(&inner).expect("expected to find git root");
        assert_eq!(found.canonicalize().unwrap(), repo.canonicalize().unwrap());
    }

    #[test]
    fn find_project_root_returns_none_for_nonexistent_start() {
        let result = find_project_root_from(Path::new("/this/path/should/not/exist/xyz123"));
        assert!(result.is_none());
    }
}
