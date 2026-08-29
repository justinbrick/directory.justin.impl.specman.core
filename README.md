# SpecMan Runtime Library

SpecMan is the runtime foundation for authoring and automating software specifications. It powers workspace discovery, dependency mapping, templating, and lifecycle workflows that keep specs consistent and reproducible.

## What It Does

- Discovers workspace roots and canonical directories.
- Builds dependency graphs for specifications, implementations, and scratch pads.
- Resolves templates from embedded defaults and workspace pointer files.
- Renders Markdown templates deterministically.
- Provides lifecycle orchestration for planning, creating, and deleting artifacts.
- Exposes structured lifecycle failures so callers can branch programmatically instead of matching on error strings.

## Reference Validation

The library includes a reference validator for sanity-checking Markdown link destinations inside a workspace. It parses inline links, reference-style links, and autolinks, and ignores image destinations.

For each discovered destination, the validator classifies it as a workspace filesystem path, an HTTPS URL, or unsupported, then validates it accordingly:

- Filesystem targets are resolved relative to the source document, normalized, and required to stay inside the workspace root; canonical paths are re-checked when possible. Missing files are reported as errors.
- HTTPS destinations are checked for well-formed URLs; optional reachability checking treats redirects as success, reports timeouts as non-fatal diagnostics, and fails on non-timeout errors.
- Fragment references on Markdown targets are validated against heading slugs using deterministic slug rules.
- SpecMan handles found inside Markdown links are rejected.
- Results are deterministic and reported in source order, returning both discovered references and issues to the caller.

## Repository

All source, issue tracking, and release notes live in the main GitHub repository:

<https://github.com/justinbrick/directory.justin.impl.specman.core>

Star the repo or follow along for roadmap updates (including relationship graphing and expanded MCP tool coverage).
