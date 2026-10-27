use crate::{PacketError, PacketReader};

pub const OPCODE_SPAWN_GUIDE: u16 = 223;
pub const OPCODE_TALK_GUIDE: u16 = 224;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuideEvent {
    Spawn { visible: bool },
    Text(String),
    Hint { id: i32, duration_ms: i32 },
}

/// Parses Maple Guide visibility and server-authored hint packets.
pub fn parse_guide_packet(bytes: &[u8]) -> Result<Option<GuideEvent>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    let opcode = packet.read_u16()?;
    match opcode {
        OPCODE_SPAWN_GUIDE => {
            if packet.is_empty() {
                return Ok(None);
            }
            Ok(Some(GuideEvent::Spawn {
                visible: packet.read_bool()?,
            }))
        }
        OPCODE_TALK_GUIDE => {
            let mode = packet.read_i8()?;
            match mode {
                0 => {
                    let text = packet.read_string()?;
                    packet.read_i32()?; // text delay
                    packet.read_i32()?; // display duration
                    Ok(Some(GuideEvent::Text(text)))
                }
                1 => Ok(Some(GuideEvent::Hint {
                    id: packet.read_i32()?,
                    duration_ms: packet.read_i32()?,
                })),
                _ => Ok(None),
            }
        }
        actual => Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_SPAWN_GUIDE,
            actual,
        }),
    }
}
