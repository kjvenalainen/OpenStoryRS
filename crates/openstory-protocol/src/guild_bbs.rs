//! Bounded v83 guild bulletin-board packets.

use crate::{PacketError, PacketReader, PacketWriter};

pub const OPCODE_GUILD_BBS_PACKET: u16 = 59;
pub const OPCODE_BBS_OPERATION: u16 = 155;
pub const MAX_BBS_TITLE_LENGTH: usize = 25;
pub const MAX_BBS_CONTENT_LENGTH: usize = 600;
pub const MAX_BBS_REPLY_LENGTH: usize = 25;
const MAX_THREADS_PER_PAGE: usize = 10;
const MAX_BBS_REPLIES: usize = 500;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BbsThread {
    pub id: i32,
    pub poster_id: i32,
    pub title: String,
    pub timestamp: i64,
    pub icon: i32,
    pub reply_count: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BbsReply {
    pub id: i32,
    pub poster_id: i32,
    pub timestamp: i64,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BbsThreadView {
    pub id: i32,
    pub poster_id: i32,
    pub timestamp: i64,
    pub title: String,
    pub content: String,
    pub icon: i32,
    pub replies: Vec<BbsReply>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuildBbsEvent {
    ThreadList {
        notice: Option<BbsThread>,
        total: i32,
        threads: Vec<BbsThread>,
    },
    ThreadView(BbsThreadView),
}

/// Parses the server's thread-list (0x06) and thread-view (0x07) responses.
pub fn parse_guild_bbs_packet(bytes: &[u8]) -> Result<GuildBbsEvent, PacketError> {
    let mut packet = PacketReader::new(bytes);
    let actual = packet.read_u16()?;
    if actual != OPCODE_GUILD_BBS_PACKET {
        return Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_GUILD_BBS_PACKET,
            actual,
        });
    }
    let kind = packet.read_u8()?;
    let event = match kind {
        0x06 => {
            let notice = if packet.read_u8()? != 0 {
                Some(read_thread(&mut packet)?)
            } else {
                None
            };
            let total = packet.read_i32()?;
            let visible = packet.read_i32()?;
            if total < 0 || !(0..=MAX_THREADS_PER_PAGE as i32).contains(&visible) {
                return Err(PacketError::InvalidBbsThreadCount(visible));
            }
            let mut threads = Vec::with_capacity(visible as usize);
            for _ in 0..visible {
                threads.push(read_thread(&mut packet)?);
            }
            GuildBbsEvent::ThreadList {
                notice,
                total,
                threads,
            }
        }
        0x07 => {
            let id = packet.read_i32()?;
            let poster_id = packet.read_i32()?;
            let timestamp = packet.read_i64()?;
            let title = packet.read_string()?;
            let content = packet.read_string()?;
            let icon = packet.read_i32()?;
            let count = packet.read_i32()?;
            if !(0..=MAX_BBS_REPLIES as i32).contains(&count) {
                return Err(PacketError::TooManyBbsReplies(MAX_BBS_REPLIES));
            }
            let mut replies = Vec::with_capacity(count as usize);
            for _ in 0..count {
                replies.push(BbsReply {
                    id: packet.read_i32()?,
                    poster_id: packet.read_i32()?,
                    timestamp: packet.read_i64()?,
                    text: packet.read_string()?,
                });
            }
            GuildBbsEvent::ThreadView(BbsThreadView {
                id,
                poster_id,
                timestamp,
                title,
                content,
                icon,
                replies,
            })
        }
        _ => return Err(PacketError::InvalidGuildBbsType(kind)),
    };
    Ok(event)
}

pub fn guild_bbs_list_packet(page: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_BBS_OPERATION);
    packet.write_u8(2);
    packet.write_i32(page.max(0));
    packet.into_bytes()
}

pub fn guild_bbs_view_packet(thread_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_BBS_OPERATION);
    packet.write_u8(3);
    packet.write_i32(thread_id);
    packet.into_bytes()
}

pub fn guild_bbs_write_packet(
    notice: bool,
    title: &str,
    content: &str,
) -> Result<Vec<u8>, PacketError> {
    validate_text_length(title, MAX_BBS_TITLE_LENGTH)?;
    validate_text_length(content, MAX_BBS_CONTENT_LENGTH)?;
    let mut packet = PacketWriter::new(OPCODE_BBS_OPERATION);
    packet.write_u8(0);
    packet.write_u8(0); // create, not edit
    packet.write_u8(u8::from(notice));
    packet.write_string(title)?;
    packet.write_string(content)?;
    packet.write_i32(0);
    Ok(packet.into_bytes())
}

pub fn guild_bbs_delete_packet(thread_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_BBS_OPERATION);
    packet.write_u8(1);
    packet.write_i32(thread_id);
    packet.into_bytes()
}

pub fn guild_bbs_reply_packet(thread_id: i32, content: &str) -> Result<Vec<u8>, PacketError> {
    validate_text_length(content, MAX_BBS_REPLY_LENGTH)?;
    let mut packet = PacketWriter::new(OPCODE_BBS_OPERATION);
    packet.write_u8(4);
    packet.write_i32(thread_id);
    packet.write_string(content)?;
    Ok(packet.into_bytes())
}

fn validate_text_length(text: &str, maximum: usize) -> Result<(), PacketError> {
    let actual = text.encode_utf16().count();
    if actual > maximum {
        return Err(PacketError::InvalidBbsTextLength { actual, maximum });
    }
    Ok(())
}

fn read_thread(packet: &mut PacketReader<'_>) -> Result<BbsThread, PacketError> {
    Ok(BbsThread {
        id: packet.read_i32()?,
        poster_id: packet.read_i32()?,
        title: packet.read_string()?,
        timestamp: packet.read_i64()?,
        icon: packet.read_i32()?,
        reply_count: packet.read_i32()?,
    })
}
