fn skill_job_can_use(job_id: u16, skill_id: i32) -> bool {
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

fn skill_use_block_reason(
    skill: &SkillUiEntry,
    character: &openstory_protocol::CharacterInfo,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
) -> Option<&'static str> {
    if skill.level > skill.master_level {
        return Some("You have not mastered that skill yet.");
    }
    if !skill_job_can_use(character.stats.job, skill.id) {
        return Some("Your job cannot use that skill.");
    }
    if i32::from(character.stats.hp) <= i32::from(skill.hp_cost) {
        return Some("You do not have enough HP to use that skill.");
    }
    if character.stats.mp < skill.mp_cost {
        return Some("You do not have enough MP to use that skill.");
    }
    let weapon_type = equipped_weapon_id(&character.look).map(|item_id| item_id / 10_000);
    if let Some(required_weapon) = skill.required_weapon
        && weapon_type != Some(required_weapon)
    {
        return Some("You cannot use that skill with your current weapon.");
    }
    if let Some(weapon_type) = weapon_type
        && weapon_projectile_prefix(weapon_type).is_some()
        && weapon_projectile_count(inventory, weapon_type) < skill.bullet_cost
    {
        return Some("You do not have enough ammunition to use that skill.");
    }
    None
}

fn character_buff_value(
    active_buffs: &HashMap<openstory_protocol::BuffStatKey, ActiveCharacterBuff>,
    group: openstory_protocol::BuffMaskGroup,
    flag: u64,
) -> i16 {
    let now = Instant::now();
    active_buffs
        .get(&openstory_protocol::BuffStatKey { group, flag })
        .filter(|buff| buff.expires_at.is_none_or(|expires_at| expires_at > now))
        .map_or(0, |buff| buff.value)
}

fn character_has_buff(
    active_buffs: &HashMap<openstory_protocol::BuffStatKey, ActiveCharacterBuff>,
    group: openstory_protocol::BuffMaskGroup,
    flag: u64,
) -> bool {
    character_buff_value(active_buffs, group, flag) != 0
}

fn character_has_mob_skill_buff(
    active_buffs: &HashMap<openstory_protocol::BuffStatKey, ActiveCharacterBuff>,
    group: openstory_protocol::BuffMaskGroup,
    flag: u64,
    mob_skill_type: u16,
) -> bool {
    let now = Instant::now();
    active_buffs
        .get(&openstory_protocol::BuffStatKey { group, flag })
        .filter(|buff| buff.expires_at.is_none_or(|expires_at| expires_at > now))
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
    active_buffs: &HashMap<openstory_protocol::BuffStatKey, ActiveCharacterBuff>,
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
    attack: i32,
    archive: &NxArchive,
    skill_archive: &NxArchive,
    skills: &HashMap<i32, openstory_protocol::SkillEntry>,
    active_buffs: &HashMap<openstory_protocol::BuffStatKey, ActiveCharacterBuff>,
    forced_stats: &ForcedEquipmentStats,
) -> i32 {
    let mut weapon_defense = 0_i64;
    let item_ids = character
        .look
        .equips
        .iter()
        .map(|(_, item_id)| *item_id)
        .collect::<HashSet<_>>();
    for item_id in item_ids {
        let Some(category) = character_equip_category(item_id) else {
            continue;
        };
        let info_path = format!("{category}/0{item_id}.img/info");
        if let Some(info) = archive.root().resolve(&info_path).ok().flatten() {
            weapon_defense =
                weapon_defense.saturating_add(integer_property(&info, "incWDEF").unwrap_or(0));
        }
    }

    weapon_defense = weapon_defense.saturating_add(i64::from(character_buff_value(
        active_buffs,
        openstory_protocol::BuffMaskGroup::Second,
        0x0000_0002_0000_0000,
    )));
    if let Some(value) = forced_stats.weapon_defense {
        weapon_defense = i64::from(value);
    }

    let achilles = [1_120_004, 1_220_005, 1_320_005]
        .into_iter()
        .find_map(|skill_id| {
            learned_skill_level(skill_archive, skills, skill_id)
                .and_then(|level| integer_property(&level, "x"))
        })
        .unwrap_or(0)
        .clamp(0, 1000) as f64
        / 1000.0;

    let damage_before_achilles = if weapon_defense <= 0 {
        attack
    } else {
        attack / 2 + i32::try_from(i64::from(attack) / weapon_defense).unwrap_or(0)
    };
    damage_before_achilles.saturating_sub((f64::from(damage_before_achilles) * achilles) as i32)
}

fn basic_attack_stats(
    character: &openstory_protocol::CharacterInfo,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
    archive: &NxArchive,
    item_archive: &NxArchive,
    skill_archive: &NxArchive,
    skills: &HashMap<i32, openstory_protocol::SkillEntry>,
    active_buffs: &HashMap<openstory_protocol::BuffStatKey, ActiveCharacterBuff>,
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
        intelligence = intelligence.saturating_add(integer_property(&info, "incINT").unwrap_or(0));
        luck = luck.saturating_add(integer_property(&info, "incLUK").unwrap_or(0));
        weapon_attack =
            weapon_attack.saturating_add(integer_property(&info, "incPAD").unwrap_or(0));
        magic_attack = magic_attack.saturating_add(integer_property(&info, "incMAD").unwrap_or(0));
        accuracy_bonus =
            accuracy_bonus.saturating_add(integer_property(&info, "incACC").unwrap_or(0));
        maximum_hp = maximum_hp.saturating_add(integer_property(&info, "incMHP").unwrap_or(0));
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

    let now = Instant::now();
    let buff_value = |group, flag| {
        active_buffs
            .get(&openstory_protocol::BuffStatKey { group, flag })
            .filter(|buff| buff.expires_at.is_none_or(|expires_at| expires_at > now))
            .map_or(0_i64, |buff| i64::from(buff.value))
    };
    let maple_warrior = buff_value(openstory_protocol::BuffMaskGroup::Second, 0x8);
    if maple_warrior != 0 {
        let multiplier = 1.0 + maple_warrior as f64 / 100.0;
        strength = (strength as f64 * multiplier) as i64;
        dexterity = (dexterity as f64 * multiplier) as i64;
        intelligence = (intelligence as f64 * multiplier) as i64;
        luck = (luck as f64 * multiplier) as i64;
    }
    weapon_attack = weapon_attack.saturating_add(buff_value(
        openstory_protocol::BuffMaskGroup::Second,
        0x0000_0001_0000_0000,
    ));
    let magic_attack_buff = buff_value(
        openstory_protocol::BuffMaskGroup::Second,
        0x0000_0004_0000_0000,
    );
    magic_total = magic_total.saturating_add(magic_attack_buff);
    accuracy_bonus = accuracy_bonus.saturating_add(buff_value(
        openstory_protocol::BuffMaskGroup::Second,
        0x0000_0010_0000_0000,
    ));
    let hyperbody_hp = buff_value(
        openstory_protocol::BuffMaskGroup::Second,
        0x0000_2000_0000_0000,
    );
    if hyperbody_hp > 0 {
        maximum_hp = maximum_hp.saturating_add(maximum_hp.saturating_mul(hyperbody_hp) / 100);
    }

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
        && let Some(level) = learned_skill_level(skill_archive, skills, skill_id)
    {
        mastery = 0.5 + integer_property(&level, "mastery").unwrap_or(0) as f64 / 100.0;
        accuracy_bonus = accuracy_bonus.saturating_add(integer_property(&level, "x").unwrap_or(0));
    }

    // Angel's Blessing contributes weapon/magic attack and accuracy.
    if let Some(level) = learned_skill_level(skill_archive, skills, 12) {
        weapon_attack = weapon_attack.saturating_add(integer_property(&level, "x").unwrap_or(0));
        let blessing_magic = integer_property(&level, "y").unwrap_or(0);
        magic_total = magic_total.saturating_add(blessing_magic);
        accuracy_bonus = accuracy_bonus.saturating_add(integer_property(&level, "z").unwrap_or(0));
    }
    if let Some(value) = forced_stats.weapon_attack {
        weapon_attack = i64::from(value);
    }
    if let Some(value) = forced_stats.magic_attack {
        magic_total = magic_base_intelligence
            .saturating_add(i64::from(value))
            .min(2000);
    }

    // Berserk's damage bonus is active only below its per-level HP threshold.
    let damage_percent = learned_skill_level(skill_archive, skills, 1_320_006)
        .filter(|level| {
            let threshold = integer_property(level, "x").unwrap_or(0).max(0);
            i64::from(character.stats.hp) * 100 <= maximum_hp.saturating_mul(threshold)
        })
        .map_or(0.0, |level| {
            integer_property(&level, "damage").unwrap_or(0) as f64 / 100.0
        });
    let damage_factor = weapon_attack.max(0) as f64 / 100.0 + damage_percent;
    let mut minimum_damage =
        ((primary as f64 * 0.9 * mastery) + secondary as f64) * multiplier * damage_factor;
    let mut maximum_damage = (primary as f64 + secondary as f64) * multiplier * damage_factor;
    // Bow, crossbow, claw, and gun attacks use the legacy close-range fallback
    // when the matching projectile stack is empty.
    if matches!(weapon_type, 145 | 146 | 147 | 149) && projectile.is_none() {
        minimum_damage /= 10.0;
        maximum_damage /= 10.0;
    }

    let accuracy = forced_stats.accuracy.map_or_else(
        || {
            let mut accuracy = ((dexterity as f64 * 0.8 + luck as f64 * 0.5) as i32)
                .saturating_add(i32::try_from(accuracy_bonus).unwrap_or(0));
            if buff_value(openstory_protocol::BuffMaskGroup::Second, STATUS_DARKNESS) != 0 {
                accuracy /= 2;
            }
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
        intelligence,
        luck,
        magic_total,
        magic_damage: false,
        element: None,
        fixed_damage: None,
        accuracy,
        critical: 0.05,
    }))
}

fn fallback_weapon_attack_class(weapon_type: i32) -> u8 {
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

fn weapon_damage_multiplier(weapon_type: i32) -> f64 {
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

fn regular_attack_stance(stats: BasicAttackStats, attack_count: u32) -> Option<u8> {
    let degenerate = stats.attack_type != openstory_protocol::AttackType::Ranged
        && matches!(stats.weapon_type, 145 | 146 | 147 | 149);
    let stances: &[u8] = if degenerate {
        match stats.attack_class {
            3 => &[30, 32],
            4 | 7 => &[30, 18],
            9 => &[27, 19],
            _ => &[],
        }
    } else {
        match stats.attack_class {
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

fn element_index(element: char) -> Option<usize> {
    match element.to_ascii_lowercase() {
        'f' => Some(0),
        'i' => Some(1),
        'l' => Some(2),
        's' => Some(3),
        'h' => Some(4),
        'p' => Some(5),
        _ => None,
    }
}

fn parse_elemental_attributes(value: &str) -> [u8; 6] {
    let mut attributes = [1; 6];
    let mut pairs = value.chars();
    while let Some(element) = pairs.next() {
        let Some(attribute) = pairs.next().and_then(|value| value.to_digit(10)) else {
            break;
        };
        if let Some(index) = element_index(element) {
            attributes[index] = attribute as u8;
        }
    }
    attributes
}

fn basic_attack_damage(
    scene: &mut MapSceneRuntime,
    attack: BasicAttackStats,
    mob: MobTraits,
) -> i32 {
    if let Some(damage) = attack.fixed_damage {
        return damage;
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
    // v83 elemAttr values are immune, neutral, strong, and weak at 0, 1, 2, and 3.
    let elemental_multiplier = attack
        .element
        .and_then(|element| element_index(element).map(|index| mob.elemental_attributes[index]))
        .map_or(1.0, |attribute| match attribute {
            0 => 0.0,
            2 => 0.5,
            3 => 1.5,
            _ => 1.0,
        });
    let min_damage = (min_damage * elemental_multiplier).max(1.0);
    let max_damage = (max_damage * elemental_multiplier).max(1.0);
    let hit_chance =
        (f64::from(attack.accuracy) / ((1.84 + 0.07 * level_delta) * mob.avoid + 1.0)).max(0.01);
    if scene.next_attack_random() >= hit_chance {
        return 0;
    }
    let mut damage = min_damage + (max_damage - min_damage) * scene.next_attack_random();
    if scene.next_attack_random() < attack.critical {
        damage *= 1.5;
    }
    damage.clamp(1.0, 999_999.0) as i32
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
