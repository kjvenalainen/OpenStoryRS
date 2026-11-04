//! Platform-independent state for the local character's active buffs.
//!
//! Protocol bit masks are converted to [`BuffKey`] values by the client. This
//! module owns replacement, cancellation, lookup, and expiry semantics without
//! depending on packet or rendering types.

use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuffGroup {
    First,
    Second,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BuffKey {
    pub group: BuffGroup,
    pub flag: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveBuff {
    pub value: i16,
    pub skill_id: i32,
    /// Original duration from the v83 packet, retained for the native buff UI.
    pub duration_ms: i32,
    pub applied_at: Instant,
    pub expires_at: Option<Instant>,
}

impl ActiveBuff {
    pub fn is_active_at(self, now: Instant) -> bool {
        self.expires_at.is_none_or(|expires_at| expires_at > now)
    }
}

#[derive(Clone, Debug, Default)]
pub struct CharacterBuffs {
    entries: HashMap<BuffKey, ActiveBuff>,
}

impl CharacterBuffs {
    /// Applies a buff, replacing an existing value for the same mask key.
    /// Nonpositive durations represent buffs with no client-side timeout.
    pub fn apply(
        &mut self,
        key: BuffKey,
        value: i16,
        skill_id: i32,
        duration_ms: i32,
        now: Instant,
    ) -> Option<ActiveBuff> {
        let expires_at = (duration_ms > 0).then(|| now + Duration::from_millis(duration_ms as u64));
        self.entries.insert(
            key,
            ActiveBuff {
                value,
                skill_id,
                duration_ms,
                applied_at: now,
                expires_at,
            },
        )
    }

    /// Removes keys whose bits overlap the corresponding server cancel masks.
    pub fn cancel_masks(&mut self, first_mask: u64, second_mask: u64) -> usize {
        let before = self.entries.len();
        self.entries.retain(|key, _| {
            let cancel_mask = match key.group {
                BuffGroup::First => first_mask,
                BuffGroup::Second => second_mask,
            };
            cancel_mask & key.flag == 0
        });
        before - self.entries.len()
    }

    /// Removes expired entries and returns the number removed.
    pub fn expire(&mut self, now: Instant) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, buff| buff.is_active_at(now));
        before - self.entries.len()
    }

    /// Returns the stored entry, including one whose expiry has elapsed.
    /// This is useful when the client needs to detect a mount transition while
    /// expiring the state.
    pub fn get(&self, key: &BuffKey) -> Option<&ActiveBuff> {
        self.entries.get(key)
    }

    /// Returns the entry only while its server-provided duration remains active.
    pub fn get_active(&self, key: &BuffKey, now: Instant) -> Option<&ActiveBuff> {
        self.entries.get(key).filter(|buff| buff.is_active_at(now))
    }

    /// Returns the value of an active buff, or zero if absent or expired.
    pub fn value(&self, key: &BuffKey, now: Instant) -> i16 {
        self.get_active(key, now).map_or(0, |buff| buff.value)
    }

    pub fn contains_key(&self, key: &BuffKey) -> bool {
        self.entries.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn values(&self) -> impl Iterator<Item = &ActiveBuff> {
        self.entries.values()
    }
}

#[cfg(test)]
mod tests {
    use super::{BuffGroup, BuffKey, CharacterBuffs};
    use std::time::{Duration, Instant};

    fn key(group: BuffGroup, flag: u64) -> BuffKey {
        BuffKey { group, flag }
    }

    #[test]
    fn applying_same_key_replaces_value_and_expiry() {
        let start = Instant::now();
        let second = start + Duration::from_millis(20);
        let mut buffs = CharacterBuffs::default();
        let key = key(BuffGroup::First, 0x04);

        assert_eq!(buffs.apply(key, 3, 10, 100, start), None);
        let replaced = buffs.apply(key, 7, 11, 200, second).unwrap();

        assert_eq!(buffs.len(), 1);
        assert_eq!(replaced.value, 3);
        assert_eq!(buffs.value(&key, second), 7);
        assert_eq!(buffs.get(&key).unwrap().skill_id, 11);
        assert_eq!(
            buffs.get(&key).unwrap().expires_at,
            Some(second + Duration::from_millis(200))
        );
    }

    #[test]
    fn cancel_masks_match_groups_and_remove_overlapping_composite_flags() {
        let now = Instant::now();
        let mut buffs = CharacterBuffs::default();
        let first = key(BuffGroup::First, 0x04);
        let second = key(BuffGroup::Second, 0x10);
        let composite = key(BuffGroup::Second, 0x30);
        let remaining = key(BuffGroup::Second, 0x40);
        for buff_key in [first, second, composite, remaining] {
            buffs.apply(buff_key, 1, 1, 0, now);
        }

        assert_eq!(buffs.cancel_masks(0x04, 0x10), 3);
        assert!(!buffs.contains_key(&first));
        assert!(!buffs.contains_key(&second));
        assert!(!buffs.contains_key(&composite));
        assert!(buffs.contains_key(&remaining));
    }

    #[test]
    fn active_values_respect_expiry_and_keep_indefinite_buffs() {
        let start = Instant::now();
        let timed = key(BuffGroup::Second, 0x80);
        let indefinite = key(BuffGroup::Second, 0x100);
        let mut buffs = CharacterBuffs::default();
        buffs.apply(timed, 4, 7, 100, start);
        buffs.apply(indefinite, -2, 8, 0, start);

        assert_eq!(buffs.value(&timed, start + Duration::from_millis(99)), 4);
        assert_eq!(buffs.value(&timed, start + Duration::from_millis(100)), 0);
        assert_eq!(buffs.value(&indefinite, start + Duration::from_secs(1)), -2);
        assert_eq!(buffs.expire(start + Duration::from_millis(100)), 1);
        assert!(buffs.contains_key(&indefinite));
        assert!(!buffs.contains_key(&timed));
    }
}
