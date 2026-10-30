use crate::core::error::SpecmanError;
use crate::graph::tree::{DependencyMapping, FilesystemDependencyMapper};
use crate::storage::persistence::WorkspacePersistence;
use crate::workspace::FilesystemWorkspaceLocator;
use std::path::Path;
use std::sync::Arc;

pub type DefaultWorkspaceLocator = Arc<FilesystemWorkspaceLocator>;
pub type DefaultPersistence = WorkspacePersistence<DefaultWorkspaceLocator>;

/// Shared environment components for Specman operations.
/// Holds the necessary state and services to perform artifact management.
pub struct SpecmanEnv {
    pub persistence: DefaultPersistence,
    pub mapping: Arc<dyn DependencyMapping>,
}

impl SpecmanEnv {
    /// Initialize the environment from the current working directory.
    pub fn from_current_dir() -> Result<Self, SpecmanError> {
        let locator = Arc::new(FilesystemWorkspaceLocator::from_current_dir()?);
        Self::new(locator)
    }

    /// Initialize the environment from a specific path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, SpecmanError> {
        let locator = Arc::new(FilesystemWorkspaceLocator::new(path.as_ref().to_path_buf()));
        Self::new(locator)
    }

    /// Create a new environment with an explicit locator.
    pub fn new(locator: DefaultWorkspaceLocator) -> Result<Self, SpecmanError> {
        let mapper = FilesystemDependencyMapper::new(locator.clone());
        let persistence = WorkspacePersistence::new(locator.clone());

        Ok(Self {
            persistence,
            mapping: Arc::new(mapper),
        })
    }
}
