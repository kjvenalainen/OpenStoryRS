//! Bounded support for the custom v83 bot-inventory snapshot used by Cosmic.

use crate::{PacketError, PacketReader, PacketWriter};

pub const OPCODE_BOT_INVENTORY: u16 = 373;
pub const OPCODE_BOT_INVENTORY_ACTION: u16 = 360;

const MAX_BOT_INVENTORY_ITEMS: usize = 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BotInventoryItem {
    /// Tab 0 represents the equipped block; tab values 1 through 5 are the
    /// inventory section IDs transmitted by the server.
    pub tab: u8,
    pub slot: i16,
    pub item_id: i32,
    pub count: i16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BotInventorySnapshot {
    pub bot_index: i8,
    pub meso: Option<i32>,
    pub items: Vec<BotInventoryItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BotInventoryEvent {
    Snapshot(BotInventorySnapshot),
    Rejected(u8),
}

/// Parses the custom BOT_INVENTORY snapshot (server receive opcode 373).
/// All items and stack-count changes are decoded into a temporary value, so a
/// malformed response cannot expose a partially updated inventory.
pub fn parse_bot_inventory(bytes: &[u8]) -> Result<BotInventoryEvent, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_BOT_INVENTORY)?;
    let response = packet.read_u8()?;
    if response != 1 {
        return Ok(BotInventoryEvent::Rejected(response));
    }

    let bot_index = packet.read_i8()?;
    let mut items = Vec::new();
    let equipped_count = usize::from(packet.read_u8()?);
    read_item_block(&mut packet, 0, equipped_count, &mut items)?;

    if !packet.is_empty() {
        let tab_count = usize::from(packet.read_u8()?);
        for _ in 0..tab_count {
            let tab = packet.read_u8()?;
            let count = usize::from(packet.read_u8()?);
            read_item_block(&mut packet, tab, count, &mut items)?;
        }
    }

    let meso = if packet.remaining() >= 4 {
        Some(packet.read_i32()?)
    } else {
        None
    };
    if !packet.is_empty() {
        let count = packet.read_i16()?;
        if count < 0 {
            return Err(PacketError::InvalidBotInventoryCount(count));
        }
        let count = usize::try_from(count).expect("nonnegative count fits usize");
        ensure_item_capacity(items.len(), count)?;
        for _ in 0..count {
            let tab = packet.read_u8()?;
            let slot = packet.read_i16()?;
            let quantity = packet.read_i16()?;
            if let Some(item) = items
                .iter_mut()
                .find(|item| item.tab == tab && item.slot == slot)
            {
                item.count = quantity;
            }
        }
    }

    Ok(BotInventoryEvent::Snapshot(BotInventorySnapshot {
        bot_index,
        meso,
        items,
    }))
}

pub fn bot_inventory_take_packet(bot_id: i32, inventory_type: i8, slot: i16) -> Vec<u8> {
    bot_inventory_slot_packet(0, bot_id, inventory_type, slot)
}

pub fn bot_inventory_give_packet(bot_id: i32, inventory_type: i8, slot: i16) -> Vec<u8> {
    bot_inventory_slot_packet(1, bot_id, inventory_type, slot)
}

pub fn bot_inventory_equip_packet(bot_id: i32, source_slot: i16, destination_slot: i16) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_BOT_INVENTORY_ACTION);
    packet.write_u8(2);
    packet.write_i32(bot_id);
    packet.write_i16(source_slot);
    packet.write_i16(destination_slot);
    packet.into_bytes()
}

fn bot_inventory_slot_packet(action: u8, bot_id: i32, inventory_type: i8, slot: i16) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_BOT_INVENTORY_ACTION);
    packet.write_u8(action);
    packet.write_i32(bot_id);
    packet.write_i8(inventory_type);
    packet.write_i16(slot);
    packet.into_bytes()
}

fn read_item_block(
    packet: &mut PacketReader<'_>,
    tab: u8,
    count: usize,
    items: &mut Vec<BotInventoryItem>,
) -> Result<(), PacketError> {
    ensure_item_capacity(items.len(), count)?;
    for _ in 0..count {
        let slot = packet.read_i16()?;
        let (item_id, item_count) = read_item_info(packet)?;
        items.push(BotInventoryItem {
            tab,
            slot,
            item_id,
            count: item_count,
        });
    }
    Ok(())
}

fn read_item_info(packet: &mut PacketReader<'_>) -> Result<(i32, i16), PacketError> {
    let item_type = packet.read_i8()?;
    let item_id = packet.read_i32()?;
    let cash = packet.read_bool()?;
    if cash {
        packet.skip(8)?;
    }
    packet.skip(8)?; // expiration

    if (5_000_000..=5_000_102).contains(&item_id) {
        packet.skip(13 + 1 + 2 + 1 + 18)?;
        return Ok((item_id, 1));
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
        packet.skip(12)?;
        Ok((item_id, 1))
    } else {
        let count = packet.read_i16()?;
        packet.read_string()?;
        packet.skip(2)?; // flags
        if matches!(item_id / 10_000, 207 | 233) {
            packet.skip(8)?;
        }
        Ok((item_id, count))
    }
}

fn ensure_item_capacity(current: usize, additional: usize) -> Result<(), PacketError> {
    if current.saturating_add(additional) > MAX_BOT_INVENTORY_ITEMS {
        Err(PacketError::TooManyBotInventoryItems(
            MAX_BOT_INVENTORY_ITEMS,
        ))
    } else {
        Ok(())
    }
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
    fn parses_bot_snapshot_and_stack_updates() {
        let mut packet = PacketWriter::new(OPCODE_BOT_INVENTORY);
        packet.write_u8(1); // success
        packet.write_i8(7); // bot index
        packet.write_u8(0); // equipped items
        packet.write_u8(1); // one inventory tab
        packet.write_u8(2); // Use tab
        packet.write_u8(1); // one item
        packet.write_i16(4); // slot
        write_stack_item(&mut packet, 2_000_001, 3);
        packet.write_i32(456); // mesos
        packet.write_i16(1); // quantity update count
        packet.write_u8(2);
        packet.write_i16(4);
        packet.write_i16(9);

        let BotInventoryEvent::Snapshot(snapshot) =
            parse_bot_inventory(&packet.into_bytes()).unwrap()
        else {
            panic!("expected inventory snapshot");
        };
        assert_eq!(snapshot.bot_index, 7);
        assert_eq!(snapshot.meso, Some(456));
        assert_eq!(
            snapshot.items,
            [BotInventoryItem {
                tab: 2,
                slot: 4,
                item_id: 2_000_001,
                count: 9,
            }]
        );
    }

    #[test]
    fn rejects_truncated_bot_items_without_returning_a_snapshot() {
        let mut packet = PacketWriter::new(OPCODE_BOT_INVENTORY);
        packet.write_u8(1);
        packet.write_i8(1);
        packet.write_u8(1);
        packet.write_i16(1);
        packet.write_i8(2); // truncated item header
        assert!(matches!(
            parse_bot_inventory(&packet.into_bytes()),
            Err(PacketError::Truncated { .. })
        ));
        assert_eq!(
            parse_bot_inventory(&[0x75, 0x01, 2]),
            Ok(BotInventoryEvent::Rejected(2))
        );
    }

    #[test]
    fn serializes_bot_inventory_actions() {
        assert_eq!(
            bot_inventory_take_packet(0x12345678, 2, 0x1234),
            [0x68, 0x01, 0, 0x78, 0x56, 0x34, 0x12, 2, 0x34, 0x12]
        );
        assert_eq!(
            bot_inventory_give_packet(1, 4, 2),
            [0x68, 0x01, 1, 1, 0, 0, 0, 4, 2, 0]
        );
        assert_eq!(
            bot_inventory_equip_packet(1, 2, -1),
            [0x68, 0x01, 2, 1, 0, 0, 0, 2, 0, 0xff, 0xff]
        );
    }

    fn write_stack_item(packet: &mut PacketWriter, item_id: i32, count: i16) {
        packet.write_i8(2);
        packet.write_i32(item_id);
        packet.write_bool(false);
        packet.write_i64(0); // expiration
        packet.write_i16(count);
        packet.write_string("").unwrap();
        packet.write_i16(0); // flags
    }
}
