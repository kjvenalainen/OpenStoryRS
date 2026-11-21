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
Live character and server data is rendered over the NX artwork. NPC shops use the
v83 NX shop frame, item lists, category tabs, quantity dialog, and transaction controls.
Their Recommended tab filters to equipment the current character can equip; item and
meso balances change only when confirmed by server updates.

The native client opens v83 login and world/channel artwork in a resizable GPU-backed
window, keeps a 1024x768 logical canvas, and scales with letterboxing. Enlarged frames
use sharper cubic reconstruction to keep bitmap text and sprite edges clearer. The
account and password fields accept text; the password is masked. The selected world and channel
can be used to request a character list. Characters display name, level, job, and map.
In the field, the camera keeps the player above the status bar and uses the visible
field area for map limits. When no other window is open, Escape opens the game menu;
select Quit Game to open the quit confirmation.
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
When a character slot is free, Create Character opens Explorer customization directly;
Explorer is the only class family supported by this client and server. The customization
page uses the Maple Island login scenery from `Map.nx` and the character boards and
controls from `UI.nx`. It reads gender-specific face, hair, color, skin, clothing, shoe,
and weapon pools from `Etc.nx`, previews the selected avatar on the platform, and checks
forbidden names locally. The page opens with appearance choices on the left and the NX
name board on the right. The Check button requests name
availability, then the hanging OK button sends the v83 creation request. Stats and dice
are omitted because character stats are generated automatically. The server's new
character entry is added to the current selection list.

Set `OPENSTORY_FONT_PATH` to select a TrueType font and optionally
`OPENSTORY_BOLD_FONT_PATH` for its bold face. The UI prefers system Arial and
falls back to bundled Roboto when Arial is unavailable. Proprietary fonts are
not bundled. The client connects to
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
shows a compact equipped-avatar preview beside each character row. Use Left/Right
to walk; the bound Jump action (Option by default) jumps, and NPC interaction (Space by
default) talks to the nearest nearby NPC. Movement
acceleration and jump force use equipped and buffed SPEED/JUMP with the legacy stat caps;
hold Up to activate map portals. The avatar uses NX-authored one-handed or two-handed
stand/walk frames and jump frames, including the legacy body arm and hand z layers.
Press F1–F7 to play and broadcast the matching facial expression; NX-authored face
frames animate locally and for other visible players.
Press Backslash to open the key bindings panel. The client loads the v83 server key map,
uses bound skills, items, and gameplay actions, and lets you rebind 44 supported built-in
menu, gameplay, and face actions. It preserves unsupported saved actions so they remain
visible and can be cleared. The Family shortcut uses the v83 action 26. The panel also
lists learned active skills and usable inventory items by their localized names; stale
skill/item bindings stay visible so they can be cleared.
Click a bound key or an action in the two-row tray, then click or press the replacement
key. You can also drag an action onto a key, or drag a bound action into the tray to
unbind it; Backspace clears the selected binding. The mouse wheel and Up/Down scroll
additional tray rows; 1–8 select the first eight visible unbound actions.
**OK** (or Enter) applies the draft using the existing v83 key-map request;
**Cancel**, the close button, and Escape discard it. **Default** restores the
classic layout and quick-slot keys, and **Clear All** clears the binding draft. Click
**Change Quick Slots** or press F12 inside this panel to open the original quick-slot editor;
select a slot and press its new key, or click a visible key outside the editor. Reusing a key
swaps its previous slot. Click **Change Quick Slots** again or press Escape to return to the keyboard. Escape remains
reserved for closing dialogs. Equipment opens the worn-equipment window, Messenger opens
the Messenger panel. The displayed default layout also drives gameplay until the server
provides a key map; cleared or rebound keys never fall back to hard-coded shortcuts.
The default Z key picks up nearby items. The default U and V keys open Alliance and Guild BBS. Pet equipment and Events are also
assignable from the tray. The client extensions use menu action IDs 43–45 for Guild BBS,
Alliance, and pet equipment; existing v83 action IDs are unchanged. Quick-slot edits are saved with OK and use
the legacy `QuickSlotKeys` setting and override the server's login-time layout. Skill macro
editor opens from the Skills window. It edits the three server-backed macros, their shout
setting, and ordered learned-skill slots; a bound macro key shouts its name when enabled and
queues its skills through the normal skill-use path. The Quick Slots action opens the
eight-slot bar above the HUD; it shows bound skill, item, cash, macro, and action icons with
the configured key labels. Bound stackable items show their total quantity across matching
inventory stacks. Click a filled slot to run its binding, or drag a Use, Setup, or
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
Visible portal entry covers the authored glow horizontally while retaining the legacy
vertical bounds, so standing on another platform does not activate it.
Click an NPC to request dialogue, including NPCs whose scripts are supplied by the server.
NPC clicks and the bound NPC Chat key also open available quest conversations.
Unscripted tutorial quests read their pages from `Quest.nx/Say.img` and send the v83
quest action only after confirmation; scripted quests keep their server-authored dialogue.
The NPC menu lists every available or active quest alongside Talk, so normal server
scripts and services remain reachable. Local quizzes use the NX answer and feedback
branches; Rain's quizzes and Robin's multi-page quiz must be answered before turn-in.
Pio's recycling returns to its starting NPC when the end NPC is omitted. Lucas's
training turn-in checks all prerequisite quests. NPCs without a server response
open a local dialogue saying `Nothing to say...` after 1.5 seconds. Shops and travel
stay server-authoritative.
The dialogue panel handles selection lists, yes/no and accept/decline prompts, text and
number input, and page navigation. Its frame and standard controls use the NX
`UIWindow.img/UtilDlgEx` artwork. Long dialogue pages scroll inside their panel with the
mouse wheel or Page Up/Down.
Press Space or click to reveal a line as it types; press Space again to advance
or accept the current choice. On text-entry prompts, Space reveals the prompt first and then
remains available for spaces in the answer; Enter submits it. Up/Down moves the selection
highlight through long lists, which also scroll with the mouse wheel. Space submits number
prompts. Outside dialogue, Space requests a conversation with the nearest NPC within
legacy interaction range; both Option keys jump. NPC shops show server-provided buy and sell
lists, with a Recommended view that filters equipment by the character's level, job, gender,
stats, and fame requirements. Inventory category tabs, scrolling, and hover details work on
their respective lists. Double-click a row to open its buy or sell prompt, or select an item
and use the matching button. Quantity
prompts, recharge, and leave actions send v83 shop requests; item and currency changes remain
server-authoritative, and transaction replies unlock the controls and show failures. The
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
server-authoritative. Movable controlled mobs pursue the player who lands a positive hit,
then stop nearby while preserving their normal hit reaction. In a field, press Enter to open
the public chat composer, type a message, and press Enter to send. After sending, gameplay
keys control the character again while the chat panel may remain open; press Enter to type
another message. Escape closes the panel. Server chat broadcasts appear in a scrolling HUD
panel, with GM messages highlighted. Server popup
notices and Fake GM notices use the v83 login notice panel and can be dismissed with Enter,
Escape, or its OK button.
Visible speakers show four-second
speech balloons drawn
from the v83 `ChatBalloon.img` frame assets. While the composer is open, Tab cycles public,
buddy, guild, alliance, and party chat. `/w <character> <message>` sends a whisper,
`/find <character>` requests the player's channel, and `/pt <message>` sends directly to
party chat. `/party create`, `/party invite <character>`, `/party join <id>`, `/party accept`,
`/party deny`, `/party leave`, `/party expel <character>`, and `/party leader <character>`
manage party membership; `/party list` toggles the party panel. Press O to show party
members, channels, and server-reported HP.

Select System Menu in the game menu to open Options. The panel exposes graphics and
effects quality, music and sound volume with mute controls, mouse speed, HP/MP warning
thresholds, screen shake, and the v83 social permissions for whispers, messenger/party/guild/
alliance/family invitations, buddy/guild/alliance chat, trade and expedition requests, and
follow requests. Drag a slider or use Tab and the arrow keys. Space toggles the selected
checkbox. Click OK or press Enter to save. Cancel or Escape restores the saved values. Incoming
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
Weapon attacks also render the NX afterimage on its authored attack frames.
Normal close attacks use the equipped weapon's NX `Afterimage` range for the selected
stance, matching the legacy client; ranged attacks use the standard projectile range.
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
area effects. NPC storage uses a compact five-column All view with native item icons,
store/take actions, and meso transfers. All stored item categories share its
selection and scrolling order. Clicking a stack opens a quantity prompt for partial
deposits and withdrawals; withdrawals retrieve the v83 stack and return the unselected
remainder to storage after the inventory update. Inventory and storage contents remain
server-authoritative. Press K to open a class-tabbed skill book built from `Skill.nx` and `String.nx`.
Hover a skill to see its description, current-level costs and effects, and next-level
effects when below its mastery cap. Tooltips follow the pointer and stay inside the logical canvas.
Learned levels update from server packets, skill points can be spent, active skills can be
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
ranged attacks continue to use the equipped ammo art. Learned Eye of Amazon and Keen Eyes
extend matching bow, crossbow, and claw attack ranges from their NX level values. Close skills
that use the regular weapon action and lack a custom hit rectangle use the equipped weapon's
afterimage range. Hit effects and hit sounds follow each attack delay and projectile flight;
server mob-death animations and sounds apply immediately when `KILL_MOB` arrives. Attack skills
play their authored Use sound before the attack action; when present, it takes precedence over
the regular-action weapon sound. Skills without an authored Use sound retain the weapon sound
when they use the regular weapon action. Teleport uses its learned NX range and stops at
foothold walls or large floor changes; Flash Jump applies its aerial movement locally.
Using a skill or mastery book now parses the v83 result and shows a success or failure
message; learned skill levels still come from the server's skill update.
Press J to open the available/active/completed quest log; its state
comes from the initial character packet and server quest updates, while names, eligibility,
and descriptions come from `Quest.nx`. Talk to the quest NPC to start an available quest.
Ready active quests can be turned in through v83 quest actions. Objective rows show held item counts and
server-reported monster progress, and nearby NPCs show available, active, or ready markers.
The field Quest Helper shows up to five active quests with their current item and monster counts.
Click **Quest Helper** in the quest log footer to reopen it.
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
movement acceleration, and seal blocks skill use. Darkness does not reduce hit chance in the
upstream attack formula.
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
field to open a letterbox-scaled inventory with five category tabs, a scrollable four-column
slot grid, cached NX icons, stack counts, meso, and slot capacity. Dropping a stack opens a
quantity selector with a slider and numeric field. Click the coin icon to use the same
selector for mesos. The slider follows the current stack count or meso balance.
Each meso drop is limited to 10–50,000 mesos.
Ordinary Use items send
the legacy `USE_ITEM` request and play their one-shot `Effect.nx/ItemEff.img` animation when
one exists; item consumption and effects on character stats remain server-authoritative.
Pet food sends the v83 `PET_FOOD` request so the server feeds the hungriest summoned pet.
Key-bound Use, Setup, and Cash items share the inventory activation path, including
pet-food handling, chair sit/stand, and special cash actions. Activating a Cash pet sends
the v83 `SPAWN_PET` toggle request. Pet spawn/removal and movement packets update map pets
using their NX stance art and distinct name labels; a pet label is hidden when it repeats
its owner's character name, and server pet-name changes refresh the visible label.
Local pets follow with foothold physics, report movement to the server, and send `PET_LOOT`
requests for nearby settled drops. Pets choose separate targets and briefly defer retries after
a pickup request; the server remains authoritative over pickup eligibility. Server pet-chat
packets display timed speech bubbles above the correct pet, while pet
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
Assign the Pet equipment action in Keyboard Settings, or click **PET EQUIP** at the bottom of
the equipment inventory to open its right-side drawer. It shows pet names, level and fullness alongside
pet gear. Click **Equip** or **Remove** on a gear row; the server inventory update remains
authoritative.
Use the Stats binding (`S` in the default layout) in the field to open Character Stats. Spend AP with the `+` buttons or use Auto
to confirm allocation to the character's primary stat; each point is sent to the server and
the panel waits for the authoritative stat update. Server `FORCED_STAT_SET` updates also
apply to combat and movement totals, appear in the stats panel, and are cleared by the
server's `RECALCULATE_STATS` packet. Use the Cash Shop binding or the HUD Cash Shop button to enter the
rebuilt Cash Shop. Its resizable catalog reads only
on-sale products with a valid commodity SN from `Etc.nx/Commodity.img`; it supports search,
category filters, item icons, and avatar try-on previews for equipment, plus currency balance
refresh, purchase confirmation, and the server-backed cash inventory, locker, and withdrawal
flow. Click an item in either the cash inventory strip or locker list to move it into the
character inventory. Charge feedback confirms when NX increases and explains when the server
does not grant it. Single-item and package purchase results appear in the cash inventory. Server
responses show reason-specific notices for purchase failures and stock availability. The
catalog's coupon field sends the legacy v83 coupon request and reports invalid, expired, used,
and wrong-server responses. Resizing the window recomputes the logical pointer position so clicks
stay aligned with the 1024×768 canvas.

To inspect a decoded field without connecting to a server, run
`OpenStoryRS --data-dir /path/to/wz --inspect-map 100000000`. The renderer culls
off-canvas sprites, supports atlas UV regions with per-sprite tint and opacity, and
batches adjacent draws that share a texture. Runtime map-art pixels use a least-recently-used
cache bounded to 128 MiB and 16,384 entries; decoded NX string bytes and bitmap pixels
use separate bounded least-recently-used caches per archive. GPU textures are reused for
the active scene. To inspect the stand, walk, and jump frames for a base appearance, run
`OpenStoryRS --data-dir /path/to/wz --inspect-avatar 0 20000 30000`.

### Offline UI rebuild previews

Run `cargo run -p openstory-client -- --ui-preview all --ui-hitboxes` to render
canonical reference cases and retained secondary screens under
`target/ui-previews`. Use `--help` for the current case names; the previous
`native-*` names and friendly aliases have been removed. Reference cases produce
native-size crops, comparisons and differences in addition to full logical and
resized compositions. The live client does not use the reference screenshots.
See [UI_REBUILD.md](UI_REBUILD.md) for implemented changes and outstanding work;
the complete UI rebuild is still in progress.
