use std::fs;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::error::SpecmanError;

/// A git reference to another SpecMan artifact (specification or implementation).
///
/// Corresponds to `Entity: GitReference` in the SpecMan Core specification.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GitReference {
    /// Local name used in `.specman/ref/` symlink naming.
    pub alias: String,
    /// SSH or HTTPS git remote URL, usable as an argument to `git clone`.
    pub url: String,
    /// A full 40-character commit hash or a git tag name (branches are never consulted).
    #[serde(rename = "ref")]
    pub ref_: String,
}

/// Specification project data within `specman.json`.
///
/// Corresponds to `Entity: SpecProject` in the SpecMan Core specification.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpecProject {
    /// Workspace-relative path to the main Markdown document. Optional; when absent,
    /// tooling MUST NOT assume a default filename.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<String>,
    /// Dependencies on other specifications.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<GitReference>,
}

/// One specification an implementation targets.
///
/// Corresponds to `Entity: ImplementsEntry` in the SpecMan Core specification.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ImplementsEntry {
    /// The specification being implemented.
    #[serde(rename = "ref")]
    pub ref_: GitReference,
    /// Regex patterns matched against full dot-delimited constraint group identifiers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<String>,
}

/// One implementation an implementation depends on.
///
/// Corresponds to `Entity: UtilizesEntry` in the SpecMan Core specification.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct UtilizesEntry {
    /// The implementation being utilized.
    #[serde(rename = "ref")]
    pub ref_: GitReference,
    /// Regex patterns for constraint groups to filter out during compliance evaluation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore_constraints: Vec<String>,
}

/// Implementation project data within `specman.json`.
///
/// Corresponds to `Entity: ImplProject` in the SpecMan Core specification.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ImplProject {
    /// The specifications this implementation targets. Must contain at least one entry.
    pub implements: Vec<ImplementsEntry>,
    /// Other implementations this implementation depends on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub utilizes: Vec<UtilizesEntry>,
}

/// Top-level `specman.json` project manifest.
///
/// Corresponds to `Entity: SpecManProject` in the SpecMan Core specification.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpecManProject {
    /// Relative file URL pointing to the canonical JSON Schema.
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// Free-form human-readable project name.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Present if and only if this project is a specification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec: Option<SpecProject>,
    /// Present if and only if this project is an implementation.
    #[serde(rename = "impl", default, skip_serializing_if = "Option::is_none")]
    pub impl_: Option<ImplProject>,
}

impl SpecManProject {
    pub fn is_spec(&self) -> bool {
        self.spec.is_some()
    }

    pub fn is_impl(&self) -> bool {
        self.impl_.is_some()
    }
}

/// Loads and validates a `specman.json` file from disk.
pub fn load_manifest(path: &Path) -> Result<SpecManProject, SpecmanError> {
    let content = fs::read_to_string(path).map_err(|err| {
        SpecmanError::Workspace(format!("failed to read {}: {}", path.display(), err))
    })?;
    parse_manifest(&content)
}

/// Parses and validates `specman.json` content.
pub fn parse_manifest(content: &str) -> Result<SpecManProject, SpecmanError> {
    let value: Value = serde_json::from_str(content)
        .map_err(|err| SpecmanError::Serialization(format!("invalid specman.json: {err}")))?;
    validate_manifest_value(&value)?;
    serde_json::from_value(value)
        .map_err(|err| SpecmanError::Serialization(format!("invalid specman.json: {err}")))
}

/// Validates a parsed `specman.json` value against the structural invariants of the data model.
pub fn validate_manifest_value(value: &Value) -> Result<(), SpecmanError> {
    let obj = value
        .as_object()
        .ok_or_else(|| SpecmanError::Serialization("specman.json must be a JSON object".into()))?;

    match obj.get("name") {
        Some(Value::String(name)) if !name.is_empty() => {}
        Some(Value::Null) | None => {
            return Err(SpecmanError::Serialization(
                "specman.json requires a non-null `name` string".into(),
            ));
        }
        _ => {
            return Err(SpecmanError::Serialization(
                "specman.json `name` must be a non-empty string".into(),
            ));
        }
    }

    let has_spec = obj.contains_key("spec");
    let has_impl = obj.contains_key("impl");
    if has_spec == has_impl {
        return Err(SpecmanError::Serialization(
            "specman.json must contain exactly one of `spec` or `impl`".into(),
        ));
    }

    for key in ["spec", "impl"] {
        if obj.get(key) == Some(&Value::Null) {
            return Err(SpecmanError::Serialization(format!(
                "specman.json field `{key}` must not be null"
            )));
        }
    }

    if let Some(spec) = obj.get("spec").and_then(Value::as_object) {
        validate_spec_project(spec)?;
    }
    if let Some(impl_) = obj.get("impl").and_then(Value::as_object) {
        validate_impl_project(impl_)?;
    }

    Ok(())
}

fn validate_spec_project(spec: &serde_json::Map<String, Value>) -> Result<(), SpecmanError> {
    if let Some(Value::Null) = spec.get("index") {
        return Err(SpecmanError::Serialization(
            "specman.json `spec.index` must not be null".into(),
        ));
    }
    if let Some(index) = spec.get("index") {
        if !index.is_string() {
            return Err(SpecmanError::Serialization(
                "specman.json `spec.index` must be a string".into(),
            ));
        }
    }
    if let Some(references) = spec.get("references") {
        validate_git_reference_array(references, "spec.references")?;
    }
    Ok(())
}

fn validate_impl_project(impl_: &serde_json::Map<String, Value>) -> Result<(), SpecmanError> {
    match impl_.get("implements") {
        Some(Value::Array(items)) if !items.is_empty() => {
            for (idx, item) in items.iter().enumerate() {
                validate_ref_entry(item, &format!("impl.implements[{idx}]"))?;
            }
        }
        Some(Value::Null) | None => {
            return Err(SpecmanError::Serialization(
                "specman.json `impl.implements` must be a non-null, non-empty array".into(),
            ));
        }
        _ => {
            return Err(SpecmanError::Serialization(
                "specman.json `impl.implements` must be an array".into(),
            ));
        }
    }

    if let Some(utilizes) = impl_.get("utilizes") {
        match utilizes {
            Value::Array(items) => {
                for (idx, item) in items.iter().enumerate() {
                    validate_ref_entry(item, &format!("impl.utilizes[{idx}]"))?;
                }
            }
            Value::Null => {
                return Err(SpecmanError::Serialization(
                    "specman.json `impl.utilizes` must not be null".into(),
                ));
            }
            _ => {
                return Err(SpecmanError::Serialization(
                    "specman.json `impl.utilizes` must be an array".into(),
                ));
            }
        }
    }

    Ok(())
}

fn validate_ref_entry(value: &Value, field: &str) -> Result<(), SpecmanError> {
    let obj = value.as_object().ok_or_else(|| {
        SpecmanError::Serialization(format!("specman.json `{field}` must be an object"))
    })?;
    match obj.get("ref") {
        Some(ref_value) => validate_git_reference_entry(ref_value, &format!("{field}.ref")),
        _ => Err(SpecmanError::Serialization(format!(
            "specman.json `{field}` must contain a `ref` object"
        ))),
    }
}

fn validate_git_reference_array(value: &Value, field: &str) -> Result<(), SpecmanError> {
    match value {
        Value::Array(items) => {
            for (idx, item) in items.iter().enumerate() {
                validate_git_reference_entry(item, &format!("{field}[{idx}]"))?;
            }
            Ok(())
        }
        Value::Null => Err(SpecmanError::Serialization(format!(
            "specman.json `{field}` must not be null"
        ))),
        _ => Err(SpecmanError::Serialization(format!(
            "specman.json `{field}` must be an array"
        ))),
    }
}

fn validate_git_reference_entry(value: &Value, field: &str) -> Result<(), SpecmanError> {
    let obj = value.as_object().ok_or_else(|| {
        SpecmanError::Serialization(format!("specman.json `{field}` must be an object"))
    })?;

    for key in ["alias", "url", "ref"] {
        match obj.get(key) {
            Some(Value::String(s)) if !s.is_empty() => {}
            Some(Value::Null) | None => {
                return Err(SpecmanError::Serialization(format!(
                    "specman.json `{field}.{key}` must be a non-null, non-empty string"
                )));
            }
            _ => {
                return Err(SpecmanError::Serialization(format!(
                    "specman.json `{field}.{key}` must be a string"
                )));
            }
        }
    }

    let url = obj.get("url").and_then(Value::as_str).unwrap_or_default();
    if !(url.starts_with("https://") || url.starts_with("git@")) {
        return Err(SpecmanError::Serialization(format!(
            "specman.json `{field}.url` must be an SSH or HTTPS git URL"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_spec_project() {
        let json = r#"{
            "name": "specman-core",
            "spec": { "index": "spec.md" }
        }"#;
        let project = parse_manifest(json).expect("parse spec project");
        assert!(project.is_spec());
        assert!(!project.is_impl());
        assert_eq!(project.name, "specman-core");
        assert_eq!(project.spec.unwrap().index.as_deref(), Some("spec.md"));
    }

    #[test]
    fn parses_impl_project_with_implements() {
        let json = r#"{
            "name": "specman-library",
            "impl": {
                "implements": [
                    { "ref": { "alias": "core", "url": "https://github.com/owner/core.git", "ref": "main" } }
                ]
            }
        }"#;
        let project = parse_manifest(json).expect("parse impl project");
        assert!(project.is_impl());
        assert_eq!(project.impl_.unwrap().implements.len(), 1);
    }

    #[test]
    fn rejects_mutual_exclusion() {
        let json = r#"{ "name": "both", "spec": {}, "impl": { "implements": [] } }"#;
        let err = parse_manifest(json).expect_err("both spec and impl should fail");
        assert!(err.to_string().contains("exactly one"));
    }

    #[test]
    fn rejects_null_index() {
        let json = r#"{ "name": "spec", "spec": { "index": null } }"#;
        let err = parse_manifest(json).expect_err("null index should fail");
        assert!(err.to_string().contains("must not be null"));
    }

    #[test]
    fn index_is_optional() {
        let json = r#"{ "name": "spec", "spec": {} }"#;
        let project = parse_manifest(json).expect("optional index");
        assert_eq!(project.spec.unwrap().index, None);
    }
}
