#!/usr/bin/env sh
#
# Regenerate the typify-derived data models from the canonical JSON Schema.
#
# `src/manifest.rs` is produced by `cargo-typify` from
# `spec/specman/models/specman.schema.json`. This script pins the exact
# invocation so the generated types are reproducible across runs and machines.
#
# The generator is the `cargo-typify` crate's `cargo typify` subcommand. It is
# NOT part of Cargo and NOT a dependency of this crate — it must be installed
# separately:
#
#     cargo install cargo-typify --version 0.8.0
#
# This script checks for it first and fails with a clear message if it is absent.
#
# IMPORTANT: `cargo typify` only emits the schema-derived types into `src/manifest.rs`,
# and that file is now pure generated output (no hand-edits). All hand-written utilities
# live separately in `src/manifest_ops.rs`, which is NOT touched by this script. So
# regeneration safe to run and does NOT destroy hand-written code.
#
# Usage: scripts/regenerate-types.sh

set -eu

SCHEMA="spec/specman/models/specman.schema.json"
OUTPUT="src/manifest.rs"
TYPIFY_VERSION="0.8.0"

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo not found on PATH" >&2
    exit 1
fi

if ! cargo typify --version >/dev/null 2>&1; then
    echo "error: 'cargo typify' is not installed." >&2
    echo "" >&2
    echo "It is the cargo-typify crate's subcommand, not part of Cargo." >&2
    echo "Install it with (pinned to the version this repo uses):" >&2
    echo "" >&2
    echo "    cargo install cargo-typify --version ${TYPIFY_VERSION}" >&2
    echo "" >&2
    exit 1
fi

# `cargo typify` is the cargo-typify subcommand (crate `cargo-typify`).
cargo typify \
    --no-builder \
    --additional-derive 'schemars::JsonSchema' \
    "${SCHEMA}" \
    -o "${OUTPUT}"

echo "Regenerated ${OUTPUT} from ${SCHEMA}."
