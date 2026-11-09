use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::core::env::SpecmanEnv;
use crate::core::error::SpecmanError;
use crate::manifest::{
    GitReference, ImplProject, ImplementationManifest, ImplementsEntry, SpecManProjectManifest,
    SpecProject, SpecificationManifest,
};
use crate::storage::persistence::PersistedArtifact;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct CreateSpecOptions {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateImplOptions {
    pub name: String,
    pub target: GitReference,
    #[serde(default)]
    pub dry_run: bool,
}

/// The body rendered for a newly created artifact.
#[derive(Clone, Debug)]
pub struct CreatedBody {
    pub body: String,
}

#[derive(Clone, Debug)]
pub enum CreateResult {
    Persisted(PersistedArtifact),
    DryRun(CreatedBody),
}

/// Builds a minimal Markdown body for a new specification.
fn specification_body(name: &str, title: &str) -> String {
    format!(
        "# {title}\n\n{}\n",
        if name.is_empty() { title } else { name }
    )
}

/// Builds a minimal Markdown body for a new implementation.
fn implementation_body(name: &str) -> String {
    format!("# {name}\n\nImplementation of `{name}`.\n")
}

pub fn create_specification(
    env: &SpecmanEnv,
    opts: CreateSpecOptions,
) -> Result<CreateResult, SpecmanError> {
    let body = specification_body(&opts.name, &opts.title);

    if opts.dry_run {
        return Ok(CreateResult::DryRun(CreatedBody { body }));
    }

    let project = SpecManProjectManifest::SpecificationManifest(SpecificationManifest {
        name: opts.name,
        description: None,
        overrides: Vec::new(),
        schema: None,
        spec: SpecProject {
            index: Some("spec.md".to_string()),
            references: Vec::new(),
        },
        tags: Vec::new(),
    });

    let persisted = env.persistence.persist_project(&project, &body)?;
    Ok(CreateResult::Persisted(persisted))
}

pub fn create_implementation(
    env: &SpecmanEnv,
    opts: CreateImplOptions,
) -> Result<CreateResult, SpecmanError> {
    let body = implementation_body(&opts.name);

    if opts.dry_run {
        return Ok(CreateResult::DryRun(CreatedBody { body }));
    }

    let project = SpecManProjectManifest::ImplementationManifest(ImplementationManifest {
        name: opts.name,
        description: None,
        overrides: Vec::new(),
        schema: None,
        impl_: ImplProject {
            implements: vec![ImplementsEntry {
                ref_: opts.target,
                constraints: Vec::new(),
            }],
        },
        tags: Vec::new(),
    });

    let persisted = env.persistence.persist_project(&project, &body)?;
    Ok(CreateResult::Persisted(persisted))
}
