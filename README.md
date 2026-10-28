# OpenStoryRS

This repository is the Rust port of the OpenStory v83 MapleStory game client. The
implementation roadmap is in [PORT_PLAN.md](PORT_PLAN.md).

The client reads the existing NX assets in place. `UI.nx` is the version 83 UI archive
and the client resolves UI asset paths from it. Keep the files in the existing
`OpenStory/wz` directory; point
the client at them with
`OPENSTORY_DATA_DIR=/path/to/wz` or `--data-dir /path/to/wz`.

Core in-game panels use their v83 frames and controls from `UI.nx`, including
stats, inventory and equipment, skills, quests, chat, map windows, and options.
Live character and server data is rendered over the NX artwork.

The native client opens v83 login and world/channel artwork in a resizable GPU-backed
window, keeps a 1024x768 logical canvas, and scales with letterboxing. The account and
password fields accept text; the password is masked. The selected world and channel
can be used to request a character list. Characters display name, level, job, and map.
In the field, the camera keeps the player above the status bar and uses the visible
field area for map limits. Escape opens the quit confirmation when no panel is handling it.
The login background's six light beams animate independently using their NX frame delays;
login buttons use their NX disabled, mouse-over, and pressed art. Keyboard focus uses NX
focus art when available and a gold outline otherwise. Tab cycles through the account field,
password field, and enabled actions; arrows move among actions, and Enter or Space activates
the focused action. Login and Guest are enabled while the session connects; clicking either
queues the request until the Maple handshake and login-start packet complete. Login also
requires an account of 1–12 characters and a password of 5–12 characters.
If the server reports that its Terms of Service have not been accepted, the client
asks the player to review them; clicking OK or pressing Enter sends acceptance, while
Escape cancels.
The selection screen follows the legacy three-slot layout and pages through all characters returned by the server.
Click a character's avatar or the Start sign to enter; Enter also selects the highlighted character. Character
selection supports PIC registration, verification, and the disabled-PIC path;
PIC entry is masked and accepts 6–16 letters or numbers from the keyboard. Each row
also has a Delete action with confirmation and optional PIC verification. Set
`OPENSTORY_MACS` and `OPENSTORY_HWID` to override the legacy selection hardware strings.
On macOS, when either override is absent, the client supplies the v83-compatible MAC/HWID
format from the primary network interface and the platform UUID, matching the legacy
client's character-selection packets. These values are sent only as part of character
selection and PIC requests to the configured Maple server.
When a character slot is free, Create Character opens Explorer, Cygnus Knight, and Aran
class choices. The customization screen reads gender-specific face, hair, color, skin,
clothing, shoe, and weapon pools from `Etc.nx`, previews the selected avatar, checks
forbidden names locally, and uses the v83 name-check/create packets. The server's new
character entry is added to the current selection list.

Set `OPENSTORY_FONT_PATH` to select a TrueType font; macOS, Windows, and Linux
system-font locations are used as fallbacks. The client connects to
`127.0.0.1:8484` by default so the packaged app can reach a local v83 server without
command-line arguments. Set `OPENSTORY_SERVER=HOST:PORT` to choose another default, or
use `--connect HOST:PORT` to override it. The client completes the Maple handshake and
sends the login-start packet before any queued login request. The login button submits the
fields. Set `OPENSTORY_ACCOUNT` and `OPENSTORY_PASSWORD` to prefill them, or set
`OPENSTORY_AUTO_LOGIN=1` to submit them
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
The v83 `SET_TRACTION` update also adjusts player friction on slippery fields. The
v83 `SHOW_COMBO` packet updates the Aran combo counter overlay, including its
three-second display after a reset.
The first `SET_FIELD` transition
uses the selected character's map and portal from the character list. The map scene shows
the selected character's body, head, face, hair, and equipped items. Character selection
shows a compact equipped-avatar preview beside each character row. Use Left/Right or A/D
to walk; left or right Option jumps; Space talks to the nearest nearby NPC. Movement
acceleration and jump force use equipped and buffed SPEED/JUMP with the legacy stat caps;
hold Up or W to activate map portals. The avatar uses NX-authored one-handed or two-handed
stand/walk frames and jump frames, including the legacy body arm and hand z layers.
Press F1–F7 to play and broadcast the matching facial expression; NX-authored face
frames animate locally and for other visible players.
Press Backslash to open the key bindings panel. The client loads the v83 server key map,
uses bound skills, items, and gameplay actions, and lets you rebind 40 supported built-in
menu, gameplay, and face actions. It preserves unsupported saved actions so they remain
visible and can be cleared. The Family shortcut uses the v83 action 26. The panel also
lists learned active skills and usable inventory items by their localized names; stale
skill/item bindings stay visible so they can be cleared.
Click an action or press 1–8 to choose it, then press the replacement key; Backspace clears
its binding and the mouse wheel scrolls the list. The panel shows the eight quick-slot keys
supplied by the server; click a slot and press a key to rebind it. Local quick-slot edits use
the legacy `QuickSlotKeys` setting and override the server's login-time layout. Skill macro
editor opens from the Skills window. It edits the three server-backed macros, their shout
setting, and ordered learned-skill slots; a bound macro key shouts its name when enabled and
queues its skills through the normal skill-use path. The Quick Slots action opens the
eight-slot bar above the HUD; it shows bound skill, item, cash, macro, and action icons with
the configured key labels. Click a filled slot to run its binding, or drag a Use, Setup, or
Cash inventory item onto a slot to update that key on the server. The minimap
loads its map bitmap and portals from `Map.nx`, shows the player,
NPCs, mobs, and other characters, and cycles through collapsed, normal, and expanded views.
Town maps open expanded, and the max-size button is disabled at its limit. It can also open
the World Map. Messenger handles
server invitations, three v83 room slots, chat, invite/decline, and dragging the window.
The server-requested World Map opens its NX-backed regional panel, shows the current-map
marker, and lets you navigate regional links. The search field filters regional markers
by map name, description, or ID as you type. It also searches the localized map catalog
across regions, ranks exact and prefix name matches first, and shows a scrollable list of
results. Selecting a result opens its regional map and highlights the matching marker;
this is map browsing, not character travel. Hovered markers show their map details and
optional NX route image. Automatic travel remains open work.
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
`FIELD_EFFECT` packets play the legacy map and screen animation paths from `Map.nx`, switch
background music, or resolve field sounds through the `Sound.nx` category fallbacks.
Server-driven Maple Guide appearances and messages now use the v83 `tutorial.img` text
when present in the active UI archive, with the legacy hint-ID fallback when that archive
contains only tutorial artwork.
`LOCK_UI` and `DISABLE_UI` packets pause mouse-driven UI actions until the server unlocks them.
Supported `SHOW_ITEM_GAIN_INCHAT` item-gain notifications now display the localized item
name and signed quantity in blue chat, skill/buff echoes show their NX effect without
duplicating a just-rendered local cast, and MonsterBook card notifications play the local
card effect.
The Monster Book window loads its cover and collected cards from the initial character state,
then applies server card updates, with category filters, card details,
name search, and a set-effect page that shows collected-card progress and set bonuses from
the legacy client data. Monster details include NX-backed drop icons and stats; scroll over
the drop grid to browse longer reward lists. Click the monster sprite to cycle through stand,
move, and die animations; the client follows NX frame delays, opacity, and scale, and the die
animation returns to stand when it finishes.
`DROP_LOOT` and `REMOVE_LOOT` update map drops with item or meso icons from the existing
NX archives. Drops animate while falling, float after landing, and fade or move to a looter
when collected; meso icons animate from their NX frame data. Press Z near a drop to send
`PICKUP_ITEM`. Reactor spawn, hit, and removal packets load art from `Reactor.nx`, place
reactors on map footholds, and play their state-specific hit animations. Basic attacks
send server-authoritative `DAMAGE_REACTOR` requests when a hittable reactor intersects the
attack arc. Regular portals animate from `MapHelper.img`; hidden portals reveal their
animation when approached. Hold Up or W in a portal to request a server map change;
same-map portals move the player to their named destination after the legacy cooldown.
Click an NPC to request dialogue, including NPCs whose scripts are supplied by the server.
When an NPC has an available scripted quest but no regular NPC script, clicking it starts
that quest's server-authored dialogue.
The dialogue panel handles selection lists, yes/no and accept/decline prompts, text and
number input, and page navigation. Its frame and standard controls use the NX
`UIWindow.img/UtilDlgEx` artwork. Long dialogue pages scroll inside their panel with the
mouse wheel or Page Up/Down.
Press Space or click to reveal a line as it types; press Space again to advance
or accept the current choice. On text-entry prompts, Space reveals the prompt first and then
remains available for spaces in the answer; Enter submits it. Up/Down moves the selection
highlight through long lists, which also scroll with the mouse wheel. Space submits number
prompts. Outside dialogue, Space requests a conversation with the nearest NPC within
legacy interaction range; both Option keys jump. When the server opens an NPC shop, its
Buy and Sell views use NX item names and
icons; sell rows show the legacy NX unit price and exclude items marked `notSale` or
cash items using the correct item archive. Quantity controls send v83 buy/sell requests, rechargeable
stock sends the recharge request, and inventory updates remain server-authoritative. The
bottom HUD uses the stock v83 `StatusBar.img` background, gauge, and toolbar sprites, with
live HP, MP, and EXP fills plus character level, name, job, and resource values. HP updates
display a two-second bar, and positive HP deltas float the legacy blue healing digits from
`Effect.nx` above the character. The native toolbar opens Status, Use/Equip inventory,
Skills, key settings, quick slots, Cash Shop, Options, and a menu for quests, social panels,
the Monster Book, and World Map. Its Trade button opens the `/trade` composer. Stun, freeze,
and shadow-web hold mobs in their stand animation until the server
cancels those statuses. `DAMAGE_MONSTER` packets float the legacy digits
from `Effect.nx` above the mob with a 1.5-second rise and fade. Controlled mobs use the
shared foothold physics and v83 `MOVE_MONSTER` updates, while remote movement remains
server-authoritative. In a field, press Enter to open public chat, type a message, and
press Enter to send; Escape closes the composer. Server chat broadcasts appear in a
scrolling HUD panel, with GM messages highlighted. Server popup notices and Fake GM notices
use the v83 login notice panel and can be dismissed with Enter, Escape, or its OK button.
Visible speakers show four-second
speech balloons drawn
from the v83 `ChatBalloon.img` frame assets. While the composer is open, Tab cycles public,
buddy, guild, alliance, and party chat. `/w <character> <message>` sends a whisper,
`/find <character>` requests the player's channel, and `/pt <message>` sends directly to
party chat. `/party create`, `/party invite <character>`, `/party join <id>`, `/party accept`,
`/party deny`, `/party leave`, `/party expel <character>`, and `/party leader <character>`
manage party membership; `/party list` toggles the party panel. Press O to show party
members, channels, and server-reported HP.

When no dialog or panel is open, press Escape to open Options. The panel exposes graphics
and effects quality, music and sound volume with mute controls, mouse speed, HP/MP warning
thresholds, screen shake, windowed mode, 800×600/1024×768 resolution, and the v83 social
permissions for whispers, messenger/party/guild/alliance/family invitations, buddy/guild/
alliance chat, trade and expedition requests, and follow requests. Drag a slider or use Tab
and the arrow keys; Space toggles the selected checkbox. Save or Enter persists the settings
and applies display changes, while Cancel or Escape restores the saved values. Incoming
whispers, group chat, and the implemented party, guild, alliance, messenger, family, and trade
request flows honor their corresponding setting. On Windows, mouse speed uses the legacy
system pointer-speed mapping; other platforms follow the C++ no-op behavior. On macOS,
settings are stored in
`~/Library/Application Support/OpenStoryRS/settings.ini`. The file also stores the Save
Login ID choice and account name. Saving options preserves these values and removes any
legacy saved-password entry. Missing social permission settings default to allow, matching
the v83 client. Expedition and follow settings are saved for compatibility, though those
request flows are not implemented yet. Mute choices are session-only, matching the C++ client.
Missing or invalid music and sound effect volumes default to 50 percent.
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
The v83 under-level meso-trade restriction and Vicious Hammer/Vega Scroll result packets also
show their legacy notices; report-result packets open a dismissible OK notice.
Click a visible character to request their v83 character-info panel; Escape or its close button
dismisses the panel. Its Party, Trade, and Guild buttons send the corresponding invites. The
Raise Fame and Lower Fame buttons send v83 fame requests, enforce the level-15 requirement,
show server rejection reasons, and update the open panel when the server confirms a change.
Clicking a visible personal-shop owner opens their shop.
Press G to open the scrollable guild roster and accept or decline pending guild invites
with Y or N. When not in a guild, click **Create Guild**, enter a 3–12 letter name, and
submit it to start the server's nearby-cofounder confirmation flow. Guild members can open
the **Ranks** editor to change all five rank titles, or the **Emblem** editor to choose
valid backgrounds, colors, and marks from `UI.nx/GuildMark.img` with a live asset preview.
The Guild and Alliance windows also use the v83 `UIWindow.img/UserList/Guild` and
`GuildUnion` panel art from `UI.nx`.
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
alliance-member updates are consumed from the server. Assign **Family** in the key bindings
panel to open the server-backed family window and pedigree list. The panel shows reputation,
motto and junior capacity; click the motto to edit it, invite a junior by name, use the selected
family ability, or leave with confirmation. Click a family-tree row to view that member; Y/N
answers incoming family-join and summon prompts. Press Left Control to
send a one-hit regular attack; guild/family level-up and job-advancement broadcasts appear
in yellow chat notices, spouse/fiance messages appear in red, and v83 ship departure,
arrival, and transit status messages appear in yellow. The client
uses the equipped weapon data, plays its NX stance, and targets the nearest mob ahead.
Bow, crossbow, claw, and gun attacks select a compatible projectile stack, add any NX
attack bonus it provides, use the v83 ranged packet and range, and animate the projectile.
Remote ranged attacks also animate their server-reported projectile; the server owns ammo
consumption. Incoming attack broadcasts animate remote characters with weapon-specific NX
stances and show normal, critical, or miss feedback above mobs. `DAMAGE_PLAYER` packets
show bounded damage/miss indicators and server-directed recoil above remote characters;
`LEFT_KNOCK_BACK` applies the v83 leftward movement impulse to the local player.
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
before sending a request; v83 `ADD_COOLDOWN` updates block casts until the server-provided
cooldown expires. Inventory consumption remains server-authoritative. Attack target
selection uses the skill level's NX hit rectangle and
horizontal range multiplier for both mobs and reactors. Skill hit animations follow the NX
character-level, skill-level, and weapon-hand variants and attach to each mob in the attack
broadcast. Local casts also play the selected NX use effect and skill action, including
level and weapon-hand variants. Projectile skills load their root or per-level `ball` frames
from `Skill.nx` for local and remote attacks, and fly toward each reported target; ordinary
ranged attacks continue to use the equipped ammo art. Teleport uses its learned NX range and stops at
foothold walls or large floor changes; Flash Jump applies its aerial movement locally.
Using a skill or mastery book now parses the v83 result and shows a success or failure
message; learned skill levels still come from the server's skill update.
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
Hyper Body HP/MP effects to local combat and HUD values. Booster combines with equipped
weapon speed for local attacks and remote attack animations. Player ailments now show in the
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
audio mixer with world-distance attenuation and a 64 MiB least-recently-used cache; a browser audio backend
remains future work.

Initial character entry now parses the legacy meso amount, inventory slot capacities,
equipped items, equipment bag, and stack items. `MODIFY_INVENTORY` add, count, remove,
and swap packets update that snapshot, including equip/unequip moves. `INVENTORY_GROW`
updates the server-authoritative capacity and the inventory grid immediately;
`SET_EXTRA_PENDANT_SLOT` records the pendant-slot entitlement and reports activation changes.
Press `I` in the
field to open a letterbox-scaled inventory with six category tabs, a scrollable four-column
slot grid, cached NX icons, stack counts, meso, and slot capacity. Ordinary Use items send
the legacy `USE_ITEM` request and play their one-shot `Effect.nx/ItemEff.img` animation when
one exists; item consumption and effects on character stats remain server-authoritative.
Pet food sends the v83 `PET_FOOD` request so the server feeds the hungriest summoned pet.
Key-bound Use, Setup, and Cash items share the inventory activation path, including
pet-food handling, chair sit/stand, and special cash actions. Activating a Cash pet sends
the v83 `SPAWN_PET` toggle request. Pet spawn/removal and movement packets update map pets
using their NX stance art and name labels; server pet-name changes refresh the visible label.
Local pets follow
with foothold physics and use the legacy pet AI cadence to send `PET_LOOT` requests for nearby
drops. Server pet-chat packets display timed speech bubbles above the correct pet, while pet
command packets play the NX success/failure interaction art, legacy alert/jump fallback, and
matching pet sound category. `PET_EXCEPTION_LIST` packets consume the pet serial and excluded
item IDs and report the server's exclusion count in yellow chat.
Clicking an Equip item sends `MOVE_ITEM` to its legacy equipment slot, and clicking a worn
item sends the unequip request to the first free Equip slot. Dragging between slots in the
same bag sends a server-authoritative `MOVE_ITEM`;
inventory updates confirm changes and rebuild the local avatar from NX art. Equipment
scrolls can be selected from Use and applied to a chosen equipped slot. Equip actions check
gender, job class, level, fame, and total stat requirements from `Character.nx` and the
currently equipped gear before sending the move. The server's scroll-result packet plays the
NX enchant success/failure animation on the affected character and reports the result in local
chat. Dragging an item outside the inventory
drops it; stack drops open a quantity prompt, cash items are blocked, and untradable items
require confirmation. Hovering an occupied slot shows its String.nx name, description,
required level, and NX equipment bonuses. Gather merges compatible stacks and closes gaps;
Sort applies the v83 server ordering for the active bag. Supported cash megaphones, Maple TV,
avatar megaphones, and pet name tags open the chat composer and send their v83
`USE_CASH_ITEM` payloads. Item megaphones accept an optional inventory attachment: enter a
message, press Enter, click an inventory item, then press Enter to send. Click the selected item
again to clear it, or press Enter on a blank line to send without an attachment. Press I to reopen
the inventory while choosing an attachment. On megaphone types that support the whisper flag, Tab
toggles it. Maple TV items with the legacy target control prompt for an optional recipient name
(up to 12 characters).
The composer accepts up to three triple-megaphone lines,
five Maple TV lines, and four avatar-megaphone lines; an empty submission sends the lines
already entered. Incoming v83 avatar-megaphones show the sender look and message in matching
Map.nx banner art for ten seconds. A 514 store permit prompts for a shop name and sends the legacy personal-shop
creation packet. A 523 Owl of Minerva opens a search panel with the server's popular items,
item-name or ID search, paged shop listings, and a warp action. A 503 hired-merchant permit
requests the server's deployment check, then prompts for a name and sends the merchant-room
creation packet when approved. Hired merchants spawn from server map packets with their NX
employee animation and sign, can be clicked to request a room visit, and update their
descriptions and visitor counts from status packets. The room screen supports visitor purchases
and chat, plus owner stocking from inventory, withdrawals, organization, meso collection,
starting the store, maintenance exit, and store closure. Cash and `tradeBlock` items are
rejected before listing and checked again before submission. Stocking posts one bundle
containing the selected stack. Server-opened personal-shop rooms use the same resizable catalog and chat
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
the panel waits for the authoritative stat update. Server `FORCED_STAT_SET` updates also
apply to combat and movement totals, appear in the stats panel, and are cleared by the
server's `RECALCULATE_STATS` packet. Press the grave accent key to enter the
rebuilt Cash Shop. Its resizable catalog reads only
on-sale products with a valid commodity SN from `Etc.nx/Commodity.img`; it supports search,
category filters, item details and icon preview, currency balances, purchase confirmation,
and the server-backed locker and withdrawal flow. Server responses show reason-specific
notices for purchase failures and stock availability. The catalog's coupon field sends the
legacy v83 coupon request and reports invalid, expired, used, and wrong-server responses.
Resizing the window
recomputes the logical pointer position so clicks stay aligned with the 1024×768 canvas.

To inspect a decoded field without connecting to a server, run
`OpenStoryRS --data-dir /path/to/wz --inspect-map 100000000`. The renderer culls
off-canvas sprites, supports atlas UV regions with per-sprite tint and opacity, and
batches adjacent draws that share a texture. Runtime map-art pixels use a least-recently-used
cache bounded to 128 MiB and 16,384 entries; decoded NX string bytes and bitmap pixels
use separate bounded least-recently-used caches per archive. GPU textures are reused for
the active scene. To inspect the stand, walk, and jump frames for a base appearance, run
`OpenStoryRS --data-dir /path/to/wz --inspect-avatar 0 20000 30000`.
