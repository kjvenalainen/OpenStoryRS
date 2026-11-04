//! Bounded parsers for small legacy login, social, event, and Duey responses.

use crate::inventory::skip_item_record;
use crate::{PacketError, PacketReader};

pub const OPCODE_RELOG_RESPONSE: u16 = 22;
pub const OPCODE_MEMO_RESULT: u16 = 41;
pub const OPCODE_SET_GENDER: u16 = 58;
pub const OPCODE_SESSION_VALUE: u16 = 90;
pub const OPCODE_DATA_CRC_CHECK_FAILED: u16 = 123;
pub const OPCODE_ADMIN_RESULT: u16 = 144;
pub const OPCODE_GMEVENT_INSTRUCTIONS: u16 = 146;
pub const OPCODE_FREDRICK_MESSAGE: u16 = 310;
pub const OPCODE_FREDRICK: u16 = 311;
pub const OPCODE_PARCEL: u16 = 322;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MiscServerEvent {
    RelogResponse {
        success: bool,
    },
    MemoResult(MemoResult),
    GenderSet(u8),
    SessionValue {
        label: String,
        value: String,
    },
    PoliceNotice(String),
    AdminResult {
        command: Option<i8>,
        mode: Option<i8>,
    },
    GmEventInstructions,
    FredrickMessage(i8),
    Fredrick(i8),
    Parcel(ParcelEvent),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoResult {
    Sent,
    Failed { error: i8 },
    Other(i8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParcelEvent {
    PackagesAvailable { count: u8 },
    ActionEnabled,
    InsufficientMesos,
    InvalidRequest,
    RecipientNotFound,
    CannotSendToSelf,
    RecipientStorageFull,
    InsufficientInventorySpace,
    Sent,
    Received,
    ReceivedFrom { sender: String, quick: bool },
    ArrivalNotification { quick: bool },
    Other(i8),
}

/// Parses legacy server responses whose body semantics do not need a new UI.
/// Unknown subcommands are retained as no-op events, as in the original client.
pub fn parse_misc_server_packet(bytes: &[u8]) -> Result<MiscServerEvent, PacketError> {
    let mut packet = PacketReader::new(bytes);
    let opcode = packet.read_u16()?;
    match opcode {
        OPCODE_RELOG_RESPONSE => Ok(MiscServerEvent::RelogResponse {
            success: packet.read_u8()? == 1,
        }),
        OPCODE_MEMO_RESULT => {
            let mode = packet.read_i8()?;
            let result = match mode {
                4 => MemoResult::Sent,
                5 => MemoResult::Failed {
                    error: if packet.is_empty() {
                        0
                    } else {
                        packet.read_i8()?
                    },
                },
                other => MemoResult::Other(other),
            };
            Ok(MiscServerEvent::MemoResult(result))
        }
        OPCODE_SET_GENDER => Ok(MiscServerEvent::GenderSet(packet.read_u8()?)),
        OPCODE_SESSION_VALUE => {
            let label = packet.read_string()?;
            let value = if packet.is_empty() {
                String::new()
            } else {
                packet.read_string()?
            };
            Ok(MiscServerEvent::SessionValue { label, value })
        }
        OPCODE_DATA_CRC_CHECK_FAILED => {
            let message = if packet.is_empty() {
                String::new()
            } else {
                packet.read_string()?
            };
            Ok(MiscServerEvent::PoliceNotice(message))
        }
        OPCODE_ADMIN_RESULT => {
            let command = if packet.is_empty() {
                None
            } else {
                Some(packet.read_i8()?)
            };
            let mode = if command.is_some() && !packet.is_empty() {
                Some(packet.read_i8()?)
            } else {
                None
            };
            Ok(MiscServerEvent::AdminResult { command, mode })
        }
        OPCODE_GMEVENT_INSTRUCTIONS => Ok(MiscServerEvent::GmEventInstructions),
        OPCODE_FREDRICK_MESSAGE => Ok(MiscServerEvent::FredrickMessage(packet.read_i8()?)),
        OPCODE_FREDRICK => Ok(MiscServerEvent::Fredrick(packet.read_i8()?)),
        OPCODE_PARCEL => parse_parcel(&mut packet),
        actual => Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_MEMO_RESULT,
            actual,
        }),
    }
}

fn parse_parcel(packet: &mut PacketReader<'_>) -> Result<MiscServerEvent, PacketError> {
    let operation = packet.read_i8()?;
    let event = match operation {
        0x08 => {
            packet.read_u8()?; // unused byte
            let count = packet.read_i8()?;
            if count < 0 {
                return Err(PacketError::InvalidParcelCount(count));
            }
            let count = count as u8;
            for _ in 0..count {
                packet.skip(4)?; // package ID
                packet.read_string_bytes(13)?; // fixed-width sender name
                packet.read_i32()?; // mesos
                packet.read_i64()?; // sent timestamp
                packet.read_i32()?; // message-present flag
                packet.skip(200)?; // fixed-width message buffer
                packet.skip(1)?; // unused
                if packet.read_u8()? == 1 {
                    skip_item_record(packet)?;
                }
            }
            packet.skip(1)?; // terminal byte
            ParcelEvent::PackagesAvailable { count }
        }
        0x09 | 0x14 => ParcelEvent::ActionEnabled,
        0x0a => ParcelEvent::InsufficientMesos,
        0x0b => ParcelEvent::InvalidRequest,
        0x0c => ParcelEvent::RecipientNotFound,
        0x0d => ParcelEvent::CannotSendToSelf,
        0x0e => ParcelEvent::RecipientStorageFull,
        0x12 => ParcelEvent::Sent,
        0x15 => ParcelEvent::InsufficientInventorySpace,
        0x17 => ParcelEvent::Received,
        0x19 => ParcelEvent::ReceivedFrom {
            sender: packet.read_string()?,
            quick: packet.read_bool()?,
        },
        0x1b => ParcelEvent::ArrivalNotification {
            quick: packet.read_bool()?,
        },
        other => ParcelEvent::Other(other),
    };
    Ok(MiscServerEvent::Parcel(event))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PacketWriter;

    #[test]
    fn parses_memo_relog_gender_and_admin_variants() {
        assert_eq!(
            parse_misc_server_packet(&[OPCODE_RELOG_RESPONSE as u8, 0, 1]).unwrap(),
            MiscServerEvent::RelogResponse { success: true }
        );
        assert_eq!(
            parse_misc_server_packet(&[OPCODE_MEMO_RESULT as u8, 0, 4]).unwrap(),
            MiscServerEvent::MemoResult(MemoResult::Sent)
        );
        assert_eq!(
            parse_misc_server_packet(&[OPCODE_MEMO_RESULT as u8, 0, 5, 2]).unwrap(),
            MiscServerEvent::MemoResult(MemoResult::Failed { error: 2 })
        );
        assert_eq!(
            parse_misc_server_packet(&[OPCODE_MEMO_RESULT as u8, 0, 5]).unwrap(),
            MiscServerEvent::MemoResult(MemoResult::Failed { error: 0 })
        );
        assert_eq!(
            parse_misc_server_packet(&[OPCODE_MEMO_RESULT as u8, 0, 1]).unwrap(),
            MiscServerEvent::MemoResult(MemoResult::Other(1))
        );
        assert_eq!(
            parse_misc_server_packet(&[OPCODE_RELOG_RESPONSE as u8, 0, 0]).unwrap(),
            MiscServerEvent::RelogResponse { success: false }
        );
        assert_eq!(
            parse_misc_server_packet(&[OPCODE_SET_GENDER as u8, 0, 1]).unwrap(),
            MiscServerEvent::GenderSet(1)
        );
        assert_eq!(
            parse_misc_server_packet(&[OPCODE_ADMIN_RESULT as u8, 0, 0x10, 1]).unwrap(),
            MiscServerEvent::AdminResult {
                command: Some(0x10),
                mode: Some(1),
            }
        );
        assert_eq!(
            parse_misc_server_packet(&[OPCODE_ADMIN_RESULT as u8, 0]).unwrap(),
            MiscServerEvent::AdminResult {
                command: None,
                mode: None,
            }
        );
    }

    #[test]
    fn parses_session_values_police_notices_and_noop_event_packets() {
        let mut value = PacketWriter::new(OPCODE_SESSION_VALUE);
        value.write_string("energy").unwrap();
        value.write_string("125").unwrap();
        assert_eq!(
            parse_misc_server_packet(&value.into_bytes()).unwrap(),
            MiscServerEvent::SessionValue {
                label: "energy".to_owned(),
                value: "125".to_owned(),
            }
        );

        let mut police = PacketWriter::new(OPCODE_DATA_CRC_CHECK_FAILED);
        police
            .write_string("Disconnected by server policy.")
            .unwrap();
        assert_eq!(
            parse_misc_server_packet(&police.into_bytes()).unwrap(),
            MiscServerEvent::PoliceNotice("Disconnected by server policy.".to_owned())
        );
        assert_eq!(
            parse_misc_server_packet(&[OPCODE_GMEVENT_INSTRUCTIONS as u8, 0]).unwrap(),
            MiscServerEvent::GmEventInstructions
        );
        let mut fredrick = PacketWriter::new(OPCODE_FREDRICK);
        fredrick.write_i8(7);
        assert_eq!(
            parse_misc_server_packet(&fredrick.into_bytes()).unwrap(),
            MiscServerEvent::Fredrick(7)
        );
        let mut fredrick_message = PacketWriter::new(OPCODE_FREDRICK_MESSAGE);
        fredrick_message.write_i8(0x23);
        assert_eq!(
            parse_misc_server_packet(&fredrick_message.into_bytes()).unwrap(),
            MiscServerEvent::FredrickMessage(0x23)
        );
        assert_eq!(
            parse_misc_server_packet(&[OPCODE_DATA_CRC_CHECK_FAILED as u8, 0]).unwrap(),
            MiscServerEvent::PoliceNotice(String::new())
        );
    }

    #[test]
    fn parses_duey_result_notifications() {
        for (operation, expected) in [
            (0x09, ParcelEvent::ActionEnabled),
            (0x14, ParcelEvent::ActionEnabled),
            (0x0a, ParcelEvent::InsufficientMesos),
            (0x0b, ParcelEvent::InvalidRequest),
            (0x0c, ParcelEvent::RecipientNotFound),
            (0x0d, ParcelEvent::CannotSendToSelf),
            (0x0e, ParcelEvent::RecipientStorageFull),
            (0x12, ParcelEvent::Sent),
            (0x15, ParcelEvent::InsufficientInventorySpace),
            (0x17, ParcelEvent::Received),
        ] {
            let mut packet = PacketWriter::new(OPCODE_PARCEL);
            packet.write_i8(operation);
            assert_eq!(
                parse_misc_server_packet(&packet.into_bytes()).unwrap(),
                MiscServerEvent::Parcel(expected)
            );
        }

        let mut from = PacketWriter::new(OPCODE_PARCEL);
        from.write_i8(0x19);
        from.write_string("MapleStory").unwrap();
        from.write_bool(true);
        assert_eq!(
            parse_misc_server_packet(&from.into_bytes()).unwrap(),
            MiscServerEvent::Parcel(ParcelEvent::ReceivedFrom {
                sender: "MapleStory".to_owned(),
                quick: true,
            })
        );

        let mut arrival = PacketWriter::new(OPCODE_PARCEL);
        arrival.write_i8(0x1b);
        arrival.write_bool(false);
        assert_eq!(
            parse_misc_server_packet(&arrival.into_bytes()).unwrap(),
            MiscServerEvent::Parcel(ParcelEvent::ArrivalNotification { quick: false })
        );
    }

    #[test]
    fn parses_duey_package_lists_and_rejects_truncated_or_negative_counts() {
        let mut packet = PacketWriter::new(OPCODE_PARCEL);
        packet.write_i8(8);
        packet.write_u8(0);
        packet.write_i8(1);
        packet.write_i32(10);
        packet.write_bytes(b"sender\0\0\0\0\0\0\0");
        packet.write_i32(25);
        packet.write_i64(1234);
        packet.write_i32(1);
        packet.write_bytes(&[0; 200]);
        packet.write_u8(0);
        packet.write_u8(0); // no item
        packet.write_u8(0); // terminal
        assert_eq!(
            parse_misc_server_packet(&packet.into_bytes()).unwrap(),
            MiscServerEvent::Parcel(ParcelEvent::PackagesAvailable { count: 1 })
        );

        let mut with_item = PacketWriter::new(OPCODE_PARCEL);
        with_item.write_i8(8);
        with_item.write_u8(0);
        with_item.write_i8(1);
        with_item.write_i32(11);
        with_item.write_bytes(b"sender\0\0\0\0\0\0\0");
        with_item.write_i32(0);
        with_item.write_i64(0);
        with_item.write_i32(0);
        with_item.write_bytes(&[0; 200]);
        with_item.write_u8(0);
        with_item.write_u8(1); // has item
        with_item.write_u8(2); // item type
        with_item.write_i32(2_000_000);
        with_item.write_bool(false);
        with_item.write_i64(0);
        with_item.write_i16(1);
        with_item.write_string("owner").unwrap();
        with_item.write_i16(0);
        with_item.write_u8(0); // terminal
        let with_item = with_item.into_bytes();
        assert_eq!(
            parse_misc_server_packet(&with_item).unwrap(),
            MiscServerEvent::Parcel(ParcelEvent::PackagesAvailable { count: 1 })
        );
        assert!(matches!(
            parse_misc_server_packet(&with_item[..with_item.len() - 5]),
            Err(PacketError::Truncated { .. })
        ));

        let mut negative_count = PacketWriter::new(OPCODE_PARCEL);
        negative_count.write_i8(8);
        negative_count.write_u8(0);
        negative_count.write_i8(-1);
        assert_eq!(
            parse_misc_server_packet(&negative_count.into_bytes()),
            Err(PacketError::InvalidParcelCount(-1))
        );
        assert!(matches!(
            parse_misc_server_packet(&[OPCODE_PARCEL as u8, 1, 8, 0, 1]),
            Err(PacketError::Truncated { .. })
        ));
        assert_eq!(
            parse_misc_server_packet(&[OPCODE_PARCEL as u8, 1, 0x7f]).unwrap(),
            MiscServerEvent::Parcel(ParcelEvent::Other(0x7f))
        );
    }

    #[test]
    fn rejects_short_fields_without_partial_events() {
        assert!(matches!(
            parse_misc_server_packet(&[OPCODE_RELOG_RESPONSE as u8, 0]),
            Err(PacketError::Truncated { .. })
        ));
        assert!(matches!(
            parse_misc_server_packet(&[OPCODE_MEMO_RESULT as u8, 0]),
            Err(PacketError::Truncated { .. })
        ));
        let mut value = PacketWriter::new(OPCODE_SESSION_VALUE);
        value.write_u16(5);
        value.write_bytes(b"abc");
        assert!(matches!(
            parse_misc_server_packet(&value.into_bytes()),
            Err(PacketError::Truncated { .. })
        ));
    }
}
