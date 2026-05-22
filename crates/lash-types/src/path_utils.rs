//! Path-related utilities shared across crates.
//!
//! Currently provides `find_git_root`, used by every layer that needs to
//! decide "where does this project end?" when walking the directory tree
//! upward looking for Lash project markers. Using the git repository's
//! top-level directory as a ceiling prevents leftover state in a user's
//! home directory (`~/.lash/`, a stray `lash.index.md`, etc.) from
//! hijacking commands invoked inside an unrelated project.

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
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return None,
        }
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
}
