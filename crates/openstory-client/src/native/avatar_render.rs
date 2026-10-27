fn append_character_avatar(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    look: &openstory_protocol::CharacterLook,
    stance: &str,
    frame: u8,
    anchor_x: f32,
    anchor_y: f32,
    cache: &mut MapArtCache,
) -> Result<(u16, Option<usize>, (f32, f32)), String> {
    let avatar_start = sprites.len();
    let skin = look.skin;
    let body_path = format!("000020{skin:02}.img/{stance}/{frame}");
    let head_path = format!("000120{skin:02}.img/{stance}/{frame}/head");
    let Some(body_frame) = archive
        .root()
        .resolve(&body_path)
        .map_err(|error| error.to_string())?
    else {
        return Ok((100, None, (0.0, 0.0)));
    };
    let Some(body) = body_frame
        .child("body")
        .map_err(|error| error.to_string())?
    else {
        return Ok((100, None, (0.0, 0.0)));
    };
    let delay_ms = integer_property(&body_frame, "delay")
        .and_then(|delay| u16::try_from(delay).ok())
        .filter(|delay| *delay > 0)
        .unwrap_or(100);
    let body_position = vector_path(&body, "map/navel").unwrap_or((0.0, 0.0));
    let body_neck = vector_path(&body, "map/neck").unwrap_or((0.0, 0.0));

    let head = archive
        .root()
        .resolve(&head_path)
        .map_err(|error| error.to_string())?;
    let head_neck = head
        .as_ref()
        .and_then(|node| vector_path(node, "map/neck"))
        .unwrap_or((0.0, 0.0));
    let head_brow = head
        .as_ref()
        .and_then(|node| vector_path(node, "map/brow"))
        .unwrap_or((0.0, 0.0));
    let head_offset = (body_neck.0 - head_neck.0, body_neck.1 - head_neck.1);
    let face_offset = (head_offset.0 + head_brow.0, head_offset.1 + head_brow.1);
    let hair_path = format!("Hair/000{:05}.img/{stance}/{frame}", look.hair_id);
    let hair_frame = archive
        .root()
        .resolve(&hair_path)
        .map_err(|error| error.to_string())?;

    if let Some(hair_frame) = hair_frame.as_ref() {
        for part_name in [
            "backHair",
            "backHairBelowCap",
            "backHairBelowCapNarrow",
            "backHairBelowCapWide",
            "backHairOverCape",
            "hairBelowBody",
        ] {
            let first_sprite = sprites.len();
            append_hair_part(
                sprites,
                hair_frame,
                part_name,
                &hair_path,
                (anchor_x, anchor_y),
                face_offset,
                cache,
            )?;
            set_sprite_depth(
                sprites,
                first_sprite,
                match part_name {
                    "hairBelowBody" => 0,
                    "backHairOverCape" => 3,
                    "backHair" => 23,
                    "backHairBelowCap" | "backHairBelowCapNarrow" | "backHairBelowCapWide" => 23,
                    _ => 0,
                },
            );
        }
    }

    append_character_body_layers(
        sprites,
        &body_frame,
        &body_path,
        (anchor_x, anchor_y),
        body_position,
        cache,
    )?;

    if let Some(head) = head.as_ref() {
        let first_sprite = sprites.len();
        append_avatar_node(
            sprites,
            head,
            &head_path,
            (anchor_x + head_offset.0, anchor_y + head_offset.1),
            cache,
        )?;
        set_sprite_depth(sprites, first_sprite, 16);
    }

    if let Some(hair_frame) = hair_frame.as_ref() {
        for (part_name, depth) in [("hairShade", 17), ("hair", 18)] {
            let first_sprite = sprites.len();
            append_hair_part(
                sprites,
                hair_frame,
                part_name,
                &hair_path,
                (anchor_x, anchor_y),
                face_offset,
                cache,
            )?;
            set_sprite_depth(sprites, first_sprite, depth);
        }
    }

    let face_path = format!("Face/000{:05}.img/default/face", look.face_id);
    if let Some(face) = archive
        .root()
        .resolve(&face_path)
        .map_err(|error| error.to_string())?
    {
        // The face's map/brow point must land on the head's brow anchor.
        // C++ Face::Frame shifts the bitmap by this authored offset before
        // drawing; account for it here because append_avatar_node only
        // applies the bitmap origin.
        let face_brow = vector_path(&face, "map/brow").unwrap_or((0.0, 0.0));
        let first_sprite = sprites.len();
        append_avatar_node(
            sprites,
            &face,
            &face_path,
            (
                anchor_x + face_offset.0 - face_brow.0,
                anchor_y + face_offset.1 - face_brow.1,
            ),
            cache,
        )?;
        set_sprite_depth(sprites, first_sprite, 19);
    }

    if let Some(hair_frame) = hair_frame.as_ref() {
        let first_sprite = sprites.len();
        append_hair_part(
            sprites,
            hair_frame,
            "hairOverHead",
            &hair_path,
            (anchor_x, anchor_y),
            face_offset,
            cache,
        )?;
        set_sprite_depth(sprites, first_sprite, 24);
    }

    let anchors = avatar_equipment_anchors(&body_frame, body_position, face_offset)?;
    append_character_equipment(
        sprites, archive, look, stance, frame, anchor_x, anchor_y, anchors, cache,
    )?;
    sprites[avatar_start..].sort_by_key(|sprite| sprite.depth_order);
    let face_texture_key = map_texture_key(&format!("character:{face_path}"), 255, false);
    let face_sprite = sprites[avatar_start..]
        .iter()
        .position(|sprite| sprite.texture_key == face_texture_key)
        .map(|offset| avatar_start + offset);
    Ok((delay_ms, face_sprite, face_offset))
}

#[derive(Clone, Copy)]
struct AvatarEquipmentAnchors {
    body: (f32, f32),
    face: (f32, f32),
    hand: (f32, f32),
    hand_move: (f32, f32),
}

fn avatar_equipment_anchors(
    body_frame: &openstory_assets::NxNode,
    body_position: (f32, f32),
    face_position: (f32, f32),
) -> Result<AvatarEquipmentAnchors, String> {
    let mut arm_hand = None;
    let mut over_hair_hand = None;
    let mut hand_move = body_position;
    for part in body_frame.children().map_err(|error| error.to_string())? {
        let layer = string_property(&part, "z");
        match layer.as_deref() {
            Some("arm") | Some("armOverHair") => {
                let position = vector_path(&part, "map/hand")
                    .zip(vector_path(&part, "map/navel"))
                    .map(|(hand, navel)| {
                        (
                            body_position.0 + hand.0 - navel.0,
                            body_position.1 + hand.1 - navel.1,
                        )
                    });
                if layer.as_deref() == Some("arm") {
                    arm_hand = position;
                } else {
                    over_hair_hand = position;
                }
            }
            Some("handBelowWeapon") => {
                if let Some(position) = vector_path(&part, "map/handMove") {
                    hand_move = position;
                }
            }
            _ => {}
        }
    }
    Ok(AvatarEquipmentAnchors {
        body: body_position,
        face: face_position,
        hand: arm_hand.or(over_hair_hand).unwrap_or(body_position),
        hand_move,
    })
}

fn append_character_body_layers(
    sprites: &mut Vec<DecodedSprite>,
    body_frame: &openstory_assets::NxNode,
    body_path: &str,
    anchor: (f32, f32),
    body_position: (f32, f32),
    cache: &mut MapArtCache,
) -> Result<(), String> {
    for part in body_frame.children().map_err(|error| error.to_string())? {
        let part_name = part.name().unwrap_or_else(|_| "part".to_owned());
        if part_name == "delay" || part_name == "face" {
            continue;
        }
        if part.bitmap().map_err(|error| error.to_string())?.is_none() {
            continue;
        }
        let layer = string_property(&part, "z").unwrap_or_else(|| part_name.clone());
        let Some(depth) = body_layer_depth(&layer) else {
            continue;
        };
        let position = if layer == "handBelowWeapon" {
            vector_path(&part, "map/handMove").unwrap_or((0.0, 0.0))
        } else {
            vector_path(&part, "map/navel").unwrap_or((0.0, 0.0))
        };
        let sprite_index = sprites.len();
        let position = if layer == "handBelowWeapon" {
            position
        } else {
            (body_position.0 - position.0, body_position.1 - position.1)
        };
        append_avatar_node(
            sprites,
            &part,
            &format!("{body_path}/{part_name}"),
            (anchor.0 + position.0, anchor.1 + position.1),
            cache,
        )?;
        set_sprite_depth(sprites, sprite_index, depth);
    }
    Ok(())
}

fn body_layer_depth(layer: &str) -> Option<i16> {
    match layer {
        "body" | "backBody" => Some(6),
        "armBelowHead" => Some(10),
        "armBelowHeadOverMailChest" => Some(13),
        "arm" => Some(27),
        "handBelowWeapon" => Some(32),
        "armOverHair" => Some(33),
        "armOverHairBelowWeapon" => Some(34),
        "handOverHair" => Some(37),
        "handOverWeapon" => Some(38),
        "head" => Some(16),
        _ => None,
    }
}

fn append_character_equipment(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    look: &openstory_protocol::CharacterLook,
    stance: &str,
    frame: u8,
    anchor_x: f32,
    anchor_y: f32,
    anchors: AvatarEquipmentAnchors,
    cache: &mut MapArtCache,
) -> Result<(), String> {
    let mut equips = look.equips.clone();
    for &(slot, item_id) in &look.masked_equips {
        if let Some((_, current_id)) = equips
            .iter_mut()
            .find(|(current_slot, _)| *current_slot == slot)
        {
            *current_id = item_id;
        } else {
            equips.push((slot, item_id));
        }
    }
    if look.weapon_id > 0
        && !equips
            .iter()
            .any(|(_, item_id)| (130..=170).contains(&(item_id / 10_000)))
    {
        equips.push((11, look.weapon_id));
    }
    equips.sort_by_key(|(slot, item_id)| (*slot, *item_id));
    let two_handed = match stance {
        "stand1" | "walk1" => false,
        "stand2" | "walk2" => true,
        _ => character_weapon_id(look).is_some_and(is_two_handed_weapon),
    };

    for (_, item_id) in equips {
        if item_id <= 0 {
            continue;
        }
        let Some(category) = character_equip_category(item_id) else {
            continue;
        };
        let image_path = format!("{category}/0{item_id}.img");
        let Some((frame_path, frame_node)) =
            resolve_equipment_frame(archive, &image_path, stance, frame)?
        else {
            continue;
        };
        let part_nodes = if frame_node
            .bitmap()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            vec![frame_node.clone()]
        } else {
            frame_node.children().map_err(|error| error.to_string())?
        };
        let prefix = item_id / 10_000;
        for part in part_nodes {
            if part.bitmap().map_err(|error| error.to_string())?.is_none() {
                continue;
            }
            let part_name = part.name().unwrap_or_else(|_| "part".to_owned());
            let (parent, parent_position) =
                last_map_vector(&part)?.unwrap_or_else(|| (String::new(), (0.0, 0.0)));
            let attachment = match prefix {
                100..=103 => anchors.face,
                109 | 130..=170 => match parent.as_str() {
                    "handMove" => anchors.hand_move,
                    "hand" => anchors.hand,
                    "navel" => anchors.body,
                    _ => anchors.body,
                },
                _ => anchors.body,
            };
            let depth = equipment_depth(
                prefix,
                string_property(&part, "z").as_deref(),
                &part_name,
                two_handed,
            );
            let sprite_index = sprites.len();
            append_map_art_sprite(
                sprites,
                &part,
                &format!("character:equipment:{item_id}/{frame_path}/{part_name}"),
                anchor_x + attachment.0 - parent_position.0,
                anchor_y + attachment.1 - parent_position.1,
                255,
                false,
                cache,
            )?;
            set_sprite_depth(sprites, sprite_index, depth);
        }
    }
    Ok(())
}

fn character_equip_category(item_id: i32) -> Option<&'static str> {
    let prefix = item_id / 10_000;
    match prefix {
        100 => Some("Cap"),
        101..=103 | 112..=114 => Some("Accessory"),
        104 => Some("Coat"),
        105 => Some("Longcoat"),
        106 => Some("Pants"),
        107 => Some("Shoes"),
        108 => Some("Glove"),
        109 => Some("Shield"),
        110 => Some("Cape"),
        111 => Some("Ring"),
        130..=170 => Some("Weapon"),
        _ => None,
    }
}

fn resolve_equipment_frame(
    archive: &NxArchive,
    image_path: &str,
    stance: &str,
    frame: u8,
) -> Result<Option<(String, openstory_assets::NxNode)>, String> {
    let candidates = [
        format!("{image_path}/{stance}/{frame}"),
        format!("{image_path}/{stance}/0"),
        format!("{image_path}/stand1/{frame}"),
        format!("{image_path}/stand1/0"),
        format!("{image_path}/default/0"),
    ];
    for path in candidates {
        if let Some(node) = archive
            .root()
            .resolve(&path)
            .map_err(|error| error.to_string())?
        {
            return Ok(Some((path, node)));
        }
    }
    Ok(None)
}

fn last_map_vector(
    part: &openstory_assets::NxNode,
) -> Result<Option<(String, (f32, f32))>, String> {
    let Some(map) = part.child("map").map_err(|error| error.to_string())? else {
        return Ok(None);
    };
    let mut result = None;
    for child in map.children().map_err(|error| error.to_string())? {
        let name = child.name().map_err(|error| error.to_string())?;
        if let NodeValue::Vector { x, y } = child.value().map_err(|error| error.to_string())? {
            result = Some((name, (x as f32, y as f32)));
        }
    }
    Ok(result)
}

fn equipment_depth(prefix: i32, z: Option<&str>, part_name: &str, two_handed: bool) -> i16 {
    if part_name == "mailArm" {
        return if two_handed { 26 } else { 28 };
    }
    match z.unwrap_or("") {
        "cape" => 1,
        "backShield" | "backWeaponOverShield" | "shieldBelowBody" => 4,
        "weaponBelowBody" | "capBelowBody" => 5,
        "gloveWristOverBody" => 7,
        "gloveOverBody" => 8,
        "shoes" => 9,
        "pants" => 11,
        "top" | "mail" => 12,
        "shieldOverHair" => 14,
        "earrings" => 15,
        "faceacc" => 20,
        "eyeacc" => 21,
        "shield" => 22,
        "cap" => 24,
        "capOverHair" => 26,
        "weaponBelowArm" => 25,
        "weapon" => {
            if two_handed {
                28
            } else {
                26
            }
        }
        "gloveWrist" => 29,
        "glove" => 30,
        "weaponOverGlove" => 31,
        "weaponOverHand" => 35,
        "weaponOverBody" => 36,
        "gloveOverHair" => 40,
        "gloveWristOverHair" => 39,
        _ => match prefix {
            100 => 24,
            101 => 20,
            102 => 21,
            103 => 15,
            104 => 12,
            105 => 12,
            106 => 11,
            107 => 9,
            108 => 30,
            109 => 22,
            110 => 1,
            111 => 20,
            112..=114 => 20,
            130..=170 => {
                if two_handed {
                    28
                } else {
                    26
                }
            }
            _ => 18,
        },
    }
}

fn set_sprite_depth(sprites: &mut [DecodedSprite], first: usize, depth: i16) {
    if let Some(sprite) = sprites.get_mut(first) {
        sprite.depth_order = depth;
    }
}

fn append_hair_part(
    sprites: &mut Vec<DecodedSprite>,
    frame: &openstory_assets::NxNode,
    name: &str,
    frame_path: &str,
    anchor: (f32, f32),
    hair_position: (f32, f32),
    cache: &mut MapArtCache,
) -> Result<(), String> {
    let Some(part) = frame.child(name).map_err(|error| error.to_string())? else {
        return Ok(());
    };
    let brow = vector_path(&part, "map/brow").unwrap_or((0.0, 0.0));
    append_avatar_node(
        sprites,
        &part,
        &format!("{frame_path}/{name}"),
        (
            anchor.0 + hair_position.0 - brow.0,
            anchor.1 + hair_position.1 - brow.1,
        ),
        cache,
    )
}

fn append_avatar_node(
    sprites: &mut Vec<DecodedSprite>,
    node: &openstory_assets::NxNode,
    path: &str,
    anchor: (f32, f32),
    cache: &mut MapArtCache,
) -> Result<(), String> {
    append_map_art_sprite(
        sprites,
        node,
        &format!("character:{path}"),
        anchor.0,
        anchor.1,
        255,
        false,
        cache,
    )
}
