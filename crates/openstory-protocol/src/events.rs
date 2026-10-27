use crate::{PacketError, PacketReader, PacketWriter};

pub const OPCODE_EVENT_INFO: u16 = 195;
pub const OPCODE_REQUEST_EVENT_INFO: u16 = 241;
const MAX_EVENT_ENTRIES: usize = 64;
const MAX_EVENT_REWARDS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventReward {
    pub item_id: i32,
    pub quantity: i16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerEventInfo {
    pub type_id: i8,
    pub name: String,
    pub description: String,
    /// Positive while running, zero when ended, negative before it starts.
    pub seconds_remaining: i32,
    pub multiplier: i16,
    pub has_item_rewards: bool,
    pub rewards: Vec<EventReward>,
}

pub fn request_event_info_packet() -> Vec<u8> {
    PacketWriter::new(OPCODE_REQUEST_EVENT_INFO).into_bytes()
}

pub fn parse_event_info(bytes: &[u8]) -> Result<Vec<ServerEventInfo>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    let opcode = packet.read_u16()?;
    if opcode != OPCODE_EVENT_INFO {
        return Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_EVENT_INFO,
            actual: opcode,
        });
    }
    let count = packet.read_i16()?;
    if count < 0 {
        return Err(PacketError::InvalidEventCount(count));
    }
    let count = count as usize;
    if count > MAX_EVENT_ENTRIES {
        return Err(PacketError::TooManyEventEntries(count));
    }

    let mut events = Vec::with_capacity(count);
    for _ in 0..count {
        let type_id = packet.read_i8()?;
        let name = packet.read_string()?;
        let description = packet.read_string()?;
        let seconds_remaining = packet.read_i32()?;
        let multiplier = packet.read_i16()?;
        let has_item_rewards = packet.read_bool()?;
        let reward_count = packet.read_i16()?;
        if reward_count < 0 {
            return Err(PacketError::InvalidEventRewardCount(reward_count));
        }
        let reward_count = reward_count as usize;
        if reward_count > MAX_EVENT_REWARDS {
            return Err(PacketError::TooManyEventRewards(reward_count));
        }
        let mut rewards = Vec::with_capacity(reward_count);
        for _ in 0..reward_count {
            rewards.push(EventReward {
                item_id: packet.read_i32()?,
                quantity: packet.read_i16()?,
            });
        }
        events.push(ServerEventInfo {
            type_id,
            name,
            description,
            seconds_remaining,
            multiplier,
            has_item_rewards,
            rewards,
        });
    }
    Ok(events)
}
