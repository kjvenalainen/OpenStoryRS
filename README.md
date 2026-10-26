# OpenStoryRS

This repository is the Rust port of the OpenStory v83 MapleStory game client. The
implementation roadmap is in [PORT_PLAN.md](PORT_PLAN.md).

The client uses the same NX assets as the legacy C++ client. Keep those files in the
existing `OpenStory/wz` directory; point the client at them with
`OPENSTORY_DATA_DIR=/path/to/wz` or `--data-dir /path/to/wz`.

The native client opens v83 login and world/channel artwork in a resizable GPU-backed
window, keeps a 1024x768 logical canvas, and scales with letterboxing. The account and
password fields accept text; the password is masked. The selected world and channel
can be used to request a character list. Characters display name, level, job, and map.
The login background's six light beams animate independently using their NX frame delays;
login buttons use their NX disabled, mouse-over, and pressed art. Keyboard focus uses NX
focus art when available and a gold outline otherwise. Tab cycles through the account field,
password field, and enabled actions; arrows move among actions, and Enter or Space activates
the focused action. Login and Guest stay disabled until the server connection is ready, and
Login also requires valid account/password lengths.
The selection screen pages through all characters returned by the server, ten at a time.
Character selection supports PIC registration, verification, and the disabled-PIC path;
PIC entry is masked and accepts 6–16 letters or numbers from the keyboard. Each row
also has a Delete action with confirmation and optional PIC verification. Set
`OPENSTORY_MACS` and `OPENSTORY_HWID` to attach legacy selection hardware strings; both
default to empty, and the client does not collect hardware identifiers automatically.
When a character slot is free, Create Character opens Explorer, Cygnus Knight, and Aran
class choices. The customization screen reads gender-specific face, hair, color, skin,
clothing, shoe, and weapon pools from `Etc.nx`, previews the selected avatar, checks
forbidden names locally, and uses the v83 name-check/create packets. The server's new
character entry is added to the current selection list.

Set `OPENSTORY_FONT_PATH` to select a TrueType font; macOS, Windows, and Linux
system-font locations are used as fallbacks. To connect, run with
`--connect HOST:PORT`. The client completes the Maple handshake and sends the
login-start packet. The login button submits the fields. Set `OPENSTORY_ACCOUNT` and
`OPENSTORY_PASSWORD` to prefill them, or set `OPENSTORY_AUTO_LOGIN=1` to submit them
after the handshake. The Save Login ID checkbox remembers the account name only; the
password is never written to settings. `OPENSTORY_MACHINE_ID` optionally supplies the legacy machine
serial as hexadecimal and defaults to zero.
Set `OPENSTORY_DEBUG_NETWORK=1` to log connection stages and packet opcodes for diagnosis;
packet contents are not logged. Set `OPENSTORY_DEBUG_LOG=/path/to/file` to keep those
diagnostic lines in a file when launching through macOS Launch Services. Set
`OPENSTORY_BGM_VOLUME` or `OPENSTORY_SFX_VOLUME` to a percentage from 0 to 100.

To package a native macOS app bundle, run `scripts/package-macos.sh`; it writes
`target/OpenStoryRS.app` and signs the local bundle ad hoc for development.

The NX reader, fixed-step clock, logical viewport, foothold physics, bounded packet
primitives, v83 Maple encryption, partial-frame decoder, and login, world-list,
character-list, server-status, redirect, compact map-change, and movement-fragment
parsers are in place. The client builds v83 `MOVE_PLAYER` snapshots and sends them when
the local movement state changes.
`openstory-assets` reads through a platform-neutral random-access source: native builds
open NX archives lazily from files, and browser hosts can supply a memory-backed source
after fetching or range-caching archive data. Browser fetch, persistence, and range-cache
policy remain part of the future WASM adapter.
After channel selection, the client loads map backgrounds, objects, tiles, and environments
from the same NX archives, caches repeated art textures, and advances animated object and
background frames using their NX delays. Scene images share bounded GPU atlas pages to
reduce texture changes while preserving authored draw order. Background entries honor
their parallax, tiling,
and horizontal or vertical motion type; server obstacle packets toggle map environments.
The first `SET_FIELD` transition
uses the selected character's map and portal from the character list. The map scene shows
the selected character's body, head, face, hair, and equipped items. Character selection
shows a compact equipped-avatar preview beside each character row. Use Left/Right or A/D
to walk; left or right Option jumps; Space talks to the nearest nearby NPC. Up or W
activates map portals. The avatar uses NX-authored one-handed or two-handed
stand/walk frames and jump frames, including the legacy body arm and hand z layers.
Press F1–F7 to play and broadcast the matching facial expression; NX-authored face
frames animate locally and for other visible players.
The client sends v83 movement snapshots when position, prior position, foothold, or
stance changes. Remote spawn, removal, movement, and look updates now drive visible
characters using NX body, face, hair, and equipped item art, aligned through attachment
points and ordered by equipment z layers, with name labels, stance-based facing, and
smoothed server positions. NPC spawn, controller, and removal packets draw art from
`Npc.nx`, including linked NPC definitions and animated stand, move, and say states
selected by server `NPC_ACTION` packets. Mob spawn,
controller, movement, and kill packets update map entities using `Mob.nx` stance art.
Summon spawn, movement, stance, and removal packets load and animate stand/fly frames
from `Skill.nx`, falling back to skill effect frames when needed; summons are ordered
with map actors by depth. Non-stationary summons use shared foothold physics between
server movement updates. Remote `SKILL_EFFECT` and skill-type `SHOW_FOREIGN_EFFECT` packets
play NX-authored skill-use effects, update remote facing and attack stance, and use the
legacy teleport effect for supported teleport skills. `SHOW_FOREIGN_EFFECT` level-up and
monster-card events play their matching `Effect.nx/BasicEff.img` animations over the local
or remote character.
`DROP_LOOT` and `REMOVE_LOOT` update map drops with item or meso icons from the existing
NX archives. Drops animate while falling, float after landing, and fade or move to a looter
when collected; meso icons animate from their NX frame data. Press Z near a drop to send
`PICKUP_ITEM`. Reactor spawn, hit, and removal packets load art from `Reactor.nx`, place
reactors on map footholds, and play their state-specific hit animations. Basic attacks
send server-authoritative `DAMAGE_REACTOR` requests when a hittable reactor intersects the
attack arc. Regular portals animate from `MapHelper.img`; hidden portals reveal their
animation when approached. Holding Up inside a portal requests a server map change, while
same-map portals move the player to their named destination after the legacy cooldown.
Click a scriptable NPC to request dialogue; the dialogue panel handles
selection lists, yes/no and accept/decline prompts, text and number input, and page
navigation. Press Space or click to reveal a line as it types; press Space again to advance
or accept the current choice. On text-entry prompts, Space reveals the prompt first and then
remains available for spaces in the answer; Enter submits it. Up/Down moves the selection
highlight through long lists, which also scroll with the mouse wheel. Space submits number
prompts. Outside dialogue, Space requests a conversation with the nearest NPC within
legacy interaction range; both Option keys jump. When the server opens an NPC shop, its
Buy and Sell views use NX item names and
icons; sell rows show the legacy NX unit price and exclude items marked `notSale` using
the correct item archive. Quantity controls send v83 buy/sell requests, rechargeable
stock sends the recharge request, and inventory updates remain server-authoritative. A bottom HUD displays
character level, live HP and MP bars, experience, and
mesos when the server sends a `CHANGE_STATS` update. HP updates display a two-second
bar, and positive HP deltas float the legacy blue healing digits from `Effect.nx` above
the character. Stun, freeze, and shadow-web hold mobs in their stand animation until the server
cancels those statuses. `DAMAGE_MONSTER` packets float the legacy digits
from `Effect.nx` above the mob with a 1.5-second rise and fade. Controlled mobs use the
shared foothold physics and v83 `MOVE_MONSTER` updates, while remote movement remains
server-authoritative. In a field, press Enter to open public chat, type a message, and
press Enter to send; Escape closes the composer. Server chat broadcasts appear in a
scrolling HUD panel, with GM messages highlighted. Visible speakers show four-second
speech balloons drawn
from the v83 `ChatBalloon.img` frame assets. While the composer is open, Tab cycles public,
buddy, guild, alliance, and party chat. `/w <character> <message>` sends a whisper,
`/find <character>` requests the player's channel, and `/pt <message>` sends directly to
party chat. `/party create`, `/party invite <character>`, `/party join <id>`, `/party accept`,
`/party deny`, `/party leave`, `/party expel <character>`, and `/party leader <character>`
manage party membership; `/party list` toggles the party panel. Press O to show party
members, channels, and server-reported HP.

When no dialog or panel is open, press Escape to open Options. Drag the music and sound
effect sliders, or use Tab and the arrow keys; Save or Enter persists them. Cancel or
Escape restores the saved values. On macOS, settings are stored in
`~/Library/Application Support/OpenStoryRS/settings.ini`. The file also stores the Save
Login ID choice and account name. Saving audio options preserves these values and removes
any legacy saved-password entry.
Incoming group chat, whisper delivery errors, and find replies appear in the colored HUD
history. Incoming whispers also open a dedicated conversation window. Press H to start a
new whisper, type a character name, press Tab to enter a message, and press Enter to send;
`/w <character>` opens or switches conversations, while `/w <character> <message>` sends
directly. Conversation histories and drafts are kept per character. Press B to show the
scrollable buddy list; `/buddy add <character>`,
`/buddy group <character> <group name>`, `/buddy filter <group name>` (or `all`),
`/buddy remove <character>`, `/buddy accept`, and `/buddy dismiss` manage buddies; the list
shows each buddy's server-assigned group. Re-adding
an existing buddy is blocked locally so it cannot silently reset their group to Default Group.
Dismissing hides the pending invite locally, matching the legacy client behavior.
`/trade <visible character>` creates a v83 trade room and invites that player; use
`/trade accept` or `/trade decline` for incoming invitations. In the trade room, press I and
drag inventory stacks onto your offer grid, enter a quantity when prompted, set a meso offer,
chat, and confirm or cancel. Server updates control both sides of the trade and the final result.
Click a visible character to request their v83 character-info panel; Escape or its close button
dismisses the panel. Its Party, Trade, and Guild buttons send the corresponding invites.
Clicking a visible personal-shop owner opens their shop.
Press G to open the scrollable guild roster and accept or decline pending guild invites
with Y or N. When not in a guild, click **Create Guild**, enter a 3–12 letter name, and
submit it to start the server's nearby-cofounder confirmation flow. Guild members can open
the **Ranks** editor to change all five rank titles, or the **Emblem** editor to choose
valid backgrounds, colors, and marks from `UI.nx/GuildMark.img` with a live asset preview.
`/guild create <name>` is the equivalent chat shortcut. When another player asks you to
cofound, press Y/N to agree or decline. `/guild invite <character>`,
`/guild leave`, `/guild expel <member>`,
`/guild rank <member> <1-5>`, `/guild ranks <r1>|<r2>|<r3>|<r4>|<r5>`,
`/guild emblem <bg> <bg color> <logo> <logo color>`, and `/guild notice <text>` send
server-authoritative guild management requests. Press V to browse the guild bulletin board, write posts and notices, reply to
threads, and delete selected posts. Press U for the alliance panel; `/alliance invite <guild>`,
`/alliance expel <guild>`, `/alliance leave`, `/alliance notice <text>`, and
`/alliance ranks <r1>|<r2>|<r3>|<r4>|<r5>` manage alliance settings. `/alliance accept` and
`/alliance deny` answer invitations, and Y/N also works for pending alliance invitations. The panel uses server-sent alliance roster
broadcasts and shows the local alliance ID while a roster is unavailable; rank-title and
alliance-member updates are consumed from the server. Press Left Control to
send a one-hit regular attack; the client
uses the equipped weapon data, plays its NX stance, and targets the nearest mob ahead.
Bow, crossbow, claw, and gun attacks select a compatible projectile stack, add any NX
attack bonus it provides, use the v83 ranged packet and range, and animate the projectile.
Remote ranged attacks also animate their server-reported projectile; the server owns ammo
consumption. Incoming attack broadcasts animate remote characters with weapon-specific NX
stances and show normal, critical, or miss feedback above mobs. `DAMAGE_PLAYER` packets
show bounded damage/miss indicators and server-directed recoil above remote characters.
Body-attacking mobs use NX
attack values and current-frame bounds to send v83 `TAKE_DAMAGE`; equipped WDEF and Achilles
reduce contact damage, and local hits show damage/miss feedback with recoil. Reports are
limited to one per 500 ms of continuous overlap while HP remains server-authoritative.
Frozen, poisoned, and stunned mobs receive the legacy blue, green, and yellow tints. When
the server authorizes a controlled mob skill, the client sends the action selected from the
mob's ordered `info/skill` entries and plays its matching NX animation. `SPAWN_MIST` and
`REMOVE_MIST` render and clear the legacy translucent poison, smoke-screen, and mob-skill
area effects. NPC shops now show NX item names and icons, support buying, selling, rechargeable stock, and server transaction
results. Perfect Pitch purchases show their item cost and available balance, and requests
are blocked when the ETC inventory lacks enough. NPC storage uses a two-column, NX-backed item view with category tabs, item
store/take actions, and meso transfers. Clicking a stack opens a quantity prompt for partial
deposits and withdrawals; withdrawals retrieve the v83 stack and return the unselected
remainder to storage after the inventory update. Inventory and storage contents remain
server-authoritative. Press K to open a class-tabbed skill book built from `Skill.nx` and `String.nx`;
learned levels update from server packets, skill points can be spent, active skills can be
used, and attack skills use their NX damage type, scaling, hit count, and target count in the
attack request path. Skill use checks the legacy job, weapon, HP, MP, and ammunition costs
before sending a request; inventory consumption remains server-authoritative. Attack target
selection uses the skill level's NX hit rectangle and
horizontal range multiplier for both mobs and reactors. Skill hit animations follow the NX
character-level, skill-level, and weapon-hand variants and attach to each mob in the attack
broadcast. Local casts also play the selected NX use effect and skill action, including
level and weapon-hand variants. Projectile skills load their root or per-level `ball` frames
from `Skill.nx` for local and remote attacks, and fly toward each reported target; ordinary
ranged attacks continue to use the equipped ammo art. Teleport uses its learned NX range and stops at
foothold walls or large floor changes; Flash Jump applies its aerial movement locally.
Press J to open the available/active/completed quest log; its state
comes from the initial character packet and server quest updates, while names, eligibility,
and descriptions come from `Quest.nx`. The available tab can start quests and ready active
quests can be turned in through v83 quest actions. Objective rows show held item counts and
server-reported monster progress, and nearby NPCs show available, active, or ready markers.
A quest with non-scripted selectable rewards displays an item chooser in its detail panel; use
its arrows or Left/Right to pick the item, and the chosen v83 selection index is sent when
turning in the quest. Quests marked with `startscript` or `endscript` use the corresponding
scripted v83 action and the existing server-driven NPC dialogue. A quest must be explicitly confirmed before forfeiting it. Server quest-clear packets
play the completion sound and NX light-pillar effect. Learned weapon mastery, Angel's Blessing, and Berserk contribute
their NX values to local attacks. `GIVE_BUFF` and `CANCEL_BUFF` use bounded v83 mask
decoding, retain timed statuses, and apply Maple Warrior, Weapon Attack, Accuracy, and
Hyper Body HP/MP effects to local combat and HUD values. Player ailments now show in the
HUD; stun blocks movement and attacks, confusion reverses horizontal input, slow reduces
movement acceleration, seal blocks skill use, and Darkness halves local attack accuracy.
Poison HP changes remain server-driven.
Other active-buff effects and combat mitigation, special projectile variants and skill
damage formula cases, character-creation
polish, birthday verification for character deletion, and
most other game UI remain under development. Mob movement now uses server-reported attack
and skill actions to play one-shot `attack1`–`attack9` and `skill1`–`skill9` animations when
those frames exist in the mob assets. When the server grants a controlled mob a skill, the
client sends its matching indexed action. Mob skill-effect visuals and expressions remain
open.
Music, UI, movement, portal, attack, mob, and skill-use sounds now play through the native
audio mixer with world-distance attenuation and a 64 MiB FIFO cache; a browser audio backend
remains future work.

Initial character entry now parses the legacy meso amount, inventory slot capacities,
equipped items, equipment bag, and stack items. `MODIFY_INVENTORY` add, count, remove,
and swap packets update that snapshot, including equip/unequip moves. Press `I` in the
field to open a letterbox-scaled inventory with six category tabs, a scrollable four-column
slot grid, cached NX icons, stack counts, meso, and slot capacity. Ordinary Use items send
the legacy `USE_ITEM` request and play their one-shot `Effect.nx/ItemEff.img` animation when
one exists; item consumption and effects on character stats remain server-authoritative.
Pet food sends the v83 `PET_FOOD` request so the server feeds the hungriest summoned pet.
Activating a Cash pet sends the v83 `SPAWN_PET` toggle request. Pet spawn/removal and
movement packets update map pets using their NX stance art and name labels. Local pets follow
with foothold physics and use the legacy pet AI cadence to send `PET_LOOT` requests for nearby
drops. Server pet-chat packets display timed speech bubbles above the correct pet, while pet
command packets play the NX success/failure interaction art, legacy alert/jump fallback, and
matching pet sound category.
Clicking an Equip item sends `MOVE_ITEM` to its legacy equipment slot, and clicking a worn
item sends the unequip request to the first free Equip slot. Dragging between slots in the
same bag sends a server-authoritative `MOVE_ITEM`;
inventory updates confirm changes and rebuild the local avatar from NX art. Equipment
scrolls can be selected from Use and applied to a chosen equipped slot. Equip actions check
gender, job class, level, fame, and total stat requirements from `Character.nx` and the
currently equipped gear before sending the move. Dragging an item outside the inventory
drops it; stack drops open a quantity prompt, cash items are blocked, and untradable items
require confirmation. Hovering an occupied slot shows its String.nx name, description,
required level, and NX equipment bonuses. Gather merges compatible stacks and closes gaps;
Sort applies the v83 server ordering for the active bag. Supported cash megaphones, Maple TV,
avatar megaphones, and pet name tags open the chat composer and send their v83
`USE_CASH_ITEM` payloads. Item megaphones currently omit the optional attached item; Maple TV
leaves the recipient name empty, and Maple TV and avatar megaphone accept one line and pad the
remaining rows. A 514 store permit prompts for a shop name and sends the legacy personal-shop
creation packet. A 523 Owl of Minerva opens a search panel with the server's popular items,
item-name or ID search, paged shop listings, and a warp action. A 503 hired-merchant permit
requests the server's deployment check, then prompts for a name and sends the merchant-room
creation packet when approved. Hired merchants spawn from server map packets with their NX
employee animation and sign, can be clicked to request a room visit, and update their
descriptions and visitor counts from status packets. The room screen supports visitor purchases
and chat, plus owner stocking from inventory, withdrawals, organization, meso collection,
starting the store, maintenance exit, and store closure. Stocking posts one bundle containing
the selected stack. Server-opened personal-shop rooms use the same resizable catalog and chat
panel, with visitor purchases, owner stocking and withdrawals, sale notifications, and store
start/exit actions. The client reads mini-room metadata from character spawn and update
packets; clicking a visible personal-shop owner opens that shop. Other targeted cash actions
and consumable-specific visuals remain open work.
The v83 move request does not support partial
stack moves within a bag, while drop and storage quantity prompts remain available. The
legacy client rejects drags between inventory categories, so slot moves stay within the
selected bag.
Press `P` to open Pet Equipment, or use the Pets button in the inventory. It shows pet
names, level and fullness alongside pet gear in the Equip inventory. Use a gear item while
the intended pet is summoned; the server inventory update remains authoritative.
Press `C` in the field to open Character Stats. Spend AP with the `+` buttons or use Auto
to confirm allocation to the character's primary stat; each point is sent to the server and
the panel waits for the authoritative stat update. Press the grave accent key to enter the
rebuilt Cash Shop. Its resizable catalog reads only
on-sale products with a valid commodity SN from `Etc.nx/Commodity.img`; it supports search,
category filters, item details and icon preview, currency balances, purchase confirmation,
and the server-backed locker and withdrawal flow.

To inspect a decoded field without connecting to a server, run
`OpenStoryRS --data-dir /path/to/wz --inspect-map 100000000`. The renderer culls
off-canvas sprites, supports atlas UV regions with per-sprite tint and opacity, and
batches adjacent draws that share a texture. Runtime map-art pixels are cached up to
128 MiB and 16,384 entries; decoded NX bitmap pixels are shared through a separate
16 MiB/4,096-entry FIFO cache per archive. GPU textures are reused for the active scene. To inspect the
stand, walk, and jump frames for a base appearance, run
`OpenStoryRS --data-dir /path/to/wz --inspect-avatar 0 20000 30000`.
