//! Bounded parsers for server-directed client UI requests.

use crate::login::expect_opcode;
use crate::{PacketError, PacketReader};

pub const OPCODE_OPEN_UI: u16 = 220;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenUiRequest {
    Inventory,
    Stats,
    SkillBook,
    BuddyList,
    WorldMap,
    Messenger,
    QuestLog,
    KeyConfig,
    MonsterBook,
    PartySearch,
    Unsupported(u8),
}

/// Parses the v83 `OPEN_UI` request. Unknown window IDs remain explicit so the client
/// can report unsupported screens without losing packet alignment.
pub fn parse_open_ui(bytes: &[u8]) -> Result<OpenUiRequest, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_OPEN_UI)?;
    Ok(match packet.read_u8()? {
        0x00 => OpenUiRequest::Inventory,
        0x01 => OpenUiRequest::Stats,
        0x02 => OpenUiRequest::SkillBook,
        0x03 => OpenUiRequest::BuddyList,
        0x04 => OpenUiRequest::WorldMap,
        0x05 => OpenUiRequest::Messenger,
        0x09 => OpenUiRequest::QuestLog,
        0x0A => OpenUiRequest::KeyConfig,
        0x12 => OpenUiRequest::MonsterBook,
        0x16 => OpenUiRequest::PartySearch,
        kind => OpenUiRequest::Unsupported(kind),
    })
}
