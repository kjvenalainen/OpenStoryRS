//! v83 saved-map state for Teleport Rocks.

use crate::login::expect_opcode;
use crate::{PacketError, PacketReader};

pub const OPCODE_MAP_TRANSFER_RESULT: u16 = 42;
const EMPTY_MAP_ID: i32 = 999_999_999;
const NORMAL_SLOTS: usize = 5;
const VIP_SLOTS: usize = 10;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TeleportRockLocations {
    pub normal: Vec<i32>,
    pub vip: Vec<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeleportRockUpdate {
    pub vip: bool,
    pub map_ids: Vec<i32>,
}

/// Parses the server's full saved-map list after a Teleport Rock add or delete.
/// Mode 2 removes and mode 3 adds; both carry the replacement list.
pub fn parse_teleport_rock_update(bytes: &[u8]) -> Result<Option<TeleportRockUpdate>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_MAP_TRANSFER_RESULT)?;
    let mode = packet.read_i8()?;
    let vip = packet.read_i8()? != 0;
    if mode != 2 && mode != 3 {
        return Ok(None);
    }

    let maximum = if vip { VIP_SLOTS } else { NORMAL_SLOTS };
    let count = (packet.remaining() / 4).min(maximum);
    let mut map_ids = Vec::with_capacity(count);
    for _ in 0..count {
        let map_id = packet.read_i32()?;
        if map_id != EMPTY_MAP_ID {
            map_ids.push(map_id);
        }
    }
    packet.skip(packet.remaining())?;
    Ok(Some(TeleportRockUpdate { vip, map_ids }))
}
