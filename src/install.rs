//! Dependency resolution, materialization, and lockfile management.
//!
//! This module implements [SpecMan Installing][spec-install] and [SpecMan Lockfile][spec-lock]:
//! it resolves every git-referenced dependency of a project, materializes each one as a full
//! physical copy under `spec/{alias}/` (with transitive dependencies nested as
//! `spec/{alias}/spec/{sub-alias}/`), and records the fully resolved graph in `specman.lock`.
//!
//! All git operations are delegated to the user's `git` CLI (clone and `ls-remote`), piggybacking
//! on their existing authentication and configuration. No symlinks or shared object stores are
//! used: every reference is a byte-for-byte copy of its repository at the exact resolved commit.
//!
//! [spec-install]: ../../spec/specman/spec.md#concept-specman-installing
//! [spec-lock]: ../../spec/specman/spec.md#concept-specman-lockfile

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::core::error::SpecmanError;
use crate::manifest::{
    GitReference, ImplProject, LockedEntry, Lockfile, SpecManProjectManifest, SpecProject,
};
use crate::workspace::WorkspacePaths;

/// The lockfile format version supported by this implementation.
pub const LOCKFILE_VERSION: i64 = 1;

/// Maximum allowed recursion depth when resolving transitive dependencies.
const MAX_RECURSION_DEPTH: usize = 10;

/// The name of the project manifest at the workspace root, alongside which `specman.lock` lives.
const MANIFEST_FILENAME: &str = "specman.json";

/// The name of the lockfile.
const LOCKFILE_FILENAME: &str = "specman.lock";

/// Splits a git URL into `(host, path)`, ignoring the scheme and `.git` suffix.
///
/// This returns a raw `{host}/{path}` decomposition used by [`GitReference::site`]; prefer the
/// public [`GitReference::site`] method for conflict/identity comparisons, which also strips a
/// trailing slash from the path.
pub fn parse_git_url(url: &str) -> (String, String) {
    let without_scheme = url
        .trim_start_matches("https://")
        .trim_start_matches("git@")
        .trim_end_matches(".git");

    let (host, path) = if let Some((h, p)) = without_scheme.split_once(':') {
        // SSH form: git@host:owner/repo(.git)
        (h.to_string(), p.to_string())
    } else {
        // HTTPS form: https://host/owner/repo(.git)
        match without_scheme.split_once('/') {
            Some((h, p)) => (h.to_string(), p.to_string()),
            None => (without_scheme.to_string(), String::new()),
        }
    };

    (host, path.trim_end_matches('/').to_string())
}

/// Returns `true` when `reference` is already a full 40-character commit hash.
fn is_commit_hash(reference: &str) -> bool {
    reference.len() == 40 && reference.chars().all(|c| c.is_ascii_hexdigit())
}

/// Resolves a tag reference to a full commit hash, or returns the hash unchanged when the ref is
/// already a 40-character hex string. Delegates to `git ls-remote --tags`.
///
/// The `locked_hash` parameter, when `Some`, short-circuits resolution entirely: installs driven
/// by a lockfile MUST use the locked hash rather than dynamically re-resolving tags.
pub fn resolve_commit(
    url: &str,
    reference: &str,
    locked_hash: Option<&str>,
) -> Result<String, SpecmanError> {
    if let Some(hash) = locked_hash {
        return Ok(hash.to_string());
    }

    if is_commit_hash(reference) {
        return Ok(reference.to_string());
    }

    let output = Command::new("git")
        .args(["ls-remote", "--tags", url, reference])
        .output()
        .map_err(|err| SpecmanError::Dependency(format!("failed to run git ls-remote: {err}")))?;

    if !output.status.success() {
        return Err(SpecmanError::Dependency(format!(
            "failed to resolve tag `{reference}` for {url}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find_map(|line| {
            let hash = line.split_whitespace().next()?;
            (is_commit_hash(hash)).then(|| hash.to_string())
        })
        .ok_or_else(|| {
            SpecmanError::Dependency(format!(
                "tag `{reference}` did not resolve to a commit for {url}"
            ))
        })
}

/// A single reference encountered during resolution, together with the context needed to produce
/// conflict diagnostics: its nested filesystem path and the alias that declared it.
#[derive(Debug, Clone)]
struct TrackedReference {
    site: String,
    requested: String,
    resolved: String,
    /// Nested materialization path relative to the workspace root (e.g. `spec/B/spec/D/`).
    path: String,
    /// The alias of the artifact that declared this reference (the parent, or `(root)` for direct).
    declared_by: String,
    alias: String,
    url: String,
    transitive: bool,
}

/// Mutable resolution state accumulated while walking the dependency graph.
struct ResolutionContext {
    /// Completed lockfile entries, in discovery order.
    entries: Vec<LockedEntry>,
    /// Conflict tracking keyed by site.
    by_site: HashMap<String, Vec<TrackedReference>>,
    /// Top-level overrides keyed by site (applied before conflict detection).
    overrides: HashMap<String, String>,
}

impl ResolutionContext {
    fn new(overrides: &[crate::manifest::OverrideEntry]) -> Self {
        let mut map = HashMap::new();
        for entry in overrides {
            map.insert(entry.site.to_string(), entry.ref_.to_string());
        }
        Self {
            entries: Vec::new(),
            by_site: HashMap::new(),
            overrides: map,
        }
    }

    /// Returns the override `ref` (tag or hash) for `site`, if any.
    fn override_ref(&self, site: &str) -> Option<&str> {
        self.overrides.get(site).map(|s| s.as_str())
    }

    /// Records a resolution and, if an override pins the site, forces the resolved hash to the
    /// override target. Overrides are applied *before* conflict detection.
    fn record(&mut self, tracked: TrackedReference) -> Result<(), SpecmanError> {
        let resolved: crate::manifest::LockedEntryResolved =
            tracked
                .resolved
                .parse()
                .map_err(|e: crate::manifest::error::ConversionError| {
                    SpecmanError::Serialization(format!("invalid resolved commit hash: {e}"))
                })?;
        let entry = LockedEntry {
            alias: tracked.alias.clone(),
            path: tracked.path.clone(),
            url: tracked.url.clone(),
            requested: tracked.requested.clone(),
            resolved,
            transitive: tracked.transitive,
        };
        self.entries.push(entry);

        let site = tracked.site.clone();
        let group = self.by_site.entry(site.clone()).or_default();
        group.push(tracked);

        // Enforce conflict detection only after recording this reference.
        self.check_conflicts(&site)?;
        Ok(())
    }

    /// Determines the effective resolved hash for `site` given a naturally resolved hash.
    /// When an override pins the site, its `ref` (a tag or hash) is resolved against `url` and
    /// takes precedence.
    fn effective_resolved(
        &self,
        site: &str,
        url: &str,
        natural: String,
    ) -> Result<String, SpecmanError> {
        match self.override_ref(site) {
            Some(override_ref) => resolve_commit(url, override_ref, None),
            None => Ok(natural),
        }
    }

    /// Checks whether multiple references to `site` resolve to distinct hashes.
    fn check_conflicts(&self, site: &str) -> Result<(), SpecmanError> {
        let refs = match self.by_site.get(site) {
            Some(refs) => refs,
            None => return Ok(()),
        };
        if refs.len() < 2 {
            return Ok(());
        }

        let distinct: std::collections::BTreeSet<&str> =
            refs.iter().map(|r| r.resolved.as_str()).collect();
        if distinct.len() <= 1 {
            return Ok(());
        }

        // Build a readable conflict report.
        let mut msg = format!(
            "version conflict for artifact site `{site}`: multiple references resolve to different commits:"
        );
        for r in refs {
            msg.push_str(&format!(
                "\n  - {} at `{}` (declared by `{}`, requested `{}`)",
                r.resolved, r.path, r.declared_by, r.requested
            ));
        }
        msg.push_str("\n  add an `overrides` entry for this site to pin it to a single commit");
        Err(SpecmanError::Dependency(msg))
    }
}

/// Returns the direct references of a project. For implementations these are the
/// `implements` entries; for specifications they are the `references` entries. Every
/// reference points to a specification, since implementations no longer depend on other
/// implementations.
pub fn references_of(project: &SpecManProjectManifest) -> Vec<&GitReference> {
    match (project.spec(), project.impl_()) {
        (Some(SpecProject { references, .. }), None) => references.iter().collect(),
        (None, Some(ImplProject { implements })) => implements.iter().map(|e| &e.ref_).collect(),
        _ => Vec::new(),
    }
}

/// Loads and validates `specman.lock` at the workspace root, if it exists.
pub fn load_lockfile(workspace: &WorkspacePaths) -> Result<Option<Lockfile>, SpecmanError> {
    let path = workspace.root().join(LOCKFILE_FILENAME);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path).map_err(SpecmanError::Io)?;
    let lock: Lockfile = serde_json::from_str(&content).map_err(|err| {
        SpecmanError::Serialization(format!("invalid {LOCKFILE_FILENAME}: {err}"))
    })?;
    if lock.version != LOCKFILE_VERSION {
        return Err(SpecmanError::Serialization(format!(
            "unsupported {LOCKFILE_FILENAME} version {} (supported: {LOCKFILE_VERSION})",
            lock.version
        )));
    }
    Ok(Some(lock))
}

/// Reads the lockfile and returns, keyed by alias, the (url, resolved hash) for each direct
/// reference. Returns `None` when no lockfile is present. Detects and reports divergence between
/// the lockfile and `specman.json`.
fn locked_hashes(
    workspace: &WorkspacePaths,
    project: &SpecManProjectManifest,
) -> Result<Option<HashMap<String, (String, String)>>, SpecmanError> {
    let Some(lock) = load_lockfile(workspace)? else {
        return Ok(None);
    };

    let mut map = HashMap::new();
    for entry in &lock.references {
        if !entry.transitive {
            map.insert(
                entry.alias.to_string(),
                (entry.url.to_string(), entry.resolved.to_string()),
            );
        }
    }

    // Detect divergence: every direct reference in `specman.json` must have a matching lock entry
    // with the same URL. A missing entry or changed URL/alias means the lockfile is stale.
    for reference in references_of(project) {
        match map.get(reference.alias.as_str()) {
            None => {
                return Err(SpecmanError::Dependency(format!(
                    "{LOCKFILE_FILENAME} is missing entry for `{}`; run an update to regenerate the lockfile",
                    reference.alias.as_str()
                )));
            }
            Some((locked_url, _hash)) if locked_url != reference.url.as_str() => {
                return Err(SpecmanError::Dependency(format!(
                    "{LOCKFILE_FILENAME} entry for `{}` has url `{locked_url}` but {MANIFEST_FILENAME} declares `{}`; run an update to regenerate the lockfile",
                    reference.alias.as_str(),
                    reference.url.as_str()
                )));
            }
            Some(_) => {}
        }
    }

    // Reject lock entries that don't correspond to any direct reference (alias/url mismatch).
    for entry in &lock.references {
        if !entry.transitive {
            let matches = references_of(project).iter().any(|r| {
                r.alias.as_str() == entry.alias.as_str() && r.url.as_str() == entry.url.as_str()
            });
            if !matches {
                return Err(SpecmanError::Dependency(format!(
                    "{LOCKFILE_FILENAME} has a stale direct entry for `{}` ({}) not present in {MANIFEST_FILENAME}",
                    entry.alias, entry.url
                )));
            }
        }
    }

    Ok(Some(map))
}

/// Writes the lockfile atomically: content is written to a temporary sibling and renamed into
/// place, so a partially written file is never observable.
fn write_lockfile(workspace: &WorkspacePaths, entries: &[LockedEntry]) -> Result<(), SpecmanError> {
    let lock = Lockfile {
        version: LOCKFILE_VERSION,
        references: entries.to_vec(),
    };
    let content = serde_json::to_string_pretty(&lock)
        .map_err(|err| SpecmanError::Serialization(err.to_string()))?;

    let dir = workspace.root();
    fs::create_dir_all(dir).map_err(SpecmanError::Io)?;
    let final_path = dir.join(LOCKFILE_FILENAME);
    let tmp_path = dir.join(format!(".{LOCKFILE_FILENAME}.tmp"));

    fs::write(&tmp_path, content).map_err(SpecmanError::Io)?;
    fs::rename(&tmp_path, &final_path).map_err(SpecmanError::Io)?;

    Ok(())
}

/// Materializes a single reference as a full physical copy under `dest_dir` (which must already be
/// the correct nested `spec/.../` path, minus the trailing `specman.json`). Returns the path to the
/// materialized directory containing the artifact's `specman.json`.
fn materialize_reference(
    reference: &GitReference,
    commit: &str,
    dest_dir: &Path,
) -> Result<PathBuf, SpecmanError> {
    // The destination directory must not already be a populated copy; re-resolution replaces it.
    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir).map_err(SpecmanError::Io)?;
    }
    let parent = dest_dir
        .parent()
        .ok_or_else(|| SpecmanError::Workspace("invalid materialization path".into()))?;
    fs::create_dir_all(parent).map_err(SpecmanError::Io)?;

    let dest_str = dest_dir.to_string_lossy().into_owned();
    let clone = Command::new("git")
        .args(["clone", reference.url.as_str(), dest_str.as_str()])
        .status()
        .map_err(|err| SpecmanError::Dependency(format!("failed to run git clone: {err}")))?;
    if !clone.success() {
        return Err(SpecmanError::Dependency(format!(
            "failed to clone {}",
            reference.url.as_str()
        )));
    }

    let checkout = Command::new("git")
        .args(["-C", dest_str.as_str(), "checkout", commit])
        .status()
        .map_err(|err| SpecmanError::Dependency(format!("failed to run git checkout: {err}")))?;
    if !checkout.success() {
        return Err(SpecmanError::Dependency(format!(
            "failed to checkout {commit} for {}",
            reference.url.as_str()
        )));
    }

    // Remove the nested `.git` directory so each copy is a plain physical snapshot, not a shared
    // store or symlink. This also prevents accidental nested-git misinterpretation.
    let dot_git = dest_dir.join(".git");
    if dot_git.exists() {
        fs::remove_dir_all(&dot_git).map_err(SpecmanError::Io)?;
    }

    Ok(dest_dir.to_path_buf())
}

/// The result of a full install: the lockfile entries plus the materialized root paths.
#[derive(Debug)]
pub struct InstallResult {
    /// Lockfile entries for every direct and transitive reference.
    pub entries: Vec<LockedEntry>,
    /// Materialization directories (each containing a `specman.json`).
    pub materialized: Vec<PathBuf>,
}

/// Installs all git-referenced dependencies of `project`, materializing them in-tree under
/// `spec/{alias}/`, enforcing conflict detection and overrides, and writing `specman.lock`.
///
/// When a `specman.lock` is present, the locked hashes take precedence over tag re-resolution and
/// any divergence from `specman.json` causes this function to fail rather than silently overwrite.
pub fn install(
    project: &SpecManProjectManifest,
    workspace: &WorkspacePaths,
) -> Result<InstallResult, SpecmanError> {
    let locked = locked_hashes(workspace, project)?;
    let mut ctx = ResolutionContext::new(project.overrides());
    let mut materialized = Vec::new();

    let spec_root = workspace.root().join("spec");
    fs::create_dir_all(&spec_root).map_err(SpecmanError::Io)?;

    install_recursive(
        project,
        workspace,
        spec_root,
        project.name(),
        0,
        &mut ctx,
        &mut materialized,
        &locked,
        &["spec".to_string()],
    )?;

    write_lockfile(workspace, &ctx.entries)?;

    Ok(InstallResult {
        entries: ctx.entries,
        materialized,
    })
}

#[allow(clippy::too_many_arguments)]
fn install_recursive(
    project: &SpecManProjectManifest,
    workspace: &WorkspacePaths,
    base_dir: PathBuf,
    declaring_alias: &str,
    depth: usize,
    ctx: &mut ResolutionContext,
    materialized: &mut Vec<PathBuf>,
    locked: &Option<HashMap<String, (String, String)>>,
    chain: &[String],
) -> Result<(), SpecmanError> {
    if depth >= MAX_RECURSION_DEPTH {
        let mut chain_desc = chain.to_vec();
        chain_desc.push(declaring_alias.to_string());
        return Err(SpecmanError::Dependency(format!(
            "dependency resolution exceeded {MAX_RECURSION_DEPTH} levels; chain: {}",
            chain_desc.join(" -> ")
        )));
    }

    for reference in references_of(project) {
        let site = reference.site();
        let natural = resolve_commit(
            reference.url.as_str(),
            &reference.ref_,
            locked
                .as_ref()
                .and_then(|m| m.get(reference.alias.as_str()).map(|(_, h)| h.as_str())),
        )?;
        let resolved = ctx.effective_resolved(&site, reference.url.as_str(), natural)?;

        // Materialization path: for a direct reference (depth 0) it is `spec/{alias}/`; for a
        // transitive reference it is `{base_dir}/{alias}` where `base_dir` already includes the
        // parent's `spec/` segment chain per the spec (`spec/B/spec/D/`). The relative path is
        // built as `spec/{alias}` + `/{sub-alias}` + ... by carrying a segment accumulator.
        let alias = reference.alias.as_str();
        let dest_dir = base_dir.join(alias);
        let mut path_segments = chain.to_vec();
        path_segments.push(alias.to_string());
        let rel_path = path_segments.join("/");

        materialize_reference(reference, &resolved, &dest_dir)?;

        let nested_manifest = dest_dir.join(MANIFEST_FILENAME);
        if !nested_manifest.is_file() {
            return Err(SpecmanError::Dependency(format!(
                "materialized artifact `{alias}` has no {MANIFEST_FILENAME}"
            )));
        }
        let nested = crate::manifest_ops::load_manifest(&nested_manifest).map_err(|err| {
            SpecmanError::Dependency(format!(
                "failed to load {}: {err}",
                nested_manifest.display()
            ))
        })?;

        let tracked = TrackedReference {
            site,
            requested: reference.ref_.clone(),
            resolved: resolved.clone(),
            path: rel_path,
            declared_by: declaring_alias.to_string(),
            alias: alias.to_string(),
            url: reference.url.to_string(),
            transitive: depth > 0,
        };
        ctx.record(tracked)?;

        materialized.push(dest_dir.clone());

        // Recurse into the nested artifact's `spec/` subtree. The relative path accumulator now
        // becomes `spec/{alias}` and the next level appends `/spec` + `/{sub-alias}`.
        let mut next_chain = path_segments.clone();
        next_chain.push("spec".to_string());
        let nested_spec_root = dest_dir.join("spec");
        install_recursive(
            &nested,
            workspace,
            nested_spec_root,
            alias,
            depth + 1,
            ctx,
            materialized,
            locked,
            &next_chain,
        )?;
    }

    Ok(())
}

/// Materializes a single reference into `spec/{alias}/` and returns the directory containing the
/// artifact's `specman.json`. This is a lower-level primitive than [`install`], which also
/// resolves the transitive graph, applies `overrides`, and writes the lockfile.
pub fn ensure_reference(
    reference: &GitReference,
    workspace: &WorkspacePaths,
) -> Result<PathBuf, SpecmanError> {
    let commit = resolve_commit(reference.url.as_str(), &reference.ref_, None)?;
    let dest = workspace.root().join("spec").join(reference.alias.as_str());
    materialize_reference(reference, &commit, &dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::OverrideEntry;

    fn hash(n: u8) -> String {
        format!("{n:040x}")
    }

    fn tracked(site: &str, resolved: &str, path: &str, alias: &str) -> TrackedReference {
        TrackedReference {
            site: site.to_string(),
            requested: "tag".to_string(),
            resolved: resolved.to_string(),
            path: path.to_string(),
            declared_by: "root".to_string(),
            alias: alias.to_string(),
            url: format!("https://example.com/{site}.git"),
            transitive: false,
        }
    }

    #[test]
    fn parse_git_url_strips_scheme_and_git_suffix() {
        assert_eq!(
            parse_git_url("https://example.com/owner/repo.git"),
            ("example.com".to_string(), "owner/repo".to_string())
        );
        assert_eq!(
            parse_git_url("git@example.com:owner/repo.git"),
            ("example.com".to_string(), "owner/repo".to_string())
        );
    }

    #[test]
    fn lockfile_roundtrips_through_json() {
        let entries = vec![LockedEntry {
            alias: "B".to_string(),
            path: "spec/B/".to_string(),
            url: "https://example.com/owner/B.git".to_string(),
            requested: "v1".to_string(),
            resolved: hash(0x0B).parse().unwrap(),
            transitive: false,
        }];
        let lock = Lockfile {
            version: LOCKFILE_VERSION,
            references: entries,
        };
        let json = serde_json::to_string_pretty(&lock).unwrap();
        let parsed: Lockfile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, LOCKFILE_VERSION);
        assert_eq!(parsed.references.len(), 1);
        assert_eq!(parsed.references[0].resolved.to_string(), hash(0x0B));
    }

    #[test]
    fn resolved_hash_enforces_40_hex() {
        assert_eq!(
            hash(0x0B)
                .parse::<crate::manifest::LockedEntryResolved>()
                .unwrap()
                .to_string(),
            hash(0x0B)
        );
        assert!(
            "not-a-hash"
                .parse::<crate::manifest::LockedEntryResolved>()
                .is_err()
        );
    }

    #[test]
    fn conflict_detection_reports_distinct_hashes() {
        let mut ctx = ResolutionContext::new(&[]);
        ctx.record(tracked("example.com/owner/repo", &hash(1), "spec/A/", "A"))
            .unwrap();
        let err = ctx
            .record(tracked("example.com/owner/repo", &hash(2), "spec/B/", "B"))
            .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("version conflict"), "{msg}");
        assert!(msg.contains("example.com/owner/repo"), "{msg}");
        assert!(msg.contains(&hash(1)), "{msg}");
        assert!(msg.contains(&hash(2)), "{msg}");
        assert!(msg.contains("overrides"), "{msg}");
    }

    #[test]
    fn same_site_same_hash_is_not_a_conflict() {
        let mut ctx = ResolutionContext::new(&[]);
        ctx.record(tracked("example.com/owner/repo", &hash(1), "spec/A/", "A"))
            .unwrap();
        ctx.record(tracked("example.com/owner/repo", &hash(1), "spec/B/", "B"))
            .unwrap();
        assert_eq!(ctx.entries.len(), 2);
    }

    #[test]
    fn override_pins_site_to_single_hash() {
        let overrides = vec![OverrideEntry {
            site: "example.com/owner/repo".parse().unwrap(),
            ref_: hash(9).parse().unwrap(),
        }];
        let ctx = ResolutionContext::new(&overrides);
        let resolved = ctx
            .effective_resolved(
                "example.com/owner/repo",
                "https://example.com/owner/repo.git",
                hash(1),
            )
            .unwrap();
        assert_eq!(resolved, hash(9));
    }
}
