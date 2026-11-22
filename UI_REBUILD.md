# UI rebuild progress

The implementation of `UI_PLAN.md` is **not complete**. This document separates
working replacements from the visual and interaction work still required. The
old live screens have not all been removed; secondary screens remain available
while their replacements are outstanding.

## Implemented foundation

- The offline catalogue uses explicit `hud-*`, `panel-*`, and `page-*` names.
  Old `native-*` names and friendly aliases were removed. `--help` lists the
  canonical catalogue from the same source used by preview dispatch.
- All 20 reference stems have fixtures, including the five-tab inventory strip.
  Preview output includes 1024x768 compositions, 1440x900 letterboxed views,
  reference-sized crops, side-by-side comparisons (reference left), differences,
  and optional hitbox images. Comparisons are omitted when reference images
  are not installed. Live rendering never reads `reference-images`.
- Comparison crops keep native pixels. The inventory-tab montage and Cash Shop
  are reduced to their supplied screenshot sizes; other panels are cropped
  without scaling their artwork to hide different native dimensions.
- `nx-assets.tsv` records bitmap dimensions, origins, animation delays and state
  paths for the relevant NX trees, including compatible v83 path resolution.
  Decoded artwork samples and typography proofs are preview artifacts only.
- Inventory, storage, quest-list and quickslot cells share geometry with input.
  Text clips against its requested rectangle, including negative origins.
- Arial is preferred from the operating system. Bundled Roboto remains the
  portable fallback. `OPENSTORY_FONT_PATH` still selects the regular face;
  `OPENSTORY_BOLD_FONT_PATH` selects the bold face. A custom regular font without
  a matching bold font uses regular headings instead of switching families.
  When Arial is unavailable, the Roboto fallback also keeps regular headings
  rather than mixing in Arial Bold. This keeps all popup text on one face family.
  Font identities are part of the bounded glyph-cache key.
- No proprietary font or NX archive was copied into the repository. Arial's
  availability is platform dependent; fontdue grayscale rasterization does not
  reproduce the original client's Windows font hinting/ClearType exactly.
- Window chrome and controls use original NX artwork. The item tooltip keeps
  its existing drawn 237x183 frame as the sole exception, matching the reference
  while `UIToolTip.img` is absent from the installed UI archive.

## Reference cases

| Case | Implemented presentation | Remaining work |
| --- | --- | --- |
| `hud-bottom-bar` | Full-width v83 bar with a widened level panel, native dark-panel chrome, individual amber level tiles, left-aligned character labels, shaded live HP/MP/EXP fills over metallic tracks, and right-aligned toolbar controls | Refine platform font rendering and match reference toolbar button states |
| `hud-chat` | Transparent nine-row history, native target/edge art, input strip | Pointer focus, scroll history and exact channel/frame alignment |
| `hud-quickslot` | Native plate, shared hit cells, default legends, live rebindings | Match custom key/icon placement and exercise drag/rebinding live |
| `hud-minimap-collapsed` | Compact native plate and native title | Reference-height border and disabled-control appearance |
| `hud-minimap-normal` | Corrected v83 six-pixel frame construction | Match reference map viewport/zoom and dimensions |
| `hud-minimap-expanded` | Corrected native frame, title and map marks | Match reference map viewport/zoom, title area and dimensions |
| `panel-item-inventory` | Fresh NX layers, tabs, clipped native icons/counts, live scrollbar and meso | Match exact fixture items, borders and tab styling; live drag verification |
| `panel-item-inventory-tabs` | Five fixtures using the live inventory renderer | Match reference item sets, tab-strip spacing and montage scaling |
| `panel-equipment-inventory` | Native v83 doll with corrected eye-accessory cell and populated face/eye/ear preview | Rebuild reference Equip/Deco/Beauty tabs, geometry, art and input |
| `panel-item-tooltip` | New equipment requirements/category/stat/upgrade layout and consume-item descriptions | Complete multi-stat equipment details and exact background/typography |
| `panel-skill-inventory` | Fresh native frame, live tabs/rows, clipped names, live SP and scrollbar | Match macro/scroll controls, add pointer states and live allocation/drag verification |
| `panel-stats` | Both live stat sheets in one fixture | Match reference values, numeric formatting and precise field geometry |
| `panel-npc-chat` | Taller native tiled frame, script colors/bold, native NPC and buttons | Match exact fixture wrapping/scrollbar, verify every message/input type live |
| `panel-npc-storage` | New compact frame/grid, native banker, all stored item categories | Match native control artwork, bottom field and portrait; live transfers |
| `panel-keyboard-settings` | Rebuilt 633x343 native frame, two-row tray, native OK/Cancel/Default/Clear All buttons, correctly anchored skill/item/action icons, duplicate bindings, pixel legends/counts and shared hitboxes; reference-matched mage/item fixture | Full live-server interaction verification and pointer dragging |
| `panel-system-options` | Live native settings renderer and deterministic fixture retained | Rebuild shared controls, separate supplementary game options, precise reference values |
| `panel-revive-character` | Native Notice/0 and OK artwork, shared button hitbox, pending state | Reconcile 286x146 native art with 285x139 reference and verify live respawn |
| `panel-quests` | Populated fixture, sixteen shared list rows, blue detail header and native NPC portrait | Reference frame dimensions, exact completed-quest fixture, detail scrolling and controls |
| `panel-monster-book` | Populated native card pages and separate live detail/animation fixtures | Rebuild the reference's combined card/detail/drop/tooltip view and interactions |
| `page-cash-shop` | Native 800x600 frame at (112,84), live avatar/inventory, product cards, balances, locker and modals | Exact tabs, gift/reserve support, pointer states, product metadata and complete reference fixture |

The installed El Nath minimap canvas is 377x113. Its natural aspect ratio differs
from the map region in the supplied references. The full compositions retain
that source data; reference crops do not squeeze the entire panel to conceal it.
The native equipment, quest, monster-book and revive frame dimensions
also differ from the supplied crops and need explicit layout work.

## Behavior retained and intentional UI changes

Game-state ownership, packet parsing/requests, server-authoritative updates,
1024x768 logical rendering, resize scaling, letterboxing and NX resolution stay
in their existing layers. The Cash Shop uses the same purchase, coupon, locker,
search and category actions with ten cards per page. It no longer substitutes a
hardcoded silhouette or name for the live character. Product/coupon/purchase
hit regions come from the active composition; modals suppress background hits.

Storage's `All` view now actually includes all stored categories. Rendering,
selection and scrolling use the same unfiltered list; withdrawal requests still
resolve the selected item's server slot. Inventory hover descriptions remain in
the separate tooltip instead of overwriting the meso field. Large numeric values
are clipped/truncated or use a fitting numeric face within their fields.

NPC shop transactions treat throwing stars and bullets as sets: inventory shows
their charge counts, while selling uses one set per transaction and buying
quantities count sets. The Recharge button is enabled for a selected projectile
only when the open shop marks that item as rechargeable and the stack is below
the server-provided charge limit. Automatic throwing-star recharge also skips
full stacks. Zero-meso, zero-pitch projectile refill entries stay available for
recharge but are omitted from the buy list.

The dialog alignment audit centers inventory, skill and shop tab labels using
NX bitmap dimensions and origins. Pet equipment opens in a right-side drawer
attached to the equipment inventory, with separate Equip and Remove actions.
NPC names fit their nameplate before centering. AP and SP
allocation hitboxes now match the visible NX buttons; stat value fields, skill
row gaps and the skill scrollbar no longer allocate points. Keyboard item counts
use the inventory's NX digit renderer, including complete counts above four
digits (`panel-keyboard-settings-large-values`). Allocation and equipment
previews include the corresponding shared hitboxes.

No live v83 server interaction was exercised during this work. Offline fixtures
are not evidence that every drag, focus, purchase, stat allocation, transfer,
scroll or respawn path is verified. Packet tests remain necessary alongside
subsequent live UI checks.

## Remaining screens

The canonical catalogue preserves preview access to login/selection/creation,
pet equipment, NPC stores, skill macros, social panels, world maps, clock,
wedding, RPS, events, dragon/carnival, megaphones, quit/server/TOS dialogs,
Cash Shop locker/coupon/confirmation and monster-book animation details. These
use the retained implementations and shared font/clipping changes. They still
need fresh presentation/input review against the read-only C++ client and the
style of the new reference cases. Their visual basis is legacy NX/C++ behavior,
not a direct reference-image match.

The untracked artwork under `crates/openstory-client/assets/ui` was reviewed via
the baseline previews and retained on disk. The replacement inventory, skills,
HUD and revive renderers do not include those screenshot-derived overlays.
No new replacement raster artwork has been generated. Missing exact-version
art/control differences are tracked above rather than presented as completed.

## Reproduce

```sh
cargo run -p openstory-client -- --ui-preview all --ui-hitboxes --out target/ui-previews/rebuild
cargo run -p openstory-client -- --ui-preview panel-item-inventory-large-values --out target/ui-previews/rebuild
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
```

Additional inventory fixtures cover empty content, scrolling and dragging.
Previews and comparison artifacts belong under `target/ui-previews`.

## Keyboard settings follow-up

The keyboard case now uses the supplied reference dimensions and keeps the
remaining native frame pixels at 1:1 scale. A single authored 34px action row is
omitted and the original bottom border is retained; the keyboard is not squeezed
to fit. Button, key, wide-key and tray bounds are shared with live input. Icons
are placed by their visible bounds, fixing the previous 32px origin displacement.
Reference fixtures use NX spell/potion artwork and counts 251/245. No screenshot
pixels or archives are included in the live renderer.

Binding and quick-slot edits are drafts until OK. Cancel/close/Escape discard
them, Default supplies the classic mapping, and Clear All empties the draft.
Only changed wire keys are sent through the existing v83 request writer.
Unbound actions beyond the two visible tray rows remain available by scrolling.
F12 enters quick-slot editing; Escape returns to the keyboard, and OK saves it.

Additional previews: `panel-keyboard-settings-empty`,
`panel-keyboard-settings-capture`, `panel-keyboard-settings-scroll`, and
`panel-keyboard-settings-quickslots`. Their images and hitbox overlays are in
`target/ui-previews/keyboard`. The keyboard artifact catalogue records the NX
artwork used to match the reference.
