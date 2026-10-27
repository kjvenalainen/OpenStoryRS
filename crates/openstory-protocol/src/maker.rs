use crate::{PacketError, PacketReader};

pub const OPCODE_MAKER_RESULT: u16 = 217;
const MAX_MAKER_ITEMS: usize = 512;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MakerResult {
    Craft {
        failed: bool,
        item: Option<(i32, i32)>,
    },
    Crystal {
        gained_item_id: i32,
        consumed_item_id: i32,
    },
    Disassembly {
        item_id: i32,
        gained: Vec<(i32, i32)>,
    },
    Unsupported {
        result: i32,
        mode: i32,
    },
}

/// Parses v83 MAKER_RESULT packets and consumes all mode-specific payload fields.
pub fn parse_maker_result(bytes: &[u8]) -> Result<MakerResult, PacketError> {
    let mut packet = PacketReader::new(bytes);
    let opcode = packet.read_u16()?;
    if opcode != OPCODE_MAKER_RESULT {
        return Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_MAKER_RESULT,
            actual: opcode,
        });
    }
    let result = packet.read_i32()?;
    let mode = packet.read_i32()?;
    match mode {
        1 | 2 => {
            let failed = packet.read_i8()? != 0;
            let item = if failed {
                None
            } else {
                Some((packet.read_i32()?, packet.read_i32()?))
            };
            discard_maker_entries(&mut packet, 8)?; // consumed item id + quantity pairs
            discard_maker_entries(&mut packet, 4)?; // gem ids
            let has_catalyst = packet.read_i8()? != 0;
            if has_catalyst {
                packet.read_i32()?;
            }
            packet.read_i32()?; // meso cost
            Ok(MakerResult::Craft { failed, item })
        }
        3 => Ok(MakerResult::Crystal {
            gained_item_id: packet.read_i32()?,
            consumed_item_id: packet.read_i32()?,
        }),
        4 => {
            let item_id = packet.read_i32()?;
            let count = read_maker_count(&mut packet, 8)?;
            let mut gained = Vec::with_capacity(count);
            for _ in 0..count {
                gained.push((packet.read_i32()?, packet.read_i32()?));
            }
            packet.read_i32()?; // meso cost
            Ok(MakerResult::Disassembly { item_id, gained })
        }
        _ => Ok(MakerResult::Unsupported { result, mode }),
    }
}

fn discard_maker_entries(
    packet: &mut PacketReader<'_>,
    bytes_per_entry: usize,
) -> Result<(), PacketError> {
    let count = read_maker_count(packet, bytes_per_entry)?;
    packet.skip(count * bytes_per_entry)
}

fn read_maker_count(
    packet: &mut PacketReader<'_>,
    bytes_per_entry: usize,
) -> Result<usize, PacketError> {
    let count = packet.read_i32()?;
    if count <= 0 {
        return Ok(0);
    }
    let count = count as usize;
    if count > MAX_MAKER_ITEMS || count > packet.remaining() / bytes_per_entry {
        return Err(PacketError::TooManyMakerEntries(count));
    }
    Ok(count)
}
