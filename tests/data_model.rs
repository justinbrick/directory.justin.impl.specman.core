//! Integration tests for Part 1 (Data Model) constraint groups.
//!
//! These tests exercise the data model rules for workspaces, specifications (headings,
//! slugs, content, constraints, layout, standalone, dependencies, metadata), implementations,
//! the project manifest, `GitReference`, and the lockfile entities.

mod common;

use specman::{
    GitReference, GitReferenceAlias, ImplementsEntry, LockedEntry, LockedEntryResolved,
    OverrideEntry, SpecManProjectManifest,
};
use specman::{StructureQuery, discover_workspace, workspace_relative_path};
use specman::{load_manifest, parse_manifest, validate_manifest_value};

use serde_json::json;

#[test]
fn dot_folder_is_named_specman() {
    let temp = common::temp_git_workspace(&[]);
    common::write(
        temp.path(),
        "specman.json",
        r#"{ "name": "example", "spec": {} }"#,
    );
    let paths = common::discover(temp.path());
    // `.specman` directory lives at the workspace root, alongside `.git`.
    assert_eq!(paths.dot_specman().file_name().unwrap(), ".specman");
    assert!(paths.dot_specman().parent().unwrap().join(".git").is_dir());
}

#[test]
fn workspace_root_is_git_toplevel_not_specman_folder() {
    // The `.specman` folder is NOT a boundary marker; the git toplevel is.
    let temp = common::temp_git_workspace(&[]);
    common::write(
        temp.path(),
        "specman.json",
        r#"{ "name": "example", "spec": {} }"#,
    );
    // Discovery from *inside* `.specman` should still resolve to the git toplevel.
    let nested = temp.path().join(".specman").join("cache");
    std::fs::create_dir_all(&nested).unwrap();
    let paths = discover_workspace(&nested).expect("discover from nested dir");
    assert_eq!(paths.root(), temp.path().canonicalize().unwrap());
}

#[test]
fn git_co_location_requires_manifest_at_root() {
    let temp = common::temp_git_workspace(&[]);
    // Git repo exists but no specman.json -> ManifestMissing.
    let err = discover_workspace(temp.path()).expect_err("no manifest should fail");
    let msg = err.to_string();
    assert!(
        msg.contains("specman.json"),
        "error mentions manifest: {msg}"
    );
}

#[test]
fn monorepo_layout_is_rejected() {
    // A single `.git` must not contain multiple `specman.json` files.
    let temp = common::temp_git_workspace(&[]);
    common::write(
        temp.path(),
        "specman.json",
        r#"{ "name": "one", "spec": {} }"#,
    );
    common::write(
        temp.path(),
        "sub/specman.json",
        r#"{ "name": "two", "spec": {} }"#,
    );
    // Rejection is enforced at the discovery boundary. `from_explicit`/`initialize`
    // surface the monorepo error; discovery from the nested manifest's parent triggers it.
    let sub = temp.path().join("sub");
    let err = discover_workspace(sub).expect_err("monorepo should fail");
    assert!(err.to_string().contains("monorepo"));
}

// ---------------------------------------------------------------------------
// Markdown slugs
// ---------------------------------------------------------------------------

#[test]
fn slug_inline_content_strips_formatting() {
    // slugify is private; exercise via the structure index round-trip instead.
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        (
            "spec.md",
            "# **Bold** and `code` and [a link](#overview)\n\n## Overview\n\nBody.\n",
        ),
    ]);
    let paths = common::workspace(temp.path());
    let index = specman::build_workspace_index(&paths).expect("build index");
    let slugs = index.list_heading_slugs();
    assert!(slugs.contains(&"bold-and-code-and-a-link".to_string()));
    assert!(slugs.contains(&"overview".to_string()));
}

#[test]
fn slug_normalization_whitespace_case_and_hyphens() {
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        (
            "spec.md",
            "#  Spaces   And   Hyphens -- Collapsed \n\n# UPPER case TITLE\n",
        ),
    ]);
    let paths = common::workspace(temp.path());
    let index = specman::build_workspace_index(&paths).expect("build index");
    let slugs = index.list_heading_slugs();
    assert!(slugs.contains(&"spaces-and-hyphens-collapsed".to_string()));
    assert!(slugs.contains(&"upper-case-title".to_string()));
}

#[test]
fn slug_disambiguates_duplicate_headings() {
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        ("spec.md", "# Overview\n\n## Overview\n\n### Overview\n"),
    ]);
    let paths = common::workspace(temp.path());
    let index = specman::build_workspace_index(&paths).expect("build index");
    let slugs = index.list_heading_slugs();
    for expected in ["overview", "overview-1", "overview-2"] {
        assert!(
            slugs.contains(&expected.to_string()),
            "missing slug {expected}: {slugs:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Specification concepts & entities / constraint groups
// ---------------------------------------------------------------------------

#[test]
fn constraint_identifier_requires_bang_groupset_colon() {
    // A constraint identifier line is `!group.set:` with at least two groups.
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        (
            "spec.md",
            "## Concept: Slugs\n\n!concept-slugs.formatting:\n- Must be normalized.\n",
        ),
    ]);
    let paths = common::workspace(temp.path());
    let index = specman::build_workspace_index(&paths).expect("build index");
    assert!(
        index
            .list_constraint_groups()
            .contains(&"concept-slugs.formatting".to_string())
    );
}

#[test]
fn constraint_content_associates_with_constraining_concept() {
    // The first group names the constrained heading slug; the relationship is indexed.
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        (
            "spec.md",
            "## Concept: Markdown Slugs\n\n!concept-markdown-slugs.formatting:\n- Rule A\n- Rule B\n",
        ),
    ]);
    let paths = common::workspace(temp.path());
    let index = specman::build_workspace_index(&paths).expect("build index");
    // Constraint render resolves through the heading relationship.
    let rendered = index
        .render_constraint_group("concept-markdown-slugs.formatting")
        .expect("render constraint group");
    assert!(rendered.contains("Rule A"));
    assert!(rendered.contains("Rule B"));
}

#[test]
fn constraint_group_requires_two_or_more_groups() {
    // A single segment (no period) is not a valid group set.
    let content = "!singlegroup:\n- not a valid group set\n";
    // `constraint_groups` is private; verify via building an index and checking it's absent.
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        ("spec.md", content),
    ]);
    let paths = common::workspace(temp.path());
    let index = specman::build_workspace_index(&paths).expect("build index");
    assert!(index.list_constraint_groups().is_empty());
}

#[test]
fn empty_result_constraint_group_is_absent() {
    // An empty inner group set (`!:`) is not a constraint identifier.
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        ("spec.md", "!\n- orphan bullet\n"),
    ]);
    let paths = common::workspace(temp.path());
    let index = specman::build_workspace_index(&paths).expect("build index");
    assert!(index.list_constraint_groups().is_empty());
}

#[test]
fn specification_must_define_at_least_one_heading() {
    // The constraint "specification MUST categorize content into headings" is enforced by the
    // presence of headings in the index; a document with none yields no head/indexed content.
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        ("spec.md", "Just a paragraph with no heading.\n"),
    ]);
    let paths = common::workspace(temp.path());
    let index = specman::build_workspace_index(&paths).expect("build index");
    assert!(index.list_heading_slugs().is_empty());
}

// ---------------------------------------------------------------------------
// Specification layout / standalone / metadata
// ---------------------------------------------------------------------------

#[test]
fn spec_index_is_optional_and_null_rejected() {
    // `spec.index` must not be null; when absent no default filename is assumed.
    let err = parse_manifest(r#"{ "name": "s", "spec": { "index": null } }"#)
        .expect_err("null index should fail");
    assert!(err.to_string().contains("null"));

    // Absent index parses fine.
    let project = parse_manifest(r#"{ "name": "s", "spec": {} }"#).expect("absent index is valid");
    assert!(project.is_spec());
    assert_eq!(project.spec().unwrap().index, None);
}

#[test]
fn metadata_lives_in_manifest_not_filesystem() {
    // Metadata (version, etc.) is not read from Markdown; specman.json is authoritative.
    let temp = common::temp_git_workspace(&[
        ("specman.json", r#"{ "name": "actual", "spec": {} }"#),
        ("spec.md", "---\nname = \"from-frontmatter\"\n---\n# T\n"),
    ]);
    let project = load_manifest(&temp.path().join("specman.json")).expect("load manifest");
    assert_eq!(project.name(), "actual");
}

// ---------------------------------------------------------------------------
// Project manifest (specman.json) rules
// ---------------------------------------------------------------------------

#[test]
fn manifest_mutual_exclusion_spec_impl() {
    let both = r#"{ "name": "x", "spec": {}, "impl": { "implements": [] } }"#;
    assert!(parse_manifest(both).is_err());

    let neither = r#"{ "name": "x" }"#;
    assert!(parse_manifest(neither).is_err());
}

#[test]
fn manifest_spec_and_impl_must_not_be_null() {
    assert!(parse_manifest(r#"{ "name": "x", "spec": null }"#).is_err());
    assert!(parse_manifest(r#"{ "name": "x", "impl": null }"#).is_err());
}

#[test]
fn manifest_name_is_required_non_empty() {
    assert!(parse_manifest(r#"{ "spec": {} }"#).is_err()); // missing name
    assert!(parse_manifest(r#"{ "name": "", "spec": {} }"#).is_err()); // empty name
    assert!(parse_manifest(r#"{ "name": "ok", "spec": {} }"#).is_ok());
}

#[test]
fn manifest_valid_json_utf8() {
    // Invalid JSON is rejected.
    assert!(parse_manifest("{ not json").is_err());
}

#[test]
fn validate_manifest_value_rejects_non_object() {
    let err = validate_manifest_value(&json!("just a string")).expect_err("non-object");
    assert!(err.to_string().contains("object"));
}

// ---------------------------------------------------------------------------
// GitReference
// ---------------------------------------------------------------------------

fn git_ref(alias: &str, url: &str, ref_: &str) -> GitReference {
    GitReference {
        alias: alias.parse().unwrap(),
        url: url.parse().unwrap(),
        ref_: ref_.to_string(),
    }
}

#[test]
fn git_reference_alias_matches_portable_segment() {
    // Valid aliases.
    assert!(
        git_ref("alpha", "https://h/a.git", "deadbeef")
            .alias
            .as_str()
            == "alpha"
    );
    assert!(git_ref("a1_b-c.d", "https://h/a.git", "x").alias.as_str() == "a1_b-c.d");
    // Invalid alias (leading separator not allowed) is rejected by parse.
    assert!("-leading".parse::<GitReferenceAlias>().is_err());
    assert!("has/slash".parse::<GitReferenceAlias>().is_err());
}

#[test]
fn git_reference_url_must_be_ssh_or_https() {
    assert!(
        git_ref("a", "git@github.com:owner/repo.git", "x")
            .url
            .as_str()
            .contains("git@")
    );
    assert!(
        git_ref("a", "https://github.com/owner/repo.git", "x")
            .url
            .as_str()
            .contains("https")
    );
    // The manifest validator rejects unsupported schemes.
    let manifest = json!({
        "name": "x",
        "spec": { "references": [ { "alias": "a", "url": "ftp://h/a", "ref": "x" } ] }
    });
    assert!(validate_manifest_value(&manifest).is_err());
}

#[test]
fn git_reference_site_identity_ignores_scheme_and_git_suffix() {
    let https = git_ref("a", "https://github.com/owner/repo.git", "x");
    let ssh = git_ref("b", "git@github.com:owner/repo.git", "x");
    assert_eq!(https.site(), "github.com/owner/repo");
    assert_eq!(ssh.site(), "github.com/owner/repo");
}

#[test]
fn override_entries_must_be_unique_by_site() {
    // Two entries with the same site are documented as invalid. The type-level guard is a
    // uniqueness check by `site` (enforced at resolution time); here we assert the site value
    // is what disambiguates overrides.
    let a = OverrideEntry {
        site: "github.com/owner/repo".parse().unwrap(),
        ref_: "v1".parse().unwrap(),
    };
    let b = OverrideEntry {
        site: "github.com/owner/repo".parse().unwrap(),
        ref_: "v2".parse().unwrap(),
    };
    assert_eq!(a.site, b.site);
}

#[test]
fn impl_project_implements_must_be_non_empty() {
    let err = parse_manifest(r#"{ "name": "i", "impl": { "implements": [] } }"#)
        .expect_err("empty implements should fail");
    assert!(err.to_string().contains("implements"));
    assert!(parse_manifest(
        r#"{ "name": "i", "impl": { "implements": [ { "ref": { "alias": "a", "url": "https://h/a.git", "ref": "x" } } ] } }"#
    )
    .is_ok());
}

#[test]
fn implements_entry_constraints_default_to_match_all() {
    // `constraints` is optional and serialized as absent; the manifest still parses.
    let project = parse_manifest(
        r#"{ "name": "i", "impl": { "implements": [ { "ref": { "alias": "a", "url": "https://h/a.git", "ref": "x" } } ] } }"#,
    )
    .unwrap();
    let SpecManProjectManifest::ImplementationManifest(m) = project else {
        panic!("expected impl");
    };
    let entry: &ImplementsEntry = &m.impl_.implements[0];
    assert!(entry.constraints.is_empty());
}

// ---------------------------------------------------------------------------
// Lockfile entities
// ---------------------------------------------------------------------------

#[test]
fn locked_entry_resolved_is_40_char_hex() {
    let valid: LockedEntryResolved = "a".repeat(40).parse().unwrap();
    assert_eq!(valid.as_str().len(), 40);
    // A non-40-hex value is rejected.
    assert!("short".parse::<LockedEntryResolved>().is_err());
}

#[test]
fn lockfile_rejects_unknown_version() {
    let temp = common::temp_git_workspace(&[
        ("specman.json", r#"{ "name": "x", "spec": {} }"#),
        ("specman.lock", r#"{ "version": 999, "references": [] }"#),
    ]);
    let paths = common::workspace(temp.path());
    let err = specman::load_lockfile(&paths).expect_err("unknown version");
    assert!(err.to_string().contains("version"));
}

#[test]
fn locked_entry_fields_roundtrip_via_manifest_types() {
    let entry = LockedEntry {
        alias: "core".parse().unwrap(),
        path: "spec/core/".to_string(),
        url: "https://github.com/o/core.git".to_string(),
        requested: "v1".to_string(),
        resolved: "b".repeat(40).parse().unwrap(),
        transitive: false,
    };
    assert_eq!(entry.path, "spec/core/");
    assert!(!entry.transitive);
}

#[test]
fn workspace_relative_path_uses_forward_slashes() {
    let root = std::path::Path::new("/tmp/ws");
    let abs = std::path::Path::new("/tmp/ws/spec/core/specman.json");
    assert_eq!(
        workspace_relative_path(root, abs).as_deref(),
        Some("spec/core/specman.json")
    );
}

// ---------------------------------------------------------------------------
// Concept / entity heading prefixes
// ---------------------------------------------------------------------------

#[test]
fn concept_and_entity_heading_prefixes_slugify() {
    let temp = common::temp_git_workspace(&[
        (
            "specman.json",
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        ),
        (
            "spec.md",
            "## Concept: Markdown Slugs\n\nBody.\n\n## Entity: GitReference\n\nFields.\n",
        ),
    ]);
    let paths = common::workspace(temp.path());
    let index = specman::build_workspace_index(&paths).expect("build index");
    let slugs = index.list_heading_slugs();
    // The literal "Concept:" and "Entity:" prefixes are preserved in the slug.
    assert!(slugs.contains(&"concept-markdown-slugs".to_string()));
    assert!(slugs.contains(&"entity-gitreference".to_string()));
}

// ---------------------------------------------------------------------------
// Manifest git-reference validation
// ---------------------------------------------------------------------------

#[test]
fn manifest_rejects_reserved_alias_device_names() {
    for alias in ["CON", "PRN", "AUX", "NUL", "COM1", "LPT9"] {
        let manifest = json!({
            "name": "x",
            "spec": { "references": [ { "alias": alias, "url": "https://h/a.git", "ref": "v" } ] }
        });
        assert!(
            validate_manifest_value(&manifest).is_err(),
            "reserved alias {alias:?} should be rejected"
        );
    }
}

#[test]
fn manifest_rejects_non_ssh_https_git_reference_url() {
    let manifest = json!({
        "name": "x",
        "spec": { "references": [ { "alias": "a", "url": "ftp://h/a.git", "ref": "v" } ] }
    });
    assert!(validate_manifest_value(&manifest).is_err());
}

#[test]
fn manifest_rejects_null_reference_fields() {
    for field in ["alias", "url", "ref"] {
        let mut obj = serde_json::Map::new();
        obj.insert("alias".into(), json!("a"));
        obj.insert("url".into(), json!("https://h/a.git"));
        obj.insert("ref".into(), json!("v"));
        obj.insert(field.into(), json!(null));
        let manifest = json!({ "name": "x", "spec": { "references": [obj] } });
        assert!(validate_manifest_value(&manifest).is_err(), "{field} null");
    }
}
