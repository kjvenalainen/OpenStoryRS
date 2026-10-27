//! Bounded v83 guild roster and operation packets.

use crate::{PacketError, PacketReader, PacketWriter};

pub const OPCODE_GUILD_OPERATION: u16 = 65;
pub const OPCODE_GUILD_OPERATION_SEND: u16 = 126;
pub const OPCODE_DENY_GUILD_REQUEST: u16 = 127;
const MAX_GUILD_MEMBERS: usize = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuildInfo {
    pub id: i32,
    pub name: String,
    pub rank_titles: [String; 5],
    pub members: Vec<GuildMember>,
    pub capacity: i32,
    pub emblem: GuildEmblem,
    pub notice: String,
    pub gp: i32,
    pub alliance_id: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuildMember {
    pub character_id: i32,
    pub name: String,
    pub job: i32,
    pub level: i32,
    pub rank: i32,
    pub online: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GuildEmblem {
    pub background: i16,
    pub background_color: i8,
    pub logo: i16,
    pub logo_color: i8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuildEvent {
    Snapshot(Option<GuildInfo>),
    Invite {
        guild_id: i32,
        inviter: String,
    },
    CreationInvite {
        leader: String,
        guild_name: String,
    },
    CreationDismissed,
    CreationFailed,
    MemberJoined {
        guild_id: i32,
        member: GuildMember,
    },
    MemberLeft {
        guild_id: i32,
        character_id: i32,
        name: String,
        expelled: bool,
    },
    GuildDisbanded {
        guild_id: i32,
        marker: i8,
    },
    CapacityChanged {
        guild_id: i32,
        capacity: i32,
    },
    MemberStatsChanged {
        guild_id: i32,
        character_id: i32,
        level: i32,
        job: i32,
    },
    MemberOnlineChanged {
        guild_id: i32,
        character_id: i32,
        online: bool,
    },
    RankTitlesChanged {
        guild_id: i32,
        titles: [String; 5],
    },
    MemberRankChanged {
        guild_id: i32,
        character_id: i32,
        rank: i32,
    },
    EmblemChanged {
        guild_id: i32,
        emblem: GuildEmblem,
    },
    NoticeChanged {
        guild_id: i32,
        notice: String,
    },
    GuildPointsChanged {
        guild_id: i32,
        points: i32,
    },
    InvitationDenied {
        name: String,
    },
    InvitationFailed {
        name: String,
    },
}

/// Parses guild operation packets sent by the v83 channel server.
/// Unknown operation codes return `Ok(None)` so unrelated guild features can
/// be added without making the rest of the channel session brittle.
pub fn parse_guild_operation(bytes: &[u8]) -> Result<Option<GuildEvent>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet)?;
    let operation = packet.read_u8()?;
    let event = match operation {
        0x03 => {
            packet.read_i32()?; // match-checker context id (zero for guild creation)
            Some(GuildEvent::CreationInvite {
                leader: packet.read_string()?,
                guild_name: packet.read_string()?,
            })
        }
        0x23 => Some(GuildEvent::CreationFailed),
        0x26 => Some(GuildEvent::CreationDismissed),
        0x05 => Some(GuildEvent::Invite {
            guild_id: packet.read_i32()?,
            inviter: packet.read_string()?,
        }),
        0x1a => {
            let in_guild = packet.read_u8()? != 0;
            if in_guild {
                Some(GuildEvent::Snapshot(Some(parse_guild_info(&mut packet)?)))
            } else {
                Some(GuildEvent::Snapshot(None))
            }
        }
        0x27 => {
            let guild_id = packet.read_i32()?;
            let character_id = packet.read_i32()?;
            let name = packet.read_string_bytes(13)?;
            let job = packet.read_i32()?;
            let level = packet.read_i32()?;
            let rank = packet.read_i32()?;
            let online = packet.read_i32()? != 0;
            packet.skip(8)?; // guild signature and alliance rank
            Some(GuildEvent::MemberJoined {
                guild_id,
                member: GuildMember {
                    character_id,
                    name,
                    job,
                    level,
                    rank,
                    online,
                },
            })
        }
        0x2c | 0x2f => Some(GuildEvent::MemberLeft {
            guild_id: packet.read_i32()?,
            character_id: packet.read_i32()?,
            name: packet.read_string()?,
            expelled: operation == 0x2f,
        }),
        0x32 => Some(GuildEvent::GuildDisbanded {
            guild_id: packet.read_i32()?,
            marker: packet.read_i8()?,
        }),
        0x3a => Some(GuildEvent::CapacityChanged {
            guild_id: packet.read_i32()?,
            // The legacy handler reads this wire byte as int8_t.
            capacity: i32::from(packet.read_i8()?),
        }),
        0x3c => Some(GuildEvent::MemberStatsChanged {
            guild_id: packet.read_i32()?,
            character_id: packet.read_i32()?,
            level: packet.read_i32()?,
            job: packet.read_i32()?,
        }),
        0x3d => Some(GuildEvent::MemberOnlineChanged {
            guild_id: packet.read_i32()?,
            character_id: packet.read_i32()?,
            online: packet.read_u8()? != 0,
        }),
        0x3e => Some(GuildEvent::RankTitlesChanged {
            guild_id: packet.read_i32()?,
            titles: read_rank_titles(&mut packet)?,
        }),
        0x40 => Some(GuildEvent::MemberRankChanged {
            guild_id: packet.read_i32()?,
            character_id: packet.read_i32()?,
            // The legacy handler reads this wire byte as int8_t.
            rank: i32::from(packet.read_i8()?),
        }),
        0x42 => Some(GuildEvent::EmblemChanged {
            guild_id: packet.read_i32()?,
            emblem: read_emblem(&mut packet)?,
        }),
        0x44 => Some(GuildEvent::NoticeChanged {
            guild_id: packet.read_i32()?,
            notice: packet.read_string()?,
        }),
        0x48 => Some(GuildEvent::GuildPointsChanged {
            guild_id: packet.read_i32()?,
            points: packet.read_i32()?,
        }),
        0x37 => Some(GuildEvent::InvitationDenied {
            name: packet.read_string()?,
        }),
        0x36 => Some(GuildEvent::InvitationFailed {
            name: packet.read_string()?,
        }),
        _ => None,
    };
    Ok(event)
}

pub fn accept_guild_invite_packet(guild_id: i32, character_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_GUILD_OPERATION_SEND);
    packet.write_u8(0x06);
    packet.write_i32(guild_id);
    packet.write_i32(character_id);
    packet.into_bytes()
}

pub fn create_guild_packet(name: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_GUILD_OPERATION_SEND);
    packet.write_u8(0x02);
    packet.write_string(name)?;
    Ok(packet.into_bytes())
}

pub fn guild_creation_confirmation_packet(accepted: bool) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_GUILD_OPERATION_SEND);
    packet.write_u8(0x1e);
    packet.write_i32(0);
    packet.write_u8(u8::from(accepted));
    packet.into_bytes()
}

pub fn invite_to_guild_packet(name: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_GUILD_OPERATION_SEND);
    packet.write_u8(0x05);
    packet.write_string(name)?;
    Ok(packet.into_bytes())
}

pub fn leave_guild_packet(character_id: i32, name: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_GUILD_OPERATION_SEND);
    packet.write_u8(0x07);
    packet.write_i32(character_id);
    packet.write_string(name)?;
    Ok(packet.into_bytes())
}

pub fn expel_guild_member_packet(character_id: i32, name: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_GUILD_OPERATION_SEND);
    packet.write_u8(0x08);
    packet.write_i32(character_id);
    packet.write_string(name)?;
    Ok(packet.into_bytes())
}

pub fn guild_rank_titles_packet(titles: &[String; 5]) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_GUILD_OPERATION_SEND);
    packet.write_u8(0x0d);
    for title in titles {
        packet.write_string(title)?;
    }
    Ok(packet.into_bytes())
}

pub fn guild_member_rank_packet(character_id: i32, rank: u8) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_GUILD_OPERATION_SEND);
    packet.write_u8(0x0e);
    packet.write_i32(character_id);
    packet.write_u8(rank);
    packet.into_bytes()
}

pub fn guild_emblem_packet(
    background: i16,
    background_color: i8,
    logo: i16,
    logo_color: i8,
) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_GUILD_OPERATION_SEND);
    packet.write_u8(0x0f);
    packet.write_i16(background);
    packet.write_i8(background_color);
    packet.write_i16(logo);
    packet.write_i8(logo_color);
    packet.into_bytes()
}

pub fn guild_notice_packet(notice: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_GUILD_OPERATION_SEND);
    packet.write_u8(0x10);
    packet.write_string(notice)?;
    Ok(packet.into_bytes())
}

pub fn deny_guild_invite_packet(inviter: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_DENY_GUILD_REQUEST);
    packet.write_u8(0);
    packet.write_string(inviter)?;
    Ok(packet.into_bytes())
}

pub(crate) fn parse_guild_info(packet: &mut PacketReader<'_>) -> Result<GuildInfo, PacketError> {
    let id = packet.read_i32()?;
    let name = packet.read_string()?;
    let rank_titles = read_rank_titles(packet)?;
    let count = usize::from(packet.read_u8()?);
    if count > MAX_GUILD_MEMBERS {
        return Err(PacketError::TooManyGuildMembers(MAX_GUILD_MEMBERS));
    }
    let mut character_ids = Vec::with_capacity(count);
    for _ in 0..count {
        character_ids.push(packet.read_i32()?);
    }
    let mut members = Vec::with_capacity(count);
    for character_id in character_ids {
        members.push(GuildMember {
            character_id,
            name: packet.read_string_bytes(13)?,
            job: packet.read_i32()?,
            level: packet.read_i32()?,
            rank: packet.read_i32()?,
            online: packet.read_i32()? != 0,
        });
        packet.skip(8)?; // guild signature and alliance rank
    }
    let capacity = packet.read_i32()?;
    let emblem = read_emblem(packet)?;
    let notice = packet.read_string()?;
    let gp = packet.read_i32()?;
    let alliance_id = packet.read_i32()?;
    Ok(GuildInfo {
        id,
        name,
        rank_titles,
        members,
        capacity,
        emblem,
        notice,
        gp,
        alliance_id,
    })
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

fn read_emblem(packet: &mut PacketReader<'_>) -> Result<GuildEmblem, PacketError> {
    Ok(GuildEmblem {
        background: packet.read_i16()?,
        background_color: packet.read_i8()?,
        logo: packet.read_i16()?,
        logo_color: packet.read_i8()?,
    })
}

fn expect_opcode(packet: &mut PacketReader<'_>) -> Result<(), PacketError> {
    let actual = packet.read_u16()?;
    if actual == OPCODE_GUILD_OPERATION {
        Ok(())
    } else {
        Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_GUILD_OPERATION,
            actual,
        })
    }
}
