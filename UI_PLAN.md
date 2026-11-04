# UI rebuild plan

## Goal and constraints

Start the game UI completely fresh. Remove all current UI cases and rebuild them
to match `reference-images` as closely as possible, paying particular attention
to font shapes and weights, overflowing assets, and text alignment. Keep the
renderer and the concept of offline UI previews. Preserve the 1024x768 logical
canvas, resize scaling and letterboxing, existing NX paths, and legacy server
behavior. Follow `AGENTS.md` and keep behavior in its owning crate.

Read NX assets in place from the existing data directory; do not copy or bundle
the archives. The adjacent `OpenStory` C++ client is read-only. Preserve user
changes, including untracked files; review existing UI-specific artwork before
deciding whether any of it is obsolete.

## 1. Remove the current UI

1. Inventory UI rendering, layout, input, and preview entry points, including
   login and character selection, HUD, panels, pages, dialogs, and overlays.
   Identify the game state, settings, packet handling, and operations each uses.
2. Separate presentation and interaction code from the underlying game and
   server behavior. Retain the latter and its protocol contracts, including
   bounded reads of malformed or truncated packets.
3. Remove the existing UI cases, their layouts, input wiring, preview fixtures,
   aliases, and catalogue entries. The current `ui_preview.rs` directly invokes
   existing builders, so retain the preview capability but replace its case
   catalogue and fixtures with a clean, extensible foundation.
4. Keep the client buildable during the transition. Track temporarily unavailable
   screens and actions explicitly, and restore them as their replacements are
   connected. Do not silently change server behavior.

## 2. Build the foundation

- Inspect `UI.nx` in the existing data directory and catalogue the nodes needed
  for each case: bitmap dimensions, origins, transparency, layers, animation,
  and button states. Use other NX archives where appropriate for dynamic game
  content such as item icons, characters, and NPCs.
- Implement focused shared primitives for placement, clipping, scrolling,
  button states, text measurement, and hit testing. Rendering and input must
  share layout geometry. Respect authored sprite origins and bounds.
- Establish typography against the references before building the panels.
  Compare available font faces, weights, sizes, glyph shapes, baselines, line
  spacing, and rasterization. Preserve configurable font selection; document
  any unavoidable font availability or licensing limitations.
- Make previews deterministic and use the same UI code as the live client.
  Supply fixture data matching the references as closely as practical. Always
  write previews and comparison artifacts to `target/ui-previews`.
- Follow the exact naming convention and stems in `reference-images`, such as
  `hud-bottom-bar`, `panel-item-inventory-tabs`, and `page-cash-shop`. Produce
  reference-sized crops for comparison as well as full 1024x768 compositions
  and resized previews for placement and scaling checks.

## 3. Rebuild the reference cases

Build and verify the following cases, including separate fixtures for the states
represented by separate images:

1. HUD: `hud-bottom-bar`, `hud-chat`, `hud-quickslot`,
   `hud-minimap-collapsed`, `hud-minimap-normal`, `hud-minimap-expanded`.
2. Core panels: `panel-item-inventory`, `panel-item-inventory-tabs`,
   `panel-equipment-inventory`, `panel-item-tooltip`, `panel-skill-inventory`,
   `panel-stats`.
3. Interaction and settings: `panel-npc-chat`, `panel-npc-storage`,
   `panel-keyboard-settings`, `panel-system-options`, `panel-revive-character`.
4. Larger interfaces: `panel-quests`, `panel-monster-book`, `page-cash-shop`.

For each case:

1. Measure the reference and map its parts to NX assets and dynamic content.
2. Compose the artwork at the correct native positions and layer order; avoid
   stretching assets to conceal layout errors. Clip or scroll content at the
   intended boundaries, and prevent sprites from spilling into adjacent areas.
3. Match text position, baseline, alignment, width, color, weight, and spacing.
   Exercise long names, large values, empty content, wrapping, truncation, and
   scrolling without overflow or misalignment.
4. Render the relevant preview and visually compare it to the reference; use
   overlays or difference images where helpful. Correct discrepancies before
   marking the case complete.
5. Connect the same implementation to live state and input. Verify hitboxes,
   button states, focus, dragging, scrolling, and relevant server actions.

## 4. Restore cases without reference images

Rebuild all remaining necessary UI cases, including login, selection, social
interfaces, and other dialogs and overlays. Match the visual style established
by the reference-backed cases and consult the read-only OpenStory client for
layout, asset paths, and behavior. Give each case a consistently named preview
and record its visual basis; do not claim a direct reference match where there
is no reference image.

If an asset is truly missing from the available NX data, an appropriate
replacement may be generated to fit the established style. First verify that
the needed asset is absent, then record the replacement and where it is used.
Do not copy or bundle NX archives.

## 5. Completion checks

- Inspect every reference-backed preview at native size against its image,
  especially fonts, clipping, sprite overflow, and text alignment. Check full
  compositions and resize scaling, as well as interaction and edge cases.
- Verify that all removed cases have either been rebuilt or are explicitly
  tracked as outstanding, and that legacy server-facing behavior is preserved.
- Run `cargo fmt --all -- --check`, `cargo check --workspace`, and appropriate
  tests, including `cargo test --workspace` when feasible. Update documentation
  when behavior or compatibility details change.
