//! Bounded v83 character inventory snapshots and modification packets.

use crate::{PacketError, PacketReader, PacketWriter};

const MAX_INVENTORY_ITEMS: usize = 1024;
const MAX_ITEMS_PER_LIST: usize = 256;
const MAX_STORAGE_ITEMS: usize = 256;
const EQUIP_STAT_COUNT: usize = 15;
const MAX_MODIFICATIONS: usize = 128;

pub const OPCODE_MODIFY_INVENTORY: u16 = 29;
pub const OPCODE_STORAGE: u16 = 309;
pub const OPCODE_MOVE_ITEM: u16 = 71;
pub const OPCODE_USE_ITEM: u16 = 72;
pub const OPCODE_USE_CASH_ITEM: u16 = 79;
pub const OPCODE_CANCEL_CHAIR: u16 = 42;
pub const OPCODE_USE_CHAIR: u16 = 43;
pub const OPCODE_GATHER_ITEMS: u16 = 69;
pub const OPCODE_SORT_ITEMS: u16 = 70;
pub const OPCODE_GATHER_RESULT: u16 = 52;
pub const OPCODE_SORT_RESULT: u16 = 53;
pub const OPCODE_SCROLL_EQUIP: u16 = 86;
pub const OPCODE_PET_FOOD: u16 = 76;
pub const OPCODE_SPAWN_PET: u16 = 98;
pub const OPCODE_STORAGE_ACTION: u16 = 62;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum InventoryType {
    #[default]
    Equip = 1,
    Use = 2,
    Setup = 3,
    Etc = 4,
    Cash = 5,
    Equipped = 6,
}

impl InventoryType {
    fn from_wire(value: i8) -> Result<Self, PacketError> {
        match value {
            -1 => Ok(Self::Equipped),
            1 => Ok(Self::Equip),
            2 => Ok(Self::Use),
            3 => Ok(Self::Setup),
            4 => Ok(Self::Etc),
            5 => Ok(Self::Cash),
            _ => Err(PacketError::InvalidInventoryType(value)),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PetInventoryData {
    pub name: String,
    pub level: u8,
    pub closeness: i16,
    pub fullness: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EquipInventoryData {
    pub upgrade_slots: u8,
    pub level: u8,
    /// STR, DEX, INT, LUK, HP, MP, weapon attack, magic, weapon defense,
    /// magic defense, accuracy, avoid, hands, speed, and jump.
    pub stats: [u16; EQUIP_STAT_COUNT],
    pub item_level: u8,
    pub item_exp: u16,
    pub vicious: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InventoryItem {
    pub inventory_type: InventoryType,
    pub slot: u16,
    pub item_id: i32,
    pub count: u16,
    pub cash: bool,
    pub cash_id: Option<i64>,
    pub expiration: i64,
    pub owner: String,
    pub flags: i16,
    pub equip: Option<EquipInventoryData>,
    pub pet: Option<PetInventoryData>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InventorySnapshot {
    pub meso: i64,
    /// Equip, use, setup, etc, and cash slot capacities.
    pub slot_maxima: [u8; 5],
    pub items: Vec<InventoryItem>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StorageSnapshot {
    pub npc_id: i32,
    pub slot_capacity: u8,
    pub meso: i32,
    pub items: Vec<InventoryItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageEvent {
    Open(StorageSnapshot),
    ItemsUpdated {
        inventory_type: InventoryType,
        slot_capacity: u8,
        items: Vec<InventoryItem>,
    },
    Arranged {
        slot_capacity: u8,
        items: Vec<InventoryItem>,
    },
    MesoUpdated {
        slot_capacity: u8,
        meso: i32,
    },
    Error(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryMovement {
    Internal,
    Unequip,
    Equip,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InventoryModification {
    Add(InventoryItem),
    ChangeCount {
        inventory_type: InventoryType,
        position: i16,
        count: u16,
    },
    Swap {
        inventory_type: InventoryType,
        position: i16,
        other_position: i16,
    },
    Remove {
        inventory_type: InventoryType,
        position: i16,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventoryUpdate {
    pub update_tick: bool,
    pub movement: InventoryMovement,
    pub modifications: Vec<InventoryModification>,
}

impl InventorySnapshot {
    pub fn item_count(&self, item_id: i32) -> u32 {
        self.items
            .iter()
            .filter(|item| item.item_id == item_id)
            .map(|item| u32::from(item.count))
            .sum()
    }

    pub fn equipped(&self) -> impl Iterator<Item = &InventoryItem> {
        self.items
            .iter()
            .filter(|item| item.inventory_type == InventoryType::Equipped)
    }

    pub fn apply(&mut self, update: InventoryUpdate) {
        let mut swaps = Vec::new();
        for modification in update.modifications {
            match modification {
                InventoryModification::Add(item) => self.upsert(item),
                InventoryModification::ChangeCount {
                    inventory_type,
                    position,
                    count,
                } => {
                    let (inventory_type, slot) = normalize_slot(inventory_type, position);
                    if let Some(item) = self
                        .items
                        .iter_mut()
                        .find(|item| item.inventory_type == inventory_type && item.slot == slot)
                    {
                        item.count = count;
                    }
                }
                InventoryModification::Remove {
                    inventory_type,
                    position,
                } => {
                    let (inventory_type, slot) = normalize_slot(inventory_type, position);
                    self.items
                        .retain(|item| item.inventory_type != inventory_type || item.slot != slot);
                }
                InventoryModification::Swap {
                    inventory_type,
                    position,
                    other_position,
                } => {
                    swaps.push((inventory_type, position, other_position));
                }
            }
        }
        for (inventory_type, position, other_position) in swaps {
            let (from_type, from_slot, to_type, to_slot) = match update.movement {
                InventoryMovement::Internal => {
                    let (kind, slot) = normalize_slot(inventory_type, position);
                    (kind, slot, kind, other_position.unsigned_abs())
                }
                InventoryMovement::Unequip => (
                    InventoryType::Equipped,
                    position.unsigned_abs(),
                    InventoryType::Equip,
                    other_position.unsigned_abs(),
                ),
                InventoryMovement::Equip => (
                    InventoryType::Equip,
                    position.unsigned_abs(),
                    InventoryType::Equipped,
                    other_position.unsigned_abs(),
                ),
                InventoryMovement::None => continue,
            };
            let source = self
                .items
                .iter()
                .position(|item| item.inventory_type == from_type && item.slot == from_slot);
            let target = self
                .items
                .iter()
                .position(|item| item.inventory_type == to_type && item.slot == to_slot);
            match (source, target) {
                (Some(source), Some(target)) if source != target => {
                    self.items[source].inventory_type = to_type;
                    self.items[source].slot = to_slot;
                    self.items[target].inventory_type = from_type;
                    self.items[target].slot = from_slot;
                }
                (Some(source), None) => {
                    self.items[source].inventory_type = to_type;
                    self.items[source].slot = to_slot;
                }
                (None, Some(target)) => {
                    self.items[target].inventory_type = from_type;
                    self.items[target].slot = from_slot;
                }
                _ => {}
            }
        }
    }

    fn upsert(&mut self, item: InventoryItem) {
        self.items.retain(|existing| {
            existing.inventory_type != item.inventory_type || existing.slot != item.slot
        });
        if self.items.len() < MAX_INVENTORY_ITEMS {
            self.items.push(item);
        }
    }
}

/// Reads the inventory prefix from the initial character-entry `SET_FIELD`
/// packet. The remaining skill, quest, and social sections are not consumed.
pub(crate) fn parse_initial_inventory(
    packet: &mut PacketReader<'_>,
) -> Result<InventorySnapshot, PacketError> {
    let meso = i64::from(packet.read_i32()?);
    let mut slot_maxima = [0; 5];
    for maximum in &mut slot_maxima {
        *maximum = packet.read_u8()?;
    }
    packet.skip(8)?;

    let mut snapshot = InventorySnapshot {
        meso,
        slot_maxima,
        items: Vec::new(),
    };

    for list_index in 0..3 {
        let mut position = packet.read_i16()?;
        let mut count = 0;
        while position != 0 {
            count += 1;
            if count > MAX_ITEMS_PER_LIST {
                return Err(PacketError::TooManyInventoryItems(MAX_ITEMS_PER_LIST));
            }
            let inventory_type = if list_index == 0 {
                InventoryType::Equipped
            } else {
                InventoryType::Equip
            };
            let (normalized_type, slot) = if list_index == 1 {
                (InventoryType::Equip, position.unsigned_abs())
            } else {
                normalize_slot(inventory_type, position)
            };
            let item = read_inventory_item(packet, normalized_type, slot)?;
            snapshot.upsert(item);
            position = packet.read_i16()?;
        }
    }

    packet.skip(2)?;
    for inventory_type in [
        InventoryType::Use,
        InventoryType::Setup,
        InventoryType::Etc,
        InventoryType::Cash,
    ] {
        let mut position = packet.read_u8()?;
        let mut count = 0;
        while position != 0 {
            count += 1;
            if count > MAX_ITEMS_PER_LIST {
                return Err(PacketError::TooManyInventoryItems(MAX_ITEMS_PER_LIST));
            }
            snapshot.upsert(read_inventory_item(
                packet,
                inventory_type,
                u16::from(position),
            )?);
            position = packet.read_u8()?;
        }
    }
    Ok(snapshot)
}

/// Parses an inventory delta packet without mutating client state.
pub fn parse_inventory_update(body: &[u8]) -> Result<InventoryUpdate, PacketError> {
    let mut packet = PacketReader::new(body);
    let opcode = packet.read_u16()?;
    if opcode != OPCODE_MODIFY_INVENTORY {
        return Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_MODIFY_INVENTORY,
            actual: opcode,
        });
    }
    let update_tick = packet.read_bool()?;
    let count = usize::from(packet.read_u8()?);
    if count > MAX_MODIFICATIONS {
        return Err(PacketError::TooManyInventoryModifications(
            MAX_MODIFICATIONS,
        ));
    }
    let mut modifications = Vec::with_capacity(count);
    for _ in 0..count {
        let mode = packet.read_u8()?;
        let inventory_type = InventoryType::from_wire(packet.read_i8()?)?;
        let position = packet.read_i16()?;
        modifications.push(match mode {
            0 => {
                let (item_type, slot) = normalize_slot(inventory_type, position);
                InventoryModification::Add(read_inventory_item(&mut packet, item_type, slot)?)
            }
            1 => InventoryModification::ChangeCount {
                inventory_type,
                position,
                count: read_nonnegative_count(&mut packet)?,
            },
            2 => InventoryModification::Swap {
                inventory_type,
                position,
                other_position: packet.read_i16()?,
            },
            3 => InventoryModification::Remove {
                inventory_type,
                position,
            },
            _ => return Err(PacketError::InvalidInventoryModification(mode)),
        });
    }
    let movement = if packet.is_empty() {
        InventoryMovement::Internal
    } else {
        match packet.read_i8()? {
            0 => InventoryMovement::Internal,
            1 => InventoryMovement::Unequip,
            2 => InventoryMovement::Equip,
            _ => InventoryMovement::None,
        }
    };
    Ok(InventoryUpdate {
        update_tick,
        movement,
        modifications,
    })
}

/// Parses the server's v83 storage response. Storage list positions are local
/// to each inventory type and start at zero; player inventory positions start
/// at one in the store request.
pub fn parse_storage_response(body: &[u8]) -> Result<StorageEvent, PacketError> {
    let mut packet = PacketReader::new(body);
    let opcode = packet.read_u16()?;
    if opcode != OPCODE_STORAGE {
        return Err(PacketError::UnexpectedOpcode {
            expected: OPCODE_STORAGE,
            actual: opcode,
        });
    }
    let mode = packet.read_u8()?;
    match mode {
        0x16 => {
            let npc_id = packet.read_i32()?;
            let slot_capacity = packet.read_u8()?;
            packet.skip(2 + 2 + 4)?;
            let meso = packet.read_i32()?;
            packet.skip(2)?;
            let count = read_storage_item_count(&mut packet)?;
            let items = read_storage_items(&mut packet, count, None)?;
            Ok(StorageEvent::Open(StorageSnapshot {
                npc_id,
                slot_capacity,
                meso,
                items,
            }))
        }
        0x09 | 0x0d => {
            let slot_capacity = packet.read_u8()?;
            let inventory_type = storage_type_from_bitfield(packet.read_i16()?)?;
            packet.skip(2 + 4)?;
            let count = read_storage_item_count(&mut packet)?;
            let items = read_storage_items(&mut packet, count, Some(inventory_type))?;
            Ok(StorageEvent::ItemsUpdated {
                inventory_type,
                slot_capacity,
                items,
            })
        }
        0x0f => {
            let slot_capacity = packet.read_u8()?;
            packet.skip(1 + 10)?;
            let count = read_storage_item_count(&mut packet)?;
            let items = read_storage_items(&mut packet, count, None)?;
            Ok(StorageEvent::Arranged {
                slot_capacity,
                items,
            })
        }
        0x13 => {
            let slot_capacity = packet.read_u8()?;
            packet.skip(2 + 2 + 4)?;
            let meso = packet.read_i32()?;
            Ok(StorageEvent::MesoUpdated {
                slot_capacity,
                meso,
            })
        }
        error_code => Ok(StorageEvent::Error(error_code)),
    }
}

pub fn storage_take_out_packet(inventory_type: InventoryType, slot: u8) -> Option<Vec<u8>> {
    if !matches!(
        inventory_type,
        InventoryType::Equip
            | InventoryType::Use
            | InventoryType::Setup
            | InventoryType::Etc
            | InventoryType::Cash
    ) {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_STORAGE_ACTION);
    packet.write_u8(4);
    packet.write_u8(inventory_type as u8);
    packet.write_u8(slot);
    Some(packet.into_bytes())
}

pub fn storage_store_packet(slot: u16, item_id: i32, quantity: u16) -> Option<Vec<u8>> {
    let slot = i16::try_from(slot).ok().filter(|slot| *slot > 0)?;
    let quantity = i16::try_from(quantity)
        .ok()
        .filter(|quantity| *quantity > 0)?;
    if item_id <= 0 {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_STORAGE_ACTION);
    packet.write_u8(5);
    packet.write_i16(slot);
    packet.write_i32(item_id);
    packet.write_i16(quantity);
    Some(packet.into_bytes())
}

pub fn storage_meso_packet(amount: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_STORAGE_ACTION);
    packet.write_u8(7);
    packet.write_i32(amount);
    packet.into_bytes()
}

pub fn storage_close_packet() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_STORAGE_ACTION);
    packet.write_u8(8);
    packet.into_bytes()
}

fn read_storage_item_count(packet: &mut PacketReader<'_>) -> Result<usize, PacketError> {
    let count = usize::from(packet.read_u8()?);
    if count > MAX_STORAGE_ITEMS {
        return Err(PacketError::TooManyStorageItems(MAX_STORAGE_ITEMS));
    }
    Ok(count)
}

fn read_storage_items(
    packet: &mut PacketReader<'_>,
    count: usize,
    forced_type: Option<InventoryType>,
) -> Result<Vec<InventoryItem>, PacketError> {
    let mut items = Vec::with_capacity(count);
    let mut next_slot = [0_u16; 5];
    for _ in 0..count {
        let mut peek = packet.clone();
        peek.read_u8()?;
        let item_id = peek.read_i32()?;
        let inventory_type = match forced_type {
            Some(inventory_type) => inventory_type,
            None => storage_type_for_item_id(item_id)
                .ok_or(PacketError::InvalidStorageItemId(item_id))?,
        };
        let index = storage_type_index(inventory_type).ok_or(
            PacketError::InvalidStorageInventoryType(inventory_type as i16),
        )?;
        let slot = next_slot[index];
        next_slot[index] = slot
            .checked_add(1)
            .ok_or(PacketError::TooManyStorageItems(MAX_STORAGE_ITEMS))?;
        items.push(read_inventory_item(packet, inventory_type, slot)?);
    }
    Ok(items)
}

fn storage_type_from_bitfield(bitfield: i16) -> Result<InventoryType, PacketError> {
    match bitfield {
        4 => Ok(InventoryType::Equip),
        8 => Ok(InventoryType::Use),
        16 => Ok(InventoryType::Setup),
        32 => Ok(InventoryType::Etc),
        64 => Ok(InventoryType::Cash),
        _ => Err(PacketError::InvalidStorageInventoryType(bitfield)),
    }
}

fn storage_type_for_item_id(item_id: i32) -> Option<InventoryType> {
    match item_id / 1_000_000 {
        1 => Some(InventoryType::Equip),
        2 => Some(InventoryType::Use),
        3 => Some(InventoryType::Setup),
        4 => Some(InventoryType::Etc),
        5 => Some(InventoryType::Cash),
        _ => None,
    }
}

fn storage_type_index(inventory_type: InventoryType) -> Option<usize> {
    match inventory_type {
        InventoryType::Equip => Some(0),
        InventoryType::Use => Some(1),
        InventoryType::Setup => Some(2),
        InventoryType::Etc => Some(3),
        InventoryType::Cash => Some(4),
        InventoryType::Equipped => None,
    }
}

/// Parses the item record attached to a successful Cash Shop locker withdrawal.
pub fn parse_cash_shop_withdrawal_item(
    body: &[u8],
    item_id: i32,
    slot: i16,
) -> Result<InventoryItem, PacketError> {
    let slot = u16::try_from(slot)
        .ok()
        .filter(|slot| *slot > 0)
        .ok_or(PacketError::InvalidCashShopInventorySlot(slot))?;
    let inventory_type = match item_id / 1_000_000 {
        1 => InventoryType::Equip,
        2 => InventoryType::Use,
        3 => InventoryType::Setup,
        4 => InventoryType::Etc,
        5 => InventoryType::Cash,
        value => return Err(PacketError::InvalidInventoryType(value as i8)),
    };
    let mut packet = PacketReader::new(body);
    let item = read_inventory_item(&mut packet, inventory_type, slot)?;
    if item.item_id != item_id {
        return Err(PacketError::CashShopItemMismatch {
            expected: item_id,
            actual: item.item_id,
        });
    }
    Ok(item)
}

/// Builds the v83 `USE_ITEM` request. The opcode is also used by the legacy
/// client for eligible cash-inventory items such as pet equipment.
pub fn use_item_packet(slot: i16, item_id: i32, client_tick: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_USE_ITEM);
    packet.write_i32(client_tick);
    packet.write_i16(slot);
    packet.write_i32(item_id);
    packet.into_bytes()
}

/// Builds the v83 cash-megaphone request for regular, super, item, and triple
/// megaphones. Item megaphones may reference an attached inventory item.
pub fn use_megaphone_packet(
    slot: i16,
    item_id: i32,
    lines: &[String],
    whisper: bool,
    attached_item: Option<(InventoryType, u16)>,
) -> Option<Vec<u8>> {
    let subtype = (item_id / 1_000) % 10;
    if slot <= 0
        || item_id / 10_000 != 507
        || lines.is_empty()
        || lines.len() > 3
        || (subtype != 6 && attached_item.is_some())
        || attached_item.is_some_and(|(inventory_type, source_slot)| {
            inventory_type == InventoryType::Equipped || source_slot == 0
        })
    {
        return None;
    }
    if lines.iter().any(|line| {
        line.is_empty() || line.chars().count() > 60 || line.chars().any(char::is_control)
    }) {
        return None;
    }

    let mut packet = PacketWriter::new(OPCODE_USE_CASH_ITEM);
    packet.write_i16(slot);
    packet.write_i32(item_id);
    packet.write_string(&lines[0]).ok()?;
    match subtype {
        0 | 1 if lines.len() == 1 => {}
        2 if lines.len() == 1 => packet.write_bool(whisper),
        6 if lines.len() == 1 => {
            packet.write_bool(whisper);
            packet.write_bool(attached_item.is_some());
            if let Some((inventory_type, source_slot)) = attached_item {
                // Cosmic's USE_CASH_ITEM handler reads both source fields as ints.
                packet.write_i32(i32::from(inventory_type as u8));
                packet.write_i32(i32::from(source_slot));
            }
        }
        7 => {
            packet.write_u8(lines.len() as u8);
            for line in &lines[1..] {
                packet.write_string(line).ok()?;
            }
            packet.write_bool(whisper);
        }
        _ => return None,
    }
    Some(packet.into_bytes())
}

/// Builds a v83 avatar-megaphone request with up to four message rows. Missing
/// rows are padded with empty strings, matching the legacy client payload.
pub fn use_avatar_megaphone_packet(
    slot: i16,
    item_id: i32,
    lines: &[String],
    whisper: bool,
) -> Option<Vec<u8>> {
    if slot <= 0
        || item_id / 10_000 != 539
        || lines.is_empty()
        || lines.len() > 4
        || lines
            .iter()
            .any(|line| line.chars().count() > 60 || line.chars().any(char::is_control))
        || lines.iter().all(String::is_empty)
    {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_USE_CASH_ITEM);
    packet.write_i16(slot);
    packet.write_i32(item_id);
    for index in 0..4 {
        packet
            .write_string(lines.get(index).map_or("", String::as_str))
            .ok()?;
    }
    packet.write_bool(whisper);
    Some(packet.into_bytes())
}

/// Builds the v83 Maple TV request with an optional named recipient and up to
/// five message rows. Missing rows are padded with empty strings.
pub fn use_maple_tv_packet(
    slot: i16,
    item_id: i32,
    recipient: &str,
    lines: &[String],
    megassenger: bool,
) -> Option<Vec<u8>> {
    if slot <= 0
        || item_id / 10_000 != 507
        || (item_id / 1_000) % 10 != 5
        || recipient.chars().count() > 12
        || recipient.chars().any(char::is_control)
        || lines.is_empty()
        || lines.len() > 5
        || lines
            .iter()
            .any(|line| line.chars().count() > 60 || line.chars().any(char::is_control))
        || lines.iter().all(String::is_empty)
    {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_USE_CASH_ITEM);
    packet.write_i16(slot);
    packet.write_i32(item_id);
    packet.write_string(recipient).ok()?;
    for index in 0..5 {
        packet
            .write_string(lines.get(index).map_or("", String::as_str))
            .ok()?;
    }
    packet.write_bool(megassenger);
    Some(packet.into_bytes())
}

/// Builds the v83 pet-name-tag request.
pub fn use_pet_name_tag_packet(slot: i16, item_id: i32, name: &str) -> Option<Vec<u8>> {
    if slot <= 0
        || item_id / 10_000 != 517
        || name.trim().is_empty()
        || name.chars().count() > 12
        || name.chars().any(char::is_control)
    {
        return None;
    }
    let mut packet = PacketWriter::new(OPCODE_USE_CASH_ITEM);
    packet.write_i16(slot);
    packet.write_i32(item_id);
    packet.write_string(name).ok()?;
    Some(packet.into_bytes())
}

/// Requests use of a portable chair from the Setup inventory.
pub fn use_chair_packet(item_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_USE_CHAIR);
    packet.write_i32(item_id);
    packet.into_bytes()
}

/// Requests standing up from a portable chair.
pub fn cancel_chair_packet() -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_CANCEL_CHAIR);
    packet.write_i16(-1);
    packet.into_bytes()
}

/// Feeds the hungriest summoned pet using the v83 `PET_FOOD` request.
pub fn pet_food_packet(slot: i16, item_id: i32, client_tick: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PET_FOOD);
    packet.write_i32(client_tick);
    packet.write_i16(slot);
    packet.write_i32(item_id);
    packet.into_bytes()
}

/// Toggles the pet stored in a Cash inventory slot using the legacy v83
/// `SPAWN_PET` request. The legacy client sends a zero tick and clears the lead flag.
pub fn spawn_pet_packet(slot: i16) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_SPAWN_PET);
    packet.write_i32(0);
    packet.write_i16(slot);
    packet.write_u8(0);
    packet.into_bytes()
}

/// Builds the v83 `SCROLL_EQUIP` request for applying a Use inventory scroll to gear.
/// The target slot is the positive equipment slot number; the wire format encodes it
/// as a negative destination and appends the legacy scroll flags field.
pub fn scroll_equip_packet(
    source_slot: i16,
    target_slot: i16,
    client_tick: i32,
) -> Option<Vec<u8>> {
    if source_slot <= 0 {
        return None;
    }
    let destination = target_slot.checked_neg().filter(|_| target_slot > 0)?;
    let mut packet = PacketWriter::new(OPCODE_SCROLL_EQUIP);
    packet.write_i32(client_tick);
    packet.write_i16(source_slot);
    packet.write_i16(destination);
    packet.write_i16(0);
    Some(packet.into_bytes())
}

/// Builds the v83 request to move, equip, or unequip an inventory item.
pub fn move_item_packet(
    inventory_type: InventoryType,
    source: i16,
    destination: i16,
    quantity: i16,
    client_tick: i32,
) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_MOVE_ITEM);
    packet.write_i32(client_tick);
    packet.write_u8(inventory_type as u8);
    packet.write_i16(source);
    packet.write_i16(destination);
    packet.write_i16(quantity);
    packet.into_bytes()
}

/// Requests the server's legacy gather action, which merges compatible stacks
/// and closes gaps in a supported inventory bag.
pub fn gather_items_packet(inventory_type: InventoryType, client_tick: i32) -> Option<Vec<u8>> {
    inventory_arrange_packet(OPCODE_GATHER_ITEMS, inventory_type, client_tick)
}

/// Requests the server's legacy item ordering for a supported inventory bag.
pub fn sort_items_packet(inventory_type: InventoryType, client_tick: i32) -> Option<Vec<u8>> {
    inventory_arrange_packet(OPCODE_SORT_ITEMS, inventory_type, client_tick)
}

fn inventory_arrange_packet(
    opcode: u16,
    inventory_type: InventoryType,
    client_tick: i32,
) -> Option<Vec<u8>> {
    if inventory_type == InventoryType::Equipped {
        return None;
    }
    let mut packet = PacketWriter::new(opcode);
    packet.write_i32(client_tick);
    packet.write_u8(inventory_type as u8);
    Some(packet.into_bytes())
}

fn read_inventory_item(
    packet: &mut PacketReader<'_>,
    inventory_type: InventoryType,
    slot: u16,
) -> Result<InventoryItem, PacketError> {
    packet.read_u8()?; // item type discriminator, redundant with the inventory section
    let item_id = packet.read_i32()?;
    let cash = packet.read_bool()?;
    let cash_id = if cash { Some(packet.read_i64()?) } else { None };
    let expiration = packet.read_i64()?;
    let mut item = InventoryItem {
        inventory_type,
        slot,
        item_id,
        count: 1,
        cash,
        cash_id,
        expiration,
        ..InventoryItem::default()
    };

    if matches!(
        inventory_type,
        InventoryType::Equip | InventoryType::Equipped
    ) {
        let upgrade_slots = packet.read_u8()?;
        let level = packet.read_u8()?;
        let mut stats = [0; EQUIP_STAT_COUNT];
        for stat in &mut stats {
            *stat = packet.read_u16()?;
        }
        item.owner = packet.read_string()?;
        item.flags = packet.read_i16()?;
        let (item_level, item_exp, vicious) = if cash {
            packet.skip(10)?;
            (0, 0, 0)
        } else {
            packet.read_u8()?;
            let item_level = packet.read_u8()?;
            packet.skip(2)?;
            let item_exp = packet.read_u16()?;
            let vicious = packet.read_i32()?;
            packet.read_i64()?;
            (item_level, item_exp, vicious)
        };
        packet.skip(12)?;
        item.equip = Some(EquipInventoryData {
            upgrade_slots,
            level,
            stats,
            item_level,
            item_exp,
            vicious,
        });
    } else if (5_000_000..=5_000_102).contains(&item_id) {
        let name = packet.read_string_bytes(13)?;
        let level = packet.read_u8()?;
        let closeness = packet.read_i16()?;
        let fullness = packet.read_u8()?;
        packet.skip(18)?;
        item.pet = Some(PetInventoryData {
            name,
            level,
            closeness,
            fullness,
        });
    } else {
        item.count = read_nonnegative_count(packet)?;
        item.owner = packet.read_string()?;
        item.flags = packet.read_i16()?;
        if item_id / 10_000 == 233 || item_id / 10_000 == 207 {
            packet.skip(8)?;
        }
    }
    Ok(item)
}

fn read_nonnegative_count(packet: &mut PacketReader<'_>) -> Result<u16, PacketError> {
    let count = packet.read_i16()?;
    u16::try_from(count).map_err(|_| PacketError::InvalidInventoryCount(count))
}

fn normalize_slot(inventory_type: InventoryType, position: i16) -> (InventoryType, u16) {
    if position < 0 {
        (InventoryType::Equipped, position.unsigned_abs())
    } else {
        (inventory_type, position as u16)
    }
}
