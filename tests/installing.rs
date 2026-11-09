//! Integration tests for [Concept: SpecMan Installing], [Concept: SpecMan Lockfile], and
//! git-reference resolution.

mod common;

use specman::{parse_git_url, resolve_commit};

fn forty(char: char) -> String {
    char.to_string().repeat(40)
}

#[test]
fn resolve_commit_passes_through_40_char_hash() {
    let hash = forty('a');
    let resolved = resolve_commit("https://unused.invalid/x.git", &hash, None).unwrap();
    assert_eq!(resolved, hash);
}

#[test]
fn resolve_commit_short_circuits_on_locked_hash() {
    // A locked hash takes precedence and avoids any tag re-resolution.
    let locked = forty('b');
    let resolved =
        resolve_commit("https://unused.invalid/x.git", "sometag", Some(&locked)).unwrap();
    assert_eq!(resolved, locked);
}

#[test]
fn resolve_commit_resolves_tag_from_local_repository() {
    let upstream = common::temp_git_workspace(&[]);
    // Create a commit so there is a HEAD to tag.
    common::write(
        upstream.path(),
        "specman.json",
        r#"{ "name": "dep", "spec": {} }"#,
    );
    common::git(upstream.path(), &["add", "-A"]);
    common::git(upstream.path(), &["commit", "-q", "-m", "initial"]);
    common::git(upstream.path(), &["tag", "v1.0.0"]);

    // `git ls-remote` works against a local filesystem path.
    let url = upstream.path().to_string_lossy().to_string();
    let resolved = resolve_commit(&url, "v1.0.0", None).expect("resolve tag");
    assert_eq!(resolved.len(), 40);
}

#[test]
fn resolve_commit_fails_for_missing_tag() {
    let upstream = common::temp_git_workspace(&[]);
    common::write(
        upstream.path(),
        "specman.json",
        r#"{ "name": "dep", "spec": {} }"#,
    );
    common::git(upstream.path(), &["add", "-A"]);
    common::git(upstream.path(), &["commit", "-q", "-m", "initial"]);

    let url = upstream.path().to_string_lossy().to_string();
    let err = resolve_commit(&url, "no-such-tag", None).expect_err("missing tag should fail");
    assert!(err.to_string().contains("no-such-tag") || err.to_string().contains("did not resolve"));
}

#[test]
fn parse_git_url_strips_scheme_and_git_suffix() {
    assert_eq!(
        parse_git_url("https://github.com/owner/repo.git"),
        ("github.com".to_string(), "owner/repo".to_string())
    );
    assert_eq!(
        parse_git_url("git@github.com:owner/repo.git"),
        ("github.com".to_string(), "owner/repo".to_string())
    );
    // Trailing slash in the path is trimmed.
    assert_eq!(
        parse_git_url("https://github.com/owner/repo.git"),
        ("github.com".to_string(), "owner/repo".to_string())
    );
}

#[test]
fn lockfile_absent_returns_none() {
    let temp = common::temp_git_workspace(&[("specman.json", r#"{ "name": "x", "spec": {} }"#)]);
    let paths = common::workspace(temp.path());
    assert!(specman::load_lockfile(&paths).unwrap().is_none());
}

#[test]
fn lockfile_rejects_malformed_json() {
    let temp = common::temp_git_workspace(&[
        ("specman.json", r#"{ "name": "x", "spec": {} }"#),
        ("specman.lock", "{ not json"),
    ]);
    let paths = common::workspace(temp.path());
    specman::load_lockfile(&paths).expect_err("malformed lockfile");
}
