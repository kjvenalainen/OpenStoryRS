# Backend parity inventory

Snapshot of the registered incoming behavior on 2026-10-28, compared with the current shared Rust tree. The adjacent C++ checkout is read-only. This inventory is about protocol, state, and simulation; UI presentation remains out of scope.

## Status key

- **IV — implemented and verified:** supported flow has a recorded server-backed or end-to-end verification result.
- **IU — implemented, unverified:** a Rust packet/state path exists, but no representative live-server result is recorded.
- **M — missing:** the original handler has non-no-op behavior and no reviewed Rust equivalent.
- **O — original no-op:** the original handler consumes/ignores the packet or deliberately leaves its visual/state effect to another authoritative packet.
- **SC — server-contract dependent:** a noncanonical/custom feature is only implementable when the supplied server contract supports it.

Incoming registration counts: **235 IU**, **3 O**, **3 SC**, **0 M**, and **0 IV** across the 241 registered opcodes. Local fixtures or deterministic tests establish parser/rule behavior, not server interoperability. No incoming flow is labeled IV because this checkout does not record a live-server or end-to-end result.

## Incoming registration inventory (all 241 registrations)

`PacketSwitch.cpp` declares 241 incoming values at [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L51)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L369) and registers one handler per value at [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L371)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L665). The registry has 241 distinct incoming numeric values and no duplicate numeric registration. Each row below is one registered handler. “Behavior/variant” summarizes the handler’s packet meaning/state effect; “Rust counterpart” gives the receive opcode definition and dispatcher location where one exists.

### Common

Common connection liveness packet.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `PING` (17, `0x011`) | answer server keepalive with PONG | [`OPCODE_PING`](crates/openstory-protocol/src/feedback.rs#L21) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L108) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L374)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L374).

### Login

Account, world, character selection, redirects, and PIC transitions.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `LOGIN_RESULT` (0, `0x000`) | accept/reject account login and start world-list transition | [`OPCODE_LOGIN_RESULT`](crates/openstory-protocol/src/login.rs#L11) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L982) | **IU** |
| `SERVERSTATUS` (3, `0x003`) | report login-server capacity/status | [`OPCODE_SERVERSTATUS_RESULT`](crates/openstory-protocol/src/login.rs#L20) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1245) | **IU** |
| `SERVERLIST` (10, `0x00a`) | replace world/channel list | [`OPCODE_SERVERLIST_RESULT`](crates/openstory-protocol/src/login.rs#L16) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1015) | **IU** |
| `CHARLIST` (11, `0x00b`) | replace selected-world character list | [`OPCODE_CHARLIST_RESULT`](crates/openstory-protocol/src/login.rs#L19) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1139) | **IU** |
| `SERVER_IP` (12, `0x00c`) | redirect and reconnect to selected channel | [`OPCODE_SERVER_IP`](crates/openstory-protocol/src/login.rs#L21) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1183) | **IU** |
| `CHARNAME_RESPONSE` (13, `0x00d`) | resolve character-name availability | [`OPCODE_CHARACTER_NAME_RESULT`](crates/openstory-protocol/src/login.rs#L39) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1177) | **IU** |
| `ADD_NEWCHAR_ENTRY` (14, `0x00e`) | append a created character | [`OPCODE_CREATE_CHARACTER_RESULT`](crates/openstory-protocol/src/login.rs#L40) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1180) | **IU** |
| `DELCHAR_RESPONSE` (15, `0x00f`) | apply delete/PIC result | [`OPCODE_DELETE_CHARACTER_RESULT`](crates/openstory-protocol/src/login.rs#L31) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L906) | **IU** |
| `RECOMMENDED_WORLDS` (27, `0x01b`) | replace recommended-world list | [`OPCODE_RECOMMENDED_WORLDS`](crates/openstory-protocol/src/login.rs#L18) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1055) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L377)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L385).

### Field entry

Initial field/MTS transitions.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `SET_FIELD` (125, `0x07d`) | enter a field and apply the initial character/map snapshot | [`OPCODE_SET_FIELD`](crates/openstory-protocol/src/login.rs#L35) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1405) | **IU** |
| `SET_ITC` (126, `0x07e`) | enter the MTS state after the Cash Shop transition | [`OPCODE_SET_MTS`](crates/openstory-protocol/src/mts.rs#L9) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L870) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L388)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L389).

### Map actors

Map actor spawn, update, movement, status, and removal transitions; the mnemonic gives the actor/action.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `SPAWN_CHAR` (160, `0x0a0`) | spawn a remote character and initialize its actor state; handler is `SpawnCharHandler` | [`OPCODE_SPAWN_CHARACTER`](crates/openstory-protocol/src/gameplay.rs#L645) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1491) | **IU** |
| `CHAR_MOVED` (185, `0x0b9`) | apply remote character movement; handler is `CharMovedHandler` | [`OPCODE_CHARACTER_MOVED`](crates/openstory-protocol/src/gameplay.rs#L649) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1674) | **IU** |
| `UPDATE_CHARLOOK` (197, `0x0c5`) | replace remote character appearance; handler is `UpdateCharLookHandler` | [`OPCODE_UPDATE_CHARACTER_LOOK`](crates/openstory-protocol/src/gameplay.rs#L659) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1867) | **IU** |
| `SHOW_FOREIGN_EFFECT` (198, `0x0c6`) | show foreign effect packet; handler is `ShowForeignEffectHandler` | [`OPCODE_SHOW_FOREIGN_EFFECT`](crates/openstory-protocol/src/gameplay.rs#L673) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1977) | **IU** |
| `REMOVE_CHAR` (161, `0x0a1`) | remove a remote character actor; handler is `RemoveCharHandler` | [`OPCODE_REMOVE_CHARACTER`](crates/openstory-protocol/src/gameplay.rs#L646) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1497) | **IU** |
| `SPAWN_PET` (168, `0x0a8`) | spawn pet packet; handler is `SpawnPetHandler` | [`OPCODE_SPAWN_PET_RECEIVED`](crates/openstory-protocol/src/gameplay.rs#L701) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1511) | **IU** |
| `MOVE_PET` (170, `0x0aa`) | move pet packet; handler is `MovePetHandler` | [`OPCODE_MOVE_PET`](crates/openstory-protocol/src/gameplay.rs#L702) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1522) | **IU** |
| `PET_CHAT` (171, `0x0ab`) | apply pet chat/emote response; handler is `PetChatHandler` | [`OPCODE_PET_CHAT_RECEIVED`](crates/openstory-protocol/src/gameplay.rs#L704) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1528) | **IU** |
| `PET_COMMAND` (174, `0x0ae`) | apply pet command response; handler is `PetCommandHandler` | [`OPCODE_PET_COMMAND_RECEIVED`](crates/openstory-protocol/src/gameplay.rs#L707) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1548) | **IU** |
| `CHALKBOARD` (164, `0x0a4`) | chalkboard packet; handler is `ChalkboardHandler` | [`OPCODE_CHALKBOARD`](crates/openstory-protocol/src/gameplay.rs#L648) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1913) | **IU** |
| `UPDATE_CHAR_BOX` (165, `0x0a5`) | update a character chalkboard box; handler is `UpdateCharBoxHandler` | [`OPCODE_UPDATE_CHARACTER_BOX`](crates/openstory-protocol/src/gameplay.rs#L661) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1905) | **IU** |
| `CANCEL_SKILL_EFFECT` (191, `0x0bf`) | cancel skill effect packet; handler is `CancelSkillEffectHandler` | [`OPCODE_CANCEL_SKILL_EFFECT`](crates/openstory-protocol/src/gameplay.rs#L677) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2071) | **IU** |
| `SUMMON_SKILL` (180, `0x0b4`) | summon skill packet; handler is `SummonSkillHandler` | [`OPCODE_SUMMON_SKILL`](crates/openstory-protocol/src/gameplay.rs#L710) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2480) | **IU** |
| `SPAWN_DRAGON` (181, `0x0b5`) | spawn dragon packet; handler is `SpawnDragonHandler` | [`OPCODE_SPAWN_DRAGON`](crates/openstory-protocol/src/gameplay.rs#L698) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2486) | **IU** |
| `MOVE_DRAGON` (182, `0x0b6`) | move dragon packet; handler is `MoveDragonHandler` | [`OPCODE_MOVE_DRAGON`](crates/openstory-protocol/src/gameplay.rs#L699) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2492) | **IU** |
| `REMOVE_DRAGON` (183, `0x0b7`) | remove dragon packet; handler is `RemoveDragonHandler` | [`OPCODE_REMOVE_DRAGON`](crates/openstory-protocol/src/gameplay.rs#L700) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2498) | **IU** |
| `SPAWN_NPC` (257, `0x101`) | spawn npc packet; handler is `SpawnNpcHandler` | [`OPCODE_SPAWN_NPC`](crates/openstory-protocol/src/gameplay.rs#L679) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2261) | **IU** |
| `REMOVE_NPC` (258, `0x102`) | remove npc packet; handler is `RemoveNpcHandler` | [`OPCODE_REMOVE_NPC`](crates/openstory-protocol/src/gameplay.rs#L680) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2291) | **IU** |
| `SPAWN_NPC_C` (259, `0x103`) | spawn npc controller packet; handler is `SpawnNpcControllerHandler` | [`OPCODE_SPAWN_NPC_CONTROLLER`](crates/openstory-protocol/src/gameplay.rs#L681) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2303) | **IU** |
| `SPAWN_MOB` (236, `0x0ec`) | spawn mob packet; handler is `SpawnMobHandler` | [`OPCODE_SPAWN_MOB`](crates/openstory-protocol/src/gameplay.rs#L686) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2336) | **IU** |
| `SPAWN_MOB_C` (238, `0x0ee`) | spawn mob controller packet; handler is `SpawnMobControllerHandler` | [`OPCODE_SPAWN_MOB_CONTROLLER`](crates/openstory-protocol/src/gameplay.rs#L688) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2366) | **IU** |
| `MOB_MOVED` (239, `0x0ef`) | mob moved packet; handler is `MobMovedHandler` | [`OPCODE_MOB_MOVED`](crates/openstory-protocol/src/gameplay.rs#L689) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2374) | **IU** |
| `SHOW_MOB_HP` (250, `0x0fa`) | show mob hp packet; handler is `ShowMobHpHandler` | [`OPCODE_SHOW_MOB_HP`](crates/openstory-protocol/src/gameplay.rs#L694) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2405) | **IU** |
| `MOVE_MONSTER_RESPONSE` (240, `0x0f0`) | move monster response packet; handler is `MoveMonsterResponseHandler` | [`OPCODE_MOVE_MOB_RESPONSE`](crates/openstory-protocol/src/gameplay.rs#L690) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2384) | **IU** |
| `DAMAGE_MONSTER` (246, `0x0f6`) | damage monster packet; handler is `DamageMonsterHandler` | [`OPCODE_DAMAGE_MOB`](crates/openstory-protocol/src/gameplay.rs#L693) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2419) | **IU** |
| `APPLY_MONSTER_STATUS` (242, `0x0f2`) | apply monster status packet; handler is `ApplyMonsterStatusHandler` | [`OPCODE_APPLY_MOB_STATUS`](crates/openstory-protocol/src/gameplay.rs#L691) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2440) | **IU** |
| `CANCEL_MONSTER_STATUS` (243, `0x0f3`) | cancel monster status packet; handler is `CancelMonsterStatusHandler` | [`OPCODE_CANCEL_MOB_STATUS`](crates/openstory-protocol/src/gameplay.rs#L692) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2450) | **IU** |
| `KILL_MOB` (237, `0x0ed`) | kill mob packet; handler is `KillMobHandler` | [`OPCODE_KILL_MOB`](crates/openstory-protocol/src/gameplay.rs#L687) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2342) | **IU** |
| `DROP_LOOT` (268, `0x10c`) | drop loot packet; handler is `DropLootHandler` | [`OPCODE_DROP_LOOT`](crates/openstory-protocol/src/gameplay.rs#L715) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2205) | **IU** |
| `REMOVE_LOOT` (269, `0x10d`) | remove loot packet; handler is `RemoveLootHandler` | [`OPCODE_REMOVE_LOOT`](crates/openstory-protocol/src/gameplay.rs#L716) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2211) | **IU** |
| `SPAWN_MIST` (273, `0x111`) | spawn mist packet; handler is `SpawnMistHandler` | [`OPCODE_SPAWN_MIST`](crates/openstory-protocol/src/gameplay.rs#L711) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2316) | **IU** |
| `REMOVE_MIST` (274, `0x112`) | remove mist packet; handler is `RemoveMistHandler` | [`OPCODE_REMOVE_MIST`](crates/openstory-protocol/src/gameplay.rs#L712) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2326) | **IU** |
| `SPAWN_DOOR` (275, `0x113`) | spawn door packet; handler is `SpawnDoorHandler` | [`OPCODE_SPAWN_DOOR`](crates/openstory-protocol/src/gameplay.rs#L684) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2243) | **IU** |
| `REMOVE_DOOR` (276, `0x114`) | remove door packet; handler is `RemoveDoorHandler` | [`OPCODE_REMOVE_DOOR`](crates/openstory-protocol/src/gameplay.rs#L685) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2249) | **IU** |
| `HIT_REACTOR` (277, `0x115`) | hit reactor packet; handler is `HitReactorHandler` | [`OPCODE_HIT_REACTOR`](crates/openstory-protocol/src/gameplay.rs#L717) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2223) | **IU** |
| `SPAWN_REACTOR` (279, `0x117`) | spawn reactor packet; handler is `SpawnReactorHandler` | [`OPCODE_SPAWN_REACTOR`](crates/openstory-protocol/src/gameplay.rs#L718) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2217) | **IU** |
| `REMOVE_REACTOR` (280, `0x118`) | remove reactor packet; handler is `RemoveReactorHandler` | [`OPCODE_REMOVE_REACTOR`](crates/openstory-protocol/src/gameplay.rs#L719) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2233) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L392)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L428).

### Summons

Summon lifecycle and combat notices.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `SPAWN_SUMMON` (175, `0x0af`) | spawn summon packet; handler is `SpawnSummonHandler` | [`OPCODE_SPAWN_SUMMON`](crates/openstory-protocol/src/gameplay.rs#L695) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2462) | **IU** |
| `REMOVE_SUMMON` (176, `0x0b0`) | remove summon packet; handler is `RemoveSummonHandler` | [`OPCODE_REMOVE_SUMMON`](crates/openstory-protocol/src/gameplay.rs#L696) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2468) | **IU** |
| `MOVE_SUMMON` (177, `0x0b1`) | move summon packet; handler is `MoveSummonHandler` | [`OPCODE_MOVE_SUMMON`](crates/openstory-protocol/src/gameplay.rs#L697) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2474) | **IU** |
| `SUMMON_ATTACK` (178, `0x0b2`) | parse attacker/target damage tuple; original leaves damage display to mob HP updates | [`parse_summon_attack`](crates/openstory-protocol/src/gameplay.rs#L2678) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2504) | **O** |
| `DAMAGE_SUMMON` (179, `0x0b3`) | parse summon damage notification; original apply_damage has no visible state effect | [`parse_summon_damage`](crates/openstory-protocol/src/gameplay.rs#L2707) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2512) | **O** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L431)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L435).

### Attacks

Remote player attack broadcasts (close, ranged, magic).

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `ATTACKED_CLOSE` (186, `0x0ba`) | apply a remote close-range attack hit; handler is `CloseAttackHandler` | [`OPCODE_ATTACKED_CLOSE`](crates/openstory-protocol/src/gameplay.rs#L656) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1690) | **IU** |
| `ATTACKED_RANGED` (187, `0x0bb`) | attacked ranged packet; handler is `RangedAttackHandler` | [`OPCODE_ATTACKED_RANGED`](crates/openstory-protocol/src/gameplay.rs#L657) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1691) | **IU** |
| `ATTACKED_MAGIC` (188, `0x0bc`) | attacked magic packet; handler is `MagicAttackHandler` | [`OPCODE_ATTACKED_MAGIC`](crates/openstory-protocol/src/gameplay.rs#L658) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1692) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L438)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L440).

### Player state and skills

Character stats, buffs, skills, key bindings, cooldowns, and Monster Book state.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `CHANGE_CHANNEL` (16, `0x010`) | reconnect to the returned channel address | [`OPCODE_CHANNEL_TRANSFER`](crates/openstory-protocol/src/login.rs#L25) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1209) | **IU** |
| `KEYMAP` (335, `0x14f`) | keymap packet; handler is `KeymapHandler` | [`OPCODE_KEYMAP`](crates/openstory-protocol/src/keymap.rs#L8) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L582) | **IU** |
| `SKILL_MACROS` (124, `0x07c`) | skill macros packet; handler is `SkillMacrosHandler` | [`OPCODE_SKILL_MACROS`](crates/openstory-protocol/src/skill_macros.rs#L5) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L591) | **IU** |
| `CHANGE_STATS` (31, `0x01f`) | apply character stat/HP/MP updates | [`OPCODE_CHARACTER_STAT_UPDATE`](crates/openstory-protocol/src/login.rs#L36) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1251) | **IU** |
| `GIVE_BUFF` (32, `0x020`) | apply character buff flags and expirations | [`OPCODE_GIVE_BUFF`](crates/openstory-protocol/src/gameplay.rs#L650) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1272) | **IU** |
| `CANCEL_BUFF` (33, `0x021`) | remove character buff flags | [`OPCODE_CANCEL_BUFF`](crates/openstory-protocol/src/gameplay.rs#L651) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1280) | **IU** |
| `RECALCULATE_STATS` (35, `0x023`) | refresh calculated stats | [`OPCODE_RECALCULATE_STATS`](crates/openstory-protocol/src/gameplay.rs#L670) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1269) | **IU** |
| `UPDATE_SKILL` (36, `0x024`) | apply learned skill levels | [`OPCODE_UPDATE_SKILL`](crates/openstory-protocol/src/skills.rs#L5) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1314) | **IU** |
| `ADD_COOLDOWN` (234, `0x0ea`) | start skill cooldown | [`OPCODE_ADD_COOLDOWN`](crates/openstory-protocol/src/gameplay.rs#L730) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2149) | **IU** |
| `SKILL_LEARN_ITEM_RESULT` (51, `0x033`) | skill learn item result packet; handler is `SkillLearnItemResultHandler` | [`OPCODE_SKILL_LEARN_ITEM_RESULT`](crates/openstory-protocol/src/gameplay.rs#L668) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1094) | **IU** |
| `MONSTER_BOOK_SET_CARD` (83, `0x053`) | update a Monster Book card count/level; handler is `MonsterBookCardHandler` | [`OPCODE_MONSTER_BOOK_SET_CARD`](crates/openstory-protocol/src/monster_book.rs#L8) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1106) | **IU** |
| `MONSTER_BOOK_SET_COVER` (84, `0x054`) | update the Monster Book cover; handler is `MonsterBookCoverHandler` | [`OPCODE_MONSTER_BOOK_SET_COVER`](crates/openstory-protocol/src/monster_book.rs#L9) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1123) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L443)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L454).

### Messaging

Status/chat/server notices, inventory/quest transitions, NPC service responses, and cash-shop responses.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `SHOW_STATUS_INFO` (39, `0x027`) | show status info packet; handler is `ShowStatusInfoHandler` | [`OPCODE_SHOW_STATUS_INFO`](crates/openstory-protocol/src/quest.rs#L7) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L796) | **IU** |
| `CHAT_RECEIVED` (162, `0x0a2`) | chat received packet; handler is `ChatReceivedHandler` | [`OPCODE_CHAT_RECEIVED`](crates/openstory-protocol/src/gameplay.rs#L647) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L644) | **IU** |
| `SCROLL_RESULT` (167, `0x0a7`) | scroll result packet; handler is `ScrollResultHandler` | [`OPCODE_SCROLL_RESULT`](crates/openstory-protocol/src/gameplay.rs#L51) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L477) | **IU** |
| `SERVER_MESSAGE` (68, `0x044`) | server message packet; handler is `ServerMessageHandler` | [`OPCODE_SERVER_MESSAGE`](crates/openstory-protocol/src/gameplay.rs#L30) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L465) | **IU** |
| `FAKE_GM_NOTICE` (116, `0x074`) | fake gm notice packet; handler is `FakeGMNoticeHandler` | [`OPCODE_FAKE_GM_NOTICE`](crates/openstory-protocol/src/gameplay.rs#L50) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L471) | **IU** |
| `WEEK_EVENT_MESSAGE` (77, `0x04d`) | week event message packet; handler is `WeekEventMessageHandler` | [`OPCODE_WEEK_EVENT_MESSAGE`](crates/openstory-protocol/src/feedback.rs#L3) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L90) | **IU** |
| `SHOW_ITEM_GAIN_INCHAT` (206, `0x0ce`) | show item gain inchat packet; handler is `ShowItemGainInChatHandler` | [`OPCODE_SHOW_ITEM_GAIN_INCHAT`](crates/openstory-protocol/src/gameplay.rs#L15) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2004) | **IU** |
| `WHISPER` (135, `0x087`) | whisper packet; handler is `WhisperHandler` | [`OPCODE_WHISPER_RECEIVED`](crates/openstory-protocol/src/gameplay.rs#L24) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L696) | **IU** |
| `MULTICHAT` (134, `0x086`) | multichat packet; handler is `MultichatHandler` | [`OPCODE_MULTI_CHAT_RECEIVED`](crates/openstory-protocol/src/gameplay.rs#L23) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L669) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L457)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L465).

### Inventory

Gather/sort acknowledgements and server-authoritative inventory changes.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `MODIFY_INVENTORY` (29, `0x01d`) | apply authoritative inventory modifications | [`OPCODE_MODIFY_INVENTORY`](crates/openstory-protocol/src/inventory.rs#L11) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1328) | **IU** |
| `GATHER_RESULT` (52, `0x034`) | gather result packet; handler is `GatherResultHandler` | [`OPCODE_GATHER_RESULT`](crates/openstory-protocol/src/inventory.rs#L22) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L643) | **IU** |
| `SORT_RESULT` (53, `0x035`) | sort result packet; handler is `SortResultHandler` | [`OPCODE_SORT_RESULT`](crates/openstory-protocol/src/inventory.rs#L23) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L643) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L468)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L470).

### NPC dialogue and shops

Script dialogue, NPC shop open/result state.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `NPC_DIALOGUE` (304, `0x130`) | npc dialogue packet; handler is `NpcDialogueHandler` | [`OPCODE_NPC_DIALOGUE`](crates/openstory-protocol/src/gameplay.rs#L732) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2072) | **IU** |
| `OPEN_NPC_SHOP` (305, `0x131`) | open npc shop packet; handler is `OpenNpcShopHandler` | [`OPCODE_OPEN_NPC_SHOP`](crates/openstory-protocol/src/gameplay.rs#L733) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2078) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L473)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L474).

### Character info and interaction

Character-info response and player interaction state.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `CHAR_INFO` (61, `0x03d`) | char info packet; handler is `CharInfoHandler` | [`OPCODE_CHARACTER_INFO`](crates/openstory-protocol/src/gameplay.rs#L662) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1060) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L477)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L477).

### Cash shop

Cash-shop snapshot, balances, and operation results.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `SET_CASH_SHOP` (127, `0x07f`) | initialize cash-shop state; handler is `SetCashShopHandler` | [`OPCODE_SET_CASH_SHOP`](crates/openstory-protocol/src/cash_shop.rs#L7) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L869) | **IU** |
| `QUERY_CASH_RESULT` (324, `0x144`) | apply a cash balance query result; handler is `QueryCashResultHandler` | [`OPCODE_QUERY_CASH_RESULT`](crates/openstory-protocol/src/cash_shop.rs#L8) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L897) | **IU** |
| `CS_OPERATION` (325, `0x145`) | cs operation packet; handler is `CashShopOperationHandler` | [`OPCODE_CASH_SHOP_OPERATION`](crates/openstory-protocol/src/cash_shop.rs#L9) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L894) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L480)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L482).

### Additional v83

Additional v83 handlers registered by the original client. UI-only notifications are included only as packet/state evidence; no new screen parity is implied.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `CHECK_SPW_RESULT` (28, `0x01c`) | check spw result packet; handler is `CheckSpwResultHandler` | [`OPCODE_CHECK_PIC_RESULT`](crates/openstory-protocol/src/login.rs#L29) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L963) | **IU** |
| `FIELD_EFFECT` (138, `0x08a`) | field effect packet; handler is `FieldEffectHandler` | [`OPCODE_FIELD_EFFECT`](crates/openstory-protocol/src/gameplay.rs#L13) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2106) | **IU** |
| `BLOW_WEATHER` (142, `0x08e`) | blow weather packet; handler is `BlowWeatherHandler` | [`OPCODE_BLOW_WEATHER`](crates/openstory-protocol/src/gameplay.rs#L14) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2133) | **IU** |
| `RELOG_RESPONSE` (22, `0x016`) | reconnect to login server when success byte is set | [`misc.rs` parse arm](crates/openstory-protocol/src/misc.rs#L69) → [`client_packets.rs` dispatcher](crates/openstory-client/src/native/client_packets.rs#L31) → [`apply_misc_server_event`](crates/openstory-client/src/native/client_packets.rs#L2783) | **IU** |
| `UPDATE_QUEST_INFO` (211, `0x0d3`) | apply quest start/complete/forfeit/info variants | [`OPCODE_UPDATE_QUEST_INFO`](crates/openstory-protocol/src/quest.rs#L8) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L819) | **IU** |
| `QUEST_CLEAR` (49, `0x031`) | remove completed quest state | [`OPCODE_QUEST_CLEAR`](crates/openstory-protocol/src/quest.rs#L6) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L803) | **IU** |
| `FAME_RESPONSE` (38, `0x026`) | fame response packet; handler is `FameResponseHandler` | [`OPCODE_FAME_RESPONSE`](crates/openstory-protocol/src/gameplay.rs#L666) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1076) | **IU** |
| `BUDDY_LIST` (63, `0x03f`) | buddy list packet; handler is `BuddyListHandler` | [`OPCODE_BUDDY_LIST`](crates/openstory-protocol/src/gameplay.rs#L22) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L747) | **IU** |
| `FAMILY` (95, `0x05f`) | family packet; handler is `FamilyHandler` | [`OPCODE_FAMILY_INFO`](crates/openstory-protocol/src/family.rs#L6) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L13) | **IU** |
| `PARTY_OPERATION` (62, `0x03e`) | party operation packet; handler is `PartyOperationHandler` | [`OPCODE_PARTY_OPERATION`](crates/openstory-protocol/src/gameplay.rs#L21) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L740) | **IU** |
| `SCRIPT_PROGRESS_MESSAGE` (122, `0x07a`) | script progress message packet; handler is `ScriptProgressMessageHandler` | [`OPCODE_SCRIPT_PROGRESS_MESSAGE`](crates/openstory-protocol/src/feedback.rs#L17) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L104) | **IU** |
| `CLOCK` (147, `0x093`) | clock packet; handler is `ClockHandler` | [`OPCODE_CLOCK`](crates/openstory-protocol/src/gameplay.rs#L8) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L461) | **IU** |
| `FORCED_STAT_SET` (34, `0x022`) | forced stat set packet; handler is `ForcedStatSetHandler` | [`OPCODE_FORCED_STAT_SET`](crates/openstory-protocol/src/gameplay.rs#L669) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1263) | **IU** |
| `NPC_ACTION` (260, `0x104`) | npc action packet; handler is `NpcActionHandler` | [`OPCODE_NPC_ACTION`](crates/openstory-protocol/src/gameplay.rs#L725) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2090) | **IU** |
| `PLAYER_HINT` (214, `0x0d6`) | player hint packet; handler is `YellowTipHandler` | [`OPCODE_PLAYER_HINT`](crates/openstory-protocol/src/gameplay.rs#L34) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L325) | **IU** |
| `BROADCAST_MSG` (212, `0x0d4`) | broadcast msg packet; handler is `BroadcastMsgHandler` | [`OPCODE_BROADCAST_MSG`](crates/openstory-protocol/src/gameplay.rs#L33) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L326) | **IU** |
| `CATCH_MONSTER` (251, `0x0fb`) | catch monster packet; handler is `CatchMonsterHandler` | [`OPCODE_CATCH_MONSTER`](crates/openstory-protocol/src/capture.rs#L4) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L59) | **IU** |
| `QUICKSLOT_INIT` (159, `0x09f`) | quickslot init packet; handler is `QuickSlotInitHandler` | [`OPCODE_QUICKSLOT_INIT`](crates/openstory-protocol/src/keymap.rs#L7) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L611) | **IU** |
| `LAST_CONNECTED_WORLD` (26, `0x01a`) | restore the last connected world selection; handler is `LastConnectedWorldHandler` | [`OPCODE_LAST_CONNECTED_WORLD`](crates/openstory-protocol/src/login.rs#L17) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1036) | **IU** |
| `CLAIM_STATUS_CHANGED` (47, `0x02f`) | update claim status; handler is `ClaimStatusChangedHandler` | [`OPCODE_CLAIM_STATUS_CHANGED`](crates/openstory-protocol/src/feedback.rs#L9) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L96) | **IU** |
| `SUE_CHARACTER_RESULT` (55, `0x037`) | apply character report/sue result; handler is `SueCharacterResultHandler` | [`OPCODE_SUE_CHARACTER_RESULT`](crates/openstory-protocol/src/feedback.rs#L10) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L97) | **IU** |
| `SET_GENDER` (58, `0x03a`) | parse and consume gender confirmation; original already has gender from login, so no state change | [`misc.rs` parse arm](crates/openstory-protocol/src/misc.rs#L87) → [`client_packets.rs` dispatcher](crates/openstory-client/src/native/client_packets.rs#L33) → no-op event arm at [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2803) | **O** |
| `FAMILY_PRIVILEGE_LIST` (100, `0x064`) | family privilege list packet; handler is `FamilyPrivilegeListHandler` | [`OPCODE_FAMILY_PRIVILEGE_LIST`](crates/openstory-protocol/src/family.rs#L11) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L18) | **IU** |
| `ADMIN_RESULT` (144, `0x090`) | mark GM and update hide state for hide-toggle result | [`misc.rs` parse arm](crates/openstory-protocol/src/misc.rs#L105) → [`client_packets.rs` dispatcher](crates/openstory-client/src/native/client_packets.rs#L36) → [`apply_misc_server_event`](crates/openstory-client/src/native/client_packets.rs#L2834) | **IU** |
| `SKILL_EFFECT` (190, `0x0be`) | skill effect packet; handler is `SkillEffectHandler` | [`OPCODE_SKILL_EFFECT`](crates/openstory-protocol/src/gameplay.rs#L676) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1921) | **IU** |
| `SET_NPC_SCRIPTABLE` (263, `0x107`) | set npc scriptable packet; handler is `SetNpcScriptableHandler` | [`OPCODE_SET_NPC_SCRIPTABLE`](crates/openstory-protocol/src/gameplay.rs#L726) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2100) | **IU** |
| `AUTO_HP_POT` (336, `0x150`) | auto hp pot packet; handler is `AutoHpPotHandler` | [`OPCODE_AUTO_HP_POT`](crates/openstory-protocol/src/gameplay.rs#L28) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2121) | **IU** |
| `AUTO_MP_POT` (337, `0x151`) | auto mp pot packet; handler is `AutoMpPotHandler` | [`OPCODE_AUTO_MP_POT`](crates/openstory-protocol/src/gameplay.rs#L29) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2127) | **IU** |
| `FIELD_OBSTACLE_ONOFF` (140, `0x08c`) | field obstacle onoff packet; handler is `FieldObstacleOnOffHandler` | [`OPCODE_FIELD_OBSTACLE_ONOFF`](crates/openstory-protocol/src/gameplay.rs#L728) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2193) | **IU** |
| `FIELD_OBSTACLE_ONE` (139, `0x08b`) | field obstacle one packet; handler is `FieldObstacleOneHandler` | [`OPCODE_FIELD_OBSTACLE_ONE`](crates/openstory-protocol/src/gameplay.rs#L727) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2181) | **IU** |
| `MEMO_RESULT` (41, `0x029`) | report memo sent/failure/other result variants | [`misc.rs` parse arm](crates/openstory-protocol/src/misc.rs#L72) → [`client_packets.rs` dispatcher](crates/openstory-client/src/native/client_packets.rs#L32) → [`apply_misc_server_event`](crates/openstory-client/src/native/client_packets.rs#L2804) | **IU** |
| `IMITATED_NPC` (81, `0x051`) | imitated npc packet; handler is `ImitatedNpcHandler` | [`OPCODE_IMITATED_NPC`](crates/openstory-protocol/src/gameplay.rs#L682) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2297) | **IU** |
| `SPAWN_PORTAL` (67, `0x043`) | spawn portal packet; handler is `SpawnPortalHandler` | [`OPCODE_SPAWN_PORTAL`](crates/openstory-protocol/src/gameplay.rs#L683) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2255) | **IU** |
| `TRADE_MONEY_LIMIT` (57, `0x039`) | trade money limit packet; handler is `TradeMoneyLimitHandler` | [`OPCODE_TRADE_MONEY_LIMIT`](crates/openstory-protocol/src/feedback.rs#L11) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L98) | **IU** |
| `INCUBATOR_RESULT` (69, `0x045`) | incubator result packet; handler is `IncubatorResultHandler` | [`OPCODE_INCUBATOR_RESULT`](crates/openstory-protocol/src/feedback.rs#L24) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L111) | **IU** |
| `SESSION_VALUE` (90, `0x05a`) | apply session-scoped values such as energy | [`misc.rs` parse arm](crates/openstory-protocol/src/misc.rs#L88) → [`client_packets.rs` dispatcher](crates/openstory-client/src/native/client_packets.rs#L34) → [`apply_misc_server_event`](crates/openstory-client/src/native/client_packets.rs#L2816) | **IU** |
| `CANCEL_NAME_CHANGE` (113, `0x071`) | cancel character name change; handler is `CancelNameChangeHandler` | [`OPCODE_CANCEL_NAME_CHANGE`](crates/openstory-protocol/src/feedback.rs#L22) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L109) | **IU** |
| `CANCEL_WORLD_TRANSFER` (114, `0x072`) | cancel world transfer packet; handler is `CancelWorldTransferHandler` | [`OPCODE_CANCEL_WORLD_TRANSFER`](crates/openstory-protocol/src/feedback.rs#L23) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L110) | **IU** |
| `GMEVENT_INSTRUCTIONS` (146, `0x092`) | show GM event instruction notification | [`misc.rs` parse arm](crates/openstory-protocol/src/misc.rs#L118) → [`client_packets.rs` dispatcher](crates/openstory-client/src/native/client_packets.rs#L37) → [`apply_misc_server_event`](crates/openstory-client/src/native/client_packets.rs#L2845) | **IU** |
| `MAPLELIFE_RESULT` (349, `0x15d`) | maplelife result packet; handler is `MapleLifeResultHandler` | [`OPCODE_MAPLELIFE_RESULT`](crates/openstory-protocol/src/feedback.rs#L15) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L102) | **IU** |
| `MAPLELIFE_ERROR` (350, `0x15e`) | maplelife error packet; handler is `MapleLifeErrorHandler` | [`OPCODE_MAPLELIFE_ERROR`](crates/openstory-protocol/src/feedback.rs#L16) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L103) | **IU** |
| `SET_TRACTION` (141, `0x08d`) | set traction packet; handler is `SetTractionHandler` | [`OPCODE_SET_TRACTION`](crates/openstory-protocol/src/gameplay.rs#L729) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2139) | **IU** |
| `OPEN_UI` (220, `0x0dc`) | open ui packet; handler is `OpenUIHandler` | [`OPCODE_OPEN_UI`](crates/openstory-protocol/src/ui.rs#L6) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L554) | **IU** |
| `LOCK_UI` (221, `0x0dd`) | lock ui packet; handler is `LockUIHandler` | [`OPCODE_LOCK_UI`](crates/openstory-protocol/src/ui.rs#L7) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L558) | **IU** |
| `DISABLE_UI` (222, `0x0de`) | disable ui packet; handler is `DisableUIHandler` | [`OPCODE_DISABLE_UI`](crates/openstory-protocol/src/ui.rs#L8) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L558) | **IU** |
| `SPAWN_GUIDE` (223, `0x0df`) | spawn guide packet; handler is `SpawnGuideHandler` | [`OPCODE_SPAWN_GUIDE`](crates/openstory-protocol/src/guide.rs#L3) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L575) | **IU** |
| `TALK_GUIDE` (224, `0x0e0`) | talk guide packet; handler is `TalkGuideHandler` | [`OPCODE_TALK_GUIDE`](crates/openstory-protocol/src/guide.rs#L4) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L575) | **IU** |
| `SHOW_COMBO` (225, `0x0e1`) | update the combo counter; handler is `ShowComboHandler` | [`OPCODE_SHOW_COMBO`](crates/openstory-protocol/src/gameplay.rs#L731) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2175) | **IU** |
| `FORCED_MAP_EQUIP` (133, `0x085`) | forced map equip packet; handler is `ForcedMapEquipHandler` | [`OPCODE_FORCED_MAP_EQUIP`](crates/openstory-protocol/src/feedback.rs#L6) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L93) | **IU** |
| `SPOUSE_CHAT` (136, `0x088`) | apply spouse chat; handler is `SpouseChatHandler` | [`OPCODE_SPOUSE_CHAT`](crates/openstory-protocol/src/social.rs#L10) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1584) | **IU** |
| `CONTI_MOVE` (148, `0x094`) | conti move packet; handler is `ContiMoveHandler` | [`OPCODE_CONTI_MOVE`](crates/openstory-protocol/src/feedback.rs#L7) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L94) | **IU** |
| `CONTI_STATE` (149, `0x095`) | conti state packet; handler is `ContiStateHandler` | [`OPCODE_CONTI_STATE`](crates/openstory-protocol/src/feedback.rs#L8) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L95) | **IU** |
| `STOP_CLOCK` (154, `0x09a`) | stop the field clock; handler is `StopClockHandler` | [`OPCODE_STOP_CLOCK`](crates/openstory-protocol/src/gameplay.rs#L9) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L542) | **IU** |
| `MAKER_RESULT` (217, `0x0d9`) | maker result packet; handler is `MakerResultHandler` | [`OPCODE_MAKER_RESULT`](crates/openstory-protocol/src/maker.rs#L3) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L483) | **IU** |
| `ALLIANCE_OPERATION` (66, `0x042`) | alliance operation packet; handler is `AllianceOperationHandler` | [`OPCODE_ALLIANCE_OPERATION`](crates/openstory-protocol/src/alliance.rs#L6) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L763) | **IU** |
| `MARRIAGE_REQUEST` (72, `0x048`) | marriage request packet; handler is `MarriageRequestHandler` | [`OPCODE_MARRIAGE_REQUEST`](crates/openstory-protocol/src/social.rs#L6) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1585) | **IU** |
| `MARRIAGE_RESULT` (73, `0x049`) | marriage result packet; handler is `MarriageResultHandler` | [`OPCODE_MARRIAGE_RESULT`](crates/openstory-protocol/src/social.rs#L7) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1586) | **IU** |
| `WEDDING_GIFT_RESULT` (74, `0x04a`) | wedding gift result packet; handler is `WeddingGiftResultHandler` | [`OPCODE_WEDDING_GIFT_RESULT`](crates/openstory-protocol/src/social.rs#L8) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1587) | **IU** |
| `NOTIFY_MARRIED_PARTNER` (75, `0x04b`) | notify married partner packet; handler is `NotifyMarriedPartnerHandler` | [`OPCODE_NOTIFY_MARRIED_PARTNER`](crates/openstory-protocol/src/social.rs#L9) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1588) | **IU** |
| `FAMILY_CHART_RESULT` (94, `0x05e`) | apply family chart result; handler is `FamilyChartResultHandler` | [`OPCODE_FAMILY_CHART_RESULT`](crates/openstory-protocol/src/family.rs#L5) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L12) | **IU** |
| `FAMILY_RESULT` (96, `0x060`) | family result packet; handler is `FamilyResultHandler` | [`OPCODE_FAMILY_RESULT`](crates/openstory-protocol/src/family.rs#L7) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L14) | **IU** |
| `FAMILY_JOIN_REQUEST` (97, `0x061`) | family join request packet; handler is `FamilyJoinRequestHandler` | [`OPCODE_FAMILY_JOIN_REQUEST`](crates/openstory-protocol/src/family.rs#L8) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L15) | **IU** |
| `FAMILY_JOIN_REQUEST_RESULT` (98, `0x062`) | family join request result packet; handler is `FamilyJoinRequestResultHandler` | [`OPCODE_FAMILY_JOIN_REQUEST_RESULT`](crates/openstory-protocol/src/family.rs#L9) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L16) | **IU** |
| `FAMILY_JOIN_ACCEPTED` (99, `0x063`) | family join accepted packet; handler is `FamilyJoinAcceptedHandler` | [`OPCODE_FAMILY_JOIN_ACCEPTED`](crates/openstory-protocol/src/family.rs#L10) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L17) | **IU** |
| `FAMILY_REP_GAIN` (101, `0x065`) | family rep gain packet; handler is `FamilyRepGainHandler` | [`OPCODE_FAMILY_REP_GAIN`](crates/openstory-protocol/src/family.rs#L12) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L19) | **IU** |
| `FAMILY_NOTIFY_LOGIN` (102, `0x066`) | family notify login packet; handler is `FamilyLoginLogoutHandler` | [`OPCODE_FAMILY_NOTIFY_LOGIN`](crates/openstory-protocol/src/family.rs#L13) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L20) | **IU** |
| `FAMILY_SUMMON_REQUEST` (104, `0x068`) | family summon request packet; handler is `FamilySummonRequestHandler` | [`OPCODE_FAMILY_SUMMON_REQUEST`](crates/openstory-protocol/src/family.rs#L14) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L21) | **IU** |
| `NOTIFY_LEVELUP` (105, `0x069`) | notify levelup packet; handler is `NotifyLevelUpHandler` | [`OPCODE_NOTIFY_LEVEL_UP`](crates/openstory-protocol/src/social.rs#L3) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1582) | **IU** |
| `NOTIFY_JOB_CHANGE` (107, `0x06b`) | notify nearby clients of a job change; handler is `NotifyJobChangeHandler` | [`OPCODE_NOTIFY_JOB_CHANGE`](crates/openstory-protocol/src/social.rs#L5) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1583) | **IU** |
| `SET_AVATAR_MEGAPHONE` (111, `0x06f`) | set avatar megaphone packet; handler is `SetAvatarMegaphoneHandler` | [`OPCODE_SET_AVATAR_MEGAPHONE`](crates/openstory-protocol/src/gameplay.rs#L31) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L43) | **IU** |
| `CLEAR_AVATAR_MEGAPHONE` (112, `0x070`) | clear avatar megaphone packet; handler is `ClearAvatarMegaphoneHandler` | [`OPCODE_CLEAR_AVATAR_MEGAPHONE`](crates/openstory-protocol/src/gameplay.rs#L32) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L51) | **IU** |
| `NEW_YEAR_CARD` (118, `0x076`) | apply New Year card result; handler is `NewYearCardHandler` | [`OPCODE_NEW_YEAR_CARD`](crates/openstory-protocol/src/feedback.rs#L14) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L101) | **IU** |
| `SET_EXTRA_PENDANT_SLOT` (121, `0x079`) | set extra pendant slot packet; handler is `SetExtraPendantSlotHandler` | [`OPCODE_SET_EXTRA_PENDANT_SLOT`](crates/openstory-protocol/src/inventory.rs#L13) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1383) | **IU** |
| `SET_BACK_EFFECT` (128, `0x080`) | set back effect packet; handler is `SetBackEffectHandler` | [`OPCODE_SET_BACK_EFFECT`](crates/openstory-protocol/src/gameplay.rs#L10) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2113) | **IU** |
| `BLOCKED_MAP` (131, `0x083`) | blocked map packet; handler is `BlockedMapHandler` | [`OPCODE_BLOCKED_MAP`](crates/openstory-protocol/src/feedback.rs#L4) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L91) | **IU** |
| `BLOCKED_SERVER` (132, `0x084`) | blocked server packet; handler is `BlockedServerHandler` | [`OPCODE_BLOCKED_SERVER`](crates/openstory-protocol/src/feedback.rs#L5) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L92) | **IU** |
| `OX_QUIZ` (145, `0x091`) | ox quiz packet; handler is `OXQuizHandler` | [`OPCODE_OX_QUIZ`](crates/openstory-protocol/src/gameplay.rs#L35) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L327) | **IU** |
| `ENTRUSTED_SHOP_CHECK` (50, `0x032`) | apply hired-merchant availability result; handler is `EntrustedShopCheckHandler` | [`OPCODE_ENTRUSTED_SHOP_CHECK_RESULT`](crates/openstory-protocol/src/trade.rs#L8) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L900) | **IU** |
| `INVENTORY_GROW` (30, `0x01e`) | inventory grow packet; handler is `InventoryGrowHandler` | [`OPCODE_INVENTORY_GROW`](crates/openstory-protocol/src/inventory.rs#L12) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1354) | **IU** |
| `MAP_TRANSFER_RESULT` (42, `0x02a`) | map transfer result packet; handler is `TrockResultHandler` | [`OPCODE_MAP_TRANSFER_RESULT`](crates/openstory-protocol/src/teleport_rock.rs#L6) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2159) | **IU** |
| `SET_TAMING_MOB_INFO` (48, `0x030`) | set taming mob info packet; handler is `TamingMobHandler` | [`OPCODE_SET_TAMING_MOB_INFO`](crates/openstory-protocol/src/gameplay.rs#L665) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1068) | **IU** |
| `SHOP_SCANNER_RESULT` (70, `0x046`) | shop scanner result packet; handler is `OwlResultHandler` | [`OPCODE_SHOP_SCANNER_RESULT`](crates/openstory-protocol/src/trade.rs#L12) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L898) | **IU** |
| `SHOP_LINK_RESULT` (71, `0x047`) | shop link result packet; handler is `OwlMessageHandler` | [`OPCODE_SHOP_LINK_RESULT`](crates/openstory-protocol/src/trade.rs#L13) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L899) | **IU** |
| `BRIDLE_MOB_CATCH_FAIL` (79, `0x04f`) | report a failed monster catch; handler is `MobCatchFailHandler` | [`OPCODE_BRIDLE_MOB_CATCH_FAIL`](crates/openstory-protocol/src/capture.rs#L3) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L61) | **IU** |
| `NOTIFY_MARRIAGE` (106, `0x06a`) | notify marriage packet; handler is `MarriageNoticeHandler` | [`OPCODE_NOTIFY_MARRIAGE`](crates/openstory-protocol/src/social.rs#L4) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1589) | **IU** |
| `DATA_CRC_CHECK_FAILED` (123, `0x07b`) | show police/server disconnect message | [`misc.rs` parse arm](crates/openstory-protocol/src/misc.rs#L97) → [`client_packets.rs` dispatcher](crates/openstory-client/src/native/client_packets.rs#L35) → [`apply_misc_server_event`](crates/openstory-client/src/native/client_packets.rs#L2827) | **IU** |
| `LEFT_KNOCK_BACK` (284, `0x11c`) | left knock back packet; handler is `LeftKnockBackHandler` | [`OPCODE_LEFT_KNOCK_BACK`](crates/openstory-protocol/src/gameplay.rs#L748) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1570) | **IU** |
| `SPAWN_HIRED_MERCHANT` (265, `0x109`) | spawn hired merchant packet; handler is `SpawnHiredMerchantHandler` | [`OPCODE_SPAWN_HIRED_MERCHANT`](crates/openstory-protocol/src/trade.rs#L14) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2267) | **IU** |
| `DESTROY_HIRED_MERCHANT` (266, `0x10a`) | destroy hired merchant packet; handler is `DestroyHiredMerchantHandler` | [`OPCODE_DESTROY_HIRED_MERCHANT`](crates/openstory-protocol/src/trade.rs#L15) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2275) | **IU** |
| `UPDATE_HIRED_MERCHANT` (267, `0x10b`) | update hired merchant packet; handler is `UpdateHiredMerchantHandler` | [`OPCODE_UPDATE_HIRED_MERCHANT`](crates/openstory-protocol/src/trade.rs#L16) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2283) | **IU** |
| `FREDRICK_MESSAGE` (310, `0x136`) | Fredrick message/status operation | [`misc.rs` parse arm](crates/openstory-protocol/src/misc.rs#L119) → [`client_packets.rs` dispatcher](crates/openstory-client/src/native/client_packets.rs#L38) → [`apply_misc_server_event`](crates/openstory-client/src/native/client_packets.rs#L2849) | **IU** |
| `FREDRICK` (311, `0x137`) | Fredrick storage/retrieval response operation | [`misc.rs` parse arm](crates/openstory-protocol/src/misc.rs#L120) → [`client_packets.rs` dispatcher](crates/openstory-client/src/native/client_packets.rs#L39) → [`apply_misc_server_event`](crates/openstory-client/src/native/client_packets.rs#L2853) | **IU** |
| `RPS_GAME` (312, `0x138`) | rps game packet; handler is `RPSGameHandler` | [`OPCODE_RPS_GAME`](crates/openstory-protocol/src/rps.rs#L6) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L31) | **IU** |
| `MESSENGER_OP` (313, `0x139`) | messenger op packet; handler is `MessengerHandler` | [`OPCODE_MESSENGER`](crates/openstory-protocol/src/messenger.rs#L8) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L548) | **IU** |
| `PET_NAMECHANGE` (172, `0x0ac`) | pet namechange packet; handler is `PetNameChangeHandler` | [`OPCODE_PET_NAME_CHANGE`](crates/openstory-protocol/src/gameplay.rs#L705) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1534) | **IU** |
| `PET_EXCEPTION_LIST` (173, `0x0ad`) | pet exception list packet; handler is `PetExceptionListHandler` | [`OPCODE_PET_EXCEPTION_LIST`](crates/openstory-protocol/src/gameplay.rs#L706) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1556) | **IU** |
| `CATCH_MONSTER_WITH_ITEM` (252, `0x0fc`) | catch monster with item packet; handler is `CatchMonsterWithItemHandler` | [`OPCODE_CATCH_MONSTER_WITH_ITEM`](crates/openstory-protocol/src/capture.rs#L5) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L60) | **IU** |
| `CANNOT_SPAWN_KITE` (270, `0x10e`) | cannot spawn kite packet; handler is `CannotSpawnKiteHandler` | [`OPCODE_CANNOT_SPAWN_KITE`](crates/openstory-protocol/src/feedback.rs#L18) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L105) | **IU** |
| `SPAWN_KITE` (271, `0x10f`) | spawn kite packet; handler is `SpawnKiteHandler` | [`OPCODE_SPAWN_KITE`](crates/openstory-protocol/src/feedback.rs#L19) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L106) | **IU** |
| `REMOVE_KITE` (272, `0x110`) | remove kite packet; handler is `RemoveKiteHandler` | [`OPCODE_REMOVE_KITE`](crates/openstory-protocol/src/feedback.rs#L20) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L107) | **IU** |
| `THROW_GRENADE` (204, `0x0cc`) | throw grenade packet; handler is `ThrowGrenadeHandler` | [`OPCODE_THROW_GRENADE`](crates/openstory-protocol/src/gameplay.rs#L678) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1941) | **IU** |
| `DOJO_WARP_UP` (207, `0x0cf`) | dojo warp up packet; handler is `DojoWarpUpHandler` | [`OPCODE_DOJO_WARP_UP`](crates/openstory-protocol/src/gameplay.rs#L45) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L337) | **IU** |
| `ARIANT_ARENA_SHOW_RESULT` (155, `0x09b`) | ariant arena show result packet; handler is `AriantShowResultHandler` | [`OPCODE_ARIANT_ARENA_SHOW_RESULT`](crates/openstory-protocol/src/gameplay.rs#L41) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L333) | **IU** |
| `PYRAMID_GAUGE` (157, `0x09d`) | pyramid gauge packet; handler is `PyramidGaugeHandler` | [`OPCODE_PYRAMID_GAUGE`](crates/openstory-protocol/src/gameplay.rs#L43) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L335) | **IU** |
| `PYRAMID_SCORE` (158, `0x09e`) | pyramid score packet; handler is `PyramidScoreHandler` | [`OPCODE_PYRAMID_SCORE`](crates/openstory-protocol/src/gameplay.rs#L44) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L336) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L485)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L591).

### Map events

Snowball, coconut, and Ariant event state.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `SNOWBALL_STATE` (281, `0x119`) | snowball state packet; handler is `SnowballStateHandler` | [`OPCODE_SNOWBALL_STATE`](crates/openstory-protocol/src/gameplay.rs#L36) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L328) | **IU** |
| `HIT_SNOWBALL` (282, `0x11a`) | hit snowball packet; handler is `HitSnowballHandler` | [`OPCODE_HIT_SNOWBALL`](crates/openstory-protocol/src/gameplay.rs#L37) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L329) | **IU** |
| `SNOWBALL_MESSAGE` (283, `0x11b`) | snowball message packet; handler is `SnowballMessageHandler` | [`OPCODE_SNOWBALL_MESSAGE`](crates/openstory-protocol/src/gameplay.rs#L38) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L330) | **IU** |
| `COCONUT_HIT` (285, `0x11d`) | coconut hit packet; handler is `CoconutHitHandler` | [`OPCODE_COCONUT_HIT`](crates/openstory-protocol/src/gameplay.rs#L39) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L331) | **IU** |
| `COCONUT_SCORE` (286, `0x11e`) | coconut score packet; handler is `CoconutScoreHandler` | [`OPCODE_COCONUT_SCORE`](crates/openstory-protocol/src/gameplay.rs#L40) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L332) | **IU** |
| `ARIANT_ARENA_USER_SCORE` (297, `0x129`) | ariant arena user score packet; handler is `AriantScoreHandler` | [`OPCODE_ARIANT_ARENA_USER_SCORE`](crates/openstory-protocol/src/gameplay.rs#L42) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L334) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L594)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L599).

### Tournament

Tournament state/result variants.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `TOURNAMENT` (315, `0x13b`) | tournament packet; handler is `TournamentHandler` | [`OPCODE_TOURNAMENT`](crates/openstory-protocol/src/gameplay.rs#L46) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L338) | **IU** |
| `TOURNAMENT_MATCH_TABLE` (316, `0x13c`) | tournament match table packet; handler is `TournamentMatchTableHandler` | [`OPCODE_TOURNAMENT_MATCH_TABLE`](crates/openstory-protocol/src/gameplay.rs#L47) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L339) | **IU** |
| `TOURNAMENT_SET_PRIZE` (317, `0x13d`) | tournament set prize packet; handler is `TournamentSetPrizeHandler` | [`OPCODE_TOURNAMENT_SET_PRIZE`](crates/openstory-protocol/src/gameplay.rs#L48) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L340) | **IU** |
| `TOURNAMENT_UEW` (318, `0x13e`) | tournament uew packet; handler is `TournamentUEWHandler` | [`OPCODE_TOURNAMENT_UEW`](crates/openstory-protocol/src/gameplay.rs#L49) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L341) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L602)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L605).

### Wedding

Wedding ceremony state/result variants.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `WEDDING_PROGRESS` (320, `0x140`) | wedding progress packet; handler is `WeddingProgressHandler` | [`OPCODE_WEDDING_PROGRESS`](crates/openstory-protocol/src/social.rs#L11) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1590) | **IU** |
| `WEDDING_CEREMONY_END` (321, `0x141`) | end the wedding ceremony state; handler is `WeddingCeremonyEndHandler` | [`OPCODE_WEDDING_CEREMONY_END`](crates/openstory-protocol/src/social.rs#L12) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1591) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L608)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L609).

### Custom Monster Life/Battle

Custom noncanonical server extensions.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `MLIFE_UPDATE` (400, `0x190`) | custom Monster Life farm/placement/move/remove/error variants | — | **SC** |
| `MBATTLE_UPDATE` (401, `0x191`) | custom Monster Battle unit/team/error variants | — | **SC** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L612)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L613).

### MapleTV

MapleTV broadcast lifecycle.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `SEND_TV` (341, `0x155`) | send tv packet; handler is `SendTVHandler` | [`OPCODE_SEND_TV`](crates/openstory-protocol/src/gameplay.rs#L25) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L628) | **IU** |
| `REMOVE_TV` (342, `0x156`) | remove tv packet; handler is `RemoveTVHandler` | [`OPCODE_REMOVE_TV`](crates/openstory-protocol/src/gameplay.rs#L26) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L632) | **IU** |
| `ENABLE_TV` (343, `0x157`) | enable tv packet; handler is `EnableTVHandler` | [`OPCODE_ENABLE_TV`](crates/openstory-protocol/src/gameplay.rs#L27) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L638) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L616)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L618).

### Item enhancement

Vicious Hammer/Vega result notifications.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `VICIOUS_HAMMER` (354, `0x162`) | vicious hammer packet; handler is `ViciousHammerHandler` | [`OPCODE_VICIOUS_HAMMER`](crates/openstory-protocol/src/feedback.rs#L12) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L99) | **IU** |
| `VEGA_SCROLL` (358, `0x166`) | vega scroll packet; handler is `VegaScrollHandler` | [`OPCODE_VEGA_SCROLL`](crates/openstory-protocol/src/feedback.rs#L13) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L100) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L621)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L622).

### Cash shop extensions

Cash-shop name/world-transfer/gachapon results.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `CS_CHECK_NAME_CHANGE` (328, `0x148`) | apply Cash Shop name-change check result; handler is `CashShopNameChangeHandler` | [`OPCODE_CS_CHECK_NAME_CHANGE`](crates/openstory-protocol/src/feedback.rs#L25) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L112) | **IU** |
| `CS_NAME_CHANGE_POSSIBLE` (329, `0x149`) | apply Cash Shop name-change eligibility result; handler is `CashShopNameChangePossibleHandler` | [`OPCODE_CS_NAME_CHANGE_POSSIBLE`](crates/openstory-protocol/src/feedback.rs#L26) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L113) | **IU** |
| `CS_TRANSFER_WORLD` (331, `0x14b`) | cs transfer world packet; handler is `CashShopTransferWorldHandler` | [`OPCODE_CS_TRANSFER_WORLD`](crates/openstory-protocol/src/feedback.rs#L27) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L114) | **IU** |
| `CS_CASH_GACHAPON_RESULT` (333, `0x14d`) | apply Cash Shop gachapon result; handler is `CashGachaponResultHandler` | [`OPCODE_CS_CASH_GACHAPON_RESULT`](crates/openstory-protocol/src/feedback.rs#L28) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L115) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L625)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L628).

### Monster Carnival

Monster Carnival start, CP, summon, message, and death updates.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `MONSTER_CARNIVAL_START` (289, `0x121`) | start Monster Carnival state; handler is `MonsterCarnivalStartHandler` | [`OPCODE_MONSTER_CARNIVAL_START`](crates/openstory-protocol/src/gameplay.rs#L52) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L448) | **IU** |
| `MONSTER_CARNIVAL_CP` (290, `0x122`) | update Monster Carnival CP; handler is `MonsterCarnivalCPHandler` | [`OPCODE_MONSTER_CARNIVAL_CP`](crates/openstory-protocol/src/gameplay.rs#L53) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L449) | **IU** |
| `MONSTER_CARNIVAL_PARTY_CP` (291, `0x123`) | update party Monster Carnival CP; handler is `MonsterCarnivalPartyCPHandler` | [`OPCODE_MONSTER_CARNIVAL_PARTY_CP`](crates/openstory-protocol/src/gameplay.rs#L54) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L450) | **IU** |
| `MONSTER_CARNIVAL_SUMMON` (292, `0x124`) | apply Monster Carnival summon; handler is `MonsterCarnivalSummonHandler` | [`OPCODE_MONSTER_CARNIVAL_SUMMON`](crates/openstory-protocol/src/gameplay.rs#L55) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L451) | **IU** |
| `MONSTER_CARNIVAL_MESSAGE` (293, `0x125`) | show Monster Carnival message; handler is `MonsterCarnivalMessageHandler` | [`OPCODE_MONSTER_CARNIVAL_MESSAGE`](crates/openstory-protocol/src/gameplay.rs#L56) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L452) | **IU** |
| `MONSTER_CARNIVAL_DIED` (294, `0x126`) | apply Monster Carnival death update; handler is `MonsterCarnivalDiedHandler` | [`OPCODE_MONSTER_CARNIVAL_DIED`](crates/openstory-protocol/src/gameplay.rs#L57) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L453) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L631)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L636).

### Foreign characters, storage, and trade

Remote character effects/state plus storage/shop/trade interactions.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `DAMAGE_PLAYER` (192, `0x0c0`) | damage player packet; handler is `DamagePlayerHandler` | [`OPCODE_DAMAGE_PLAYER`](crates/openstory-protocol/src/gameplay.rs#L654) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1849) | **IU** |
| `FACIAL_EXPRESSION` (193, `0x0c1`) | facial expression packet; handler is `FacialExpressionHandler` | [`OPCODE_FACIAL_EXPRESSION`](crates/openstory-protocol/src/gameplay.rs#L671) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1859) | **IU** |
| `GIVE_FOREIGN_BUFF` (199, `0x0c7`) | give foreign buff packet; handler is `GiveForeignBuffHandler` | [`OPCODE_GIVE_FOREIGN_BUFF`](crates/openstory-protocol/src/gameplay.rs#L652) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1288) | **IU** |
| `CANCEL_FOREIGN_BUFF` (200, `0x0c8`) | cancel foreign buff packet; handler is `CancelForeignBuffHandler` | [`OPCODE_CANCEL_FOREIGN_BUFF`](crates/openstory-protocol/src/gameplay.rs#L653) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1306) | **IU** |
| `UPDATE_PARTYMEMBER_HP` (201, `0x0c9`) | update partymember hp packet; handler is `UpdatePartyMemberHPHandler` | [`OPCODE_UPDATE_PARTY_MEMBER_HP`](crates/openstory-protocol/src/gameplay.rs#L721) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L780) | **IU** |
| `GUILD_NAME_CHANGED` (202, `0x0ca`) | update a remote guild name; handler is `GuildNameChangedHandler` | [`OPCODE_GUILD_NAME_CHANGED`](crates/openstory-protocol/src/gameplay.rs#L736) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1883) | **IU** |
| `GUILD_MARK_CHANGED` (203, `0x0cb`) | update a remote guild emblem; handler is `GuildMarkChangedHandler` | [`OPCODE_GUILD_MARK_CHANGED`](crates/openstory-protocol/src/gameplay.rs#L737) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1889) | **IU** |
| `GUILD_OPERATION` (65, `0x041`) | guild operation packet; handler is `GuildOperationHandler` | [`OPCODE_GUILD_OPERATION`](crates/openstory-protocol/src/guild.rs#L5) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L754) | **IU** |
| `GUILD_BBS` (59, `0x03b`) | guild bbs packet; handler is `GuildBBSHandler` | [`OPCODE_GUILD_BBS_PACKET`](crates/openstory-protocol/src/guild_bbs.rs#L5) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L772) | **IU** |
| `CANCEL_CHAIR` (205, `0x0cd`) | remove a remote chair; handler is `CancelChairHandler` | [`OPCODE_CANCEL_CHAIR_BROADCAST`](crates/openstory-protocol/src/gameplay.rs#L675) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2056) | **IU** |
| `SHOW_CHAIR` (196, `0x0c4`) | show a remote chair; handler is `ShowChairHandler` | [`OPCODE_SHOW_CHAIR`](crates/openstory-protocol/src/gameplay.rs#L674) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2041) | **IU** |
| `SHOW_ITEM_EFFECT` (194, `0x0c2`) | show item effect packet; handler is `ShowItemEffectHandler` | [`OPCODE_SHOW_ITEM_EFFECT`](crates/openstory-protocol/src/gameplay.rs#L660) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1895) | **IU** |
| `STORAGE` (309, `0x135`) | storage packet; handler is `StorageHandler` | [`OPCODE_STORAGE`](crates/openstory-protocol/src/inventory.rs#L14) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L1399) | **IU** |
| `CONFIRM_SHOP_TRANSACTION` (306, `0x132`) | confirm shop transaction packet; handler is `ConfirmShopTransactionHandler` | [`OPCODE_CONFIRM_NPC_SHOP_TRANSACTION`](crates/openstory-protocol/src/gameplay.rs#L734) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2084) | **IU** |
| `PARCEL` (322, `0x142`) | Duey package listing and delivery-result variants | [`misc.rs` parse arm and bounded parser](crates/openstory-protocol/src/misc.rs#L121) → [`client_packets.rs` dispatcher](crates/openstory-client/src/native/client_packets.rs#L40) → [`apply_misc_server_event`](crates/openstory-client/src/native/client_packets.rs#L2857) | **IU** |
| `PLAYER_INTERACTION` (314, `0x13a`) | player interaction packet; handler is `PlayerInteractionHandler` | [`OPCODE_PLAYER_INTERACTION_RESULT`](crates/openstory-protocol/src/trade.rs#L17) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L903) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L639)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L654).

### MTS

Maple Trading System responses; suboperations are expanded below.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `MTS_OPERATION` (348, `0x15c`) | apply MTS operation variant | [`OPCODE_MTS_OPERATION`](crates/openstory-protocol/src/mts.rs#L7) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L882) | **IU** |
| `MTS_OPERATION2` (347, `0x15b`) | replace MTS cash/Maple Point balances | [`OPCODE_MTS_CASH`](crates/openstory-protocol/src/mts.rs#L8) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L876) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L657)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L658).

### Event info

Server event/reward listing.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `EVENT_INFO` (195, `0x0c3`) | replace server event/reward listing | [`OPCODE_EVENT_INFO`](crates/openstory-protocol/src/events.rs#L3) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L37) | **IU** |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L661)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L661).

### Bot inventory

Custom bot inventory snapshot.

| Incoming opcode (decimal / hex) | Behavior / state variant | Rust counterpart | Status |
| --- | --- | --- | --- |
| `BOT_INVENTORY` (373, `0x175`) | replace custom bot inventory snapshot or reject it | [`parse_bot_inventory`](crates/openstory-protocol/src/bot_inventory.rs#L36) → [`client_packets.rs` dispatcher/state](crates/openstory-client/src/native/client_packets.rs#L903) | **SC** — Cosmic has no server send/receive contract for this custom opcode. |

Source registrations: [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L664)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L664).

### Variant detail: MTS and bot inventory

| Incoming opcode | Wire variant / state transition | Rust counterpart | Status |
| --- | --- | --- | --- |
| `MTS_OPERATION` `0x15C`, operation `0x15` | Replace listing page (total/count/tab/category/page, bounded item listings). | [`mts.rs::parse_mts_operation`](crates/openstory-protocol/src/mts.rs#L50) → native event application in [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L889) | **IU** |
| `MTS_OPERATION` `0x15C`, operation `0x1D` | Sell confirmation. | Same parser/event route. | **IU** |
| `MTS_OPERATION` `0x15C`, operation `0x21` | Replace transfer inventory; ignore trailing extensions as the original does. | Same parser/event route. | **IU** |
| `MTS_OPERATION` `0x15C`, operation `0x23` | Replace not-yet-sold list; parse empty-list terminator when count is zero. | Same parser/event route. | **IU** |
| `MTS_OPERATION` `0x15C`, operation `0x27` | Transfer-confirmed quantity and destination position. | Same parser/event route. | **IU** |
| `MTS_OPERATION` `0x15C`, operation `0x33` / `0x34` | Buy confirmed / buy failed (failure carries one reserved byte). | Same parser/event route. | **IU** |
| `MTS_OPERATION` `0x15C`, operation `0x3D` | Wanted-listing completion counters. | Same parser/event route. | **IU** |
| `MTS_OPERATION` `0x15C`, unknown operation | Surface as `Unknown`; do not consume an assumed payload, matching original default no-op. | [`mts.rs`](crates/openstory-protocol/src/mts.rs#L111) | **O** |
| `MTS_OPERATION2` `0x15B` | Replace NX prepaid and Maple Point balances. | [`mts.rs::parse_mts_cash`](crates/openstory-protocol/src/mts.rs#L119) | **IU** |
| `BOT_INVENTORY` `0x175`, result `1` | Parse complete equipped/inventory tabs, mesos, and stack-count amendments into a snapshot before applying it. | [`bot_inventory.rs::parse_bot_inventory`](crates/openstory-protocol/src/bot_inventory.rs#L36) → [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L903) | **SC** |
| `BOT_INVENTORY` `0x175`, result other than `1` | Return a rejected result without applying a partial snapshot. | Same parser/event route. | **SC** |

MTS is canonical v83 packet behavior. Bot inventory is custom to adjacent OpenStory, but the supplied Cosmic server does not implement it: its [`RecvOpcode.java`](../Cosmic/src/main/java/net/opcodes/RecvOpcode.java#L203)–[`RecvOpcode.java`](../Cosmic/src/main/java/net/opcodes/RecvOpcode.java#L205) ends its opcode enum with `USE_HAMMER(0x104)` and defines no bot opcodes; the receive registrations in [`PacketProcessor.java`](../Cosmic/src/main/java/net/PacketProcessor.java#L274)–[`PacketProcessor.java`](../Cosmic/src/main/java/net/PacketProcessor.java#L452) include no bot handler, and the server send-opcode enum ends without bot inventory at [`SendOpcode.java`](../Cosmic/src/main/java/net/opcodes/SendOpcode.java#L345)–[`SendOpcode.java`](../Cosmic/src/main/java/net/opcodes/SendOpcode.java#L356). Keep both `BOT_INVENTORY` `0x175` snapshots and `BOT_INV_ACTION` `0x168` requests **SC** unless the target server contract adds them. MTS incoming `0x15C` and outgoing `0xFD` deliberately use different direction-specific values. Original OpenStory wire details are in [MTSHandlers.cpp](../OpenStory/src/Net/Handlers/MTSHandlers.cpp#L25)–[MTSHandlers.cpp](../OpenStory/src/Net/Handlers/MTSHandlers.cpp#L250), [MTSPackets.h](../OpenStory/src/Net/Packets/MTSPackets.h#L24)–[MTSPackets.h](../OpenStory/src/Net/Packets/MTSPackets.h#L179), [BotInventoryHandlers.cpp](../OpenStory/src/Net/Handlers/BotInventoryHandlers.cpp#L27)–[BotInventoryHandlers.cpp](../OpenStory/src/Net/Handlers/BotInventoryHandlers.cpp#L129), and [BotInventoryPackets.h](../OpenStory/src/Net/Packets/BotInventoryPackets.h#L24)–[BotInventoryPackets.h](../OpenStory/src/Net/Packets/BotInventoryPackets.h#L67). Rust fixtures cover MTS responses and bot parsing/action bytes; they prove local packet handling, not Cosmic compatibility.

## Outgoing request/builders (feature-level)

Outgoing definitions are in [OutPacket.h](../OpenStory/src/Net/OutPacket.h#L36)–[OutPacket.h](../OpenStory/src/Net/OutPacket.h#L197). The Rust counterparts below are grouped by feature as requested; outgoing packet constants must be read in the client→server namespace, not matched to same-number receive packets.

| Feature / outgoing variants | Rust builder location | Status / notes |
| --- | --- | --- |
| Login/world/character: login, server/world/character list, TOS, select/create/delete, PIC register/verify, ping/pong, login start, `SET_GENDER` | [`login.rs`](crates/openstory-protocol/src/login.rs#L600), [`legacy_requests.rs`](crates/openstory-protocol/src/legacy_requests.rs#L17), and public builders in [`lib.rs`](crates/openstory-protocol/src/lib.rs#L217) | **IU** for the implemented flows; no live-server evidence. `SET_GENDER` uses the reviewed enable-byte/gender shape; the incoming gender confirmation remains an original no-op. |
| Field/movement/combat: map/portal transfer, channel transfer, `MOVE_PLAYER`, `PLAYER_MAP_TRANSFER`, `PLAYER_UPDATE`, close/ranged/magic attack, damage, reactor damage, mob-controller movement | [`gameplay.rs`](crates/openstory-protocol/src/gameplay.rs#L600), [`gameplay.rs`](crates/openstory-protocol/src/gameplay.rs#L2323), [`gameplay.rs`](crates/openstory-protocol/src/gameplay.rs#L3992), [`legacy_requests.rs`](crates/openstory-protocol/src/legacy_requests.rs#L61) | **IU** for map transfer and existing movement/combat builders. `PLAYER_UPDATE` has a byte-shape builder, but the original deliberately does not send it: supplied Cosmic uses client→server opcode 223 for party-search update and has no `PLAYER_UPDATE` handler. |
| NPC dialogue/shop/storage: talk, next/selection/text/number responses, buy/sell/recharge/leave, storage deposit/withdraw/meso/close | [`gameplay.rs`](crates/openstory-protocol/src/gameplay.rs#L2323), [`inventory.rs`](crates/openstory-protocol/src/inventory.rs#L523) | **IU**; server owns transaction and inventory outcomes. Token-for-mesos shop purchase remains **SC** and intentionally disabled per [`PORT_PLAN.md`](PORT_PLAN.md#L769)–[`PORT_PLAN.md`](PORT_PLAN.md#L772). |
| Inventory/items/equipment/pets/chairs/scrolls: use/move/sort/gather/scroll/equip/pet food/spawn/loot/name tag/chat/command/cash items | [`inventory.rs`](crates/openstory-protocol/src/inventory.rs#L668)–[`inventory.rs`](crates/openstory-protocol/src/inventory.rs#L904), [`gameplay.rs`](crates/openstory-protocol/src/gameplay.rs#L2331), [`legacy_requests.rs`](crates/openstory-protocol/src/legacy_requests.rs#L33)–[`legacy_requests.rs`](crates/openstory-protocol/src/legacy_requests.rs#L44) | **IU**; item restrictions and authoritative response behavior need broader deterministic tests and live exercise. Pet chat and command builders have byte fixtures. |
| Skills/stats/macros/key map/quick slots: spend AP/SP, use skill, skill macro edit, key map edit, `MONSTER_BOOK_COVER` | [`skills.rs`](crates/openstory-protocol/src/skills.rs#L80), [`skill_macros.rs`](crates/openstory-protocol/src/skill_macros.rs#L58), [`keymap.rs`](crates/openstory-protocol/src/keymap.rs#L83), [`gameplay.rs`](crates/openstory-protocol/src/gameplay.rs#L635), [`legacy_requests.rs`](crates/openstory-protocol/src/legacy_requests.rs#L54) | **IU** for AP/SP, skills, macros, key map, and Monster Book cover requests; byte fixtures cover the cover item ID and clear-cover value. |
| Pet/character actions: pet chat, pet command, NPC action synchronization echo | [`legacy_requests.rs`](crates/openstory-protocol/src/legacy_requests.rs#L33)–[`legacy_requests.rs`](crates/openstory-protocol/src/legacy_requests.rs#L44) | **IU** for pet chat/command builders. **SC** for the NPC action echo: C++ sends it on each inbound NPC action, while its source comment says server-authoritative engines ignore it; confirm the supplied server contract before adding it. |
| Party/buddy/chat/whisper/find/fame/character info | [`gameplay.rs`](crates/openstory-protocol/src/gameplay.rs#L1895)–[`gameplay.rs`](crates/openstory-protocol/src/gameplay.rs#L2270) | **IU**; incoming event handling exists, but no server-backed social test evidence is recorded. |
| Guild/alliance/BBS/family/messenger/trade/merchant/wedding | [`guild.rs`](crates/openstory-protocol/src/guild.rs#L221), [`guild_bbs.rs`](crates/openstory-protocol/src/guild_bbs.rs#L120), [`alliance.rs`](crates/openstory-protocol/src/alliance.rs#L245), [`family.rs`](crates/openstory-protocol/src/family.rs#L272), [`messenger.rs`](crates/openstory-protocol/src/messenger.rs#L1), [`trade.rs`](crates/openstory-protocol/src/trade.rs#L227), [`legacy_requests.rs`](crates/openstory-protocol/src/legacy_requests.rs#L73) | **IU** for the implemented request families. Wedding registry operations 6 (add gift) and 7 (take gift) have typed builders and byte fixtures. Cosmic defines/registers `WEDDING_ACTION` at [`RecvOpcode.java`](../Cosmic/src/main/java/net/opcodes/RecvOpcode.java#L143) and [`PacketProcessor.java`](../Cosmic/src/main/java/net/PacketProcessor.java#L442); [`WeddingHandler.java` op 6 fields](../Cosmic/src/main/java/net/server/channel/handlers/WeddingHandler.java#L44) and [op 7 fields](../Cosmic/src/main/java/net/server/channel/handlers/WeddingHandler.java#L109) define the request shapes. Adjacent C++ only declares the outgoing opcode in [`OutPacket.h`](../OpenStory/src/Net/OutPacket.h#L192)–[`OutPacket.h`](../OpenStory/src/Net/OutPacket.h#L193), with no packet class, call site, or UI dispatch. |
| Cash Shop/coupon/locker/withdrawal, MapleTV, MTS | [`cash_shop.rs`](crates/openstory-protocol/src/cash_shop.rs#L1), [`inventory.rs`](crates/openstory-protocol/src/inventory.rs#L680), [`mts.rs`](crates/openstory-protocol/src/mts.rs#L133) | **IU**; MTS incoming/outgoing operation variants have fixtures, but no live server flow is recorded. |
| Custom bot inventory take/give/equip requests | [`bot_inventory.rs`](crates/openstory-protocol/src/bot_inventory.rs#L90) | **SC**; Cosmic's [`RecvOpcode.java`](../Cosmic/src/main/java/net/opcodes/RecvOpcode.java#L203)–[`RecvOpcode.java`](../Cosmic/src/main/java/net/opcodes/RecvOpcode.java#L205) has no `BOT_INV_ACTION` `0x168`, and [`PacketProcessor.java`](../Cosmic/src/main/java/net/PacketProcessor.java#L274)–[`PacketProcessor.java`](../Cosmic/src/main/java/net/PacketProcessor.java#L452) registers no handler for it. |
| Party search, RPS, event-info request | [`party_search.rs`](crates/openstory-protocol/src/party_search.rs#L55), [`rps.rs`](crates/openstory-protocol/src/rps.rs#L41), [`events.rs`](crates/openstory-protocol/src/events.rs#L26) | **IU** for packet builders, server contracts/end-to-end behavior unverified. |
| Report/admin | [`legacy_requests.rs`](crates/openstory-protocol/src/legacy_requests.rs#L108), [`legacy_requests.rs`](crates/openstory-protocol/src/legacy_requests.rs#L144) | **IU** for the two explicit report variants and admin enter-map commands; byte fixtures follow the C++ packet classes. |
| Monster Life/Battle custom requests | No matching Rust builder found. | **SC**; the custom opcodes require the supplied server's scripts and exact wire contract. |

The outgoing names and direction-specific numbers above follow [OutPacket.h](../OpenStory/src/Net/OutPacket.h#L39)–[OutPacket.h](../OpenStory/src/Net/OutPacket.h#L197). Rust now has byte-fixtured builders for `SET_GENDER`, `MONSTER_BOOK_COVER`, `PET_CHAT_SEND`, `PET_COMMAND_SEND`, `PLAYER_MAP_TRANSFER`, `PLAYER_UPDATE`, both Cosmic `WEDDING_ACTION` registry variants, the explicit `REPORT` variants, and admin enter-map commands. `PLAYER_UPDATE` is not dispatched because the supplied Cosmic server assigns client→server opcode 223 to party-search update and has no player-update handler, as documented in [SetFieldHandlers.cpp](../OpenStory/src/Net/Handlers/SetFieldHandlers.cpp#L150). The wedding builders use Cosmic's registered handler contract and are **IU**; the adjacent C++ checkout declares the opcode but provides no packet class, payload call site, or UI dispatch. Bot inventory actions have local byte fixtures but remain **SC** because Cosmic lacks their opcodes and handler registrations. Duey/Fredrick are incoming response paths in the 241-row inventory; the C++ outgoing enum reviewed here does not list separate mail request opcodes. `NPC_ACTION` is listed as an outgoing enum at [OutPacket.h](../OpenStory/src/Net/OutPacket.h#L78)–[OutPacket.h](../OpenStory/src/Net/OutPacket.h#L82), with the stated server-ignore caveat.

## Direction-specific opcode overlaps

The number belongs to a receive or send namespace. **None of the 241 incoming registrations duplicates another incoming numeric value.** However, 56 numeric values also appear in the outgoing `OutPacket::Opcode` enum; these are cross-direction collisions, not duplicate registrations. The name can differ as well: e.g. incoming `SET_ITC` is `0x07E`, outgoing `GUILD_OPERATION` is also `0x07E`; incoming MTS operations use `0x15C`, outgoing MTS operations use `0x0FD`. Never compare by number without recording direction.

| Decimal / hex | Server → client (`PacketSwitch`) | Client → server (`OutPacket`) |
| ---: | --- | --- |
| 11 / `0x00b` | `CHARLIST` | `SERVERLIST_REQUEST` |
| 22 / `0x016` | `RELOG_RESPONSE` | `CREATE_CHAR` |
| 29 / `0x01d` | `MODIFY_INVENTORY` | `REGISTER_PIC` |
| 30 / `0x01e` | `INVENTORY_GROW` | `SELECT_CHAR_PIC` |
| 35 / `0x023` | `RECALCULATE_STATS` | `LOGIN_START` |
| 38 / `0x026` | `FAME_RESPONSE` | `CHANGEMAP` |
| 39 / `0x027` | `SHOW_STATUS_INFO` | `CHANGE_CHANNEL` |
| 41 / `0x029` | `MEMO_RESULT` | `MOVE_PLAYER` |
| 42 / `0x02a` | `MAP_TRANSFER_RESULT` | `CANCEL_CHAIR` |
| 48 / `0x030` | `SET_TAMING_MOB_INFO` | `TAKE_DAMAGE` |
| 49 / `0x031` | `QUEST_CLEAR` | `GENERAL_CHAT` |
| 57 / `0x039` | `TRADE_MONEY_LIMIT` | `MONSTER_BOOK_COVER` |
| 58 / `0x03a` | `SET_GENDER` | `TALK_TO_NPC` |
| 61 / `0x03d` | `CHAR_INFO` | `NPC_SHOP_ACTION` |
| 62 / `0x03e` | `PARTY_OPERATION` | `STORAGE_ACTION` |
| 63 / `0x03f` | `BUDDY_LIST` | `HIRED_MERCHANT_REQUEST` |
| 66 / `0x042` | `ALLIANCE_OPERATION` | `OWL_ACTION` |
| 67 / `0x043` | `SPAWN_PORTAL` | `OWL_WARP` |
| 69 / `0x045` | `INCUBATOR_RESULT` | `GATHER_ITEMS` |
| 70 / `0x046` | `SHOP_SCANNER_RESULT` | `SORT_ITEMS` |
| 71 / `0x047` | `SHOP_LINK_RESULT` | `MOVE_ITEM` |
| 72 / `0x048` | `MARRIAGE_REQUEST` | `USE_ITEM` |
| 79 / `0x04f` | `BRIDLE_MOB_CATCH_FAIL` | `USE_CASH_ITEM` |
| 90 / `0x05a` | `SESSION_VALUE` | `SPEND_SP` |
| 95 / `0x05f` | `FAMILY` | `GIVE_FAME` |
| 97 / `0x061` | `FAMILY_JOIN_REQUEST` | `CHAR_INFO_REQUEST` |
| 98 / `0x062` | `FAMILY_JOIN_REQUEST_RESULT` | `SPAWN_PET` |
| 106 / `0x06a` | `NOTIFY_MARRIAGE` | `REPORT` |
| 107 / `0x06b` | `NOTIFY_JOB_CHANGE` | `QUEST_ACTION` |
| 122 / `0x07a` | `SCRIPT_PROGRESS_MESSAGE` | `MESSENGER` |
| 123 / `0x07b` | `DATA_CRC_CHECK_FAILED` | `PLAYER_INTERACTION` |
| 124 / `0x07c` | `SKILL_MACROS` | `PARTY_OPERATION` |
| 125 / `0x07d` | `SET_FIELD` | `DENY_PARTY_REQUEST` |
| 126 / `0x07e` | `SET_ITC` | `GUILD_OPERATION` |
| 127 / `0x07f` | `SET_CASH_SHOP` | `DENY_GUILD_REQUEST` |
| 128 / `0x080` | `SET_BACK_EFFECT` | `ADMIN_COMMAND` |
| 135 / `0x087` | `WHISPER` | `CHANGE_KEYMAP` |
| 136 / `0x088` | `SPOUSE_CHAT` | `RPS_ACTION` |
| 138 / `0x08a` | `FIELD_EFFECT` | `WEDDING_ACTION` |
| 144 / `0x090` | `ADMIN_RESULT` | `DENY_ALLIANCE_REQUEST` |
| 145 / `0x091` | `OX_QUIZ` | `OPEN_FAMILY_PEDIGREE` |
| 146 / `0x092` | `GMEVENT_INSTRUCTIONS` | `OPEN_FAMILY` |
| 147 / `0x093` | `CLOCK` | `ADD_FAMILY` |
| 148 / `0x094` | `CONTI_MOVE` | `SEPARATE_FAMILY_BY_SENIOR` |
| 149 / `0x095` | `CONTI_STATE` | `SEPARATE_FAMILY_BY_JUNIOR` |
| 155 / `0x09b` | `ARIANT_ARENA_SHOW_RESULT` | `BBS_OPERATION` |
| 168 / `0x0a8` | `SPAWN_PET` | `PET_CHAT_SEND` |
| 170 / `0x0aa` | `MOVE_PET` | `PET_LOOT` |
| 188 / `0x0bc` | `ATTACKED_MAGIC` | `MOVE_MONSTER` |
| 197 / `0x0c5` | `UPDATE_CHARLOOK` | `NPC_ACTION` |
| 202 / `0x0ca` | `GUILD_NAME_CHANGED` | `PICKUP_ITEM` |
| 205 / `0x0cd` | `CANCEL_CHAIR` | `DAMAGE_REACTOR` |
| 207 / `0x0cf` | `DOJO_WARP_UP` | `PLAYER_MAP_TRANSFER` |
| 220 / `0x0dc` | `OPEN_UI` | `PARTY_SEARCH_REGISTER` |
| 222 / `0x0de` | `DISABLE_UI` | `PARTY_SEARCH_START` |
| 223 / `0x0df` | `SPAWN_GUIDE` | `PLAYER_UPDATE` |

## Legacy incoming behavior coverage

| Incoming behavior | C++ evidence | Current Rust status |
| --- | --- | --- |
| Successful `RELOG_RESPONSE` returns to login server. | [LoginHandlers.cpp](../OpenStory/src/Net/Handlers/LoginHandlers.cpp#L322)–[LoginHandlers.cpp](../OpenStory/src/Net/Handlers/LoginHandlers.cpp#L335) | **IU** — bounded parser in [`misc.rs`](crates/openstory-protocol/src/misc.rs#L65), routed by [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L31), reconnect transition in [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2783). |
| GM flag and hide-toggle update on `ADMIN_RESULT`. | [MiscHandlers.cpp](../OpenStory/src/Net/Handlers/MiscHandlers.cpp#L102)–[MiscHandlers.cpp](../OpenStory/src/Net/Handlers/MiscHandlers.cpp#L122) | **IU** — [`misc.rs`](crates/openstory-protocol/src/misc.rs#L105) parser and [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2834) state application. |
| Session value `energy` updates field state. | [MiscHandlers.cpp](../OpenStory/src/Net/Handlers/MiscHandlers.cpp#L161)–[MiscHandlers.cpp](../OpenStory/src/Net/Handlers/MiscHandlers.cpp#L179) | **IU** — [`misc.rs`](crates/openstory-protocol/src/misc.rs#L88) parser and [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2816) state application. |
| Memo sent/failure result notification. | [MessagingHandlers.cpp](../OpenStory/src/Net/Handlers/MessagingHandlers.cpp#L618)–[MessagingHandlers.cpp](../OpenStory/src/Net/Handlers/MessagingHandlers.cpp#L641) | **IU** — [`misc.rs`](crates/openstory-protocol/src/misc.rs#L72) parses sent/failure/other modes; [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2804) applies notifications. |
| Police/CRC rejection notice. | [ShopStorageHandlers.cpp](../OpenStory/src/Net/Handlers/ShopStorageHandlers.cpp#L1090)–[ShopStorageHandlers.cpp](../OpenStory/src/Net/Handlers/ShopStorageHandlers.cpp#L1104) | **IU** — [`misc.rs`](crates/openstory-protocol/src/misc.rs#L97) parser and [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2827) notice handling. |
| Duey package list/delivery result variants. | [ShopStorageHandlers.cpp](../OpenStory/src/Net/Handlers/ShopStorageHandlers.cpp#L866)–[ShopStorageHandlers.cpp](../OpenStory/src/Net/Handlers/ShopStorageHandlers.cpp#L920) | **IU** — bounded operation parser in [`misc.rs`](crates/openstory-protocol/src/misc.rs#L129); notifications/state messages in [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2857). |
| Fredrick message and retrieval result. | [ShopStorageHandlers.cpp](../OpenStory/src/Net/Handlers/ShopStorageHandlers.cpp#L1210)–[ShopStorageHandlers.cpp](../OpenStory/src/Net/Handlers/ShopStorageHandlers.cpp#L1224) | **IU** — parsers in [`misc.rs`](crates/openstory-protocol/src/misc.rs#L119)–[`misc.rs`](crates/openstory-protocol/src/misc.rs#L120), dispatched in [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2849). |
| GM event instruction notice. | [EventHandlers.cpp](../OpenStory/src/Net/Handlers/EventHandlers.cpp#L272)–[EventHandlers.cpp](../OpenStory/src/Net/Handlers/EventHandlers.cpp#L276) | **IU** — [`misc.rs`](crates/openstory-protocol/src/misc.rs#L118) parser and [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2845) notification. |
| Gender confirmation. | [LoginHandlers.cpp](../OpenStory/src/Net/Handlers/LoginHandlers.cpp#L354)–[LoginHandlers.cpp](../OpenStory/src/Net/Handlers/LoginHandlers.cpp#L360) | **O** — Rust parses and consumes it in [`misc.rs`](crates/openstory-protocol/src/misc.rs#L87) and [`client_packets.rs`](crates/openstory-client/src/native/client_packets.rs#L2803); like C++, it makes no state change because gender is already established by login. |
| Summon attack target/damage tuple. | [MapObjectHandlers.cpp](../OpenStory/src/Net/Handlers/MapObjectHandlers.cpp#L733)–[MapObjectHandlers.cpp](../OpenStory/src/Net/Handlers/MapObjectHandlers.cpp#L748) | **O** — Rust bounded-parses it; like C++, visual damage is driven by mob HP updates. |
| Summon damage notification. | [MapObjectHandlers.cpp](../OpenStory/src/Net/Handlers/MapObjectHandlers.cpp#L750)–[MapObjectHandlers.cpp](../OpenStory/src/Net/Handlers/MapObjectHandlers.cpp#L758) | **O** — Rust bounded-parses it and preserves the original `Summon::apply_damage` visual no-op. |
| Monster Life farm/placement/move/remove/error and Monster Battle unit/team/error modes. | [PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L354)–[PacketSwitch.cpp](../OpenStory/src/Net/PacketSwitch.cpp#L368), [MonsterHandlers.cpp](../OpenStory/src/Net/Handlers/MonsterHandlers.cpp#L94)–[MonsterHandlers.cpp](../OpenStory/src/Net/Handlers/MonsterHandlers.cpp#L183) | **SC** — opcodes are custom/noncanonical; implement only against supplied server scripts and exact contract. |

## Verification and architecture gaps

- The plan requires native end-to-end evidence for login through field entry, channel transfer, movement/combat, inventory/use, quests, social flows, shop/storage transactions, and failure/disconnect paths. This inventory found no such result in the current notes; all implemented runtime rows remain IU.
- The inventory is exhaustive at registration/opcode granularity, but it is not a field-by-field semantic diff for every branch of multipurpose handlers. Full branch audits remain broad for quest updates ([`QuestHandlers.cpp`](../OpenStory/src/Net/Handlers/QuestHandlers.cpp#L33)), inventory modifications ([`InventoryHandlers.cpp`](../OpenStory/src/Net/Handlers/InventoryHandlers.cpp#L50)), party/guild/BBS operations ([`SocialHandlers.cpp`](../OpenStory/src/Net/Handlers/SocialHandlers.cpp#L319), [`SocialHandlers.cpp`](../OpenStory/src/Net/Handlers/SocialHandlers.cpp#L644), [`SocialHandlers.cpp`](../OpenStory/src/Net/Handlers/SocialHandlers.cpp#L956)), player interaction/storage ([`ShopStorageHandlers.cpp`](../OpenStory/src/Net/Handlers/ShopStorageHandlers.cpp#L105), [`ShopStorageHandlers.cpp`](../OpenStory/src/Net/Handlers/ShopStorageHandlers.cpp#L275)), and Cash Shop operations ([`CashShopHandlers.cpp`](../OpenStory/src/Net/Handlers/CashShopHandlers.cpp#L71)). Their IU labels mean a corresponding Rust route exists, not that every original payload variant is proven equivalent.
- Focused local tests now cover MTS operation variants and malformed counts, custom bot snapshots/actions, bounded summon and miscellaneous packets, exact-byte legacy requests, core weapon/skill rules, buff transitions, and movement. See [`mts.rs`](crates/openstory-protocol/src/mts.rs#L322), [`bot_inventory.rs`](crates/openstory-protocol/src/bot_inventory.rs#L190), [`misc.rs`](crates/openstory-protocol/src/misc.rs#L170), [`legacy_requests.rs`](crates/openstory-protocol/src/legacy_requests.rs#L158), [`gameplay.rs`](crates/openstory-protocol/src/gameplay.rs#L4348), [`combat.rs`](crates/openstory-core/src/combat.rs#L70), and [`buffs.rs`](crates/openstory-core/src/buffs.rs#L126). Those fixtures do not assert server interoperability.
- Combat multipliers, fallback weapon class, stance, skill-use restrictions, and active character buff state now live in platform-independent `openstory-core` modules; the native client converts protocol buff masks at its boundary and keeps its existing UI refresh hooks. This is incremental migration, not complete backend ownership. Character, inventory, quest, and actor state remain substantially in `openstory-client` ([`lib.rs`](crates/openstory-core/src/lib.rs#L5)).
- Keep the NPC token-for-mesos purchase disabled until the supplied server fixes or verifies its item-removal contract, as recorded in [`PORT_PLAN.md`](PORT_PLAN.md#L769)–[`PORT_PLAN.md`](PORT_PLAN.md#L772).
- The implementation plan remains open pending native end-to-end flow evidence, field-by-field audits of the multipurpose handlers, and resolution or explicit deferral of server-dependent behavior. The custom Monster Life/Battle and bot inventory paths remain server-contract dependent; the NPC token-for-mesos purchase remains disabled. Required checks and live flows are listed in [`BACKEND_PLAN.md`](BACKEND_PLAN.md#L51)–[`BACKEND_PLAN.md`](BACKEND_PLAN.md#L68).
