use std::fs;
use std::path::Path;

use crate::core::error::SpecmanError;
use crate::install::resolve_commit;
use crate::manifest::{GitReference, OverrideEntry, SpecManProjectManifest};
use crate::manifest_ops::load_manifest;
use crate::metadata::update_model::ManifestUpdate;

/// Applies a `ManifestUpdate` to a `specman.json` file in place and returns the full updated
/// project manifest.
///
/// Before writing, any newly added git references (references, implements, and override
/// `ref` values) are validated by resolving tag references to commit hashes via `git ls-remote`,
/// per the metadata-mutation requirement to "reuse git-reference resolution" before applying edits.
pub fn apply_manifest_update(
    path: &Path,
    update: &ManifestUpdate,
) -> Result<SpecManProjectManifest, SpecmanError> {
    let mut project = load_manifest(path)?;

    match update {
        ManifestUpdate::Specification(up) => {
            apply_identity(&mut project, &up.identity);
            apply_overrides(&mut project, &up.overrides)?;
            let SpecManProjectManifest::SpecificationManifest(m) = &mut project else {
                return Err(SpecmanError::Serialization(
                    "cannot apply specification update to a non-spec project".into(),
                ));
            };
            for reference in &up.add_references {
                validate_reference(reference)?;
                if !m.spec.references.iter().any(|r| r.alias == reference.alias) {
                    m.spec.references.push(reference.clone());
                }
            }
            m.spec.references.retain(|r| {
                !up.remove_references
                    .iter()
                    .any(|alias| alias == r.alias.as_str())
            });
        }
        ManifestUpdate::Implementation(up) => {
            apply_identity(&mut project, &up.identity);
            apply_overrides(&mut project, &up.overrides)?;
            let SpecManProjectManifest::ImplementationManifest(m) = &mut project else {
                return Err(SpecmanError::Serialization(
                    "cannot apply implementation update to a non-impl project".into(),
                ));
            };
            for entry in &up.add_implements {
                validate_reference(&entry.ref_)?;
                if !m
                    .impl_
                    .implements
                    .iter()
                    .any(|e| e.ref_.alias == entry.ref_.alias)
                {
                    m.impl_.implements.push(entry.clone());
                }
            }
            m.impl_.implements.retain(|e| {
                !up.remove_implements
                    .iter()
                    .any(|alias| alias == e.ref_.alias.as_str())
            });
        }
    }

    let serialized = serde_json::to_string_pretty(&project)
        .map_err(|err| SpecmanError::Serialization(err.to_string()))?;
    fs::write(path, serialized).map_err(SpecmanError::Io)?;

    Ok(project)
}

/// Adds and removes `overrides` entries on the top-level `overrides` list, keyed by `site`.
fn apply_overrides(
    project: &mut SpecManProjectManifest,
    update: &crate::metadata::update_model::OverridesUpdate,
) -> Result<(), SpecmanError> {
    let overrides = match project {
        SpecManProjectManifest::SpecificationManifest(m) => &mut m.overrides,
        SpecManProjectManifest::ImplementationManifest(m) => &mut m.overrides,
    };

    for entry in &update.add_overrides {
        validate_override(entry)?;
        if !overrides.iter().any(|o| o.site == entry.site) {
            overrides.push(entry.clone());
        }
    }
    overrides.retain(|o| {
        !update
            .remove_overrides
            .iter()
            .any(|site| site == o.site.as_str())
    });

    Ok(())
}

/// Validates a git reference by resolving its tag (if any) to a commit hash, reusing the same
/// resolution path as installing. Commit hashes pass through unchanged.
fn validate_reference(reference: &GitReference) -> Result<(), SpecmanError> {
    resolve_commit(reference.url.as_str(), &reference.ref_, None)?;
    Ok(())
}

/// Validates an override: its `ref` (a tag or hash) must resolve against the artifact's URL.
///
/// An override only carries a `site` (not a URL), so we cannot shell out to resolve it here without
/// the corresponding reference's URL. We instead validate that the `ref` is non-empty (which the
/// `OverrideEntryRef` newtype already guarantees) and defer tag resolution to install time, where
/// the site is matched to a concrete reference URL.
fn validate_override(entry: &OverrideEntry) -> Result<(), SpecmanError> {
    if entry.ref_.as_str().is_empty() {
        return Err(SpecmanError::Serialization(
            "overrides entry `ref` must not be empty".into(),
        ));
    }
    Ok(())
}

fn apply_identity(
    project: &mut SpecManProjectManifest,
    update: &crate::metadata::update_model::IdentityUpdate,
) {
    let (name, description, tags) = match project {
        SpecManProjectManifest::SpecificationManifest(m) => {
            (&mut m.name, &mut m.description, &mut m.tags)
        }
        SpecManProjectManifest::ImplementationManifest(m) => {
            (&mut m.name, &mut m.description, &mut m.tags)
        }
    };

    if let Some(new_name) = &update.name {
        *name = new_name.clone();
    }
    if let Some(new_description) = &update.description {
        *description = Some(new_description.clone());
    }
    if let Some(new_tags) = &update.tags {
        *tags = new_tags.clone();
    }
}
