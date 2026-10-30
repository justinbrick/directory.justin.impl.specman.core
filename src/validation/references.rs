//! Reference validation for Markdown artifacts.
//!
//! Implements [Concept: Reference Validation] from the SpecMan Core specification:
//! discover inline-link destinations, classify them (workspace filesystem, HTTPS,
//! or unsupported), and validate them against the workspace boundary and heading
//! slugs without mutating the source artifact.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::core::error::SpecmanError;
use crate::workspace::WorkspacePaths;

/// Validation mode for HTTPS destinations.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HttpsValidationMode {
    /// Only validate that the destination is a well-formed HTTPS URL.
    #[default]
    CheckSyntax,
    /// Additionally perform a reachability check (a `GET`/redirect-following request).
    CheckReachability,
}

/// HTTPS validation options.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct HttpsValidationOptions {
    pub mode: HttpsValidationMode,
}

impl Default for HttpsValidationOptions {
    fn default() -> Self {
        Self {
            mode: HttpsValidationMode::CheckSyntax,
        }
    }
}

/// Transitive traversal options.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct TransitiveValidationOptions {
    /// Whether to validate linked documents beyond the source artifact.
    pub enabled: bool,
    /// Maximum traversal depth/count for transitive validation.
    pub max_documents: usize,
}

impl Default for TransitiveValidationOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            max_documents: 0,
        }
    }
}

/// Configuration for the reference-validation capability.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ReferenceValidationOptions {
    pub https: HttpsValidationOptions,
    pub transitive: TransitiveValidationOptions,
}

impl Default for ReferenceValidationOptions {
    fn default() -> Self {
        Self {
            https: HttpsValidationOptions::default(),
            transitive: TransitiveValidationOptions::default(),
        }
    }
}

/// Classification of a discovered link destination.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum ReferenceDestinationKind {
    /// A filesystem path that resolves inside the active workspace.
    WorkspaceFilesystem,
    /// An HTTPS URL.
    Https,
    /// A destination using a scheme outside the supported set (or unknown).
    Unsupported,
}

/// A discovered link destination with source-range context.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DiscoveredReference {
    pub destination: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub destination_kind: ReferenceDestinationKind,
}

/// The failure category of a single reference-validation error.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum ReferenceIssueType {
    /// Destination uses a scheme outside the supported set.
    UnsupportedScheme,
    /// Resolved filesystem destination escapes the workspace root.
    WorkspaceBoundaryViolation,
    /// Filesystem destination does not point to an existing file.
    MissingFile,
    /// HTTPS destination is not a well-formed HTTPS URL.
    InvalidHttpsUrl,
    /// HTTPS destination could not be reached (non-timeout failure).
    UnreachableHttps,
    /// Fragment does not match an existing heading slug in the target document.
    UnknownFragment,
    /// A declared dependency could not be resolved (for example, a git reference).
    UnresolvedDependency,
}

/// A single reference-validation failure with enough context for a helpful message.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ReferenceValidationIssue {
    pub source_artifact: String,
    pub destination: String,
    pub issue_type: ReferenceIssueType,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

/// Structured result of validating the references in a Markdown artifact.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReferenceValidationResult {
    pub references: Vec<DiscoveredReference>,
    pub errors: Vec<ReferenceValidationIssue>,
    /// Non-fatal diagnostics (for example, HTTPS reachability timeouts).
    pub warnings: Vec<ReferenceValidationIssue>,
    pub status: crate::validation::StatusResult,
}

impl ReferenceValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validates the references in a Markdown artifact located on disk.
pub fn validate_references(
    workspace: &WorkspacePaths,
    source_path: &Path,
    options: &ReferenceValidationOptions,
) -> Result<ReferenceValidationResult, SpecmanError> {
    let content = std::fs::read_to_string(source_path).map_err(SpecmanError::Io)?;
    Ok(validate_reference_content(
        workspace,
        source_path,
        &content,
        options,
    ))
}

/// Validates the references found in already-loaded Markdown content.
///
/// `source_path` MUST be the absolute path of the artifact being validated so that
/// filesystem destinations resolve relative to its directory and fragment-only
/// links resolve against the source document itself.
pub fn validate_reference_content(
    workspace: &WorkspacePaths,
    source_path: &Path,
    content: &str,
    options: &ReferenceValidationOptions,
) -> ReferenceValidationResult {
    let mut references = Vec::new();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut visited = BTreeSet::new();

    validate_reference_content_inner(
        workspace,
        source_path,
        content,
        options,
        &mut references,
        &mut errors,
        &mut warnings,
        &mut visited,
    );

    let status = if errors.is_empty() {
        crate::validation::StatusResult::Pass
    } else {
        crate::validation::StatusResult::Fail
    };

    ReferenceValidationResult {
        references,
        errors,
        warnings,
        status,
    }
}

fn validate_reference_content_inner(
    workspace: &WorkspacePaths,
    source_path: &Path,
    content: &str,
    options: &ReferenceValidationOptions,
    references: &mut Vec<DiscoveredReference>,
    errors: &mut Vec<ReferenceValidationIssue>,
    warnings: &mut Vec<ReferenceValidationIssue>,
    visited: &mut BTreeSet<PathBuf>,
) {
    let root = workspace.root();
    let source_dir = source_path.parent().unwrap_or(root);

    for link in crate::markdown::parse_links(content) {
        let destination = link.destination.as_str();
        let destination_kind = classify_destination(destination);
        references.push(DiscoveredReference {
            destination: destination.to_string(),
            line: Some(link.line),
            column: Some(link.column),
            destination_kind,
        });

        match validate_destination(destination, source_path, source_dir, root, options) {
            Ok(()) => {
                // When transitive validation is enabled, follow workspace Markdown links.
                if options.transitive.enabled
                    && destination_kind == ReferenceDestinationKind::WorkspaceFilesystem
                {
                    if let Some(target) = resolve_filesystem_link(destination, source_dir, root) {
                        follow_transitive(
                            workspace, &target, options, references, errors, warnings, visited,
                        );
                    }
                }
            }
            Err((issue_type, non_fatal)) => {
                let issue = ReferenceValidationIssue {
                    source_artifact: source_path.to_string_lossy().to_string(),
                    destination: destination.to_string(),
                    issue_type,
                    line: Some(link.line),
                    column: Some(link.column),
                };
                if non_fatal {
                    warnings.push(issue);
                } else {
                    errors.push(issue);
                }
            }
        }
    }
}

/// Resolves a fragment-less workspace filesystem link to an absolute target path (if it exists
/// inside the workspace and is a Markdown file).
fn resolve_filesystem_link(destination: &str, source_dir: &Path, root: &Path) -> Option<PathBuf> {
    // Strip any fragment component (`path.md#heading`).
    let path_part = destination.split('#').next().unwrap_or(destination);
    if path_part.is_empty() {
        return None;
    }
    let candidate = source_dir.join(path_part);
    if !candidate.starts_with(root) {
        return None;
    }
    if candidate.extension().and_then(|e| e.to_str()) != Some("md") {
        return None;
    }
    if candidate.is_file() {
        Some(candidate)
    } else {
        None
    }
}

/// Recursively validates a linked Markdown document, bounded by `max_documents` and protected
/// against cycles.
#[allow(clippy::too_many_arguments)]
fn follow_transitive(
    workspace: &WorkspacePaths,
    target: &Path,
    options: &ReferenceValidationOptions,
    references: &mut Vec<DiscoveredReference>,
    errors: &mut Vec<ReferenceValidationIssue>,
    warnings: &mut Vec<ReferenceValidationIssue>,
    visited: &mut BTreeSet<PathBuf>,
) {
    // `max_documents == 0` is treated as "no limit" only when transitive is enabled with an
    // explicit non-zero cap; otherwise stop once we've already visited the cap.
    if options.transitive.max_documents > 0 && visited.len() >= options.transitive.max_documents {
        return;
    }
    if !visited.insert(target.to_path_buf()) {
        return;
    }

    let content = match std::fs::read_to_string(target) {
        Ok(content) => content,
        Err(_) => return,
    };

    validate_reference_content_inner(
        workspace, target, &content, options, references, errors, warnings, visited,
    );
}

fn classify_destination(destination: &str) -> ReferenceDestinationKind {
    if destination.starts_with("https://") {
        return ReferenceDestinationKind::Https;
    }
    if destination.contains("://") || has_scheme(destination) {
        return ReferenceDestinationKind::Unsupported;
    }
    ReferenceDestinationKind::WorkspaceFilesystem
}

fn validate_destination(
    destination: &str,
    source_path: &Path,
    source_dir: &Path,
    root: &Path,
    options: &ReferenceValidationOptions,
) -> Result<(), (ReferenceIssueType, bool)> {
    if destination.starts_with("https://") {
        return validate_https(destination, options);
    }
    if destination.contains("://") || has_scheme(destination) {
        return Err((ReferenceIssueType::UnsupportedScheme, false));
    }

    let (path_part, fragment) = split_fragment(destination);
    let target_path = if path_part.is_empty() {
        source_path.to_path_buf()
    } else {
        crate::workspace::normalize_workspace_path(&source_dir.join(path_part))
    };

    // Enforce the workspace boundary using the lexically-normalized path.
    if !target_path.starts_with(root) {
        return Err((ReferenceIssueType::WorkspaceBoundaryViolation, false));
    }

    // Re-check the boundary against the canonicalized path when possible.
    if let Ok(canonical) = std::fs::canonicalize(&target_path) {
        if !canonical.starts_with(root) {
            return Err((ReferenceIssueType::WorkspaceBoundaryViolation, false));
        }
    }

    if !target_path.is_file() {
        return Err((ReferenceIssueType::MissingFile, false));
    }

    if let Some(fragment) = fragment {
        if is_markdown(&target_path) && !heading_exists(&target_path, &fragment) {
            return Err((ReferenceIssueType::UnknownFragment, false));
        }
    }

    Ok(())
}

fn validate_https(
    destination: &str,
    options: &ReferenceValidationOptions,
) -> Result<(), (ReferenceIssueType, bool)> {
    let url =
        url::Url::parse(destination).map_err(|_| (ReferenceIssueType::InvalidHttpsUrl, false))?;
    if url.scheme() != "https" {
        return Err((ReferenceIssueType::InvalidHttpsUrl, false));
    }

    if options.https.mode == HttpsValidationMode::CheckSyntax {
        return Ok(());
    }

    match reachability(&url) {
        Reachability::Ok | Reachability::Redirect => Ok(()),
        Reachability::Timeout => Err((ReferenceIssueType::UnreachableHttps, true)),
        Reachability::Error => Err((ReferenceIssueType::UnreachableHttps, false)),
    }
}

enum Reachability {
    Ok,
    Redirect,
    Timeout,
    Error,
}

fn reachability(url: &url::Url) -> Reachability {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .redirects(5)
        .build();

    match agent.get(url.as_str()).call() {
        Ok(_) => Reachability::Ok,
        Err(ureq::Error::Status(code, _)) => {
            if (300..400).contains(&code) {
                Reachability::Redirect
            } else {
                Reachability::Error
            }
        }
        Err(ureq::Error::Transport(transport)) => {
            if is_timeout(&transport) {
                Reachability::Timeout
            } else {
                Reachability::Error
            }
        }
    }
}

fn is_timeout(transport: &ureq::Transport) -> bool {
    if transport.kind() != ureq::ErrorKind::Io {
        return false;
    }
    std::error::Error::source(transport)
        .and_then(|source| source.downcast_ref::<std::io::Error>())
        .map(|io| {
            matches!(
                io.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            )
        })
        .unwrap_or(false)
}

fn has_scheme(destination: &str) -> bool {
    match destination.find(':') {
        Some(index) => {
            let scheme = &destination[..index];
            !scheme.is_empty()
                && scheme
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic())
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
        }
        None => false,
    }
}

fn split_fragment(destination: &str) -> (&str, Option<String>) {
    match destination.find('#') {
        Some(index) => {
            let fragment = &destination[index + 1..];
            if fragment.is_empty() {
                (&destination[..index], None)
            } else {
                (&destination[..index], Some(fragment.to_string()))
            }
        }
        None => (destination, None),
    }
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
}

fn heading_exists(path: &Path, fragment: &str) -> bool {
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    crate::markdown::extract_headings(&content)
        .iter()
        .any(|heading| heading.slug == fragment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::WorkspacePaths;

    fn workspace(temp: &tempfile::TempDir) -> WorkspacePaths {
        WorkspacePaths::new(temp.path().to_path_buf(), temp.path().join(".specman"))
    }

    fn write(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn validates_workspace_links_and_fragments() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = workspace(&temp);
        let root = workspace.root().to_path_buf();

        write(
            &root.join("guide.md"),
            "# Guide\n\n## Install\n\nSome content.\n",
        );
        write(
            &root.join("spec.md"),
            "See [guide](guide.md) and [install](guide.md#install).\n",
        );

        let result = validate_references(
            &workspace,
            &root.join("spec.md"),
            &ReferenceValidationOptions::default(),
        )
        .unwrap();
        assert!(result.is_valid());
        assert_eq!(result.references.len(), 2);
    }

    #[test]
    fn rejects_missing_file_and_unknown_fragment() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = workspace(&temp);
        let root = workspace.root().to_path_buf();

        write(
            &root.join("spec.md"),
            "[gone](missing.md) [bad](guide.md#nope).\n",
        );
        write(&root.join("guide.md"), "# Guide\n");

        let result = validate_references(
            &workspace,
            &root.join("spec.md"),
            &ReferenceValidationOptions::default(),
        )
        .unwrap();

        assert!(!result.is_valid());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.issue_type == ReferenceIssueType::MissingFile)
        );
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.issue_type == ReferenceIssueType::UnknownFragment)
        );
    }

    #[test]
    fn rejects_workspace_escape() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = workspace(&temp);
        let root = workspace.root().to_path_buf();

        write(&root.join("spec.md"), "[outside](../outside.md).\n");

        let result = validate_references(
            &workspace,
            &root.join("spec.md"),
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
    fn rejects_unsupported_scheme() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = workspace(&temp);
        let root = workspace.root().to_path_buf();

        write(&root.join("spec.md"), "[mail](mailto:test@example.com).\n");

        let result = validate_references(
            &workspace,
            &root.join("spec.md"),
            &ReferenceValidationOptions::default(),
        )
        .unwrap();

        assert!(
            result
                .errors
                .iter()
                .any(|e| e.issue_type == ReferenceIssueType::UnsupportedScheme)
        );
    }

    #[test]
    fn accepts_https_without_reachability() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = workspace(&temp);
        let root = workspace.root().to_path_buf();

        write(
            &root.join("spec.md"),
            "[spec](https://example.com/guide.md).\n",
        );

        let result = validate_references(
            &workspace,
            &root.join("spec.md"),
            &ReferenceValidationOptions::default(),
        )
        .unwrap();

        assert!(result.is_valid());
    }

    #[test]
    fn transitive_validation_follows_linked_documents() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = workspace(&temp);
        let root = workspace.root().to_path_buf();

        write(&root.join("a.md"), "[b](b.md).\n");
        write(&root.join("b.md"), "[missing](gone.md).\n");

        let mut options = ReferenceValidationOptions::default();
        options.transitive.enabled = true;
        options.transitive.max_documents = 10;

        let result = validate_references(&workspace, &root.join("a.md"), &options).unwrap();

        // The broken link in the transitively reached `b.md` must be reported.
        assert!(!result.is_valid());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.issue_type == ReferenceIssueType::MissingFile
                    && e.destination == "gone.md")
        );
    }

    #[test]
    fn transitive_validation_disabled_does_not_follow() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = workspace(&temp);
        let root = workspace.root().to_path_buf();

        write(&root.join("a.md"), "[b](b.md).\n");
        write(&root.join("b.md"), "[missing](gone.md).\n");

        // Default options have transitive disabled.
        let result = validate_references(
            &workspace,
            &root.join("a.md"),
            &ReferenceValidationOptions::default(),
        )
        .unwrap();

        // `a.md`'s own link to `b.md` is valid, so the result passes; `b.md` is not traversed.
        assert!(result.is_valid());
    }
}
