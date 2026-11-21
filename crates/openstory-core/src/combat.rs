//! Platform-independent character combat rules.
//!
//! Asset-backed inputs such as a weapon's attack class are resolved by the
//! client and passed here as values. This module owns the v83 arithmetic and
//! stance selection that does not depend on NX or transport state.

/// Returns the legacy base-damage multiplier for a MapleStory weapon category.
/// `weapon_type` is the item ID prefix (`item_id / 10_000`).
pub fn weapon_damage_multiplier(weapon_type: i32) -> f64 {
    match weapon_type {
        130 => 4.0,
        131 | 132 | 137 | 138 => 4.4,
        133 | 146 | 147 | 149 => 3.6,
        140 => 4.6,
        141 | 142 | 148 => 4.8,
        143 | 144 => 5.0,
        145 => 3.4,
        _ => 0.0,
    }
}

/// Fallback v83 attack class for weapon nodes whose NX `info/attack` is absent.
pub fn fallback_weapon_attack_class(weapon_type: i32) -> u8 {
    match weapon_type {
        130..=133 => 1,
        143..=144 => 2,
        145 => 3,
        146 => 4,
        140..=142 => 5,
        137..=138 => 6,
        147..=148 => 7,
        149 => 9,
        _ => 0,
    }
}

/// Selects the stance sequence used by a regular attack.
///
/// `is_ranged_attack` preserves the client distinction used for the special
/// degenerate bow, crossbow, claw, and gun attack animations.
pub fn regular_attack_stance(
    attack_class: u8,
    weapon_type: i32,
    is_ranged_attack: bool,
    attack_count: u32,
) -> Option<u8> {
    let degenerate = !is_ranged_attack && matches!(weapon_type, 145 | 146 | 147 | 149);
    let stances: &[u8] = if degenerate {
        match attack_class {
            3 => &[30, 32],
            4 | 7 => &[30, 18],
            9 => &[27, 19],
            _ => &[],
        }
    } else {
        match attack_class {
            1 => &[15, 16, 23, 24, 25],
            2 => &[18, 27],
            3 => &[11],
            4 => &[12],
            5 => &[15, 16, 30, 31, 32],
            6 | 7 => &[23, 24],
            9 => &[10],
            _ => &[],
        }
    };
    (!stances.is_empty()).then(|| stances[attack_count as usize % stances.len()])
}

/// Checks whether a character's job progression includes the job encoded by
/// a v83 skill ID. Beginner, Cygnus, and Aran job lines keep their legacy
/// special parent-job mappings.
pub fn skill_job_can_use(job_id: u16, skill_id: i32) -> bool {
    let required_job = skill_id / 10_000;
    if required_job == 0 {
        return true;
    }
    let is_beginner = matches!(job_id, 0 | 900 | 1000 | 2000);
    let job_level = if is_beginner {
        0
    } else if job_id == 910 || job_id % 100 == 0 {
        1
    } else if job_id % 10 == 0 {
        2
    } else if job_id % 10 == 1 {
        3
    } else {
        4
    };
    let beginner_id = if (2000..=2112).contains(&job_id) {
        2000
    } else if (1000..=1512).contains(&job_id) {
        1000
    } else if matches!(job_id, 900 | 910) {
        900
    } else {
        0
    };
    let subjobs = [
        beginner_id,
        if job_id == 910 {
            910
        } else {
            job_id / 100 * 100
        },
        job_id / 10 * 10,
        if job_level == 4 { job_id - 1 } else { job_id },
        job_id,
    ];
    subjobs
        .into_iter()
        .take(job_level + 1)
        .any(|subjob| i32::from(subjob) == required_job)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillUseBlockReason {
    NotMastered,
    WrongJob,
    InsufficientHp,
    InsufficientMp,
    WrongWeapon,
    InsufficientAmmo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillUseRules {
    pub skill_id: i32,
    pub skill_level: i32,
    pub master_level: i32,
    pub job_id: u16,
    pub current_hp: u16,
    pub hp_cost: u16,
    pub current_mp: u16,
    pub mp_cost: u16,
    pub required_weapon: Option<i32>,
    pub weapon_type: Option<i32>,
    pub uses_projectile: bool,
    pub available_projectiles: u16,
    pub projectile_cost: u16,
}

/// Evaluates deterministic v83 skill-use restrictions after the client has
/// resolved weapon and inventory data from NX and the authoritative snapshot.
pub fn evaluate_skill_use(rules: SkillUseRules) -> Option<SkillUseBlockReason> {
    // v83 reports a master level only for skills that use mastery books.
    // A zero value means the skill has no separate mastery cap, not that
    // every learned level is unmastered.
    if rules.master_level > 0 && rules.skill_level > rules.master_level {
        return Some(SkillUseBlockReason::NotMastered);
    }
    if !skill_job_can_use(rules.job_id, rules.skill_id) {
        return Some(SkillUseBlockReason::WrongJob);
    }
    if rules.current_hp <= rules.hp_cost {
        return Some(SkillUseBlockReason::InsufficientHp);
    }
    if rules.current_mp < rules.mp_cost {
        return Some(SkillUseBlockReason::InsufficientMp);
    }
    if rules
        .required_weapon
        .is_some_and(|required| rules.weapon_type != Some(required))
    {
        return Some(SkillUseBlockReason::WrongWeapon);
    }
    if rules.uses_projectile && rules.available_projectiles < rules.projectile_cost {
        return Some(SkillUseBlockReason::InsufficientAmmo);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        SkillUseBlockReason, SkillUseRules, evaluate_skill_use, fallback_weapon_attack_class,
        regular_attack_stance, skill_job_can_use, weapon_damage_multiplier,
    };

    #[test]
    fn weapon_damage_multipliers_match_v83_categories() {
        for (weapon_type, expected) in [
            (130, 4.0),
            (132, 4.4),
            (143, 5.0),
            (145, 3.4),
            (146, 3.6),
            (140, 4.6),
            (148, 4.8),
            (999, 0.0),
        ] {
            assert_eq!(weapon_damage_multiplier(weapon_type), expected);
        }
    }

    #[test]
    fn missing_nx_attack_class_uses_legacy_weapon_fallbacks() {
        for (weapon_type, expected) in [
            (130, 1),
            (143, 2),
            (145, 3),
            (146, 4),
            (142, 5),
            (138, 6),
            (148, 7),
            (149, 9),
            (999, 0),
        ] {
            assert_eq!(fallback_weapon_attack_class(weapon_type), expected);
        }
    }

    #[test]
    fn regular_attack_stances_cycle_and_handle_degenerate_weapons() {
        assert_eq!(regular_attack_stance(1, 130, false, 0), Some(15));
        assert_eq!(regular_attack_stance(1, 130, false, 4), Some(25));
        assert_eq!(regular_attack_stance(1, 130, false, 5), Some(15));
        assert_eq!(regular_attack_stance(3, 145, false, 0), Some(30));
        assert_eq!(regular_attack_stance(3, 145, false, 1), Some(32));
        assert_eq!(regular_attack_stance(3, 145, true, 0), Some(11));
        assert_eq!(regular_attack_stance(0, 130, false, 0), None);
    }

    #[test]
    fn skill_job_restrictions_follow_legacy_job_progression() {
        assert!(skill_job_can_use(110, 1_004)); // beginner skill
        assert!(skill_job_can_use(110, 1_000_000)); // warrior parent job
        assert!(skill_job_can_use(110, 1_100_000)); // fighter job
        assert!(!skill_job_can_use(110, 1_200_000)); // magician job
        assert!(skill_job_can_use(112, 1_110_000)); // fourth job keeps third job skills
        assert!(skill_job_can_use(2000, 20_000_000)); // Aran beginner line
        assert!(!skill_job_can_use(2000, 1_000_000));
    }

    #[test]
    fn skill_use_restrictions_preserve_legacy_check_order_and_costs() {
        let base = SkillUseRules {
            skill_id: 1_100_000,
            skill_level: 3,
            master_level: 20,
            job_id: 110,
            current_hp: 100,
            hp_cost: 10,
            current_mp: 100,
            mp_cost: 10,
            required_weapon: Some(130),
            weapon_type: Some(130),
            uses_projectile: true,
            available_projectiles: 5,
            projectile_cost: 2,
        };
        assert_eq!(evaluate_skill_use(base), None);
        assert_eq!(
            evaluate_skill_use(SkillUseRules {
                skill_level: 21,
                ..base
            }),
            Some(SkillUseBlockReason::NotMastered)
        );
        assert_eq!(
            evaluate_skill_use(SkillUseRules {
                job_id: 120,
                ..base
            }),
            Some(SkillUseBlockReason::WrongJob)
        );
        assert_eq!(
            evaluate_skill_use(SkillUseRules {
                current_hp: 10,
                ..base
            }),
            Some(SkillUseBlockReason::InsufficientHp)
        );
        assert_eq!(
            evaluate_skill_use(SkillUseRules {
                current_mp: 9,
                ..base
            }),
            Some(SkillUseBlockReason::InsufficientMp)
        );
        assert_eq!(
            evaluate_skill_use(SkillUseRules {
                weapon_type: Some(140),
                ..base
            }),
            Some(SkillUseBlockReason::WrongWeapon)
        );
        assert_eq!(
            evaluate_skill_use(SkillUseRules {
                available_projectiles: 1,
                ..base
            }),
            Some(SkillUseBlockReason::InsufficientAmmo)
        );
    }
}
