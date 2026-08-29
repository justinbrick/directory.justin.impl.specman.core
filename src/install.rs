use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::core::error::SpecmanError;
use crate::manifest::{GitReference, ImplProject, SpecManProject, SpecProject};
use crate::workspace::WorkspacePaths;

const MAX_RECURSION_DEPTH: usize = 10;

/// Returns the global reference cache root: `$HOME/.specman/ref`.
pub fn cache_root() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".specman").join("ref")
}

/// Computes the cache directory name `{host}_{path}_{hash}`.
pub fn ref_name(host: &str, path: &str, hash: &str) -> String {
    format!("{host}_{path}_{hash}")
}

/// Splits a git URL into `(host, path)`, ignoring the scheme and `.git` suffix.
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

/// Resolves a tag reference to a full commit hash, or returns the hash unchanged when the ref
/// is already a 40-character hex string.
pub fn resolve_commit(url: &str, reference: &str) -> Result<String, SpecmanError> {
    if reference.len() == 40 && reference.chars().all(|c| c.is_ascii_hexdigit()) {
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
            (hash.len() == 40 && hash.chars().all(|c| c.is_ascii_hexdigit()))
                .then(|| hash.to_string())
        })
        .ok_or_else(|| {
            SpecmanError::Dependency(format!(
                "tag `{reference}` did not resolve to a commit for {url}"
            ))
        })
}

/// Ensures a `GitReference` is present in the global cache and symlinked into the workspace's
/// `.specman/ref/` directory. Returns the local cache path containing the artifact's `specman.json`.
pub fn ensure_reference(
    reference: &GitReference,
    workspace: &WorkspacePaths,
) -> Result<PathBuf, SpecmanError> {
    let (host, path) = parse_git_url(&reference.url);
    let commit = resolve_commit(&reference.url, &reference.ref_)?;
    let name = ref_name(&host, &path, &commit);
    let root = cache_root();
    let cache_dir = root.join(&name);

    if !cache_dir.is_dir() {
        let parent = cache_dir.parent().unwrap_or(root.as_path()).to_path_buf();
        fs::create_dir_all(&parent).map_err(SpecmanError::Io)?;

        let cache_dir_str = cache_dir.to_string_lossy().into_owned();
        let clone = Command::new("git")
            .args(["clone", reference.url.as_str(), cache_dir_str.as_str()])
            .status()
            .map_err(|err| SpecmanError::Dependency(format!("failed to run git clone: {err}")))?;
        if !clone.success() {
            return Err(SpecmanError::Dependency(format!(
                "failed to clone {}",
                reference.url
            )));
        }

        let checkout = Command::new("git")
            .args(["-C", cache_dir_str.as_str(), "checkout", commit.as_str()])
            .status()
            .map_err(|err| {
                SpecmanError::Dependency(format!("failed to run git checkout: {err}"))
            })?;
        if !checkout.success() {
            return Err(SpecmanError::Dependency(format!(
                "failed to checkout {commit} for {}",
                reference.url
            )));
        }
    }

    // Create or update the local alias symlink.
    let ref_dir = workspace.ref_dir();
    fs::create_dir_all(&ref_dir).map_err(SpecmanError::Io)?;
    let link = ref_dir.join(&reference.alias);
    let _ = fs::remove_file(&link);
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(&cache_dir, &link).map_err(|err| {
            SpecmanError::Workspace(format!(
                "failed to symlink {} -> {}: {}",
                link.display(),
                cache_dir.display(),
                err
            ))
        })?;
    }
    #[cfg(not(unix))]
    {
        fs::copy(cache_dir.join("specman.json"), &link).map_err(|err| SpecmanError::Io(err))?;
    }

    Ok(cache_dir)
}

fn references_of(project: &SpecManProject) -> Vec<&GitReference> {
    match (&project.spec, &project.impl_) {
        (Some(SpecProject { references, .. }), None) => references.iter().collect(),
        (
            None,
            Some(ImplProject {
                implements,
                utilizes,
            }),
        ) => implements
            .iter()
            .map(|e| &e.ref_)
            .chain(utilizes.iter().map(|u| &u.ref_))
            .collect(),
        _ => Vec::new(),
    }
}

/// Recursively resolves every git-referenced dependency of a project, cloning as needed.
pub fn install(
    project: &SpecManProject,
    workspace: &WorkspacePaths,
) -> Result<Vec<PathBuf>, SpecmanError> {
    let mut resolved = Vec::new();
    install_recursive(project, workspace, 0, &mut resolved)?;
    Ok(resolved)
}

fn install_recursive(
    project: &SpecManProject,
    workspace: &WorkspacePaths,
    depth: usize,
    resolved: &mut Vec<PathBuf>,
) -> Result<(), SpecmanError> {
    if depth > MAX_RECURSION_DEPTH {
        return Err(SpecmanError::Dependency(format!(
            "dependency resolution exceeded {MAX_RECURSION_DEPTH} levels"
        )));
    }

    for reference in references_of(project) {
        let cache_dir = ensure_reference(reference, workspace)?;
        let manifest_path = cache_dir.join("specman.json");
        let nested = crate::manifest::load_manifest(&manifest_path).map_err(|err| {
            SpecmanError::Dependency(format!(
                "failed to load {}: {}",
                manifest_path.display(),
                err
            ))
        })?;
        install_recursive(&nested, workspace, depth + 1, resolved)?;
        resolved.push(cache_dir);
    }

    Ok(())
}
