//! Integration tests exercising the crate's public API surface.

use std::fs;

use specman::{
    ReferenceValidationOptions, StructureQuery, WorkspacePaths, build_workspace_index,
    validate_reference_content, validate_references,
};

fn workspace(root: &std::path::Path) -> WorkspacePaths {
    WorkspacePaths::new(root.to_path_buf(), root.join(".specman"))
}

fn write(path: &std::path::Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn builds_and_queries_a_structure_index() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = workspace(temp.path());

    write(
        &temp.path().join("specman.json"),
        r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
    );
    write(
        &temp.path().join("spec.md"),
        "# Intro\n\nSee [details](#details).\n\n## Details\n\nBody.\n",
    );

    let index = build_workspace_index(&workspace).expect("build index");
    assert!(index.list_heading_slugs().contains(&"intro".to_string()));
    assert!(index.list_heading_slugs().contains(&"details".to_string()));

    let rendered = index.render_heading("intro").expect("render intro");
    assert!(rendered.contains("Body."));
}

#[test]
fn validates_references_through_public_api() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = workspace(temp.path());

    write(&temp.path().join("spec.md"), "[gone](missing.md).\n");

    let result = validate_references(
        &workspace,
        &temp.path().join("spec.md"),
        &ReferenceValidationOptions::default(),
    )
    .expect("validate references");

    assert!(!result.is_valid());
    assert_eq!(result.errors.len(), 1);

    let content = "[gone](missing.md).\n";
    let direct = validate_reference_content(
        &workspace,
        &temp.path().join("spec.md"),
        content,
        &ReferenceValidationOptions::default(),
    );
    assert_eq!(direct.errors.len(), 1);
}
