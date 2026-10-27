//! Bounded v83 key-map and quick-slot packets.

use crate::login::expect_opcode;
use crate::{PacketError, PacketReader, PacketWriter};

pub const OPCODE_CHANGE_KEYMAP: u16 = 135;
pub const OPCODE_QUICKSLOT_INIT: u16 = 159;
pub const OPCODE_KEYMAP: u16 = 335;
pub const KEYMAP_ENTRY_COUNT: usize = 90;
pub const QUICKSLOT_KEY_COUNT: usize = 8;
const MAX_KEYMAP_UPDATES: usize = KEYMAP_ENTRY_COUNT;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyBinding {
    pub kind: u8,
    pub action: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyMapUpdate {
    pub enable_actions: bool,
    /// v83 sends one mapping for every Maple key index, 0 through 89.
    pub bindings: Vec<KeyBinding>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuickSlotInit {
    /// `None` means the server requested the client's default quick-slot keys.
    pub keys: Option<[u8; QUICKSLOT_KEY_COUNT]>,
}

/// Reads the server's complete v83 key map. The leading byte is the legacy
/// enable-actions flag; the following 90 entries are a type byte and action int.
pub fn parse_keymap(bytes: &[u8]) -> Result<KeyMapUpdate, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_KEYMAP)?;
    let enable_actions = match packet.read_u8()? {
        0 => false,
        1 => true,
        flag => return Err(PacketError::InvalidKeyMapFlag(flag)),
    };
    let mut bindings = Vec::with_capacity(KEYMAP_ENTRY_COUNT);
    for _ in 0..KEYMAP_ENTRY_COUNT {
        bindings.push(KeyBinding {
            kind: packet.read_u8()?,
            action: packet.read_i32()?,
        });
    }
    Ok(KeyMapUpdate {
        enable_actions,
        bindings,
    })
}

/// Reads the login-time optional eight-key quick-slot layout.
pub fn parse_quickslot_init(bytes: &[u8]) -> Result<QuickSlotInit, PacketError> {
    let mut packet = PacketReader::new(bytes);
    expect_opcode(&mut packet, OPCODE_QUICKSLOT_INIT)?;
    match packet.read_u8()? {
        0 => Ok(QuickSlotInit { keys: None }),
        1 => {
            let mut keys = [0; QUICKSLOT_KEY_COUNT];
            let mut seen = [false; KEYMAP_ENTRY_COUNT];
            for key in &mut keys {
                let raw_key = packet.read_i32()?;
                if !(1..KEYMAP_ENTRY_COUNT as i32).contains(&raw_key) {
                    return Err(PacketError::InvalidQuickSlotKey(raw_key));
                }
                let key_index = raw_key as usize;
                if seen[key_index] {
                    return Err(PacketError::DuplicateQuickSlotKey(raw_key));
                }
                seen[key_index] = true;
                *key = raw_key as u8;
            }
            Ok(QuickSlotInit { keys: Some(keys) })
        }
        flag => Err(PacketError::InvalidQuickSlotFlag(flag)),
    }
}

/// Encodes one or more changed key mappings using the v83 CHANGE_KEYMAP shape.
pub fn change_keymap_packet(updates: &[(u8, KeyBinding)]) -> Result<Vec<u8>, PacketError> {
    if updates.len() > MAX_KEYMAP_UPDATES {
        return Err(PacketError::TooManyKeyMapUpdates(updates.len()));
    }
    let mut seen = [false; KEYMAP_ENTRY_COUNT];
    for (key, _) in updates {
        if usize::from(*key) >= KEYMAP_ENTRY_COUNT {
            return Err(PacketError::InvalidKeyMapKey(*key));
        }
        if seen[usize::from(*key)] {
            return Err(PacketError::DuplicateKeyMapKey(*key));
        }
        seen[usize::from(*key)] = true;
    }

    let mut packet = PacketWriter::new(OPCODE_CHANGE_KEYMAP);
    packet.write_i32(0); // mode
    packet.write_i32(updates.len() as i32);
    for (key, binding) in updates {
        packet.write_i32(i32::from(*key));
        packet.write_u8(binding.kind);
        packet.write_i32(binding.action);
    }
    Ok(packet.into_bytes())
}
