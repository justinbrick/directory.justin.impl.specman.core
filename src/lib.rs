pub(crate) mod core;
pub(crate) mod graph;
pub(crate) mod index;
pub(crate) mod install;
pub(crate) mod manifest;
pub(crate) mod markdown;
pub(crate) mod metadata;
pub(crate) mod ops;
pub(crate) mod storage;
pub(crate) mod templates;
pub(crate) mod validation;
pub(crate) mod workspace;

pub use core::env::SpecmanEnv;
pub use core::error::{LifecycleError, SpecmanError};
pub use core::shared::{EntityKind, SchemaRef, SemVer};
pub use graph::tree::{
    ArtifactId, ArtifactKind, ArtifactSummary, DependencyEdge, DependencyMapping,
    DependencyRelation, DependencyTree, FilesystemDependencyMapper,
};
pub use index::{
    ArtifactKey, ArtifactRecord, ConstraintIdentifier, ConstraintRecord,
    FilesystemStructureIndexer, HeadingIdentifier, HeadingRecord, RelationshipEdge,
    RelationshipKind, StructureIndexing, StructureQuery, WorkspaceIndex, build_workspace_index,
};
pub use install::{cache_root, ensure_reference, install, parse_git_url, ref_name, resolve_commit};
pub use manifest::{
    GitReference, ImplProject, ImplementsEntry, SpecManProject, SpecProject, UtilizesEntry,
    load_manifest, parse_manifest, validate_manifest_value,
};
pub use metadata::{
    IdentityUpdate, ImplementationUpdate, ManifestUpdate, SpecificationUpdate,
    apply_manifest_update,
};
pub use ops::create::{
    CreateImplOptions, CreateResult, CreateSpecOptions, create_implementation, create_specification,
};
pub use ops::delete::{DeleteOptions, DeletePlan, DeleteResult, delete_artifact};
pub use storage::adapter::{DataModelAdapter, InMemoryAdapter};
pub use storage::persistence::{
    ArtifactRemovalStore, PersistedArtifact, RemovedArtifact, WorkspacePersistence,
};
pub use templates::catalog::{ResolvedTemplate, TemplateCatalog};
pub use templates::engine::{
    ImplContext, MarkdownTemplateEngine, RenderedTemplate, SpecContext, TemplateDescriptor,
    TemplateEngine, TemplateLocator, TemplateProvenance, TemplateScenario, TemplateTier, TokenMap,
};
pub use validation::{
    ArtifactStatus, ComplianceReport, DiscoveredReference, HttpsValidationMode,
    HttpsValidationOptions, ReferenceDestinationKind, ReferenceIssueType, ReferenceValidationIssue,
    ReferenceValidationOptions, ReferenceValidationResult, SourceLocation, StatusResult,
    TransitiveValidationOptions, ValidationTag, ValidationType, WorkspaceStatusConfig,
    WorkspaceStatusReport, generate_report, parse_tags, scan_source_root, validate_compliance,
    validate_reference_content, validate_references, validate_workspace_status,
};
pub use workspace::{
    FilesystemWorkspaceLocator, WorkspaceContext, WorkspaceDiscovery, WorkspaceError,
    WorkspaceLocator, WorkspacePaths, discover as discover_workspace, workspace_relative_path,
};
