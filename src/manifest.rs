#![allow(clippy::redundant_closure_call)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::match_single_binding)]
#![allow(clippy::clone_on_copy)]

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::core::error::SpecmanError;

#[doc = "`GitReference`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, schemars :: JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitReference {
    #[doc = "A local name for the referenced artifact. Used in local path resolution and lockfile naming. Should be stable across versions of the referencing project. Must be a portable filename segment matching ^[A-Za-z0-9][A-Za-z0-9._-]*$."]
    pub alias: GitReferenceAlias,
    #[doc = "A reference to a specific commit. Either a full 40-character SHA-1 commit hash or a git tag name. Branches are never consulted."]
    #[serde(rename = "ref")]
    pub ref_: ::std::string::String,
    #[doc = "The git remote URL. Must be an SSH git link (git@gitservice.com:owner/repo.git) or an HTTPS git link (https://gitservice.com/owner/repo.git). Must be usable as an argument to git clone."]
    pub url: GitReferenceUrl,
}
#[doc = "A local name for the referenced artifact. Used in local path resolution and lockfile naming. Should be stable across versions of the referencing project. Must be a portable filename segment matching ^[A-Za-z0-9][A-Za-z0-9._-]*$."]
#[derive(
    :: serde :: Serialize,
    Clone,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    schemars :: JsonSchema,
)]
#[serde(transparent)]
pub struct GitReferenceAlias(::std::string::String);
impl ::std::ops::Deref for GitReferenceAlias {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<GitReferenceAlias> for ::std::string::String {
    fn from(value: GitReferenceAlias) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for GitReferenceAlias {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
            ::std::sync::LazyLock::new(|| {
                ::regress::Regex::new("^[A-Za-z0-9][A-Za-z0-9._-]*$").unwrap()
            });
        if PATTERN.find(value).is_none() {
            return Err("doesn't match pattern \"^[A-Za-z0-9][A-Za-z0-9._-]*$\"".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for GitReferenceAlias {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for GitReferenceAlias {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for GitReferenceAlias {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
#[doc = "The git remote URL. Must be an SSH git link (git@gitservice.com:owner/repo.git) or an HTTPS git link (https://gitservice.com/owner/repo.git). Must be usable as an argument to git clone."]
#[derive(
    :: serde :: Serialize,
    Clone,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    schemars :: JsonSchema,
)]
#[serde(transparent)]
pub struct GitReferenceUrl(::std::string::String);
impl ::std::ops::Deref for GitReferenceUrl {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<GitReferenceUrl> for ::std::string::String {
    fn from(value: GitReferenceUrl) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for GitReferenceUrl {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
            ::std::sync::LazyLock::new(|| {
                ::regress::Regex::new("^(https://[^/]+/.+\\.git|git@[^:]+:.+\\.git)$").unwrap()
            });
        if PATTERN.find(value).is_none() {
            return Err(
                "doesn't match pattern \"^(https://[^/]+/.+\\.git|git@[^:]+:.+\\.git)$\"".into(),
            );
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for GitReferenceUrl {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for GitReferenceUrl {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for GitReferenceUrl {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
#[doc = "`ImplProject`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, schemars :: JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImplProject {
    #[doc = "The specifications this implementation targets. Must contain at least one entry."]
    pub implements: ::std::vec::Vec<ImplementsEntry>,
    #[doc = "Other implementations this implementation depends on. May be absent or an empty array, but must not be null."]
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub utilizes: ::std::vec::Vec<UtilizesEntry>,
}
#[doc = "`ImplementsEntry`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, schemars :: JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImplementsEntry {
    #[doc = "A list of regex patterns. Matched against full dot-delimited constraint group identifiers (e.g., concept-slug.category). If absent or empty, functionally equivalent to ['.*'] (all constraint groups are included). Must not be null."]
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub constraints: ::std::vec::Vec<::std::string::String>,
    #[doc = "The specification being implemented. The referenced artifact must be a specification."]
    #[serde(rename = "ref")]
    pub ref_: GitReference,
}
#[doc = "`LockedEntry`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, schemars :: JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LockedEntry {
    #[doc = "The local name for the referenced artifact, matching the alias in the corresponding GitReference."]
    pub alias: ::std::string::String,
    #[doc = "The nested filesystem path to the reference's materialization location, relative to the workspace root (for example spec/B/spec/D/). For a direct reference this equals spec/{alias}/. Used to derive the on-disk materialization location."]
    pub path: ::std::string::String,
    #[doc = "The reference value as declared in specman.json (either a tag name or a commit hash)."]
    pub requested: ::std::string::String,
    #[doc = "The full git commit hash that the reference resolved to at generation time. Must be a 40-character commit hash identifier."]
    pub resolved: LockedEntryResolved,
    #[doc = "Whether the entry is a direct reference declared in specman.json (false) or a transitively reached dependency (true)."]
    pub transitive: bool,
    #[doc = "The git remote URL of the referenced artifact."]
    pub url: ::std::string::String,
}
#[doc = "The full git commit hash that the reference resolved to at generation time. Must be a 40-character commit hash identifier."]
#[derive(
    :: serde :: Serialize,
    Clone,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    schemars :: JsonSchema,
)]
#[serde(transparent)]
pub struct LockedEntryResolved(::std::string::String);
impl ::std::ops::Deref for LockedEntryResolved {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<LockedEntryResolved> for ::std::string::String {
    fn from(value: LockedEntryResolved) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for LockedEntryResolved {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
            ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9a-fA-F]{40}$").unwrap());
        if PATTERN.find(value).is_none() {
            return Err("doesn't match pattern \"^[0-9a-fA-F]{40}$\"".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for LockedEntryResolved {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for LockedEntryResolved {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for LockedEntryResolved {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
#[doc = "`Lockfile`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, schemars :: JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Lockfile {
    #[doc = "The resolved entries for every direct and transitive dependency."]
    pub references: ::std::vec::Vec<LockedEntry>,
    #[doc = "The lockfile format version. Implementations must reject lockfiles whose version they do not understand."]
    pub version: i64,
}
#[doc = "`OverrideEntry`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, schemars :: JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OverrideEntry {
    #[doc = "The commit hash or git tag to force for every reference whose derived site matches."]
    #[serde(rename = "ref")]
    pub ref_: OverrideEntryRef,
    #[doc = "The normalized artifact site (host/path), as derived from a GitReference url. Overrides match by site, not alias."]
    pub site: OverrideEntrySite,
}
#[doc = "The commit hash or git tag to force for every reference whose derived site matches."]
#[derive(
    :: serde :: Serialize,
    Clone,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    schemars :: JsonSchema,
)]
#[serde(transparent)]
pub struct OverrideEntryRef(::std::string::String);
impl ::std::ops::Deref for OverrideEntryRef {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<OverrideEntryRef> for ::std::string::String {
    fn from(value: OverrideEntryRef) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for OverrideEntryRef {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for OverrideEntryRef {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for OverrideEntryRef {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for OverrideEntryRef {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
#[doc = "The normalized artifact site (host/path), as derived from a GitReference url. Overrides match by site, not alias."]
#[derive(
    :: serde :: Serialize,
    Clone,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    schemars :: JsonSchema,
)]
#[serde(transparent)]
pub struct OverrideEntrySite(::std::string::String);
impl ::std::ops::Deref for OverrideEntrySite {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<OverrideEntrySite> for ::std::string::String {
    fn from(value: OverrideEntrySite) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for OverrideEntrySite {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for OverrideEntrySite {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for OverrideEntrySite {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for OverrideEntrySite {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
#[doc = "Canonical JSON Schema for a specman.json project manifest file."]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, schemars :: JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum SpecManProjectManifest {
    Variant0 {
        #[doc = "A human-readable description of the project."]
        #[serde(skip_serializing_if = "::std::option::Option::is_none")]
        description: ::std::option::Option<::std::string::String>,
        #[doc = "The project name. Any UTF-8 string; free-form and human-readable."]
        name: ::std::string::String,
        #[doc = "Explicit version resolutions applied during dependency resolution. May be absent or an empty array, but must not be null."]
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        overrides: ::std::vec::Vec<OverrideEntry>,
        #[doc = "A relative file URL pointing to this schema."]
        #[serde(
            rename = "$schema",
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        schema: ::std::option::Option<::std::string::String>,
        spec: SpecProject,
        #[doc = "Tags for categorization. Must not be null."]
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        tags: ::std::vec::Vec<::std::string::String>,
    },
    Variant1 {
        #[doc = "A human-readable description of the project."]
        #[serde(skip_serializing_if = "::std::option::Option::is_none")]
        description: ::std::option::Option<::std::string::String>,
        #[serde(rename = "impl")]
        impl_: ImplProject,
        #[doc = "The project name. Any UTF-8 string; free-form and human-readable."]
        name: ::std::string::String,
        #[doc = "Explicit version resolutions applied during dependency resolution. May be absent or an empty array, but must not be null."]
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        overrides: ::std::vec::Vec<OverrideEntry>,
        #[doc = "A relative file URL pointing to this schema."]
        #[serde(
            rename = "$schema",
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        schema: ::std::option::Option<::std::string::String>,
        #[doc = "Tags for categorization. Must not be null."]
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        tags: ::std::vec::Vec<::std::string::String>,
    },
}
#[doc = "`SpecProject`"]
#[derive(
    :: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, Default, schemars :: JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct SpecProject {
    #[doc = "OPTIONAL. The workspace-relative path to the main Markdown document for this specification. When present, the value MUST NOT be null and MUST reference an existing file; this is the only document scanned for constraint groups. When absent, tooling MUST NOT assume a default filename."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub index: ::std::option::Option<::std::string::String>,
    #[doc = "A list of dependencies on other specifications. May be absent or an empty array, but must not be null."]
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub references: ::std::vec::Vec<GitReference>,
}
#[doc = "`UtilizesEntry`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, schemars :: JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UtilizesEntry {
    #[doc = "A list of regex patterns. Constraint groups from the utilized implementation that match any of these patterns are filtered out and ignored during compliance evaluation. May be absent or an empty array, but must not be null."]
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub ignore_constraints: ::std::vec::Vec<::std::string::String>,
    #[doc = "The implementation being utilized. The referenced artifact must be an implementation."]
    #[serde(rename = "ref")]
    pub ref_: GitReference,
}
#[doc = " Error types."]
pub mod error {
    #[doc = r" Error from a `TryFrom` or `FromStr` implementation."]
    pub struct ConversionError(::std::borrow::Cow<'static, str>);
    impl ::std::error::Error for ConversionError {}
    impl ::std::fmt::Display for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl ::std::fmt::Debug for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Debug::fmt(&self.0, f)
        }
    }
    impl From<&'static str> for ConversionError {
        fn from(value: &'static str) -> Self {
            Self(value.into())
        }
    }
    impl From<String> for ConversionError {
        fn from(value: String) -> Self {
            Self(value.into())
        }
    }
}

/// Backwards-compatible alias for the root manifest type generated by `typify`.
pub type SpecManProject = SpecManProjectManifest;

impl SpecManProjectManifest {
    /// The project name (`name`).
    pub fn name(&self) -> &str {
        match self {
            SpecManProjectManifest::Variant0 { name, .. }
            | SpecManProjectManifest::Variant1 { name, .. } => name,
        }
    }

    /// The project description (`description`), if present.
    pub fn description(&self) -> Option<&str> {
        match self {
            SpecManProjectManifest::Variant0 { description, .. }
            | SpecManProjectManifest::Variant1 { description, .. } => description.as_deref(),
        }
    }

    /// The project tags (`tags`).
    pub fn tags(&self) -> &[String] {
        match self {
            SpecManProjectManifest::Variant0 { tags, .. }
            | SpecManProjectManifest::Variant1 { tags, .. } => tags,
        }
    }

    /// The dependency-resolution overrides (`overrides`).
    pub fn overrides(&self) -> &[OverrideEntry] {
        match self {
            SpecManProjectManifest::Variant0 { overrides, .. }
            | SpecManProjectManifest::Variant1 { overrides, .. } => overrides,
        }
    }

    /// The child `spec` object when this project is a specification.
    pub fn spec(&self) -> Option<&SpecProject> {
        match self {
            SpecManProjectManifest::Variant0 { spec, .. } => Some(spec),
            SpecManProjectManifest::Variant1 { .. } => None,
        }
    }

    /// The child `impl` object when this project is an implementation.
    pub fn impl_(&self) -> Option<&ImplProject> {
        match self {
            SpecManProjectManifest::Variant1 { impl_, .. } => Some(impl_),
            SpecManProjectManifest::Variant0 { .. } => None,
        }
    }

    /// Whether this project is a specification (`spec` present).
    pub fn is_spec(&self) -> bool {
        self.spec().is_some()
    }

    /// Whether this project is an implementation (`impl` present).
    pub fn is_impl(&self) -> bool {
        self.impl_().is_some()
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
