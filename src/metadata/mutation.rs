use std::fs;
use std::path::Path;

use crate::core::error::SpecmanError;
use crate::manifest::{load_manifest, SpecManProject};
use crate::metadata::update_model::{IdentityUpdate, ManifestUpdate};

/// Applies a `ManifestUpdate` to a `specman.json` file in place and returns the full updated
/// project manifest.
pub fn apply_manifest_update(
    path: &Path,
    update: &ManifestUpdate,
) -> Result<SpecManProject, SpecmanError> {
    let mut project = load_manifest(path)?;

    match update {
        ManifestUpdate::Specification(up) => {
            apply_identity(&mut project, &up.identity);
            let Some(spec) = project.spec.as_mut() else {
                return Err(SpecmanError::Serialization(
                    "cannot apply specification update to a non-spec project".into(),
                ));
            };
            for reference in &up.add_references {
                if !spec.references.iter().any(|r| r.alias == reference.alias) {
                    spec.references.push(reference.clone());
                }
            }
            spec.references
                .retain(|r| !up.remove_references.iter().any(|alias| alias == &r.alias));
        }
        ManifestUpdate::Implementation(up) => {
            apply_identity(&mut project, &up.identity);
            let Some(impl_) = project.impl_.as_mut() else {
                return Err(SpecmanError::Serialization(
                    "cannot apply implementation update to a non-impl project".into(),
                ));
            };
            for entry in &up.add_implements {
                if !impl_.implements.iter().any(|e| e.ref_.alias == entry.ref_.alias) {
                    impl_.implements.push(entry.clone());
                }
            }
            impl_
                .implements
                .retain(|e| !up.remove_implements.iter().any(|alias| alias == &e.ref_.alias));
            for entry in &up.add_utilizes {
                if !impl_.utilizes.iter().any(|e| e.ref_.alias == entry.ref_.alias) {
                    impl_.utilizes.push(entry.clone());
                }
            }
            impl_
                .utilizes
                .retain(|e| !up.remove_utilizes.iter().any(|alias| alias == &e.ref_.alias));
        }
    }

    let serialized = serde_json::to_string_pretty(&project)
        .map_err(|err| SpecmanError::Serialization(err.to_string()))?;
    fs::write(path, serialized).map_err(SpecmanError::Io)?;

    Ok(project)
}

fn apply_identity(project: &mut SpecManProject, update: &IdentityUpdate) {
    if let Some(name) = &update.name {
        project.name = name.clone();
    }
    if let Some(description) = &update.description {
        project.description = Some(description.clone());
    }
    if let Some(tags) = &update.tags {
        project.tags = tags.clone();
    }
}
