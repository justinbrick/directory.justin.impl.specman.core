//! Hand-written extensions and utilities for the `typify`-generated manifest types.
//!
//! The data-model types themselves live in [`crate::manifest`] and are produced by
//! `cargo-typify` from `spec/specman/models/specman.schema.json`; that file is never
//! hand-edited. This module contains everything that cannot be generated but is still
//! required at runtime:
//!
//! * the load/parse/validate pipeline for `specman.json` files, and
//! * behavioral helpers such as [`GitReference::site`].

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::core::error::SpecmanError;
use crate::manifest::{
    GitReference, ImplProject, OverrideEntry, SpecManProjectManifest, SpecProject,
};

impl SpecManProjectManifest {
    /// The project name (`name`).
    pub fn name(&self) -> &str {
        match self {
            SpecManProjectManifest::SpecificationManifest(m) => &m.name,
            SpecManProjectManifest::ImplementationManifest(m) => &m.name,
        }
    }

    /// The project description (`description`), if present.
    pub fn description(&self) -> Option<&str> {
        match self {
            SpecManProjectManifest::SpecificationManifest(m) => m.description.as_deref(),
            SpecManProjectManifest::ImplementationManifest(m) => m.description.as_deref(),
        }
    }

    /// The dependency-resolution overrides (`overrides`).
    pub fn overrides(&self) -> &[OverrideEntry] {
        match self {
            SpecManProjectManifest::SpecificationManifest(m) => &m.overrides,
            SpecManProjectManifest::ImplementationManifest(m) => &m.overrides,
        }
    }

    /// The child `spec` object when this project is a specification.
    pub fn spec(&self) -> Option<&SpecProject> {
        match self {
            SpecManProjectManifest::SpecificationManifest(m) => Some(&m.spec),
            SpecManProjectManifest::ImplementationManifest(_) => None,
        }
    }

    /// The child `impl` object when this project is an implementation.
    pub fn impl_(&self) -> Option<&ImplProject> {
        match self {
            SpecManProjectManifest::ImplementationManifest(m) => Some(&m.impl_),
            SpecManProjectManifest::SpecificationManifest(_) => None,
        }
    }

    /// Whether this project is a specification (`spec` present).
    pub fn is_spec(&self) -> bool {
        matches!(self, SpecManProjectManifest::SpecificationManifest(_))
    }

    /// Whether this project is an implementation (`impl` present).
    pub fn is_impl(&self) -> bool {
        matches!(self, SpecManProjectManifest::ImplementationManifest(_))
    }
}

impl GitReference {
    /// Derives the normalized artifact *site* identity from `self.url`.
    ///
    /// The site ignores the URL scheme (SSH vs HTTPS) and the `.git` suffix, and is
    /// normalized to the form `{host}/{path}` (for example `example.com/owner/repo`).
    /// Two `GitReference` values refer to the same artifact if and only if their derived
    /// sites are equal.
    pub fn site(&self) -> String {
        let without_scheme = self
            .url
            .as_str()
            .trim_start_matches("https://")
            .trim_start_matches("git@")
            .trim_end_matches(".git");

        let (host, path) = if let Some((h, p)) = without_scheme.split_once(':') {
            (h.to_string(), p.to_string())
        } else {
            match without_scheme.split_once('/') {
                Some((h, p)) => (h.to_string(), p.to_string()),
                None => (without_scheme.to_string(), String::new()),
            }
        };

        let path = path.trim_end_matches('/');
        if path.is_empty() {
            host
        } else {
            format!("{host}/{path}")
        }
    }
}

/// Loads and validates a `specman.json` file from disk.
pub fn load_manifest(path: &Path) -> Result<SpecManProjectManifest, SpecmanError> {
    let content = fs::read_to_string(path).map_err(|err| {
        SpecmanError::Workspace(format!("failed to read {}: {}", path.display(), err))
    })?;
    parse_manifest(&content)
}

/// Parses and validates `specman.json` content.
pub fn parse_manifest(content: &str) -> Result<SpecManProjectManifest, SpecmanError> {
    let value: Value = serde_json::from_str(content)
        .map_err(|err| SpecmanError::Serialization(format!("invalid specman.json: {err}")))?;
    validate_manifest_value(&value)?;
    serde_json::from_value(value)
        .map_err(|err| SpecmanError::Serialization(format!("invalid specman.json: {err}")))
}

/// Validates a parsed `specman.json` value against the structural invariants that the generated
/// types do not enforce on their own (for example, a non-empty `name` and a non-null `spec.index`).
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

    if let Some(alias) = obj.get("alias").and_then(Value::as_str) {
        if is_reserved_alias(alias) {
            return Err(SpecmanError::Serialization(format!(
                "specman.json `{field}.alias` is a reserved device name"
            )));
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

/// Returns `true` when an alias case-insensitively equals a Windows reserved device name
/// (or ends with a dot or space, which the `pattern` already rejects but we guard defensively).
fn is_reserved_alias(alias: &str) -> bool {
    let upper = alias.to_ascii_uppercase();
    let stem = upper.trim_end_matches('.');
    if stem.is_empty() || stem.ends_with(' ') || stem.ends_with('.') {
        return true;
    }
    matches!(stem, "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && stem.starts_with("COM")
            && stem[3..].chars().all(|c| c.is_ascii_digit())
            && stem[3..].parse::<u8>().is_ok())
        || (stem.len() == 4
            && stem.starts_with("LPT")
            && stem[3..].chars().all(|c| c.is_ascii_digit())
            && stem[3..].parse::<u8>().is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::GitReferenceUrl;

    #[test]
    fn parses_spec_project() {
        let json = r#"{
            "name": "specman-core",
            "spec": { "index": "spec.md" }
        }"#;
        let project = parse_manifest(json).expect("parse spec project");
        assert!(project.is_spec());
        assert!(!project.is_impl());
        assert_eq!(project.name(), "specman-core");
        assert_eq!(project.spec().unwrap().index.as_deref(), Some("spec.md"));
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
        assert_eq!(project.impl_().unwrap().implements.len(), 1);
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
        assert_eq!(project.spec().unwrap().index, None);
    }

    #[test]
    fn derives_site_from_https_and_ssh_urls() {
        let https = "https://github.com/owner/repo.git"
            .parse::<GitReferenceUrl>()
            .unwrap();
        let ssh = "git@github.com:owner/repo.git"
            .parse::<GitReferenceUrl>()
            .unwrap();

        let ref_https = GitReference {
            alias: "a".parse().unwrap(),
            url: https,
            ref_: "deadbeef".to_string(),
        };
        let ref_ssh = GitReference {
            alias: "b".parse().unwrap(),
            url: ssh,
            ref_: "deadbeef".to_string(),
        };

        assert_eq!(ref_https.site(), "github.com/owner/repo");
        assert_eq!(ref_ssh.site(), "github.com/owner/repo");
        assert_eq!(ref_https.site(), ref_ssh.site());
    }
}
