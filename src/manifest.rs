#![allow(clippy::redundant_closure_call)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::match_single_binding)]
#![allow(clippy::clone_on_copy)]

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
#[doc = "An implementation project manifest."]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, schemars :: JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImplementationManifest {
    #[doc = "A human-readable description of the project."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub description: ::std::option::Option<::std::string::String>,
    #[serde(rename = "impl")]
    pub impl_: ImplProject,
    #[doc = "The project name. Any UTF-8 string; free-form and human-readable."]
    pub name: ::std::string::String,
    #[doc = "Explicit version resolutions applied during dependency resolution. May be absent or an empty array, but must not be null."]
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub overrides: ::std::vec::Vec<OverrideEntry>,
    #[doc = "A relative file URL pointing to this schema."]
    #[serde(
        rename = "$schema",
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub schema: ::std::option::Option<::std::string::String>,
    #[doc = "Tags for categorization. Must not be null."]
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub tags: ::std::vec::Vec<::std::string::String>,
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
#[serde(untagged)]
pub enum SpecManProjectManifest {
    SpecificationManifest(SpecificationManifest),
    ImplementationManifest(ImplementationManifest),
}
impl ::std::convert::From<SpecificationManifest> for SpecManProjectManifest {
    fn from(value: SpecificationManifest) -> Self {
        Self::SpecificationManifest(value)
    }
}
impl ::std::convert::From<ImplementationManifest> for SpecManProjectManifest {
    fn from(value: ImplementationManifest) -> Self {
        Self::ImplementationManifest(value)
    }
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
#[doc = "A specification project manifest."]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, schemars :: JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpecificationManifest {
    #[doc = "A human-readable description of the project."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub description: ::std::option::Option<::std::string::String>,
    #[doc = "The project name. Any UTF-8 string; free-form and human-readable."]
    pub name: ::std::string::String,
    #[doc = "Explicit version resolutions applied during dependency resolution. May be absent or an empty array, but must not be null."]
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub overrides: ::std::vec::Vec<OverrideEntry>,
    #[doc = "A relative file URL pointing to this schema."]
    #[serde(
        rename = "$schema",
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub schema: ::std::option::Option<::std::string::String>,
    pub spec: SpecProject,
    #[doc = "Tags for categorization. Must not be null."]
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub tags: ::std::vec::Vec<::std::string::String>,
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
