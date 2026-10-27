use crate::login::{expect_opcode, read_character_look};
use crate::packet::PacketWriter;
use crate::{CharacterLook, InventoryType, PacketError, PacketReader};

/// v83 client opcode for player-to-player interactions and personal shops.
pub const OPCODE_PLAYER_INTERACTION: u16 = 123;
pub const OPCODE_HIRED_MERCHANT_REQUEST: u16 = 63;
pub const OPCODE_ENTRUSTED_SHOP_CHECK_RESULT: u16 = 50;
pub const OPCODE_OWL_ACTION: u16 = 66;
pub const OPCODE_OWL_WARP: u16 = 67;
pub const OPCODE_USE_CASH_ITEM: u16 = 79;
pub const OPCODE_SHOP_SCANNER_RESULT: u16 = 70;
pub const OPCODE_SHOP_LINK_RESULT: u16 = 71;
pub const OPCODE_SPAWN_HIRED_MERCHANT: u16 = 265;
pub const OPCODE_DESTROY_HIRED_MERCHANT: u16 = 266;
pub const OPCODE_UPDATE_HIRED_MERCHANT: u16 = 267;
pub const OPCODE_PLAYER_INTERACTION_RESULT: u16 = 314;
const MAX_OWL_RESULTS: usize = 100;
const MAX_OWL_TOP_ITEMS: usize = 10;
const MAX_MERCHANT_VISITORS: usize = 3;
const MAX_MERCHANT_MESSAGES: usize = 60;
const MAX_MERCHANT_ITEMS: usize = 16;
const MAX_TRADE_ITEMS: usize = 9;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwlShopResult {
    pub owner: String,
    pub map_id: i32,
    pub description: String,
    pub bundles: i32,
    pub per_bundle: i32,
    pub price: i32,
    pub owner_id: i32,
    pub channel: i8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiredMerchantSpawn {
    pub owner_id: i32,
    pub object_id: i32,
    pub item_id: i32,
    pub x: i16,
    pub y: i16,
    pub owner_name: String,
    pub description: String,
    pub skin: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HiredMerchantRemove {
    pub owner_id: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiredMerchantUpdate {
    pub owner_id: i32,
    pub object_id: i32,
    pub description: String,
    pub sprite_index: u8,
    pub visitors: i8,
    pub visitor_capacity: i8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerchantVisitor {
    pub slot: i8,
    pub look: CharacterLook,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerchantChatMessage {
    pub text: String,
    pub speaker: i8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerchantStockItem {
    pub bundles: i16,
    pub per_bundle: i16,
    pub price: i32,
    pub item_id: i32,
    pub sold_out: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiredMerchantRoom {
    /// Mini-room type from the v83 `PLAYER_INTERACTION_RESULT` setup packet:
    /// 4 for a personal shop and 5 for a hired merchant.
    pub room_type: u8,
    pub my_slot: i16,
    pub item_id: i32,
    pub owner_name: String,
    pub owner_look: Option<CharacterLook>,
    pub description: String,
    pub first_time: bool,
    pub meso: i32,
    pub visitors: Vec<MerchantVisitor>,
    pub messages: Vec<MerchantChatMessage>,
    pub items: Vec<MerchantStockItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeParticipant {
    pub slot: u8,
    pub look: CharacterLook,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeRoom {
    pub self_slot: u8,
    pub partner: Option<TradeParticipant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeItemUpdate {
    pub player: u8,
    /// Zero-based local slot; the v83 wire slot is one-based.
    pub slot: u8,
    pub item_id: i32,
    pub quantity: i16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlayerInteractionRoomReply {
    Error(u8),
    HiredMerchant(HiredMerchantRoom),
    PersonalShop(HiredMerchantRoom),
    TradeInvite {
        inviter: String,
        room_id: i32,
    },
    TradeRoom(TradeRoom),
    TradePartnerJoined(TradeParticipant),
    TradeItem(TradeItemUpdate),
    TradeMeso {
        player: u8,
        meso: i32,
    },
    TradeConfirmed,
    TradeEnded {
        player: u8,
        result: u8,
    },
    VisitorJoined(MerchantVisitor),
    VisitorLeft(i8),
    Exited,
    Chat(MerchantChatMessage),
    ItemsUpdated {
        meso: Option<i32>,
        items: Vec<MerchantStockItem>,
    },
    ItemSold {
        slot: i8,
        quantity: i16,
        buyer: String,
    },
    OtherAction(i8),
    OtherRoom(u8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwlSearchEvent {
    Results {
        item_id: i32,
        shops: Vec<OwlShopResult>,
    },
    TopItems(Vec<i32>),
}

pub fn parse_hired_merchant_spawn(packet: &[u8]) -> Result<HiredMerchantSpawn, PacketError> {
    let mut packet = PacketReader::new(packet);
    expect_opcode(&mut packet, OPCODE_SPAWN_HIRED_MERCHANT)?;
    let owner_id = packet.read_i32()?;
    let item_id = packet.read_i32()?;
    let (x, y) = packet.read_point_i16()?;
    packet.read_i16()?;
    let owner_name = packet.read_string()?;
    packet.read_u8()?;
    let object_id = packet.read_i32()?;
    let description = packet.read_string()?;
    let skin = packet.read_i8()?;
    packet.read_u8()?;
    packet.read_u8()?;
    Ok(HiredMerchantSpawn {
        owner_id,
        object_id,
        item_id,
        x,
        y,
        owner_name,
        description,
        skin,
    })
}

pub fn parse_hired_merchant_remove(packet: &[u8]) -> Result<HiredMerchantRemove, PacketError> {
    let mut packet = PacketReader::new(packet);
    expect_opcode(&mut packet, OPCODE_DESTROY_HIRED_MERCHANT)?;
    Ok(HiredMerchantRemove {
        owner_id: packet.read_i32()?,
    })
}

pub fn parse_hired_merchant_update(packet: &[u8]) -> Result<HiredMerchantUpdate, PacketError> {
    let mut packet = PacketReader::new(packet);
    expect_opcode(&mut packet, OPCODE_UPDATE_HIRED_MERCHANT)?;
    let owner_id = packet.read_i32()?;
    let _room_type = packet.read_u8()?;
    let object_id = packet.read_i32()?;
    let description = packet.read_string()?;
    let sprite_index = packet.read_u8()?;
    let visitors = packet.read_i8()?;
    let visitor_capacity = packet.read_i8()?;
    Ok(HiredMerchantUpdate {
        owner_id,
        object_id,
        description,
        sprite_index,
        visitors,
        visitor_capacity,
    })
}

/// Enters a mini-room (including a personal shop or hired merchant) by object ID.
pub fn mini_room_visit_packet(object_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(4);
    packet.write_i32(object_id);
    packet.into_bytes()
}

pub fn trade_create_packet() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(0);
    packet.write_u8(3);
    packet.into_bytes()
}

pub fn trade_invite_packet(character_id: i32) -> Option<Vec<u8>> {
    if character_id <= 0 {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(2);
    packet.write_i32(character_id);
    Some(packet.into_bytes())
}

pub fn trade_decline_packet() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(3);
    packet.into_bytes()
}

pub fn trade_visit_packet(room_id: i32) -> Option<Vec<u8>> {
    (room_id > 0).then(|| mini_room_visit_packet(room_id))
}

pub fn trade_set_item_packet(
    inventory_type: InventoryType,
    inventory_slot: i16,
    quantity: i16,
    trade_slot: u8,
) -> Option<Vec<u8>> {
    if inventory_type == InventoryType::Equipped
        || inventory_slot <= 0
        || quantity <= 0
        || !(1..=MAX_TRADE_ITEMS as u8).contains(&trade_slot)
    {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(15);
    packet.write_u8(inventory_type as u8);
    packet.write_i16(inventory_slot);
    packet.write_i16(quantity);
    packet.write_u8(trade_slot);
    Some(packet.into_bytes())
}

pub fn trade_set_meso_packet(meso: i32) -> Option<Vec<u8>> {
    if meso < 0 {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(16);
    packet.write_i32(meso);
    Some(packet.into_bytes())
}

pub fn trade_confirm_packet() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(17);
    packet.into_bytes()
}

pub fn trade_exit_packet() -> Vec<u8> {
    merchant_exit_packet()
}

pub fn trade_chat_packet(message: &str) -> Option<Vec<u8>> {
    merchant_chat_packet(message)
}

/// Parses the hired-merchant room setup reply sent by `PLAYER_INTERACTION`.
/// Counts are capped before allocating so a malformed server frame cannot grow
/// the room UI without bound.
pub fn parse_hired_merchant_room_reply(
    bytes: &[u8],
) -> Result<PlayerInteractionRoomReply, PacketError> {
    parse_player_interaction_room_reply(bytes, Some(5))
}

/// Parses v83 personal-shop and hired-merchant room replies. `active_room_type`
/// is needed only for item-list refreshes, whose payload differs slightly
/// between the two room types and does not repeat the room type on the wire.
pub fn parse_player_interaction_room_reply(
    bytes: &[u8],
    active_room_type: Option<u8>,
) -> Result<PlayerInteractionRoomReply, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_PLAYER_INTERACTION_RESULT)?;
    let action = packet.read_i8()?;
    match action {
        2 => {
            let invite_type = packet.read_u8()?;
            let inviter = packet.read_string()?;
            let room_id = if packet.is_empty() {
                0
            } else {
                packet.read_i32()?
            };
            if invite_type == 3 {
                return Ok(PlayerInteractionRoomReply::TradeInvite { inviter, room_id });
            }
            return Ok(PlayerInteractionRoomReply::OtherAction(action));
        }
        4 => {
            if active_room_type == Some(3) {
                let slot = packet.read_u8()?;
                let look = read_character_look(&mut packet)?;
                let name = packet.read_string()?;
                return Ok(PlayerInteractionRoomReply::TradePartnerJoined(
                    TradeParticipant { slot, look, name },
                ));
            }
            let slot = packet.read_i8()?;
            let look = read_character_look(&mut packet)?;
            let name = packet.read_string()?;
            return Ok(PlayerInteractionRoomReply::VisitorJoined(MerchantVisitor {
                slot,
                look,
                name,
            }));
        }
        6 => {
            packet.read_u8()?;
            let speaker = packet.read_i8()?;
            let text = packet.read_string()?;
            return Ok(PlayerInteractionRoomReply::Chat(MerchantChatMessage {
                text,
                speaker,
            }));
        }
        10 => {
            if active_room_type == Some(3) {
                let player = packet.read_u8()?;
                let result = packet.read_u8()?;
                if player > 1 {
                    return Err(PacketError::InvalidTradePlayer(player));
                }
                return Ok(PlayerInteractionRoomReply::TradeEnded { player, result });
            }
            if packet.is_empty() {
                return Ok(PlayerInteractionRoomReply::Exited);
            }
            let slot = packet.read_i8()?;
            return if (1..=3).contains(&slot) {
                Ok(PlayerInteractionRoomReply::VisitorLeft(slot))
            } else {
                Ok(PlayerInteractionRoomReply::Exited)
            };
        }
        15 if active_room_type == Some(3) => {
            let player = packet.read_u8()?;
            let wire_slot = packet.read_u8()?;
            if player > 1 {
                return Err(PacketError::InvalidTradePlayer(player));
            }
            if !(1..=MAX_TRADE_ITEMS as u8).contains(&wire_slot) {
                return Err(PacketError::InvalidTradeSlot(wire_slot));
            }
            // TradeSetItemPacket sends slots 1..=9; the UI stores them as 0..=8.
            let slot = wire_slot - 1;
            let (item_id, quantity) = read_shop_item_data(&mut packet)?;
            return Ok(PlayerInteractionRoomReply::TradeItem(TradeItemUpdate {
                player,
                slot,
                item_id,
                quantity,
            }));
        }
        16 if active_room_type == Some(3) => {
            let player = packet.read_u8()?;
            if player > 1 {
                return Err(PacketError::InvalidTradePlayer(player));
            }
            let meso = packet.read_i32()?;
            if meso < 0 {
                return Err(PacketError::InvalidTradeMeso(meso));
            }
            return Ok(PlayerInteractionRoomReply::TradeMeso { player, meso });
        }
        17 if active_room_type == Some(3) => {
            return Ok(PlayerInteractionRoomReply::TradeConfirmed);
        }
        0x19 => {
            let room_type = active_room_type.unwrap_or(5);
            let meso = if room_type == 5 {
                Some(packet.read_i32()?)
            } else {
                None
            };
            let count = bounded_merchant_count(i16::from(packet.read_i8()?), MAX_MERCHANT_ITEMS)?;
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                let bundles = packet.read_i16()?;
                let per_bundle = packet.read_i16()?;
                let price = packet.read_i32()?;
                let item_id = read_shop_item_id(&mut packet)?;
                items.push(MerchantStockItem {
                    bundles,
                    per_bundle,
                    price,
                    item_id,
                    sold_out: bundles <= 0,
                });
            }
            return Ok(PlayerInteractionRoomReply::ItemsUpdated { meso, items });
        }
        0x1a => {
            let slot = packet.read_i8()?;
            let quantity = packet.read_i16()?;
            let buyer = packet.read_string()?;
            return Ok(PlayerInteractionRoomReply::ItemSold {
                slot,
                quantity,
                buyer,
            });
        }
        5 => {}
        _ => return Ok(PlayerInteractionRoomReply::OtherAction(action)),
    }
    let room_type = packet.read_u8()?;
    if room_type == 0 {
        return Ok(PlayerInteractionRoomReply::Error(packet.read_u8()?));
    }
    if room_type == 3 {
        packet.read_u8()?; // Trade room mode.
        let player_count = packet.read_u8()?;
        let partner = if player_count == 1 {
            let slot = packet.read_u8()?;
            if slot > 1 {
                return Err(PacketError::InvalidTradePlayer(slot));
            }
            let look = read_character_look(&mut packet)?;
            let name = packet.read_string()?;
            Some(TradeParticipant { slot, look, name })
        } else {
            None
        };
        let self_slot = packet.read_u8()?;
        if self_slot > 1 {
            return Err(PacketError::InvalidTradePlayer(self_slot));
        }
        read_character_look(&mut packet)?;
        packet.read_string()?;
        if !packet.is_empty() {
            packet.read_u8()?; // Optional 0xff end marker.
        }
        return Ok(PlayerInteractionRoomReply::TradeRoom(TradeRoom {
            self_slot,
            partner,
        }));
    }
    if room_type == 4 {
        packet.read_u8()?;
        let my_slot = i16::from(packet.read_i8()?);
        let sold_count =
            bounded_merchant_count(i16::from(packet.read_i8()?), MAX_MERCHANT_MESSAGES)?;
        for _ in 0..sold_count {
            packet.read_i32()?;
            packet.read_i16()?;
            packet.read_i32()?;
            packet.read_string()?;
        }
        let owner_look = Some(read_character_look(&mut packet)?);
        let owner_name = packet.read_string()?;
        let visitors = read_merchant_visitors(&mut packet)?;
        let description = packet.read_string()?;
        packet.read_u8()?;
        let item_count = bounded_merchant_count(i16::from(packet.read_i8()?), MAX_MERCHANT_ITEMS)?;
        let items = read_merchant_items(&mut packet, item_count)?;
        return Ok(PlayerInteractionRoomReply::PersonalShop(
            HiredMerchantRoom {
                room_type,
                my_slot,
                item_id: 0,
                owner_name,
                owner_look,
                description,
                first_time: false,
                meso: 0,
                visitors,
                messages: Vec::new(),
                items,
            },
        ));
    }
    if room_type != 5 {
        return Ok(PlayerInteractionRoomReply::OtherRoom(room_type));
    }

    packet.read_u8()?;
    let my_slot = packet.read_i16()?;
    let item_id = packet.read_i32()?;
    packet.read_string()?; // Server's optional room metadata.

    let visitors = read_merchant_visitors(&mut packet)?;

    let message_count = packet.read_i16()?;
    let message_count = bounded_merchant_count(message_count, MAX_MERCHANT_MESSAGES)?;
    let mut messages = Vec::with_capacity(message_count);
    for _ in 0..message_count {
        messages.push(MerchantChatMessage {
            text: packet.read_string()?,
            speaker: packet.read_i8()?,
        });
    }

    let owner_name = packet.read_string()?;
    let mut first_time = false;
    if my_slot == 0 {
        packet.read_i16()?;
        packet.read_i16()?;
        first_time = packet.read_u8()? != 0;
        let sold_count = bounded_merchant_count(i16::from(packet.read_i8()?), MAX_MERCHANT_ITEMS)?;
        for _ in 0..sold_count {
            packet.read_i32()?;
            packet.read_i16()?;
            packet.read_i32()?;
            packet.read_string()?;
        }
        packet.read_i32()?;
    }

    let description = packet.read_string()?;
    packet.read_u8()?;
    let meso = packet.read_i32()?;
    let item_count = bounded_merchant_count(i16::from(packet.read_i8()?), MAX_MERCHANT_ITEMS)?;
    if item_count == 0 {
        packet.read_u8()?;
    }
    let items = read_merchant_items(&mut packet, item_count)?;

    Ok(PlayerInteractionRoomReply::HiredMerchant(
        HiredMerchantRoom {
            room_type,
            my_slot,
            item_id,
            owner_name,
            owner_look: None,
            description,
            first_time,
            meso,
            visitors,
            messages,
            items,
        },
    ))
}

fn read_merchant_visitors(
    packet: &mut PacketReader<'_>,
) -> Result<Vec<MerchantVisitor>, PacketError> {
    let mut visitors = Vec::new();
    let mut terminated = false;
    while !packet.is_empty() {
        let slot = packet.read_i8()?;
        if slot == -1 || slot == 0x7f {
            terminated = true;
            break;
        }
        if visitors.len() >= MAX_MERCHANT_VISITORS {
            return Err(PacketError::TooManyMerchantEntries(MAX_MERCHANT_VISITORS));
        }
        visitors.push(MerchantVisitor {
            slot,
            look: read_character_look(packet)?,
            name: packet.read_string()?,
        });
    }
    if !terminated {
        return Err(PacketError::MissingMiniRoomTerminator);
    }
    Ok(visitors)
}

fn read_merchant_items(
    packet: &mut PacketReader<'_>,
    count: usize,
) -> Result<Vec<MerchantStockItem>, PacketError> {
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        let bundles = packet.read_i16()?;
        let per_bundle = packet.read_i16()?;
        let price = packet.read_i32()?;
        let item_id = read_shop_item_id(packet)?;
        items.push(MerchantStockItem {
            bundles,
            per_bundle,
            price,
            item_id,
            sold_out: bundles <= 0,
        });
    }
    Ok(items)
}

fn bounded_merchant_count(value: i16, maximum: usize) -> Result<usize, PacketError> {
    let count = usize::try_from(value).map_err(|_| PacketError::InvalidMerchantCount(value))?;
    if count > maximum {
        return Err(PacketError::TooManyMerchantEntries(maximum));
    }
    Ok(count)
}

fn read_shop_item_id(packet: &mut PacketReader<'_>) -> Result<i32, PacketError> {
    read_shop_item_data(packet).map(|(item_id, _)| item_id)
}

fn read_shop_item_data(packet: &mut PacketReader<'_>) -> Result<(i32, i16), PacketError> {
    let item_type = packet.read_i8()?;
    let item_id = packet.read_i32()?;
    let cash = packet.read_bool()?;
    if cash {
        packet.skip(8)?;
    }
    packet.read_i64()?; // Expiration.

    let quantity = if (5_000_000..=5_000_102).contains(&item_id) {
        packet.skip(13 + 1 + 2 + 1 + 18)?;
        1
    } else if item_type == 1 {
        packet.skip(2 + 30)?;
        packet.read_string()?;
        packet.read_i16()?;
        if cash {
            packet.skip(10)?;
        } else {
            packet.skip(1 + 1 + 2 + 2 + 4 + 8)?;
        }
        packet.skip(12)?;
        1
    } else {
        let count = packet.read_i16()?;
        if count < 0 {
            return Err(PacketError::InvalidTradeCount(count));
        }
        packet.read_string()?;
        packet.read_i16()?;
        if item_id / 10_000 == 233 || item_id / 10_000 == 207 {
            packet.skip(8)?;
        }
        count.max(1)
    };
    Ok((item_id, quantity))
}

pub fn merchant_buy_packet(slot: u8, quantity: i16) -> Option<Vec<u8>> {
    if usize::from(slot) >= MAX_MERCHANT_ITEMS || quantity <= 0 {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(0x17);
    packet.write_u8(slot);
    packet.write_i16(quantity);
    Some(packet.into_bytes())
}

pub fn merchant_add_item_packet(
    inventory_type: InventoryType,
    slot: i16,
    bundles: i16,
    per_bundle: i16,
    price: i32,
) -> Option<Vec<u8>> {
    if !(InventoryType::Equip as i8..=InventoryType::Cash as i8).contains(&(inventory_type as i8))
        || slot <= 0
        || bundles <= 0
        || per_bundle <= 0
        || price <= 0
        || price > 999_999_999
    {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(0x16);
    packet.write_u8(inventory_type as u8);
    packet.write_i16(slot);
    packet.write_i16(bundles);
    packet.write_i16(per_bundle);
    packet.write_i32(price);
    Some(packet.into_bytes())
}

pub fn merchant_chat_packet(message: &str) -> Option<Vec<u8>> {
    if message.trim().is_empty()
        || message.encode_utf16().count() > 60
        || message.chars().any(char::is_control)
    {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(6);
    packet.write_string(message).ok()?;
    Some(packet.into_bytes())
}

pub fn merchant_buy_open_packet() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(0x0b);
    packet.write_u8(1);
    packet.into_bytes()
}

pub fn merchant_exit_packet() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(10);
    packet.into_bytes()
}

pub fn merchant_take_item_packet(slot: u8) -> Option<Vec<u8>> {
    if usize::from(slot) >= MAX_MERCHANT_ITEMS {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(0x26);
    packet.write_i16(i16::from(slot));
    Some(packet.into_bytes())
}

/// Removes a personal-shop listing from the owner's current room.
pub fn personal_shop_remove_item_packet(slot: u8) -> Option<Vec<u8>> {
    if usize::from(slot) >= MAX_MERCHANT_ITEMS {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(0x1b);
    packet.write_i16(i16::from(slot));
    Some(packet.into_bytes())
}

pub fn merchant_maintenance_off_packet() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(0x27);
    packet.into_bytes()
}

pub fn merchant_organize_packet() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(0x28);
    packet.into_bytes()
}

pub fn merchant_take_meso_packet() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(28);
    packet.into_bytes()
}

pub fn merchant_close_packet() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(0x29);
    packet.into_bytes()
}

/// Opens a personal shop using a 514xxxx store permit.
///
/// The server packet is the legacy `PLAYER_INTERACTION` create-shop action:
/// action 0, personal-shop type 4, description, three zero option bytes, and
/// the permit item ID.
pub fn create_personal_shop_packet(description: &str, permit_item_id: i32) -> Option<Vec<u8>> {
    create_shop_packet(description, permit_item_id, 514, 4)
}

/// Opens a hired merchant using a 503xxxx permit after the server approves its check.
pub fn create_hired_merchant_shop_packet(
    description: &str,
    permit_item_id: i32,
) -> Option<Vec<u8>> {
    create_shop_packet(description, permit_item_id, 503, 5)
}

fn create_shop_packet(
    description: &str,
    permit_item_id: i32,
    permit_category: i32,
    room_type: u8,
) -> Option<Vec<u8>> {
    if permit_item_id / 10_000 != permit_category
        || description.trim().is_empty()
        || description.encode_utf16().count() > 30
        || description.chars().any(char::is_control)
    {
        return None;
    }

    let mut packet = PacketWriter::new(OPCODE_PLAYER_INTERACTION);
    packet.write_u8(0);
    packet.write_u8(room_type);
    packet.write_string(description).ok()?;
    packet.write_u8(0);
    packet.write_u8(0);
    packet.write_u8(0);
    packet.write_i32(permit_item_id);
    Some(packet.into_bytes())
}

/// Requests validation of a hired-merchant permit's deployment location.
pub fn hired_merchant_request_packet() -> Vec<u8> {
    PacketWriter::new(OPCODE_HIRED_MERCHANT_REQUEST).into_bytes()
}

/// Parses the server's hired-merchant permit check mode.
pub fn parse_hired_merchant_check_result(packet: &[u8]) -> Result<i8, PacketError> {
    if packet.len() < 2 {
        return Err(PacketError::Truncated {
            position: 0,
            requested: 2,
            remaining: packet.len(),
        });
    }
    let actual = u16::from_le_bytes([packet[0], packet[1]]);
    if actual != OPCODE_ENTRUSTED_SHOP_CHECK_RESULT {
        return Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_ENTRUSTED_SHOP_CHECK_RESULT,
            actual,
        });
    }
    PacketReader::new(&packet[2..]).read_i8()
}

/// Requests the server's most searched items for the Owl of Minerva panel.
pub fn owl_top_items_packet() -> Vec<u8> {
    PacketWriter::new(OPCODE_OWL_ACTION).into_bytes()
}

/// Searches shop listings for one item using an Owl of Minerva.
pub fn owl_search_packet(slot: i16, owl_item_id: i32, search_item_id: i32) -> Option<Vec<u8>> {
    if slot <= 0 || owl_item_id / 10_000 != 523 || search_item_id <= 0 {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_USE_CASH_ITEM);
    packet.write_i16(slot);
    packet.write_i32(owl_item_id);
    packet.write_i32(search_item_id);
    Some(packet.into_bytes())
}

/// Warps to a shop selected from an Owl search result.
pub fn owl_warp_packet(owner_id: i32, map_id: i32) -> Option<Vec<u8>> {
    if owner_id <= 0 || map_id < 0 {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_OWL_WARP);
    packet.write_i32(owner_id);
    packet.write_i32(map_id);
    Some(packet.into_bytes())
}

/// Parses server top-item or shop-search results for the Owl of Minerva.
pub fn parse_owl_search_event(packet: &[u8]) -> Result<Option<OwlSearchEvent>, PacketError> {
    if packet.len() < 2 {
        return Err(PacketError::Truncated {
            position: 0,
            requested: 2,
            remaining: packet.len(),
        });
    }
    let actual = u16::from_le_bytes([packet[0], packet[1]]);
    if actual != OPCODE_SHOP_SCANNER_RESULT {
        return Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_SHOP_SCANNER_RESULT,
            actual,
        });
    }
    let mut reader = PacketReader::new(&packet[2..]);
    match reader.read_u8()? {
        6 => {
            reader.read_i32()?; // Server search serial.
            let item_id = reader.read_i32()?;
            let count = reader.read_i32()?;
            if count < 0 {
                return Err(PacketError::InvalidOwlResultCount(count));
            }
            let count = count as usize;
            if count > MAX_OWL_RESULTS {
                return Err(PacketError::TooManyOwlResults(count));
            }
            let mut shops = Vec::with_capacity(count);
            for _ in 0..count {
                shops.push(OwlShopResult {
                    owner: reader.read_string()?,
                    map_id: reader.read_i32()?,
                    description: reader.read_string()?,
                    bundles: reader.read_i32()?,
                    per_bundle: reader.read_i32()?,
                    price: reader.read_i32()?,
                    owner_id: reader.read_i32()?,
                    channel: reader.read_i8()?,
                });
            }
            Ok(Some(OwlSearchEvent::Results { item_id, shops }))
        }
        7 => {
            let count = reader.read_i8()?;
            if count < 0 {
                return Err(PacketError::InvalidOwlTopItemCount(count));
            }
            let count = count as usize;
            if count > MAX_OWL_TOP_ITEMS {
                return Err(PacketError::TooManyOwlTopItems(count));
            }
            let mut item_ids = Vec::with_capacity(count);
            for _ in 0..count {
                let item_id = reader.read_i32()?;
                if item_id <= 0 {
                    return Err(PacketError::InvalidOwlItemId(item_id));
                }
                item_ids.push(item_id);
            }
            Ok(Some(OwlSearchEvent::TopItems(item_ids)))
        }
        _ => Ok(None),
    }
}

/// Parses a shop-link error response. Code zero means no error notification.
pub fn parse_owl_link_result(packet: &[u8]) -> Result<Option<i8>, PacketError> {
    if packet.len() < 2 {
        return Err(PacketError::Truncated {
            position: 0,
            requested: 2,
            remaining: packet.len(),
        });
    }
    let actual = u16::from_le_bytes([packet[0], packet[1]]);
    if actual != OPCODE_SHOP_LINK_RESULT {
        return Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_SHOP_LINK_RESULT,
            actual,
        });
    }
    let mut reader = PacketReader::new(&packet[2..]);
    match reader.read_i8()? {
        0 => Ok(None),
        message => Ok(Some(message)),
    }
}
