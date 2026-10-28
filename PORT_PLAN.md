# OpenStoryRS Port Plan

## Objective

Port the complete in-game OpenStory MapleStory client from the adjacent `OpenStory`
C++ repository into this Rust repository. Preserve the current v83 server behavior and
use the same NX asset files. Improve CPU/GPU efficiency, support a resizable window
with modern scaling, and keep native macOS, native Windows, and future browser/WASM
builds in scope.

The Rust client product is named **OpenStoryRS**. The separate C# WPF launcher is not
part of the game runtime port in this plan.

## Repository facts and constraints

- `OpenStoryRS` started as an empty Git shell.
- `OpenStory` contains 626 C++ source/header files across graphics, input/UI,
  gameplay/physics, character data, protocol handlers, and platform code.
- The active asset set is 17 `.nx` files in `OpenStory/wz`, about 3.8 GB total.
  Rust reads them in place; do not copy them into this repository or bundle. The
  client reads the version 83 UI assets from `UI.nx` and resolves every UI path
  against that archive.
- The current C++ checkout contains uncommitted user changes. Treat it as a read-only
  reference and do not reset, reformat, or edit it.
- The client should use the larger legacy 1024x768 game canvas. Preserve those logical
  coordinates while uniformly scaling into the resizable drawable area, with
  letterboxing when the window has a different aspect ratio.
- Browser/WASM needs explicit platform adapters. Browser code cannot use ordinary
  filesystem or TCP sockets like the native client. A future browser deployment will
  need a WebSocket-compatible server gateway and an asset delivery/selection plan.

## Architecture

Use a Rust workspace with separated crates/modules for:

1. `openstory-assets`: NX indexing, typed nodes, bitmap/audio decoding, lazy loading,
   and caches. Implement the needed reader in Rust rather than depending on C++ at
   runtime.
2. `openstory-protocol`: bounded packet readers/writers, Maple framing and crypto,
   opcode handling, and transport-independent session events.
3. `openstory-core`: character/map/game state, fixed-step updates, physics, combat,
   and domain events, with no window or graphics API dependencies.
4. `openstory-render`: 2D GPU renderer, sprite/texture batching, font cache, effects,
   and logical-to-physical viewport transforms.
5. `openstory-client`: app lifecycle, platform adapters, input/window/audio, settings,
   and composition of the crates above.

Use `wgpu` for the shared native/browser renderer. Keep window/canvas, storage, audio,
and networking APIs behind platform traits so game and UI state do not depend on an
operating system.

## Execution stages

### 0. Baseline and behavior inventory

- Record legacy feature boundaries and protocol behavior before translating modules.
- Preserve the existing C++ working tree and its uncommitted changes.
- Track defects encountered during the port; fix them in the Rust implementation and
  note compatibility-sensitive behavior separately.

### 1. Rust workspace and application shell

- Add the workspace layout, target-specific dependency boundaries, configuration and
  data-path resolution, logging/error types, and a native application lifecycle.
- Establish the resizable window and logical viewport contract first.
- Keep file and socket use out of shared game logic.

### 2. NX asset layer

- Map the NoLifeNx APIs actually consumed by the client to a Rust-owned node model.
- Implement bounded parsing and decompression, bitmap/audio extraction, and typed
  accessors that report missing or malformed data instead of panicking.
- Add lazy loading and cache eviction suitable for multi-gigabyte asset sets; support
  native filesystem reads and an abstract browser asset source.

### 3. Renderer, input, and audio platform seams

- Implement batched sprite rendering, texture atlases, text/glyph caching, alpha and
  additive effects, clipping, and resize handling.
- Convert pointer/key coordinates through the same logical viewport transform used to
  render, including high-DPI framebuffer sizes.
- Define a common audio API and platform implementations without retaining BASS or
  other C++ runtime dependencies.

### 4. Protocol and connection lifecycle

- Port packet primitives, exact v83 framing/opcodes, handshake crypto, partial-read
  handling, reconnect/redirect flows, and handler dispatch.
- Use a native TCP transport first; keep a transport trait for a future browser
  WebSocket gateway.
- Reject malformed lengths and truncated fields safely and isolate handler failures.

### 5. Core game and data systems

- Port static item/job/skill data, character/inventory models, map loading, foothold
  physics, movement, mobs/NPCs/drops/reactors, combat, buffs, and effects.
- Preserve the fixed-step simulation and decouple rendering interpolation from state
  updates and network messages.

### 6. Full UI and feature parity

- Port login, world/channel selection, character creation/selection, in-game HUD,
  inventory/equipment, skill/quest windows, dialogs, shops/storage, chat, party/buddy/
  guild/social flows, settings, and all remaining legacy screens.
- Keep existing controls and persisted settings compatible where practical.

#### Explicit known-bad fixes

- [x] **Pet equipment window:** add a dedicated window for pet status and pet gear. It must
  show owned pets and pet equipment from the character's Equip inventory, send the v83
  equip move for pet gear, and refresh from server inventory updates.
- [x] **Cash shop rewrite:** replace the legacy fixed layout with a new 1024x768-logical,
  resizable catalog and locker experience. Use `Etc.nx/Commodity.img` as the source of
  purchaseable products and their server SN, price, count, and on-sale status. Include
  search, category filters, item detail/preview, balances, purchase confirmation, and
  the server-authoritative locker/withdrawal flow. Do not send purchases for products
  without a valid commodity SN.
- [x] **NPC dialogue keyboard action:** Space uses the legacy NPC Chat binding to talk to
  the nearest NPC in range. NPC lines reveal at the legacy typing cadence; Space or a click
  reveals the current line immediately, and a following Space advances or accepts the
  highlighted choice. Up/Down moves through the full selection list and the mouse wheel
  scrolls it. Space submits number prompts without triggering a gameplay jump; on text prompts
  it reveals the prompt first, then remains available for answer spaces and Enter submits.
  The dialogue header shows the Space/Enter action
  for each prompt, including the highlighted choice and Yes/Accept prompts. Long dialogue pages
  scroll within the panel by mouse wheel or Page Up/Down. Either Option key jumps.
- [x] **Facial expressions:** F1–F7 send v83 emotes; local and remote characters animate
  the matching NX face frames, with expression textures scoped to the active scene.

### 7. Platform delivery and performance pass

- Make macOS the first native delivery target, then enable Windows without duplicating
  game logic.
- Keep the browser target buildable at the crate/interface level; complete browser
  networking and asset delivery when a compatible server gateway is available.
- Profile asset loading, update allocations, draw-call batching, GPU uploads, and map
  rendering. Fix discovered correctness and performance defects as they surface.
- Keep the native application, bundle, window title, CLI, and package named OpenStoryRS.

## Initial known defect to track

The dormant non-encrypted branch in `OpenStory/src/Net/Cryptography.cpp` refers to
`slength` and `HEADERLEN`, while the method declares `length` and the shared constant is
`HEADER_LENGTH`. `USE_CRYPTO` currently masks this branch. The Rust packet layer will
implement framing from the declared sizes and reject out-of-range values explicitly.

The vendored `NoLifeNx` bitmap decoder also writes every decoded image through one
process-global mutable 8 MiB vector. Concurrent decodes can overwrite one another and
large images force shared-buffer growth. The Rust decoder returns bounded per-call
buffers, so independent archives can be decoded safely without a global scratch area.

The v83 UI archive contains `Login.img/NewChar`, `NewCharKnight`, and
`NewCharAran`, but not the `Login.img/CustomizeChar` node referenced by the newer C++
creation screens. The Rust creator resolves the v83 class-specific nodes directly.
The installed version 83 UI archive is named `UI.nx`; UI paths resolve directly
against it.

The legacy C++ `CHANGE_STATS` handler reads the skin field as a 16-bit value, while the
adjacent Cosmic v83 server writes one byte. The Rust decoder follows the server wire
format so packets that update skin and following fields stay aligned.

The legacy deletion-result UI labels response state `10` as a birthday failure, but the
v83 server uses `10` for too many requests and `18` for invalid birthday verification. The
Rust client maps the server states directly and retains distinct messages for guild,
wedding, family, and world-transfer deletion restrictions. The standard `DELETE_CHAR`
request carries only the PIC and character ID, and the supplied Cosmic handler does not
request a birthday. Its state `18` is still handled if another v83 server reports an
invalid birthday; a birthday-entry step requires a server-specific wire contract.

The legacy quest log labels `prop=-1` rewards as selectable but always sends the quest
completion action without a selection. Cosmic's v83 handler passes that missing value into
the selective reward list, so a quest with such rewards can fail during completion. The Rust
quest log now exposes an item chooser, filters choices with the server's gender/job rules,
and sends the matching selection index.

## Current status

- OpenStoryRS naming is reflected in the CLI, macOS app bundle, executable, and window.
  The login screen uses the legacy Save ID checkbox to remember the account only; the
  password is never saved, and later audio-setting writes preserve the login preference.
  Space starts dialogue with the nearest nearby NPC, reveals typed dialogue, then advances
  pages or accepts the highlighted choice; Up/Down moves through selection prompts without consuming text-entry
  spaces. Holding X sits on nearby map-authored seats; movement or either Option key stands.
  `P` opens the new
  Pet Equipment panel, which reads pet records and PetEquip items from the Equip inventory
  and sends the legacy `MOVE_ITEM` pet-equip request. Activating a Cash pet sends the
  legacy summon/dismiss packet. Pet sprites use their NX stance animations, names, and server
  spawn/movement updates. Local pets follow with the shared foothold physics, choose nearby
  drops using the legacy pet AI cadence, and send the v83 `PET_LOOT` request. Pet-chat packets
  show timed bubbles; pet-command packets play NX interaction sequences or legacy alert/jump
  fallbacks with the matching pet sound category. Either Option key jumps. `C` opens the
  character stats and AP window; each AP is sent as a server-authoritative v83 `SPEND_AP`
  request and the UI waits for the stat update before allowing another. Grave accent opens
  the rebuilt Cash Shop, which uses the
  `Commodity.img` sale catalog, search/categories, product details and icon preview,
  currency selection, purchase confirmation, server balances, locker snapshots, and
  server-confirmed withdrawals. Coupon redemption sends the legacy `COUPON_CODE` request
  and maps the v83 `0x5C` result codes for invalid, expired, used, and wrong-server coupons.
  Offline previews of the catalog, locker, coupon, and confirmation states now use the same
  renderer as the live client. Native macOS compilation covers these changes; the Cash Shop
  has not yet been exercised against a live server response.
  The quest detail pane now lets the player choose among eligible `prop=-1` item rewards
  with on-screen arrows or Left/Right, and completion sends the corresponding v83 choice
  index. Incoming `SERVER_MESSAGE` packets now route v83 notices, local/global and item/triple
  megaphones, scrolling banners, and NPC broadcasts through chat, the banner, and NPC dialogue
  UI. The v83 `SCROLL_RESULT` response plays the NX enchant success/failure effect on the
  affected character and displays the legacy result message locally. `SERVER_MESSAGE` popup
  notices and `FAKE_GM_NOTICE` now use a dismissible OK modal with the v83 login notice art and
  notice sound; Enter, Escape, and the OK button dismiss it. The v83 archive lacks the newer
  `Basic.img/Notice6`, so the modal is composed over `Login.img/Notice/backgrnd/0`.

- The minimap now reads the current map's canvas, center, magnification, portals, and marker
  art from the existing NX archives. Its collapsed and expanded panels use `UIWindow2.img`
  assets; a bounded 10 Hz refresh tracks the local player, NPCs, mobs, and other characters.
  Its map button opens the World Map, and `Backslash` key bindings can assign a toggle key.
  Maps that hide or omit the minimap do not draw one. Messenger packets now drive a draggable
  three-slot room panel with invite/decline prompts and bounded chat; Escape and the Close
  button leave the server room. `OpenUI` mode 5 and the client key-bindings panel can open it.
  The channel selector can be assigned from that panel too; it requests a v83 transfer and
  reconnects to the redirected channel address with the active character.
  Server key-map actions now also open the existing World Map, Monster Book, character info,
  whisper composer, and public/party/buddy/guild/alliance chat flows. The
  offline chat preview now uses the live Rust chat-panel renderer; it does not probe the newer
  `StatusBar2.img` tree, which is absent from the selected v83 archive.
  The Family action opens a v83-backed overview and family-member list with reputation, motto,
  junior invitations, a confirmed leave action, abilities, Y/N join and summon responses,
  and clickable pedigree navigation. The motto editor and junior invite flow use native
  keyboard/IME text input with server payload limits.
  key-binding editor now exposes 40 assignable built-in menu, gameplay, and face actions
  with mouse selection, scrolling, keyboard capture, and Backspace unbinding; unsupported
  saved actions remain visible and can be cleared. Menu action 26 opens Family as in v83,
  action 27 remains the unsupported Boss Party action, and action 28 opens Options. Learned
  active skills and usable inventory items appear by localized name; stale skill/item
  bindings remain available to clear. The eight quick-slot keys can be rebound from the
  same panel with unique-key swapping and are saved in the legacy `QuickSlotKeys` setting;
  a locally saved layout takes precedence over the server's login-time layout. Skill macros
  load and save through the v83 packets; the Skills window editor supports the three macro
  names, shout setting, ordered learned-skill slots, and macro hotkeys. Triggered skills queue
  through the ordinary skill-use path. The Quick Slots action now toggles the legacy v83
  eight-slot panel above the HUD. The panel draws current key labels and icons for bound
  skills, items, cash items, macros, and built-in actions; clicking a slot runs its binding.
  Dragging a Use, Setup, or Cash item from Inventory onto a slot updates that key locally
  and sends the v83 `CHANGE_KEYMAP` update so the binding persists on the server.

- Passive HP/MP recovery sends the v83 `HEAL_OVER_TIME` packet every 10 seconds using
  the legacy level/INT formula and `IMPROVE_HP_RECOVERY` bonus. Server-reported HP loss,
  attacks, Dark Sight, local invincibility, and death reset or pause its timer. Chair
  recovery uses the legacy 3x multiplier. Portable Setup chairs send `USE_CHAIR` and
  `CANCEL_CHAIR`, animate their NX `effect`/`effect2` frames around the seated pose, and
  consume `SHOW_CHAIR`/`CANCEL_CHAIR` broadcasts for remote seated poses. Map-authored seats
  load their coordinates from each map's `seat` nodes; holding X within the legacy 10-pixel
  range snaps the local character to the seat and sends sit stance 20/21 to the server.
  Movement and either Option key stand up without sending a chair-cancel packet.

- The plan was recorded before implementation. The C++ repository has only been
  inspected; its working changes remain untouched.
- The release macOS app packages successfully, its plist validates, and its ad-hoc
  code signature verifies. A direct native bundle-binary launch reaches Winit and draws
  the client window. Against the supplied local v83 server, the client completed account
  authentication, requested the world list, and parsed Scania with three channels. The
  server sends each world in its own packet followed by a separate `0xFF` terminator;
  the Rust client now accumulates those batches. Login, handshake direction masks, and
  world-list parsing are runtime-verified. A desktop-capable run on 2026-10-26 completed
  the login flow and received the three-channel Scania world list. Character-list selection,
  channel redirect, and actual map entry remain to be verified. Managed sandbox launches
  stop before Winit startup; desktop execution permissions are needed for runtime checks.
  The packaged app defaults to `127.0.0.1:8484` (overridable with `OPENSTORY_SERVER`);
  Login and Guest activate while connecting, and their requests wait until the handshake
  and login-start packet complete.
- Recent parity fixes preserve ASCII login-rank movement markers, signed guild
  capacity/rank and alliance count bytes, and the server's boolean semantics. Guild
  disband clears local guild state; foothold physics extrapolates ground lines through
  linked segment endpoints, and attacks combine signed Booster speed with NX weapon speed.
  Resizing refreshes the pointer transform immediately, and the renderer retries frame
  acquisition after recovering from a lost or outdated surface. Missing audio settings
  now default to the legacy 50 percent.
- Renderer backend features are now selected per target: Metal for macOS, DX12/Vulkan
  for Windows, WebGPU/WebGL for WASM, and Vulkan/GLES on other native systems. This
  keeps platform backends from being enabled together; WASM still needs its browser
  event loop, asset provider, audio adapter, and WebSocket transport.
- The native sprite renderer now uploads one 48-byte instance per visible sprite and
  generates its quad in the vertex shader, replacing six 32-byte vertices. This cuts
  per-frame sprite geometry upload by 75% while retaining authored sprite order and
  adjacent texture/blend batching; frame-time profiling remains open.
- Public and social chat now follow the v83 `GENERAL_CHAT`, `MULTICHAT`, `WHISPER`, party,
  and buddy-list packet formats. `/party` supports create, join, invite/accept/deny, leave,
  expel, and leader change commands; `O` opens a server-driven six-member panel with
  channel and live HP state. Bounded buddy snapshots, online updates, capacity, and invites
  feed a scrollable panel toggled with `B`; `/buddy add`, `/buddy remove`, `/buddy accept`,
  and `/buddy dismiss` cover basic list management. Enter opens the composer, Tab cycles
  public/buddy/guild/alliance/party targets, and `/w`, `/find`, and `/pt` send whisper,
  location, and party-message requests. Incoming group messages, whispers, delivery failures,
  and location replies appear in the colored HUD history. `H` opens a dedicated whisper
  composer; `/w <character>` opens or switches per-character conversations, and
  `/w <character> <message>` sends directly. Incoming whispers open their sender's conversation, which retains
  its own bounded message history and draft. Visible public-chat speakers
  display four-second balloons composed from the `ChatBalloon.img/0` nine-slice and arrow
  assets. Guild operation packets now decode bounded roster snapshots and live member,
  rank, capacity, emblem, notice, and GP updates; `G` opens a scrollable guild roster and
  pending invitations can be accepted with Y or declined with N. The guild BBS has bounded
  list/detail packet handling, pagination, post selection, reply display, 25/600/25 UTF-16
  field limits, post and notice creation, reply submission, and confirmed post deletion.
  `/guild create` now starts the server's nearby-cofounder confirmation flow; creation requests
  and Y/N responses use the v83 guild operation packets. `/guild invite`, `/guild leave`,
  `/guild expel`, `/guild rank`, `/guild ranks`, `/guild emblem`, and `/guild notice` send the
  server's v83 management operations; returned guild events keep the panel state authoritative.
  The guild panel now includes a create form, a five-title editor, and an emblem editor that
  enumerates valid background/mark IDs and color variants from `UI.nx/GuildMark.img` and
  previews the selected bitmaps. Guild and Alliance panels use the v83 `UIWindow.img/UserList/Guild`
  and `GuildUnion` backgrounds from the selected UI archive, so the current `UI.nx` works
  without the newer `GuildUI.img` tree. Offline previews composite these same live panel renderers.
  The chat commands remain available as shortcuts.
  Alliance operations now decode bounded v83 alliance snapshots and guild roster broadcasts.
  `U` opens the alliance panel; `/alliance` supports list, invite, expel, leave, notice,
  rank-title, accept, and deny actions, and Y/N answers pending invitations. Alliance rank-title, member-online,
  member-level, and job updates are decoded; the panel keeps the local alliance ID visible
  while waiting for a roster broadcast. Buddy entries already preserve their server group;
  `/buddy group <character> <group name>` now edits that group with the v83 add/update operation,
  `/buddy filter <group name>|all` narrows the list, and group membership is visible on each row.
  Re-adding an existing buddy is rejected locally so it cannot silently reset a custom group.
  Player-to-player trade now supports `/trade <visible character>`, invitation acceptance or
  decline, bounded room/item parsing, stack quantity offers, mesos, confirmation, cancellation,
  and trade chat. The custom resizable room uses the local inventory and server-reported offers;
  its live transaction flow has not yet been exercised against a v83 server.
  Clicking a visible character now sends the legacy character-info request; the bounded
  response parser and NX-named panel display level/job/fame, guild/alliance, pet, mount,
  wishlist, Monster Book, and medal data; its Party, Trade, and Guild actions send the
  corresponding invitations. The v83 `SET_TAMING_MOB_INFO` update refreshes the local
  character's mount level, EXP, and tiredness in the open panel and seeds later character-info
  replies when that response omits mount data. Raise/Lower Fame sends the v83 fame request,
  enforces the level-15 check, displays server rejection reasons, and updates the open panel
  on success.
  The character-info and fame flows have not yet been exercised against the local server.
- The minimap now follows the legacy min/normal/max cycle: keyboard toggles visit all three
  sizes, the size buttons step one level at a time and show the disabled state at each limit,
  and NX town maps start expanded. Normal and expanded layouts use their separate
  `UIWindow2.img/MiniMap` frame art when present; v83 `UIWindow.img/MiniMap` frame art is
  resolved for the current archive, with small generated zoom controls where v83 has none.
  Offline previews cover all three states with the active v83 UI archive.
- The server-requested World Map now uses `Map.nx/WorldMap.img`, `MapHelper.img/worldMap`,
  and `UIWindow2.img/WorldMap` for the regional panel, current-map marker, and link navigation.
  Its search field accepts up to eight characters and filters regional markers by localized
  map name, description, or map ID. A lazy catalog searches all regional `MapList` entries;
  results rank exact and prefix name matches before other name, ID, and description matches,
  and a scrollable result list opens the corresponding region and highlights the selected
  map marker. Escape closes the search pane before returning to a parent map. Search is
  case-insensitive and the text overlay alpha-composites onto the authored panel instead of
  erasing its pixels. Hovering a map marker shows its localized name, description, and ID,
  and draws its NX path overlay when present. Clicking a map marker or search result selects
  the destination and keeps its available NX route overlay and destination marker visible;
  the selected destination tooltip identifies the route preview, and clicking its marker again
  clears that preview. World-map links show their NX overlay on
  hover and open the linked region when clicked. The legacy `UIWorldMap.cpp` creates
  `BtAutoFly_1` and `BtNaviRegister` controls, but `button_pressed` has no actions for either
  ID, and the matching Cosmic source has no world-map travel/register handler or packet.
  Rust therefore implements destination selection and the NX route preview locally; actual
  travel still uses the normal in-game transport interactions. Monster
  Book packet data drives the NX-backed collection pages, category filters, card details,
  name search, and v83 set-effect page with collected-card score and progress. Monster
  details now read reward item IDs from `String.nx/MonsterBook.img`, resolve item icons from
  the existing NX archives, and scroll the grid with the mouse wheel. Detail sprites now
  cycle through stand, move, and die states on click and follow NX frame delays, opacity, and
  scale; die animations return to stand when they finish. The legacy detail panel shows HP
  and MP as colored text values; it does not use animated gauge bars.
- The Rust workspace and application shell are in place. It opens a resizable wgpu
  window with a 1024x768 logical canvas and letterboxes its 4:3 content. The login
  screen uses layered v83 art from Map.nx and UI.nx, with editable account and
  password fields, password masking, viewport-mapped mouse input, and login/guest/quit
  controls. Successful authentication now opens a world/channel picker built from
  legacy assets; selecting a world and channel requests that channel's character list.
  A basic character list view displays server stats, supports PIC setup/verification,
  and provides character deletion with confirmation. Character selection follows the
  server-IP redirect to a channel connection, sending the separate channel-login packet
  after its handshake. Character slots select and highlight on click, while the NX Select
  sign or Enter begins the existing PIC/network selection flow. NX New and Delete signs
  use their visible bounds; Delete retains its PIC and confirmation safeguards. The roster
  pages in groups of three using the legacy arrow signs, with wrapping arrows, Left/Right
  slot navigation, PageUp/PageDown paging, and Tab/Enter keyboard focus for Select/New/Delete.
  A bounded compact `SET_FIELD` parser now handles in-channel map
  changes; initial entry uses the selected character's map and portal from the login
  character list. The map loader reads foothold groups to place the initial view at
  ground level. The first map scene loads NX backgrounds, object frames, and tiles in
  legacy depth order, with viewport culling and GPU texture reuse. Background parallax,
  independent horizontal/vertical motion, and tiled wrapping follow each entry's type;
  server obstacle packets now control map environment visibility and animation resets.
  The v83 `SET_TRACTION` double updates local foothold friction on slippery fields, while
  `SHOW_COMBO` drives the Aran combo counter with the legacy three-second reset display. A
  map entry composes the selected character's body, head, face, hair, and equipped items. It plays
  authored one-handed or two-handed stand/walk frames and jump frames at NX delays.
  Character selection renders three equipped-avatar previews per page at the legacy slot
  positions; each preview hit area covers the sprite and nameplate without overlapping its
  neighbors. Creating a character returns focus to its new slot and the Select action. The
  avatar renderer now composes every legacy body z layer, including arm-below-head,
  hand-below-weapon, and hand-over-hair parts, so those layers interleave with clothing
  and weapons. Arrow keys or A/D drive local
  foothold movement at the shared 8 ms tick, applying NX-equipped and server-buffed SPEED/JUMP
  totals with the legacy 140/123 caps; either Option key jumps, Space talks to the
  nearest nearby NPC and advances that dialogue, the camera
  follows, and the base avatar turns. Nearby map art is retained within a 512-pixel
  camera margin. Avatars resolve equipped hats, face accessories, armor, shoes, gloves,
  capes, shields, and weapons from their NX categories, align item attachment vectors to
  body/head/hand anchors, honor masked slots, and order sprites by legacy z layers. The
  initial field entry now parses meso, slot maxima, equipped items, equipment, and stack
  items; inventory add/count/remove/swap packets update that state, and `I` opens a
  six-tab inventory with a scrollable four-column slot grid, cached NX item icons, stack
  counts, meso, and slot capacity. Ordinary Use items send the v83 `USE_ITEM` request;
  manual and automatic `USE_ITEM` requests play the matching `Sound.nx/Item.img` sound,
  following the C++ exact-item, 10,000-ID group, and Red Potion fallback order.
  clicking an Equip item sends the legacy `MOVE_ITEM` request for its equipment slot, and
  clicking a worn item unequips it into the first free Equip slot. Dragging within a bag
  sends a server-authoritative `MOVE_ITEM`; inventory updates rebuild the local avatar from
  NX art. Equip actions check gender, job class, level, fame, and total stat requirements
  before sending the move. Hovering an occupied slot shows its String.nx name, description,
  required level, and NX equipment bonuses. Gather and Sort send the v83 inventory opcodes;
  the server merges compatible stacks, closes gaps, or orders items, and its updates remain
  authoritative. The v83 move request does not support partial stack moves within a bag.
  The legacy client rejects cross-category drags, so slot moves stay within the selected bag.
  Available one-shot `Effect.nx/ItemEff.img` animations play locally when a usable item has
  one. Key-bound Use, Setup, and Cash items use the same activation path as inventory
  clicks, so pet food, chairs, and special cash items keep their item-specific behavior.
  Other Cash items fall through to the legacy generic `USE_ITEM` request. Pet food uses
  the distinct v83 `PET_FOOD` request so the server feeds the hungriest summoned pet, and
  activating a Cash inventory pet sends the legacy `SPAWN_PET` toggle request.
  `AUTO_HP_POT` and `AUTO_MP_POT` now select the server-configured Use items; with a
  summoned pet present, the client sends the matching `USE_ITEM` below 50% HP or MP and
  applies the legacy four-second cooldown.
  Standalone pet spawn/removal and movement packets plus pet records in
  remote-character spawns now update local/remote map actors; pet stance art and names load
  from the existing NX files. `PET_NAMECHANGE` now refreshes the first pet's server-authored
  name label and reports the change. Local pets follow their owner using shared foothold physics;
  the v83 pet AI selects nearby drops and sends the cash-serial `PET_LOOT` request with the
  legacy cooldowns. Pet chat bubbles and server pet-command interactions now use the pet slot,
  NX interaction results, fallback stances, and the matching pet sound category. Supported
  server `CHALKBOARD` updates now retain or clear each character's overhead message, with a
  board rendered for both the local character and visible remote players. Supported
  regular, super, item, and triple cash megaphones, Maple TV, avatar megaphones, and pet name
  tags compose through chat and send their v83 `USE_CASH_ITEM` payloads. Item megaphones
  accept an optional inventory attachment selected after entering the message; clicking the
  selected item again clears it, and `I` reopens the inventory during selection. The packet
  carries the inventory tab and slot in the format expected by the v83 server. `Tab` toggles
  the whisper flag for megaphone variants that serialize it. Maple TV types
  with the legacy target control prompt for an optional recipient name capped at 12 characters;
  Star TV types omit that
  prompt. The composer collects up to three triple-megaphone lines, five Maple TV lines, and four
  avatar-megaphone lines; pressing Enter on a blank row sends the lines already entered.
  Incoming `SEND_TV`, `REMOVE_TV`, and `ENABLE_TV` packets are now bounded-parsed. TV NPCs
  use their `Npc.nx` message/ad anchors to draw shared NX screen art, themed message art,
  sender/partner previews, and broadcast text; removal and the server's remaining-time field
  clear the broadcast. The three idle ad reels and both themed overlays advance through their
  NX-authored frames using each frame's delay and origin metadata.
  Other targeted cash
  actions remain open work; 514 store permits prompt for a shop name and send the legacy
  personal-shop creation packet. A 523 Owl of Minerva opens a panel with server top searches,
  item-name or item-ID search, paged listing results, and a warp action. Hired-merchant permits
  now request the server's deployment check and, after approval, prompt for a shop name and send
  the type-5 merchant-room packet. Hired merchant spawn, removal, and visitor-status packets
  now create animated NX map actors with their sign labels; clicking the merchant sends the
  legacy mini-room visit action. The room-entry response now loads bounded visitor, chat,
  meso, and stock state. Visitors can buy and chat; owners can stock an inventory stack at a
  chosen price, withdraw listings, organize stock, collect mesos, start the store, end
  maintenance, and close the store. One inventory stack is currently submitted as one bundle.
  Personal-shop room responses now use the same resizable catalog for visitor purchases and chat,
  owner stocking/withdrawal, sold-out notifications, and start/exit actions. Map discovery and
  direct visits use mini-room metadata from character spawn and update packets; clicking a
  visible owner enters their personal shop. Other targeted cash actions and
  consumable-specific visuals remain open.
  Equipment scrolls can
  be selected from the Use bag and
  applied to a chosen equipped slot with the v83 `SCROLL_EQUIP` request. Dragging an item
  outside the inventory sends the v83 drop request; stack drops accept a bounded quantity,
  cash items are blocked, and NX `tradeBlock` items require confirmation; drop quantities
  are sent through the v83 packet. Hovering an inventory item now opens a v83-styled tooltip
  using the `UIToolTip.img` art from `UI.nx`; equipment details use the server's current
  stats and show differences from the item's base Item.nx values. Item.nx lookup paths now
  match the actual nested node
  names for inventory icons, drops, and projectiles; the nested item node does not include
  an `.img` suffix. Avatar frames keep
  scene draw indices separate from GPU texture indices, so dynamic equipment refreshes
  resolve the intended texture while retaining map draw suppression. Full equipment edge
  cases and special projectile rules,
  full legacy character-creation polish and remaining modal dialogs
  remain open work. Title login buttons now show NX disabled, mouse-over, pressed, and
  keyboard-focus states where the archive provides them, with a gold-outline fallback for
  buttons lacking dedicated focus art; Tab and arrows navigate enabled actions, while Enter
  and Space activate the focused action. Login and Guest are available during connection;
  requests clicked before the handshake wait behind the login-start packet. Login also checks
  the credential lengths. The Login button hit target takes precedence over the overlapping
  account-field area, so its left/lower edge still submits. Login's Save ID
  control now persists the account name and
  checkbox state without storing the password; audio settings updates preserve these values.
  The field HUD now draws its 1024x71 bottom bar, gauge tracks/fills, notice art, and toolbar
  sprites from the stock v83 `UI.nx/StatusBar.img`. Its small controls open the implemented
  status, inventory, skill, key, and quick-slot panels; the legacy menu exposes the quest,
  social, Monster Book, and World Map screens, and the large controls open Cash Shop, Options,
  and the `/trade` composer. The Event List requests v83 `EVENT_INFO`, refreshes its local
  countdown, and displays server descriptions, multipliers, rewards, and remaining time over
  the v83 `UIWindow.img/EventWindow` frame from `UI.nx`; it does not require the newer
  `UIWindow2.img/EventList` or `UIWindow4.img/TimeEvent` trees. The quick-slot panel hitboxes
  follow its bottom-bar anchor.
  An offline `native-status-bar` preview composites those same NX sprites and the transparent
  live-stat overlay over a gameplay scene to catch regressions in the legacy bottom toolbar.
  Collapsed chat keeps the latest message inside the bar and Enter expands the history panel,
  matching the legacy collapsed/open states instead of leaving a Rust-styled panel onscreen.
  Redraws are capped near 60 Hz. The
  renderer supports reusable GPU textures, source-UV atlas draws, per-sprite tint and
  opacity, culls off-canvas sprites, and batches adjacent quads sharing a texture page into
  one draw call. Scene art is packed into 2048-pixel atlas pages with duplicated gutters
  for clean linear sampling; oversized images retain standalone textures. Runtime NX
  map-art pixels use a 128 MiB, 16,384-entry LRU cache while GPU pages are shared for
  the current scene. Map object and `back/ani` frames now
  advance from their NX delays while inactive frame draws stay culled. The six login light
  beams now animate independently through their NX-authored frames and retain additive
  blending. Repeated software-rendered panel text reuses glyph coverage
  from a bounded 16 MiB FIFO cache. GPU glyph-atlas rendering, general scene atlasing,
  complete environment and animated-effect parity remain open work. `FIELD_EFFECT` now
  renders one-shot map-screen animations from `Map.nx/Effect.img`, changes BGM, and plays
  field sounds with the legacy `Party1` fallback. `BLOW_WEATHER` resolves the cash item
  path from `Item.nx`, animates weather particles or fog from `Map.nx`, and surfaces an
  optional server weather message.
- `openstory-core` contains the 1024x768 letterbox viewport conversion, an 8 ms
  fixed-step clock with capped catch-up, and an initial foothold geometry/index layer.
  Footholds are indexed in 64-pixel buckets rather than one entry per covered pixel;
  duplicate/reserved IDs and empty or degenerate bounds are handled safely. Normal,
  flying, and swimming physics now have a fixed-tick body model with slope traversal,
  gravity, traction, linked foothold walls/edges, and interpolated positions. The old
  left-walk slope comparison bug is corrected. Map footholds drive initial spawn
  height and local movement/camera tracking. The client now emits v83 absolute player
  movement fragments when its rounded position, previous position, foothold, or stance
  changes. Map `ladderRope` geometry now supports directional Up/Down grabs, x snapping,
  fixed-step ladder/rope movement, native ladder and rope stances/animations, bounds drop-off,
  jump dismounts with the legacy climb cooldown, and synchronized v83 movement fragments.
  Initial inventory snapshots and deltas are supported. `INVENTORY_GROW` updates all six
  capacity classes, including equipped slots, and refreshes the inventory grid from the
  authoritative server value. `SET_EXTRA_PENDANT_SLOT` stores the v83 entitlement state and
  reports activation/deactivation to the player. Item-specific targeting, server correction, complete
  skill/passive-stat parity, special projectile variants, and other gameplay systems remain
  open work. The client parses v83
  character spawn, remove, movement,
  and look-update packets. It also parses `SPAWN_NPC`, `REMOVE_NPC`, and `SPAWN_NPC_C`.
  Remote characters are drawn from their NX skin, face, and hair animations, with name
  labels, stance-based facing, server-position easing, and snap behavior for teleports
  and large corrections. Their v83 guild names and six-byte emblem values are now read
  from `SPAWN_CHAR`; guild-name and guild-mark refresh packets update the matching remote
  player, and guild names render in the legacy medium-blue tag below the character name.
  Character spawns retain the server's persistent item-effect ID;
  `SHOW_ITEM_EFFECT` starts or clears it for local and remote characters. `ItemEff.img`
  flat, two-layer, and per-frame split animations loop on independent character cursors,
  and equipped rings with authored item effects refresh when equipment changes.
  `SKILL_EFFECT` and skill-type `SHOW_FOREIGN_EFFECT` packets load
  NX skill-use frames, apply skill-selected attack stances, follow remote character facing,
  and use the legacy teleport animation for supported teleport skills. Level-up and
  Cosmic's job-change (8), quest-clear (9), scroll-success (15), and Maker-result (16)
  `SHOW_FOREIGN_EFFECT` packets play their matching `BasicEff.img` animations over local
  and remote characters; legacy 6/7 job/quest aliases are also accepted. Recovery (10)
  packets show the legacy blue heal digits over local or remote characters without granting
  invincibility. `SHOW_ITEM_GAIN_INCHAT` item-gain variants display the
  localized name and signed quantity in blue chat, skill/buff echoes render NX use effects
  with a 720 ms duplicate-cast guard, and the MonsterBook card variant plays its local
  character effect; remaining feedback modes are still open. The renderer interleaves
  actors after their legacy map layer's static tile and object art, sorting by world Y,
  actor kind, and ID within each layer; foreground map art and global overlays remain in
  the final pass. Local attack casts now use the same NX level/weapon-aware effects and
  skill-selected action stances; non-attack skill casts also show their local use effects.
  Remote `SPAWN_CHAR` and Monster Riding buff packets now load `Character.nx/TamingMob`
  mount animations, use the v83 gait fallbacks and frame delays, draw the mount below the
  rider, and follow each frame's `map/navel` seat point. Mount cancellation removes the
  remote mount. Local Monster Riding buffs use the equipped TamedMob and the same animated
  mount/seat path; equip and buff cancellation refresh the local state. Local and remote deaths
  now use the v83 Tomb fall/land sequence and personalized ghost composite, preserving the
  animation across repeated dead-state packets and remote look updates. Equipment edge cases,
  less common skill hit variants, remaining non-skill foreign effects, and the full legacy
  character state machine remain open work. Map `info/swim` now selects
  the v83 swimming physics mode and `fly` avatar animation; remote `ALERT` and `SWIM` state
  bytes also select their authored stance frames instead of being treated as attacks. Timed
  attack impacts now use legacy action or
  afterimage delays; live comparison against gameplay is still needed.
- `openstory-assets` has a bounds-checked, lazy PKG4 NX reader with typed nodes, LZ4
  bitmap decoding, audio extraction, and validation of the 17 legacy data files. Its
  on-disk header and record layout match the vendored NoLifeNx definitions. Node records
  use a bounded LRU cache; strings use a 16 MiB/65,536-entry LRU cache, limiting
  long-session archive metadata memory.
  Native file reads now use a shared 64 KiB block LRU bounded to 4 MiB per archive,
  reducing repeated positional reads without loading complete NX files. The exported
  `CachedSource` can wrap other immutable random-access providers with caller-selected
  block and byte limits.
  Decoded bitmap pixels now use a per-archive 16 MiB/4096-entry least-recently-used
  cache and can be shared by renderer paths without copying. Cache hits refresh recency
  under the existing mutex. `RandomAccessSource` and `MemorySource`
  let a browser host provide already-fetched or range-cached archive bytes without a
  native file API; browser fetch, persistent storage, and asynchronous range prefetch
  remain open work.
- `openstory-protocol` has bounded packet read/write primitives and a stream decoder
  for partial and coalesced plain frames. The v83 Maple AES/OFB and byte transforms,
  IV updates, bounded encrypted headers, and partial/coalesced encrypted-frame decoder
  are now implemented. A native TCP worker reads the 16-byte handshake, sends the
  legacy login-start opcode, and forwards decoded packets without blocking rendering.
  Login-result and server/channel-list parsers, account-login and selection packet
  builders, optional environment-configured login, and ping response are in place.
  Direction-specific send/receive version masks now match the v83 server cipher. The
  world-list parser handles Cosmic's one-world-per-packet batches and separate
  terminator; last-connected-world metadata selects the server-preferred world, and
  recommended-world metadata is bounded and parsed (the v83 UI intentionally ignores
  it). The credential UI sends login requests and feeds the login-result/world-list
  parsers. Native execution has authenticated against the local server and displayed
  the parsed three-channel Scania entry. Client state consumes a public, platform-neutral
  `openstory_protocol::SessionTransport` contract with shared connected, packet, and
  disconnected events; the native TCP worker implements that contract behind its
  factory so a browser WebSocket adapter can reuse the same packet event and reconnect
  contract. The browser lifecycle, async connection setup, and compatible gateway remain
  open work.
  Bounded character-list parsing now covers character stats, ranking, equipment, and
  appearance, and the client reports parsed character summaries. The world/channel
  screen now drives the character-list request. Character selection displays three
  characters per page; clicking an avatar highlights it, and the NX Select/New/Delete
  controls operate the existing v83 selection or PIC flow. Keyboard navigation moves
  between slots, pages, and signboard actions. The server-IP redirect and channel reconnect handlers are implemented.
  The Start hit target now expands the decoded NX button bounds, including its authored
  origin, and its alignment was checked in a populated-screen hitbox preview. A live
  character-entry run is still needed to verify the server-side transition. The selection
  list pages through the full bounded server roster instead of hiding entries after the
  first ten.
  PIC registration and verification now follow
  the legacy opcodes, including masked input, confirmation, the disabled-PIC path,
  and incorrect-PIC retry. Character deletion has a confirmation action, optional
  PIC verification, and a bounded result handler. The supplied v83 request and server
  handler have no birthday challenge field or step; result state 18 is surfaced as an
  invalid-birthday failure when a server reports it.
  Character creation now has Explorer/Cygnus/Aran class selection, gender-specific
  appearance pools from `Etc.nx`, localized face/hair/equipment labels, an avatar preview,
  forbidden-name filtering, name availability packets, character creation packets, and
  live insertion of the created character into selection. Its bundled Roboto font matches
  the legacy client's font asset. Final UI polish and server-specific failure notices remain.
  The SET_FIELD parser reads the initial character's ID, stats, and inventory prefix or
  the compact map-change form. The bounded inventory parser handles the legacy equip,
  use, setup, etc, and cash item payloads plus `MODIFY_INVENTORY` deltas. The `I` key
  displays a six-tab inventory with a scrollable four-column slot grid and cached NX item
  icons. Use-inventory clicks dispatch `USE_ITEM`; equip, unequip, and within-bag drag moves
  dispatch `MOVE_ITEM`. The outgoing
  `MOVE_PLAYER` builder matches the legacy 9-byte prefix and absolute movement fragment;
  the native client sends changed snapshots at physics ticks. Bounded `CHANGE_STATS`
  packets update local character values, including variable Evan SP tables and the
  composite pet mask. The bottom HUD uses the stock v83 `StatusBar.img` background,
  gauge tracks/fills, toolbar buttons, and notice art; its live HP, MP, and EXP fills
  follow the v83 experience table. A transparent overlay draws the name and level/job
  in the stock two-row layout and uses `StatusBar.img/number` glyphs for right-aligned
  HP/MP/EXP readouts. Positive HP deltas show legacy blue `NoBlue` healing digits, and
  local and remote player damage uses the violet `NoViolet` glyphs from
  `Effect.nx/BasicEff.img`. When Zombify is active, a successful Heal cast followed by
  the server-reported HP loss displays violet backlash damage without granting contact
  invincibility. Local `GIVE_BUFF` and
  `CANCEL_BUFF` packets
  use bounded mask parsing, timed status storage, and composite Battleship-mask handling;
  Maple Warrior, Weapon Attack, Accuracy, and Hyper Body HP/MP affect local combat or HUD
  calculations. The `0x4000` Puppet buff and Zombify disease share a v83 mask, so the
  preserved mob-skill type distinguishes the Undead HUD status from Puppet. Booster's
  signed speed value combines with weapon attack speed for local
  attacks and remote attack animations. Player ailment statuses now appear in the HUD, stun blocks movement and
  attacks, confusion reverses horizontal controls, slow reduces acceleration, seal blocks
  skill use, and Darkness halves local attack accuracy. Timed active buffs now show their
  Skill.nx or item NX icon in the upper-right HUD, with the legacy bottom-up duration cover
  and final-three-second flashing. Poison HP changes remain
  server-authoritative. Achilles reduction
  applies to local mob-touch damage; other active-buff effects and combat mitigation
  remain open work.
  A bounded decoder handles all known v83 movement-fragment command layouts and rejects
  unknown commands before desynchronizing the packet. The client dispatches bounded parsers for `SPAWN_CHAR`,
  `REMOVE_CHAR`, `CHAR_MOVED`, and `UPDATE_CHARLOOK`; parsed changes update remote
  appearance and movement state and drive the rendered actor sprites. Position easing,
  teleport snapping, and movement stance/facing are ported. `DAMAGE_PLAYER` uses a bounded
  v83 parser and displays server-reported damage or miss feedback; hits apply packet-directed
  recoil without altering the remote movement target.
  Local mob contact reads NX `bodyAttack`/`PADamage` and current-frame bounds, calculates
  equipped WDEF and Achilles mitigation, rate-limits repeat reports, displays local
  damage/miss feedback and hit sound, and emits `TAKE_DAMAGE`; HP remains authoritative.
  Contact hits now grant the legacy two-second invincibility window and character flash;
  Dark Sight protects from contact while Booster no longer incorrectly does. Remote
  `GIVE_FOREIGN_BUFF` and `CANCEL_FOREIGN_BUFF` packets now fade and restore visible
  characters for Dark Sight and apply or remove remote Monster Riding art. Remote
  `DAMAGE_PLAYER` feedback also flashes the actor during the same invincibility window.
  Authoritative HP changes to zero now stop local movement and attacks, play the tombstone
  sound, animate the falling/landed tomb and hovering ghost with the character's head and
  hair, and show a modal nearest-town respawn notice; OK, Enter, Escape, or Space sends the
  legacy dead-character map-change request. Position correction for the local player
  remains unavailable because this server relays `MOVE_PLAYER` only to other clients.
  Initial `SET_FIELD` now parses the skill cooldown list and bounded active/completed quest
  sections, then preserves Teleport Rock destinations and reads the Monster Book cover and
  collected cards in the legacy minigame/ring/rock order. The v83 `ADD_COOLDOWN` packet sets
  or clears a skill's server-provided cooldown, and local skill casts honor its remaining
  duration. `MAP_TRANSFER_RESULT` refreshes the normal or VIP saved-map list after rock use.
  `SHOW_STATUS_INFO` updates quest progress, forfeits, and completions; `J` opens an
  NX-backed available/active/completed quest log with quest descriptions, live item and monster
  objective counters, an available-quest tab, server-authoritative quest start and turn-in
  actions, and a confirmed forfeit action. Map NPCs show NX-animated available,
  in-progress, and turn-in-ready quest marks using level, job, prerequisite, item, and mob
  progress. Quest-clear packets now play the NX completion sound and light-pillar effect.
  Quest log actions now select scripted start/end packets when the NX `startscript` or
  `endscript` requirement is set; those actions enter the same server-driven NPC dialogue
  flow. Non-scripted reward-selection items are browsable and their choice index is sent
  on turn-in; scripted reward choices are handled by the server dialogue.
  Quest-specific transitions still depend on the server having the matching quest scripts.
  Character-creation polish, server-specific birthday challenge support, remaining
  general opcode handlers, and browser
  WebSocket gateway remain open work. NPC spawn/remove and controller packets update
  map entities; the client loads linked NPC stand art from `Npc.nx`, shares GPU textures,
  and removes entities on both removal packet forms. It also parses `SPAWN_MOB`,
  `KILL_MOB`, `SPAWN_MOB_C`, `MOB_MOVED`, `MOVE_MONSTER_RESPONSE`, and `DAMAGE_MONSTER`,
  then loads mob stance frames and movement traits from `Mob.nx`. Controlled mobs use the
  shared foothold physics, legacy speed values, deterministic roam/jump choices, and send
  bounded `MOVE_MONSTER` claims and movement updates. Server-owned movement stays
  authoritative; duplicate spawn packets now reposition/revive an existing mob without
  uploading its art a second time or resetting an already acknowledged control claim.
  Mob stance fallback, direction metadata, death frames,
  and removal are connected to map state. Mob HP percentage and bounded status-mask
  packets update map state. HP bars appear for two seconds; boss mobs also show a four-second
  top-center HP gauge built from the v83 `UIWindow.img/MobGage` frames and boss icons, with
  names resolved from `String.nx`. Stun, freeze, and shadow-web
  statuses hold the mob in its stand animation until cancelled. `DAMAGE_MONSTER` displays
  the legacy `Effect.nx` digit sprites above a mob using a single GPU atlas, with the
  legacy rise and fade timing. A missing death clip falls back to an available mob stance
  and remains until the server removes it, matching the legacy `set_stance` behavior.
  Frozen, poisoned, and stunned mobs use the legacy blue,
  green, and yellow tint priorities. Mob spawn effect `-2` fades in over 40 fixed 8 ms
  updates. Kill animation `2` fades out from the current opacity over the same interval;
  `0` removes immediately, `1` plays the authored death clip, and `3` remains a no-op.
  Duplicate spawns that revive an existing mob restore full opacity. Mob movement metadata
  now selects one-shot `attack1`–
  `attack9` and `skill1`–`skill9` NX animations. A server-authorized controlled-mob skill is
  mapped through the mob's ordered `info/skill` entries and sent back as the matching v83
  action. `SPAWN_MIST` and `REMOVE_MIST` render and clear the legacy translucent poison,
  smoke-screen, and mob-skill area regions. NPC art now loads every authored NX animation
  state except `info`, advances each clip with its stored frame delays, and selects another
  available state when a clip completes, matching the legacy NPC stance cycle. Server
  `NPC_ACTION` codes still select `stand`, `move`, or `say`; absent art falls back to the
  stand/first available clip while preserving walk semantics. `move` and authored `walk`
  states now use shared foothold physics and reverse at platform edges. Summon spawn,
  movement, stance, and removal opcodes update the map actor
  state; stand/fly animation frames load from `Skill.nx`, with skill effect frames as a
  fallback. Server movement fragments update absolute and relative summon positions, and
  non-stationary summons use shared foothold physics between updates; animated summons
  participate in actor depth sorting. `SPAWN_DRAGON`, `MOVE_DRAGON`, and `REMOVE_DRAGON`
  now animate Evan dragons from the five job-tiered `Mob.nx` growth-stage assets, update
  their facing and movement, and advance them with map foothold physics; `native-dragon`
  previews the v83 sprite. Bounded `DROP_LOOT` and `REMOVE_LOOT` parsers update
  map drops, which resolve item/meso `iconRaw` assets from the existing NX files, animate
  meso frames, use shared foothold physics to fall and land, and fade or magnet toward the
  looter on removal. Duplicate active drops preserve their motion, and pet pickup effects
  fade without following the character. Pressing Z emits the v83 `PICKUP_ITEM` request for
  a nearby drop. Scriptable map NPCs respond to clicks, and Space sends `TALK_TO_NPC` for
  the nearest NPC within the legacy interaction range; typed dialogue
  and runtime scriptability packets are parsed, and a compact logical-coordinate panel
  supports selection, yes/no, accept/decline, text, number, next, and back/next responses.
  Scriptable NPC IDs persist within the active map scene so either packet ordering works.
  Script text expands player, NPC, map, mob, and item-name macros, strips style/control
  markers, preserves menu links, and removes repeated item IDs after their names. Item, quest,
  skill, and face icon macros now resolve the matching item/QuestIcon/Skill/Face NX assets and
  render inline with dialogue and choice text, including after redraws. Scripted quest state
  transitions still depend on server-provided quest scripts and their supported actions.
  `OPEN_NPC_SHOP` is bounded-parsed into original server slots and supports NX-named/icon
  buy and inventory sell views, quantity selection, rechargeable stock, `NPC_SHOP_ACTION`
  buy/sell/recharge/leave requests, and transaction-result feedback. Inventory changes
  refresh the sell view and map transitions clear shop state. The sell view omits items
  whose NX `info/notSale` flag is set, resolving equipment metadata from `Character.nx`
  and consumables from `Item.nx`; the transaction path rechecks that filtered inventory
  selection before sending a request. Sell rows display the item's legacy NX unit price
  with grouped meso digits. Perfect Pitch prices display the proper
  item cost and current ETC balance, and the client checks the required amount again when
  a confirmed purchase is sent. Items with neither meso nor Perfect Pitch pricing now show
  an in-game unsupported-currency notice instead of only logging to stderr. The separate
  token-for-mesos branch remains unsupported: Cosmic's `Shop.buy` path does not remove the
  token item when calculating the converted balance, so sending that transaction could
  produce an inconsistent result. Remaining item restrictions and richer server-error
  feedback are still open.
  Single-unit buys and sells plus rechargeable stock show a confirmation modal with mouse
  and Enter/Escape controls; stack quantities remain adjustable in the shop panel.
  Storage NPC responses now use a bounded v83 parser and an NX-backed, resizable-window
  panel with category tabs, scrollable storage and inventory lists, server-backed item
  actions, and typed meso transfers. Stack deposits and withdrawals now open quantity
  prompts; partial withdrawals retrieve the full stack and return the unselected remainder
  after the matching inventory delta arrives. Storage contents update only from server
  responses; live server verification remains open.
  Initial character entry now reads the bounded skillbook prefix, and `UPDATE_SKILL`
  refreshes learned/master levels. `K` opens a job-tabbed skill book populated from
  `Skill.nx` and `String.nx`; its rows can request SP allocation or use active skills.
  The v83 skill-book result packet is parsed and success/failure is reported in the HUD;
  learned levels remain driven by the separate server skill update.
  Skill attacks use per-level NX fixed or scaled damage, magic defense, hit counts, and
  target counts in the server-authoritative attack flow. Skill target selection now uses
  the legacy mastery/job/weapon checks and refuses casts without enough HP, MP, or matching
  ammo for the level's `bulletConsume`; inventory consumption stays server-authoritative.
  Attack target selection uses
  each level's NX hit rectangle and horizontal range multiplier for mob and reactor hits,
  including the legacy magic-skill range. Mob hit animations now resolve their NX variants
  by character level, skill level, and weapon hand, then follow local and broadcast target
  lists with mob head anchors and z ordering. Magic skill damage now derives
  its local range from INT/LUK, equipped and buffed MATK, and the NX `mad`/`mastery`
  values. The compact mastery rank maps to the localized skill-description percentage
  (for example, rank 1 → 15%, rank 10 → 60%); this fixes the legacy path that parsed
  `mad` but never applied it to damage. Skill and mob `elemAttr` values now apply the
  v83 neutral, resistance, weakness, and immunity multipliers to local attack damage.
  Other active skills emit `USE_SKILL`; teleport uses the learned NX range with foothold
  collision checks, and Flash Jump applies its aerial movement locally. Fourth-job master
  levels, skill prerequisites,
  and per-job SP tables constrain allocation. Learned weapon mastery, Angel's Blessing,
  and Berserk now contribute their NX-defined accuracy, attack, mastery, and conditional
  damage values to local attacks. Skills with authored `ball` frames now load root or
  per-level projectile art and launch toward each mob target for local and remote attacks;
  basic ranged attacks retain their equipped-ammo projectiles. Incoming v83 character
  attack broadcasts are decoded with bounded damage lines. They
  play weapon-appropriate NX attack stances for remote characters and show normal and
  critical damage plus the legacy normal miss glyph above affected mobs. Damage lines
  keep their legacy spacing and stacking. Left Control sends a one-hit regular attack
  packet and plays its weapon-appropriate local stance, using equipped NX stat data and
  the nearest mob in the facing arc when one is available. Compatible arrow, bolt, star,
  and bullet stacks select ranged attack packets, add any projectile NX attack bonus,
  target the legacy ranged arc, and animate local and remote projectile sprites; ammo counts remain
  server-authoritative. Reactor spawn, hit, and removal packets use `Reactor.nx` art,
  place reactors on map footholds, and play state-specific hit/removal animations. Hittable
  reactors are discovered by scanning every numeric state for an `event` node, fixing the
  legacy spawn-state-only check. Basic attacks send `DAMAGE_REACTOR` for the nearest
  hittable reactor intersecting the facing attack arc; hit state remains server-authoritative.
  Map portal entries now retain their destination metadata and legacy touch bounds. Regular
  and hidden portal animations load from `MapHelper.img`; hidden portals reveal on contact.
  Holding Up in a portal follows the legacy 48-tick warp cooldown, locally respawns at a named
  same-map destination, or sends the legacy `CHANGE_MAP` request for a cross-map transition.
  `SPAWN_DOOR`/`REMOVE_DOOR` and `SPAWN_PORTAL` now create or remove server-owned doors;
  door animations use the dedicated `MapHelper.img/portal/game/portal` node when present and
  fall back to the visible v83 portal animation when that node is absent. Server
  `CLOCK` time/countdown packets and `STOP_CLOCK` now drive the top-center v83 timer, using
  `UIWindow.img/muruengRaid` assets from the selected UI archive; countdown expiry follows
  the legacy fixed-step cadence, and `native-clock` provides an offline asset preview.
  Monster Carnival opcodes 289–294 now parse start, individual/party CP, summon, phase-message,
  and elimination updates. The client displays the v83 `UIWindow.img/MonsterCarnival` panel
  from `UI.nx`, including team/CP/summon counts, with the legacy close and side-view actions;
  `native-monster-carnival` provides an offline preview.
  `PLAYER_HINT`, `BROADCAST_MSG`, OX Quiz, Snowball, and Coconut event packets now follow the
  C++ chat notifications with bounded v83 payload parsing. Ariant result, Pyramid score,
  Dojo warp-up, and Tournament state notifications use the same event parser.
  `MAKER_RESULT` now decodes craft, crystal, and disassembly payloads, resolves item names from
  the existing String NX archive, and reports the resulting item changes in chat.
  `WEEK_EVENT_MESSAGE`, `BLOCKED_MAP`, and `BLOCKED_SERVER` now preserve MapleTip/notice text
  and surface the v83 entry or channel restriction reason.
  Monster Rider catch, item-assisted catch, and Bridle failure results also report the server's
  success or failure message. `SET_AVATAR_MEGAPHONE` and `CLEAR_AVATAR_MEGAPHONE` now consume
  the v83 packet layout, render the sender's character and four message strings in the 10-second
  Map.nx banner, and select the Burning/Bright/Heart/Tiger variants. The renderer follows the
  three authored message rows and drops overflow as the C++ client does; `native-avatar-megaphone`
  provides an offline preview using the same v83 archive art.
  `SET_BACK_EFFECT` now resolves and plays the server-selected map effect through the same
  v83 NX animation path as field effects. Remaining combat
  mitigation, special projectile variants and remaining skill damage formula
  cases, mob skill-effect visuals (server-selected mob skills now send their matching client
  action and play its NX animation when available, and `SPAWN_MIST`/`REMOVE_MIST` render the
  legacy colored area regions), and disease applications now play their per-level
  `Skill.nx/MobSkill.img` animation when the server-selected Seal, Darkness, or Weakness
  skill has authored effect frames. Local `GIVE_BUFF` and disease `GIVE_FOREIGN_BUFF` packets
  drive the animation; foreign disease decoding also handles Cosmic's persisted Slow mask
  layout. Other disease skills have no authored NX effect frames in this v83 asset set.
  Mob status drawing follows the C++ frozen double-pass blue overlay while preserving the
  NX-authored mob pose and other status tints.
  NPC shop sell lists now reject `notSale` and cash items. Personal-shop and hired-merchant
  stock selection and submission reject cash items and NX `tradeBlock` items, including a
  second check against the current inventory slot before sending. Remaining disease-specific
  visuals and other shop restrictions remain open work.
- The native macOS client build passes. This pass validates macOS only. The target
  boundaries retain Windows and browser/WASM as future platform goals; WASM still needs
  a browser lifecycle, a browser-backed asset source, and a WebSocket adapter, and
  Windows has not had a native run in this pass.
- Native `Sound.nx` skill-use sounds now follow local skill casts and remote skill-use or
  attack packets through the existing distance-attenuated mixer. The audio payload cache
  evicts older entries at a 64 MiB/4096-entry bound so new sounds can still be cached.
  Escape opens the options panel with graphics/effects quality, BGM/SFX and mute, mouse
  speed, HP/MP warnings, screen shake, windowed mode, and the legacy 800x600/1024x768 choices.
  Saved `Width`, `Height`, and `Fullscreen` values configure startup and the Options controls
  apply display changes live. Graphics quality follows the legacy thresholds for
  foregrounds, environments, mists, and weather; effects quality applies the legacy 2–50
  concurrent-effect budget to rendered skill, projectile, damage, quest-clear, and screen
  effects. A zero warning threshold disables its low-resource overlay. Audio, display, input,
  gameplay, and all twelve v83 social permission values persist in per-user platform settings
  files, and cancel restores the saved values. Mute is session-only like the C++ client;
  Windows applies its system mouse-speed mapping, while other platforms preserve the C++
  no-op. Supported whisper, group-chat, invite, family, and trade receive paths honor their
  permissions; expedition and follow request flows are not yet implemented. Skill hit sounds
  resolve `Sound.nx/Skill.img/<skill>/Hit` at local or remote targets using the v83 action
  timeline's signed frame delays, the weapon afterimage first-frame fallback, and the legacy
  attack-speed factor. The same timeline schedules each target's hit effect and damage line;
  a missing action frame retains the C++ zero-delay behavior. Live timing comparison and
  the remaining sound coverage still need a gameplay run.
  The C++ client stores the screen-shake preference but has no shake trigger path;
  simple-minimap mode, the unsupported legacy menu actions, broader legacy sound coverage,
  and a browser audio backend remain open work.
- The mage damage formula and equipment/buff stat path compile for native macOS. The
  C++ client does not apply parsed `mad` to the damage range, so matching the corrected
  local damage display against server-authoritative combat remains to be confirmed in a
  mage combat session. Element parsing was checked against Fire Arrow's localized
  weakness/resistance description and mob `elemAttr` examples in the installed NX files.
- The executable can list archive statistics, inspect/decode an NX node, summarize a map
  with `--inspect-map`, and resolve character base animation with `--inspect-avatar`.
  The 100000000 map currently resolves to 324 visible sprite instances across 90 unique
  textures and 23 animated map-art entries with 135 frames. The sample
  `--inspect-avatar 0 20000 30000`
  resolves eight stand/walk/jump frames with 56 frame layers across 49 textures. The
  archive validator covered all 17 NX files, and the v83 login background decoded as
  800x600. NX archive files are opened lazily and are not copied into this repository.
  Node, string, and decoded-bitmap caches now evict by LRU recency through bounded stale-entry
  queues, avoiding full-cache scans when large archive walks replace cached entries.
- Incoming v83 `FORCED_STAT_SET` packets now decode all 12 mask-controlled values.
  Primary stats update immediately; forced PAD/MAD/ACC and WDEF feed local combat, while
  SPEED/JUMP override movement totals with legacy caps. The status panel shows forced
  totals, and `RECALCULATE_STATS` clears those derived overrides. Live server behavior still
  needs confirmation.
- Full legacy parity remains open. Known gaps include remaining map/environment and effect
  states, combat formula and status coverage, consumable-specific item actions, richer
  server failure feedback, and additional client screens. The browser lifecycle, WebSocket
  gateway, ranged asset delivery, persistence, and browser audio adapter also remain future
  work; native macOS is the first delivery target.
- v83 guild/family level-up and job-advancement packets now decode their group, name, and
  level/job fields and display the matching legacy yellow chat notice.
- v83 spouse/fiance chat and ship transition/status packets now decode into the legacy
  red/yellow chat notices. `FORCED_MAP_EQUIP` now reports the server-applied map restriction.
- v83 marriage requests/results, wedding gift results, and partner map changes retain the
  reference client's social feedback in chat. Ceremony progress/end packets use the v83
  `UIWindow.img/Wedding` Cathedral panel and server-provided countdown; `native-wedding`
  previews the panel offline from the active v83 UI archive. The countdown keeps the reference
  placement over the invitation copy and still needs a contrast/layout pass.
- `SPAWN_GUIDE` and both `TALK_GUIDE` modes now display the reference client's yellow chat
  feedback. Canned text resolves from the selected v83 UI archive's `tutorial.img`, then falls
  back to its legacy hint ID when only tutorial art is present.
- The `LEFT_KNOCK_BACK` response now applies the C++ client's `hspeed = -4` impulse to the
  local map player.
- `PET_EXCEPTION_LIST` now bounded-parses its pet ID and signed item list and displays the
  reference handler's exclusion-count notice.
- `LOCK_UI` and `DISABLE_UI` now share a v83 boolean parser and gate mouse-driven UI actions;
  keyboard input remains available like the reference client.
- Added v83 feedback for character-report results, the under-level trade meso limit, Vicious
  Hammer, and Vega Scroll; report results use the existing dismissible OK notice.
- MapleLife name rejection/error and New Year card result packets retain the C++ client's
  v83 feedback messages.
- `IMITATED_NPC` now decodes the v83 disguise look, renders the NPC as the supplied player
  avatar with the installed Character NX equipment, and removes the NPC on the mode-zero
  packet, matching the reference handler.
- `THROW_GRENADE` now plays the remote caster's skill action and draws the skill's authored
  `effect` animation once at the server-provided landing point, using the v83 skill NX data.
- `SCRIPT_PROGRESS_MESSAGE` and `CANNOT_SPAWN_KITE` now render as fading v83 status text;
  kite spawn/removal packets retain the reference yellow chat notices. Server `PING` packets
  now receive the v83 `PONG` response on the active session. Name-change/world-transfer
  cancellation results and incubator completion also retain their v83 chat notices. Cash Shop
  name-availability, name-change eligibility, world-transfer results, and Gachapon outcomes now
  use the reference client's transient status or chat feedback.
- `RPS_GAME` now opens the server-driven Rock-Paper-Scissors panel from the v83
  `UIWindow.img/RpsGame` assets. Start, hand selection, retry/continue, exit and Escape controls
  follow the reference packet flow, and results update the displayed hands and win count.
