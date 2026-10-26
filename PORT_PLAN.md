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
  Rust should read these in place; do not copy them into this repository or bundle.
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
  for each prompt, including the highlighted choice and Yes/Accept prompts. Either Option key jumps.
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

The supplied v83 `UI-v83.nx` contains `Login.img/NewChar`, `NewCharKnight`, and
`NewCharAran`, but not the `Login.img/CustomizeChar` node referenced by the newer C++
creation screens. The Rust creator resolves the v83 class-specific nodes directly so
this asset set remains usable without changing or replacing the archives.

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
  server-confirmed withdrawals. Native macOS compilation covers these changes; the
  updated Cash Shop has not yet been exercised against a live Cash Shop server response.
  The quest detail pane now lets the player choose among eligible `prop=-1` item rewards
  with on-screen arrows or Left/Right, and completion sends the corresponding v83 choice
  index.

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
  previews the selected bitmaps. The chat commands remain available as shortcuts.
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
  corresponding invitations. The character-info request/response flow has not yet been
  exercised against the local server.
- The Rust workspace and application shell are in place. It opens a resizable wgpu
  window with a 1024x768 logical canvas and letterboxes its 4:3 content. The login
  screen uses layered v83 art from Map.nx and UI-v83.nx, with editable account and
  password fields, password masking, viewport-mapped mouse input, and login/guest/quit
  controls. Successful authentication now opens a world/channel picker built from
  legacy assets; selecting a world and channel requests that channel's character list.
  A basic character list view displays server stats, supports PIC setup/verification,
  and provides character deletion with confirmation. Character selection now follows the
  server-IP redirect to a channel connection, sending the separate channel-login packet
  after its handshake. A bounded compact `SET_FIELD` parser now handles in-channel map
  changes; initial entry uses the selected character's map and portal from the login
  character list. The map loader reads foothold groups to place the initial view at
  ground level. The first map scene loads NX backgrounds, object frames, and tiles in
  legacy depth order, with viewport culling and GPU texture reuse. Background parallax,
  independent horizontal/vertical motion, and tiled wrapping follow each entry's type;
  server obstacle packets now control map environment visibility and animation resets. A map entry
  composes the selected character's body, head, face, hair, and equipped items. It plays
  authored one-handed or two-handed stand/walk frames and jump frames at NX delays.
  Character selection renders a compact equipped-avatar preview beside each row. The
  avatar renderer now composes every legacy body z layer, including arm-below-head,
  hand-below-weapon, and hand-over-hair parts, so those layers interleave with clothing
  and weapons. Arrow keys or A/D drive local
  foothold movement at the shared 8 ms tick; either Option key jumps, Space talks to the
  nearest nearby NPC and advances that dialogue, the camera
  follows, and the base avatar turns. Nearby map art is retained within a 512-pixel
  camera margin. Avatars resolve equipped hats, face accessories, armor, shoes, gloves,
  capes, shields, and weapons from their NX categories, align item attachment vectors to
  body/head/hand anchors, honor masked slots, and order sprites by legacy z layers. The
  initial field entry now parses meso, slot maxima, equipped items, equipment, and stack
  items; inventory add/count/remove/swap packets update that state, and `I` opens a
  six-tab inventory with a scrollable four-column slot grid, cached NX item icons, stack
  counts, meso, and slot capacity. Ordinary Use items send the v83 `USE_ITEM` request;
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
  one. Pet food uses the distinct v83 `PET_FOOD` request so the server feeds the hungriest
  summoned pet, and activating a Cash inventory pet sends the legacy `SPAWN_PET`
  toggle request. Standalone pet spawn/removal and movement packets plus pet records in
  remote-character spawns now update local/remote map actors; pet stance art and names load
  from the existing NX files. Local pets follow their owner using shared foothold physics;
  the v83 pet AI selects nearby drops and sends the cash-serial `PET_LOOT` request with the
  legacy cooldowns. Pet chat bubbles and server pet-command interactions now use the pet slot,
  NX interaction results, fallback stances, and the matching pet sound category. Supported
  regular, super, item, and triple cash megaphones, Maple TV, avatar megaphones, and pet name
  tags compose through chat and send their v83 `USE_CASH_ITEM` payloads. Item megaphones
  currently omit the optional attached item; Maple TV leaves the recipient name empty, and
  Maple TV and avatar megaphone accept one line and pad remaining rows. Other targeted cash
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
  are sent through the v83 packet. Item.nx lookup paths now match the actual nested node
  names for inventory icons, drops, and projectiles; the nested item node does not include
  an `.img` suffix. Avatar frames keep
  scene draw indices separate from GPU texture indices, so dynamic equipment refreshes
  resolve the intended texture while retaining map draw suppression. Full equipment edge
  cases and special projectile rules,
  full legacy character-creation polish, remaining modal dialogs, and link destinations
  remain open work. Title login buttons now show NX disabled, mouse-over, pressed, and
  keyboard-focus states where the archive provides them, with a gold-outline fallback for
  buttons lacking dedicated focus art; Tab and arrows navigate enabled actions, while Enter
  and Space activate the focused action. Login and Guest use the disabled state until the
  server connection is ready, and Login also checks the credential lengths. Login's Save ID
  control now persists the account name and
  checkbox state without storing the password; audio settings updates preserve these values.
  Redraws are capped near 60 Hz. The
  renderer supports reusable GPU textures, source-UV atlas draws, per-sprite tint and
  opacity, culls off-canvas sprites, and batches adjacent quads sharing a texture page into
  one draw call. Scene art is packed into 2048-pixel atlas pages with duplicated gutters
  for clean linear sampling; oversized images retain standalone textures. Runtime NX
  map-art pixels use a 128 MiB, 16,384-entry FIFO cache while GPU pages are shared for
  the current scene. Map object and `back/ani` frames now
  advance from their NX delays while inactive frame draws stay culled. The six login light
  beams now animate independently through their NX-authored frames and retain additive
  blending. Repeated software-rendered panel text reuses glyph coverage
  from a bounded 16 MiB FIFO cache. GPU glyph-atlas rendering, general scene atlasing,
  complete environment and animated-effect parity remain open work.
- `openstory-core` contains the 1024x768 letterbox viewport conversion, an 8 ms
  fixed-step clock with capped catch-up, and an initial foothold geometry/index layer.
  Footholds are indexed in 64-pixel buckets rather than one entry per covered pixel;
  duplicate/reserved IDs and empty or degenerate bounds are handled safely. Normal,
  flying, and swimming physics now have a fixed-tick body model with slope traversal,
  gravity, traction, linked foothold walls/edges, and interpolated positions. The old
  left-walk slope comparison bug is corrected. Map footholds drive initial spawn
  height and local movement/camera tracking. The client now emits v83 absolute player
  movement fragments when its rounded position, previous position, foothold, or stance
  changes. Initial inventory snapshots and updates are supported; item-specific targeting,
  server correction, complete skill/passive-stat parity, special projectile variants, and
  other gameplay systems remain open work. The client parses v83
  character spawn, remove, movement,
  and look-update packets. It also parses `SPAWN_NPC`, `REMOVE_NPC`, and `SPAWN_NPC_C`.
  Remote characters are drawn from their NX skin, face, and hair animations, with name
  labels, stance-based facing, server-position easing, and snap behavior for teleports
  and large corrections. `SKILL_EFFECT` and skill-type `SHOW_FOREIGN_EFFECT` packets load
  NX skill-use frames, apply skill-selected attack stances, follow remote character facing,
  and use the legacy teleport animation for supported teleport skills. Level-up and
  monster-card `SHOW_FOREIGN_EFFECT` events also play their `BasicEff.img` animation over
  local and remote characters. The renderer draws
  drops, mobs, NPCs, local player, and remote players in deterministic world-Y order before
  foreground map art. Local attack casts now use the same NX level/weapon-aware effects and
  skill-selected action stances; non-attack skill casts also show their local use effects.
  Equipment edge cases, less common skill hit variants, skill hit-sound timing, cancellation
  and remaining non-skill foreign effects, and the full legacy character state machine remain
  open work.
- `openstory-assets` has a bounds-checked, lazy PKG4 NX reader with typed nodes, LZ4
  bitmap decoding, audio extraction, and validation of the 17 legacy data files. Its
  on-disk header and record layout match the vendored NoLifeNx definitions. Node records
  and strings use bounded FIFO caches, limiting long-session archive metadata memory.
  Decoded bitmap pixels now use a per-archive 16 MiB/4096-entry FIFO cache and can be
  shared by renderer paths without copying. `RandomAccessSource` and `MemorySource`
  let a browser host provide already-fetched or range-cached archive bytes without a
  native file API; browser fetch/persistence and an efficient range-cache adapter remain
  open work. Archive-level audio caching also remains open work.
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
  screen now drives the character-list request. Character-list display and selection
  redirect are partially implemented. The selection list now pages through the full
  bounded server roster instead of silently hiding entries after the first ten.
  PIC registration and verification now follow
  the legacy opcodes, including masked input, confirmation, the disabled-PIC path,
  and incorrect-PIC retry. Character deletion has a confirmation action, optional
  PIC verification, and a bounded result handler. The supplied v83 request and server
  handler have no birthday challenge field or step; result state 18 is surfaced as an
  invalid-birthday failure when a server reports it.
  Character creation now has Explorer/Cygnus/Aran class selection, gender-specific
  appearance pools from `Etc.nx`, an avatar preview, forbidden-name filtering, name
  availability packets, character creation packets, and live insertion of the created
  character into selection. Final UI polish and server-specific failure notices remain.
  The SET_FIELD parser reads the initial character's ID, stats, and inventory prefix or
  the compact map-change form. The bounded inventory parser handles the legacy equip,
  use, setup, etc, and cash item payloads plus `MODIFY_INVENTORY` deltas. The `I` key
  displays a six-tab inventory with a scrollable four-column slot grid and cached NX item
  icons. Use-inventory clicks dispatch `USE_ITEM`; equip, unequip, and within-bag drag moves
  dispatch `MOVE_ITEM`. The outgoing
  `MOVE_PLAYER` builder matches the legacy 9-byte prefix and absolute movement fragment;
  the native client sends changed snapshots at physics ticks. Bounded `CHANGE_STATS`
  packets update local character values, including variable Evan SP tables and the
  composite pet mask, and a reusable GPU texture displays live HP, MP, EXP, and meso
  values when supplied in the logical map HUD. Positive HP deltas also show the legacy
  blue `NoBlue` healing digits from `Effect.nx/BasicEff.img`. Local `GIVE_BUFF` and
  `CANCEL_BUFF` packets
  use bounded mask parsing, timed status storage, and composite Battleship-mask handling;
  Maple Warrior, Weapon Attack, Accuracy, and Hyper Body HP/MP affect local combat or HUD
  calculations. Player ailment statuses now appear in the HUD, stun blocks movement and
  attacks, confusion reverses horizontal controls, slow reduces acceleration, seal blocks
  skill use, and Darkness halves local attack accuracy. Poison HP changes remain
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
  `DAMAGE_PLAYER` feedback also flashes the actor during the same invincibility window.
  Authoritative HP changes to zero now stop local movement and attacks, play the tombstone
  sound, animate the falling/landed tomb and hovering ghost with the character's head and
  hair, and show a modal nearest-town respawn notice; OK, Enter, Escape, or Space sends the
  legacy dead-character map-change request. Position correction for the local player
  remains unavailable because this server relays `MOVE_PLAYER` only to other clients.
  Initial `SET_FIELD` now consumes the cooldown and bounded active/completed quest sections.
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
  packets update map state. HP bars appear for two seconds; stun, freeze, and shadow-web
  statuses hold the mob in its stand animation until cancelled. `DAMAGE_MONSTER` displays
  the legacy `Effect.nx` digit sprites above a mob using a single GPU atlas, with the
  legacy rise and fade timing. Frozen, poisoned, and stunned mobs use the legacy blue,
  green, and yellow tint priorities. Mob movement metadata now selects one-shot `attack1`–
  `attack9` and `skill1`–`skill9` NX animations. A server-authorized controlled-mob skill is
  mapped through the mob's ordered `info/skill` entries and sent back as the matching v83
  action. `SPAWN_MIST` and `REMOVE_MIST` render and clear the legacy translucent poison,
  smoke-screen, and mob-skill area regions. NPC stand animations now advance through NX-authored
  frames using their stored delays. NPC art also loads the authored `stand`, `move`, and
  `say` states, and server `NPC_ACTION` packets select the matching animation with a safe
  stand fallback. Summon spawn, movement, stance, and removal opcodes update the map actor
  state; stand/fly animation frames load from `Skill.nx`, with skill effect frames as a
  fallback. Server movement fragments update absolute and relative summon positions, and
  non-stationary summons use shared foothold physics between updates; animated summons
  participate in actor depth sorting. Bounded `DROP_LOOT` and `REMOVE_LOOT` parsers update
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
  a confirmed purchase is sent. The server's separate token-for-mesos shop branch has no
  clear client wire contract and remains open, along with richer server-error feedback
  and remaining item restrictions.
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
  Holding Up/W inside a portal follows the 48-tick warp cooldown, locally respawns at a named
  same-map destination, or sends the legacy `CHANGE_MAP` request for a cross-map transition.
  other combat mitigation, special projectile variants and remaining skill damage formula
  cases, mob skill-effect visuals (server-selected mob skills now send their matching client
  action and play its NX animation when available, and `SPAWN_MIST`/`REMOVE_MIST` render the
  legacy colored area regions),
  disease-specific healing effects, spawn/fade effects,
  additional NPC movement states, richer NPC shop transaction feedback, and remaining
  shop inventory restrictions remain open work.
- The native macOS client build passes. This pass validates macOS only. The target
  boundaries retain Windows and browser/WASM as future platform goals; WASM still needs
  a browser lifecycle, a browser-backed asset source, and a WebSocket adapter, and
  Windows has not had a native run in this pass.
- Native `Sound.nx` skill-use sounds now follow local skill casts and remote skill-use or
  attack packets through the existing distance-attenuated mixer. The audio payload cache
  evicts older entries at a 64 MiB/4096-entry bound so new sounds can still be cached.
  Escape opens the audio options panel with live BGM/effects sliders and save/cancel
  actions. Volumes persist in per-user platform settings files, and cancel restores the
  last saved values. Key bindings, graphics/effect quality, screen shake, warning
  thresholds, social preferences, skill hit-sound timing, broader legacy sound coverage,
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
- Full legacy parity remains open. Known gaps include remaining map/environment and effect
  states, combat formula and status coverage, consumable-specific item actions, richer
  server failure feedback, and additional client screens. The browser lifecycle, WebSocket
  gateway, ranged asset delivery, persistence, and browser audio adapter also remain future
  work; native macOS is the first delivery target.
