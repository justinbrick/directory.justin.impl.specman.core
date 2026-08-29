use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::manifest::{GitReference, ImplementsEntry, UtilizesEntry};

/// Shared identity fields for updates.
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
pub struct IdentityUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Update fields for specification projects (operate on `specman.json`).
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
pub struct SpecificationUpdate {
    #[serde(flatten)]
    pub identity: IdentityUpdate,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add_references: Vec<GitReference>,
    /// Aliases of `spec.references` entries to remove.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_references: Vec<String>,
}

/// Update fields for implementation projects (operate on `specman.json`).
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
pub struct ImplementationUpdate {
    #[serde(flatten)]
    pub identity: IdentityUpdate,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add_implements: Vec<ImplementsEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_implements: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add_utilizes: Vec<UtilizesEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_utilizes: Vec<String>,
}

/// Discriminated enum for type-safe `specman.json` updates.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind")]
pub enum ManifestUpdate {
    Specification(SpecificationUpdate),
    Implementation(ImplementationUpdate),
}
