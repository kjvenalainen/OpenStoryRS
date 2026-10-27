//! Bounded v83 cash shop packets and locker updates.

use crate::{PacketError, PacketReader, PacketWriter};

pub const OPCODE_CHANGE_MAP: u16 = 38;
pub const OPCODE_ENTER_CASH_SHOP: u16 = 40;
pub const OPCODE_SET_CASH_SHOP: u16 = 127;
pub const OPCODE_QUERY_CASH_RESULT: u16 = 324;
pub const OPCODE_CASH_SHOP_OPERATION: u16 = 325;
pub const OPCODE_BUY_CASH_ITEM: u16 = 229;
pub const OPCODE_COUPON_CODE: u16 = 230;

const MAX_CASH_SHOP_LOCKER_ITEMS: usize = 256;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CashShopItem {
    pub instance_id: i64,
    pub item_id: i32,
    pub commodity_sn: i32,
    pub count: u16,
    pub expiration: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CashShopOperation {
    LockerSnapshot {
        items: Vec<CashShopItem>,
        item_slots: u16,
        character_slots: u16,
    },
    PurchaseAccepted,
    ItemPurchased(CashShopItem),
    ItemWithdrawn {
        inventory_slot: i16,
        item_record: Vec<u8>,
    },
    PurchaseDenied(u8),
    CouponRedeemed,
    /// Cash Shop message code returned for failed shop operations, including
    /// coupon validation failures.
    CashShopMessage(u8),
    Unknown(u8),
}

/// Acknowledges the server's transition packet without parsing its redundant
/// character snapshot; the active character/inventory state is already held by
/// the game session.
pub fn parse_cash_shop_open(body: &[u8]) -> Result<(), PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_SET_CASH_SHOP)
}

/// Parses server-reported NX Credit, Maple Points, and NX Prepaid balances.
pub fn parse_cash_balances(body: &[u8]) -> Result<[i32; 3], PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_QUERY_CASH_RESULT)?;
    Ok([packet.read_i32()?, packet.read_i32()?, packet.read_i32()?])
}

pub fn parse_cash_shop_operation(body: &[u8]) -> Result<CashShopOperation, PacketError> {
    let mut packet = PacketReader::new(body);
    expect_opcode(&mut packet, OPCODE_CASH_SHOP_OPERATION)?;
    let operation = packet.read_u8()?;
    match operation {
        0x4B => {
            let count = usize::from(packet.read_u16()?);
            if count > MAX_CASH_SHOP_LOCKER_ITEMS {
                return Err(PacketError::TooManyCashShopItems(
                    MAX_CASH_SHOP_LOCKER_ITEMS,
                ));
            }
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                items.push(read_cash_shop_item(&mut packet)?);
            }
            Ok(CashShopOperation::LockerSnapshot {
                items,
                item_slots: packet.read_u16()?,
                character_slots: packet.read_u16()?,
            })
        }
        0x4A => Ok(CashShopOperation::PurchaseAccepted),
        0x57 => Ok(CashShopOperation::ItemPurchased(read_cash_shop_item(
            &mut packet,
        )?)),
        0x68 => {
            let inventory_slot = packet.read_i16()?;
            let item_record = packet.read_bytes(packet.remaining())?.to_vec();
            Ok(CashShopOperation::ItemWithdrawn {
                inventory_slot,
                item_record,
            })
        }
        0x4C => Ok(CashShopOperation::PurchaseDenied(packet.read_u8()?)),
        0x59 => Ok(CashShopOperation::CouponRedeemed),
        // Cosmic/HeavenMS encode Cash Shop messages as a one-byte code;
        // coupon failures include invalid, expired, and already-used codes.
        0x5C => Ok(CashShopOperation::CashShopMessage(packet.read_u8()?)),
        operation => Ok(CashShopOperation::Unknown(operation)),
    }
}

pub fn enter_cash_shop_packet() -> Vec<u8> {
    PacketWriter::new(OPCODE_ENTER_CASH_SHOP).into_bytes()
}

/// The legacy client exits the Cash Shop with a bare CHANGE_MAP opcode.
pub fn exit_cash_shop_packet() -> Vec<u8> {
    PacketWriter::new(OPCODE_CHANGE_MAP).into_bytes()
}

/// `currency` uses the server wire values: 1 = NX Credit, 2 = Maple Points,
/// 4 = NX Prepaid. `commodity_sn` must come from the NX commodity table.
pub fn buy_cash_item_packet(currency: u8, commodity_sn: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_BUY_CASH_ITEM);
    packet.write_u8(3);
    packet.write_u8(0);
    packet.write_i32(i32::from(currency));
    packet.write_i32(commodity_sn);
    packet.into_bytes()
}

pub fn withdraw_cash_item_packet(instance_id: i64) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_BUY_CASH_ITEM);
    packet.write_u8(0x0D);
    packet.write_i64(instance_id);
    packet.into_bytes()
}

pub fn cash_shop_coupon_packet(code: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_COUPON_CODE);
    packet.write_string(code)?;
    Ok(packet.into_bytes())
}

fn read_cash_shop_item(packet: &mut PacketReader<'_>) -> Result<CashShopItem, PacketError> {
    let instance_id = packet.read_i64()?;
    packet.read_i32()?; // account id
    packet.read_i32()?; // character id
    let item_id = packet.read_i32()?;
    let commodity_sn = packet.read_i32()?;
    let count = packet.read_u16()?;
    packet.skip(13)?; // sender
    let expiration = packet.read_i64()?;
    packet.read_i64()?; // reserved
    Ok(CashShopItem {
        instance_id,
        item_id,
        commodity_sn,
        count,
        expiration,
    })
}

fn expect_opcode(packet: &mut PacketReader<'_>, expected: u16) -> Result<(), PacketError> {
    let actual = packet.read_u16()?;
    if actual == expected {
        Ok(())
    } else {
        Err(PacketError::UnexpectedOpcode { expected, actual })
    }
}
