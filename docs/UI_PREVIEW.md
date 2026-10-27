# Offline UI preview runner

The native client can render deterministic screenshots of login, character
selection, and gameplay windows without opening a window or connecting to a game
server. Login and character-flow screens use production scene builders. Gameplay
window previews use the original NX window layers and authored origins.

```sh
cargo run -p openstory-client -- \
  --data-dir ../OpenStory/wz \
  --ui-preview all \
  --out target/ui-previews \
  --ui-hitboxes
```

Available previews are `login`, `login-active`, `world-select`, `character-select-empty`,
`character-select-populated`, `character-create-explorer`, `native-keybindings`,
`native-items`, `native-equipment`, `native-skills`, `native-pet-equipment`,
`native-npc-store`, `native-cash-shop`, `native-monster-book`,
`native-monster-book-detail`, `native-status`,
`native-quest-log`,
`native-chat`, `native-party`, `native-buddy`, `native-guild`,
`native-alliance`, `native-world-map`, `native-world-map-search`,
`native-world-map-marker`, `native-world-map-result`,
`native-options`, `native-audio-settings`, and `all`. Character creation enters
the Explorer customization scene directly. No server credentials or network
connection are used.

World Map previews use the Victoria region; the search case finds Orbis across regions, the
marker case displays a map detail tooltip, and the result case shows the selected Orbis map.

Each case writes 1024x768 and 1440x900 PNGs. The widescreen output uses the
client's 1024x768 aspect-ratio-preserving viewport and letterboxing.
`--ui-hitboxes` also writes a 1024x768 image with colored outlines around
interactive regions for login-flow scenes and NX window bounds for gameplay
previews. Gameplay-window cases use the original NX window art. The Monster
Book detail preview composes the selected Snail card, monster stats, and NX-backed
drop icons over the original NX window. A `*-layout.txt` sidecar reports NX
layer pixels and hitbox bounds that extend beyond the logical canvas; gameplay
reports exclude the map's intentionally off-screen scenery from the window
bounds audit. The 1440x900 output provides a separate aspect-preserving viewport
check.

The preview renderer composites decoded BGRA sprites in software so it can run
without a display server. Texture sampling can differ slightly from WGPU's
linear sampler. These are static composition references, not interactive UI
simulations.
