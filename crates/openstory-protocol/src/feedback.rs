use crate::{PacketError, PacketReader};

pub const OPCODE_WEEK_EVENT_MESSAGE: u16 = 77;
pub const OPCODE_BLOCKED_MAP: u16 = 131;
pub const OPCODE_BLOCKED_SERVER: u16 = 132;
pub const OPCODE_FORCED_MAP_EQUIP: u16 = 133;
pub const OPCODE_CONTI_MOVE: u16 = 148;
pub const OPCODE_CONTI_STATE: u16 = 149;
pub const OPCODE_CLAIM_STATUS_CHANGED: u16 = 47;
pub const OPCODE_SUE_CHARACTER_RESULT: u16 = 55;
pub const OPCODE_TRADE_MONEY_LIMIT: u16 = 57;
pub const OPCODE_VICIOUS_HAMMER: u16 = 354;
pub const OPCODE_VEGA_SCROLL: u16 = 358;
pub const OPCODE_NEW_YEAR_CARD: u16 = 118;
pub const OPCODE_MAPLELIFE_RESULT: u16 = 349;
pub const OPCODE_MAPLELIFE_ERROR: u16 = 350;
pub const OPCODE_SCRIPT_PROGRESS_MESSAGE: u16 = 122;
pub const OPCODE_CANNOT_SPAWN_KITE: u16 = 270;
pub const OPCODE_SPAWN_KITE: u16 = 271;
pub const OPCODE_REMOVE_KITE: u16 = 272;
pub const OPCODE_PING: u16 = 17;
pub const OPCODE_CANCEL_NAME_CHANGE: u16 = 113;
pub const OPCODE_CANCEL_WORLD_TRANSFER: u16 = 114;
pub const OPCODE_INCUBATOR_RESULT: u16 = 69;
pub const OPCODE_CS_CHECK_NAME_CHANGE: u16 = 328;
pub const OPCODE_CS_NAME_CHANGE_POSSIBLE: u16 = 329;
pub const OPCODE_CS_TRANSFER_WORLD: u16 = 331;
pub const OPCODE_CS_CASH_GACHAPON_RESULT: u16 = 333;

const MAX_CASH_SHOP_WORLDS: usize = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerFeedback {
    WeekEventMessage(String),
    BlockedMap(i8),
    BlockedServer(i8),
    ForcedMapEquip,
    ContiMove(Option<i8>),
    ContiState(Option<i8>),
    ClaimStatusChanged(Option<u8>),
    SueCharacterResult(Option<i8>),
    TradeMoneyLimit,
    ViciousHammerUsed(i32),
    ViciousHammerComplete,
    ViciousHammerOther(i8),
    VegaScroll(i8),
    NewYearCard(i8),
    MapleLifeResult(i32),
    MapleLifeError(i32),
    ScriptProgressMessage(Option<String>),
    CannotSpawnKite,
    KiteSpawn { name: String, message: String },
    KiteRemoved,
    Ping,
    CancelResult(Option<CancelRequestResponse>),
    IncubatorFinished,
    CashShop(CashShopFeedback),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CashShopFeedback {
    NameAvailability {
        name: String,
        in_use: bool,
    },
    NameChangePossible(i8),
    WorldTransfer {
        error: i8,
        worlds: Option<Vec<String>>,
    },
    GachaponFailed,
    GachaponOpened {
        remaining: i32,
    },
    GachaponOther(i8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelRequestKind {
    NameChange,
    WorldTransfer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CancelRequestResponse {
    pub kind: CancelRequestKind,
    pub success: bool,
    pub reason: Option<i8>,
}

/// Parses v83 server feedback and entry restrictions.
pub fn parse_server_feedback(bytes: &[u8]) -> Result<ServerFeedback, PacketError> {
    let mut packet = PacketReader::new(bytes);
    let opcode = packet.read_u16()?;
    match opcode {
        OPCODE_WEEK_EVENT_MESSAGE => {
            packet.read_u8()?; // legacy padding byte (normally 0xFF)
            Ok(ServerFeedback::WeekEventMessage(packet.read_string()?))
        }
        OPCODE_BLOCKED_MAP => Ok(ServerFeedback::BlockedMap(packet.read_i8()?)),
        OPCODE_BLOCKED_SERVER => Ok(ServerFeedback::BlockedServer(packet.read_i8()?)),
        OPCODE_FORCED_MAP_EQUIP => Ok(ServerFeedback::ForcedMapEquip),
        OPCODE_CONTI_MOVE => {
            if packet.is_empty() {
                return Ok(ServerFeedback::ContiMove(None));
            }
            packet.read_i8()?; // event type (10 = ship/crog)
            Ok(ServerFeedback::ContiMove(Some(packet.read_i8()?)))
        }
        OPCODE_CONTI_STATE => {
            if packet.is_empty() {
                return Ok(ServerFeedback::ContiState(None));
            }
            let state = packet.read_i8()?;
            if !packet.is_empty() {
                packet.read_i8()?; // optional padding byte
            }
            Ok(ServerFeedback::ContiState(Some(state)))
        }
        OPCODE_CLAIM_STATUS_CHANGED => {
            Ok(ServerFeedback::ClaimStatusChanged(if packet.is_empty() {
                None
            } else {
                Some(packet.read_u8()?)
            }))
        }
        OPCODE_SUE_CHARACTER_RESULT => {
            Ok(ServerFeedback::SueCharacterResult(if packet.is_empty() {
                None
            } else {
                Some(packet.read_i8()?)
            }))
        }
        OPCODE_TRADE_MONEY_LIMIT => Ok(ServerFeedback::TradeMoneyLimit),
        OPCODE_VICIOUS_HAMMER => {
            let operation = packet.read_i8()?;
            match operation {
                0x39 => {
                    packet.read_i32()?; // reserved zero
                    Ok(ServerFeedback::ViciousHammerUsed(packet.read_i32()?))
                }
                0x3d => {
                    packet.read_i32()?; // reserved zero
                    Ok(ServerFeedback::ViciousHammerComplete)
                }
                other => Ok(ServerFeedback::ViciousHammerOther(other)),
            }
        }
        OPCODE_VEGA_SCROLL => Ok(ServerFeedback::VegaScroll(packet.read_i8()?)),
        OPCODE_NEW_YEAR_CARD => Ok(ServerFeedback::NewYearCard(packet.read_i8()?)),
        OPCODE_MAPLELIFE_RESULT => Ok(ServerFeedback::MapleLifeResult(packet.read_i32()?)),
        OPCODE_MAPLELIFE_ERROR => {
            packet.read_u8()?; // legacy padding byte
            Ok(ServerFeedback::MapleLifeError(packet.read_i32()?))
        }
        OPCODE_SCRIPT_PROGRESS_MESSAGE => Ok(ServerFeedback::ScriptProgressMessage(
            if packet.is_empty() {
                None
            } else {
                Some(packet.read_string()?)
            },
        )),
        OPCODE_CANNOT_SPAWN_KITE => Ok(ServerFeedback::CannotSpawnKite),
        OPCODE_SPAWN_KITE => {
            packet.read_i32()?; // object id
            packet.read_i32()?; // item id
            let message = packet.read_string()?;
            let name = packet.read_string()?;
            packet.read_i16()?; // x
            packet.read_i16()?; // foothold
            Ok(ServerFeedback::KiteSpawn { name, message })
        }
        OPCODE_REMOVE_KITE => {
            packet.read_i8()?; // animation
            packet.read_i32()?; // object id
            Ok(ServerFeedback::KiteRemoved)
        }
        OPCODE_PING => Ok(ServerFeedback::Ping),
        OPCODE_CANCEL_NAME_CHANGE | OPCODE_CANCEL_WORLD_TRANSFER => {
            let kind = if opcode == OPCODE_CANCEL_NAME_CHANGE {
                CancelRequestKind::NameChange
            } else {
                CancelRequestKind::WorldTransfer
            };
            let response = if packet.is_empty() {
                None
            } else {
                let success = packet.read_bool()?;
                let reason = if !success && !packet.is_empty() {
                    Some(packet.read_i8()?)
                } else {
                    None
                };
                Some(CancelRequestResponse {
                    kind,
                    success,
                    reason,
                })
            };
            Ok(ServerFeedback::CancelResult(response))
        }
        OPCODE_INCUBATOR_RESULT => Ok(ServerFeedback::IncubatorFinished),
        OPCODE_CS_CHECK_NAME_CHANGE => Ok(ServerFeedback::CashShop(
            CashShopFeedback::NameAvailability {
                name: packet.read_string()?,
                in_use: packet.read_bool()?,
            },
        )),
        OPCODE_CS_NAME_CHANGE_POSSIBLE => {
            packet.read_i32()?; // reserved zero
            let error = packet.read_i8()?;
            packet.read_i32()?; // reserved zero
            Ok(ServerFeedback::CashShop(
                CashShopFeedback::NameChangePossible(error),
            ))
        }
        OPCODE_CS_TRANSFER_WORLD => {
            packet.read_i32()?; // reserved zero
            let error = packet.read_i8()?;
            packet.read_i32()?; // reserved zero
            let worlds = if packet.read_bool()? {
                let count = packet.read_i32()?;
                if count < 0 {
                    return Err(PacketError::InvalidCashShopWorldCount(count));
                }
                let count = count as usize;
                if count > MAX_CASH_SHOP_WORLDS {
                    return Err(PacketError::TooManyCashShopWorlds(count));
                }
                let mut worlds = Vec::with_capacity(count);
                for _ in 0..count {
                    worlds.push(packet.read_string()?);
                }
                Some(worlds)
            } else {
                None
            };
            Ok(ServerFeedback::CashShop(CashShopFeedback::WorldTransfer {
                error,
                worlds,
            }))
        }
        OPCODE_CS_CASH_GACHAPON_RESULT => {
            let operation = packet.read_i8()?;
            let feedback = match operation {
                value if value == 0xE4_u8 as i8 => CashShopFeedback::GachaponFailed,
                value if value == 0xE5_u8 as i8 => {
                    packet.read_i64()?; // box cash id
                    CashShopFeedback::GachaponOpened {
                        remaining: packet.read_i32()?,
                    }
                }
                other => CashShopFeedback::GachaponOther(other),
            };
            Ok(ServerFeedback::CashShop(feedback))
        }
        _ => Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_WEEK_EVENT_MESSAGE,
            actual: opcode,
        }),
    }
}
