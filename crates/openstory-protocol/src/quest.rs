//! Bounded v83 quest-log snapshots and quest action packets.

use crate::{PacketError, PacketReader, PacketWriter};

pub const OPCODE_QUEST_ACTION: u16 = 107;
pub const OPCODE_QUEST_CLEAR: u16 = 49;
pub const OPCODE_SHOW_STATUS_INFO: u16 = 39;
pub const OPCODE_UPDATE_QUEST_INFO: u16 = 211;
const MAX_QUEST_ENTRIES: usize = 2048;
const MAX_COOLDOWNS: usize = 512;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QuestLog {
    pub started: Vec<QuestEntry>,
    pub completed: Vec<CompletedQuest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestEntry {
    pub id: i16,
    /// Server quest progress string, whose fixed-width counters are retained as sent.
    pub data: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletedQuest {
    pub id: i16,
    pub timestamp: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuestUpdate {
    Forfeited { id: i16 },
    Progress { id: i16, data: String },
    Completed { id: i16, timestamp: i64 },
}

/// A server-reported reason a quest action could not be completed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestActionFailure {
    RequirementsNotMet { quest_id: i16 },
    NotEnoughMesos,
    ItemEquipped,
    MissingRequiredItem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestTimer {
    pub quest_id: i16,
    pub time_ms: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuestTimerUpdate {
    Set { timers: Vec<QuestTimer> },
    Lifted { position: i16, quest_id: i16 },
}

impl QuestLog {
    pub fn apply(&mut self, update: QuestUpdate) {
        match update {
            QuestUpdate::Forfeited { id } => {
                self.started.retain(|quest| quest.id != id);
            }
            QuestUpdate::Progress { id, data } => {
                self.completed.retain(|quest| quest.id != id);
                if let Some(quest) = self.started.iter_mut().find(|quest| quest.id == id) {
                    quest.data = data;
                } else {
                    self.started.push(QuestEntry { id, data });
                }
                self.started.sort_by_key(|quest| quest.id);
            }
            QuestUpdate::Completed { id, timestamp } => {
                self.started.retain(|quest| quest.id != id);
                if let Some(quest) = self.completed.iter_mut().find(|quest| quest.id == id) {
                    quest.timestamp = timestamp;
                } else {
                    self.completed.push(CompletedQuest { id, timestamp });
                }
                self.completed.sort_by_key(|quest| quest.id);
            }
        }
    }
}

/// Reads the quest-log section after the initial character skill and cooldown lists.
pub(crate) fn parse_initial_quest_log(
    packet: &mut PacketReader<'_>,
) -> Result<QuestLog, PacketError> {
    let started_count = read_count(packet)?;
    let mut log = QuestLog::default();
    for _ in 0..started_count {
        let id = packet.read_i16()?;
        let data = packet.read_string()?;
        if let Some(existing) = log.started.iter_mut().find(|quest| quest.id == id) {
            existing.data = data;
        } else {
            log.started.push(QuestEntry { id, data });
        }
    }

    let completed_count = read_count(packet)?;
    for _ in 0..completed_count {
        let id = packet.read_i16()?;
        let timestamp = packet.read_i64()?;
        if let Some(existing) = log.completed.iter_mut().find(|quest| quest.id == id) {
            existing.timestamp = timestamp;
        } else {
            log.completed.push(CompletedQuest { id, timestamp });
        }
    }
    log.started.sort_by_key(|quest| quest.id);
    log.completed.sort_by_key(|quest| quest.id);
    Ok(log)
}

/// Parses only the quest-state variant of `SHOW_STATUS_INFO`; unrelated status modes return `None`.
pub fn parse_quest_status_update(bytes: &[u8]) -> Result<Option<QuestUpdate>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_SHOW_STATUS_INFO)?;
    let mode = packet.read_u8()?;
    if mode != 1 {
        return Ok(None);
    }
    let id = packet.read_i16()?;
    let status = packet.read_i8()?;
    let update = match status {
        0 => QuestUpdate::Forfeited { id },
        1 => {
            let data = packet.read_string()?;
            if packet.remaining() >= 5 {
                packet.skip(5)?;
            }
            QuestUpdate::Progress { id, data }
        }
        2 => QuestUpdate::Completed {
            id,
            timestamp: packet.read_i64()?,
        },
        _ => return Ok(None),
    };
    Ok(Some(update))
}

/// Decodes the visual quest-completion notification packet.
pub fn parse_quest_clear(bytes: &[u8]) -> Result<i16, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_QUEST_CLEAR)?;
    packet.read_i16()
}

/// Returns the quest ID when `UPDATE_QUEST_INFO` reports an expired quest.
pub fn parse_expired_quest(bytes: &[u8]) -> Result<Option<i16>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_UPDATE_QUEST_INFO)?;
    if packet.read_u8()? == 0x0f {
        Ok(Some(packet.read_i16()?))
    } else {
        Ok(None)
    }
}

/// Parses `UPDATE_QUEST_INFO` failure modes that explain why a quest action failed.
/// Mode `0x0A` includes a quest ID; modes `0x0B`, `0x0D`, and `0x0E` have no payload.
pub fn parse_quest_action_failure(bytes: &[u8]) -> Result<Option<QuestActionFailure>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_UPDATE_QUEST_INFO)?;
    let failure = match packet.read_u8()? {
        0x0a => Some(QuestActionFailure::RequirementsNotMet {
            quest_id: packet.read_i16()?,
        }),
        0x0b => Some(QuestActionFailure::NotEnoughMesos),
        0x0d => Some(QuestActionFailure::ItemEquipped),
        0x0e => Some(QuestActionFailure::MissingRequiredItem),
        _ => None,
    };
    Ok(failure)
}

/// Parses timed-quest variants of `UPDATE_QUEST_INFO` (modes 6 and 7).
pub fn parse_quest_timer_update(bytes: &[u8]) -> Result<Option<QuestTimerUpdate>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_UPDATE_QUEST_INFO)?;
    let update = match packet.read_u8()? {
        6 => {
            let count = read_count(&mut packet)?;
            let mut timers = Vec::with_capacity(count);
            for _ in 0..count {
                timers.push(QuestTimer {
                    quest_id: packet.read_i16()?,
                    time_ms: packet.read_i32()?,
                });
            }
            Some(QuestTimerUpdate::Set { timers })
        }
        7 => Some(QuestTimerUpdate::Lifted {
            position: packet.read_i16()?,
            quest_id: packet.read_i16()?,
        }),
        _ => None,
    };
    Ok(update)
}

pub fn start_quest_packet(id: i16, npc_id: i32, scripted: bool) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_QUEST_ACTION);
    packet.write_u8(if scripted { 4 } else { 1 });
    packet.write_i16(id);
    packet.write_i32(npc_id);
    packet.into_bytes()
}

pub fn complete_quest_packet(
    id: i16,
    npc_id: i32,
    selection: Option<i16>,
    scripted: bool,
) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_QUEST_ACTION);
    packet.write_u8(if scripted { 5 } else { 2 });
    packet.write_i16(id);
    packet.write_i32(npc_id);
    // Scripted end-quest actions (action 5) do not carry the optional reward
    // selection field. The server's scripted quest conversation owns those
    // choices through NPC_TALK_MORE responses.
    if !scripted {
        if let Some(selection) = selection.filter(|selection| *selection >= 0) {
            packet.write_i16(selection);
        }
    }
    packet.into_bytes()
}

pub fn forfeit_quest_packet(id: i16) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_QUEST_ACTION);
    packet.write_u8(3);
    packet.write_i16(id);
    packet.into_bytes()
}

pub fn restore_lost_quest_item_packet(id: i16, item_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_QUEST_ACTION);
    packet.write_u8(0);
    packet.write_i16(id);
    packet.write_i32(0);
    packet.write_i32(item_id);
    packet.into_bytes()
}

pub(crate) fn skip_initial_cooldowns(packet: &mut PacketReader<'_>) -> Result<(), PacketError> {
    let count = packet.read_i16()?;
    if count < 0 {
        return Err(PacketError::InvalidQuestCount(count));
    }
    let count = count as usize;
    if count > MAX_COOLDOWNS {
        return Err(PacketError::TooManyCooldowns(MAX_COOLDOWNS));
    }
    for _ in 0..count {
        packet.skip(6)?; // skill ID and remaining cooldown in milliseconds
    }
    Ok(())
}

fn read_count(packet: &mut PacketReader<'_>) -> Result<usize, PacketError> {
    let count = packet.read_i16()?;
    if count < 0 {
        return Err(PacketError::InvalidQuestCount(count));
    }
    let count = count as usize;
    if count > MAX_QUEST_ENTRIES {
        return Err(PacketError::TooManyQuestEntries(MAX_QUEST_ENTRIES));
    }
    Ok(count)
}

fn expect_opcode(packet: &mut PacketReader<'_>, expected: u16) -> Result<(), PacketError> {
    let actual = packet.read_u16()?;
    if actual == expected {
        Ok(())
    } else {
        Err(PacketError::UnexpectedOpcode { expected, actual })
    }
}
