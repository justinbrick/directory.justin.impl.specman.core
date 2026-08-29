use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::core::error::SpecmanError;
use crate::graph::tree::{ArtifactId, ArtifactKind};
use crate::manifest::SpecManProject;
use crate::workspace::{WorkspaceLocator, WorkspacePaths};

/// An artifact that was persisted to disk.
#[derive(Clone, Debug)]
pub struct PersistedArtifact {
    pub artifact: ArtifactId,
    pub path: PathBuf,
    pub workspace: WorkspacePaths,
}

/// An artifact that was removed from disk.
#[derive(Clone, Debug)]
pub struct RemovedArtifact {
    pub artifact: ArtifactId,
    pub directory: PathBuf,
    pub workspace: WorkspacePaths,
}

/// Persists `specman.json` project manifests and their Markdown bodies at the repository root.
pub struct WorkspacePersistence<L: WorkspaceLocator> {
    locator: L,
}

impl<L: WorkspaceLocator> WorkspacePersistence<L> {
    pub fn new(locator: L) -> Self {
        Self { locator }
    }

    pub fn workspace(&self) -> Result<WorkspacePaths, SpecmanError> {
        self.locator.workspace()
    }

    /// Writes a `specman.json` manifest plus the project's Markdown body to the repository root.
    pub fn persist_project(
        &self,
        project: &SpecManProject,
        body: &str,
    ) -> Result<PersistedArtifact, SpecmanError> {
        let workspace = self.workspace()?;
        let root = workspace.root();

        let manifest_json = serde_json::to_string_pretty(project)
            .map_err(|err| SpecmanError::Serialization(err.to_string()))?;
        fs::write(root.join("specman.json"), manifest_json).map_err(SpecmanError::Io)?;

        let (kind, body_path) = if project.spec.is_some() {
            let index = project
                .spec
                .as_ref()
                .and_then(|spec| spec.index.as_deref())
                .unwrap_or("spec.md");
            (ArtifactKind::Specification, root.join(index))
        } else {
            (ArtifactKind::Implementation, root.join("impl.md"))
        };

        fs::write(&body_path, body).map_err(SpecmanError::Io)?;

        Ok(PersistedArtifact {
            artifact: ArtifactId {
                kind,
                name: project.name.clone(),
            },
            path: body_path,
            workspace,
        })
    }

    pub fn remove_artifact(&self, _artifact: &ArtifactId) -> Result<RemovedArtifact, SpecmanError> {
        let workspace = self.workspace()?;
        let root = workspace.root().to_path_buf();

        let manifest = root.join("specman.json");
        if manifest.exists() {
            fs::remove_file(&manifest).map_err(SpecmanError::Io)?;
        }
        let body = root.join("impl.md");
        if body.exists() {
            fs::remove_file(&body).map_err(SpecmanError::Io)?;
        }
        let body = root.join("spec.md");
        if body.exists() {
            fs::remove_file(&body).map_err(SpecmanError::Io)?;
        }

        Ok(RemovedArtifact {
            artifact: _artifact.clone(),
            directory: root,
            workspace,
        })
    }
}

/// Trait implemented by persistence layers that can remove artifacts.
pub trait ArtifactRemovalStore: Send + Sync {
    fn remove_artifact(&self, artifact: &ArtifactId) -> Result<RemovedArtifact, SpecmanError>;
}

impl<L: WorkspaceLocator> ArtifactRemovalStore for WorkspacePersistence<L> {
    fn remove_artifact(&self, artifact: &ArtifactId) -> Result<RemovedArtifact, SpecmanError> {
        WorkspacePersistence::remove_artifact(self, artifact)
    }
}

impl<S> ArtifactRemovalStore for Arc<S>
where
    S: ArtifactRemovalStore,
{
    fn remove_artifact(&self, artifact: &ArtifactId) -> Result<RemovedArtifact, SpecmanError> {
        (**self).remove_artifact(artifact)
    }
}
