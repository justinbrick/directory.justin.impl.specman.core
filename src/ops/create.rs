use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::core::env::SpecmanEnv;
use crate::core::error::SpecmanError;
use crate::manifest::{GitReference, ImplProject, ImplementsEntry, SpecManProject, SpecProject};
use crate::storage::persistence::PersistedArtifact;
use crate::templates::engine::{
    RenderedTemplate, TemplateScenario, TokenMap,
};

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

#[derive(Clone, Debug)]
pub enum CreateResult {
    Persisted(PersistedArtifact),
    DryRun(RenderedTemplate),
}

pub fn create_specification(
    env: &SpecmanEnv,
    opts: CreateSpecOptions,
) -> Result<CreateResult, SpecmanError> {
    // [ENSURES: concept-lifecycle-automation.requirements:CHECK]
    let resolved = env.catalog.resolve(TemplateScenario::Specification)?;

    let mut tokens = TokenMap::new();
    tokens.insert(
        "output_name".to_string(),
        serde_json::Value::String(opts.name.clone()),
    );
    tokens.insert(
        "title".to_string(),
        serde_json::Value::String(opts.title.clone()),
    );

    let mut rendered = env.templates.render(&resolved.descriptor, &tokens)?;
    rendered.provenance = Some(resolved.provenance);

    if opts.dry_run {
        return Ok(CreateResult::DryRun(rendered));
    }

    let project = SpecManProject {
        name: opts.name,
        spec: Some(SpecProject {
            index: Some("spec.md".to_string()),
            references: Vec::new(),
        }),
        ..Default::default()
    };

    let persisted = env.persistence.persist_project(&project, &rendered.body)?;
    Ok(CreateResult::Persisted(persisted))
}

pub fn create_implementation(
    env: &SpecmanEnv,
    opts: CreateImplOptions,
) -> Result<CreateResult, SpecmanError> {
    let resolved = env.catalog.resolve(TemplateScenario::Implementation)?;

    let mut tokens = TokenMap::new();
    tokens.insert(
        "output_name".to_string(),
        serde_json::Value::String(opts.name.clone()),
    );

    let mut rendered = env.templates.render(&resolved.descriptor, &tokens)?;
    rendered.provenance = Some(resolved.provenance);

    if opts.dry_run {
        return Ok(CreateResult::DryRun(rendered));
    }

    let project = SpecManProject {
        name: opts.name,
        impl_: Some(ImplProject {
            implements: vec![ImplementsEntry {
                ref_: opts.target,
                constraints: Vec::new(),
            }],
            utilizes: Vec::new(),
        }),
        ..Default::default()
    };

    let persisted = env.persistence.persist_project(&project, &rendered.body)?;
    Ok(CreateResult::Persisted(persisted))
}

// Re-export the context types used by templates.
