//! Shared helpers for integration tests.
//!
//! These helpers provision temporary SpecMan workspaces on disk. A workspace is a git
//! repository toplevel containing a `specman.json` manifest (plus an optional `.specman`
//! directory), matching the rules in [Concept: SpecMan Workspace] and
//! [Concept: Workspace Discovery].
#![allow(dead_code)]

use std::fs;
use std::path::Path;
use std::process::Command;

use specman::{WorkspacePaths, discover_workspace};

/// Run `git` inside `dir`. Panics on failure so test setup is self-documenting.
pub fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("HOME", "/tmp")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed in {dir:?}");
}

/// Creates a temporary git repository (with `.git` and a `.specman` directory) rooted at
/// the returned directory. Also writes the provided files (paths relative to the root).
pub fn temp_git_workspace(files: &[(&str, &str)]) -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    init_git_repo(temp.path());
    for (rel, content) in files {
        write(temp.path(), rel, content);
    }
    temp
}

/// Initializes `root` as a git repository and creates the `.specman` directory.
pub fn init_git_repo(root: &Path) {
    fs::create_dir_all(root).unwrap();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.com"]);
    git(root, &["config", "user.name", "Test User"]);
    git(root, &["config", "commit.gpgsign", "false"]);
    fs::create_dir_all(root.join(".specman")).unwrap();
}

/// Writes `content` to `root`-relative `rel`, creating parent directories as needed.
pub fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// Builds a `WorkspacePaths` for an already-provisioned temp directory.
pub fn workspace(root: &Path) -> WorkspacePaths {
    WorkspacePaths::new(root.to_path_buf(), root.join(".specman"))
}

/// Discovers the workspace rooted at `root` (which must be a git toplevel with a manifest).
pub fn discover(root: &Path) -> WorkspacePaths {
    discover_workspace(root).expect("discover workspace")
}
