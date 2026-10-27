use crate::{PacketError, PacketReader};

pub const OPCODE_BRIDLE_MOB_CATCH_FAIL: u16 = 79;
pub const OPCODE_CATCH_MONSTER: u16 = 251;
pub const OPCODE_CATCH_MONSTER_WITH_ITEM: u16 = 252;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatchResult {
    Ordinary { success: bool },
    WithItem { success: bool },
    BridleFailure { reason: i8 },
}

/// Parses v83 catch results and consumes the associated mob/item identifiers.
pub fn parse_catch_result(bytes: &[u8]) -> Result<CatchResult, PacketError> {
    let mut packet = PacketReader::new(bytes);
    let opcode = packet.read_u16()?;
    match opcode {
        OPCODE_CATCH_MONSTER => {
            packet.read_i32()?; // mob object id
            let success = packet.remaining() > 0 && packet.read_i8()? == 1;
            Ok(CatchResult::Ordinary { success })
        }
        OPCODE_CATCH_MONSTER_WITH_ITEM => {
            packet.read_i32()?; // mob object id
            packet.read_i32()?; // catching item id
            Ok(CatchResult::WithItem {
                success: packet.read_i8()? != 0,
            })
        }
        OPCODE_BRIDLE_MOB_CATCH_FAIL => {
            let reason = packet.read_i8()?;
            packet.read_i32()?;
            packet.read_i32()?;
            Ok(CatchResult::BridleFailure { reason })
        }
        _ => Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_CATCH_MONSTER,
            actual: opcode,
        }),
    }
}
