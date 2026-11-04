//! Typed builders for reviewed v83 requests that do not fit an existing family.
//!
//! Opcode values here belong to the client-to-server namespace. In particular,
//! these values may overlap with unrelated server-to-client opcodes.

use crate::{PacketError, PacketWriter};

pub const OPCODE_SET_GENDER_SEND: u16 = 8;
pub const OPCODE_MONSTER_BOOK_COVER_SEND: u16 = 57;
pub const OPCODE_REPORT_SEND: u16 = 106;
pub const OPCODE_ADMIN_COMMAND_SEND: u16 = 128;
pub const OPCODE_PET_CHAT_SEND: u16 = 168;
pub const OPCODE_PET_COMMAND_SEND: u16 = 169;
pub const OPCODE_PLAYER_MAP_TRANSFER_SEND: u16 = 207;
pub const OPCODE_PLAYER_UPDATE_SEND: u16 = 223;
pub const OPCODE_WEDDING_ACTION_SEND: u16 = 138;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
}

/// Builds the v83 gender-selection request: enabled byte followed by gender.
pub fn set_gender_packet(gender: Gender) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_SET_GENDER_SEND);
    packet.write_u8(1);
    packet.write_u8(u8::from(gender == Gender::Female));
    packet.into_bytes()
}

/// Builds the pet chat request, including the reserved fields from v83.
pub fn pet_chat_packet(pet_id: i32, action: i8, text: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_PET_CHAT_SEND);
    packet.write_i32(pet_id);
    packet.write_i32(0);
    packet.write_u8(0);
    packet.write_i8(action);
    packet.write_string(text)?;
    Ok(packet.into_bytes())
}

/// Builds the pet interaction request using the command index from `Pet.nx`.
pub fn pet_command_packet(pet_id: i32, command: i8) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PET_COMMAND_SEND);
    packet.write_i32(pet_id);
    packet.write_i32(0);
    packet.write_u8(0);
    packet.write_i8(command);
    packet.into_bytes()
}

/// Builds the Monster Book cover request; item ID zero clears the cover.
pub fn monster_book_cover_packet(card_item_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_MONSTER_BOOK_COVER_SEND);
    packet.write_i32(card_item_id);
    packet.into_bytes()
}

/// Requests completion of a client-side map transition.
pub fn player_map_transfer_packet() -> Vec<u8> {
    PacketWriter::new(OPCODE_PLAYER_MAP_TRANSFER_SEND).into_bytes()
}

/// Builds the legacy player-stat update request.
///
/// The supplied Cosmic server has no handler for this opcode, so native client
/// flows must not dispatch it; 223 is used by that server's party-search update.
pub fn player_update_packet() -> Vec<u8> {
    PacketWriter::new(OPCODE_PLAYER_UPDATE_SEND).into_bytes()
}

/// Adds a gift to a Cosmic wedding registry (operation 6).
pub fn wedding_add_item_packet(slot: i16, item_id: i32, quantity: i16) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_WEDDING_ACTION_SEND);
    packet.write_u8(6);
    packet.write_i16(slot);
    packet.write_i32(item_id);
    packet.write_i16(quantity);
    packet.into_bytes()
}

/// Collects a gift from a wedding registry (operation 7).
pub fn wedding_take_item_packet(inventory_type: u8, item_position: i8) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_WEDDING_ACTION_SEND);
    packet.write_u8(7);
    packet.write_u8(inventory_type);
    packet.write_i8(item_position);
    packet.into_bytes()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportRequest<'a> {
    IllegalProgram {
        victim: &'a str,
        reason: i8,
        description: &'a str,
    },
    Conversation {
        victim: &'a str,
        reason: i8,
        description: &'a str,
        chat_log: &'a str,
    },
}

/// Builds one of the two report packet variants defined by the legacy client.
pub fn report_packet(report: ReportRequest<'_>) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_REPORT_SEND);
    match report {
        ReportRequest::IllegalProgram {
            victim,
            reason,
            description,
        } => {
            packet.write_i8(0);
            packet.write_string(victim)?;
            packet.write_i8(reason);
            packet.write_string(description)?;
        }
        ReportRequest::Conversation {
            victim,
            reason,
            description,
            chat_log,
        } => {
            packet.write_i8(1);
            packet.write_string(victim)?;
            packet.write_i8(reason);
            packet.write_string(description)?;
            packet.write_string(chat_log)?;
        }
    }
    Ok(packet.into_bytes())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdminMapOperation {
    ShowUsers,
    AlertAdmins,
}

/// Builds the explicit legacy admin-enter-map command variants.
pub fn admin_enter_map_packet(operation: AdminMapOperation) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_ADMIN_COMMAND_SEND);
    packet.write_u8(0x11); // Enter-map mode.
    packet.write_u8(match operation {
        AdminMapOperation::ShowUsers => 0,
        AdminMapOperation::AlertAdmins => 12,
    });
    packet.into_bytes()
}

// The legacy client declares WEDDING_ACTION without a local packet class or
// call site. The supplied Cosmic handler defines gift-add and gift-claim
// payloads, so only those server-reviewed operations are exposed here.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pet_requests_match_legacy_bytes() {
        assert_eq!(
            pet_chat_packet(0x0102_0304, -1, "hi").unwrap(),
            vec![168, 0, 4, 3, 2, 1, 0, 0, 0, 0, 0, 255, 2, 0, b'h', b'i']
        );
        assert_eq!(
            pet_command_packet(7, 12),
            vec![169, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 12]
        );
        assert_eq!(
            pet_chat_packet(1, 0, &"x".repeat(usize::from(u16::MAX) + 1)),
            Err(PacketError::StringTooLong(usize::from(u16::MAX) + 1))
        );
    }

    #[test]
    fn gender_cover_and_map_requests_match_legacy_bytes() {
        assert_eq!(set_gender_packet(Gender::Male), vec![8, 0, 1, 0]);
        assert_eq!(set_gender_packet(Gender::Female), vec![8, 0, 1, 1]);
        assert_eq!(
            monster_book_cover_packet(2_380_123),
            vec![57, 0, 0x5b, 0x51, 0x24, 0]
        );
        assert_eq!(monster_book_cover_packet(0), vec![57, 0, 0, 0, 0, 0]);
        assert_eq!(player_map_transfer_packet(), vec![207, 0]);
        assert_eq!(player_update_packet(), vec![223, 0]);
    }

    #[test]
    fn report_variants_match_legacy_bytes() {
        assert_eq!(
            report_packet(ReportRequest::IllegalProgram {
                victim: "Bob",
                reason: 2,
                description: "bot",
            })
            .unwrap(),
            vec![106, 0, 0, 3, 0, b'B', b'o', b'b', 2, 3, 0, b'b', b'o', b't']
        );
        assert_eq!(
            report_packet(ReportRequest::Conversation {
                victim: "Eve",
                reason: 1,
                description: "spam",
                chat_log: "hello",
            })
            .unwrap(),
            vec![
                106, 0, 1, 3, 0, b'E', b'v', b'e', 1, 4, 0, b's', b'p', b'a', b'm', 5, 0, b'h',
                b'e', b'l', b'l', b'o'
            ]
        );
    }

    #[test]
    fn admin_map_commands_match_legacy_bytes() {
        assert_eq!(
            admin_enter_map_packet(AdminMapOperation::ShowUsers),
            vec![128, 0, 0x11, 0]
        );
        assert_eq!(
            admin_enter_map_packet(AdminMapOperation::AlertAdmins),
            vec![128, 0, 0x11, 12]
        );
    }

    #[test]
    fn cosmic_wedding_registry_requests_match_server_handler_bytes() {
        assert_eq!(
            wedding_add_item_packet(0x1234, 0x0102_0304, 5),
            vec![138, 0, 6, 0x34, 0x12, 4, 3, 2, 1, 5, 0]
        );
        assert_eq!(wedding_take_item_packet(4, -1), vec![138, 0, 7, 4, 255]);
    }
}
