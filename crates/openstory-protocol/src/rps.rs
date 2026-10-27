//! v83 Rock-Paper-Scissors NPC minigame packets.

use crate::{PacketError, PacketReader, PacketWriter};

pub const OPCODE_RPS_ACTION: u16 = 136;
pub const OPCODE_RPS_GAME: u16 = 312;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RpsGameEvent {
    Open { npc_id: i32 },
    SelectionResult { player: i8, opponent: i8 },
    NotEnoughMesos,
    Close,
    Unknown(i8),
}

pub fn parse_rps_game(bytes: &[u8]) -> Result<RpsGameEvent, PacketError> {
    let mut packet = PacketReader::new(bytes);
    let opcode = packet.read_u16()?;
    if opcode != OPCODE_RPS_GAME {
        return Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_RPS_GAME,
            actual: opcode,
        });
    }

    Ok(match packet.read_i8()? {
        8 => RpsGameEvent::Open {
            npc_id: packet.read_i32()?,
        },
        0x0b => RpsGameEvent::SelectionResult {
            player: packet.read_i8()?,
            opponent: packet.read_i8()?,
        },
        0x06 => RpsGameEvent::NotEnoughMesos,
        0x0d => RpsGameEvent::Close,
        other => RpsGameEvent::Unknown(other),
    })
}

pub fn rps_selection_packet(selection: u8) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_RPS_ACTION);
    packet.write_u8(selection);
    packet.into_bytes()
}
