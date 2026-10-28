# AGENTS.md

OpenStoryRS is a Rust v83 MapleStory client. Keep changes focused and preserve
legacy server behavior, the 1024x768 logical canvas, resize scaling, and NX paths.

## Architecture

Put behavior in its owning crate; avoid cross-layer dependencies:

- `openstory-assets`: NX data; `openstory-protocol`: v83 packets and crypto.
- `openstory-core`: game state/simulation; no window or graphics APIs.
- `openstory-render`: GPU rendering; `openstory-client`: app, UI, input, audio.

## Working rules

- Inspect only relevant files and nearby patterns; use `README.md` and
  `PORT_PLAN.md` for details when needed. Avoid broad unrelated edits.
- Preserve existing user changes. Treat adjacent `OpenStory` C++ as read-only.
- Read NX assets from the existing data directory; never copy or bundle them.
- Keep packet reads bounded; malformed/truncated input must not panic.
- For UI changes, render the relevant UI-preview case and check it against the
  requested result; use UI-previews whenever available.
- Update docs only when behavior or compatibility details changed.
- Always write UI previews to `target/ui-previews` folder.

Useful checks: `cargo fmt --all -- --check`, `cargo check --workspace`,
`cargo test --workspace`. Use `OPENSTORY_DATA_DIR=/path/to/wz` (or `--data-dir`)
for local NX assets. See `README.md` for other runtime options.
