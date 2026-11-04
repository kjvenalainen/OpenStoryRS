//! Bounded v83 Maple Trading System packets.

use crate::{PacketError, PacketReader, PacketWriter};

pub const OPCODE_ENTER_MTS: u16 = 156;
pub const OPCODE_MTS_OPERATION_SEND: u16 = 253;
pub const OPCODE_MTS_OPERATION: u16 = 348;
pub const OPCODE_MTS_CASH: u16 = 347;
pub const OPCODE_SET_MTS: u16 = 126;

const MAX_MTS_LISTINGS: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MtsListing {
    pub listing_id: i32,
    pub item_id: i32,
    pub quantity: i16,
    pub taxes: i32,
    pub price: i32,
    pub seller: String,
    pub is_equip: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MtsEvent {
    ListingPage {
        total_items: i32,
        tab: i32,
        category: i32,
        page: i32,
        pages: i32,
        listings: Vec<MtsListing>,
    },
    SellConfirmed,
    TransferInventory(Vec<MtsListing>),
    NotYetSold(Vec<MtsListing>),
    TransferConfirmed {
        quantity: i32,
        position: i32,
    },
    BuyConfirmed,
    BuyFailed,
    WantedListingOver {
        nx: i32,
        items: i32,
    },
    Unknown(u8),
}

/// Parses an MTS operation response (server receive opcode 348).
/// Unknown operation bytes are surfaced without attempting to interpret their
/// payload, matching the original handler's default no-op branch.
pub fn parse_mts_operation(bytes: &[u8]) -> Result<MtsEvent, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_MTS_OPERATION)?;
    let operation = packet.read_u8()?;
    match operation {
        0x15 => {
            let total_items = packet.read_i32()?;
            let count = packet.read_i32()?;
            let tab = packet.read_i32()?;
            let category = packet.read_i32()?;
            let page = packet.read_i32()?;
            packet.skip(2)?;
            let count = checked_mts_count(count)?;
            let mut listings = Vec::with_capacity(count);
            for _ in 0..count {
                listings.push(read_listing(&mut packet)?);
            }
            packet.skip(1)?; // trailing success byte
            Ok(MtsEvent::ListingPage {
                total_items,
                tab,
                category,
                page,
                pages: if total_items > 0 {
                    total_items.saturating_add(15) / 16
                } else {
                    1
                },
                listings,
            })
        }
        0x1d => Ok(MtsEvent::SellConfirmed),
        0x21 => {
            let count = checked_mts_count(packet.read_i32()?)?;
            let listings = read_listings(&mut packet, count)?;
            // The original client ignores server extensions after this list.
            packet.skip(packet.remaining())?;
            Ok(MtsEvent::TransferInventory(listings))
        }
        0x23 => {
            let count = checked_mts_count(packet.read_i32()?)?;
            let listings = if count == 0 {
                packet.read_i32()?; // server's empty-list terminator
                Vec::new()
            } else {
                read_listings(&mut packet, count)?
            };
            Ok(MtsEvent::NotYetSold(listings))
        }
        0x27 => Ok(MtsEvent::TransferConfirmed {
            quantity: packet.read_i32()?,
            position: packet.read_i32()?,
        }),
        0x33 => Ok(MtsEvent::BuyConfirmed),
        0x34 => {
            packet.skip(1)?;
            Ok(MtsEvent::BuyFailed)
        }
        0x3d => Ok(MtsEvent::WantedListingOver {
            nx: packet.read_i32()?,
            items: packet.read_i32()?,
        }),
        other => Ok(MtsEvent::Unknown(other)),
    }
}

/// Parses the MTS balance response (server receive opcode 347).
pub fn parse_mts_cash(bytes: &[u8]) -> Result<[i32; 2], PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_MTS_CASH)?;
    Ok([packet.read_i32()?, packet.read_i32()?])
}

/// Parses the full v83 `SET_ITC` transition snapshot, including the character,
/// inventory, skills, quests, teleport rocks, Monster Book, account name, and
/// 28-byte MTS configuration block.
pub fn parse_mts_open(bytes: &[u8]) -> Result<crate::login::MtsOpenSnapshot, PacketError> {
    crate::login::parse_mts_character_snapshot(bytes)
}

pub fn enter_mts_packet() -> Vec<u8> {
    PacketWriter::new(OPCODE_ENTER_MTS).into_bytes()
}

pub fn mts_change_page_packet(tab: i32, category: i32, page: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_MTS_OPERATION_SEND);
    packet.write_u8(5);
    packet.write_i32(tab);
    packet.write_i32(category);
    packet.write_i32(page);
    packet.into_bytes()
}

pub fn mts_search_packet(
    tab: i32,
    category: i32,
    search_type: i32,
    search: &str,
) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_MTS_OPERATION_SEND);
    packet.write_u8(6);
    packet.write_i32(tab);
    packet.write_i32(category);
    packet.write_i32(0);
    packet.write_i32(search_type);
    packet.write_string(search)?;
    Ok(packet.into_bytes())
}

/// Builds the legacy MTS sell operation. `item_type` is 1 for equips; other
/// values use the stack-item shape, including the v83 rechargeable padding.
pub fn mts_sell_packet(
    item_type: i8,
    item_id: i32,
    stars: i16,
    owner: &str,
    slot: i16,
    quantity: i16,
    price: i32,
) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_MTS_OPERATION_SEND);
    packet.write_u8(2);
    packet.write_i8(item_type);
    packet.write_i32(item_id);
    packet.write_i16(0);
    packet.write_bytes(&[0; 7]);
    if item_type == 1 {
        packet.write_bytes(&[0; 32]);
    } else {
        packet.write_i16(stars);
    }
    packet.write_string(owner)?;
    if item_type == 1 {
        packet.write_bytes(&[0; 32]);
    } else {
        packet.write_i16(0);
    }
    let rechargeable = item_type != 1 && matches!(item_id / 10_000, 207 | 233);
    if rechargeable {
        packet.write_bytes(&[0; 8]);
    }
    packet.write_i32(i32::from(slot));
    if rechargeable {
        packet.write_i32(0);
    } else {
        packet.write_i32(i32::from(quantity));
    }
    packet.write_i32(price);
    Ok(packet.into_bytes())
}

pub fn mts_cancel_sale_packet(listing_id: i32) -> Vec<u8> {
    mts_id_operation_packet(7, listing_id)
}

pub fn mts_transfer_packet(listing_id: i32) -> Vec<u8> {
    mts_id_operation_packet(8, listing_id)
}

pub fn mts_add_cart_packet(listing_id: i32) -> Vec<u8> {
    mts_id_operation_packet(9, listing_id)
}

pub fn mts_remove_cart_packet(listing_id: i32) -> Vec<u8> {
    mts_id_operation_packet(10, listing_id)
}

pub fn mts_buy_packet(listing_id: i32) -> Vec<u8> {
    mts_id_operation_packet(16, listing_id)
}

pub fn mts_buy_from_cart_packet(listing_id: i32) -> Vec<u8> {
    mts_id_operation_packet(17, listing_id)
}

fn mts_id_operation_packet(operation: u8, listing_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_MTS_OPERATION_SEND);
    packet.write_u8(operation);
    packet.write_i32(listing_id);
    packet.into_bytes()
}

fn read_listings(
    packet: &mut PacketReader<'_>,
    count: usize,
) -> Result<Vec<MtsListing>, PacketError> {
    let mut listings = Vec::with_capacity(count);
    for _ in 0..count {
        listings.push(read_listing(packet)?);
    }
    Ok(listings)
}

fn read_listing(packet: &mut PacketReader<'_>) -> Result<MtsListing, PacketError> {
    let (item_id, quantity, is_equip) = read_item_info(packet)?;
    let listing_id = packet.read_i32()?;
    let taxes = packet.read_i32()?;
    let price = packet.read_i32()?;
    packet.skip(4 + 8)?; // reserved word and listing expiration
    let seller = packet.read_string()?;
    let character_name_length = usize::from(packet.read_u16()?);
    packet.skip(character_name_length)?;
    packet.skip(28)?;
    Ok(MtsListing {
        listing_id,
        item_id,
        quantity,
        taxes,
        price,
        seller,
        is_equip,
    })
}

fn read_item_info(packet: &mut PacketReader<'_>) -> Result<(i32, i16, bool), PacketError> {
    let item_type = packet.read_i8()?;
    let item_id = packet.read_i32()?;
    let cash = packet.read_bool()?;
    if cash {
        packet.skip(8)?;
    }
    packet.skip(8)?; // expiration

    if (5_000_000..=5_000_102).contains(&item_id) {
        packet.skip(13 + 1 + 2 + 1 + 18)?;
        return Ok((item_id, 1, false));
    }
    if item_type == 1 {
        packet.skip(2 + 15 * 2)?;
        packet.read_string()?;
        packet.skip(2)?; // flags
        if cash {
            packet.skip(10)?;
        } else {
            packet.skip(1 + 1 + 2 + 2 + 4 + 8)?;
        }
        packet.skip(8 + 4)?; // expiration tail and -1 sentinel
        Ok((item_id, 1, true))
    } else {
        let quantity = packet.read_i16()?;
        packet.read_string()?;
        packet.skip(2)?; // flags
        if matches!(item_id / 10_000, 207 | 233) {
            packet.skip(8)?;
        }
        Ok((item_id, quantity, false))
    }
}

fn checked_mts_count(count: i32) -> Result<usize, PacketError> {
    if count < 0 {
        return Err(PacketError::InvalidMtsCount(count));
    }
    let count = count as usize;
    if count > MAX_MTS_LISTINGS {
        return Err(PacketError::TooManyMtsListings(MAX_MTS_LISTINGS));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mts_confirmations_and_unknown_operations() {
        assert_eq!(
            parse_mts_operation(&[0x5c, 0x01, 0x1d]).unwrap(),
            MtsEvent::SellConfirmed
        );
        assert_eq!(
            parse_mts_operation(&[0x5c, 0x01, 0x7f]).unwrap(),
            MtsEvent::Unknown(0x7f)
        );
    }

    #[test]
    fn parses_listing_transfer_and_confirmation_responses() {
        let mut page = PacketWriter::new(OPCODE_MTS_OPERATION);
        page.write_u8(0x15);
        page.write_i32(20);
        page.write_i32(1);
        page.write_i32(2);
        page.write_i32(3);
        page.write_i32(4);
        page.write_u8(1);
        page.write_u8(1);
        write_listing(&mut page, 0x1234);
        page.write_u8(1);
        assert_eq!(
            parse_mts_operation(&page.into_bytes()).unwrap(),
            MtsEvent::ListingPage {
                total_items: 20,
                tab: 2,
                category: 3,
                page: 4,
                pages: 2,
                listings: vec![MtsListing {
                    listing_id: 0x1234,
                    item_id: 2_000_001,
                    quantity: 7,
                    taxes: 20,
                    price: 100,
                    seller: "seller".to_owned(),
                    is_equip: false,
                }],
            }
        );

        let mut transfer = PacketWriter::new(OPCODE_MTS_OPERATION);
        transfer.write_u8(0x21);
        transfer.write_i32(1);
        write_listing(&mut transfer, 9);
        assert!(matches!(
            parse_mts_operation(&transfer.into_bytes()).unwrap(),
            MtsEvent::TransferInventory(items) if items.len() == 1 && items[0].listing_id == 9
        ));

        let mut unsold = PacketWriter::new(OPCODE_MTS_OPERATION);
        unsold.write_u8(0x23);
        unsold.write_i32(0);
        unsold.write_i32(0);
        assert_eq!(
            parse_mts_operation(&unsold.into_bytes()).unwrap(),
            MtsEvent::NotYetSold(Vec::new())
        );
        let mut transfer_result = PacketWriter::new(OPCODE_MTS_OPERATION);
        transfer_result.write_u8(0x27);
        transfer_result.write_i32(3);
        transfer_result.write_i32(8);
        assert_eq!(
            parse_mts_operation(&transfer_result.into_bytes()).unwrap(),
            MtsEvent::TransferConfirmed {
                quantity: 3,
                position: 8
            }
        );
        assert_eq!(
            parse_mts_operation(&[0x5c, 0x01, 0x33]).unwrap(),
            MtsEvent::BuyConfirmed
        );
        assert_eq!(
            parse_mts_operation(&[0x5c, 0x01, 0x34, 0x42]).unwrap(),
            MtsEvent::BuyFailed
        );
        assert_eq!(
            parse_mts_operation(&[0x5c, 0x01, 0x3d, 1, 0, 0, 0, 2, 0, 0, 0]).unwrap(),
            MtsEvent::WantedListingOver { nx: 1, items: 2 }
        );
        assert_eq!(
            parse_mts_cash(&[0x5b, 0x01, 5, 0, 0, 0, 8, 0, 0, 0]).unwrap(),
            [5, 8]
        );
    }

    #[test]
    fn rejects_truncated_mts_payloads() {
        assert!(matches!(
            parse_mts_operation(&[0x5c, 0x01, 0x27, 1, 0]),
            Err(PacketError::Truncated { .. })
        ));
        assert!(matches!(
            parse_mts_cash(&[0x5b, 0x01, 1, 0, 0, 0]),
            Err(PacketError::Truncated { .. })
        ));
        let mut negative_count = PacketWriter::new(OPCODE_MTS_OPERATION);
        negative_count.write_u8(0x15);
        negative_count.write_i32(0);
        negative_count.write_i32(-1);
        negative_count.write_i32(0);
        negative_count.write_i32(0);
        negative_count.write_i32(0);
        negative_count.write_u8(1);
        negative_count.write_u8(1);
        assert_eq!(
            parse_mts_operation(&negative_count.into_bytes()),
            Err(PacketError::InvalidMtsCount(-1))
        );
    }

    #[test]
    fn parses_full_mts_open_snapshot_and_rejects_truncated_config() {
        let bytes = mts_open_snapshot_packet(0);
        let snapshot = parse_mts_open(&bytes).unwrap();
        assert_eq!(snapshot.character_id, 77);
        assert_eq!(snapshot.stats.name, "OpenStory");
        assert_eq!(snapshot.stats.level, 10);
        assert_eq!(snapshot.stats.map_id, 100_000_000);
        assert_eq!(snapshot.stats.meso, Some(321));
        assert_eq!(snapshot.inventory.meso, 321);
        assert_eq!(snapshot.inventory.slot_maxima, [24, 24, 24, 24, 24]);
        assert_eq!(snapshot.skills, Vec::new());
        assert_eq!(snapshot.cooldowns, Vec::new());
        assert_eq!(snapshot.quest_log.started.len(), 0);
        assert_eq!(
            snapshot.teleport_rock.normal,
            [1000, 1001, 1002, 1003, 1004]
        );
        assert_eq!(snapshot.teleport_rock.vip, (1005..1015).collect::<Vec<_>>());
        assert_eq!(snapshot.monster_book.cover_item_id, 2_380_001);
        assert_eq!(snapshot.buddy_capacity, 20);
        assert_eq!(snapshot.linked_character_name, None);
        assert_eq!(snapshot.account_name, "account");

        let mut truncated = bytes;
        truncated.pop();
        assert!(matches!(
            parse_mts_open(&truncated),
            Err(PacketError::Truncated { .. })
        ));
    }

    #[test]
    fn parses_evans_remaining_skill_point_table_in_mts_stats() {
        let snapshot = parse_mts_open(&mts_open_snapshot_packet(2200)).unwrap();
        assert_eq!(snapshot.stats.skill_points, 0);
        assert_eq!(snapshot.stats.skill_points_by_job, [(0, 1), (1, 2)]);
    }

    #[test]
    fn serializes_representative_mts_requests() {
        assert_eq!(enter_mts_packet(), [0x9c, 0x00]);
        assert_eq!(
            mts_change_page_packet(1, 2, 3),
            [0xfd, 0x00, 5, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]
        );
        assert_eq!(
            mts_buy_packet(0x12345678),
            [0xfd, 0x00, 16, 0x78, 0x56, 0x34, 0x12]
        );
        assert_eq!(
            mts_search_packet(1, 2, 3, "x").unwrap(),
            [
                0xfd, 0x00, 6, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 1, 0, b'x'
            ]
        );
        assert_eq!(mts_cancel_sale_packet(9), [0xfd, 0, 7, 9, 0, 0, 0]);
        assert_eq!(mts_transfer_packet(9), [0xfd, 0, 8, 9, 0, 0, 0]);
        assert_eq!(mts_add_cart_packet(9), [0xfd, 0, 9, 9, 0, 0, 0]);
        assert_eq!(mts_remove_cart_packet(9), [0xfd, 0, 10, 9, 0, 0, 0]);
        assert_eq!(mts_buy_from_cart_packet(9), [0xfd, 0, 17, 9, 0, 0, 0]);
    }

    #[test]
    fn sell_packet_includes_v83_equipment_and_rechargeable_padding() {
        let equip = mts_sell_packet(1, 1_000_001, 0, "A", 2, 1, 500).unwrap();
        assert_eq!(&equip[..3], &[0xfd, 0x00, 2]);
        assert_eq!(
            equip.len(),
            2 + 1 + 1 + 4 + 2 + 7 + 32 + 2 + 1 + 32 + 4 + 4 + 4
        );

        let projectile = mts_sell_packet(2, 2_070_001, 800, "A", 2, 100, 500).unwrap();
        assert_eq!(&projectile[..3], &[0xfd, 0x00, 2]);
        assert_eq!(
            projectile.len(),
            2 + 1 + 1 + 4 + 2 + 7 + 2 + 3 + 2 + 8 + 4 + 4 + 4
        );
    }

    fn write_listing(packet: &mut PacketWriter, listing_id: i32) {
        packet.write_i8(2); // stack item
        packet.write_i32(2_000_001);
        packet.write_bool(false);
        packet.write_i64(0); // item expiration
        packet.write_i16(7);
        packet.write_string("owner").unwrap();
        packet.write_i16(0); // flags
        packet.write_i32(listing_id);
        packet.write_i32(20); // taxes
        packet.write_i32(100); // price
        packet.write_i32(0);
        packet.write_i64(0); // listing expiration
        packet.write_string("seller").unwrap();
        packet.write_string("seller").unwrap();
        packet.write_bytes(&[0; 28]);
    }

    fn mts_open_snapshot_packet(job: u16) -> Vec<u8> {
        let mut packet = PacketWriter::new(OPCODE_SET_MTS);
        packet.write_i64(0); // character-entry timestamp
        packet.write_u8(1); // transition marker
        packet.write_i32(77); // character id
        let mut name = [0; 13];
        name[..9].copy_from_slice(b"OpenStory");
        packet.write_bytes(&name);
        packet.write_bool(true);
        packet.write_u8(0); // skin
        packet.write_i32(20_000); // face
        packet.write_i32(30_000); // hair
        for _ in 0..3 {
            packet.write_i64(0);
        }
        packet.write_u8(10); // level
        packet.write_u16(job);
        for value in [12, 13, 14, 15, 100, 110, 80, 90, 3] {
            packet.write_u16(value);
        }
        if matches!(job, 2001 | 2200 | 2210..=2218) {
            packet.write_u8(2);
            packet.write_u8(0);
            packet.write_u8(1);
            packet.write_u8(1);
            packet.write_u8(2);
        } else {
            packet.write_u16(4); // remaining skill points
        }
        packet.write_i32(500); // experience
        packet.write_u16(2); // fame
        packet.write_i32(0); // gacha experience
        packet.write_i32(100_000_000); // map id
        packet.write_u8(3); // portal
        packet.write_i32(0); // stats timestamp
        packet.write_u8(20); // buddy capacity
        packet.write_bool(false); // linked character name

        packet.write_i32(321); // mesos
        packet.write_bytes(&[24; 5]); // inventory slot maxima
        packet.write_bytes(&[0; 8]);
        packet.write_i16(0);
        packet.write_i16(0);
        packet.write_i16(0);
        packet.write_i16(0); // zero-position terminator
        packet.write_bytes(&[0; 4]); // use/setup/etc/cash terminators
        packet.write_i16(0); // skills
        packet.write_i16(0); // cooldowns
        packet.write_i16(0); // started quests
        packet.write_i16(0); // completed quests
        packet.write_bytes(&[0; 2]); // minigame
        packet.write_i16(0); // ring 1
        packet.write_i16(0); // ring 2
        packet.write_i16(0); // ring 3
        for map_id in 1000..1015 {
            packet.write_i32(map_id);
        }
        packet.write_i32(2_380_001); // Monster Book cover
        packet.write_u8(0); // reserved
        packet.write_i16(0); // cards
        packet.write_i16(0); // New Year cards
        packet.write_i16(0); // area info
        packet.write_i16(0); // character snapshot terminator
        packet.write_string("account").unwrap();
        packet.write_bytes(&[0x5a; 28]); // MTS configuration block
        packet.into_bytes()
    }
}
