//! Bounded parsers for the v83 Monster Book card and cover updates.

use crate::login::expect_opcode;
use crate::{PacketError, PacketReader};

const MAX_INITIAL_SET_FIELD_ENTRIES: usize = 4096;

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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MonsterBookSnapshot {
    pub cover_item_id: i32,
    pub cards: Vec<(i32, i8)>,
}

/// Skips the minigame and fixed-width ring sections preceding the saved teleport
/// rock locations and Monster Book in an initial v83 `SET_FIELD` packet.
pub(crate) fn skip_initial_minigame_and_rings(
    packet: &mut PacketReader<'_>,
) -> Result<(), PacketError> {
    packet.skip(2)?; // Minigame state.
    skip_fixed_records(packet, "ring 1", 33)?;
    skip_fixed_records(packet, "ring 2", 37)?;
    skip_fixed_records(packet, "ring 3", 48)
}

/// Reads the Monster Book portion of the v83 initial `SET_FIELD` character state.
pub(crate) fn parse_initial_monster_book(
    packet: &mut PacketReader<'_>,
) -> Result<MonsterBookSnapshot, PacketError> {
    let cover_item_id = packet.read_i32()?.max(0);
    packet.skip(1)?; // Reserved byte.
    let card_count = read_initial_count(packet, "Monster Book cards")?;
    let mut cards = Vec::with_capacity(card_count);
    for _ in 0..card_count {
        let card_offset = i32::from(packet.read_i16()?);
        let level = packet.read_i8()?;
        cards.push((2_380_000 + card_offset, level));
    }
    Ok(MonsterBookSnapshot {
        cover_item_id,
        cards,
    })
}

fn skip_fixed_records(
    packet: &mut PacketReader<'_>,
    section: &'static str,
    record_size: usize,
) -> Result<(), PacketError> {
    let count = read_initial_count(packet, section)?;
    packet.skip(count * record_size)
}

fn read_initial_count(
    packet: &mut PacketReader<'_>,
    section: &'static str,
) -> Result<usize, PacketError> {
    let count = packet.read_i16()?;
    if count < 0 {
        return Err(PacketError::InvalidInitialSetFieldCount { section, count });
    }
    let count = count as usize;
    if count > MAX_INITIAL_SET_FIELD_ENTRIES {
        return Err(PacketError::TooManyInitialSetFieldEntries {
            section,
            maximum: MAX_INITIAL_SET_FIELD_ENTRIES,
        });
    }
    Ok(count)
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
