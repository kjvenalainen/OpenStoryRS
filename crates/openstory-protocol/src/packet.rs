use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PacketError {
    Truncated {
        position: usize,
        requested: usize,
        remaining: usize,
    },
    StringTooLong(usize),
    InvalidFrameLength(usize),
    InvalidHandshakeLength(usize),
    InvalidEncryptedHeader {
        expected: u16,
        actual: u16,
    },
    UnexpectedOpcode {
        expected: u16,
        actual: u16,
    },
    MissingWorldTerminator,
    InvalidCharacterCount(u8),
    TooManyRecommendedWorlds(u8),
    TooManyEquipmentSlots(usize),
    InvalidMovementCommand(u8),
    InvalidClockType(u8),
    InvalidAttackCounts {
        targets: usize,
        hits: usize,
    },
    InvalidAttackDamageCount {
        expected: usize,
        actual: usize,
    },
    InvalidAttackDamage(i32),
    InvalidForcedStatMask(u32),
    InvalidTraction,
    TooManyChatRecipients(usize),
    InvalidBuddyGroupLength(usize),
    InvalidInventoryType(i8),
    InvalidPetSlot(u8),
    InvalidPetSpawnMode(i8),
    InvalidInventoryModification(u8),
    InvalidInventoryCount(i16),
    TooManyInventoryItems(usize),
    TooManyInventoryModifications(usize),
    InvalidSkillCount(i16),
    TooManySkillEntries(usize),
    TooManySkillMacros(usize),
    SkillMacroNameTooLong(usize),
    InvalidSkillMacroSkillId(i32),
    InvalidQuickSlotFlag(u8),
    InvalidKeyMapFlag(u8),
    InvalidQuickSlotKey(i32),
    DuplicateQuickSlotKey(i32),
    InvalidKeyMapKey(u8),
    DuplicateKeyMapKey(u8),
    TooManyKeyMapUpdates(usize),
    InvalidQuestCount(i16),
    TooManyQuestEntries(usize),
    TooManyCooldowns(usize),
    InvalidInitialSetFieldCount {
        section: &'static str,
        count: i16,
    },
    TooManyInitialSetFieldEntries {
        section: &'static str,
        maximum: usize,
    },
    TooManyGuildMembers(usize),
    InvalidAllianceGuildCount(i32),
    TooManyAllianceGuilds(usize),
    InvalidGuildBbsType(u8),
    InvalidBbsThreadCount(i32),
    TooManyBbsReplies(usize),
    InvalidBbsTextLength {
        actual: usize,
        maximum: usize,
    },
    InvalidNpcShopCount(i16),
    InvalidFieldObstacleCount(i32),
    TooManyCashShopItems(usize),
    InvalidCashShopWorldCount(i32),
    TooManyCashShopWorlds(usize),
    InvalidCashShopInventorySlot(i16),
    InvalidStorageInventoryType(i16),
    InvalidStorageItemId(i32),
    TooManyStorageItems(usize),
    CashShopItemMismatch {
        expected: i32,
        actual: i32,
    },
    InvalidNpcShopAction {
        slot: i16,
        item_id: i32,
        quantity: i16,
    },
    InvalidOwlResultCount(i32),
    InvalidOwlTopItemCount(i8),
    InvalidOwlItemId(i32),
    TooManyOwlResults(usize),
    TooManyOwlTopItems(usize),
    TooManyMakerEntries(usize),
    InvalidMerchantCount(i16),
    TooManyMerchantEntries(usize),
    MissingMiniRoomTerminator,
    InvalidTradePlayer(u8),
    InvalidTradeSlot(u8),
    InvalidTradeCount(i16),
    InvalidTradeMeso(i32),
    InvalidCharacterInfoCount {
        section: &'static str,
        count: i32,
    },
    InvalidFamilyCount {
        section: &'static str,
        count: i32,
    },
    InvalidEventCount(i16),
    TooManyEventEntries(usize),
    InvalidEventRewardCount(i16),
    TooManyEventRewards(usize),
}

impl fmt::Display for PacketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated {
                position,
                requested,
                remaining,
            } => write!(
                f,
                "packet underflow at byte {position}: requested {requested}, only {remaining} remain"
            ),
            Self::StringTooLong(length) => {
                write!(
                    f,
                    "packet string has {length} bytes; v83 length limit is 65535"
                )
            }
            Self::InvalidFrameLength(length) => write!(f, "invalid packet body length {length}"),
            Self::InvalidHandshakeLength(length) => write!(
                f,
                "Maple handshake is {length} bytes; 16 bytes are required"
            ),
            Self::InvalidEncryptedHeader { expected, actual } => write!(
                f,
                "encrypted packet header mask {actual:#06x} does not match receive IV mask {expected:#06x}"
            ),
            Self::UnexpectedOpcode { expected, actual } => {
                write!(f, "expected packet opcode {expected}, received {actual}")
            }
            Self::MissingWorldTerminator => write!(f, "world list is missing its terminator"),
            Self::InvalidCharacterCount(count) => {
                write!(f, "character list contains {count} entries; maximum is 64")
            }
            Self::TooManyRecommendedWorlds(count) => {
                write!(
                    f,
                    "recommended-world list contains {count} entries; maximum is 64"
                )
            }
            Self::TooManyEquipmentSlots(count) => {
                write!(f, "character contains more than {count} equipment slots")
            }
            Self::InvalidMovementCommand(command) => {
                write!(f, "unknown movement fragment command {command}")
            }
            Self::InvalidClockType(kind) => write!(f, "unknown v83 clock packet type {kind}"),
            Self::InvalidAttackCounts { targets, hits } => write!(
                f,
                "attack has {targets} targets and {hits} hits; v83 permits 0..=15 targets and 1..=15 hits"
            ),
            Self::InvalidAttackDamageCount { expected, actual } => write!(
                f,
                "attack target has {actual} damage lines; {expected} were declared"
            ),
            Self::InvalidAttackDamage(damage) => {
                write!(f, "attack damage cannot be negative: {damage}")
            }
            Self::InvalidForcedStatMask(mask) => {
                write!(
                    f,
                    "forced-stat packet contains unsupported mask {mask:#010x}"
                )
            }
            Self::InvalidTraction => {
                write!(f, "field traction must be a finite v83 double")
            }
            Self::TooManyChatRecipients(count) => {
                write!(
                    f,
                    "chat recipient list contains {count} entries; maximum is 255"
                )
            }
            Self::InvalidBuddyGroupLength(length) => write!(
                f,
                "buddy group name has {length} UTF-16 code units; v83 permits at most 16"
            ),
            Self::InvalidInventoryType(value) => {
                write!(f, "unknown inventory type {value}")
            }
            Self::InvalidPetSlot(value) => {
                write!(f, "invalid pet slot {value}; v83 permits slots 0 through 2")
            }
            Self::InvalidPetSpawnMode(value) => {
                write!(f, "unknown pet spawn mode {value}")
            }
            Self::InvalidInventoryModification(mode) => {
                write!(f, "unknown inventory modification mode {mode}")
            }
            Self::InvalidInventoryCount(count) => {
                write!(f, "inventory stack count cannot be negative: {count}")
            }
            Self::TooManyInventoryItems(maximum) => {
                write!(f, "inventory list exceeds {maximum} items")
            }
            Self::TooManyInventoryModifications(maximum) => {
                write!(f, "inventory packet exceeds {maximum} modifications")
            }
            Self::InvalidSkillCount(count) => {
                write!(f, "skill packet contains a negative entry count: {count}")
            }
            Self::TooManySkillEntries(maximum) => {
                write!(f, "skill packet exceeds {maximum} entries")
            }
            Self::TooManySkillMacros(count) => {
                write!(
                    f,
                    "skill-macro packet contains {count} entries; maximum is 3"
                )
            }
            Self::SkillMacroNameTooLong(length) => {
                write!(f, "skill-macro name has {length} characters; maximum is 13")
            }
            Self::InvalidSkillMacroSkillId(skill_id) => {
                write!(f, "skill-macro contains invalid skill id {skill_id}")
            }
            Self::InvalidQuickSlotFlag(flag) => {
                write!(f, "invalid quick-slot init flag {flag}; expected 0 or 1")
            }
            Self::InvalidKeyMapFlag(flag) => {
                write!(
                    f,
                    "invalid key-map enable-actions flag {flag}; expected 0 or 1"
                )
            }
            Self::InvalidQuickSlotKey(key) => {
                write!(f, "invalid quick-slot Maple key index {key}")
            }
            Self::DuplicateQuickSlotKey(key) => {
                write!(f, "quick-slot layout repeats Maple key index {key}")
            }
            Self::InvalidKeyMapKey(key) => {
                write!(f, "invalid Maple key index {key}; expected 0 through 89")
            }
            Self::DuplicateKeyMapKey(key) => {
                write!(f, "key-map update repeats Maple key index {key}")
            }
            Self::TooManyKeyMapUpdates(count) => {
                write!(f, "key-map update has {count} entries; maximum is 90")
            }
            Self::InvalidQuestCount(count) => {
                write!(f, "quest packet contains a negative entry count: {count}")
            }
            Self::TooManyQuestEntries(maximum) => {
                write!(f, "quest log exceeds {maximum} entries")
            }
            Self::TooManyCooldowns(maximum) => {
                write!(
                    f,
                    "initial character state exceeds {maximum} cooldown entries"
                )
            }
            Self::InvalidInitialSetFieldCount { section, count } => {
                write!(
                    f,
                    "initial SET_FIELD {section} has a negative count: {count}"
                )
            }
            Self::TooManyInitialSetFieldEntries { section, maximum } => {
                write!(f, "initial SET_FIELD {section} exceeds {maximum} entries")
            }
            Self::TooManyGuildMembers(maximum) => {
                write!(f, "guild roster exceeds {maximum} members")
            }
            Self::InvalidAllianceGuildCount(count) => {
                write!(
                    f,
                    "alliance packet contains a negative guild count: {count}"
                )
            }
            Self::TooManyAllianceGuilds(maximum) => {
                write!(f, "alliance roster exceeds the {maximum}-guild limit")
            }
            Self::InvalidGuildBbsType(value) => {
                write!(f, "unknown guild BBS response type {value:#04x}")
            }
            Self::InvalidBbsThreadCount(count) => {
                write!(f, "guild BBS returned an invalid thread count {count}")
            }
            Self::TooManyBbsReplies(maximum) => {
                write!(f, "guild BBS thread exceeds {maximum} replies")
            }
            Self::InvalidBbsTextLength { actual, maximum } => {
                write!(
                    f,
                    "guild BBS text is {actual} UTF-16 code units; maximum is {maximum}"
                )
            }
            Self::InvalidNpcShopCount(count) => {
                write!(f, "NPC shop contains an invalid item count: {count}")
            }
            Self::InvalidFieldObstacleCount(count) => {
                write!(
                    f,
                    "field obstacle packet contains an invalid count: {count}"
                )
            }
            Self::TooManyCashShopItems(maximum) => {
                write!(f, "cash shop locker exceeds {maximum} items")
            }
            Self::InvalidCashShopWorldCount(count) => {
                write!(
                    f,
                    "cash shop transfer contains an invalid world count: {count}"
                )
            }
            Self::TooManyCashShopWorlds(maximum) => {
                write!(f, "cash shop transfer exceeds {maximum} worlds")
            }
            Self::InvalidCashShopInventorySlot(slot) => {
                write!(f, "invalid inventory slot {slot} in cash shop withdrawal")
            }
            Self::InvalidStorageInventoryType(value) => {
                write!(
                    f,
                    "unknown inventory type bitfield {value} in storage packet"
                )
            }
            Self::InvalidStorageItemId(item_id) => {
                write!(f, "invalid item id {item_id} in storage packet")
            }
            Self::TooManyStorageItems(maximum) => {
                write!(f, "storage packet exceeds {maximum} items")
            }
            Self::CashShopItemMismatch { expected, actual } => write!(
                f,
                "cash shop withdrawal item id {actual} does not match expected id {expected}"
            ),
            Self::InvalidNpcShopAction {
                slot,
                item_id,
                quantity,
            } => write!(
                f,
                "invalid NPC shop action for slot {slot}, item {item_id}, quantity {quantity}"
            ),
            Self::InvalidOwlResultCount(count) => {
                write!(f, "Owl search returned a negative result count: {count}")
            }
            Self::InvalidOwlTopItemCount(count) => {
                write!(
                    f,
                    "Owl top-item response returned an invalid count: {count}"
                )
            }
            Self::InvalidOwlItemId(item_id) => {
                write!(
                    f,
                    "Owl top-item response returned an invalid item ID: {item_id}"
                )
            }
            Self::TooManyOwlResults(count) => {
                write!(f, "Owl search returned {count} results; maximum is 100")
            }
            Self::TooManyOwlTopItems(count) => write!(
                f,
                "Owl top-item response returned {count} entries; maximum is 10"
            ),
            Self::TooManyMakerEntries(count) => {
                write!(f, "Maker result contains an invalid item count: {count}")
            }
            Self::InvalidMerchantCount(count) => {
                write!(f, "hired merchant room returned an invalid count: {count}")
            }
            Self::TooManyMerchantEntries(maximum) => {
                write!(f, "hired merchant room exceeds the {maximum}-entry limit")
            }
            Self::MissingMiniRoomTerminator => {
                write!(f, "mini-room visitor list is missing its terminator")
            }
            Self::InvalidTradePlayer(player) => {
                write!(
                    f,
                    "trade packet refers to player slot {player}; only 0 and 1 are valid"
                )
            }
            Self::InvalidTradeSlot(slot) => {
                write!(
                    f,
                    "trade packet refers to item slot {slot}; only slots 0 through 8 are valid"
                )
            }
            Self::InvalidTradeCount(count) => {
                write!(f, "trade item count is outside the v83 range: {count}")
            }
            Self::InvalidTradeMeso(meso) => {
                write!(f, "trade meso offer cannot be negative: {meso}")
            }
            Self::InvalidCharacterInfoCount { section, count } => {
                write!(f, "character info {section} count {count} is invalid")
            }
            Self::InvalidFamilyCount { section, count } => {
                write!(f, "family {section} count {count} is invalid")
            }
            Self::InvalidEventCount(count) => {
                write!(f, "event list contains a negative event count: {count}")
            }
            Self::TooManyEventEntries(count) => {
                write!(f, "event list contains {count} entries; maximum is 64")
            }
            Self::InvalidEventRewardCount(count) => {
                write!(f, "event contains a negative reward count: {count}")
            }
            Self::TooManyEventRewards(count) => {
                write!(f, "event contains {count} rewards; maximum is 64")
            }
        }
    }
}

impl Error for PacketError {}

#[derive(Clone, Debug)]
pub struct PacketReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> PacketReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub fn read_bytes(&mut self, length: usize) -> Result<&'a [u8], PacketError> {
        let remaining = self.remaining();
        if length > remaining {
            return Err(PacketError::Truncated {
                position: self.position,
                requested: length,
                remaining,
            });
        }
        let start = self.position;
        self.position += length;
        Ok(&self.bytes[start..self.position])
    }

    pub fn skip(&mut self, length: usize) -> Result<(), PacketError> {
        self.read_bytes(length).map(|_| ())
    }

    pub fn read_bool(&mut self) -> Result<bool, PacketError> {
        Ok(self.read_u8()? == 1)
    }

    pub fn read_u8(&mut self) -> Result<u8, PacketError> {
        Ok(self.read_bytes(1)?[0])
    }

    pub fn read_i8(&mut self) -> Result<i8, PacketError> {
        Ok(self.read_u8()? as i8)
    }

    pub fn read_u16(&mut self) -> Result<u16, PacketError> {
        Ok(u16::from_le_bytes(
            self.read_bytes(2)?.try_into().expect("length checked"),
        ))
    }

    pub fn read_i16(&mut self) -> Result<i16, PacketError> {
        Ok(i16::from_le_bytes(
            self.read_bytes(2)?.try_into().expect("length checked"),
        ))
    }

    pub fn read_u32(&mut self) -> Result<u32, PacketError> {
        Ok(u32::from_le_bytes(
            self.read_bytes(4)?.try_into().expect("length checked"),
        ))
    }

    pub fn read_i32(&mut self) -> Result<i32, PacketError> {
        Ok(i32::from_le_bytes(
            self.read_bytes(4)?.try_into().expect("length checked"),
        ))
    }

    pub fn read_i64(&mut self) -> Result<i64, PacketError> {
        Ok(i64::from_le_bytes(
            self.read_bytes(8)?.try_into().expect("length checked"),
        ))
    }

    pub fn read_u64(&mut self) -> Result<u64, PacketError> {
        Ok(u64::from_le_bytes(
            self.read_bytes(8)?.try_into().expect("length checked"),
        ))
    }

    pub fn read_point_i16(&mut self) -> Result<(i16, i16), PacketError> {
        Ok((self.read_i16()?, self.read_i16()?))
    }

    pub fn read_string(&mut self) -> Result<String, PacketError> {
        let length = usize::from(self.read_u16()?);
        self.read_string_bytes(length)
    }

    /// Matches the legacy padded-string reader by removing zero bytes wherever they
    /// occur, while safely handling arbitrary v83/UTF-8 payload bytes.
    pub fn read_string_bytes(&mut self, length: usize) -> Result<String, PacketError> {
        let bytes = self
            .read_bytes(length)?
            .iter()
            .copied()
            .filter(|byte| *byte != 0)
            .collect::<Vec<_>>();
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

#[derive(Clone, Debug, Default)]
pub struct PacketWriter {
    bytes: Vec<u8>,
}

impl PacketWriter {
    pub fn new(opcode: u16) -> Self {
        Self {
            bytes: opcode.to_le_bytes().to_vec(),
        }
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    pub fn write_bool(&mut self, value: bool) {
        self.write_u8(u8::from(value));
    }

    pub fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub fn write_i8(&mut self, value: i8) {
        self.write_u8(value as u8);
    }

    pub fn write_u16(&mut self, value: u16) {
        self.write_bytes(&value.to_le_bytes());
    }

    pub fn write_i16(&mut self, value: i16) {
        self.write_bytes(&value.to_le_bytes());
    }

    pub fn write_u32(&mut self, value: u32) {
        self.write_bytes(&value.to_le_bytes());
    }

    pub fn write_i32(&mut self, value: i32) {
        self.write_bytes(&value.to_le_bytes());
    }

    pub fn write_i64(&mut self, value: i64) {
        self.write_bytes(&value.to_le_bytes());
    }

    pub fn write_point_i16(&mut self, x: i16, y: i16) {
        self.write_i16(x);
        self.write_i16(y);
    }

    pub fn write_string(&mut self, value: &str) -> Result<(), PacketError> {
        let length = value.len();
        let length_u16 = u16::try_from(length).map_err(|_| PacketError::StringTooLong(length))?;
        self.write_u16(length_u16);
        self.write_bytes(value.as_bytes());
        Ok(())
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}
