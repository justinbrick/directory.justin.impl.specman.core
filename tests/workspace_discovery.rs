//! Integration tests for [Concept: Workspace Discovery].

mod common;

use specman::{WorkspaceDiscovery, WorkspaceError};

#[test]
fn discovery_from_subdirectory_resolves_to_toplevel() {
    let temp = common::temp_git_workspace(&[("specman.json", r#"{ "name": "x", "spec": {} }"#)]);
    let sub = temp.path().join("a").join("b");
    std::fs::create_dir_all(&sub).unwrap();

    let ctx = WorkspaceDiscovery::initialize(&sub).expect("discover from subdirectory");
    assert_eq!(ctx.paths().root(), temp.path().canonicalize().unwrap());
}

#[test]
fn discovery_errors_when_no_git_repository() {
    let temp = tempfile::tempdir().unwrap();
    // No `.git` directory anywhere in the ancestry.
    let err = WorkspaceDiscovery::initialize(temp.path()).expect_err("no git repo");
    assert!(matches!(err, WorkspaceError::NotFound { .. }));
}

#[test]
fn discovery_errors_when_manifest_missing() {
    let temp = common::temp_git_workspace(&[]); // git repo but no specman.json
    let err = WorkspaceDiscovery::initialize(temp.path()).expect_err("missing manifest");
    assert!(matches!(err, WorkspaceError::ManifestMissing { .. }));
}

#[test]
fn explicit_root_requires_manifest() {
    let temp = common::temp_git_workspace(&[]);
    let err =
        WorkspaceDiscovery::from_explicit(temp.path().to_path_buf()).expect_err("no manifest");
    assert!(matches!(err, WorkspaceError::ManifestMissing { .. }));
}

#[test]
fn explicit_root_rejects_relative_path() {
    let err =
        WorkspaceDiscovery::from_explicit("relative/path").expect_err("relative path rejected");
    assert!(matches!(err, WorkspaceError::RelativePath { .. }));
}

#[test]
fn create_provisions_workspace_and_is_idempotent() {
    let temp = tempfile::tempdir().unwrap();
    common::init_git_repo(temp.path());

    let ctx = WorkspaceDiscovery::create(temp.path().to_path_buf()).expect("create workspace");
    assert!(temp.path().join(".specman").is_dir());
    assert!(temp.path().join(".specman/cache").is_dir());
    assert!(temp.path().join("spec").is_dir());

    // Idempotent: creating again succeeds without error.
    let ctx2 = WorkspaceDiscovery::create(temp.path().to_path_buf()).expect("create again");
    assert_eq!(ctx2.paths().root(), ctx.paths().root());
}

#[test]
fn create_rejects_nested_workspace() {
    let parent = tempfile::tempdir().unwrap();
    common::init_git_repo(parent.path());
    std::fs::write(
        parent.path().join("specman.json"),
        r#"{ "name": "p", "spec": {} }"#,
    )
    .unwrap();

    let child = parent.path().join("child");
    std::fs::create_dir_all(&child).unwrap();
    common::init_git_repo(&child);

    let err = WorkspaceDiscovery::create(child).expect_err("nested workspace rejected");
    assert!(matches!(err, WorkspaceError::NestedWorkspace { .. }));
}

#[test]
fn workspace_context_rejects_scheme_locators() {
    let temp = common::temp_git_workspace(&[("specman.json", r#"{ "name": "x", "spec": {} }"#)]);
    let ctx = WorkspaceDiscovery::initialize(temp.path()).expect("discover");
    // HTTPS and other URL schemes are rejected as workspace locators.
    let err = ctx
        .resolve_locator("https://example.com/not-a-file")
        .expect_err("https rejected");
    assert!(matches!(err, WorkspaceError::InvalidHandle { .. }));
}
