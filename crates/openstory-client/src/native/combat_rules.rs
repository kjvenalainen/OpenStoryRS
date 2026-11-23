fn skill_use_block_reason(
    skill: &SkillUiEntry,
    character: &openstory_protocol::CharacterInfo,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
) -> Option<&'static str> {
    let weapon_type = equipped_weapon_id(&character.look).map(|item_id| item_id / 10_000);
    let uses_projectile = weapon_type.is_some_and(|kind| weapon_projectile_prefix(kind).is_some());
    let available_projectiles =
        weapon_type.map_or(0, |kind| weapon_projectile_count(inventory, kind));
    match evaluate_skill_use(SkillUseRules {
        skill_id: skill.id,
        skill_level: skill.level,
        master_level: skill.master_level,
        job_id: character.stats.job,
        current_hp: character.stats.hp,
        hp_cost: skill.hp_cost,
        current_mp: character.stats.mp,
        mp_cost: skill.mp_cost,
        required_weapon: skill.required_weapon,
        weapon_type,
        uses_projectile,
        available_projectiles,
        projectile_cost: skill.bullet_cost,
    }) {
        Some(SkillUseBlockReason::NotMastered) => Some("You have not mastered that skill yet."),
        Some(SkillUseBlockReason::WrongJob) => Some("Your job cannot use that skill."),
        Some(SkillUseBlockReason::InsufficientHp) => {
            Some("You do not have enough HP to use that skill.")
        }
        Some(SkillUseBlockReason::InsufficientMp) => {
            Some("You do not have enough MP to use that skill.")
        }
        Some(SkillUseBlockReason::WrongWeapon) => {
            Some("You cannot use that skill with your current weapon.")
        }
        Some(SkillUseBlockReason::InsufficientAmmo) => {
            Some("You do not have enough ammunition to use that skill.")
        }
        None => None,
    }
}

fn attack_horizontal_bounds(
    player_x: f64,
    facing_right: bool,
    range: [i16; 4],
    horizontal_range_multiplier: f64,
    horizontal_range_bonus: i16,
) -> (f64, f64) {
    let horizontal_edge = (f64::from(range[0]) - f64::from(horizontal_range_bonus))
        * horizontal_range_multiplier;
    if facing_right {
        (
            player_x - f64::from(range[1]),
            player_x - horizontal_edge,
        )
    } else {
        (
            player_x + horizontal_edge,
            player_x + f64::from(range[1]),
        )
    }
}

fn character_buff_value(
    active_buffs: &CharacterBuffs,
    group: openstory_protocol::BuffMaskGroup,
    flag: u64,
) -> i16 {
    let now = Instant::now();
    active_buffs.value(&character_buff_key(group, flag), now)
}

fn character_has_buff(
    active_buffs: &CharacterBuffs,
    group: openstory_protocol::BuffMaskGroup,
    flag: u64,
) -> bool {
    character_buff_value(active_buffs, group, flag) != 0
}

fn character_can_use_skill(job_id: u16, skill_id: i32) -> bool {
    let Ok(required_job) = u16::try_from(skill_id / 10_000) else {
        return false;
    };
    required_job == 0
        || (0..=4).any(|tab| skill_subjob_for_tab(job_id, tab) == Some(required_job))
}

fn character_has_mob_skill_buff(
    active_buffs: &CharacterBuffs,
    group: openstory_protocol::BuffMaskGroup,
    flag: u64,
    mob_skill_type: u16,
) -> bool {
    let now = Instant::now();
    active_buffs
        .get_active(&character_buff_key(group, flag), now)
        .is_some_and(|buff| {
            // MobSkillId is serialized as a little-endian pair of u16s:
            // type first, then level. Character skill IDs do not overlap.
            let encoded_skill_id = buff.skill_id as u32;
            encoded_skill_id as u16 == mob_skill_type && (encoded_skill_id >> 16) as u16 != 0
        })
}

fn player_movement_forces(
    character: &openstory_protocol::CharacterInfo,
    archive: &NxArchive,
    active_buffs: &CharacterBuffs,
    forced_stats: &ForcedEquipmentStats,
    is_slowed: bool,
) -> (f64, f64, f64) {
    let mut speed = 100_i64;
    let mut jump = 100_i64;
    let equipped_items = character
        .look
        .equips
        .iter()
        .map(|(_, item_id)| *item_id)
        .collect::<HashSet<_>>();
    for item_id in equipped_items {
        let Some(category) = character_equip_category(item_id) else {
            continue;
        };
        let info_path = format!("{category}/0{item_id}.img/info");
        let Some(info) = archive.root().resolve(&info_path).ok().flatten() else {
            continue;
        };
        speed = speed.saturating_add(integer_property(&info, "incSpeed").unwrap_or(0));
        jump = jump.saturating_add(integer_property(&info, "incJump").unwrap_or(0));
    }
    speed = speed.saturating_add(i64::from(character_buff_value(
        active_buffs,
        openstory_protocol::BuffMaskGroup::Second,
        0x0000_0080_0000_0000,
    )));
    jump = jump.saturating_add(i64::from(character_buff_value(
        active_buffs,
        openstory_protocol::BuffMaskGroup::Second,
        0x0000_0100_0000_0000,
    )));
    if let Some(value) = forced_stats.speed {
        speed = i64::from(value);
    }
    if let Some(value) = forced_stats.jump {
        jump = i64::from(value);
    }

    // The legacy stat table caps total speed at 140 and jump at 123.
    let speed = speed.clamp(0, 140) as f64;
    let jump = jump.clamp(0, 123) as f64;
    let mut walk_force = 0.05 + 0.11 * speed / 100.0;
    if is_slowed {
        walk_force *= 0.5;
    }
    (walk_force, 1.0 + 3.5 * jump / 100.0, speed / 100.0)
}

fn calculate_player_touch_damage(
    character: &openstory_protocol::CharacterInfo,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
    attack: i32,
    archive: &NxArchive,
    skill_archive: &NxArchive,
    skills: &HashMap<i32, openstory_protocol::SkillEntry>,
    active_buffs: &CharacterBuffs,
    forced_stats: &ForcedEquipmentStats,
) -> i32 {
    let mut weapon_defense = 0_i64;
    let mut found_inventory_defense = false;
    if let Some(inventory) = inventory {
        for item in inventory.equipped() {
            if let Some(equip) = item.equip.as_ref() {
                // The wire equipment stat table already includes scroll and
                // upgrade bonuses, matching CharStats::get_total(WDEF).
                weapon_defense =
                    weapon_defense.saturating_add(i64::from(equip.stats[8]));
                found_inventory_defense = true;
            }
        }
    }
    if !found_inventory_defense {
        for (_, item_id) in &character.look.equips {
            let Some(category) = character_equip_category(*item_id) else {
                continue;
            };
            let info_path = format!("{category}/0{item_id}.img/info");
            if let Some(info) = archive.root().resolve(&info_path).ok().flatten() {
                // Character.nx stores physical defense as incPDD (also used by
                // the inventory UI), not incWDEF.
                weapon_defense = weapon_defense
                    .saturating_add(integer_property(&info, "incPDD").unwrap_or(0));
            }
        }
    }

    weapon_defense = weapon_defense
        .saturating_add(i64::from(character_buff_value(
            active_buffs,
            openstory_protocol::BuffMaskGroup::Second,
            0x0000_0002_0000_0000,
        )))
        .min(999);
    if let Some(value) = forced_stats.weapon_defense {
        weapon_defense = i64::from(value);
    }

    let achilles = [1_120_004, 1_220_005, 1_320_005]
        .into_iter()
        .find_map(|skill_id| {
            character_can_use_skill(character.stats.job, skill_id)
                .then(|| learned_skill_level(skill_archive, skills, skill_id))
                .flatten()
                .and_then(|level| integer_property(&level, "x"))
        })
        .unwrap_or(0)
        .clamp(0, 1000) as f64
        / 1000.0;

    // Keep the v83 reference order: no defense means no reduction; otherwise
    // use half the mob's attack plus attack divided by total weapon defense.
    let damage_before_achilles = if weapon_defense == 0 {
        attack
    } else {
        attack / 2 + i32::try_from(i64::from(attack) / weapon_defense).unwrap_or(0)
    };
    damage_before_achilles.saturating_sub((f64::from(damage_before_achilles) * achilles) as i32)
}

fn player_avoidability(
    character: &openstory_protocol::CharacterInfo,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
    skill_archive: &NxArchive,
    skills: &HashMap<i32, openstory_protocol::SkillEntry>,
    active_buffs: &CharacterBuffs,
    forced_stats: &ForcedEquipmentStats,
) -> i32 {
    let equipment_avoidability = inventory.map_or(0_i32, |inventory| {
        inventory
            .equipped()
            .filter_map(|item| item.equip.as_ref())
            .map(|equip| i32::from(equip.stats[11]))
            .sum()
    });
    let angel_blessing_avoidability = if character_can_use_skill(character.stats.job, 12) {
        learned_skill_level(skill_archive, skills, 12)
            .and_then(|level| integer_property(&level, "z"))
            .unwrap_or(0)
    } else {
        0
    };
    let buff_avoidability = i32::from(character_buff_value(
        active_buffs,
        openstory_protocol::BuffMaskGroup::Second,
        0x0000_0020_0000_0000,
    ));
    forced_stats.avoidability.map_or_else(
        || {
            equipment_avoidability
                .saturating_add(i32::from(character.stats.luck) / 2)
                .saturating_add(i32::from(character.stats.dexterity) / 4)
                .saturating_add(
                    angel_blessing_avoidability.clamp(0, i32::MAX as i64) as i32,
                )
                .saturating_add(buff_avoidability)
        },
        i32::from,
    )
}

fn combat_hit_chance(
    accuracy: i32,
    attacker_level: i32,
    target_level: i32,
    avoidability: f64,
) -> f64 {
    let level_delta = target_level.saturating_sub(attacker_level).max(0) as f32;
    f64::from(
        (accuracy as f32
            / ((1.84_f32 + 0.07_f32 * level_delta) * avoidability.max(0.0) as f32 + 1.0))
            .clamp(0.01, 1.0),
    )
}

fn basic_attack_stats(
    character: &openstory_protocol::CharacterInfo,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
    archive: &NxArchive,
    item_archive: &NxArchive,
    skill_archive: &NxArchive,
    skills: &HashMap<i32, openstory_protocol::SkillEntry>,
    active_buffs: &CharacterBuffs,
    forced_stats: &ForcedEquipmentStats,
) -> Result<Option<BasicAttackStats>, String> {
    let weapon_id = equipped_weapon_id(&character.look);
    let Some(weapon_id) = weapon_id else {
        return Ok(None);
    };
    let weapon_type = weapon_id / 10_000;
    let weapon_path = format!("Weapon/0{weapon_id}.img");
    let weapon_image = archive
        .root()
        .resolve(&weapon_path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("weapon node {weapon_path:?} was not found"))?;
    let weapon_info = weapon_image
        .child("info")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("weapon {weapon_id} has no info node"))?;
    let attack_class = integer_property(&weapon_info, "attack")
        .and_then(|value| u8::try_from(value).ok())
        .filter(|value| *value != 0)
        .unwrap_or_else(|| fallback_weapon_attack_class(weapon_type));
    let weapon_attack_speed = integer_property(&weapon_info, "attackSpeed")
        .and_then(|value| i8::try_from(value).ok())
        .filter(|value| *value != 0)
        .unwrap_or(6);
    let booster = character_buff_value(
        active_buffs,
        openstory_protocol::BuffMaskGroup::Second,
        0x0000_0800_0000_0000,
    ) as i8;
    let attack_speed = weapon_attack_speed.saturating_add(booster);
    let projectile_prefix = weapon_projectile_prefix(weapon_type);
    let projectile = inventory.and_then(|inventory| {
        let prefix = projectile_prefix?;
        inventory
            .items
            .iter()
            .filter(|item| {
                item.inventory_type == openstory_protocol::InventoryType::Use
                    && item.count > 0
                    && item.item_id / 1_000 == prefix
            })
            .min_by_key(|item| item.slot)
    });
    let attack_type = if projectile.is_some() {
        openstory_protocol::AttackType::Ranged
    } else {
        openstory_protocol::AttackType::Close
    };

    let mut strength = i64::from(character.stats.strength);
    let mut dexterity = i64::from(character.stats.dexterity);
    let mut intelligence = i64::from(character.stats.intelligence);
    let mut luck = i64::from(character.stats.luck);
    let mut weapon_attack = 0_i64;
    let mut magic_attack = 0_i64;
    let mut accuracy_bonus = 0_i64;
    let mut maximum_hp = i64::from(character.stats.max_hp);
    let equipped_stats = inventory.and_then(|inventory| {
        let mut totals = [0_i64; 15];
        let mut found = false;
        for item in inventory.equipped() {
            let Some(equip) = item.equip.as_ref() else {
                continue;
            };
            found = true;
            for (total, value) in totals.iter_mut().zip(equip.stats) {
                *total = total.saturating_add(i64::from(value));
            }
        }
        found.then_some(totals)
    });
    if let Some(equipped_stats) = equipped_stats {
        // Upstream totals equipped-item stats from the inventory records. Those
        // values include scrolls and upgrades, while NX `info` only describes
        // the base item.
        strength = strength.saturating_add(equipped_stats[0]).min(999);
        dexterity = dexterity.saturating_add(equipped_stats[1]).min(999);
        intelligence = intelligence.saturating_add(equipped_stats[2]).min(999);
        luck = luck.saturating_add(equipped_stats[3]).min(999);
        maximum_hp = maximum_hp.saturating_add(equipped_stats[4]).min(500_000);
        weapon_attack = equipped_stats[6].min(999);
        magic_attack = equipped_stats[7].min(2_000);
        accuracy_bonus = equipped_stats[10];
    } else {
        let mut item_ids = character
            .look
            .equips
            .iter()
            .map(|(_, item_id)| *item_id)
            .collect::<HashSet<_>>();
        item_ids.insert(weapon_id);
        for item_id in item_ids {
            let Some(category) = character_equip_category(item_id) else {
                continue;
            };
            let info_path = format!("{category}/0{item_id}.img/info");
            let Some(info) = archive
                .root()
                .resolve(&info_path)
                .map_err(|error| error.to_string())?
            else {
                continue;
            };
            strength = strength.saturating_add(integer_property(&info, "incSTR").unwrap_or(0));
            dexterity = dexterity.saturating_add(integer_property(&info, "incDEX").unwrap_or(0));
            intelligence =
                intelligence.saturating_add(integer_property(&info, "incINT").unwrap_or(0));
            luck = luck.saturating_add(integer_property(&info, "incLUK").unwrap_or(0));
            weapon_attack =
                weapon_attack.saturating_add(integer_property(&info, "incPAD").unwrap_or(0));
            magic_attack =
                magic_attack.saturating_add(integer_property(&info, "incMAD").unwrap_or(0));
            accuracy_bonus =
                accuracy_bonus.saturating_add(integer_property(&info, "incACC").unwrap_or(0));
            maximum_hp = maximum_hp.saturating_add(integer_property(&info, "incMHP").unwrap_or(0));
        }
        strength = strength.min(999);
        dexterity = dexterity.min(999);
        intelligence = intelligence.min(999);
        luck = luck.min(999);
        maximum_hp = maximum_hp.min(500_000);
        weapon_attack = weapon_attack.min(999);
        magic_attack = magic_attack.min(2_000);
    }
    // Cosmic caps the pre-buff total (base INT + equipment INT/MATK) at
    // 2,000. Maple Warrior changes total INT but intentionally does not
    // change this magic subtotal.
    let magic_base_intelligence = intelligence;
    let mut magic_total = magic_base_intelligence
        .saturating_add(magic_attack)
        .min(2000);
    if let Some(projectile) = projectile {
        let projectile_prefix = projectile.item_id / 10_000;
        let projectile_node = format!("0{}", projectile.item_id);
        let projectile_info_path =
            format!("Consume/0{projectile_prefix}.img/{projectile_node}/info");
        if let Some(info) = item_archive
            .root()
            .resolve(&projectile_info_path)
            .map_err(|error| error.to_string())?
        {
            weapon_attack =
                weapon_attack.saturating_add(integer_property(&info, "incPAD").unwrap_or(0));
        }
    }
    weapon_attack = weapon_attack.min(999);

    let now = Instant::now();
    let buff_value = |group, flag| {
        active_buffs
            .get_active(&character_buff_key(group, flag), now)
            .map_or(0_i64, |buff| i64::from(buff.value))
    };
    // CharStats adds accuracy derived from the unmodified DEX/LUK totals before
    // applying percentage buffs such as Maple Warrior.
    let accuracy_dexterity = dexterity;
    let accuracy_luck = luck;
    let maple_warrior = buff_value(openstory_protocol::BuffMaskGroup::Second, 0x8);
    if maple_warrior != 0 {
        let apply_percent = |value: i64| {
            value.saturating_add(value.saturating_mul(maple_warrior) / 100)
                .min(999)
        };
        strength = apply_percent(strength);
        dexterity = apply_percent(dexterity);
        intelligence = apply_percent(intelligence);
        luck = apply_percent(luck);
    }
    weapon_attack = weapon_attack
        .saturating_add(buff_value(
            openstory_protocol::BuffMaskGroup::Second,
            0x0000_0001_0000_0000,
        ))
        .min(999);
    let magic_attack_buff = buff_value(
        openstory_protocol::BuffMaskGroup::Second,
        0x0000_0004_0000_0000,
    );
    magic_total = magic_total.saturating_add(magic_attack_buff);

    let job_group = character.stats.job / 100;
    let (primary, secondary) = match job_group {
        2 | 6..=8 => (intelligence, luck),
        3 => (dexterity, strength),
        4 => (luck, dexterity),
        5 if weapon_type == 149 => (dexterity, strength),
        5 => (strength, dexterity),
        _ => (strength, dexterity),
    };
    let multiplier = weapon_damage_multiplier(weapon_type);
    let mut mastery = 0.0;
    let mastery_skill = match weapon_type {
        130 | 140 => Some(1_100_000),
        131 | 141 => Some(1_100_001),
        132 | 142 => Some(1_200_001),
        143 => Some(1_300_000),
        144 => Some(1_300_001),
        _ => None,
    };
    if let Some(skill_id) = mastery_skill
        && character_can_use_skill(character.stats.job, skill_id)
        && let Some(level) = learned_skill_level(skill_archive, skills, skill_id)
    {
        mastery = 0.5 + integer_property(&level, "mastery").unwrap_or(0) as f64 / 100.0;
        accuracy_bonus = accuracy_bonus.saturating_add(integer_property(&level, "x").unwrap_or(0));
    }

    // Angel's Blessing contributes weapon/magic attack and accuracy.
    if let Some(level) = learned_skill_level(skill_archive, skills, 12) {
        weapon_attack = weapon_attack
            .saturating_add(integer_property(&level, "x").unwrap_or(0))
            .min(999);
        let blessing_magic = integer_property(&level, "y").unwrap_or(0);
        magic_total = magic_total.saturating_add(blessing_magic);
        accuracy_bonus = accuracy_bonus.saturating_add(integer_property(&level, "z").unwrap_or(0));
    }
    accuracy_bonus = accuracy_bonus.saturating_add(buff_value(
        openstory_protocol::BuffMaskGroup::Second,
        0x0000_0010_0000_0000,
    ));
    accuracy_bonus = accuracy_bonus.min(999);
    if let Some(value) = forced_stats.weapon_attack {
        weapon_attack = i64::from(value);
    }
    if let Some(value) = forced_stats.magic_attack {
        magic_total = magic_base_intelligence
            .saturating_add(i64::from(value))
            .min(2000);
    }

    // Berserk's damage bonus is active only below its per-level HP threshold.
    let damage_percent = character_can_use_skill(character.stats.job, 1_320_006)
        .then(|| learned_skill_level(skill_archive, skills, 1_320_006))
        .flatten()
        .filter(|level| {
            let threshold = integer_property(level, "x").unwrap_or(0).max(0);
            i64::from(character.stats.hp) * 100 <= maximum_hp.saturating_mul(threshold)
        })
        .map_or(0.0, |level| {
            integer_property(&level, "damage").unwrap_or(0) as f64 / 100.0
        });
    let primary = (primary as f32 * multiplier as f32) as i64;
    let damage_factor = weapon_attack as f32 / 100.0 + damage_percent as f32;
    let minimum_damage = (((primary as f32 * 0.9 * mastery as f32) + secondary as f32)
        * damage_factor) as i32 as f64;
    let maximum_damage =
        ((primary as f32 + secondary as f32) * damage_factor) as i32 as f64;
    let accuracy = forced_stats.accuracy.map_or_else(
        || {
            let accuracy = ((accuracy_dexterity as f32 * 0.8 + accuracy_luck as f32 * 0.5) as i32)
                .saturating_add(i32::try_from(accuracy_bonus).unwrap_or(0));
            accuracy
        },
        i32::from,
    );

    Ok(Some(BasicAttackStats {
        weapon_type,
        attack_type,
        projectile_id: projectile.map(|projectile| projectile.item_id),
        attack_class,
        speed: attack_speed,
        level: i32::from(character.stats.level),
        minimum_damage,
        maximum_damage,
        magic_total,
        magic_damage: false,
        fixed_damage: None,
        accuracy,
        critical: 0.05,
    }))
}

struct StatusDetailCombatValues {
    attack_range: Option<(i32, i32)>,
    magic: i32,
    accuracy: i32,
    speed: i32,
    jump: i32,
}

fn status_detail_combat_values(
    character: &openstory_protocol::CharacterInfo,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    skill_archive: &NxArchive,
    skills: &HashMap<i32, openstory_protocol::SkillEntry>,
    active_buffs: &CharacterBuffs,
    forced_stats: &ForcedEquipmentStats,
) -> StatusDetailCombatValues {
    let attack = basic_attack_stats(
        character,
        inventory,
        character_archive,
        item_archive,
        skill_archive,
        skills,
        active_buffs,
        forced_stats,
    )
    .ok()
    .flatten();
    let clamp = |value: f64| value.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32;
    let gear_accuracy = inventory.map_or(0_i32, |inventory| {
        inventory
            .equipped()
            .filter_map(|item| item.equip.as_ref())
            .map(|equip| i32::from(equip.stats[10]))
            .sum()
    });
    let gear_magic = inventory.map_or(0_i32, |inventory| {
        inventory
            .equipped()
            .filter_map(|item| item.equip.as_ref())
            .map(|equip| i32::from(equip.stats[7]))
            .sum()
    });
    let accuracy = attack.map_or_else(
        || {
            forced_stats.accuracy.map_or_else(
                || {
                    ((f64::from(character.stats.dexterity) * 0.8
                        + f64::from(character.stats.luck) * 0.5)
                        as i32)
                        .saturating_add(gear_accuracy)
                },
                i32::from,
            )
        },
        |attack| attack.accuracy,
    );
    let (_, jump_force, speed_multiplier) =
        player_movement_forces(character, character_archive, active_buffs, forced_stats, false);
    StatusDetailCombatValues {
        attack_range: attack.map(|attack| {
            (
                clamp(attack.minimum_damage),
                clamp(attack.maximum_damage),
            )
        }),
        magic: attack.map_or_else(
            || {
                i32::from(character.stats.intelligence)
                    .saturating_add(forced_stats.magic_attack.map_or(gear_magic, i32::from))
            },
            |attack| attack.magic_total.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
        ),
        accuracy,
        speed: clamp(speed_multiplier * 100.0),
        jump: clamp((jump_force - 1.0) * 100.0 / 3.5),
    }
}

fn basic_attack_damage(
    scene: &mut MapSceneRuntime,
    attack: BasicAttackStats,
    mob: MobTraits,
) -> (i32, bool) {
    if let Some(damage) = attack.fixed_damage {
        return (damage.clamp(1, 999_999), false);
    }
    let level_delta = (mob.level - attack.level).max(0) as f64;
    let (min_damage, max_damage) = if attack.magic_damage {
        (
            (attack.minimum_damage - (1.0 + 0.01 * level_delta) * mob.magic_defense * 0.6).max(1.0),
            (attack.maximum_damage - (1.0 + 0.01 * level_delta) * mob.magic_defense * 0.5).max(1.0),
        )
    } else {
        (
            (attack.minimum_damage * (1.0 - 0.01 * level_delta) - mob.physical_defense * 0.6)
                .max(1.0),
            (attack.maximum_damage * (1.0 - 0.01 * level_delta) - mob.physical_defense * 0.5)
                .max(1.0),
        )
    };
    let hit_chance = combat_hit_chance(attack.accuracy, attack.level, mob.level, mob.avoid);
    if scene.next_attack_random() >= hit_chance {
        return (0, false);
    }
    let mut damage = min_damage + (max_damage - min_damage) * scene.next_attack_random();
    let critical = scene.next_attack_random() < attack.critical;
    if critical {
        damage *= 1.5;
    }
    (damage.clamp(1.0, 999_999.0) as i32, critical)
}

fn character_weapon_attack_stances(
    archive: &NxArchive,
    look: &openstory_protocol::CharacterLook,
) -> Result<&'static [(u8, &'static str)], String> {
    const ONE_HANDED: &[(u8, &str)] = &[
        (15, "stabO1"),
        (16, "stabO2"),
        (17, "stabOF"),
        (23, "swingO1"),
        (24, "swingO2"),
        (25, "swingO3"),
        (26, "swingOF"),
    ];
    const SPEAR_POLEARM: &[(u8, &str)] = &[
        (18, "stabT1"),
        (20, "stabTF"),
        (27, "swingP1"),
        (29, "swingPF"),
    ];
    const BOW: &[(u8, &str)] = &[
        (11, "shoot1"),
        (13, "shootF"),
        (30, "swingT1"),
        (32, "swingT3"),
    ];
    const CROSSBOW: &[(u8, &str)] = &[
        (12, "shoot2"),
        (13, "shootF"),
        (18, "stabT1"),
        (30, "swingT1"),
    ];
    const TWO_HANDED: &[(u8, &str)] = &[
        (15, "stabO1"),
        (16, "stabO2"),
        (17, "stabOF"),
        (30, "swingT1"),
        (31, "swingT2"),
        (32, "swingT3"),
        (33, "swingTF"),
    ];
    const MAGIC: &[(u8, &str)] = &[(23, "swingO1"), (24, "swingO2"), (26, "swingOF")];
    const CLAW: &[(u8, &str)] = &[(23, "swingO1"), (24, "swingO2"), (26, "swingOF")];
    const GUN: &[(u8, &str)] = &[(10, "shot"), (19, "stabT2"), (27, "swingP1")];

    let Some(weapon_id) = character_weapon_id(look) else {
        return Ok(&[]);
    };
    let info_path = format!("Weapon/0{weapon_id}.img/info");
    let attack_type = archive
        .root()
        .resolve(&info_path)
        .map_err(|error| error.to_string())?
        .and_then(|info| integer_property(&info, "attack"))
        .and_then(|value| u8::try_from(value).ok())
        .filter(|value| *value != 0)
        .unwrap_or_else(|| match weapon_id / 10_000 {
            130..=133 => 1,
            143..=144 => 2,
            145 => 3,
            146 => 4,
            140..=142 => 5,
            137..=138 => 6,
            147..=148 => 7,
            149 => 9,
            _ => 0,
        });
    Ok(match attack_type {
        1 => ONE_HANDED,
        2 => SPEAR_POLEARM,
        3 => BOW,
        4 => CROSSBOW,
        5 => TWO_HANDED,
        6 => MAGIC,
        7 => CLAW,
        9 => GUN,
        _ => &[],
    })
}

fn is_two_handed_weapon(item_id: i32) -> bool {
    matches!(item_id / 10_000, 138 | 140..=144 | 146)
}
