//! Learned-skill state and v83 skill packets.

use crate::{PacketError, PacketReader, PacketWriter};

pub const OPCODE_UPDATE_SKILL: u16 = 36;
pub const OPCODE_SPEND_SKILL_POINT: u16 = 90;
pub const OPCODE_USE_SKILL: u16 = 91;
const MAX_SKILL_ENTRIES: usize = 512;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SkillEntry {
    pub id: i32,
    pub level: i32,
    pub master_level: i32,
    pub expiration: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillUpdate {
    pub enable_actions: bool,
    pub skills: Vec<SkillEntry>,
}

/// Reads the skillbook section of the initial character-entry packet.
pub(crate) fn parse_initial_skillbook(
    packet: &mut PacketReader<'_>,
) -> Result<Vec<SkillEntry>, PacketError> {
    let count = read_count(packet)?;
    let mut skills = Vec::with_capacity(count);
    for _ in 0..count {
        let id = packet.read_i32()?;
        let level = packet.read_i32()?;
        let expiration = packet.read_i64()?;
        let master_level = if (id % 100_000) / 10_000 == 2 {
            packet.read_i32()?
        } else {
            0
        };
        skills.push(SkillEntry {
            id,
            level,
            master_level,
            expiration,
        });
    }
    Ok(skills)
}

/// Parses the v83 `UPDATE_SKILL` packet, including its optional one-byte suffix.
pub fn parse_skill_update(bytes: &[u8]) -> Result<SkillUpdate, PacketError> {
    let mut packet = PacketReader::new(bytes);
    let opcode = packet.read_u16()?;
    if opcode != OPCODE_UPDATE_SKILL {
        return Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_UPDATE_SKILL,
            actual: opcode,
        });
    }
    let enable_actions = packet.read_bool()?;
    let count = read_count(&mut packet)?;
    let mut skills = Vec::with_capacity(count);
    for _ in 0..count {
        skills.push(SkillEntry {
            id: packet.read_i32()?,
            level: packet.read_i32()?,
            master_level: packet.read_i32()?,
            expiration: packet.read_i64()?,
        });
    }
    if !packet.is_empty() {
        packet.read_u8()?;
    }
    Ok(SkillUpdate {
        enable_actions,
        skills,
    })
}

/// Builds a request to raise a skill by one level using one skill point.
pub fn spend_skill_point_packet(skill_id: i32, client_tick: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_SPEND_SKILL_POINT);
    packet.write_i32(client_tick);
    packet.write_i32(skill_id);
    packet.into_bytes()
}

/// Builds a v83 active-skill use request. Monster Magnet has two reserved bytes.
pub fn use_skill_packet(skill_id: i32, level: u8, client_tick: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_USE_SKILL);
    packet.write_i32(client_tick);
    packet.write_i32(skill_id);
    packet.write_u8(level);
    if skill_id % 10_000_000 == 1_004 {
        packet.write_u16(0);
    }
    packet.into_bytes()
}

fn read_count(packet: &mut PacketReader<'_>) -> Result<usize, PacketError> {
    let raw_count = packet.read_i16()?;
    if raw_count < 0 {
        return Err(PacketError::InvalidSkillCount(raw_count));
    }
    let count = raw_count as usize;
    if count > MAX_SKILL_ENTRIES {
        return Err(PacketError::TooManySkillEntries(MAX_SKILL_ENTRIES));
    }
    Ok(count)
}
