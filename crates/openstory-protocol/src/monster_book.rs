//! Bounded parsers for the v83 Monster Book card and cover updates.

use crate::login::expect_opcode;
use crate::{PacketError, PacketReader};

pub const OPCODE_MONSTER_BOOK_SET_CARD: u16 = 83;
pub const OPCODE_MONSTER_BOOK_SET_COVER: u16 = 84;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonsterBookCardUpdate {
    /// Legacy flag: 0 means the book is full, 1 means it is not full.
    pub full: i8,
    /// Full card item ID, usually in the 238xxxx range.
    pub card_item_id: i32,
    /// Card level as transmitted by the server (the C++ handler narrows this to i8).
    pub card_level: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonsterBookCoverUpdate {
    pub card_item_id: i32,
}

/// Parses the fixed-size v83 `MONSTER_BOOK_SET_CARD` packet.
///
/// Wire shape after the two-byte opcode: `full: i8`, `card_item_id: i32`,
/// `card_level: i32`, all little-endian. Reads are bounded by `PacketReader`.
pub fn parse_monster_book_set_card(bytes: &[u8]) -> Result<MonsterBookCardUpdate, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_MONSTER_BOOK_SET_CARD)?;
    Ok(MonsterBookCardUpdate {
        full: packet.read_i8()?,
        card_item_id: packet.read_i32()?,
        card_level: packet.read_i32()?,
    })
}

/// Parses the fixed-size v83 `MONSTER_BOOK_SET_COVER` packet.
///
/// Wire shape after the two-byte opcode: `card_item_id: i32`, little-endian.
pub fn parse_monster_book_set_cover(bytes: &[u8]) -> Result<MonsterBookCoverUpdate, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_MONSTER_BOOK_SET_COVER)?;
    Ok(MonsterBookCoverUpdate {
        card_item_id: packet.read_i32()?,
    })
}
