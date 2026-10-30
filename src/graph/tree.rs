use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::core::error::SpecmanError;
use crate::install;
use crate::manifest::{ImplProject, SpecManProject, SpecProject, load_manifest};
use crate::workspace::{WorkspaceLocator, WorkspacePaths};

/// Identity for any SpecMan artifact (specification or implementation).
#[derive(
    Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
)]
pub struct ArtifactId {
    pub kind: ArtifactKind,
    pub name: String,
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}://{}", self.kind, self.name)
    }
}

/// Artifact kind segmentation.
#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
)]
pub enum ArtifactKind {
    #[default]
    Specification,
    Implementation,
}

/// Lightweight summary of an artifact discovered while traversing dependencies.
#[derive(
    Clone, Debug, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct ArtifactSummary {
    pub id: ArtifactId,
    pub metadata: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_path: Option<String>,
}

/// Directed edge between two artifacts.
#[derive(
    Clone, Debug, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct DependencyEdge {
    pub from: ArtifactSummary,
    pub to: ArtifactSummary,
    pub relation: DependencyRelation,
    pub optional: bool,
}

/// Relationship classification for dependency edges.
#[derive(
    Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
)]
pub enum DependencyRelation {
    #[default]
    Upstream,
    Downstream,
}

/// Aggregated dependency data across upstream, downstream, and combined views.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, Default)]
pub struct DependencyTree {
    // [ENSURES: entity-dependencytree.requirements:CHECK]
    pub root: ArtifactSummary,
    pub upstream: Vec<DependencyEdge>,
    pub downstream: Vec<DependencyEdge>,
    pub aggregate: Vec<DependencyEdge>,
}

impl DependencyTree {
    pub fn empty(root: ArtifactSummary) -> Self {
        Self {
            root,
            upstream: Vec::new(),
            downstream: Vec::new(),
            aggregate: Vec::new(),
        }
    }

    /// Returns true when the dependency tree contains downstream artifacts that should block
    /// deletion of the root artifact.
    pub fn has_blocking_dependents(&self) -> bool {
        self.downstream.iter().any(|edge| !edge.optional)
    }
}

/// Contract for dependency traversal services.
pub trait DependencyMapping: Send + Sync {
    fn dependency_tree(&self, root: &ArtifactId) -> Result<DependencyTree, SpecmanError>;
    fn upstream(&self, root: &ArtifactId) -> Result<Vec<DependencyEdge>, SpecmanError>;
    fn downstream(&self, root: &ArtifactId) -> Result<Vec<DependencyEdge>, SpecmanError>;
}

/// Filesystem-backed dependency mapper that traverses `specman.json` projects and resolves
/// git references into the in-tree `spec/{alias}/` materialization.
pub struct FilesystemDependencyMapper<L: WorkspaceLocator> {
    workspace: L,
}

/// A locator used to identify the root of a dependency-tree traversal.
#[derive(Clone, Debug)]
pub enum DependencyLocator {
    /// A path to a `specman.json` manifest.
    Path(PathBuf),
    /// A git reference, materialized (or cloned) under `spec/{alias}/`.
    GitReference(crate::manifest::GitReference),
    /// An alias resolved through the project lockfile to a nested `spec/...` path.
    Alias(String),
}

impl<L: WorkspaceLocator> FilesystemDependencyMapper<L> {
    pub fn new(workspace: L) -> Self {
        Self { workspace }
    }

    /// Builds a dependency tree by reading the `specman.json` located at the provided path.
    pub fn dependency_tree_from_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<DependencyTree, SpecmanError> {
        let workspace = self.workspace_paths()?;
        let project = load_manifest(path.as_ref())?;
        self.build_tree(project, workspace)
    }

    /// Builds a dependency tree from a normalized locator (a manifest path, a git reference, or
    /// an alias resolved through the lockfile).
    pub fn dependency_tree_from_locator(
        &self,
        locator: DependencyLocator,
    ) -> Result<DependencyTree, SpecmanError> {
        match locator {
            DependencyLocator::Path(path) => self.dependency_tree_from_path(path),
            DependencyLocator::GitReference(reference) => {
                let workspace = self.workspace_paths()?;
                let dir = crate::install::ensure_reference(&reference, &workspace)?;
                self.dependency_tree_from_path(dir.join("specman.json"))
            }
            DependencyLocator::Alias(alias) => {
                let workspace = self.workspace_paths()?;
                let path = self.resolve_alias(&workspace, &alias)?;
                self.dependency_tree_from_path(path)
            }
        }
    }

    /// Resolves an alias to its `specman.json` path via the lockfile (falling back to a direct
    /// `spec/{alias}/specman.json` if no lockfile entry exists).
    fn resolve_alias(
        &self,
        workspace: &WorkspacePaths,
        alias: &str,
    ) -> Result<PathBuf, SpecmanError> {
        if let Some(lock) = crate::install::load_lockfile(workspace)? {
            if let Some(entry) = lock.references.iter().find(|e| e.alias == alias) {
                return Ok(workspace.root().join(&entry.path).join("specman.json"));
            }
        }
        Ok(workspace
            .root()
            .join("spec")
            .join(alias)
            .join("specman.json"))
    }

    fn workspace_paths(&self) -> Result<WorkspacePaths, SpecmanError> {
        self.workspace.workspace()
    }

    /// Builds a dependency tree rooted at the current workspace project.
    pub fn dependency_tree_for_artifact(
        &self,
        _root: &ArtifactId,
    ) -> Result<DependencyTree, SpecmanError> {
        let workspace = self.workspace_paths()?;
        let manifest_path = workspace.root().join("specman.json");
        let project = load_manifest(&manifest_path)?;
        self.build_tree(project, workspace)
    }

    fn build_tree(
        &self,
        project: SpecManProject,
        workspace: WorkspacePaths,
    ) -> Result<DependencyTree, SpecmanError> {
        let root = summarize_project(&project, Some(workspace.root().to_path_buf()));
        let mut edges = Vec::new();
        let mut visited: BTreeSet<String> = BTreeSet::new();
        visited.insert(root.id.to_string());

        for (reference, _kind) in references_of(&project) {
            let mut cache_dir = install::ensure_reference(&reference, &workspace)?;
            cache_dir = cache_dir.join("specman.json");
            let nested = load_manifest(&cache_dir)
                .map_err(|err| SpecmanError::Dependency(format!("{cache_dir:?}: {err}")))?;
            let nested_root = summarize_project(&nested, Some(cache_dir.clone()));
            self.visit(
                &nested,
                &nested_root,
                &root,
                workspace.clone(),
                &mut edges,
                &mut visited,
            )?;
        }

        edges.sort();
        edges.dedup();
        let upstream: Vec<DependencyEdge> = edges
            .iter()
            .filter(|edge| matches!(edge.relation, DependencyRelation::Upstream))
            .cloned()
            .collect();
        let downstream: Vec<DependencyEdge> = edges
            .iter()
            .filter(|edge| matches!(edge.relation, DependencyRelation::Downstream))
            .cloned()
            .collect();

        Ok(DependencyTree {
            root,
            upstream,
            downstream,
            aggregate: edges,
        })
    }

    fn visit(
        &self,
        project: &SpecManProject,
        summary: &ArtifactSummary,
        root: &ArtifactSummary,
        workspace: WorkspacePaths,
        edges: &mut Vec<DependencyEdge>,
        visited: &mut BTreeSet<String>,
    ) -> Result<(), SpecmanError> {
        edges.push(DependencyEdge {
            from: root.clone(),
            to: summary.clone(),
            relation: DependencyRelation::Upstream,
            optional: false,
        });

        let key = summary.id.to_string();
        if !visited.insert(key) {
            return Err(SpecmanError::Dependency(format!(
                "dependency cycle detected: {}",
                summary.id
            )));
        }

        for (reference, _kind) in references_of(project) {
            let mut cache_dir = install::ensure_reference(&reference, &workspace)?;
            cache_dir = cache_dir.join("specman.json");
            let nested = load_manifest(&cache_dir)
                .map_err(|err| SpecmanError::Dependency(format!("{cache_dir:?}: {err}")))?;
            let nested_root = summarize_project(&nested, Some(cache_dir.clone()));
            self.visit(
                &nested,
                &nested_root,
                summary,
                workspace.clone(),
                edges,
                visited,
            )?;
        }

        Ok(())
    }
}

impl<L: WorkspaceLocator> DependencyMapping for FilesystemDependencyMapper<L> {
    fn dependency_tree(&self, root: &ArtifactId) -> Result<DependencyTree, SpecmanError> {
        self.dependency_tree_for_artifact(root)
    }

    fn upstream(&self, root: &ArtifactId) -> Result<Vec<DependencyEdge>, SpecmanError> {
        let tree = self.dependency_tree(root)?;
        Ok(tree.upstream)
    }

    fn downstream(&self, root: &ArtifactId) -> Result<Vec<DependencyEdge>, SpecmanError> {
        let tree = self.dependency_tree(root)?;
        Ok(tree.downstream)
    }
}

impl<M> DependencyMapping for Arc<M>
where
    M: DependencyMapping,
{
    fn dependency_tree(&self, root: &ArtifactId) -> Result<DependencyTree, SpecmanError> {
        (**self).dependency_tree(root)
    }

    fn upstream(&self, root: &ArtifactId) -> Result<Vec<DependencyEdge>, SpecmanError> {
        (**self).upstream(root)
    }

    fn downstream(&self, root: &ArtifactId) -> Result<Vec<DependencyEdge>, SpecmanError> {
        (**self).downstream(root)
    }
}

fn summarize_project(project: &SpecManProject, resolved_path: Option<PathBuf>) -> ArtifactSummary {
    let kind = if project.is_spec() {
        ArtifactKind::Specification
    } else {
        ArtifactKind::Implementation
    };
    let mut metadata = BTreeMap::new();
    if let Some(description) = project.description() {
        metadata.insert("description".to_string(), description.to_string());
    }

    ArtifactSummary {
        id: ArtifactId {
            kind,
            name: project.name().to_string(),
        },
        metadata,
        resolved_path: resolved_path.map(|p| p.to_string_lossy().to_string()),
    }
}

fn references_of(project: &SpecManProject) -> Vec<(crate::manifest::GitReference, ArtifactKind)> {
    match (project.spec(), project.impl_()) {
        (Some(SpecProject { references, .. }), None) => references
            .iter()
            .cloned()
            .map(|r| (r, ArtifactKind::Specification))
            .collect(),
        (
            None,
            Some(ImplProject {
                implements,
                utilizes,
            }),
        ) => {
            let mut out: Vec<(crate::manifest::GitReference, ArtifactKind)> = implements
                .iter()
                .map(|e| (e.ref_.clone(), ArtifactKind::Specification))
                .collect();
            out.extend(
                utilizes
                    .iter()
                    .map(|u| (u.ref_.clone(), ArtifactKind::Implementation)),
            );
            out
        }
        _ => Vec::new(),
    }
}
