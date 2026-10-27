fn append_character_animations(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    look: &openstory_protocol::CharacterLook,
    anchor_x: f32,
    anchor_y: f32,
    cache: &mut MapArtCache,
) -> Result<Vec<AvatarFrameAsset>, String> {
    let mut frames = Vec::new();
    let two_handed = character_weapon_id(look).is_some_and(is_two_handed_weapon);
    let (stand_stance, walk_stance) = if two_handed {
        ("stand2", "walk2")
    } else {
        ("stand1", "walk1")
    };
    let mut stance_motions = vec![
        (stand_stance, AvatarMotion::Stand),
        (walk_stance, AvatarMotion::Walk),
        ("alert", AvatarMotion::Alert),
        ("prone", AvatarMotion::Crouch),
        ("jump", AvatarMotion::Jump),
        ("fly", AvatarMotion::Swim),
        ("ladder", AvatarMotion::Ladder),
        ("rope", AvatarMotion::Rope),
        ("sit", AvatarMotion::Sit),
    ];
    for &(stance_id, stance_name) in character_weapon_attack_stances(archive, look)? {
        stance_motions.push((stance_name, AvatarMotion::Attack(stance_id)));
    }
    for (stance, motion) in stance_motions {
        let stance_path = format!("000020{:02}.img/{stance}", look.skin);
        let Some(stance_node) = archive
            .root()
            .resolve(&stance_path)
            .map_err(|error| error.to_string())?
        else {
            continue;
        };
        for frame_node in numeric_children(&stance_node)? {
            let frame_id = frame_node
                .name()
                .map_err(|error| error.to_string())?
                .parse::<u8>()
                .map_err(|error| format!("invalid avatar frame id: {error}"))?;
            let first_sprite = sprites.len();
            let (delay_ms, face_sprite, face_offset) = append_character_avatar(
                sprites, archive, look, stance, frame_id, anchor_x, anchor_y, cache,
            )?;
            if sprites.len() > first_sprite {
                frames.push(AvatarFrameAsset {
                    motion,
                    delay_ms,
                    sprites: first_sprite..sprites.len(),
                    face_sprite,
                    face_offset,
                });
            }
        }
    }
    Ok(frames)
}

fn append_character_preview(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    look: &openstory_protocol::CharacterLook,
    anchor_x: f32,
    anchor_y: f32,
    scale: f32,
    facing_right: bool,
    cache: &mut MapArtCache,
) -> Result<(), String> {
    let stance = if character_weapon_id(look).is_some_and(is_two_handed_weapon) {
        "stand2"
    } else {
        "stand1"
    };
    let first_sprite = sprites.len();
    append_character_avatar(sprites, archive, look, stance, 0, anchor_x, anchor_y, cache)?;
    for sprite in &mut sprites[first_sprite..] {
        sprite.rect.x = anchor_x + (sprite.rect.x - anchor_x) * scale;
        sprite.rect.y = anchor_y + (sprite.rect.y - anchor_y) * scale;
        sprite.rect.width *= scale;
        sprite.rect.height *= scale;
        if facing_right {
            sprite.rect.x = 2.0 * anchor_x - (sprite.rect.x + sprite.rect.width);
            sprite.pixels = prepare_map_pixels(
                &sprite.pixels,
                sprite.texture_width,
                sprite.texture_height,
                255,
                true,
            );
            sprite.texture_key.push_str(":mirror-right");
        }
    }
    Ok(())
}

fn character_weapon_id(look: &openstory_protocol::CharacterLook) -> Option<i32> {
    look.masked_equips
        .iter()
        .chain(look.equips.iter())
        .map(|(_, item_id)| *item_id)
        .find(|item_id| (130..=170).contains(&(item_id / 10_000)))
        .or_else(|| (look.weapon_id > 0).then_some(look.weapon_id))
}

fn equipped_mount_id(look: &openstory_protocol::CharacterLook) -> i32 {
    look.equips
        .iter()
        .find(|(slot, _)| slot.unsigned_abs() == 18)
        .map_or(0, |(_, item_id)| *item_id)
}

fn equipped_ring_effect_id(
    look: &openstory_protocol::CharacterLook,
    effect_archive: &NxArchive,
) -> i32 {
    look.equips
        .iter()
        .filter(|(slot, _)| matches!(slot.unsigned_abs(), 12 | 13 | 15 | 16))
        .map(|(_, item_id)| *item_id)
        .find(|item_id| {
            effect_archive
                .root()
                .resolve(&format!("ItemEff.img/{item_id}"))
                .ok()
                .flatten()
                .is_some()
        })
        .unwrap_or(0)
}

fn is_weapon_item(item_id: i32) -> bool {
    matches!(item_id / 10_000, 130..=133 | 137..=138 | 140..=149)
}

fn equipped_weapon_id(look: &openstory_protocol::CharacterLook) -> Option<i32> {
    look.equips
        .iter()
        .map(|(_, item_id)| *item_id)
        .find(|item_id| is_weapon_item(*item_id))
        .or_else(|| is_weapon_item(look.weapon_id).then_some(look.weapon_id))
}

fn is_valid_weapon_type(weapon_type: i32) -> bool {
    matches!(weapon_type, 130..=133 | 137..=138 | 140..=149 | 170)
}

fn weapon_projectile_prefix(weapon_type: i32) -> Option<i32> {
    match weapon_type {
        145 => Some(2060),
        146 => Some(2061),
        147 => Some(2070),
        149 => Some(2330),
        _ => None,
    }
}

fn weapon_projectile_count(
    inventory: Option<&openstory_protocol::InventorySnapshot>,
    weapon_type: i32,
) -> u16 {
    let Some(prefix) = weapon_projectile_prefix(weapon_type) else {
        return 0;
    };
    inventory
        .into_iter()
        .flat_map(|inventory| inventory.items.iter())
        .filter(|item| {
            item.inventory_type == openstory_protocol::InventoryType::Use
                && item.count > 0
                && item.item_id / 1_000 == prefix
        })
        .min_by_key(|item| item.slot)
        .map_or(0, |item| item.count)
}
