use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::core::error::SpecmanError;
use crate::graph::tree::{ArtifactId, ArtifactKind, FilesystemDependencyMapper};
use crate::manifest::{SpecManProject, load_manifest};
use crate::workspace::{WorkspaceLocator, WorkspacePaths};

mod references;

pub use references::{
    DiscoveredReference, HttpsValidationMode, HttpsValidationOptions, ReferenceDestinationKind,
    ReferenceIssueType, ReferenceValidationIssue, ReferenceValidationOptions,
    ReferenceValidationResult, TransitiveValidationOptions, validate_reference_content,
    validate_references,
};

const BINARY_CHECK_BYTES: usize = 8000;

fn is_binary(content: &[u8]) -> bool {
    content.iter().take(BINARY_CHECK_BYTES).any(|&b| b == 0)
}

/// A validation tag is the concrete syntax used to define a validation anchor.
#[derive(
    Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct ValidationTag {
    pub identifier: String,
    pub tag_type: ValidationType,
    pub location: SourceLocation,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum ValidationType {
    Test,
    Check,
    Manual,
}

#[derive(
    Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct SourceLocation {
    pub file_path: String,
    pub line_number: usize,
}

/// Compliance report mapping specification constraint groups to implementation anchors.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ComplianceReport {
    pub specification: ArtifactId,
    pub implementation: ArtifactId,
    pub scan_root: PathBuf,
    pub coverage: BTreeMap<String, Vec<ValidationTag>>,
    pub missing: Vec<String>,
    pub orphans: Vec<ValidationTag>,
}

/// Parses `[ENSURES: <id>:<type>]` tags from text content.
///
/// The `ENSURES` keyword is case-insensitive. The optional type is `TEST` (the default),
/// `CHECK`, or `MANUAL`.
pub fn parse_tags(content: &str, file_path: &Path) -> Vec<ValidationTag> {
    fn is_ensures_keyword(s: &str) -> bool {
        const KEYWORD: &str = "ENSURES:";
        s.get(..KEYWORD.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(KEYWORD))
    }

    let mut tags = Vec::new();
    for (line_index, line) in content.lines().enumerate() {
        let mut cursor = 0;
        while let Some(bracket_rel) = line[cursor..].find('[') {
            let bracket = cursor + bracket_rel;
            let after_bracket = &line[bracket + 1..];
            if !is_ensures_keyword(after_bracket) {
                cursor = bracket + 1;
                continue;
            }

            let Some(rel_end) = after_bracket.find(']') else {
                break;
            };
            let end = bracket + 1 + rel_end;
            let body = &after_bracket["ENSURES:".len()..rel_end];

            let mut parts = body.trim().split(':');
            let Some(identifier) = parts.next().map(str::trim) else {
                cursor = end + 1;
                continue;
            };
            let tag_type = match parts.next().map(str::trim) {
                Some("CHECK") => ValidationType::Check,
                Some("MANUAL") => ValidationType::Manual,
                Some("TEST") | None => ValidationType::Test,
                // Unrecognized types fall back to the default test type, matching the
                // specification's default-behavior rule while remaining permissive.
                Some(_) => ValidationType::Test,
            };

            tags.push(ValidationTag {
                identifier: identifier.to_string(),
                tag_type,
                location: SourceLocation {
                    file_path: file_path.to_string_lossy().to_string(),
                    line_number: line_index + 1,
                },
            });
            cursor = end + 1;
        }
    }
    tags
}

/// Recursively scans a source root for validation tags, respecting `.gitignore` rules,
/// skipping hidden files and directories, binary files, and the `target`/`.specman`/`.git`
/// directories.
pub fn scan_source_root(root: &Path) -> Result<Vec<ValidationTag>, SpecmanError> {
    let mut tags = Vec::new();

    let walker = ignore::WalkBuilder::new(root)
        .hidden(false)
        .ignore(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            name != "target" && name != ".specman" && name != ".git"
        })
        .build();

    for entry in walker {
        let entry = entry.map_err(|err| {
            SpecmanError::Workspace(format!("failed to scan {}: {err}", root.display()))
        })?;

        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }

        let path = entry.path();
        let bytes = fs::read(path).map_err(SpecmanError::Io)?;
        if is_binary(&bytes) {
            continue;
        }
        if let Ok(content) = String::from_utf8(bytes) {
            tags.extend(parse_tags(&content, path));
        }
    }

    Ok(tags)
}

/// Builds a compliance report from a set of parsed tags and known constraint groups.
pub fn generate_report(
    specification: ArtifactId,
    implementation: ArtifactId,
    scan_root: PathBuf,
    spec_constraints: &[String],
    tags: Vec<ValidationTag>,
) -> ComplianceReport {
    let mut coverage: BTreeMap<String, Vec<ValidationTag>> = BTreeMap::new();
    let mut orphans = Vec::new();

    for tag in tags {
        if spec_constraints.iter().any(|c| c == &tag.identifier) {
            coverage
                .entry(tag.identifier.clone())
                .or_default()
                .push(tag);
        } else {
            orphans.push(tag);
        }
    }

    let missing = spec_constraints
        .iter()
        .filter(|c| !coverage.contains_key(*c))
        .cloned()
        .collect();

    ComplianceReport {
        specification,
        implementation,
        scan_root,
        coverage,
        missing,
        orphans,
    }
}

/// Validates compliance by scanning the implementation source and matching tags against the
/// provided specification constraint groups.
pub fn validate_compliance(
    workspace: &WorkspacePaths,
    implementation: ArtifactId,
    specification: ArtifactId,
    spec_constraints: &[String],
) -> Result<ComplianceReport, SpecmanError> {
    let tags = scan_source_root(workspace.root())?;
    Ok(generate_report(
        specification,
        implementation,
        workspace.root().to_path_buf(),
        spec_constraints,
        tags,
    ))
}

/// Workspace status configuration.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceStatusConfig {
    pub structure: bool,
    pub references: bool,
    pub cycles: bool,
    pub compliance: bool,
}

impl Default for WorkspaceStatusConfig {
    fn default() -> Self {
        Self {
            structure: true,
            references: true,
            cycles: true,
            compliance: true,
        }
    }
}

/// Per-artifact validation results.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ArtifactStatus {
    pub structure_errors: Vec<String>,
    pub reference_errors: Vec<ReferenceValidationIssue>,
    pub compliance_missing: Vec<String>,
    pub compliance_orphans: Vec<ValidationTag>,
}

/// Aggregated workspace status report.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceStatusReport {
    pub global_status: StatusResult,
    pub spec_impl_status: StatusResult,
    pub artifacts: BTreeMap<ArtifactId, ArtifactStatus>,
    pub cycle_errors: Vec<String>,
    pub structure_errors: Vec<String>,
    pub artifact_count: usize,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum StatusResult {
    #[default]
    Pass,
    Fail,
}

/// Validates the current workspace project across the enabled validation categories.
pub fn validate_workspace_status(
    workspace: &WorkspacePaths,
    config: &WorkspaceStatusConfig,
) -> Result<WorkspaceStatusReport, SpecmanError> {
    let manifest_path = workspace.root().join("specman.json");
    let manifest = match load_manifest(&manifest_path) {
        Ok(manifest) => manifest,
        Err(err) => {
            return Ok(WorkspaceStatusReport {
                global_status: StatusResult::Fail,
                spec_impl_status: StatusResult::Fail,
                structure_errors: vec![err.to_string()],
                artifact_count: 1,
                ..Default::default()
            });
        }
    };

    let artifact_id = ArtifactId {
        kind: if manifest.is_spec() {
            ArtifactKind::Specification
        } else {
            ArtifactKind::Implementation
        },
        name: manifest.name().to_string(),
    };

    let mut artifact_status = ArtifactStatus::default();
    let mut cycle_errors = Vec::new();

    if config.references {
        if let Some(body) = project_body(workspace, &manifest).filter(|path| path.is_file()) {
            let options = ReferenceValidationOptions::default();
            match validate_references(workspace, &body, &options) {
                Ok(result) => artifact_status.reference_errors.extend(result.errors),
                Err(_) => artifact_status
                    .reference_errors
                    .push(unresolved_dependency_issue(&artifact_id, &body)),
            }
        }
    }

    if config.cycles {
        match detect_cycles(workspace, &artifact_id) {
            Ok(errors) => cycle_errors = errors,
            Err(_) => artifact_status
                .reference_errors
                .push(unresolved_dependency_issue(&artifact_id, &manifest_path)),
        }
    }

    if config.compliance && manifest.is_impl() {
        if let Ok(constraints) = governing_constraints(workspace, &manifest) {
            let tags = scan_source_root(workspace.root())?;
            let report = generate_report(
                artifact_id.clone(),
                artifact_id.clone(),
                workspace.root().to_path_buf(),
                &constraints,
                tags,
            );
            artifact_status.compliance_missing = report.missing;
            artifact_status.compliance_orphans = report.orphans;
        }
    }

    let has_errors = !artifact_status.structure_errors.is_empty()
        || !artifact_status.reference_errors.is_empty()
        || !cycle_errors.is_empty()
        || !artifact_status.compliance_missing.is_empty();

    let status = if has_errors {
        StatusResult::Fail
    } else {
        StatusResult::Pass
    };

    let mut artifacts = BTreeMap::new();
    artifacts.insert(artifact_id, artifact_status);

    Ok(WorkspaceStatusReport {
        global_status: status,
        spec_impl_status: status,
        artifacts,
        cycle_errors,
        structure_errors: Vec::new(),
        artifact_count: 1,
    })
}

fn project_body(workspace: &WorkspacePaths, manifest: &SpecManProject) -> Option<PathBuf> {
    match manifest.spec() {
        Some(spec) => spec
            .index
            .as_deref()
            .map(|index| workspace.root().join(index)),
        None => Some(workspace.root().join("impl.md")),
    }
}

fn unresolved_dependency_issue(
    artifact_id: &ArtifactId,
    destination: &Path,
) -> ReferenceValidationIssue {
    ReferenceValidationIssue {
        source_artifact: artifact_id.to_string(),
        destination: destination.to_string_lossy().to_string(),
        issue_type: ReferenceIssueType::UnresolvedDependency,
        line: None,
        column: None,
    }
}

fn detect_cycles(
    workspace: &WorkspacePaths,
    artifact_id: &ArtifactId,
) -> Result<Vec<String>, SpecmanError> {
    let mapper = FilesystemDependencyMapper::new(FixedWorkspaceLocator(workspace.clone()));
    match mapper.dependency_tree_for_artifact(artifact_id) {
        Ok(_) => Ok(Vec::new()),
        Err(SpecmanError::Dependency(message)) if message.contains("cycle") => Ok(vec![message]),
        Err(err) => Err(err),
    }
}

/// A `WorkspaceLocator` that returns a previously discovered workspace without re-scanning the
/// filesystem (which would re-enforce `.specman`/`.git` co-location rules).
struct FixedWorkspaceLocator(WorkspacePaths);

impl WorkspaceLocator for FixedWorkspaceLocator {
    fn workspace(&self) -> Result<WorkspacePaths, SpecmanError> {
        Ok(self.0.clone())
    }
}

/// Extracts constraint groups from the specifications listed in `impl.implements` only;
/// transitive specification dependencies are intentionally excluded.
fn governing_constraints(
    workspace: &WorkspacePaths,
    manifest: &SpecManProject,
) -> Result<Vec<String>, SpecmanError> {
    let Some(impl_) = manifest.impl_() else {
        return Ok(Vec::new());
    };

    let mut groups = Vec::new();
    for entry in &impl_.implements {
        let cache_dir = crate::install::ensure_reference(&entry.ref_, workspace)?;
        let spec_manifest = load_manifest(&cache_dir.join("specman.json"))?;
        if let Some(index) = spec_manifest.spec().and_then(|spec| spec.index.as_deref()) {
            let body = cache_dir.join(index);
            if body.is_file() {
                let content = fs::read_to_string(&body).map_err(SpecmanError::Io)?;
                groups.extend(crate::index::constraint_groups(&content));
            }
        }
    }

    groups.sort();
    groups.dedup();
    Ok(groups)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::parse_manifest;

    fn workspace(root: &Path) -> WorkspacePaths {
        WorkspacePaths::new(root.to_path_buf(), root.join(".specman"))
    }

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn parses_validation_tags() {
        let tags = parse_tags(
            "// [ENSURES: concept-a.formatting:TEST]\n// [ENSURES: concept-b.formatting:CHECK]\n",
            Path::new("lib.rs"),
        );
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].identifier, "concept-a.formatting");
        assert_eq!(tags[0].tag_type, ValidationType::Test);
        assert_eq!(tags[1].tag_type, ValidationType::Check);
        assert_eq!(tags[1].location.line_number, 2);
    }

    #[test]
    fn parses_case_insensitive_ensures_keyword() {
        let tags = parse_tags(
            "// [ensures: concept-a.formatting:MANUAL]\n// [EnSuReS: concept-b.formatting]\n",
            Path::new("lib.rs"),
        );
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].identifier, "concept-a.formatting");
        assert_eq!(tags[0].tag_type, ValidationType::Manual);
        // Omitted type defaults to TEST.
        assert_eq!(tags[1].tag_type, ValidationType::Test);
    }

    #[test]
    fn scanner_respects_gitignore() {
        let temp = tempfile::tempdir().unwrap();
        write(
            &temp.path().join("src/lib.rs"),
            "// [ENSURES: concept-a.formatting:TEST]\n",
        );
        write(
            &temp.path().join("ignored/generated.rs"),
            "// [ENSURES: concept-b.ordering:CHECK]\n",
        );
        write(&temp.path().join(".gitignore"), "/ignored\n");

        let tags = scan_source_root(temp.path()).unwrap();
        let ids: Vec<&str> = tags.iter().map(|t| t.identifier.as_str()).collect();
        assert_eq!(ids, vec!["concept-a.formatting"]);
    }

    #[test]
    fn reports_missing_and_orphans() {
        let spec = ArtifactId {
            kind: ArtifactKind::Specification,
            name: "core".to_string(),
        };
        let impl_ = ArtifactId {
            kind: ArtifactKind::Implementation,
            name: "lib".to_string(),
        };
        let tags = vec![ValidationTag {
            identifier: "concept-a.formatting".to_string(),
            tag_type: ValidationType::Test,
            location: SourceLocation {
                file_path: "lib.rs".to_string(),
                line_number: 1,
            },
        }];

        let report = generate_report(
            spec,
            impl_,
            PathBuf::from("/tmp"),
            &[
                "concept-a.formatting".to_string(),
                "concept-b.ordering".to_string(),
            ],
            tags,
        );

        assert_eq!(report.missing, vec!["concept-b.ordering"]);
        assert!(report.orphans.is_empty());
    }

    #[test]
    fn workspace_status_reports_structure_error() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = workspace(temp.path());
        write(&temp.path().join("specman.json"), "{ not valid json");

        let report =
            validate_workspace_status(&workspace, &WorkspaceStatusConfig::default()).unwrap();
        assert_eq!(report.global_status, StatusResult::Fail);
        assert!(!report.structure_errors.is_empty());
    }

    #[test]
    fn workspace_status_passes_for_spec_without_references() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = workspace(temp.path());
        write(
            &temp.path().join("specman.json"),
            r#"{ "name": "example", "spec": {} }"#,
        );

        let report =
            validate_workspace_status(&workspace, &WorkspaceStatusConfig::default()).unwrap();
        assert_eq!(report.global_status, StatusResult::Pass);
        assert_eq!(report.artifact_count, 1);
    }

    #[test]
    fn workspace_status_reports_broken_inline_links() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = workspace(temp.path());
        write(
            &temp.path().join("specman.json"),
            r#"{ "name": "example", "spec": { "index": "spec.md" } }"#,
        );
        write(
            &temp.path().join("spec.md"),
            "# Spec\n\nSee [gone](missing.md).\n",
        );

        let report =
            validate_workspace_status(&workspace, &WorkspaceStatusConfig::default()).unwrap();
        assert_eq!(report.global_status, StatusResult::Fail);
        let status = report
            .artifacts
            .values()
            .next()
            .expect("one artifact status");
        assert!(
            status
                .reference_errors
                .iter()
                .any(|e| e.issue_type == ReferenceIssueType::MissingFile)
        );
    }

    #[test]
    fn parses_manifest_projects() {
        let project = parse_manifest(r#"{ "name": "core", "spec": { "index": "spec.md" } }"#)
            .expect("valid manifest");
        assert!(project.is_spec());
    }
}
