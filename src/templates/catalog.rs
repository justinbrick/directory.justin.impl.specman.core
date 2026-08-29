use std::fs;
use std::path::PathBuf;

use crate::core::error::SpecmanError;
use crate::templates::engine::{
    TemplateDescriptor, TemplateLocator, TemplateProvenance, TemplateScenario, TemplateTier,
};
use crate::workspace::WorkspacePaths;

const EMBEDDED_SPEC: &str = include_str!("spec/spec.md");
const EMBEDDED_IMPL: &str = include_str!("impl/impl.md");

/// Resolved template plus provenance metadata.
#[derive(Clone, Debug)]
pub struct ResolvedTemplate {
    pub descriptor: TemplateDescriptor,
    pub provenance: TemplateProvenance,
}

/// Resolves templates from workspace overrides and embedded defaults.
pub struct TemplateCatalog {
    workspace: WorkspacePaths,
}

impl TemplateCatalog {
    pub fn new(workspace: WorkspacePaths) -> Self {
        Self { workspace }
    }

    /// Resolves a template for the requested scenario.
    pub fn resolve(&self, scenario: TemplateScenario) -> Result<ResolvedTemplate, SpecmanError> {
        let (name, body) = match scenario {
            TemplateScenario::Specification => ("spec.md", EMBEDDED_SPEC),
            TemplateScenario::Implementation => ("impl.md", EMBEDDED_IMPL),
        };

        // Prefer a workspace override, then fall back to the embedded default.
        let override_path = self.templates_dir().join(name);
        let (path, tier, locator) = if override_path.is_file() {
            (
                override_path,
                TemplateTier::WorkspaceOverride,
                "workspace://override".to_string(),
            )
        } else {
            let cache_dir = self.workspace.cache_dir();
            let cache_path = cache_dir.join("templates").join(name);
            fs::create_dir_all(cache_path.parent().unwrap_or(cache_dir.as_path()))
                .map_err(SpecmanError::Io)?;
            fs::write(&cache_path, body).map_err(SpecmanError::Io)?;
            (
                cache_path,
                TemplateTier::EmbeddedDefault,
                format!("embedded://{name}"),
            )
        };

        let required_tokens = match scenario {
            TemplateScenario::Specification => vec!["output_name".to_string(), "title".to_string()],
            TemplateScenario::Implementation => vec!["output_name".to_string()],
        };

        Ok(ResolvedTemplate {
            descriptor: TemplateDescriptor {
                locator: TemplateLocator::FilePath(path),
                scenario,
                required_tokens,
            },
            provenance: TemplateProvenance {
                tier,
                locator,
                ..Default::default()
            },
        })
    }

    fn templates_dir(&self) -> PathBuf {
        self.workspace.dot_specman().join("templates")
    }
}
