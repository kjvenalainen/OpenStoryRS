//! Bounded v83 guild-alliance packets.

use crate::guild::parse_guild_info;
use crate::{PacketError, PacketReader, PacketWriter};

pub const OPCODE_ALLIANCE_OPERATION: u16 = 66;
pub const OPCODE_ALLIANCE_OPERATION_SEND: u16 = 143;
pub const OPCODE_DENY_ALLIANCE_REQUEST: u16 = 144;
const MAX_ALLIANCE_GUILDS: usize = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllianceGuild {
    pub id: i32,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllianceInfo {
    pub id: i32,
    pub name: String,
    pub rank_titles: [String; 5],
    pub capacity: i32,
    pub notice: String,
    pub guilds: Vec<AllianceGuild>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AllianceEvent {
    Invite {
        alliance_id: i32,
        inviter: String,
    },
    ShowInfo {
        alliance_id: i32,
        character_id: i32,
    },
    Snapshot(AllianceInfo),
    GuildList(Vec<AllianceGuild>),
    MemberOnline {
        alliance_id: i32,
        guild_id: i32,
        character_id: i32,
        online: bool,
    },
    MemberStatsChanged {
        alliance_id: i32,
        guild_id: i32,
        character_id: i32,
        level: i32,
        job: i32,
    },
    RankTitlesChanged {
        alliance_id: i32,
        titles: [String; 5],
    },
    NoticeChanged {
        alliance_id: i32,
        notice: String,
    },
    Disbanded {
        alliance_id: i32,
    },
}

/// Parses alliance invitations and v83 alliance state broadcasts.
pub fn parse_alliance_operation(bytes: &[u8]) -> Result<Option<AllianceEvent>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    let actual = packet.read_u16()?;
    if actual != OPCODE_ALLIANCE_OPERATION {
        return Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_ALLIANCE_OPERATION,
            actual,
        });
    }
    let operation = packet.read_u8()?;
    let event = match operation {
        0x02 => Some(AllianceEvent::ShowInfo {
            alliance_id: packet.read_i32()?,
            character_id: packet.read_i32()?,
        }),
        0x03 => {
            let alliance_id = packet.read_i32()?;
            let inviter = packet.read_string()?;
            packet.skip(2)?;
            Some(AllianceEvent::Invite {
                alliance_id,
                inviter,
            })
        }
        0x0c => {
            // The legacy client reads this as an int8 and only accepts the
            // exact value 1 as "has alliance".
            if packet.read_i8()? != 1 {
                Some(AllianceEvent::Snapshot(AllianceInfo {
                    id: 0,
                    name: String::new(),
                    rank_titles: std::array::from_fn(|_| String::new()),
                    capacity: 0,
                    notice: String::new(),
                    guilds: Vec::new(),
                }))
            } else {
                let id = packet.read_i32()?;
                let name = packet.read_string()?;
                let rank_titles = read_rank_titles(&mut packet)?;
                let count = read_guild_count_i8(&mut packet)?;
                let capacity = packet.read_i32()?;
                let mut guilds = Vec::with_capacity(count);
                for _ in 0..count {
                    guilds.push(AllianceGuild {
                        id: packet.read_i32()?,
                        name: String::new(),
                    });
                }
                let notice = packet.read_string()?;
                Some(AllianceEvent::Snapshot(AllianceInfo {
                    id,
                    name,
                    rank_titles,
                    capacity,
                    notice,
                    guilds,
                }))
            }
        }
        0x0d => {
            let count = read_guild_count_i32(&mut packet)?;
            let mut guilds = Vec::with_capacity(count);
            for _ in 0..count {
                let guild = parse_guild_info(&mut packet)?;
                guilds.push(AllianceGuild {
                    id: guild.id,
                    name: guild.name,
                });
            }
            Some(AllianceEvent::GuildList(guilds))
        }
        0x0e => Some(AllianceEvent::MemberOnline {
            alliance_id: packet.read_i32()?,
            guild_id: packet.read_i32()?,
            character_id: packet.read_i32()?,
            // C++ InPacket::read_bool() tests byte == 1, not nonzero.
            online: packet.read_u8()? == 1,
        }),
        0x0f => {
            let id = packet.read_i32()?;
            let name = packet.read_string()?;
            let rank_titles = read_rank_titles(&mut packet)?;
            let count = read_guild_count_i8(&mut packet)?;
            let mut guild_ids = Vec::with_capacity(count);
            for _ in 0..count {
                guild_ids.push(packet.read_i32()?);
            }
            let capacity = packet.read_i32()?;
            packet.skip(2)?;
            let mut guilds = read_full_guilds(&mut packet, count)?;
            merge_guild_ids(&mut guilds, guild_ids);
            Some(AllianceEvent::Snapshot(AllianceInfo {
                id,
                name,
                rank_titles,
                capacity,
                notice: String::new(),
                guilds,
            }))
        }
        0x10 => {
            let id = packet.read_i32()?;
            let name = packet.read_string()?;
            let rank_titles = read_rank_titles(&mut packet)?;
            let count = read_guild_count_i8(&mut packet)?;
            let mut guilds = Vec::with_capacity(count);
            for _ in 0..count {
                guilds.push(AllianceGuild {
                    id: packet.read_i32()?,
                    name: String::new(),
                });
            }
            let capacity = packet.read_i32()?;
            let notice = packet.read_string()?;
            let removed_id = packet.read_i32()?;
            let _removed_guild = parse_guild_info(&mut packet)?;
            packet.skip(1)?;
            guilds.retain(|guild| guild.id != removed_id);
            Some(AllianceEvent::Snapshot(AllianceInfo {
                id,
                name,
                rank_titles,
                capacity,
                notice,
                guilds,
            }))
        }
        0x12 => {
            let id = packet.read_i32()?;
            let name = packet.read_string()?;
            let rank_titles = read_rank_titles(&mut packet)?;
            let count = read_guild_count_i8(&mut packet)?;
            let mut guilds = Vec::with_capacity(count);
            for _ in 0..count {
                guilds.push(AllianceGuild {
                    id: packet.read_i32()?,
                    name: String::new(),
                });
            }
            let capacity = packet.read_i32()?;
            let notice = packet.read_string()?;
            let new_guild_id = packet.read_i32()?;
            let new_guild = parse_guild_info(&mut packet)?;
            if let Some(guild) = guilds.iter_mut().find(|guild| guild.id == new_guild_id) {
                guild.name = new_guild.name;
            }
            Some(AllianceEvent::Snapshot(AllianceInfo {
                id,
                name,
                rank_titles,
                capacity,
                notice,
                guilds,
            }))
        }
        0x1c => Some(AllianceEvent::NoticeChanged {
            alliance_id: packet.read_i32()?,
            notice: packet.read_string()?,
        }),
        0x18 => Some(AllianceEvent::MemberStatsChanged {
            alliance_id: packet.read_i32()?,
            guild_id: packet.read_i32()?,
            character_id: packet.read_i32()?,
            level: packet.read_i32()?,
            job: packet.read_i32()?,
        }),
        0x1a => Some(AllianceEvent::RankTitlesChanged {
            alliance_id: packet.read_i32()?,
            titles: read_rank_titles(&mut packet)?,
        }),
        0x1d => Some(AllianceEvent::Disbanded {
            alliance_id: packet.read_i32()?,
        }),
        _ => None,
    };
    Ok(event)
}

pub fn alliance_info_request_packet() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_ALLIANCE_OPERATION_SEND);
    packet.write_u8(0x01);
    packet.into_bytes()
}

pub fn alliance_leave_packet() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_ALLIANCE_OPERATION_SEND);
    packet.write_u8(0x02);
    packet.into_bytes()
}

pub fn alliance_invite_packet(guild_name: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_ALLIANCE_OPERATION_SEND);
    packet.write_u8(0x03);
    packet.write_string(guild_name)?;
    Ok(packet.into_bytes())
}

pub fn alliance_expel_guild_packet(guild_id: i32, alliance_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_ALLIANCE_OPERATION_SEND);
    packet.write_u8(0x06);
    packet.write_i32(guild_id);
    packet.write_i32(alliance_id);
    packet.into_bytes()
}

pub fn alliance_rank_titles_packet(titles: &[String; 5]) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_ALLIANCE_OPERATION_SEND);
    packet.write_u8(0x08);
    for title in titles {
        packet.write_string(title)?;
    }
    Ok(packet.into_bytes())
}

pub fn alliance_notice_packet(notice: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_ALLIANCE_OPERATION_SEND);
    packet.write_u8(0x0a);
    packet.write_string(notice)?;
    Ok(packet.into_bytes())
}

pub fn alliance_accept_invite_packet(alliance_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_ALLIANCE_OPERATION_SEND);
    packet.write_u8(0x04);
    packet.write_i32(alliance_id);
    packet.into_bytes()
}

pub fn deny_alliance_invite_packet(
    inviter: &str,
    guild_name: &str,
) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_DENY_ALLIANCE_REQUEST);
    packet.write_u8(0);
    packet.write_string(inviter)?;
    packet.write_string(guild_name)?;
    Ok(packet.into_bytes())
}

fn read_full_guilds(
    packet: &mut PacketReader<'_>,
    count: usize,
) -> Result<Vec<AllianceGuild>, PacketError> {
    let mut guilds = Vec::with_capacity(count);
    for _ in 0..count {
        let guild = parse_guild_info(packet)?;
        guilds.push(AllianceGuild {
            id: guild.id,
            name: guild.name,
        });
    }
    Ok(guilds)
}

fn merge_guild_ids(guilds: &mut Vec<AllianceGuild>, ids: Vec<i32>) {
    for id in ids {
        if !guilds.iter().any(|guild| guild.id == id) {
            guilds.push(AllianceGuild {
                id,
                name: String::new(),
            });
        }
    }
}

fn read_guild_count_i8(packet: &mut PacketReader<'_>) -> Result<usize, PacketError> {
    let raw_count = packet.read_i8()?;
    if raw_count < 0 {
        return Err(PacketError::InvalidAllianceGuildCount(i32::from(raw_count)));
    }
    let count = raw_count as usize;
    validate_guild_count(count)?;
    Ok(count)
}

fn read_guild_count_i32(packet: &mut PacketReader<'_>) -> Result<usize, PacketError> {
    let count = packet.read_i32()?;
    if count < 0 {
        return Err(PacketError::InvalidAllianceGuildCount(count));
    }
    let count = count as usize;
    validate_guild_count(count)?;
    Ok(count)
}

fn validate_guild_count(count: usize) -> Result<(), PacketError> {
    if count > MAX_ALLIANCE_GUILDS {
        return Err(PacketError::TooManyAllianceGuilds(MAX_ALLIANCE_GUILDS));
    }
    Ok(())
}

fn read_rank_titles(packet: &mut PacketReader<'_>) -> Result<[String; 5], PacketError> {
    Ok([
        packet.read_string()?,
        packet.read_string()?,
        packet.read_string()?,
        packet.read_string()?,
        packet.read_string()?,
    ])
}
