# AGENTS.md

## Project

OpenStoryRS is a Rust port of the v83 MapleStory client. Preserve compatibility
with the legacy client and server while keeping the 1024x768 logical game canvas,
resizable-window scaling, and the existing NX asset paths.

## Workspace

This is a Cargo workspace. Keep responsibilities in the existing crates:

- `openstory-assets`: NX archives and asset loading.
- `openstory-protocol`: packet formats, framing, encryption, and protocol data.
- `openstory-core`: game state and simulation, independent of window/graphics APIs.
- `openstory-render`: GPU rendering and viewport transforms.
- `openstory-client`: application lifecycle, native UI, input, audio, and composition.

Prefer adding code to the crate that owns the behavior instead of widening
dependencies between layers. Keep platform-specific APIs behind the existing
platform boundaries where practical.

## Development

- Preserve existing work in the working tree. Do not discard or overwrite user
  changes when editing or formatting files.
- Treat the adjacent `OpenStory` C++ repository as a read-only reference. Do not
  edit, reset, or reformat it.
- Read the needed `.nx` files from the existing data directory; do not copy or
  bundle the multi-gigabyte asset set in this repository.
- Keep packet parsing bounded and handle malformed or truncated input without
  panicking. Preserve wire-compatible v83 behavior.
- Follow nearby Rust style and keep changes focused. Update `README.md` or
  `PORT_PLAN.md` when a user-visible capability or compatibility finding makes
  their descriptions stale.

## Useful commands

- `cargo fmt --all -- --check` checks formatting.
- `cargo check --workspace` checks the workspace build.
- `cargo test --workspace` runs workspace tests.

The native client can use `OPENSTORY_DATA_DIR=/path/to/wz` or `--data-dir` to
locate the existing NX assets. See `README.md` for runtime settings and launch
options.
