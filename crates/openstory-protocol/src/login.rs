use crate::inventory::{InventorySnapshot, parse_initial_inventory};
use crate::monster_book::{
    MonsterBookSnapshot, parse_initial_monster_book, skip_initial_minigame_and_rings,
};
use crate::quest::{QuestLog, SkillCooldown, parse_initial_cooldowns, parse_initial_quest_log};
use crate::skills::{SkillEntry, parse_initial_skillbook};
use crate::teleport_rock::TeleportRockLocations;
use crate::{PacketError, PacketReader, PacketWriter};

pub const OPCODE_LOGIN: u16 = 1;
pub const OPCODE_LOGIN_RESULT: u16 = 0;
pub const OPCODE_ACCEPT_TOS: u16 = 7;
pub const OPCODE_CHARLIST_REQUEST: u16 = 5;
pub const OPCODE_SERVERSTATUS_REQUEST: u16 = 6;
pub const OPCODE_SERVERLIST_REQUEST: u16 = 11;
pub const OPCODE_SERVERLIST_RESULT: u16 = 10;
pub const OPCODE_LAST_CONNECTED_WORLD: u16 = 26;
pub const OPCODE_RECOMMENDED_WORLDS: u16 = 27;
pub const OPCODE_CHARLIST_RESULT: u16 = 11;
pub const OPCODE_SERVERSTATUS_RESULT: u16 = 3;
pub const OPCODE_SERVER_IP: u16 = 12;
/// Login-server rejection returned after character selection.
pub const OPCODE_AFTER_LOGIN_ERROR: u16 = 9;
/// In-game channel-transfer response carrying the destination IPv4 address and port.
pub const OPCODE_CHANNEL_TRANSFER: u16 = 16;
pub const OPCODE_SELECT_CHARACTER: u16 = 19;
pub const OPCODE_REGISTER_PIC: u16 = 29;
pub const OPCODE_SELECT_CHARACTER_PIC: u16 = 30;
pub const OPCODE_CHECK_PIC_RESULT: u16 = 28;
pub const OPCODE_DELETE_CHARACTER: u16 = 23;
pub const OPCODE_DELETE_CHARACTER_RESULT: u16 = 15;
pub const OPCODE_PLAYER_LOGIN: u16 = 20;
pub const OPCODE_PONG: u16 = 24;
pub const OPCODE_LOGIN_START: u16 = 35;
pub const OPCODE_SET_FIELD: u16 = 125;
pub const OPCODE_CHARACTER_STAT_UPDATE: u16 = 31;
pub const OPCODE_CHARACTER_NAME_REQUEST: u16 = 21;
pub const OPCODE_CREATE_CHARACTER: u16 = 22;
pub const OPCODE_CHARACTER_NAME_RESULT: u16 = 13;
pub const OPCODE_CREATE_CHARACTER_RESULT: u16 = 14;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountInfo {
    pub account_id: i32,
    pub gender: u8,
    pub admin: bool,
    pub name: String,
    pub muted: bool,
    pub pin_disabled: bool,
    pub pic_state: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoginResult {
    Accepted(AccountInfo),
    Rejected { reason: i32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PicResult {
    pub reason: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeleteCharacterResult {
    pub character_id: i32,
    pub state: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldChannel {
    pub name: String,
    pub load: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldInfo {
    pub id: u8,
    pub name: String,
    pub flag: u8,
    pub message: String,
    pub channels: Vec<WorldChannel>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerListBatch {
    pub worlds: Vec<WorldInfo>,
    pub complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecommendedWorld {
    pub id: i32,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharacterStats {
    pub name: String,
    pub female: bool,
    pub pet_ids: [i64; 3],
    pub level: u8,
    pub job: u16,
    pub strength: u16,
    pub dexterity: u16,
    pub intelligence: u16,
    pub luck: u16,
    pub hp: u16,
    pub max_hp: u16,
    pub mp: u16,
    pub max_mp: u16,
    pub ability_points: u16,
    pub skill_points: u16,
    pub skill_points_by_job: Vec<(u8, u8)>,
    pub experience: i32,
    pub meso: Option<i32>,
    pub fame: u16,
    pub map_id: i32,
    pub portal: u8,
    pub rank: Option<(i32, i8)>,
    pub job_rank: Option<(i32, i8)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharacterLook {
    pub female: bool,
    pub skin: u8,
    pub face_id: i32,
    pub hair_id: i32,
    pub equips: Vec<(i8, i32)>,
    pub masked_equips: Vec<(i8, i32)>,
    pub weapon_id: i32,
    pub pet_ids: [i32; 3],
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharacterInfo {
    pub id: i32,
    pub stats: CharacterStats,
    pub look: CharacterLook,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterList {
    pub channel_id: u8,
    pub characters: Vec<CharacterInfo>,
    pub pic_state: u8,
    pub slots: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerStatus {
    Normal,
    HighlyPopulated,
    Full,
    Unknown(i16),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerRedirect {
    pub address: std::net::Ipv4Addr,
    pub port: u16,
    pub character_id: i32,
}

/// Parses a login-server rejection returned after character selection.
pub fn parse_after_login_error(body: &[u8]) -> Result<i16, PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_AFTER_LOGIN_ERROR)?;
    packet.read_i16()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelTransfer {
    pub address: std::net::Ipv4Addr,
    pub port: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapChange {
    pub map_id: i32,
    pub portal: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetField {
    MapChange(MapChange),
    CharacterEntry {
        character_id: i32,
        stats: CharacterStats,
        inventory: Option<InventorySnapshot>,
        skills: Vec<SkillEntry>,
        cooldowns: Vec<SkillCooldown>,
        quest_log: Option<QuestLog>,
        teleport_rock: Option<TeleportRockLocations>,
        monster_book: Option<MonsterBookSnapshot>,
    },
}

/// Decodes the account result packet and the account data sent on success.
pub fn parse_login_result(body: &[u8]) -> Result<LoginResult, PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_LOGIN_RESULT)?;
    let reason = packet.read_i32()?;
    if reason != 0 {
        return Ok(LoginResult::Rejected { reason });
    }

    packet.skip(2)?;
    let account_id = packet.read_i32()?;
    let gender = packet.read_u8()?;
    let admin = packet.read_bool()?;
    packet.skip(2)?;
    let name = packet.read_string()?;
    packet.skip(1)?;
    let muted = packet.read_bool()?;
    packet.read_i64()?; // mute expiry
    packet.read_i64()?; // account creation timestamp
    packet.skip(4)?; // legacy world selection message
    let pin_disabled = packet.read_bool()?;
    let pic_state = packet.read_u8()?;

    Ok(LoginResult::Accepted(AccountInfo {
        account_id,
        gender,
        admin,
        name,
        muted,
        pin_disabled,
        pic_state,
    }))
}

/// Parses one v83 server-list packet, which may contain worlds, the terminator,
/// or both. Cosmic sends one world per packet and a separate terminator packet.
pub fn parse_server_list(body: &[u8]) -> Result<ServerListBatch, PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_SERVERLIST_RESULT)?;
    let mut worlds = Vec::new();

    loop {
        if packet.is_empty() {
            return if worlds.is_empty() {
                Err(PacketError::MissingWorldTerminator)
            } else {
                Ok(ServerListBatch {
                    worlds,
                    complete: false,
                })
            };
        }
        let id = packet.read_i8()?;
        if id == -1 {
            return Ok(ServerListBatch {
                worlds,
                complete: true,
            });
        }

        let name = packet.read_string()?;
        let flag = packet.read_u8()?;
        let message = packet.read_string()?;
        packet.skip(5)?;
        let channel_count = usize::from(packet.read_u8()?);
        let mut channels = Vec::with_capacity(channel_count);
        for _ in 0..channel_count {
            let channel_name = packet.read_string()?;
            let load = packet.read_i32()?;
            packet.skip(3)?;
            channels.push(WorldChannel {
                name: channel_name,
                load,
            });
        }
        packet.skip(2)?;
        worlds.push(WorldInfo {
            id: id as u8,
            name,
            flag,
            message,
            channels,
        });
    }
}

/// Reads the server's preferred world ID. `-1` means no prior selection.
pub fn parse_last_connected_world(body: &[u8]) -> Result<i32, PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_LAST_CONNECTED_WORLD)?;
    packet.read_i32()
}

/// Parses the v83 recommended-world metadata packet. The legacy v83 client
/// consumes this data but does not display it.
pub fn parse_recommended_worlds(body: &[u8]) -> Result<Vec<RecommendedWorld>, PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_RECOMMENDED_WORLDS)?;
    let count = packet.read_u8()?;
    if count > 64 {
        return Err(PacketError::TooManyRecommendedWorlds(count));
    }

    let mut worlds = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let id = packet.read_i32()?;
        let message = if id == -1 {
            String::new()
        } else {
            packet.read_string()?
        };
        worlds.push(RecommendedWorld { id, message });
    }
    Ok(worlds)
}

/// Parses the v83 character-list response, including each character's stats,
/// equipped items, masked equipment, and optional ranking fields.
pub fn parse_character_list(body: &[u8]) -> Result<CharacterList, PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_CHARLIST_RESULT)?;
    let channel_id = packet.read_u8()?;
    let count = packet.read_u8()?;
    if count > 64 {
        return Err(PacketError::InvalidCharacterCount(count));
    }
    let mut characters = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        characters.push(read_character(&mut packet)?);
    }
    let pic_state = packet.read_u8()?;
    let slots = packet.read_i32()?;
    Ok(CharacterList {
        channel_id,
        characters,
        pic_state,
        slots,
    })
}

/// Parses the name availability response used by the v83 character creator.
pub fn parse_character_name_result(body: &[u8]) -> Result<(String, bool), PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_CHARACTER_NAME_RESULT)?;
    Ok((packet.read_string()?, packet.read_bool()?))
}

/// Parses the character entry appended after a successful creation request.
pub fn parse_created_character(body: &[u8]) -> Result<CharacterInfo, PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_CREATE_CHARACTER_RESULT)?;
    packet.skip(1)?; // result/status byte consumed by the legacy client
    read_character(&mut packet)
}

/// Decodes the login server's population response and preserves unfamiliar
/// status values so callers can display an appropriate message.
pub fn parse_server_status(body: &[u8]) -> Result<ServerStatus, PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_SERVERSTATUS_RESULT)?;
    Ok(match packet.read_i16()? {
        0 => ServerStatus::Normal,
        1 => ServerStatus::HighlyPopulated,
        2 => ServerStatus::Full,
        value => ServerStatus::Unknown(value),
    })
}

/// Parses the v83 server-IP packet emitted after character selection.
pub fn parse_server_redirect(body: &[u8]) -> Result<ServerRedirect, PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_SERVER_IP)?;
    packet.skip(2)?; // redirect flags consumed by the legacy client
    let octets: [u8; 4] = packet
        .read_bytes(4)?
        .try_into()
        .expect("length checked by packet reader");
    let port = packet.read_u16()?;
    let character_id = packet.read_i32()?;
    Ok(ServerRedirect {
        address: std::net::Ipv4Addr::from(octets),
        port,
        character_id,
    })
}

/// Parses the v83 in-game channel-transfer response. After the opcode the
/// server sends one reserved byte, four IPv4 octets, and a little-endian port.
pub fn parse_channel_transfer(body: &[u8]) -> Result<ChannelTransfer, PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_CHANNEL_TRANSFER)?;
    packet.skip(1)?; // reserved byte consumed by the legacy client
    let octets: [u8; 4] = packet
        .read_bytes(4)?
        .try_into()
        .expect("length checked by packet reader");
    Ok(ChannelTransfer {
        address: std::net::Ipv4Addr::from(octets),
        port: packet.read_u16()?,
    })
}

/// Parses the map-change and initial character-entry headers from `SET_FIELD`.
/// The initial form carries more inventory state after the character stats; this
/// parser consumes the bounded stats prefix needed to identify the actual map.
pub fn parse_set_field(body: &[u8]) -> Result<SetField, PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_SET_FIELD)?;
    packet.read_i32()?; // channel id
    let mode1 = packet.read_i8()?;
    let mode2 = packet.read_i8()?;
    if mode1 == 0 && mode2 == 0 {
        packet.skip(3)?;
        return Ok(SetField::MapChange(MapChange {
            map_id: packet.read_i32()?,
            portal: packet.read_u8()?,
        }));
    }
    packet.skip(23)?;
    let character_id = packet.read_i32()?;
    let stats = read_character_stats(&mut packet)?;
    let inventory = if packet.is_empty() {
        None
    } else {
        packet.read_u8()?; // buddy capacity
        if packet.read_bool()? {
            packet.read_string()?; // linked character name
        }
        Some(parse_initial_inventory(&mut packet)?)
    };
    let skills = if inventory.is_some() && !packet.is_empty() {
        parse_initial_skillbook(&mut packet)?
    } else {
        Vec::new()
    };
    let (cooldowns, quest_log) = if inventory.is_some() && !packet.is_empty() {
        let cooldowns = parse_initial_cooldowns(&mut packet)?;
        (cooldowns, Some(parse_initial_quest_log(&mut packet)?))
    } else {
        (Vec::new(), None)
    };
    let (teleport_rock, monster_book) = if inventory.is_some() && !packet.is_empty() {
        skip_initial_minigame_and_rings(&mut packet)?;
        let normal = read_initial_map_ids(&mut packet, 5)?;
        let vip = read_initial_map_ids(&mut packet, 10)?;
        let teleport_rock = TeleportRockLocations { normal, vip };
        let monster_book = parse_initial_monster_book(&mut packet)?;
        (Some(teleport_rock), Some(monster_book))
    } else {
        (None, None)
    };
    Ok(SetField::CharacterEntry {
        character_id,
        stats,
        inventory,
        skills,
        cooldowns,
        quest_log,
        teleport_rock,
        monster_book,
    })
}

fn read_initial_map_ids(
    packet: &mut PacketReader<'_>,
    count: usize,
) -> Result<Vec<i32>, PacketError> {
    (0..count).map(|_| packet.read_i32()).collect()
}

/// Convenience parser for callers interested only in an in-channel map change.
pub fn parse_map_change(body: &[u8]) -> Result<Option<MapChange>, PacketError> {
    match parse_set_field(body)? {
        SetField::MapChange(change) => Ok(Some(change)),
        SetField::CharacterEntry { .. } => Ok(None),
    }
}

fn read_character(packet: &mut PacketReader<'_>) -> Result<CharacterInfo, PacketError> {
    let id = packet.read_i32()?;
    let mut stats = read_character_stats(packet)?;
    let look = read_character_look(packet)?;

    packet.read_bool()?; // rank info is present
    if packet.read_bool()? {
        let rank = packet.read_i32()?;
        let rank_movement = packet.read_i32()?;
        let job_rank = packet.read_i32()?;
        let job_rank_movement = packet.read_i32()?;
        stats.rank = Some((rank, movement_sign(rank_movement)));
        stats.job_rank = Some((job_rank, movement_sign(job_rank_movement)));
    }

    Ok(CharacterInfo { id, stats, look })
}

fn read_character_stats(packet: &mut PacketReader<'_>) -> Result<CharacterStats, PacketError> {
    let name = packet.read_string_bytes(13)?;
    let female = packet.read_bool()?;
    let _skin = packet.read_u8()?;
    let _face_id = packet.read_i32()?;
    let _hair_id = packet.read_i32()?;
    let mut pet_ids = [0_i64; 3];
    for pet_id in &mut pet_ids {
        *pet_id = packet.read_i64()?;
    }
    let level = packet.read_u8()?;
    let job = packet.read_u16()?;
    let strength = packet.read_u16()?;
    let dexterity = packet.read_u16()?;
    let intelligence = packet.read_u16()?;
    let luck = packet.read_u16()?;
    let hp = packet.read_u16()?;
    let max_hp = packet.read_u16()?;
    let mp = packet.read_u16()?;
    let max_mp = packet.read_u16()?;
    let ability_points = packet.read_u16()?;
    let skill_points = packet.read_u16()?;
    let experience = packet.read_i32()?;
    let fame = packet.read_u16()?;
    packet.skip(4)?; // gacha experience
    let map_id = packet.read_i32()?;
    let portal = packet.read_u8()?;
    packet.skip(4)?; // timestamp
    Ok(CharacterStats {
        name,
        female,
        pet_ids,
        level,
        job,
        strength,
        dexterity,
        intelligence,
        luck,
        hp,
        max_hp,
        mp,
        max_mp,
        ability_points,
        skill_points,
        skill_points_by_job: Vec::new(),
        experience,
        meso: None,
        fame,
        map_id,
        portal,
        rank: None,
        job_rank: None,
    })
}

pub(crate) fn read_character_look(
    packet: &mut PacketReader<'_>,
) -> Result<CharacterLook, PacketError> {
    let female = packet.read_bool()?;
    let skin = packet.read_u8()?;
    let face_id = packet.read_i32()?;
    packet.read_bool()?; // megaphone
    let hair_id = packet.read_i32()?;
    let equips = read_equipment_slots(packet)?;
    let masked_equips = read_equipment_slots(packet)?;
    let weapon_id = packet.read_i32()?;
    let mut pet_ids = [0_i32; 3];
    for pet_id in &mut pet_ids {
        *pet_id = packet.read_i32()?;
    }
    Ok(CharacterLook {
        female,
        skin,
        face_id,
        hair_id,
        equips,
        masked_equips,
        weapon_id,
        pet_ids,
    })
}

fn read_equipment_slots(packet: &mut PacketReader<'_>) -> Result<Vec<(i8, i32)>, PacketError> {
    let mut equips = Vec::new();
    loop {
        let slot = packet.read_u8()?;
        if slot == u8::MAX {
            return Ok(equips);
        }
        if equips.len() >= 64 {
            return Err(PacketError::TooManyEquipmentSlots(64));
        }
        equips.push((slot as i8, packet.read_i32()?));
    }
}

fn movement_sign(value: i32) -> i8 {
    match value.cmp(&0) {
        std::cmp::Ordering::Less => b'-' as i8,
        std::cmp::Ordering::Equal => b'=' as i8,
        std::cmp::Ordering::Greater => b'+' as i8,
    }
}

pub(crate) fn expect_opcode(
    packet: &mut PacketReader<'_>,
    expected: u16,
) -> Result<(), PacketError> {
    let actual = packet.read_u16()?;
    if actual == expected {
        Ok(())
    } else {
        Err(PacketError::UnexpectedOpcode { expected, actual })
    }
}

pub fn login_start() -> Vec<u8> {
    PacketWriter::new(OPCODE_LOGIN_START).into_bytes()
}

/// Confirms the server's Terms of Service after the user explicitly accepts.
pub fn accept_terms_of_service() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_ACCEPT_TOS);
    packet.write_u8(1);
    packet.into_bytes()
}

/// Builds the legacy v83 account login request. `machine_id` is emitted in
/// little-endian byte order, matching the C++ client's reversed volume serial.
pub fn login_request(
    account: &str,
    password: &str,
    machine_id: u32,
) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_LOGIN);
    packet.write_string(account)?;
    packet.write_string(password)?;
    packet.write_bytes(&[0; 6]);
    packet.write_bytes(&machine_id.to_le_bytes());
    Ok(packet.into_bytes())
}

pub fn request_world_list() -> Vec<u8> {
    PacketWriter::new(OPCODE_SERVERLIST_REQUEST).into_bytes()
}

pub fn request_characters(world: u8, channel: u8) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_CHARLIST_REQUEST);
    packet.write_u8(0);
    packet.write_u8(world);
    packet.write_u8(channel);
    packet.into_bytes()
}

/// Requests a name reservation/check before sending the character appearance.
pub fn request_character_name(name: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_CHARACTER_NAME_REQUEST);
    packet.write_string(name)?;
    Ok(packet.into_bytes())
}

/// Builds the legacy v83 character creation packet. The server expects all
/// appearance fields, including job and hair color, as 32-bit integers.
#[allow(clippy::too_many_arguments)]
pub fn create_character(
    name: &str,
    job: u16,
    face: i32,
    hair: i32,
    hair_color: u8,
    skin: u8,
    top: i32,
    bottom: i32,
    shoes: i32,
    weapon: i32,
    female: bool,
) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_CREATE_CHARACTER);
    packet.write_string(name)?;
    packet.write_i32(i32::from(job));
    packet.write_i32(face);
    packet.write_i32(hair);
    packet.write_i32(i32::from(hair_color));
    packet.write_i32(i32::from(skin));
    packet.write_i32(top);
    packet.write_i32(bottom);
    packet.write_i32(shoes);
    packet.write_i32(weapon);
    packet.write_bool(female);
    Ok(packet.into_bytes())
}

pub fn request_server_status(world: i16) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_SERVERSTATUS_REQUEST);
    packet.write_i16(world);
    packet.into_bytes()
}

pub fn select_character(
    character_id: i32,
    mac_addresses: &str,
    hardware_id: &str,
) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_SELECT_CHARACTER);
    packet.write_i32(character_id);
    packet.write_string(mac_addresses)?;
    packet.write_string(hardware_id)?;
    Ok(packet.into_bytes())
}

/// Builds the v83 PIC registration request for a character. The leading
/// reserved byte is present in the legacy client's opcode 29 packet.
pub fn register_character_pic(
    character_id: i32,
    pic: &str,
    mac_addresses: &str,
    hardware_id: &str,
) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_REGISTER_PIC);
    packet.write_u8(0);
    packet.write_i32(character_id);
    packet.write_string(mac_addresses)?;
    packet.write_string(hardware_id)?;
    packet.write_string(pic)?;
    Ok(packet.into_bytes())
}

/// Builds the v83 PIC verification request for a character.
pub fn select_character_with_pic(
    pic: &str,
    character_id: i32,
    mac_addresses: &str,
    hardware_id: &str,
) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_SELECT_CHARACTER_PIC);
    packet.write_string(pic)?;
    packet.write_i32(character_id);
    packet.write_string(mac_addresses)?;
    packet.write_string(hardware_id)?;
    Ok(packet.into_bytes())
}

/// Parses the one-byte response used by the legacy PIC check handler.
pub fn parse_pic_result(body: &[u8]) -> Result<PicResult, PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_CHECK_PIC_RESULT)?;
    Ok(PicResult {
        reason: packet.read_u8()?,
    })
}

/// Builds a v83 character deletion request. The PIC field is empty when the
/// account has PIC disabled and contains the verified PIC otherwise.
pub fn delete_character(character_id: i32, pic: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_DELETE_CHARACTER);
    packet.write_string(pic)?;
    packet.write_i32(character_id);
    Ok(packet.into_bytes())
}

/// Parses the result of a character deletion request.
pub fn parse_delete_character_result(body: &[u8]) -> Result<DeleteCharacterResult, PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_DELETE_CHARACTER_RESULT)?;
    Ok(DeleteCharacterResult {
        character_id: packet.read_i32()?,
        state: packet.read_u8()?,
    })
}

pub fn player_login(character_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PLAYER_LOGIN);
    packet.write_i32(character_id);
    packet.into_bytes()
}

pub fn pong() -> Vec<u8> {
    PacketWriter::new(OPCODE_PONG).into_bytes()
}
