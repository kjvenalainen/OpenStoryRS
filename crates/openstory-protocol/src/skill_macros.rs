//! Bounded v83 skill-macro packets.

use crate::{PacketError, PacketReader, PacketWriter};

pub const OPCODE_SKILL_MACROS: u16 = 124;
pub const OPCODE_SKILL_MACRO_MODIFIED: u16 = 110;
pub const SKILL_MACRO_COUNT: usize = 3;
pub const SKILL_MACRO_SKILL_COUNT: usize = 3;
pub const MAX_SKILL_MACRO_NAME_CHARS: usize = 13;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkillMacro {
    pub name: String,
    pub shout: bool,
    pub skills: [i32; SKILL_MACRO_SKILL_COUNT],
}

/// Reads the server's configured skill macros. Servers may send fewer than the
/// three client slots; callers fill missing entries with the default macro.
pub fn parse_skill_macros(bytes: &[u8]) -> Result<Vec<SkillMacro>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    let opcode = packet.read_u16()?;
    if opcode != OPCODE_SKILL_MACROS {
        return Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_SKILL_MACROS,
            actual: opcode,
        });
    }
    let count = usize::from(packet.read_u8()?);
    if count > SKILL_MACRO_COUNT {
        return Err(PacketError::TooManySkillMacros(count));
    }

    let mut macros = Vec::with_capacity(count);
    for _ in 0..count {
        let name = packet.read_string()?;
        if name.chars().count() > MAX_SKILL_MACRO_NAME_CHARS {
            return Err(PacketError::SkillMacroNameTooLong(name.chars().count()));
        }
        let shout = packet.read_bool()?;
        let mut skills = [0; SKILL_MACRO_SKILL_COUNT];
        for skill_id in &mut skills {
            *skill_id = packet.read_i32()?;
            if *skill_id < 0 {
                return Err(PacketError::InvalidSkillMacroSkillId(*skill_id));
            }
        }
        macros.push(SkillMacro {
            name,
            shout,
            skills,
        });
    }
    Ok(macros)
}

/// Builds the v83 save request. The caller normally supplies all three slots.
pub fn save_skill_macros_packet(macros: &[SkillMacro]) -> Result<Vec<u8>, PacketError> {
    if macros.len() > SKILL_MACRO_COUNT {
        return Err(PacketError::TooManySkillMacros(macros.len()));
    }
    let mut packet = PacketWriter::new(OPCODE_SKILL_MACRO_MODIFIED);
    packet.write_u8(macros.len() as u8);
    for macro_entry in macros {
        let name_length = macro_entry.name.chars().count();
        if name_length > MAX_SKILL_MACRO_NAME_CHARS {
            return Err(PacketError::SkillMacroNameTooLong(name_length));
        }
        packet.write_string(&macro_entry.name)?;
        packet.write_bool(macro_entry.shout);
        for skill_id in macro_entry.skills {
            if skill_id < 0 {
                return Err(PacketError::InvalidSkillMacroSkillId(skill_id));
            }
            packet.write_i32(skill_id);
        }
    }
    Ok(packet.into_bytes())
}
