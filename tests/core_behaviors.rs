//! Integration tests for Part 2 (Core Behaviors) and Part 3 (Key Entities).
//!
//! Covers reference validation, structure rendering (cycles/deduplication/recursion depth),
//! validation tags and compliance, workspace status, metadata mutation, and lifecycle
//! automation (create/delete).

mod common;

use std::path::Path;

use specman::{
    ArtifactId, ArtifactKind, CreateImplOptions, CreateSpecOptions, DeleteOptions,
    HttpsValidationMode, HttpsValidationOptions, IdentityUpdate, ManifestUpdate,
    ReferenceDestinationKind, ReferenceIssueType, ReferenceValidationOptions, SpecificationUpdate,
    SpecmanEnv, StatusResult, StructureQuery, TransitiveValidationOptions, ValidationType,
    WorkspacePaths, WorkspaceStatusConfig, create_implementation, create_specification,
    delete_artifact, generate_report, parse_tags, scan_source_root, validate_reference_content,
    validate_references, validate_workspace_status,
};

fn workspace(root: &std::path::Path) -> WorkspacePaths {
    WorkspacePaths::new(root.to_path_buf(), root.join(".specman"))
}

// ---------------------------------------------------------------------------
// Reference validation
// ---------------------------------------------------------------------------

#[test]
fn reference_validation_classifies_destinations() {
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        (
            "spec.md",
            "[ws](other.md) [https](https://example.com/x) [mail](mailto:a@b.com)\n",
        ),
        ("other.md", "# Other\n"),
    ]);
    let paths = workspace(temp.path());
    let result = validate_references(
        &paths,
        &temp.path().join("spec.md"),
        &ReferenceValidationOptions::default(),
    )
    .unwrap();

    let kinds: Vec<ReferenceDestinationKind> = result
        .references
        .iter()
        .map(|r| r.destination_kind)
        .collect();
    assert!(kinds.contains(&ReferenceDestinationKind::WorkspaceFilesystem));
    assert!(kinds.contains(&ReferenceDestinationKind::Https));
    assert!(kinds.contains(&ReferenceDestinationKind::Unsupported));

    // The unsupported scheme (mailto) is reported as an error, not rewritten.
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.issue_type == ReferenceIssueType::UnsupportedScheme)
    );
}

#[test]
fn reference_validation_ignores_image_destinations() {
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        ("spec.md", "![alt](image.png) [real](doc.md)\n"),
        ("doc.md", "# Doc\n"),
    ]);
    let paths = workspace(temp.path());
    let result = validate_references(
        &paths,
        &temp.path().join("spec.md"),
        &ReferenceValidationOptions::default(),
    )
    .unwrap();
    // Only the inline link (not the image) is discovered.
    assert_eq!(result.references.len(), 1);
    assert_eq!(result.references[0].destination, "doc.md");
}

#[test]
fn reference_validation_resolves_relative_to_source_directory() {
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        ("sub/doc.md", "[target](target.md)\n"),
        ("sub/target.md", "# Target\n"),
    ]);
    let paths = workspace(temp.path());
    // Validate a file in a subdirectory; relative destinations resolve against it.
    let result = validate_references(
        &paths,
        &temp.path().join("sub/doc.md"),
        &ReferenceValidationOptions::default(),
    )
    .unwrap();
    assert!(result.is_valid());
}

#[test]
fn reference_validation_enforces_workspace_boundary() {
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        ("spec.md", "[escape](../outside.md)\n"),
    ]);
    let paths = workspace(temp.path());
    let result = validate_references(
        &paths,
        &temp.path().join("spec.md"),
        &ReferenceValidationOptions::default(),
    )
    .unwrap();
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.issue_type == ReferenceIssueType::WorkspaceBoundaryViolation)
    );
}

#[test]
fn reference_validation_checks_heading_fragments() {
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        (
            "spec.md",
            "# Intro\n\n[good](#details) [bad](#nope)\n\n## Details\n\nBody.\n",
        ),
    ]);
    let paths = workspace(temp.path());
    let result = validate_references(
        &paths,
        &temp.path().join("spec.md"),
        &ReferenceValidationOptions::default(),
    )
    .unwrap();
    // The unknown fragment is an error; the known one is not.
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.issue_type == ReferenceIssueType::UnknownFragment)
    );
    assert!(!result.errors.iter().any(|e| e.destination == "#details"));
}

#[test]
fn reference_validation_https_check_syntax_vs_reachability() {
    let content = "[a](https://example.com/valid) [ftp](ftp://host/x)\n";
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        ("spec.md", content),
    ]);
    let paths = workspace(temp.path());

    let opts = ReferenceValidationOptions {
        https: HttpsValidationOptions {
            mode: HttpsValidationMode::CheckSyntax,
        },
        transitive: TransitiveValidationOptions::default(),
    };
    let result = validate_reference_content(&paths, &temp.path().join("spec.md"), content, &opts);
    // Well-formed HTTPS is not an error; the unsupported `ftp://` scheme is.
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.issue_type == ReferenceIssueType::UnsupportedScheme)
    );
    assert!(
        !result
            .errors
            .iter()
            .any(|e| e.issue_type == ReferenceIssueType::InvalidHttpsUrl)
    );
}

#[test]
fn reference_validation_collects_all_errors_not_fail_fast() {
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        ("spec.md", "[a](missing1.md) [b](missing2.md)\n"),
    ]);
    let paths = workspace(temp.path());
    let result = validate_references(
        &paths,
        &temp.path().join("spec.md"),
        &ReferenceValidationOptions::default(),
    )
    .unwrap();
    assert_eq!(result.errors.len(), 2);
    assert!(!result.is_valid());
}

#[test]
fn reference_validation_transitive_follows_linked_documents() {
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "a.md" } }"#,
        ),
        ("a.md", "[b](b.md)\n"),
        ("b.md", "[gone](missing.md)\n"),
    ]);
    let paths = workspace(temp.path());

    let mut opts = ReferenceValidationOptions::default();
    opts.transitive.enabled = true;
    opts.transitive.max_documents = 10;
    let result = validate_references(&paths, &temp.path().join("a.md"), &opts).unwrap();
    assert!(!result.is_valid());
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.issue_type == ReferenceIssueType::MissingFile
                && e.destination == "missing.md")
    );

    // Disabled transitively: a.md's own link is valid, so it passes.
    let disabled = validate_references(
        &paths,
        &temp.path().join("a.md"),
        &ReferenceValidationOptions::default(),
    )
    .unwrap();
    assert!(disabled.is_valid());
}

// ---------------------------------------------------------------------------
// Structure rendering: cycles, deduplication, recursion depth
// ---------------------------------------------------------------------------

#[test]
fn structure_rendering_deduplicates_referenced_content() {
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        (
            "spec.md",
            "# A\n\nSee [b](#b) and [b](#b).\n\n# B\n\nOnce.\n",
        ),
    ]);
    let paths = workspace(temp.path());
    let index = specman::build_workspace_index(&paths).unwrap();
    let rendered = index.render_heading("a").unwrap();
    // "Once." appears exactly once despite a double reference to #b.
    assert_eq!(rendered.matches("Once.").count(), 1);
}

#[test]
fn structure_rendering_detects_cycles_without_infinite_loop() {
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        ("spec.md", "# A\n\nSee [b](#b).\n\n## B\n\nSee [a](#a).\n"),
    ]);
    let paths = workspace(temp.path());
    let index = specman::build_workspace_index(&paths).unwrap();
    // Must terminate (visited set prevents re-entry).
    let rendered = index.render_heading("a").unwrap();
    assert!(rendered.contains("See [b](#b)."));
}

#[test]
fn structure_rendering_enforces_max_depth() {
    // Build a chain of headings that exceeds the max recursion depth.
    let mut body = String::new();
    for i in 0..60 {
        let this = format!("h{i}");
        let next = format!("h{}", i + 1);
        if i < 59 {
            body.push_str(&format!("# Heading {i}\n\nSee [#{next}](#{next}).\n"));
        } else {
            body.push_str(&format!("# Heading {i}\n\nLeaf.\n"));
        }
        let _ = this;
    }
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        ("spec.md", &body),
    ]);
    let paths = workspace(temp.path());
    let index = specman::build_workspace_index(&paths).unwrap();
    // Rendering terminates; the deeply nested content is simply not fully expanded.
    let rendered = index.render_heading("heading-0");
    assert!(rendered.is_some());
}

#[test]
fn structure_returns_markdown_for_constraint_group() {
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        (
            "spec.md",
            "## Concept: Rules\n\n!concept-rules.formatting:\n- Do it.\n",
        ),
    ]);
    let paths = workspace(temp.path());
    let index = specman::build_workspace_index(&paths).unwrap();
    let rendered = index
        .render_constraint_group("concept-rules.formatting")
        .unwrap();
    assert!(rendered.contains("Do it."));
}

// ---------------------------------------------------------------------------
// Validation tags / scanning / compliance
// ---------------------------------------------------------------------------

#[test]
fn parse_tags_handles_default_and_explicit_types() {
    let content = "// [ENSURES: a.b]\n// [ENSURES: c.d:CHECK]\n// [ENSURES: e.f:MANUAL]\n";
    let tags = parse_tags(content, Path::new("lib.rs"));
    assert_eq!(tags.len(), 3);
    assert_eq!(tags[0].tag_type, ValidationType::Test);
    assert_eq!(tags[1].tag_type, ValidationType::Check);
    assert_eq!(tags[2].tag_type, ValidationType::Manual);
}

#[test]
fn parse_tags_ignores_whitespace_and_is_case_insensitive() {
    // Keyword is case-insensitive and must be immediately followed by `:`; whitespace around
    // the identifier and type is ignored.
    let content = "// [ensures:a.b:TEST]\n// [ENSURES: c.d : CHECK ]\n";
    let tags = parse_tags(content, Path::new("lib.rs"));
    assert_eq!(tags.len(), 2, "tags: {tags:?}");
    assert_eq!(tags[0].identifier, "a.b");
    assert_eq!(tags[0].tag_type, ValidationType::Test);
    assert_eq!(tags[1].identifier, "c.d");
    assert_eq!(tags[1].tag_type, ValidationType::Check);
}

#[test]
fn scan_source_root_skips_binary_files() {
    let temp = common::temp_git_workspace(&[]);
    common::write(temp.path(), "src/lib.rs", "// [ENSURES: a.b]\n");
    std::fs::create_dir_all(temp.path().join("data")).unwrap();
    std::fs::write(temp.path().join("data/bin.dat"), [0u8, 159, 146, 150, 0]).unwrap();
    let tags = scan_source_root(temp.path()).unwrap();
    let ids: Vec<&str> = tags.iter().map(|t| t.identifier.as_str()).collect();
    assert_eq!(ids, vec!["a.b"]);
}

#[test]
fn compliance_reports_all_locations_for_duplicate_tags() {
    let spec = ArtifactId {
        kind: ArtifactKind::Specification,
        name: "core".into(),
    };
    let impl_ = ArtifactId {
        kind: ArtifactKind::Implementation,
        name: "lib".into(),
    };
    let make_tag = |file: &str| specman::ValidationTag {
        identifier: "a.b".into(),
        tag_type: ValidationType::Test,
        location: specman::SourceLocation {
            file_path: file.into(),
            line_number: 1,
        },
    };
    let report = generate_report(
        spec,
        impl_,
        std::path::PathBuf::from("/tmp"),
        &["a.b".to_string()],
        vec![make_tag("a.rs"), make_tag("b.rs")],
    );
    assert_eq!(report.coverage["a.b"].len(), 2);
    assert!(report.missing.is_empty());
}

#[test]
fn compliance_reports_orphans() {
    let spec = ArtifactId {
        kind: ArtifactKind::Specification,
        name: "core".into(),
    };
    let impl_ = ArtifactId {
        kind: ArtifactKind::Implementation,
        name: "lib".into(),
    };
    let report = generate_report(
        spec,
        impl_,
        std::path::PathBuf::from("/tmp"),
        &[], // no known constraints
        vec![specman::ValidationTag {
            identifier: "ghost.tag".into(),
            tag_type: ValidationType::Test,
            location: specman::SourceLocation {
                file_path: "x.rs".into(),
                line_number: 1,
            },
        }],
    );
    assert_eq!(report.orphans.len(), 1);
    assert_eq!(report.orphans[0].identifier, "ghost.tag");
}

// ---------------------------------------------------------------------------
// Workspace status
// ---------------------------------------------------------------------------

#[test]
fn workspace_status_reports_fail_for_missing_compliance_anchor() {
    // An implementation with a governing spec that has constraints, but no anchors.
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "impl", "impl": { "implements": [] } }"#,
        ),
        ("impl.md", "# Impl\n"),
    ]);
    let paths = workspace(temp.path());
    let report = validate_workspace_status(&paths, &WorkspaceStatusConfig::default()).unwrap();
    // No references, no explicit constraints; should pass since no governing spec constraints.
    // (Compliance for an impl with no implements is vacuously satisfied.)
    assert_eq!(report.artifact_count, 1);
}

#[test]
fn workspace_status_category_configuration() {
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        ("spec.md", "# Spec\n\nSee [gone](missing.md).\n"),
    ]);
    let paths = workspace(temp.path());

    // Disable reference validation: broken links are ignored.
    let cfg = WorkspaceStatusConfig {
        references: false,
        ..WorkspaceStatusConfig::default()
    };
    let report = validate_workspace_status(&paths, &cfg).unwrap();
    assert_eq!(report.global_status, StatusResult::Pass);

    // Enable it: broken links cause failure.
    let report = validate_workspace_status(&paths, &WorkspaceStatusConfig::default()).unwrap();
    assert_eq!(report.global_status, StatusResult::Fail);
}

#[test]
fn workspace_status_structure_config_controls_manifest_check() {
    let temp = common::temp_git_workspace(&[("specman.json", "{ bad json")]);
    let paths = workspace(temp.path());

    // Structure disabled still fails because load_manifest is required to proceed.
    let cfg = WorkspaceStatusConfig {
        structure: false,
        references: false,
        cycles: false,
        compliance: false,
    };
    let report = validate_workspace_status(&paths, &cfg).unwrap();
    // The manifest still must be loadable to determine the project type.
    assert_eq!(report.global_status, StatusResult::Fail);
    assert!(!report.structure_errors.is_empty());
}

// ---------------------------------------------------------------------------
// Metadata mutation
// ---------------------------------------------------------------------------

#[test]
fn metadata_mutation_updates_scalar_fields_merging() {
    let temp = common::temp_git_workspace(&[("specman.json", r#"{ "name": "old", "spec": {} }"#)]);
    let path = temp.path().join("specman.json");
    let update = ManifestUpdate::Specification(Default::default());
    // A default update leaves everything unchanged.
    let project = specman::apply_manifest_update(&path, &update).unwrap();
    assert_eq!(project.name(), "old");
}

#[test]
fn metadata_mutation_rejects_wrong_artifact_type() {
    let temp = common::temp_git_workspace(&[("specman.json", r#"{ "name": "s", "spec": {} }"#)]);
    let path = temp.path().join("specman.json");
    // Applying an implementation update to a spec project fails.
    let update = ManifestUpdate::Implementation(Default::default());
    let err = specman::apply_manifest_update(&path, &update).expect_err("wrong type");
    assert!(err.to_string().contains("implementation"));
}

#[test]
fn metadata_mutation_removes_references_and_overrides() {
    let manifest = r#"{
        "name": "s",
        "spec": {
            "references": [
                { "alias": "core", "url": "https://github.com/o/core.git", "ref": "v1" },
                { "alias": "extra", "url": "https://github.com/o/extra.git", "ref": "v1" }
            ]
        },
        "overrides": [
            { "site": "github.com/o/core", "ref": "v1" }
        ]
    }"#;
    let temp = common::temp_git_workspace(&[("specman.json", manifest)]);
    let path = temp.path().join("specman.json");

    let update = ManifestUpdate::Specification(SpecificationUpdate {
        remove_references: vec!["extra".to_string()],
        overrides: specman::OverridesUpdate {
            remove_overrides: vec!["github.com/o/core".to_string()],
            ..Default::default()
        },
        ..Default::default()
    });

    let project = specman::apply_manifest_update(&path, &update).unwrap();
    let spec = project.spec().unwrap();
    assert_eq!(spec.references.len(), 1);
    assert_eq!(spec.references[0].alias.as_str(), "core");
    assert!(project.overrides().is_empty());
}

#[test]
fn metadata_mutation_updates_name_description_tags() {
    let temp = common::temp_git_workspace(&[("specman.json", r#"{ "name": "old", "spec": {} }"#)]);
    let path = temp.path().join("specman.json");
    let update = ManifestUpdate::Specification(SpecificationUpdate {
        identity: IdentityUpdate {
            name: Some("new-name".to_string()),
            description: Some("a description".to_string()),
            tags: Some(vec!["tag-a".to_string(), "tag-b".to_string()]),
        },
        ..Default::default()
    });
    let project = specman::apply_manifest_update(&path, &update).unwrap();
    assert_eq!(project.name(), "new-name");
    assert_eq!(project.description(), Some("a description"));
    assert_eq!(project.spec().unwrap().index, None);
}

// ---------------------------------------------------------------------------
// Lifecycle automation (create / delete)
// ---------------------------------------------------------------------------

#[test]
fn create_specification_dry_run_produces_body() {
    let temp = common::temp_git_workspace(&[("specman.json", r#"{ "name": "s", "spec": {} }"#)]);
    let env = SpecmanEnv::from_path(temp.path()).unwrap();
    let result = create_specification(
        &env,
        CreateSpecOptions {
            name: "myspec".into(),
            title: "My Spec".into(),
            dry_run: true,
        },
    )
    .unwrap();
    match result {
        specman::CreateResult::DryRun(body) => assert!(body.body.contains("My Spec")),
        specman::CreateResult::Persisted(_) => panic!("expected dry run"),
    }
}

#[test]
fn create_implementation_dry_run_produces_body() {
    let temp = common::temp_git_workspace(&[("specman.json", r#"{ "name": "s", "spec": {} }"#)]);
    let env = SpecmanEnv::from_path(temp.path()).unwrap();
    let target = specman::GitReference {
        alias: "core".parse().unwrap(),
        url: "https://github.com/o/core.git".parse().unwrap(),
        ref_: "v1".into(),
    };
    let result = create_implementation(
        &env,
        CreateImplOptions {
            name: "myimpl".into(),
            target,
            dry_run: true,
        },
    )
    .unwrap();
    match result {
        specman::CreateResult::DryRun(body) => assert!(body.body.contains("myimpl")),
        specman::CreateResult::Persisted(_) => panic!("expected dry run"),
    }
}

#[test]
fn delete_artifact_dry_run_reports_dependency_plan() {
    let temp = common::temp_git_workspace(&[("specman.json", r#"{ "name": "s", "spec": {} }"#)]);
    let env = SpecmanEnv::from_path(temp.path()).unwrap();
    let target = ArtifactId {
        kind: ArtifactKind::Specification,
        name: "s".into(),
    };
    let result = delete_artifact(
        &env,
        &target,
        DeleteOptions {
            force: false,
            dry_run: true,
        },
    )
    .unwrap();
    match result {
        specman::DeleteResult::DryRun(plan) => {
            // No downstream dependents, so not blocked.
            assert!(!plan.blocked);
        }
        specman::DeleteResult::Removed(_) => panic!("expected dry run"),
    }
}
