//! Bounded v83 messenger-room packets and requests.

use crate::login::{expect_opcode, read_character_look};
use crate::packet::PacketWriter;
use crate::{CharacterLook, PacketError, PacketReader};

/// Server-to-client messenger-room operation (legacy receive opcode 313 / `0x139`).
pub const OPCODE_MESSENGER: u16 = 313;

/// Client-to-server messenger-room request (legacy send opcode 122 / `0x7A`).
pub const OPCODE_MESSENGER_SEND: u16 = 122;

/// A decoded v83 messenger-room operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessengerEvent {
    /// Operation `0x00`: a member joined with their appearance and channel.
    PlayerJoined {
        slot: i8,
        look: CharacterLook,
        name: String,
        channel: u8,
        reserved: u8,
    },
    /// Operation `0x01`: the server assigned this client's room slot.
    SlotAssigned { slot: i8 },
    /// Operation `0x02`: a room member left.
    PlayerLeft { slot: i8 },
    /// Operation `0x03`: another player invited us to a messenger room.
    Invite { from: String, messenger_id: i32 },
    /// Operation `0x05`: an invite decline/note from another player.
    Note { text: String },
    /// Operation `0x06`: raw relayed chat text, conventionally `name : message`.
    Chat { text: String },
    /// An operation this client does not yet interpret. The remainder of that
    /// packet is intentionally left opaque because unknown operations have no
    /// known v83 payload shape.
    Unsupported { mode: u8 },
}

/// Parses a v83 server messenger packet. All fields are read through the bounded
/// packet reader; member appearance uses the same bounded character-look parser
/// as the login and character-spawn paths.
pub fn parse_messenger(bytes: &[u8]) -> Result<MessengerEvent, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_MESSENGER)?;
    Ok(match packet.read_u8()? {
        0x00 => MessengerEvent::PlayerJoined {
            slot: packet.read_i8()?,
            look: read_character_look(&mut packet)?,
            name: packet.read_string()?,
            channel: packet.read_u8()?,
            reserved: packet.read_u8()?,
        },
        0x01 => MessengerEvent::SlotAssigned {
            slot: packet.read_i8()?,
        },
        0x02 => MessengerEvent::PlayerLeft {
            slot: packet.read_i8()?,
        },
        0x03 => {
            let from = packet.read_string()?;
            packet.read_u8()?; // Reserved byte.
            let messenger_id = packet.read_i32()?;
            packet.read_u8()?; // Reserved byte.
            MessengerEvent::Invite { from, messenger_id }
        }
        0x05 => MessengerEvent::Note {
            text: packet.read_string()?,
        },
        0x06 => MessengerEvent::Chat {
            text: packet.read_string()?,
        },
        mode => MessengerEvent::Unsupported { mode },
    })
}

/// Requests an empty messenger room (client operation `0x00`).
pub fn messenger_open_packet() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_MESSENGER_SEND);
    packet.write_u8(0x00);
    packet.into_bytes()
}

/// Accepts an invitation. The v83 client accepts by sending the ordinary
/// messenger-open operation; the invite's messenger ID is not part of that
/// request payload.
pub fn messenger_accept_invite_packet() -> Vec<u8> {
    messenger_open_packet()
}

/// Invites a named player (client operation `0x03`).
pub fn messenger_invite_packet(name: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_MESSENGER_SEND);
    packet.write_u8(0x03);
    packet.write_string(name)?;
    Ok(packet.into_bytes())
}

/// Sends a messenger chat line (client operation `0x06`).
pub fn messenger_chat_packet(message: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_MESSENGER_SEND);
    packet.write_u8(0x06);
    packet.write_string(message)?;
    Ok(packet.into_bytes())
}

/// Leaves the current messenger room (client operation `0x02`).
pub fn messenger_leave_packet() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_MESSENGER_SEND);
    packet.write_u8(0x02);
    packet.into_bytes()
}

/// Declines a messenger invitation with a note (client operation `0x05`).
pub fn messenger_decline_invite_packet(
    inviter_name: &str,
    text: &str,
) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_MESSENGER_SEND);
    packet.write_u8(0x05);
    packet.write_string(text)?;
    packet.write_string(inviter_name)?;
    packet.write_u8(0); // Legacy reserved byte.
    Ok(packet.into_bytes())
}
