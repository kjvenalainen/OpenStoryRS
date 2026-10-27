//! v83 gameplay packet builders.

use crate::login::{expect_opcode, read_character_look};
use crate::{CharacterLook, PacketError, PacketReader, PacketWriter};

pub const OPCODE_MOVE_PLAYER: u16 = 41;
/// Requests a channel change; the response is a channel transfer packet (opcode 16).
pub const OPCODE_CHANGE_CHANNEL: u16 = 39;
pub const OPCODE_FIELD_EFFECT: u16 = 138;
pub const OPCODE_BLOW_WEATHER: u16 = 142;
pub const OPCODE_SHOW_ITEM_GAIN_INCHAT: u16 = 206;
pub const OPCODE_GENERAL_CHAT: u16 = 49;
pub const OPCODE_SPEND_ABILITY_POINT: u16 = 87;
pub const OPCODE_HEAL_OVER_TIME: u16 = 89;
pub const OPCODE_MULTI_CHAT_SEND: u16 = 119;
pub const OPCODE_WHISPER_SEND: u16 = 120;
pub const OPCODE_PARTY_OPERATION: u16 = 62;
pub const OPCODE_BUDDY_LIST: u16 = 63;
pub const OPCODE_MULTI_CHAT_RECEIVED: u16 = 134;
pub const OPCODE_WHISPER_RECEIVED: u16 = 135;
pub const OPCODE_SEND_TV: u16 = 341;
pub const OPCODE_REMOVE_TV: u16 = 342;
pub const OPCODE_ENABLE_TV: u16 = 343;

const MAPLE_TV_LINE_COUNT: usize = 5;
const MAX_MAPLE_TV_LINE_BYTES: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapleTvBroadcast {
    pub has_partner: bool,
    pub type_id: u8,
    pub sender_look: CharacterLook,
    pub partner_look: Option<CharacterLook>,
    pub lines: Vec<String>,
    pub remaining_ms: u32,
}

/// Parses the v83 world-wide Maple TV broadcast packet. The packet carries one
/// character look (and a second for partnered broadcasts), exactly five message
/// strings, and an optional signed remaining-time value in milliseconds.
pub fn parse_send_tv(bytes: &[u8]) -> Result<MapleTvBroadcast, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SEND_TV)?;
    let has_partner = packet.read_u8()? == 3;
    let type_id = packet.read_u8()?;
    let sender_look = read_character_look(&mut packet)?;
    let partner_look = if has_partner {
        Some(read_character_look(&mut packet)?)
    } else {
        None
    };

    let mut lines = Vec::with_capacity(MAPLE_TV_LINE_COUNT);
    for _ in 0..MAPLE_TV_LINE_COUNT {
        let line = packet.read_string()?;
        if line.len() > MAX_MAPLE_TV_LINE_BYTES {
            return Err(PacketError::StringTooLong(line.len()));
        }
        lines.push(line);
    }

    let remaining_ms = if packet.remaining() >= 4 {
        u32::try_from(packet.read_i32()?.max(0)).unwrap_or(0)
    } else {
        15_000
    };
    Ok(MapleTvBroadcast {
        has_partner,
        type_id,
        sender_look,
        partner_look,
        lines,
        remaining_ms: if remaining_ms == 0 {
            15_000
        } else {
            remaining_ms
        },
    })
}

pub fn parse_remove_tv(bytes: &[u8]) -> Result<(), PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_REMOVE_TV)
}

/// The v83 client currently has no visual action for ENABLE_TV, but consumes
/// its full payload so malformed/truncated packets are still detected.
pub fn parse_enable_tv(bytes: &[u8]) -> Result<(i32, u8), PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_ENABLE_TV)?;
    Ok((packet.read_i32()?, packet.read_u8()?))
}

/// Builds the v83 in-game channel switch request: `channel: u8`, `tick: i32`.
/// The legacy client sends a zero tick for this transition.
pub fn change_channel_packet(channel: u8) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_CHANGE_CHANNEL);
    packet.write_u8(channel);
    packet.write_i32(0);
    packet.into_bytes()
}
const MAX_PET_SLOTS: u8 = 3;
const MAX_CHARACTER_INFO_PETS: usize = 3;
const MAX_CHARACTER_WISHLIST: usize = 10;
const MAX_CHARACTER_MEDAL_QUESTS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbilityStat {
    Hp,
    Mp,
    Strength,
    Dexterity,
    Intelligence,
    Luck,
}

impl AbilityStat {
    const fn maple_stat_mask(self) -> u32 {
        match self {
            Self::Hp => 0x0000_0400,
            Self::Mp => 0x0000_1000,
            Self::Strength => 0x0000_0040,
            Self::Dexterity => 0x0000_0080,
            Self::Intelligence => 0x0000_0100,
            Self::Luck => 0x0000_0200,
        }
    }
}

/// Builds a v83 `SPEND_AP` request for one server-authoritative stat point.
pub fn spend_ability_point_packet(stat: AbilityStat, client_tick: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_SPEND_ABILITY_POINT);
    packet.write_i32(client_tick);
    packet.write_u32(stat.maple_stat_mask());
    packet.into_bytes()
}
pub const OPCODE_PARTY_OPERATION_SEND: u16 = 124;
pub const OPCODE_BUDDY_LIST_MODIFY: u16 = 130;
pub const OPCODE_DENY_PARTY_INVITE: u16 = 125;
pub const OPCODE_CHANGE_MAP: u16 = 38;
pub const OPCODE_SPAWN_CHARACTER: u16 = 160;
pub const OPCODE_REMOVE_CHARACTER: u16 = 161;
pub const OPCODE_CHAT_RECEIVED: u16 = 162;
pub const OPCODE_CHARACTER_MOVED: u16 = 185;
pub const OPCODE_GIVE_BUFF: u16 = 32;
pub const OPCODE_CANCEL_BUFF: u16 = 33;
pub const OPCODE_GIVE_FOREIGN_BUFF: u16 = 199;
pub const OPCODE_CANCEL_FOREIGN_BUFF: u16 = 200;
pub const OPCODE_DAMAGE_PLAYER: u16 = 192;
pub const OPCODE_TAKE_DAMAGE: u16 = 48;
pub const OPCODE_ATTACKED_CLOSE: u16 = 186;
pub const OPCODE_ATTACKED_RANGED: u16 = 187;
pub const OPCODE_ATTACKED_MAGIC: u16 = 188;
pub const OPCODE_UPDATE_CHARACTER_LOOK: u16 = 197;
pub const OPCODE_SHOW_ITEM_EFFECT: u16 = 194;
pub const OPCODE_UPDATE_CHARACTER_BOX: u16 = 165;
pub const OPCODE_CHARACTER_INFO: u16 = 61;
pub const OPCODE_CHARACTER_INFO_REQUEST: u16 = 97;
pub const OPCODE_FAME_RESPONSE: u16 = 38;
pub const OPCODE_GIVE_FAME: u16 = 95;
pub const OPCODE_SKILL_LEARN_ITEM_RESULT: u16 = 51;
pub const OPCODE_FORCED_STAT_SET: u16 = 34;
pub const OPCODE_RECALCULATE_STATS: u16 = 35;
pub const OPCODE_FACIAL_EXPRESSION: u16 = 193;
pub const OPCODE_FACE_EXPRESSION: u16 = 51;
pub const OPCODE_SHOW_FOREIGN_EFFECT: u16 = 198;
pub const OPCODE_SHOW_CHAIR: u16 = 196;
pub const OPCODE_CANCEL_CHAIR_BROADCAST: u16 = 205;
pub const OPCODE_SKILL_EFFECT: u16 = 190;
pub const OPCODE_CANCEL_SKILL_EFFECT: u16 = 191;
pub const OPCODE_SPAWN_NPC: u16 = 257;
pub const OPCODE_REMOVE_NPC: u16 = 258;
pub const OPCODE_SPAWN_NPC_CONTROLLER: u16 = 259;
pub const OPCODE_SPAWN_MOB: u16 = 236;
pub const OPCODE_KILL_MOB: u16 = 237;
pub const OPCODE_SPAWN_MOB_CONTROLLER: u16 = 238;
pub const OPCODE_MOB_MOVED: u16 = 239;
pub const OPCODE_MOVE_MOB_RESPONSE: u16 = 240;
pub const OPCODE_APPLY_MOB_STATUS: u16 = 242;
pub const OPCODE_CANCEL_MOB_STATUS: u16 = 243;
pub const OPCODE_DAMAGE_MOB: u16 = 246;
pub const OPCODE_SHOW_MOB_HP: u16 = 250;
pub const OPCODE_SPAWN_SUMMON: u16 = 175;
pub const OPCODE_REMOVE_SUMMON: u16 = 176;
pub const OPCODE_MOVE_SUMMON: u16 = 177;
pub const OPCODE_SPAWN_PET_RECEIVED: u16 = 168;
pub const OPCODE_MOVE_PET: u16 = 170;
pub const OPCODE_PET_LOOT: u16 = 170;
pub const OPCODE_PET_CHAT_RECEIVED: u16 = 171;
pub const OPCODE_PET_COMMAND_RECEIVED: u16 = 174;
pub const OPCODE_SUMMON_ATTACK: u16 = 178;
pub const OPCODE_DAMAGE_SUMMON: u16 = 179;
pub const OPCODE_SUMMON_SKILL: u16 = 180;
pub const OPCODE_SPAWN_MIST: u16 = 273;
pub const OPCODE_REMOVE_MIST: u16 = 274;
pub const OPCODE_MOVE_MOB: u16 = 188;
pub const OPCODE_PICKUP_ITEM: u16 = 202;
pub const OPCODE_DROP_LOOT: u16 = 268;
pub const OPCODE_REMOVE_LOOT: u16 = 269;
pub const OPCODE_HIT_REACTOR: u16 = 277;
pub const OPCODE_SPAWN_REACTOR: u16 = 279;
pub const OPCODE_REMOVE_REACTOR: u16 = 280;
pub const OPCODE_DAMAGE_REACTOR: u16 = 205;
pub const OPCODE_UPDATE_PARTY_MEMBER_HP: u16 = 201;
pub const OPCODE_TALK_TO_NPC: u16 = 58;
pub const OPCODE_NPC_TALK_MORE: u16 = 60;
pub const OPCODE_NPC_SHOP_ACTION: u16 = 61;
pub const OPCODE_NPC_ACTION: u16 = 260;
pub const OPCODE_SET_NPC_SCRIPTABLE: u16 = 263;
pub const OPCODE_FIELD_OBSTACLE_ONE: u16 = 139;
pub const OPCODE_FIELD_OBSTACLE_ONOFF: u16 = 140;
pub const OPCODE_NPC_DIALOGUE: u16 = 304;
pub const OPCODE_OPEN_NPC_SHOP: u16 = 305;
pub const OPCODE_CONFIRM_NPC_SHOP_TRANSACTION: u16 = 306;
pub const OPCODE_CHARACTER_STAT_UPDATE: u16 = crate::login::OPCODE_CHARACTER_STAT_UPDATE;
const BUFFSTAT_BATTLESHIP: u64 = 0x0000_000A_0000_0040;
const SKILL_MESO_EXPLOSION: i32 = 4_211_006;
const SKILL_RAPID_FIRE: i32 = 5_221_004;
const SKILL_PIERCE: i32 = 5_721_001;
const SKILL_PIERCE_ARROW: i32 = 4_341_002;
const SKILL_RAPID_FIRE_CROSSBOW: i32 = 4_341_003;

pub const OPCODE_CLOSE_ATTACK: u16 = 44;
pub const OPCODE_RANGED_ATTACK: u16 = 45;
pub const OPCODE_MAGIC_ATTACK: u16 = 46;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackType {
    Close,
    Ranged,
    Magic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttackDamageTarget {
    pub object_id: i32,
    pub damage_lines: Vec<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerAttackPacket {
    pub attack_type: AttackType,
    pub hit_count: u8,
    pub skill_id: i32,
    pub charge: i32,
    pub display: u8,
    pub to_left: bool,
    pub stance: u8,
    /// Signed attack-speed modifier encoded as one byte by the v83 protocol.
    pub speed: i8,
    pub targets: Vec<AttackDamageTarget>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteCharacterSpawn {
    pub id: i32,
    pub name: String,
    pub level: u8,
    pub job: u16,
    pub look: CharacterLook,
    pub item_effect_id: i32,
    pub x: i16,
    pub y: i16,
    pub stance: u8,
    pub pets: Vec<MapPetSpawn>,
    pub mini_room: Option<CharacterMiniRoom>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterMiniRoom {
    pub room_type: i8,
    pub object_id: i32,
    pub description: String,
    pub skin: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteCharacterMiniRoomUpdate {
    pub character_id: i32,
    pub mini_room: Option<CharacterMiniRoom>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoteCharacterItemEffect {
    pub character_id: i32,
    /// Zero clears the persistent `Effect.nx/ItemEff.img` aura.
    pub item_id: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterInfoPet {
    pub item_id: i32,
    pub name: String,
    pub level: u8,
    pub closeness: i16,
    pub fullness: u8,
    pub label_ring_item_id: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterMountInfo {
    pub level: i32,
    pub experience: i32,
    pub tiredness: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterMedalInfo {
    pub item_id: i32,
    pub quest_ids: Vec<i16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterMonsterBookInfo {
    pub level: i32,
    pub normal_cards: i32,
    pub special_cards: i32,
    pub total_cards: i32,
    pub cover_item_id: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterInfoReply {
    pub character_id: i32,
    pub level: u8,
    pub job: u16,
    pub fame: i16,
    pub married: bool,
    pub guild: String,
    pub alliance: String,
    pub pets: Vec<CharacterInfoPet>,
    pub mount: Option<CharacterMountInfo>,
    pub wishlist: Vec<i32>,
    pub monster_book: CharacterMonsterBookInfo,
    pub medal: Option<CharacterMedalInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FameResponse {
    Success {
        character_name: String,
        raised: bool,
        fame: i16,
    },
    CannotFameSelf,
    AlreadyFamed,
    TargetLevelTooLow,
    PlayerLevelTooLow,
    Unknown(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillLearnItemResult {
    pub character_id: i32,
    pub marker: u8,
    pub skill_id: i32,
    pub max_level: i32,
    pub can_use: bool,
    pub success: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ForcedStatSet {
    pub strength: Option<u16>,
    pub dexterity: Option<u16>,
    pub intelligence: Option<u16>,
    pub luck: Option<u16>,
    pub weapon_attack: Option<i16>,
    pub weapon_defense: Option<i16>,
    pub magic_attack: Option<i16>,
    pub magic_defense: Option<i16>,
    pub accuracy: Option<i16>,
    pub avoidability: Option<i16>,
    pub speed: Option<u8>,
    pub jump: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapPetSpawn {
    pub character_id: i32,
    pub slot: u8,
    pub item_id: i32,
    pub name: String,
    pub unique_id: i32,
    pub x: i16,
    pub y: i16,
    pub stance: u8,
    pub foothold: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PetSpawnEvent {
    Spawn(MapPetSpawn),
    Remove {
        character_id: i32,
        slot: u8,
        hunger: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapPetMovement {
    pub character_id: i32,
    pub slot: u8,
    pub pet_id: i32,
    pub fragments: Vec<MovementFragment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PetChatUpdate {
    pub character_id: i32,
    pub slot: u8,
    pub act: i8,
    pub text: String,
    pub balloon_type: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PetCommandUpdate {
    pub character_id: i32,
    pub slot: u8,
    pub interaction_type: i8,
    pub animation: i8,
    pub talk: bool,
    pub balloon_type: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatMessage {
    pub character_id: i32,
    pub gm: bool,
    pub message: String,
    pub chat_type: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultiChatType {
    Buddy,
    Party,
    Guild,
    Alliance,
}

impl MultiChatType {
    const fn wire_value(self) -> i8 {
        match self {
            Self::Buddy => 0,
            Self::Party => 1,
            Self::Guild => 2,
            Self::Alliance => 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiChatMessage {
    pub chat_type: i8,
    pub from: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartyMember {
    pub character_id: i32,
    pub name: String,
    pub job: i32,
    pub level: i32,
    /// A nonnegative value is the server channel; a negative value means offline.
    pub channel: i32,
    pub map_id: i32,
    pub hp: Option<(i32, i32)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartySnapshot {
    pub party_id: i32,
    pub leader_id: i32,
    pub members: Vec<PartyMember>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PartyEvent {
    Invite {
        party_id: i32,
        from: String,
    },
    Created {
        party_id: i32,
    },
    Snapshot(PartySnapshot),
    Removed {
        party_id: i32,
        character_id: i32,
        disbanded: bool,
        expelled: bool,
        name: String,
        snapshot: Option<PartySnapshot>,
    },
    MemberJoined {
        party_id: i32,
        name: String,
        snapshot: Option<PartySnapshot>,
    },
    LeaderChanged {
        character_id: i32,
    },
    Status {
        code: i8,
        name: Option<String>,
    },
    DoorUpdated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartyMemberHp {
    pub character_id: i32,
    pub hp: i32,
    pub max_hp: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuddyEntry {
    pub character_id: i32,
    pub name: String,
    pub status: i8,
    /// A nonnegative value is the zero-based server channel; negative is offline.
    pub channel: i32,
    pub group: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuddyEvent {
    FullList(Vec<BuddyEntry>),
    Invite { character_id: i32, name: String },
    Channel { character_id: i32, channel: i32 },
    Capacity(u8),
    Error(i8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WhisperMessage {
    Incoming {
        from: String,
        channel: i8,
        from_admin: bool,
        message: String,
    },
    FindResult {
        name: String,
        result: i8,
    },
    DeliveryResult {
        name: String,
        result: i8,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteCharacterMovement {
    pub id: i32,
    pub fragments: Vec<MovementFragment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoteFacialExpression {
    pub character_id: i32,
    pub expression: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteAttackType {
    Close,
    Ranged,
    Magic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoteAttackDamage {
    pub damage: i32,
    pub critical: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemotePlayerDamage {
    pub character_id: i32,
    pub skill: i8,
    pub damage: i32,
    pub monster_id: Option<i32>,
    pub direction: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteAttackTarget {
    pub object_id: i32,
    pub damage_lines: Vec<RemoteAttackDamage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteCharacterAttack {
    pub attacker_id: i32,
    pub attack_type: RemoteAttackType,
    pub skill_level: u8,
    pub skill_id: i32,
    pub display: u8,
    pub to_left: bool,
    pub stance: u8,
    pub speed: i8,
    pub bullet_id: i32,
    pub targets: Vec<RemoteAttackTarget>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForeignSkillEffect {
    pub character_id: i32,
    pub skill_id: i32,
    pub level: i8,
    pub flags: i8,
    pub speed: i8,
    pub direction: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForeignCharacterEffect {
    pub character_id: i32,
    pub effect_type: i8,
    /// Maker result: zero is success, nonzero is failure. Other effects use zero.
    pub variant: i8,
    /// Positive recovery amount for foreign character effect type 10.
    pub heal_amount: Option<i8>,
    pub skill_id: Option<i32>,
    pub level: i8,
    pub direction: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemGainInChat {
    Item { item_id: i32, quantity: i32 },
    MonsterBookCard,
    SkillUse { skill_id: i32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteCharacterLookUpdate {
    pub id: i32,
    pub look: CharacterLook,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapNpcSpawn {
    pub object_id: i32,
    pub npc_id: i32,
    pub x: i16,
    pub y: i16,
    pub flipped: bool,
    pub foothold: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapChairShow {
    pub character_id: i32,
    pub item_id: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapChairCancel {
    pub character_id: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapDropSpawn {
    pub object_id: i32,
    pub item_id: i32,
    pub meso: bool,
    pub owner_id: i32,
    pub pickup_type: i8,
    pub destination_x: i16,
    pub destination_y: i16,
    pub source_x: i16,
    pub source_y: i16,
    pub mode: i8,
    pub player_drop: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapDropRemove {
    pub object_id: i32,
    pub mode: i8,
    pub looter_id: Option<i32>,
    pub looted_by_pet: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapReactorSpawn {
    pub object_id: i32,
    pub reactor_id: i32,
    pub state: i8,
    pub x: i16,
    pub y: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapReactorState {
    pub object_id: i32,
    pub state: i8,
    pub x: i16,
    pub y: i16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NpcControllerUpdate {
    Remove { object_id: i32 },
    Spawn(MapNpcSpawn),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcDialogue {
    pub npc_id: i32,
    pub message_type: i8,
    pub speaker: i8,
    pub text: String,
    pub number_bounds: Option<(i32, i32, i32)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcShopItem {
    pub item_id: i32,
    pub price: i32,
    pub pitch: i32,
    pub time: i32,
    pub charge_price: i16,
    pub buyable: i16,
    pub rechargeable: bool,
    pub slot: i16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcShop {
    pub npc_id: i32,
    pub items: Vec<NpcShopItem>,
    pub currency_item_id: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NpcAction {
    pub object_id: i32,
    pub action: Option<i8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldObstacleToggle {
    pub name: String,
    pub mode: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldEffect {
    ScreenAnimation(String),
    Sound(String),
    Music(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlowWeatherUpdate {
    /// v83 uses 0 to clear weather, 1 for snow, and 2 for rain.
    pub kind: i8,
    /// Cash weather item ID. Zero means there is no weather item.
    pub item_id: i32,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CharacterStatChange {
    Skin(u8),
    Face(i32),
    Hair(i32),
    Level(u8),
    Job(u16),
    Strength(u16),
    Dexterity(u16),
    Intelligence(u16),
    Luck(u16),
    Hp(u16),
    MaxHp(u16),
    Mp(u16),
    MaxMp(u16),
    AbilityPoints(u16),
    SkillPoints(u16),
    SkillPointTable(Vec<(u8, u8)>),
    Experience(i32),
    Fame(u16),
    Meso(i32),
    Pet(i32),
    GachaExperience(i32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterStatUpdate {
    pub enable_actions: bool,
    pub changes: Vec<CharacterStatChange>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuffMaskGroup {
    First,
    Second,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BuffStatKey {
    pub group: BuffMaskGroup,
    pub flag: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterBuffEntry {
    pub key: BuffStatKey,
    pub value: i16,
    pub skill_id: i32,
    pub duration_ms: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterBuffUpdate {
    pub entries: Vec<CharacterBuffEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterBuffCancel {
    pub first_mask: u64,
    pub second_mask: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForeignMobDiseaseEffect {
    pub character_id: i32,
    pub skill_type: u16,
    pub skill_level: u16,
}

/// Reads the mob-skill identity from a disease-only `GIVE_FOREIGN_BUFF`.
/// Cosmic encodes the disease source as two shorts (mob skill type, level);
/// Poison also carries its damage value first. Its special persisted Slow
/// packet uses the legacy first-mask bit instead of Disease::SLOW's usual bit.
/// Other foreign buffs have different payload layouts, so they return `None`.
pub fn parse_foreign_mob_disease_effect(
    bytes: &[u8],
) -> Result<Option<ForeignMobDiseaseEffect>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_GIVE_FOREIGN_BUFF)?;
    let character_id = packet.read_i32()?;
    let first_mask = packet.read_u64()?;
    let second_mask = packet.read_u64()?;

    let slow_first_mask = 0x0000_0800_0000_0000;
    let slow_first_only = first_mask == slow_first_mask && second_mask == 0;
    let disease_mask = 0xC01E_0000_0008_4081_u64;
    if !slow_first_only && (first_mask != 0 || second_mask == 0 || second_mask & !disease_mask != 0)
    {
        return Ok(None);
    }

    let mut mask = if slow_first_only { 1 } else { second_mask };
    let mut effect = None;
    while mask != 0 {
        let flag = 1_u64 << mask.trailing_zeros();
        mask &= !flag;

        // Disease::POISON (0x4000000000000) serializes one extra short value.
        if !slow_first_only && flag == 0x0004_0000_0000_0000 {
            let _damage = packet.read_i16()?;
        }
        let skill_type = packet.read_u16()?;
        let skill_level = packet.read_u16()?;
        let expected_skill_type = match flag {
            0x0000_0000_0000_0001 => 126, // Slow.
            0x0000_0000_0000_0080 => 128, // Seduce.
            0x0000_0000_0000_4000 => 133, // Zombify/Undead.
            0x0000_0000_0008_0000 => 132, // Confuse.
            0x0002_0000_0000_0000 => 123, // Stun.
            0x0004_0000_0000_0000 => 125, // Poison.
            0x0008_0000_0000_0000 => 120, // Seal.
            0x0010_0000_0000_0000 => 121, // Darkness.
            0x4000_0000_0000_0000 => 122, // Weaken.
            0x8000_0000_0000_0000 => 124, // Curse.
            _ if slow_first_only => 126,
            _ => 0,
        };
        if skill_type == expected_skill_type && skill_level != 0 {
            effect = Some(ForeignMobDiseaseEffect {
                character_id,
                skill_type,
                skill_level,
            });
        }
    }
    if packet.remaining() != 4 || packet.read_i16()? != 0 || packet.read_i16()? != 900 {
        return Ok(None);
    }
    Ok(effect)
}

pub fn parse_character_buff_update(bytes: &[u8]) -> Result<CharacterBuffUpdate, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_GIVE_BUFF)?;
    let first_mask = packet.read_u64()?;
    let second_mask = packet.read_u64()?;
    let mut entries =
        Vec::with_capacity(first_mask.count_ones() as usize + second_mask.count_ones() as usize);

    // Battleship is represented by three overlapping server bits, but carries
    // one record. The legacy client treats this exact mask as a special case.
    if second_mask == BUFFSTAT_BATTLESHIP {
        entries.push(read_character_buff_entry(
            &mut packet,
            BuffMaskGroup::Second,
            BUFFSTAT_BATTLESHIP,
        )?);
    } else {
        read_character_buff_entries(&mut packet, BuffMaskGroup::First, first_mask, &mut entries)?;
        read_character_buff_entries(
            &mut packet,
            BuffMaskGroup::Second,
            second_mask,
            &mut entries,
        )?;
    }

    Ok(CharacterBuffUpdate { entries })
}

pub fn parse_character_buff_cancel(bytes: &[u8]) -> Result<CharacterBuffCancel, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_CANCEL_BUFF)?;
    Ok(CharacterBuffCancel {
        first_mask: packet.read_u64()?,
        second_mask: packet.read_u64()?,
    })
}

fn read_character_buff_entries(
    packet: &mut PacketReader<'_>,
    group: BuffMaskGroup,
    mut mask: u64,
    entries: &mut Vec<CharacterBuffEntry>,
) -> Result<(), PacketError> {
    // Consume in bit order so every set bit maps to exactly one 10-byte record,
    // including bits that currently have no gameplay effect in this port.
    while mask != 0 {
        let bit = 1_u64 << mask.trailing_zeros();
        entries.push(read_character_buff_entry(packet, group, bit)?);
        mask &= !bit;
    }
    Ok(())
}

fn read_character_buff_entry(
    packet: &mut PacketReader<'_>,
    group: BuffMaskGroup,
    flag: u64,
) -> Result<CharacterBuffEntry, PacketError> {
    Ok(CharacterBuffEntry {
        key: BuffStatKey { group, flag },
        value: packet.read_i16()?,
        skill_id: packet.read_i32()?,
        duration_ms: packet.read_i32()?,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapMobSpawn {
    pub object_id: i32,
    pub mob_id: i32,
    pub control_mode: i8,
    pub x: i16,
    pub y: i16,
    pub stance: u8,
    pub foothold: u16,
    pub team: i8,
    pub is_new_spawn: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MobControllerUpdate {
    Control { object_id: i32, mode: i8 },
    Spawn(MapMobSpawn),
    NoSpawnData { object_id: i32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapMobMovement {
    pub object_id: i32,
    pub skill_possible: bool,
    pub action: i8,
    pub skill_id: u8,
    pub skill_level: u8,
    pub option: i16,
    pub start_x: i16,
    pub start_y: i16,
    pub fragments: Vec<MovementFragment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapMistSpawn {
    pub object_id: i32,
    pub mist_type: i32,
    pub owner_id: i32,
    pub skill_id: i32,
    pub skill_level: i8,
    pub delay: i16,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapSummonSpawn {
    pub owner_id: i32,
    pub object_id: i32,
    pub skill_id: i32,
    pub skill_level: i8,
    pub x: i16,
    pub y: i16,
    pub stance: u8,
    pub movement_type: u8,
    pub attacks: bool,
    pub animated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapSummonRemove {
    pub owner_id: i32,
    pub object_id: i32,
    pub animation: i8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapSummonMovement {
    pub owner_id: i32,
    pub object_id: i32,
    pub start_x: i16,
    pub start_y: i16,
    pub fragments: Vec<MovementFragment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SummonSkillUpdate {
    pub owner_id: i32,
    pub skill_id: i32,
    pub stance: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MobHpUpdate {
    pub object_id: i32,
    pub percentage: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MobDamage {
    pub object_id: i32,
    pub damage: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MobMovementResponse {
    pub object_id: i32,
    pub move_id: i16,
    pub use_skills: bool,
    pub current_mp: u16,
    pub skill_id: i8,
    pub skill_level: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MobStatusMaskGroup {
    First,
    Second,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MobStatusEntry {
    pub group: MobStatusMaskGroup,
    pub flag: u32,
    pub value: i16,
    pub skill_id: i32,
    pub duration: i16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MobStatusUpdate {
    pub object_id: i32,
    pub first_mask: u32,
    pub second_mask: u32,
    pub entries: Vec<MobStatusEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MobStatusCancel {
    pub object_id: i32,
    pub first_mask: u32,
    pub second_mask: u32,
}

pub fn parse_mob_hp(bytes: &[u8]) -> Result<MobHpUpdate, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SHOW_MOB_HP)?;
    Ok(MobHpUpdate {
        object_id: packet.read_i32()?,
        percentage: packet.read_i8()?,
    })
}

pub fn parse_drop_spawn(bytes: &[u8]) -> Result<MapDropSpawn, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_DROP_LOOT)?;
    let mode = packet.read_i8()?;
    let object_id = packet.read_i32()?;
    let meso = packet.read_bool()?;
    let item_id = packet.read_i32()?;
    let owner_id = packet.read_i32()?;
    let pickup_type = packet.read_i8()?;
    let (destination_x, destination_y) = packet.read_point_i16()?;
    packet.skip(4)?;
    let (source_x, source_y) = if mode != 2 {
        let point = packet.read_point_i16()?;
        packet.skip(2)?;
        point
    } else {
        (destination_x, destination_y)
    };
    if !meso {
        packet.skip(8)?;
    }
    let player_drop = !packet.read_bool()?;
    Ok(MapDropSpawn {
        object_id,
        item_id,
        meso,
        owner_id,
        pickup_type,
        destination_x,
        destination_y,
        source_x,
        source_y,
        mode,
        player_drop,
    })
}

pub fn parse_drop_remove(bytes: &[u8]) -> Result<MapDropRemove, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_REMOVE_LOOT)?;
    let mode = packet.read_i8()?;
    let object_id = packet.read_i32()?;
    let looter_id = if mode > 1 {
        Some(packet.read_i32()?)
    } else {
        None
    };
    let looted_by_pet = mode > 1 && !packet.is_empty() && packet.read_u8()? != 0;
    Ok(MapDropRemove {
        object_id,
        mode,
        looter_id,
        looted_by_pet,
    })
}

/// Parses the public-chat broadcast emitted by the v83 channel server.
pub fn parse_chat_received(bytes: &[u8]) -> Result<ChatMessage, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_CHAT_RECEIVED)?;
    Ok(ChatMessage {
        character_id: packet.read_i32()?,
        gm: packet.read_bool()?,
        message: packet.read_string()?,
        chat_type: packet.read_i8()?,
    })
}

/// Builds the legacy public-chat request packet.
pub fn general_chat_packet(message: &str, show: bool) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_GENERAL_CHAT);
    packet.write_string(message)?;
    packet.write_bool(show);
    Ok(packet.into_bytes())
}

/// Builds a buddy, party, guild, or alliance chat packet. An empty recipient list
/// is the normal v83 form; the server resolves current channel membership.
pub fn multi_chat_packet(
    chat_type: MultiChatType,
    recipients: &[i32],
    message: &str,
) -> Result<Vec<u8>, PacketError> {
    let count = u8::try_from(recipients.len())
        .map_err(|_| PacketError::TooManyChatRecipients(recipients.len()))?;
    let mut packet = PacketWriter::new(OPCODE_MULTI_CHAT_SEND);
    packet.write_i8(chat_type.wire_value());
    packet.write_u8(count);
    for recipient in recipients {
        packet.write_i32(*recipient);
    }
    packet.write_string(message)?;
    Ok(packet.into_bytes())
}

pub fn parse_multi_chat(bytes: &[u8]) -> Result<MultiChatMessage, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_MULTI_CHAT_RECEIVED)?;
    Ok(MultiChatMessage {
        chat_type: packet.read_i8()?,
        from: packet.read_string()?,
        message: packet.read_string()?,
    })
}

/// Parses incoming whispers and the two v83 whisper/find result forms.
/// Unknown mode bytes are safely ignored by returning `None`.
pub fn parse_whisper(bytes: &[u8]) -> Result<Option<WhisperMessage>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_WHISPER_RECEIVED)?;
    let mode = packet.read_i8()?;
    let message = match mode {
        0x12 => Some(WhisperMessage::Incoming {
            from: packet.read_string()?,
            channel: packet.read_i8()?,
            from_admin: packet.read_bool()?,
            message: packet.read_string()?,
        }),
        0x09 => Some(WhisperMessage::FindResult {
            name: packet.read_string()?,
            result: packet.read_i8()?,
        }),
        0x0A => Some(WhisperMessage::DeliveryResult {
            name: packet.read_string()?,
            result: packet.read_i8()?,
        }),
        _ => None,
    };
    Ok(message)
}

pub fn whisper_packet(target: &str, message: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_WHISPER_SEND);
    packet.write_u8(0x06);
    packet.write_string(target)?;
    packet.write_string(message)?;
    Ok(packet.into_bytes())
}

pub fn find_player_packet(target: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_WHISPER_SEND);
    packet.write_u8(0x05);
    packet.write_string(target)?;
    Ok(packet.into_bytes())
}

const PARTY_MEMBER_COUNT: usize = 6;
const PARTY_SNAPSHOT_BYTES: usize =
    PARTY_MEMBER_COUNT * 4 + PARTY_MEMBER_COUNT * 13 + PARTY_MEMBER_COUNT * 4 * 4 + 4;

/// Parses the v83 server party-operation result. The fixed six-member block is
/// length-checked before any fields are read, avoiding the legacy reader's
/// partial-block underflow on disband and leave responses.
pub fn parse_party_operation(bytes: &[u8]) -> Result<Option<PartyEvent>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_PARTY_OPERATION)?;
    let operation = packet.read_i8()?;
    let event = match operation {
        4 => Some(PartyEvent::Invite {
            party_id: packet.read_i32()?,
            from: packet.read_string()?,
        }),
        7 => {
            let party_id = packet.read_i32()?;
            Some(PartyEvent::Snapshot(read_party_snapshot(
                &mut packet,
                party_id,
            )?))
        }
        8 => {
            let party_id = packet.read_i32()?;
            packet.skip(16)?; // Door town/map and position.
            Some(PartyEvent::Created { party_id })
        }
        0x0C => {
            let party_id = packet.read_i32()?;
            let character_id = packet.read_i32()?;
            let status = packet.read_i8()?;
            if status == 0 {
                if packet.remaining() >= 4 {
                    packet.read_i32()?; // Repeated party ID in disband result.
                }
                Some(PartyEvent::Removed {
                    party_id,
                    character_id,
                    disbanded: true,
                    expelled: false,
                    name: String::new(),
                    snapshot: None,
                })
            } else {
                let expelled = packet.read_u8()? != 0;
                let name = packet.read_string()?;
                let snapshot = read_optional_party_snapshot(&mut packet, party_id)?;
                Some(PartyEvent::Removed {
                    party_id,
                    character_id,
                    disbanded: false,
                    expelled,
                    name,
                    snapshot,
                })
            }
        }
        0x0F => {
            let party_id = packet.read_i32()?;
            let name = packet.read_string()?;
            Some(PartyEvent::MemberJoined {
                party_id,
                name,
                snapshot: read_optional_party_snapshot(&mut packet, party_id)?,
            })
        }
        0x1B => {
            let character_id = packet.read_i32()?;
            packet.skip(1)?;
            Some(PartyEvent::LeaderChanged { character_id })
        }
        0x23 => {
            if packet.remaining() >= 2 {
                packet.skip(2)?;
            }
            Some(PartyEvent::DoorUpdated)
        }
        code => {
            let name = if matches!(code, 22 | 23) && !packet.is_empty() {
                Some(packet.read_string()?)
            } else {
                None
            };
            Some(PartyEvent::Status { code, name })
        }
    };
    Ok(event)
}

fn read_optional_party_snapshot(
    packet: &mut PacketReader<'_>,
    party_id: i32,
) -> Result<Option<PartySnapshot>, PacketError> {
    if packet.is_empty() {
        Ok(None)
    } else {
        read_party_snapshot(packet, party_id).map(Some)
    }
}

fn read_party_snapshot(
    packet: &mut PacketReader<'_>,
    party_id: i32,
) -> Result<PartySnapshot, PacketError> {
    if packet.remaining() < PARTY_SNAPSHOT_BYTES {
        packet.skip(PARTY_SNAPSHOT_BYTES)?;
    }
    let mut character_ids = [0; PARTY_MEMBER_COUNT];
    for character_id in &mut character_ids {
        *character_id = packet.read_i32()?;
    }
    let mut names = Vec::with_capacity(PARTY_MEMBER_COUNT);
    for _ in 0..PARTY_MEMBER_COUNT {
        names.push(packet.read_string_bytes(13)?);
    }
    let mut jobs = [0; PARTY_MEMBER_COUNT];
    for job in &mut jobs {
        *job = packet.read_i32()?;
    }
    let mut levels = [0; PARTY_MEMBER_COUNT];
    for level in &mut levels {
        *level = packet.read_i32()?;
    }
    let mut channels = [0; PARTY_MEMBER_COUNT];
    for channel in &mut channels {
        *channel = packet.read_i32()?;
    }
    let leader_id = packet.read_i32()?;
    let mut map_ids = [0; PARTY_MEMBER_COUNT];
    for map_id in &mut map_ids {
        *map_id = packet.read_i32()?;
    }
    let members = (0..PARTY_MEMBER_COUNT)
        .filter(|index| character_ids[*index] != 0)
        .map(|index| PartyMember {
            character_id: character_ids[index],
            name: std::mem::take(&mut names[index]),
            job: jobs[index],
            level: levels[index],
            channel: channels[index],
            map_id: map_ids[index],
            hp: None,
        })
        .collect();
    Ok(PartySnapshot {
        party_id,
        leader_id,
        members,
    })
}

pub fn parse_party_member_hp(bytes: &[u8]) -> Result<PartyMemberHp, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_UPDATE_PARTY_MEMBER_HP)?;
    Ok(PartyMemberHp {
        character_id: packet.read_i32()?,
        hp: packet.read_i32()?,
        max_hp: packet.read_i32()?,
    })
}

fn party_operation(operation: i8) -> PacketWriter {
    let mut packet = PacketWriter::new(OPCODE_PARTY_OPERATION_SEND);
    packet.write_i8(operation);
    packet
}

pub fn create_party_packet() -> Vec<u8> {
    party_operation(1).into_bytes()
}

pub fn leave_party_packet() -> Vec<u8> {
    party_operation(2).into_bytes()
}

pub fn join_party_packet(party_id: i32) -> Vec<u8> {
    let mut packet = party_operation(3);
    packet.write_i32(party_id);
    packet.into_bytes()
}

pub fn invite_to_party_packet(name: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = party_operation(4);
    packet.write_string(name)?;
    Ok(packet.into_bytes())
}

pub fn expel_party_member_packet(character_id: i32) -> Vec<u8> {
    let mut packet = party_operation(5);
    packet.write_i32(character_id);
    packet.into_bytes()
}

pub fn change_party_leader_packet(character_id: i32) -> Vec<u8> {
    let mut packet = party_operation(6);
    packet.write_i32(character_id);
    packet.into_bytes()
}

pub fn deny_party_invite_packet(inviter: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_DENY_PARTY_INVITE);
    packet.write_i8(0);
    packet.write_string(inviter)?;
    Ok(packet.into_bytes())
}

/// Parses server buddy-list snapshots, invitations, online updates, capacity,
/// and the legacy operation-coded errors.
pub fn parse_buddy_list(bytes: &[u8]) -> Result<Option<BuddyEvent>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_BUDDY_LIST)?;
    let operation = packet.read_i8()?;
    let event = match operation {
        7 => {
            let count = usize::from(packet.read_u8()?);
            let mut entries = Vec::with_capacity(count);
            for _ in 0..count {
                entries.push(BuddyEntry {
                    character_id: packet.read_i32()?,
                    name: packet.read_string_bytes(13)?,
                    status: packet.read_i8()?,
                    channel: packet.read_i32()?,
                    group: packet.read_string_bytes(13)?,
                });
                packet.read_i32()?; // Entry map-id/padding field.
            }
            for _ in 0..count {
                packet.read_i32()?; // Current map IDs are not shown in this window.
            }
            Some(BuddyEvent::FullList(entries))
        }
        9 => Some(BuddyEvent::Invite {
            character_id: packet.read_i32()?,
            name: packet.read_string()?,
        }),
        0x14 => {
            let character_id = packet.read_i32()?;
            packet.skip(1)?;
            Some(BuddyEvent::Channel {
                character_id,
                channel: packet.read_i32()?,
            })
        }
        0x15 => Some(BuddyEvent::Capacity(packet.read_u8()?)),
        0x0B | 0x0C | 0x0D | 0x0F => Some(BuddyEvent::Error(operation)),
        _ => None,
    };
    Ok(event)
}

pub fn add_buddy_packet(name: &str) -> Result<Vec<u8>, PacketError> {
    set_buddy_group_packet(name, "Default Group")
}

/// Adds a buddy or changes the group of an existing entry using the v83
/// BUDDYLIST_MODIFY add operation.
pub fn set_buddy_group_packet(name: &str, group: &str) -> Result<Vec<u8>, PacketError> {
    let group_length = group.encode_utf16().count();
    if group_length > 16 {
        return Err(PacketError::InvalidBuddyGroupLength(group_length));
    }
    let mut packet = PacketWriter::new(OPCODE_BUDDY_LIST_MODIFY);
    packet.write_u8(1);
    packet.write_string(name)?;
    packet.write_string(group)?;
    Ok(packet.into_bytes())
}

pub fn accept_buddy_packet(character_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_BUDDY_LIST_MODIFY);
    packet.write_u8(2);
    packet.write_i32(character_id);
    packet.into_bytes()
}

pub fn delete_buddy_packet(character_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_BUDDY_LIST_MODIFY);
    packet.write_u8(3);
    packet.write_i32(character_id);
    packet.into_bytes()
}

pub fn parse_reactor_spawn(bytes: &[u8]) -> Result<MapReactorSpawn, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SPAWN_REACTOR)?;
    let object_id = packet.read_i32()?;
    let reactor_id = packet.read_i32()?;
    let state = packet.read_i8()?;
    let (x, y) = packet.read_point_i16()?;
    packet.skip(3)?;
    Ok(MapReactorSpawn {
        object_id,
        reactor_id,
        state,
        x,
        y,
    })
}

pub fn parse_reactor_hit(bytes: &[u8]) -> Result<MapReactorState, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_HIT_REACTOR)?;
    let object_id = packet.read_i32()?;
    let state = packet.read_i8()?;
    let (x, y) = packet.read_point_i16()?;
    packet.read_i16()?; // Stance.
    packet.skip(2)?; // Padding and animation delay.
    Ok(MapReactorState {
        object_id,
        state,
        x,
        y,
    })
}

pub fn parse_reactor_remove(bytes: &[u8]) -> Result<MapReactorState, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_REMOVE_REACTOR)?;
    Ok(MapReactorState {
        object_id: packet.read_i32()?,
        state: packet.read_i8()?,
        x: packet.read_i16()?,
        y: packet.read_i16()?,
    })
}

pub fn pickup_item(object_id: i32, x: i16, y: i16) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PICKUP_ITEM);
    packet.write_i32(0);
    packet.write_u8(0);
    packet.write_point_i16(x, y);
    packet.write_i32(object_id);
    packet.into_bytes()
}

/// Requests pickup of a drop by the pet identified by its cash serial number.
pub fn pet_loot_packet(pet_id: i32, object_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PET_LOOT);
    packet.write_i32(pet_id);
    packet.write_bytes(&[0; 13]);
    packet.write_i32(object_id);
    packet.into_bytes()
}

pub fn face_expression(expression: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_FACE_EXPRESSION);
    packet.write_i32(expression);
    packet.into_bytes()
}

pub fn change_map_packet(
    died: bool,
    target_id: i32,
    target_portal: &str,
    use_wheel: bool,
) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_CHANGE_MAP);
    packet.write_bool(died);
    packet.write_i32(target_id);
    packet.write_string(target_portal)?;
    packet.write_u8(0);
    packet.write_i16(if use_wheel { 1 } else { 0 });
    Ok(packet.into_bytes())
}

pub fn damage_reactor_packet(
    object_id: i32,
    x: i16,
    y: i16,
    stance: i16,
    skill_id: i32,
) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_DAMAGE_REACTOR);
    packet.write_i32(object_id);
    packet.write_point_i16(x, y);
    packet.write_i16(stance);
    packet.write_bytes(&[0; 4]);
    packet.write_i32(skill_id);
    packet.into_bytes()
}

/// Builds the v83 client report for a local character hit by a mob.
pub fn take_damage_packet(
    timestamp: u32,
    damage: i32,
    mob_id: i32,
    mob_object_id: i32,
    direction: u8,
) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_TAKE_DAMAGE);
    packet.write_u32(timestamp);
    packet.write_i8(-1); // Touch damage.
    packet.write_u8(0); // Element.
    packet.write_i32(damage);
    packet.write_i32(mob_id);
    packet.write_i32(mob_object_id);
    packet.write_u8(direction);
    packet.into_bytes()
}

/// Builds the v83 client report for passive HP/MP recovery.
pub fn heal_over_time_packet(heal_hp: i16, heal_mp: i16) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_HEAL_OVER_TIME);
    packet.write_bytes(&[0; 8]);
    packet.write_i16(heal_hp);
    packet.write_i16(heal_mp);
    packet.into_bytes()
}

pub fn parse_mob_damage(bytes: &[u8]) -> Result<MobDamage, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_DAMAGE_MOB)?;
    let object_id = packet.read_i32()?;
    packet.skip(1)?;
    Ok(MobDamage {
        object_id,
        damage: packet.read_i32()?,
    })
}

pub fn parse_mob_movement_response(bytes: &[u8]) -> Result<MobMovementResponse, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_MOVE_MOB_RESPONSE)?;
    Ok(MobMovementResponse {
        object_id: packet.read_i32()?,
        move_id: packet.read_i16()?,
        use_skills: packet.read_bool()?,
        current_mp: packet.read_u16()?,
        skill_id: packet.read_i8()?,
        skill_level: packet.read_i8()?,
    })
}

/// Builds the legacy MOVE_MONSTER absolute-position claim/update packet.
pub fn move_mob(object_id: i32, movement_nibble: i8, movement: PlayerMovement) -> Vec<u8> {
    move_mob_action(object_id, movement_nibble, 0, 0, 0, 0, movement)
}

/// Builds a legacy mob movement packet carrying an attack or mob-skill action.
pub fn move_mob_action(
    object_id: i32,
    movement_nibble: i8,
    action: i8,
    skill_id: u8,
    skill_level: u8,
    option: i16,
    movement: PlayerMovement,
) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_MOVE_MOB);
    packet.write_i32(object_id);
    packet.write_i16(1); // Mob movement type.
    packet.write_i8(movement_nibble);
    packet.write_i8(action);
    packet.write_u8(skill_id);
    packet.write_u8(skill_level);
    packet.write_i16(option);
    packet.write_bytes(&[0; 13]);
    packet.write_point_i16(movement.x, movement.y);
    packet.write_u8(1);
    packet.write_u8(0); // Absolute movement fragment.
    packet.write_point_i16(movement.x, movement.y);
    packet.write_point_i16(movement.last_x, movement.last_y);
    packet.write_u16(movement.foothold);
    packet.write_u8(movement.stance);
    packet.write_i16(movement.duration);
    packet.into_bytes()
}

pub fn parse_mob_status(bytes: &[u8]) -> Result<MobStatusUpdate, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_APPLY_MOB_STATUS)?;
    let object_id = packet.read_i32()?;
    packet.skip(8)?;
    let first_mask = packet.read_u32()?;
    let second_mask = packet.read_u32()?;
    let mut entries =
        Vec::with_capacity(first_mask.count_ones() as usize + second_mask.count_ones() as usize);
    read_status_entries(
        &mut packet,
        MobStatusMaskGroup::First,
        first_mask,
        &mut entries,
    )?;
    read_status_entries(
        &mut packet,
        MobStatusMaskGroup::Second,
        second_mask,
        &mut entries,
    )?;
    Ok(MobStatusUpdate {
        object_id,
        first_mask,
        second_mask,
        entries,
    })
}

pub fn parse_mob_status_cancel(bytes: &[u8]) -> Result<MobStatusCancel, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_CANCEL_MOB_STATUS)?;
    let object_id = packet.read_i32()?;
    packet.skip(8)?;
    Ok(MobStatusCancel {
        object_id,
        first_mask: packet.read_u32()?,
        second_mask: packet.read_u32()?,
    })
}

fn read_status_entries(
    packet: &mut PacketReader<'_>,
    group: MobStatusMaskGroup,
    mask: u32,
    entries: &mut Vec<MobStatusEntry>,
) -> Result<(), PacketError> {
    for bit in 0..u32::BITS {
        let flag = 1_u32 << bit;
        if mask & flag != 0 {
            entries.push(MobStatusEntry {
                group,
                flag,
                value: packet.read_i16()?,
                skill_id: packet.read_i32()?,
                duration: packet.read_i16()?,
            });
        }
    }
    Ok(())
}

pub fn parse_mob_spawn(bytes: &[u8]) -> Result<MapMobSpawn, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SPAWN_MOB)?;
    let object_id = packet.read_i32()?;
    parse_mob_spawn_fields(&mut packet, object_id, 0)
}

pub fn parse_mob_kill(bytes: &[u8]) -> Result<(i32, i8), PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_KILL_MOB)?;
    Ok((packet.read_i32()?, packet.read_i8()?))
}

pub fn parse_mob_controller(bytes: &[u8]) -> Result<MobControllerUpdate, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SPAWN_MOB_CONTROLLER)?;
    let mode = packet.read_i8()?;
    let object_id = packet.read_i32()?;
    if mode == 0 {
        return Ok(MobControllerUpdate::Control { object_id, mode });
    }
    if packet.is_empty() {
        return Ok(MobControllerUpdate::NoSpawnData { object_id });
    }
    parse_mob_spawn_fields(&mut packet, object_id, mode).map(MobControllerUpdate::Spawn)
}

pub fn parse_mob_movement(bytes: &[u8]) -> Result<MapMobMovement, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_MOB_MOVED)?;
    let object_id = packet.read_i32()?;
    packet.skip(1)?; // v83 padding.
    let skill_possible = packet.read_u8()? != 0;
    let action = packet.read_i8()?;
    let skill_id = packet.read_u8()?;
    let skill_level = packet.read_u8()?;
    let option = packet.read_i16()?;
    let (start_x, start_y) = packet.read_point_i16()?;
    let fragments = parse_movements(&mut packet)?;
    Ok(MapMobMovement {
        object_id,
        skill_possible,
        action,
        skill_id,
        skill_level,
        option,
        start_x,
        start_y,
        fragments,
    })
}

pub fn parse_mist_spawn(bytes: &[u8]) -> Result<MapMistSpawn, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SPAWN_MIST)?;
    let object_id = packet.read_i32()?;
    let mist_type = packet.read_i32()?;
    let owner_id = packet.read_i32()?;
    let skill_id = packet.read_i32()?;
    let skill_level = packet.read_i8()?;
    let delay = packet.read_i16()?;
    let left = packet.read_i32()?;
    let top = packet.read_i32()?;
    let right = packet.read_i32()?;
    let bottom = packet.read_i32()?;
    packet.skip(4)?; // Reserved.
    Ok(MapMistSpawn {
        object_id,
        mist_type,
        owner_id,
        skill_id,
        skill_level,
        delay,
        left,
        top,
        right,
        bottom,
    })
}

pub fn parse_mist_remove(bytes: &[u8]) -> Result<i32, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_REMOVE_MIST)?;
    packet.read_i32()
}

pub fn parse_summon_spawn(bytes: &[u8]) -> Result<MapSummonSpawn, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SPAWN_SUMMON)?;
    let owner_id = packet.read_i32()?;
    let object_id = packet.read_i32()?;
    let skill_id = packet.read_i32()?;
    packet.skip(1)?; // v83 summon marker (0x0A).
    let skill_level = packet.read_i8()?;
    let (x, y) = packet.read_point_i16()?;
    let stance = packet.read_u8()?;
    packet.skip(2)?; // Padding.
    let movement_type = packet.read_u8()?;
    let attacks = packet.read_bool()?;
    let animated = packet.read_bool()?;
    Ok(MapSummonSpawn {
        owner_id,
        object_id,
        skill_id,
        skill_level,
        x,
        y,
        stance,
        movement_type,
        attacks,
        animated,
    })
}

pub fn parse_summon_remove(bytes: &[u8]) -> Result<MapSummonRemove, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_REMOVE_SUMMON)?;
    Ok(MapSummonRemove {
        owner_id: packet.read_i32()?,
        object_id: packet.read_i32()?,
        animation: packet.read_i8()?,
    })
}

pub fn parse_summon_movement(bytes: &[u8]) -> Result<MapSummonMovement, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_MOVE_SUMMON)?;
    let owner_id = packet.read_i32()?;
    let object_id = packet.read_i32()?;
    let (start_x, start_y) = packet.read_point_i16()?;
    let fragments = parse_movements(&mut packet)?;
    Ok(MapSummonMovement {
        owner_id,
        object_id,
        start_x,
        start_y,
        fragments,
    })
}

pub fn parse_summon_skill(bytes: &[u8]) -> Result<SummonSkillUpdate, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SUMMON_SKILL)?;
    Ok(SummonSkillUpdate {
        owner_id: packet.read_i32()?,
        skill_id: packet.read_i32()?,
        stance: packet.read_u8()?,
    })
}

fn parse_mob_spawn_fields(
    packet: &mut PacketReader<'_>,
    object_id: i32,
    control_mode: i8,
) -> Result<MapMobSpawn, PacketError> {
    packet.skip(1)?; // Controller marker.
    let mob_id = packet.read_i32()?;
    packet.skip(16)?;
    let (x, y) = packet.read_point_i16()?;
    let stance = packet.read_u8()?;
    packet.skip(2)?;
    let foothold = packet.read_u16()?;
    let effect = packet.read_i8()?;
    if effect > 0 {
        packet.skip(3)?;
        if effect == 15 {
            packet.skip(1)?;
        }
    }
    let team = packet.read_i8()?;
    packet.skip(4)?;
    Ok(MapMobSpawn {
        object_id,
        mob_id,
        control_mode,
        x,
        y,
        stance,
        foothold,
        team,
        is_new_spawn: effect == -2,
    })
}

pub fn parse_npc_spawn(bytes: &[u8]) -> Result<MapNpcSpawn, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SPAWN_NPC)?;
    let object_id = packet.read_i32()?;
    parse_npc_spawn_fields(&mut packet, object_id)
}

pub fn parse_npc_remove(bytes: &[u8]) -> Result<i32, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_REMOVE_NPC)?;
    packet.read_i32()
}

pub fn parse_npc_controller(bytes: &[u8]) -> Result<NpcControllerUpdate, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SPAWN_NPC_CONTROLLER)?;
    let mode = packet.read_i8()?;
    let object_id = packet.read_i32()?;
    if mode == 0 {
        return Ok(NpcControllerUpdate::Remove { object_id });
    }
    parse_npc_spawn_fields(&mut packet, object_id).map(NpcControllerUpdate::Spawn)
}

pub fn parse_npc_dialogue(bytes: &[u8]) -> Result<NpcDialogue, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_NPC_DIALOGUE)?;
    packet.skip(1)?;
    let npc_id = packet.read_i32()?;
    let message_type = packet.read_i8()?;
    let speaker = packet.read_i8()?;
    let text = packet.read_string()?;
    let number_bounds = if message_type == 3 && packet.remaining() >= 12 {
        Some((packet.read_i32()?, packet.read_i32()?, packet.read_i32()?))
    } else {
        None
    };
    Ok(NpcDialogue {
        npc_id,
        message_type,
        speaker,
        text,
        number_bounds,
    })
}

/// Parse the v83 OPEN_NPC_SHOP payload. Shops omit the final currency item ID
/// on older servers, so an absent trailer means mesos (zero).
pub fn parse_npc_shop(bytes: &[u8]) -> Result<NpcShop, PacketError> {
    const MAX_SHOP_ITEMS: i16 = 2048;
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_OPEN_NPC_SHOP)?;
    let npc_id = packet.read_i32()?;
    let count = packet.read_i16()?;
    if !(0..=MAX_SHOP_ITEMS).contains(&count) {
        return Err(PacketError::InvalidNpcShopCount(count));
    }
    let mut items = Vec::with_capacity(usize::from(count as u16));
    for slot in 0..count {
        let item_id = packet.read_i32()?;
        let price = packet.read_i32()?;
        let pitch = packet.read_i32()?;
        let time = packet.read_i32()?;
        packet.skip(4)?;
        let rechargeable = packet.read_i16()? != 1;
        let (charge_price, buyable) = if rechargeable {
            packet.skip(4)?;
            (packet.read_i16()?, packet.read_i16()?)
        } else {
            (0, packet.read_i16()?)
        };
        items.push(NpcShopItem {
            item_id,
            price,
            pitch,
            time,
            charge_price,
            buyable,
            rechargeable,
            slot,
        });
    }
    let currency_item_id = match packet.remaining() {
        0 => 0,
        4 => packet.read_i32()?,
        remaining => {
            return Err(PacketError::InvalidFrameLength(remaining));
        }
    };
    Ok(NpcShop {
        npc_id,
        items,
        currency_item_id,
    })
}

pub fn parse_npc_shop_transaction_result(bytes: &[u8]) -> Result<i8, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_CONFIRM_NPC_SHOP_TRANSACTION)?;
    packet.read_i8()
}

pub fn parse_npc_action(bytes: &[u8]) -> Result<NpcAction, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_NPC_ACTION)?;
    let object_id = packet.read_i32()?;
    let action = (!packet.is_empty()).then(|| packet.read_i8()).transpose()?;
    Ok(NpcAction { object_id, action })
}

pub fn parse_chair_show(bytes: &[u8]) -> Result<MapChairShow, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SHOW_CHAIR)?;
    Ok(MapChairShow {
        character_id: packet.read_i32()?,
        item_id: packet.read_i32()?,
    })
}

pub fn parse_chair_cancel(bytes: &[u8]) -> Result<MapChairCancel, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_CANCEL_CHAIR_BROADCAST)?;
    Ok(MapChairCancel {
        character_id: packet.read_i32()?,
    })
}

pub fn parse_character_stat_update(
    bytes: &[u8],
    current_job: u16,
) -> Result<CharacterStatUpdate, PacketError> {
    const PET_MASK: u32 = 0x180008;
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_CHARACTER_STAT_UPDATE)?;
    let enable_actions = packet.read_bool()?;
    let mask = packet.read_u32()?;
    let has_pet_update = mask & PET_MASK == PET_MASK;
    let mut job = current_job;
    let mut changes = Vec::with_capacity(20);

    if mask & 0x000001 != 0 {
        changes.push(CharacterStatChange::Skin(packet.read_u8()?));
    }
    if mask & 0x000002 != 0 {
        changes.push(CharacterStatChange::Face(packet.read_i32()?));
    }
    if mask & 0x000004 != 0 {
        changes.push(CharacterStatChange::Hair(packet.read_i32()?));
    }
    if mask & 0x000010 != 0 {
        changes.push(CharacterStatChange::Level(packet.read_u8()?));
    }
    if mask & 0x000020 != 0 {
        job = packet.read_u16()?;
        changes.push(CharacterStatChange::Job(job));
    }
    if mask & 0x000040 != 0 {
        changes.push(CharacterStatChange::Strength(packet.read_u16()?));
    }
    if mask & 0x000080 != 0 {
        changes.push(CharacterStatChange::Dexterity(packet.read_u16()?));
    }
    if mask & 0x000100 != 0 {
        changes.push(CharacterStatChange::Intelligence(packet.read_u16()?));
    }
    if mask & 0x000200 != 0 {
        changes.push(CharacterStatChange::Luck(packet.read_u16()?));
    }
    if mask & 0x000400 != 0 {
        changes.push(CharacterStatChange::Hp(packet.read_u16()?));
    }
    if mask & 0x000800 != 0 {
        changes.push(CharacterStatChange::MaxHp(packet.read_u16()?));
    }
    if mask & 0x001000 != 0 {
        changes.push(CharacterStatChange::Mp(packet.read_u16()?));
    }
    if mask & 0x002000 != 0 {
        changes.push(CharacterStatChange::MaxMp(packet.read_u16()?));
    }
    if mask & 0x004000 != 0 {
        changes.push(CharacterStatChange::AbilityPoints(packet.read_u16()?));
    }
    if mask & 0x008000 != 0 && !has_pet_update {
        if has_sp_table(job) {
            let count = usize::from(packet.read_u8()?);
            let mut table = Vec::with_capacity(count);
            for _ in 0..count {
                table.push((packet.read_u8()?, packet.read_u8()?));
            }
            changes.push(CharacterStatChange::SkillPointTable(table));
        } else {
            changes.push(CharacterStatChange::SkillPoints(packet.read_u16()?));
        }
    }
    if mask & 0x010000 != 0 {
        changes.push(CharacterStatChange::Experience(packet.read_i32()?));
    }
    if mask & 0x020000 != 0 {
        changes.push(CharacterStatChange::Fame(packet.read_u16()?));
    }
    if mask & 0x040000 != 0 {
        changes.push(CharacterStatChange::Meso(packet.read_i32()?));
    }
    if has_pet_update {
        changes.push(CharacterStatChange::Pet(packet.read_i32()?));
    }
    if mask & 0x200000 != 0 {
        changes.push(CharacterStatChange::GachaExperience(packet.read_i32()?));
    }

    Ok(CharacterStatUpdate {
        enable_actions,
        changes,
    })
}

fn has_sp_table(job: u16) -> bool {
    job == 2001 || job == 2200 || (2210..=2218).contains(&job)
}

pub fn parse_npc_scriptable(bytes: &[u8]) -> Result<Vec<i32>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SET_NPC_SCRIPTABLE)?;
    let count = usize::from(packet.read_u8()?);
    let mut npc_ids = Vec::with_capacity(count);
    for _ in 0..count {
        npc_ids.push(packet.read_i32()?);
        packet.read_string()?;
        packet.skip(8)?;
    }
    Ok(npc_ids)
}

/// Parses a v83 `FIELD_EFFECT` event. Type 1 only announces a boss HP bar and
/// has no client payload; unsupported effect types are safely ignored.
pub fn parse_field_effect(bytes: &[u8]) -> Result<Option<FieldEffect>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_FIELD_EFFECT)?;
    if packet.is_empty() {
        return Ok(None);
    }
    let effect_type = packet.read_i8()?;
    let effect = match effect_type {
        0 | 2 | 3 => Some(FieldEffect::ScreenAnimation(packet.read_string()?)),
        4 => Some(FieldEffect::Sound(packet.read_string()?)),
        6 => Some(FieldEffect::Music(packet.read_string()?)),
        _ => None,
    };
    packet.skip(packet.remaining())?;
    Ok(effect)
}

/// Parses the v83 `BLOW_WEATHER` packet. The final message is optional.
pub fn parse_blow_weather(bytes: &[u8]) -> Result<BlowWeatherUpdate, PacketError> {
    const MAX_WEATHER_MESSAGE_BYTES: usize = 512;
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_BLOW_WEATHER)?;
    let kind = packet.read_i8()?;
    let item_id = packet.read_i32()?;
    let message = if packet.is_empty() {
        String::new()
    } else {
        let message = packet.read_string()?;
        if message.len() > MAX_WEATHER_MESSAGE_BYTES {
            return Err(PacketError::StringTooLong(message.len()));
        }
        message
    };
    packet.skip(packet.remaining())?;
    Ok(BlowWeatherUpdate {
        kind,
        item_id,
        message,
    })
}

pub fn parse_field_obstacle_one(bytes: &[u8]) -> Result<FieldObstacleToggle, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_FIELD_OBSTACLE_ONE)?;
    Ok(FieldObstacleToggle {
        name: packet.read_string()?,
        mode: packet.read_i32()?,
    })
}

pub fn parse_field_obstacle_list(bytes: &[u8]) -> Result<Vec<FieldObstacleToggle>, PacketError> {
    const MAX_FIELD_OBSTACLES: i32 = 4096;
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_FIELD_OBSTACLE_ONOFF)?;
    let count = packet.read_i32()?;
    if !(0..=MAX_FIELD_OBSTACLES).contains(&count) {
        return Err(PacketError::InvalidFieldObstacleCount(count));
    }
    let mut toggles = Vec::with_capacity(count as usize);
    for _ in 0..count {
        toggles.push(FieldObstacleToggle {
            name: packet.read_string()?,
            mode: packet.read_i32()?,
        });
    }
    Ok(toggles)
}

pub fn talk_to_npc(object_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_TALK_TO_NPC);
    packet.write_i32(object_id);
    packet.into_bytes()
}

/// Buy or sell an item from the server's shop list. `buy` selects action 0;
/// `false` selects action 1. Shop inventory remains server-authoritative.
pub fn npc_shop_item_action(
    slot: i16,
    item_id: i32,
    quantity: i16,
    buy: bool,
) -> Result<Vec<u8>, PacketError> {
    if slot < 0 || item_id <= 0 || quantity <= 0 {
        return Err(PacketError::InvalidNpcShopAction {
            slot,
            item_id,
            quantity,
        });
    }
    let mut packet = PacketWriter::new(OPCODE_NPC_SHOP_ACTION);
    packet.write_i8(if buy { 0 } else { 1 });
    packet.write_i16(slot);
    packet.write_i32(item_id);
    packet.write_i16(quantity);
    Ok(packet.into_bytes())
}

pub fn npc_shop_recharge(slot: i16) -> Result<Vec<u8>, PacketError> {
    if slot < 0 {
        return Err(PacketError::InvalidNpcShopAction {
            slot,
            item_id: 0,
            quantity: 0,
        });
    }
    let mut packet = PacketWriter::new(OPCODE_NPC_SHOP_ACTION);
    packet.write_i8(2);
    packet.write_i16(slot);
    Ok(packet.into_bytes())
}

pub fn npc_shop_leave() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_NPC_SHOP_ACTION);
    packet.write_i8(3);
    packet.into_bytes()
}

pub fn npc_talk_more(message_type: i8, response: i8) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_NPC_TALK_MORE);
    packet.write_i8(message_type);
    packet.write_i8(response);
    packet.into_bytes()
}

pub fn npc_talk_selection(selection: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_NPC_TALK_MORE);
    packet.write_i8(4);
    packet.write_i8(1);
    packet.write_i32(selection);
    packet.into_bytes()
}

pub fn npc_talk_number(value: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_NPC_TALK_MORE);
    packet.write_i8(3);
    packet.write_i8(1);
    packet.write_i32(value);
    packet.into_bytes()
}

pub fn npc_talk_text(value: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_NPC_TALK_MORE);
    packet.write_i8(2);
    packet.write_i8(1);
    packet.write_string(value)?;
    Ok(packet.into_bytes())
}

fn parse_npc_spawn_fields(
    packet: &mut PacketReader<'_>,
    object_id: i32,
) -> Result<MapNpcSpawn, PacketError> {
    let npc_id = packet.read_i32()?;
    let (x, y) = packet.read_point_i16()?;
    let flipped = packet.read_bool()?;
    let foothold = packet.read_u16()?;
    packet.skip(5)?; // Talk range and minimap visibility.
    Ok(MapNpcSpawn {
        object_id,
        npc_id,
        x,
        y,
        flipped,
        foothold,
    })
}

pub fn parse_remote_character_spawn(bytes: &[u8]) -> Result<RemoteCharacterSpawn, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SPAWN_CHARACTER)?;
    let id = packet.read_i32()?;
    let level = packet.read_u8()?;
    let name = packet.read_string()?;

    packet.read_string()?; // Guild name.
    packet.skip(6)?; // Guild emblem and colors.
    packet.skip(8)?;

    let morphed = packet.read_i32()? == 2;
    let buff_mask = packet.read_u32()?;
    if morphed {
        packet.skip(2)?;
    } else if buff_mask & 0x0020_0000 != 0 {
        packet.skip(1)?;
    }
    packet.skip(4)?; // Second foreign buff mask.
    packet.skip(43)?;
    packet.skip(4)?; // Riding mount.
    packet.skip(61)?;

    let job = packet.read_i16()? as u16;
    let look = read_character_look(&mut packet)?;
    packet.read_i32()?; // Count of item 5110000.
    let item_effect_id = packet.read_i32()?;
    packet.read_i32()?; // Chair.
    let (x, y) = packet.read_point_i16()?;
    let stance = packet.read_u8()?;
    packet.skip(3)?; // Pet list marker padding.
    let mut pets = Vec::with_capacity(3);
    for slot in 0..3 {
        if packet.is_empty() || packet.read_i8()? != 1 {
            break;
        }
        pets.push(read_character_pet_spawn(&mut packet, id, slot)?);
    }

    // The spawn tail is mount state followed by the character's mini-room
    // sign (personal shop, trade box, etc). Older or truncated variants may
    // omit the tail; only parse it when the complete mount prefix and type byte
    // are present so pet parsing remains compatible with those packets.
    let mini_room = if packet.remaining() >= 13 {
        packet.skip(12)?; // Mount level, experience, and tiredness.
        read_character_mini_room(&mut packet, true)?
    } else {
        None
    };

    Ok(RemoteCharacterSpawn {
        id,
        name,
        level,
        job,
        look,
        item_effect_id,
        x,
        y,
        stance,
        pets,
        mini_room,
    })
}

pub fn parse_remote_character_item_effect(
    bytes: &[u8],
) -> Result<RemoteCharacterItemEffect, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SHOW_ITEM_EFFECT)?;
    Ok(RemoteCharacterItemEffect {
        character_id: packet.read_i32()?,
        item_id: packet.read_i32()?,
    })
}

pub fn parse_remote_character_mini_room(
    bytes: &[u8],
) -> Result<RemoteCharacterMiniRoomUpdate, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_UPDATE_CHARACTER_BOX)?;
    let character_id = packet.read_i32()?;
    let mini_room = read_character_mini_room(&mut packet, false)?;
    Ok(RemoteCharacterMiniRoomUpdate {
        character_id,
        mini_room,
    })
}

fn read_character_mini_room(
    packet: &mut PacketReader<'_>,
    spawn_tail: bool,
) -> Result<Option<CharacterMiniRoom>, PacketError> {
    let room_type = packet.read_i8()?;
    if room_type == 0 {
        return Ok(None);
    }
    let object_id = packet.read_i32()?;
    let description = packet.read_string()?;
    let skin = if packet.remaining() >= 2 {
        packet.skip(1)?; // Chalkboard/sign visibility flag.
        packet.read_u8()?
    } else {
        0
    };
    if spawn_tail && packet.remaining() >= 3 {
        packet.skip(3)?;
    }
    Ok(Some(CharacterMiniRoom {
        room_type,
        object_id,
        description,
        skin,
    }))
}

/// Builds the legacy request, including its four-byte request nonce.
pub fn character_info_request_packet(character_id: i32) -> Option<Vec<u8>> {
    if character_id <= 0 {
        return None;
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos()
        & 0x7fff_ffff;
    let mut packet = PacketWriter::new(OPCODE_CHARACTER_INFO_REQUEST);
    packet.write_i32(nonce as i32);
    packet.write_i32(character_id);
    Some(packet.into_bytes())
}

/// Builds the v83 fame request for a visible character.
pub fn give_fame_packet(character_id: i32, raise: bool) -> Option<Vec<u8>> {
    if character_id <= 0 {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_GIVE_FAME);
    packet.write_i32(character_id);
    packet.write_u8(u8::from(raise));
    Some(packet.into_bytes())
}

/// Parses the v83 fame response, including the updated target fame on success.
pub fn parse_fame_response(bytes: &[u8]) -> Result<FameResponse, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_FAME_RESPONSE)?;
    let response = match packet.read_u8()? {
        0 => {
            let character_name = packet.read_string()?;
            let raised = packet.read_u8()? != 0;
            let fame = if packet.remaining() >= 2 {
                packet.read_i16()?
            } else {
                0
            };
            if !packet.is_empty() {
                packet.read_i16()?; // Legacy padding.
            }
            FameResponse::Success {
                character_name,
                raised,
                fame,
            }
        }
        1 => FameResponse::CannotFameSelf,
        2 => FameResponse::AlreadyFamed,
        3 => FameResponse::TargetLevelTooLow,
        5 => FameResponse::PlayerLevelTooLow,
        mode => FameResponse::Unknown(mode),
    };
    Ok(response)
}

/// Parses the v83 result sent after using a skill book or mastery book.
pub fn parse_skill_learn_item_result(bytes: &[u8]) -> Result<SkillLearnItemResult, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SKILL_LEARN_ITEM_RESULT)?;
    Ok(SkillLearnItemResult {
        character_id: packet.read_i32()?,
        marker: packet.read_u8()?,
        skill_id: packet.read_i32()?,
        max_level: packet.read_i32()?,
        can_use: packet.read_u8()? != 0,
        success: packet.read_u8()? != 0,
    })
}

/// Parses the v83 forced-stat bitmask used for server-provided character totals.
pub fn parse_forced_stat_set(bytes: &[u8]) -> Result<ForcedStatSet, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_FORCED_STAT_SET)?;
    let mask = packet.read_u32()?;
    if mask & !0x0fff != 0 {
        return Err(PacketError::InvalidForcedStatMask(mask));
    }

    let mut stats = ForcedStatSet::default();
    if mask & (1 << 0) != 0 {
        stats.strength = Some(packet.read_i16()? as u16);
    }
    if mask & (1 << 1) != 0 {
        stats.dexterity = Some(packet.read_i16()? as u16);
    }
    if mask & (1 << 2) != 0 {
        stats.intelligence = Some(packet.read_i16()? as u16);
    }
    if mask & (1 << 3) != 0 {
        stats.luck = Some(packet.read_i16()? as u16);
    }
    if mask & (1 << 4) != 0 {
        stats.weapon_attack = Some(packet.read_i16()?);
    }
    if mask & (1 << 5) != 0 {
        stats.weapon_defense = Some(packet.read_i16()?);
    }
    if mask & (1 << 6) != 0 {
        stats.magic_attack = Some(packet.read_i16()?);
    }
    if mask & (1 << 7) != 0 {
        stats.magic_defense = Some(packet.read_i16()?);
    }
    if mask & (1 << 8) != 0 {
        stats.accuracy = Some(packet.read_i16()?);
    }
    if mask & (1 << 9) != 0 {
        stats.avoidability = Some(packet.read_i16()?);
    }
    if mask & (1 << 10) != 0 {
        stats.speed = Some(packet.read_u8()?);
    }
    if mask & (1 << 11) != 0 {
        stats.jump = Some(packet.read_u8()?);
    }
    Ok(stats)
}

/// Parses the bounded v83 character-info response used by the map character window.
pub fn parse_character_info(bytes: &[u8]) -> Result<CharacterInfoReply, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_CHARACTER_INFO)?;
    let character_id = packet.read_i32()?;
    let level = packet.read_u8()?;
    let job = packet.read_i16()? as u16;
    let fame = packet.read_i16()?;
    let married = packet.read_bool()?;
    let guild = packet.read_string()?;
    let alliance = packet.read_string()?;
    packet.read_u8()?; // Medal visibility information.

    let mut pets = Vec::new();
    loop {
        let available = packet.read_u8()?;
        if available == 0 {
            break;
        }
        if pets.len() >= MAX_CHARACTER_INFO_PETS {
            return Err(PacketError::InvalidCharacterInfoCount {
                section: "pets",
                count: i32::try_from(pets.len() + 1).unwrap_or(i32::MAX),
            });
        }
        pets.push(CharacterInfoPet {
            item_id: packet.read_i32()?,
            name: packet.read_string()?,
            level: packet.read_u8()?,
            closeness: packet.read_i16()?,
            fullness: packet.read_u8()?,
            // The packet also contains an unknown short before the ring ID.
            // It has no UI meaning but must be consumed to keep alignment.
            label_ring_item_id: {
                packet.read_i16()?;
                packet.read_i32()?
            },
        });
    }

    let mount = if packet.read_u8()? != 0 {
        Some(CharacterMountInfo {
            level: packet.read_i32()?,
            experience: packet.read_i32()?,
            tiredness: packet.read_i32()?,
        })
    } else {
        None
    };

    let wishlist_count = packet.read_i8()?;
    let wishlist_count =
        usize::try_from(wishlist_count).map_err(|_| PacketError::InvalidCharacterInfoCount {
            section: "wishlist",
            count: i32::from(wishlist_count),
        })?;
    if wishlist_count > MAX_CHARACTER_WISHLIST {
        return Err(PacketError::InvalidCharacterInfoCount {
            section: "wishlist",
            count: i32::try_from(wishlist_count).unwrap_or(i32::MAX),
        });
    }
    let mut wishlist = Vec::with_capacity(wishlist_count);
    for _ in 0..wishlist_count {
        wishlist.push(packet.read_i32()?);
    }

    let monster_book = CharacterMonsterBookInfo {
        level: packet.read_i32()?,
        normal_cards: packet.read_i32()?,
        special_cards: packet.read_i32()?,
        total_cards: packet.read_i32()?,
        cover_item_id: packet.read_i32()?,
    };

    let medal = if packet.is_empty() {
        None
    } else {
        let item_id = packet.read_i32()?;
        let count = packet.read_i16()?;
        let count = usize::try_from(count).map_err(|_| PacketError::InvalidCharacterInfoCount {
            section: "medal quests",
            count: i32::from(count),
        })?;
        if count > MAX_CHARACTER_MEDAL_QUESTS {
            return Err(PacketError::InvalidCharacterInfoCount {
                section: "medal quests",
                count: i32::try_from(count).unwrap_or(i32::MAX),
            });
        }
        let mut quest_ids = Vec::with_capacity(count);
        for _ in 0..count {
            quest_ids.push(packet.read_i16()?);
        }
        Some(CharacterMedalInfo { item_id, quest_ids })
    };

    Ok(CharacterInfoReply {
        character_id,
        level,
        job,
        fame,
        married,
        guild,
        alliance,
        pets,
        mount,
        wishlist,
        monster_book,
        medal,
    })
}

fn read_character_pet_spawn(
    packet: &mut PacketReader<'_>,
    character_id: i32,
    slot: u8,
) -> Result<MapPetSpawn, PacketError> {
    let item_id = packet.read_i32()?;
    let name = packet.read_string()?;
    let unique_id = packet.read_i32()?;
    packet.skip(4)?; // High half of the unique id.
    let (x, y) = packet.read_point_i16()?;
    let stance = packet.read_u8()?;
    let foothold = packet.read_i32()?;
    Ok(MapPetSpawn {
        character_id,
        slot,
        item_id,
        name,
        unique_id,
        x,
        y,
        stance,
        foothold,
    })
}

pub fn parse_pet_spawn(bytes: &[u8]) -> Result<PetSpawnEvent, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SPAWN_PET_RECEIVED)?;
    let character_id = packet.read_i32()?;
    let slot = packet.read_u8()?;
    if slot >= MAX_PET_SLOTS {
        return Err(PacketError::InvalidPetSlot(slot));
    }
    let mode = packet.read_i8()?;
    match mode {
        1 => {
            packet.skip(1)?;
            Ok(PetSpawnEvent::Spawn(read_character_pet_spawn(
                &mut packet,
                character_id,
                slot,
            )?))
        }
        0 => Ok(PetSpawnEvent::Remove {
            character_id,
            slot,
            hunger: packet.read_bool()?,
        }),
        _ => Err(PacketError::InvalidPetSpawnMode(mode)),
    }
}

pub fn parse_pet_movement(bytes: &[u8]) -> Result<MapPetMovement, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_MOVE_PET)?;
    let character_id = packet.read_i32()?;
    let slot = packet.read_u8()?;
    if slot >= MAX_PET_SLOTS {
        return Err(PacketError::InvalidPetSlot(slot));
    }
    let pet_id = packet.read_i32()?;
    let fragments = parse_movements(&mut packet)?;
    Ok(MapPetMovement {
        character_id,
        slot,
        pet_id,
        fragments,
    })
}

pub fn parse_pet_chat(bytes: &[u8]) -> Result<PetChatUpdate, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_PET_CHAT_RECEIVED)?;
    let character_id = packet.read_i32()?;
    let slot = packet.read_u8()?;
    if slot >= MAX_PET_SLOTS {
        return Err(PacketError::InvalidPetSlot(slot));
    }
    packet.skip(1)?; // Reserved byte.
    let act = packet.read_i8()?;
    let text = packet.read_string()?;
    let balloon_type = packet.read_u8()?;
    Ok(PetChatUpdate {
        character_id,
        slot,
        act,
        text,
        balloon_type,
    })
}

pub fn parse_pet_command(bytes: &[u8]) -> Result<PetCommandUpdate, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_PET_COMMAND_RECEIVED)?;
    let character_id = packet.read_i32()?;
    let slot = packet.read_u8()?;
    if slot >= MAX_PET_SLOTS {
        return Err(PacketError::InvalidPetSlot(slot));
    }
    let interaction_type = packet.read_i8()?;
    let animation = packet.read_i8()?;
    let talk = !packet.read_bool()?;
    // Some servers append the balloon style, while the v83 reference client
    // documents that this byte may be absent.
    let balloon_type = if packet.is_empty() {
        None
    } else {
        Some(packet.read_u8()?)
    };
    Ok(PetCommandUpdate {
        character_id,
        slot,
        interaction_type,
        animation,
        talk,
        balloon_type,
    })
}

pub fn parse_remote_character_remove(bytes: &[u8]) -> Result<i32, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_REMOVE_CHARACTER)?;
    packet.read_i32()
}

pub fn parse_foreign_skill_effect(bytes: &[u8]) -> Result<ForeignSkillEffect, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SKILL_EFFECT)?;
    let effect = ForeignSkillEffect {
        character_id: packet.read_i32()?,
        skill_id: packet.read_i32()?,
        level: packet.read_i8()?,
        flags: packet.read_i8()?,
        speed: packet.read_i8()?,
        direction: packet.read_i8()?,
    };
    packet.skip(packet.remaining())?;
    Ok(effect)
}

pub fn parse_show_foreign_effect(
    bytes: &[u8],
) -> Result<Option<ForeignCharacterEffect>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SHOW_FOREIGN_EFFECT)?;
    let character_id = packet.read_i32()?;
    let effect_type = packet.read_i8()?;
    let effect = match effect_type {
        // Level-up and monster-card animations have no additional payload.
        0 | 13 => Some(ForeignCharacterEffect {
            character_id,
            effect_type,
            variant: 0,
            heal_amount: None,
            skill_id: None,
            level: 0,
            direction: -1,
        }),
        // Job-change and quest-clear effects carry an optional legacy marker
        // byte. Their presentation is determined by effect_type.
        6 | 7 => Some(ForeignCharacterEffect {
            character_id,
            effect_type,
            variant: 0,
            heal_amount: None,
            skill_id: None,
            level: 0,
            direction: -1,
        }),
        // Cosmic's v83 server IDs: job change, quest clear, and successful
        // scroll feedback are sent as plain effect bytes.
        8 | 9 | 15 => Some(ForeignCharacterEffect {
            character_id,
            effect_type,
            variant: 0,
            heal_amount: None,
            skill_id: None,
            level: 0,
            direction: -1,
        }),
        // Cosmic sends a 32-bit 0/1 result for foreign Maker effects.
        16 => Some(ForeignCharacterEffect {
            character_id,
            effect_type,
            variant: i8::from(packet.read_i32()? != 0),
            heal_amount: None,
            skill_id: None,
            level: 0,
            direction: -1,
        }),
        // Cosmic's recovery broadcast carries the signed HP amount as one byte.
        10 => Some(ForeignCharacterEffect {
            character_id,
            effect_type,
            variant: 0,
            heal_amount: Some(packet.read_i8()?),
            skill_id: None,
            level: 0,
            direction: -1,
        }),
        // Skill use/affected layouts have either a trailing direction or the
        // Cosmic 5-argument layout with a zero marker, level, and direction.
        1 | 2 => {
            let skill_id = packet.read_i32()?;
            let mut level = 0;
            let mut direction = packet.read_i8()?;
            if direction == 0 && packet.remaining() >= 2 {
                level = packet.read_i8()?;
                direction = packet.read_i8()?;
            }
            Some(ForeignCharacterEffect {
                character_id,
                effect_type,
                variant: 0,
                heal_amount: None,
                skill_id: Some(skill_id),
                level,
                direction,
            })
        }
        _ => None,
    };
    packet.skip(packet.remaining())?;
    Ok(effect)
}

/// Parses the supported v83 item-gain notifications. Other legacy feedback
/// modes are safely consumed and ignored until their presentation is ported.
pub fn parse_item_gain_in_chat(bytes: &[u8]) -> Result<Option<ItemGainInChat>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SHOW_ITEM_GAIN_INCHAT)?;
    let mode = packet.read_i8()?;
    let effect = match mode {
        1 => {
            let skill_id = packet.read_i32()?;
            (skill_id > 0).then_some(ItemGainInChat::SkillUse { skill_id })
        }
        3 => {
            let subtype = packet.read_i8()?;
            if subtype == 1 {
                let item_id = packet.read_i32()?;
                let quantity = packet.read_i32()?;
                (item_id > 0).then_some(ItemGainInChat::Item { item_id, quantity })
            } else {
                None
            }
        }
        13 => Some(ItemGainInChat::MonsterBookCard),
        _ => None,
    };
    packet.skip(packet.remaining())?;
    Ok(effect)
}

pub fn parse_remote_player_damage(bytes: &[u8]) -> Result<RemotePlayerDamage, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_DAMAGE_PLAYER)?;
    let character_id = packet.read_i32()?;
    let skill = packet.read_i8()?;
    if skill == -3 {
        packet.skip(4)?; // DamagePlayer's trap/mine padding.
    }
    let damage = packet.read_i32()?;
    let (monster_id, direction) = if skill != -4 && packet.remaining() >= 5 {
        (Some(packet.read_i32()?), Some(packet.read_u8()?))
    } else {
        (None, None)
    };
    Ok(RemotePlayerDamage {
        character_id,
        skill,
        damage,
        monster_id,
        direction,
    })
}

pub fn parse_remote_character_movement(
    bytes: &[u8],
) -> Result<RemoteCharacterMovement, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_CHARACTER_MOVED)?;
    let id = packet.read_i32()?;
    packet.skip(4)?;
    let fragments = parse_movements(&mut packet)?;
    Ok(RemoteCharacterMovement { id, fragments })
}

/// Builds the v83 client-to-server packet emitted for a player attack.
///
/// The reserved fields and ranged-attack suffix follow the legacy packet layout;
/// counts and per-target line lengths are validated before any bytes are emitted.
pub fn build_player_attack_packet(attack: &PlayerAttackPacket) -> Result<Vec<u8>, PacketError> {
    let target_count = attack.targets.len();
    let hit_count = usize::from(attack.hit_count);
    if target_count > 15 || !(1..=15).contains(&hit_count) {
        return Err(PacketError::InvalidAttackCounts {
            targets: target_count,
            hits: hit_count,
        });
    }
    for target in &attack.targets {
        if target.damage_lines.len() != hit_count {
            return Err(PacketError::InvalidAttackDamageCount {
                expected: hit_count,
                actual: target.damage_lines.len(),
            });
        }
        if let Some(damage) = target.damage_lines.iter().find(|damage| **damage < 0) {
            return Err(PacketError::InvalidAttackDamage(*damage));
        }
    }

    let opcode = match attack.attack_type {
        AttackType::Close => OPCODE_CLOSE_ATTACK,
        AttackType::Ranged => OPCODE_RANGED_ATTACK,
        AttackType::Magic => OPCODE_MAGIC_ATTACK,
    };
    let mut packet = PacketWriter::new(opcode);
    packet.write_u8(0);
    packet.write_u8(((target_count as u8) << 4) | attack.hit_count);
    packet.write_i32(attack.skill_id);
    if attack.charge > 0 {
        packet.write_i32(attack.charge);
    }
    write_zeros(&mut packet, 8);
    packet.write_u8(attack.display);
    packet.write_bool(attack.to_left);
    packet.write_u8(attack.stance);
    packet.write_u8(0);
    packet.write_i8(attack.speed);

    if attack.attack_type == AttackType::Ranged {
        packet.write_u8(0);
        packet.write_bool(attack.to_left);
        write_zeros(&mut packet, 7);
        if matches!(
            attack.skill_id,
            SKILL_RAPID_FIRE | SKILL_PIERCE | SKILL_PIERCE_ARROW | SKILL_RAPID_FIRE_CROSSBOW
        ) {
            write_zeros(&mut packet, 4);
        }
    } else {
        write_zeros(&mut packet, 4);
    }

    for target in &attack.targets {
        packet.write_i32(target.object_id);
        write_zeros(&mut packet, 14);
        for damage in &target.damage_lines {
            packet.write_i32(*damage);
        }
        if attack.skill_id != SKILL_RAPID_FIRE {
            write_zeros(&mut packet, 4);
        }
    }

    Ok(packet.into_bytes())
}

fn write_zeros(packet: &mut PacketWriter, count: usize) {
    packet.write_bytes(&[0; 14][..count.min(14)]);
    if count > 14 {
        let remaining = count - 14;
        for _ in 0..remaining {
            packet.write_u8(0);
        }
    }
}

/// Decodes the bounded v83 attack broadcast used to animate another character
/// and display its damage lines.
pub fn parse_remote_character_attack(bytes: &[u8]) -> Result<RemoteCharacterAttack, PacketError> {
    let mut packet = PacketReader::new(bytes);
    let opcode = packet.read_u16()?;
    let attack_type = match opcode {
        OPCODE_ATTACKED_CLOSE => RemoteAttackType::Close,
        OPCODE_ATTACKED_RANGED => RemoteAttackType::Ranged,
        OPCODE_ATTACKED_MAGIC => RemoteAttackType::Magic,
        actual => {
            return Err(PacketError::UnexpectedOpcode {
                expected: OPCODE_ATTACKED_CLOSE,
                actual,
            });
        }
    };
    let attacker_id = packet.read_i32()?;
    let counts = packet.read_u8()?;
    packet.skip(1)?;
    let skill_level = packet.read_u8()?;
    let skill_id = if skill_level > 0 {
        packet.read_i32()?
    } else {
        0
    };
    let display = packet.read_u8()?;
    let to_left = packet.read_bool()?;
    let stance = packet.read_u8()?;
    let speed = packet.read_i8()?;
    packet.skip(1)?;
    let bullet_id = packet.read_i32()?;
    let target_count = usize::from(counts >> 4);
    let hit_count = usize::from(counts & 0x0f);
    let mut targets = Vec::with_capacity(target_count);
    for _ in 0..target_count {
        let object_id = packet.read_i32()?;
        packet.skip(1)?;
        let damage_count = if skill_id == SKILL_MESO_EXPLOSION {
            usize::from(packet.read_u8()?)
        } else {
            hit_count
        };
        let mut damage_lines = Vec::with_capacity(damage_count);
        for _ in 0..damage_count {
            let raw_damage = packet.read_u32()?;
            damage_lines.push(RemoteAttackDamage {
                damage: (raw_damage & 0x7fff_ffff) as i32,
                critical: raw_damage & 0x8000_0000 != 0,
            });
        }
        targets.push(RemoteAttackTarget {
            object_id,
            damage_lines,
        });
    }
    Ok(RemoteCharacterAttack {
        attacker_id,
        attack_type,
        skill_level,
        skill_id,
        display,
        to_left,
        stance,
        speed,
        bullet_id,
        targets,
    })
}

pub fn parse_remote_character_look(bytes: &[u8]) -> Result<RemoteCharacterLookUpdate, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_UPDATE_CHARACTER_LOOK)?;
    let id = packet.read_i32()?;
    packet.skip(1)?;
    let look = read_character_look(&mut packet)?;
    Ok(RemoteCharacterLookUpdate { id, look })
}

pub fn parse_facial_expression(bytes: &[u8]) -> Result<RemoteFacialExpression, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_FACIAL_EXPRESSION)?;
    Ok(RemoteFacialExpression {
        character_id: packet.read_i32()?,
        expression: packet.read_i32()?,
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MovementKind {
    #[default]
    None,
    Absolute,
    Relative,
    JumpDown,
    Teleport,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MovementFragment {
    pub command: u8,
    pub kind: MovementKind,
    pub x: i16,
    pub y: i16,
    pub last_x: i16,
    pub last_y: i16,
    pub foothold: u16,
    pub stance: u8,
    pub duration: i16,
}

/// Absolute player state carried by the v83 MOVE_PLAYER fragment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerMovement {
    pub x: i16,
    pub y: i16,
    pub last_x: i16,
    pub last_y: i16,
    pub foothold: u16,
    /// Legacy character state byte, including the low bit used for facing left.
    pub stance: u8,
    pub duration: i16,
}

/// Builds the one-fragment absolute movement update emitted by the legacy client.
pub fn move_player(movement: PlayerMovement) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_MOVE_PLAYER);
    packet.write_bytes(&[0; 9]);
    packet.write_u8(1);
    packet.write_u8(0); // Absolute movement fragment.
    packet.write_point_i16(movement.x, movement.y);
    packet.write_point_i16(movement.last_x, movement.last_y);
    packet.write_u16(movement.foothold);
    packet.write_u8(movement.stance);
    packet.write_i16(movement.duration);
    packet.into_bytes()
}

/// Parses a movement-fragment count and its command-specific v83 layouts.
///
/// Unknown commands fail the entire parse because their unrecognized payload size
/// would make later fragments unsafe to interpret.
pub fn parse_movements(
    packet: &mut PacketReader<'_>,
) -> Result<Vec<MovementFragment>, PacketError> {
    let count = usize::from(packet.read_u8()?);
    let mut movements = Vec::with_capacity(count);
    for _ in 0..count {
        let command = packet.read_u8()?;
        let mut fragment = MovementFragment {
            command,
            ..Default::default()
        };
        match command {
            0 | 5 | 17 => {
                fragment.kind = MovementKind::Absolute;
                fragment.x = packet.read_i16()?;
                fragment.y = packet.read_i16()?;
                fragment.last_x = packet.read_i16()?;
                fragment.last_y = packet.read_i16()?;
                fragment.foothold = packet.read_u16()?;
                fragment.stance = packet.read_u8()?;
                fragment.duration = packet.read_i16()?;
            }
            1 | 2 | 6 | 12 | 13 | 16 | 18 | 19 | 20 | 22 => {
                fragment.kind = MovementKind::Relative;
                fragment.x = packet.read_i16()?;
                fragment.y = packet.read_i16()?;
                fragment.stance = packet.read_u8()?;
                fragment.duration = packet.read_i16()?;
            }
            3 | 4 | 7 | 8 | 9 | 11 => {
                fragment.kind = if command == 3 || command == 4 {
                    MovementKind::Teleport
                } else {
                    MovementKind::Absolute
                };
                fragment.x = packet.read_i16()?;
                fragment.y = packet.read_i16()?;
                fragment.last_x = packet.read_i16()?;
                fragment.last_y = packet.read_i16()?;
                fragment.stance = packet.read_u8()?;
            }
            15 => {
                fragment.kind = MovementKind::JumpDown;
                fragment.x = packet.read_i16()?;
                fragment.y = packet.read_i16()?;
                fragment.last_x = packet.read_i16()?;
                fragment.last_y = packet.read_i16()?;
                fragment.foothold = packet.read_u16()?;
                packet.skip(2)?;
                fragment.stance = packet.read_u8()?;
                fragment.duration = packet.read_i16()?;
            }
            10 => packet.skip(1)?,
            14 => packet.skip(9)?,
            21 => packet.skip(3)?,
            _ => return Err(PacketError::InvalidMovementCommand(command)),
        }
        movements.push(fragment);
    }
    Ok(movements)
}
