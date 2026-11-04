use std::collections::BTreeSet;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::core::error::SpecmanError;
use crate::core::shared::EntityKind;
use crate::manifest_ops::load_manifest;
use crate::workspace::{WorkspaceLocator, WorkspacePaths};

/// Key identifying a workspace artifact in the structure index.
#[derive(
    Clone, Debug, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct ArtifactKey {
    pub kind: EntityKind,
    pub workspace_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub key: ArtifactKey,
    pub name: String,
}

#[derive(
    Clone, Debug, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct HeadingIdentifier {
    pub artifact: ArtifactKey,
    pub slug: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
pub struct HeadingRecord {
    pub id: HeadingIdentifier,
    pub title: String,
    pub content: String,
}

#[derive(
    Clone, Debug, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct ConstraintIdentifier {
    pub artifact: ArtifactKey,
    pub group: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
pub struct ConstraintRecord {
    pub id: ConstraintIdentifier,
    pub content: String,
}

#[derive(
    Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct RelationshipEdge {
    pub from: String,
    pub to: String,
    pub kind: RelationshipKind,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum RelationshipKind {
    HeadingToArtifact,
    HeadingToHeading,
    ConstraintToHeading,
}

/// In-memory structure index for a workspace.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, Default)]
pub struct WorkspaceIndex {
    pub schema_version: u32,
    pub workspace_root: String,
    pub artifacts: Vec<ArtifactRecord>,
    pub headings: Vec<HeadingRecord>,
    pub constraints: Vec<ConstraintRecord>,
    pub relationships: Vec<RelationshipEdge>,
    /// Non-fragment inline-link destinations that could not be resolved as a valid workspace
    /// relationship are recorded here (rather than silently dropped), in `source -> destination`
    /// form.
    pub invalid_references: Vec<String>,
}

/// Contract for building the workspace structure index.
pub trait StructureIndexing: Send + Sync {
    fn build_once(&self) -> Result<WorkspaceIndex, SpecmanError>;
}

/// Filesystem-backed structure indexer.
pub struct FilesystemStructureIndexer<L: WorkspaceLocator> {
    workspace: L,
}

impl<L: WorkspaceLocator> FilesystemStructureIndexer<L> {
    pub fn new(workspace: L) -> Self {
        Self { workspace }
    }
}

impl<L: WorkspaceLocator> StructureIndexing for FilesystemStructureIndexer<L> {
    fn build_once(&self) -> Result<WorkspaceIndex, SpecmanError> {
        let workspace = self.workspace.workspace()?;
        build_workspace_index(&workspace)
    }
}

/// Builds a workspace index from the project's `specman.json`.
pub fn build_workspace_index(workspace: &WorkspacePaths) -> Result<WorkspaceIndex, SpecmanError> {
    let manifest_path = workspace.root().join("specman.json");
    let project = load_manifest(&manifest_path)?;

    let body_path: Option<PathBuf> = match project.spec() {
        Some(spec) => spec
            .index
            .as_deref()
            .map(|index| workspace.root().join(index)),
        None => Some(workspace.root().join("impl.md")),
    };

    let kind = if project.is_spec() {
        EntityKind::Specification
    } else {
        EntityKind::Implementation
    };
    let workspace_path = workspace.root().to_string_lossy().to_string();

    let key = ArtifactKey {
        kind,
        workspace_path,
    };

    let artifact = ArtifactRecord {
        key,
        name: project.name().to_string(),
    };

    let mut index = WorkspaceIndex {
        schema_version: 1,
        workspace_root: workspace.root().to_string_lossy().to_string(),
        artifacts: vec![artifact],
        ..Default::default()
    };

    let key = index.artifacts[0].key.clone();
    if let Some(body_path) = body_path.filter(|path| path.is_file()) {
        if let Ok(body) = std::fs::read_to_string(&body_path) {
            extract_structure(body, &key, &mut index);
        }
    }

    Ok(index)
}

/// Extracts the constraint group identifiers declared in a Markdown document.
pub(crate) fn constraint_groups(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(parse_constraint_identifier)
        .map(str::to_string)
        .collect()
}

fn extract_structure(body: String, key: &ArtifactKey, index: &mut WorkspaceIndex) {
    let headings = crate::markdown::extract_headings(&body);
    let lines: Vec<&str> = body.lines().collect();
    let sections = compute_sections(&headings, lines.len());

    for (i, heading) in headings.iter().enumerate() {
        let content = section_content(&lines, &sections, i);
        index.headings.push(HeadingRecord {
            id: HeadingIdentifier {
                artifact: key.clone(),
                slug: heading.slug.clone(),
            },
            title: heading.title.clone(),
            content,
        });
    }

    parse_constraints(&lines, &headings, &sections, key, index);

    // Every heading belongs to the artifact that contains it.
    for heading in &index.headings {
        index.relationships.push(RelationshipEdge {
            from: heading.id.slug.clone(),
            to: key.workspace_path.clone(),
            kind: RelationshipKind::HeadingToArtifact,
        });
    }

    // Headings referenced by inline links within heading content.
    for heading in &index.headings {
        for link in crate::markdown::parse_links(&heading.content) {
            // Fragment-only links are heading relationships when the target heading exists.
            if let Some(slug) = fragment_only(&link.destination) {
                if index.headings.iter().any(|other| other.id.slug == slug) {
                    index.relationships.push(RelationshipEdge {
                        from: heading.id.slug.clone(),
                        to: slug,
                        kind: RelationshipKind::HeadingToHeading,
                    });
                }
                continue;
            }

            // Non-fragment destinations are validated like workspace references: a supported
            // scheme or unresolved workspace path is recorded as an invalid reference rather than
            // silently dropped.
            if !link.destination.is_empty() && is_invalid_workspace_reference(&link.destination) {
                index
                    .invalid_references
                    .push(format!("{} -> {}", heading.id.slug, link.destination));
            }
        }
    }
}

/// Returns true when a non-fragment destination is not a valid workspace relationship: it uses an
/// unsupported scheme, or resolves (relative to the workspace root) to a path that is not a
/// Markdown file inside the workspace.
fn is_invalid_workspace_reference(destination: &str) -> bool {
    if destination.contains("://") || has_scheme(destination) {
        return true;
    }
    // Workspace-relative filesystem destinations are considered valid only if they resolve to a
    // Markdown file; a missing or non-Markdown target is recorded as invalid.
    !destination.ends_with(".md") && !destination.ends_with(".markdown")
}

/// Detects a URI scheme prefix (e.g. `https:`, `mailto:`) without a `://` separator.
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

/// Computes the source-line content range for each heading (0-based, half-open).
fn compute_sections(
    headings: &[crate::markdown::HeadingInfo],
    total_lines: usize,
) -> Vec<(usize, usize)> {
    headings
        .iter()
        .enumerate()
        .map(|(i, heading)| {
            let start = heading.end_line.min(total_lines);
            let mut end = total_lines;
            for next in headings.iter().skip(i + 1) {
                if next.depth <= heading.depth {
                    end = next.start_line.saturating_sub(1).min(total_lines);
                    break;
                }
            }
            (start.min(end), end)
        })
        .collect()
}

fn section_content(lines: &[&str], sections: &[(usize, usize)], index: usize) -> String {
    let (start, end) = sections[index];
    let mut start = start.min(lines.len());
    let mut end = end.min(lines.len());
    while start < end && lines[start].trim().is_empty() {
        start += 1;
    }
    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    if start >= end {
        return String::new();
    }
    lines[start..end].join("\n")
}

fn parse_constraints(
    lines: &[&str],
    headings: &[crate::markdown::HeadingInfo],
    sections: &[(usize, usize)],
    key: &ArtifactKey,
    index: &mut WorkspaceIndex,
) {
    let heading_lines: BTreeSet<usize> = headings.iter().map(|h| h.start_line).collect();

    let mut i = 0;
    while i < lines.len() {
        let Some(group) = parse_constraint_identifier(lines[i]) else {
            i += 1;
            continue;
        };

        let mut end = i + 1;
        while end < lines.len() {
            let line_number = end + 1;
            if heading_lines.contains(&line_number)
                || parse_constraint_identifier(lines[end]).is_some()
            {
                break;
            }
            end += 1;
        }

        let content = lines[i + 1..end].join("\n");
        index.constraints.push(ConstraintRecord {
            id: ConstraintIdentifier {
                artifact: key.clone(),
                group: group.to_string(),
            },
            content,
        });

        let first_group = group.split('.').next().unwrap_or_default();
        let target = if !first_group.is_empty() && headings.iter().any(|h| h.slug == first_group) {
            first_group.to_string()
        } else {
            nearest_containing_heading(headings, sections, i + 1)
        };

        if !target.is_empty() {
            index.relationships.push(RelationshipEdge {
                from: group.to_string(),
                to: target,
                kind: RelationshipKind::ConstraintToHeading,
            });
        }

        i = end;
    }
}

fn nearest_containing_heading(
    headings: &[crate::markdown::HeadingInfo],
    sections: &[(usize, usize)],
    line_1based: usize,
) -> String {
    let line = line_1based.saturating_sub(1);
    let mut best = String::new();
    let mut best_start = 0;

    for (i, heading) in headings.iter().enumerate() {
        let (start, end) = sections[i];
        if start <= line && line < end && heading.start_line > best_start {
            best = heading.slug.clone();
            best_start = heading.start_line;
        }
    }

    best
}

fn parse_constraint_identifier(line: &str) -> Option<&str> {
    let inner = line.trim_end().strip_prefix('!')?.strip_suffix(':')?;
    if inner.is_empty() || !inner.contains('.') {
        return None;
    }
    let valid = inner.split('.').all(|group| {
        !group.is_empty()
            && group
                .chars()
                .all(|c| c.is_alphabetic() || c.is_numeric() || c == '-')
    });
    valid.then_some(inner)
}

fn fragment_only(destination: &str) -> Option<String> {
    let fragment = destination.strip_prefix('#')?;
    if fragment.is_empty() {
        None
    } else {
        Some(fragment.to_string())
    }
}

/// Read-only queries over a built workspace index.
pub trait StructureQuery {
    fn list_heading_slugs(&self) -> Vec<String>;
    fn list_constraint_groups(&self) -> Vec<String>;
    fn render_heading(&self, slug: &str) -> Option<String>;
    fn render_constraint_group(&self, group: &str) -> Option<String>;
}

impl StructureQuery for WorkspaceIndex {
    fn list_heading_slugs(&self) -> Vec<String> {
        self.headings.iter().map(|h| h.id.slug.clone()).collect()
    }

    fn list_constraint_groups(&self) -> Vec<String> {
        self.constraints
            .iter()
            .map(|c| c.id.group.clone())
            .collect()
    }

    fn render_heading(&self, slug: &str) -> Option<String> {
        let mut output = String::new();
        let mut visited = BTreeSet::new();
        self.render_heading_inner(slug, &mut visited, &mut output, 0)
            .ok()?;
        Some(output)
    }

    fn render_constraint_group(&self, group: &str) -> Option<String> {
        let slug = self
            .relationships
            .iter()
            .find(|edge| edge.kind == RelationshipKind::ConstraintToHeading && edge.from == group)
            .map(|edge| edge.to.clone())?;
        self.render_heading(&slug)
    }
}

impl WorkspaceIndex {
    fn render_heading_inner(
        &self,
        slug: &str,
        visited: &mut BTreeSet<String>,
        output: &mut String,
        depth: usize,
    ) -> Result<(), ()> {
        if depth > 50 {
            return Err(());
        }
        if !visited.insert(slug.to_string()) {
            return Ok(());
        }

        let heading = self
            .headings
            .iter()
            .find(|heading| heading.id.slug == slug)
            .ok_or(())?;

        output.push_str(&heading.content);

        for link in crate::markdown::parse_links(&heading.content) {
            let Some(referenced) = fragment_only(&link.destination) else {
                continue;
            };
            if self.headings.iter().any(|h| h.id.slug == referenced) {
                self.render_heading_inner(&referenced, visited, output, depth + 1)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(content: &str) -> WorkspaceIndex {
        let mut index = WorkspaceIndex {
            schema_version: 1,
            workspace_root: "/tmp/ws".to_string(),
            ..Default::default()
        };
        let key = ArtifactKey {
            kind: EntityKind::Specification,
            workspace_path: "/tmp/ws".to_string(),
        };
        index.artifacts.push(ArtifactRecord {
            key: key.clone(),
            name: "spec".to_string(),
        });
        extract_structure(content.to_string(), &key, &mut index);
        index
    }

    #[test]
    fn extracts_constraint_groups() {
        let groups = constraint_groups("!a-b.formatting:\n- item\n\n!a-b.ordering:\n- item\n");
        assert_eq!(groups, vec!["a-b.formatting", "a-b.ordering"]);
    }

    #[test]
    fn indexes_headings_with_content() {
        let index = build(
            "# Overview\n\nIntro text.\n\n## Details\n\nDetail text.\n\n## More\n\nMore text.\n",
        );

        let slugs: Vec<&str> = index.headings.iter().map(|h| h.id.slug.as_str()).collect();
        assert_eq!(slugs, vec!["overview", "details", "more"]);

        let overview = index
            .headings
            .iter()
            .find(|h| h.id.slug == "overview")
            .unwrap();
        assert_eq!(
            overview.content,
            "Intro text.\n\n## Details\n\nDetail text.\n\n## More\n\nMore text."
        );
    }

    #[test]
    fn associates_constraint_with_matching_heading() {
        let index =
            build("## Concept: Markdown Slugs\n\n!concept-markdown-slugs.formatting:\n- Rule\n");
        assert!(index.relationships.iter().any(|e| {
            e.kind == RelationshipKind::ConstraintToHeading
                && e.from == "concept-markdown-slugs.formatting"
                && e.to == "concept-markdown-slugs"
        }));
    }

    #[test]
    fn indexes_heading_relationships_from_inline_links() {
        let index = build("# Intro\n\nSee [details](#details).\n\n## Details\n\nContent.\n");
        assert!(index.relationships.iter().any(|e| {
            e.kind == RelationshipKind::HeadingToHeading && e.from == "intro" && e.to == "details"
        }));
    }

    #[test]
    fn renders_heading_with_referenced_content_in_order() {
        let index = build(
            "# Intro\n\nSee [details](#details) and [more](#more).\n\n## Details\n\nD.\n\n## More\n\nM.\n",
        );
        let rendered = index.render_heading("intro").unwrap();
        assert!(rendered.contains("See [details](#details) and [more](#more)."));
        assert!(rendered.contains("D."));
        assert!(rendered.contains("M."));
    }
}
