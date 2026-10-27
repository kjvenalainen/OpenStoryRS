use crate::{PacketError, PacketReader};

pub const OPCODE_NOTIFY_LEVEL_UP: u16 = 105;
pub const OPCODE_NOTIFY_MARRIAGE: u16 = 106;
pub const OPCODE_NOTIFY_JOB_CHANGE: u16 = 107;
pub const OPCODE_MARRIAGE_REQUEST: u16 = 72;
pub const OPCODE_MARRIAGE_RESULT: u16 = 73;
pub const OPCODE_WEDDING_GIFT_RESULT: u16 = 74;
pub const OPCODE_NOTIFY_MARRIED_PARTNER: u16 = 75;
pub const OPCODE_SPOUSE_CHAT: u16 = 136;
pub const OPCODE_WEDDING_PROGRESS: u16 = 320;
pub const OPCODE_WEDDING_CEREMONY_END: u16 = 321;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocialNotification {
    LevelUp {
        guild: bool,
        level: i32,
        name: String,
    },
    JobChange {
        guild: bool,
        name: String,
    },
    SpouseChat {
        spouse: bool,
        name: Option<String>,
        text: String,
    },
    MarriageRequest {
        wishlist: bool,
        name: Option<String>,
    },
    MarriageResult {
        code: i8,
    },
    WeddingGiftResult {
        code: i8,
    },
    MarriedPartnerMoved,
    MarriageNotice {
        guild: bool,
        name: String,
    },
    WeddingProgress {
        step: i8,
    },
    WeddingCeremonyEnd,
}

/// Parses v83 guild and family level-up/job advancement broadcasts.
pub fn parse_social_notification(bytes: &[u8]) -> Result<SocialNotification, PacketError> {
    let mut packet = PacketReader::new(bytes);
    let opcode = packet.read_u16()?;
    match opcode {
        OPCODE_NOTIFY_LEVEL_UP => {
            let notification_type = packet.read_i8()?;
            Ok(SocialNotification::LevelUp {
                guild: notification_type == 2,
                level: packet.read_i32()?,
                name: packet.read_string()?,
            })
        }
        OPCODE_NOTIFY_JOB_CHANGE => {
            let notification_type = packet.read_i8()?;
            let _job_id = packet.read_i32()?;
            Ok(SocialNotification::JobChange {
                guild: notification_type == 0,
                name: packet.read_string()?,
            })
        }
        OPCODE_SPOUSE_CHAT => {
            let mode = packet.read_i8()?;
            let name = if mode == 5 {
                Some(packet.read_string()?)
            } else {
                None
            };
            packet.read_i8()?; // submode
            Ok(SocialNotification::SpouseChat {
                spouse: mode == 5,
                name,
                text: packet.read_string()?,
            })
        }
        OPCODE_MARRIAGE_REQUEST => {
            let mode = packet.read_i8()?;
            let name = if mode == 9 {
                None
            } else {
                let name = packet.read_string()?;
                packet.read_i32()?; // Proposer character ID.
                Some(name)
            };
            Ok(SocialNotification::MarriageRequest {
                wishlist: mode == 9,
                name,
            })
        }
        OPCODE_MARRIAGE_RESULT => Ok(SocialNotification::MarriageResult {
            code: packet.read_i8()?,
        }),
        OPCODE_WEDDING_GIFT_RESULT => Ok(SocialNotification::WeddingGiftResult {
            code: packet.read_i8()?,
        }),
        OPCODE_NOTIFY_MARRIED_PARTNER => {
            packet.read_i32()?; // Destination map ID.
            packet.read_i32()?; // Partner character ID.
            Ok(SocialNotification::MarriedPartnerMoved)
        }
        OPCODE_NOTIFY_MARRIAGE => {
            let notification_type = packet.read_i8()?;
            Ok(SocialNotification::MarriageNotice {
                guild: notification_type == 0,
                name: packet.read_string()?,
            })
        }
        OPCODE_WEDDING_PROGRESS => {
            let step = packet.read_i8()?;
            packet.read_i32()?; // Groom character ID.
            packet.read_i32()?; // Bride character ID.
            Ok(SocialNotification::WeddingProgress { step })
        }
        OPCODE_WEDDING_CEREMONY_END => {
            packet.read_i32()?; // Groom character ID.
            packet.read_i32()?; // Bride character ID.
            Ok(SocialNotification::WeddingCeremonyEnd)
        }
        actual => Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_NOTIFY_LEVEL_UP,
            actual,
        }),
    }
}
