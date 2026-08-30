# Implementation — SpecMan Library Rust Crate

This page documents behavior that is specific to the SpecMan Rust reference implementation: concrete defaults, limits, on-disk layout, and edge cases you need to be aware of when using it. It is not a restatement of the [SpecMan Core](../../spec/specman-core/spec.md) concepts.

## Runtime Requirements

- The implementation does not speak git itself; it shells out to the system `git` CLI. `git` must be on `PATH` for reference resolution, cloning, and dependency traversal.
- It performs no git authentication itself; it relies on your existing credentials (SSH agent, credential helper, etc.).

## Workspace Rules

- A workspace root is the nearest directory that contains both a `.specman` directory and a sibling `.git` directory. Discovery walks upward from the start path until it finds both.
- Creating a workspace requires `git init` to have been run first; otherwise it errors.
- Nested workspaces are rejected: you cannot create a workspace inside an ancestor that already has `.specman`.
- Workspace resolution only accepts workspace-relative paths. Locator schemes such as `spec://`, `impl://`, `scratch://`, and `http(s)://` are rejected.

## Git Reference Resolution

- The `ref` field is a full 40-character commit hash or a git tag name. Branches are never consulted.
- A 40-character hex value is used as-is; anything else is resolved with `git ls-remote --tags`.
- The `url` field must begin with `https://` or `git@` (SSH). `http://` and other schemes are rejected.
- Dependency resolution recurses with a maximum depth of 10; deeper graphs fail with a descriptive error.

## On-Disk Cache and Symlinks

- Referenced artifacts are cloned into a global cache rooted at `$HOME/.specman/ref/`.
- Each cache entry is keyed by the host, the repository path (with any trailing `.git` removed), and the resolved commit hash. Because the hash is part of the key, the same commit is cloned once and reused; a tag that moves to a new commit produces a new clone.
- Clones are reused whenever their directory already exists, and nothing is ever cleaned up automatically — the cache grows over time.
- Inside a workspace, references appear as symlinks under `.specman/ref/{alias}` pointing into the cache. On non-Unix platforms, symlinking is not used; instead a copy of the referenced `specman.json` is placed at the alias path.

## Validation Anchors

- Validation anchors in source files use the syntax `[ENSURES: <identifier>:<type>]`, where `<identifier>` is the full dot-delimited constraint-group identifier.
- The `<type>` is `CHECK` or `MANUAL` (uppercase). Any other value — including `TEST`, lowercase, or an empty/omitted type — is treated as a test anchor.

## Compliance Scanning

- Source scanning is recursive and skips the `.git`, `.specman`, and `target` directories, plus any file that looks binary (a NUL byte within the first 8000 bytes).
- Only constraint groups from the specifications listed in `impl.implements` are considered; transitive specification dependencies are excluded.
- "Missing" means a governing constraint group has no matching anchor; "orphan" means an anchor references a group that is not governing. Orphans are reported but do **not** by themselves fail a workspace status check; missing anchors do.

## Reference Validation

- By default, validation is offline: HTTPS links are only checked for well-formedness. Reachability checking is opt-in.
- Reachability checks use a 10-second connect timeout and a 30-second overall timeout, following up to 5 redirects. A redirect (3xx) counts as success; a timeout is reported as a non-fatal warning; any other failure is an error.
- Only `https://` destinations are treated as remote. `http://` and any other scheme are reported as unsupported.
- Fragment (`#...`) references are validated only against Markdown targets (`.md` / `.markdown`, case-insensitive); fragments on other file types are ignored.
- Image destinations are ignored, and reference-style links are resolved against their definitions before validation.
- A transitive-validation option exists but is not implemented yet.

## Structure Indexing

- The body document is `spec.index` when present; otherwise `impl.md`. A specification with no `index` field has no indexed body — there is no `spec.md` fallback.
- Constraint-group declarations use the syntax `!<group>.<subgroup>:` at the start of a line; the identifier must contain at least one dot.
- Rendering a heading recursively follows fragment-only inline links to other headings, with deduplication, cycle protection, and a maximum recursion depth of 50.

## Templates

- Scenario templates resolve in this order: a workspace override under `.specman/templates/`, then the embedded default (which is materialized into the workspace cache).
- Remote (URL) templates are not supported yet.
- Rendering uses Handlebars-style token substitution with HTML escaping disabled.
- Specification templates require an `output_name` and a `title` token; implementation templates require only `output_name`.

## Create, Persist, and Delete

- Creating an artifact writes `specman.json` and its Markdown body at the repository root. Specification bodies go to `spec.md`; implementation bodies go to `impl.md`.
- Both create and delete support a dry-run flag: create returns the rendered template without writing, and delete returns a plan (with a `blocked` flag) without removing anything.
- Deletion is blocked when any non-optional downstream dependent exists, unless `force` is set. Removal deletes `specman.json`, `spec.md`, and `impl.md`.

## Workspace Status

- The aggregated status check has four independently toggleable categories — structure, references, cycles, and compliance — all enabled by default.
- Compliance checking runs only for implementation projects.
- A status check fails on structure errors, reference errors, dependency cycles, or missing compliance anchors. Orphan anchors alone do not fail it.

## Error Surface

- Failures are surfaced as structured errors rather than bare strings, so callers can branch on categories such as "deletion blocked" without parsing error messages.
