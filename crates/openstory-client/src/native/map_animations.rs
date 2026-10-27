fn append_npc_quest_mark_animation(
    sprites: &mut Vec<DecodedSprite>,
    ui_archive: &NxArchive,
    map_archive: &NxArchive,
    mark: NpcQuestMark,
    anchor_x: f32,
    anchor_y: f32,
    cache: &mut MapArtCache,
) -> Result<Vec<NpcQuestMarkFrameAsset>, String> {
    let (icon, fallback, label) = match mark {
        NpcQuestMark::None => return Ok(Vec::new()),
        NpcQuestMark::Available => (
            "UIWindow.img/QuestIcon/0",
            "MapHelper.img/quest/chatNext",
            "available",
        ),
        NpcQuestMark::InProgress => (
            "UIWindow.img/QuestIcon/1",
            "MapHelper.img/quest/chatSelf",
            "in-progress",
        ),
        NpcQuestMark::Complete => (
            "UIWindow.img/QuestIcon/2",
            "MapHelper.img/quest/chatComplete",
            "complete",
        ),
    };
    let source = ui_archive
        .root()
        .resolve(icon)
        .map_err(|error| error.to_string())?
        .or(map_archive
            .root()
            .resolve(fallback)
            .map_err(|error| error.to_string())?);
    let Some(source) = source else {
        return Ok(Vec::new());
    };
    let mut frames = numeric_children(&source)?;
    if frames.is_empty() {
        frames.push(source);
    }
    let mut assets = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        let first_sprite = sprites.len();
        append_map_art_sprite(
            sprites,
            frame,
            &format!("npc-quest-mark:{label}/{index}"),
            anchor_x,
            anchor_y,
            255,
            false,
            cache,
        )?;
        if sprites.len() == first_sprite {
            continue;
        }
        let delay_ms = integer_property(frame, "delay")
            .and_then(|delay| u16::try_from(delay).ok())
            .filter(|delay| *delay > 0)
            .unwrap_or(150);
        assets.push(NpcQuestMarkFrameAsset {
            delay_ms,
            sprites: first_sprite..sprites.len(),
        });
    }
    Ok(assets)
}

fn append_quest_clear_animation(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    cache: &mut MapArtCache,
) -> Result<Vec<NpcQuestMarkFrameAsset>, String> {
    let path = "BasicEff.img/QuestClear";
    let Some(effect) = archive
        .root()
        .resolve(path)
        .map_err(|error| error.to_string())?
    else {
        return Ok(Vec::new());
    };
    let mut frames = numeric_children(&effect)?;
    if frames.is_empty() {
        frames.push(effect);
    }
    let mut assets = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        let first_sprite = sprites.len();
        append_map_art_sprite(
            sprites,
            frame,
            &format!("effect:quest-clear/{index}"),
            512.0,
            GAMEPLAY_CAMERA_CENTER_Y,
            255,
            false,
            cache,
        )?;
        if sprites.len() == first_sprite {
            continue;
        }
        let delay_ms = integer_property(frame, "delay")
            .and_then(|delay| u16::try_from(delay).ok())
            .filter(|delay| *delay > 0)
            .unwrap_or(100);
        assets.push(NpcQuestMarkFrameAsset {
            delay_ms,
            sprites: first_sprite..sprites.len(),
        });
    }
    Ok(assets)
}

fn append_npc_animations(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    npc_id: i32,
    cache: &mut MapArtCache,
) -> Result<(Vec<NpcFrameAsset>, NpcTraits), String> {
    let image_path = format!("{npc_id:07}.img");
    let mut resolved_path = image_path.clone();
    let mut npc = archive
        .root()
        .resolve(&image_path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("NPC node {image_path:?} was not found"))?;
    let link = npc
        .child("info")
        .map_err(|error| error.to_string())?
        .and_then(|info| string_property(&info, "link"));
    if let Some(link) = link.filter(|link| !link.is_empty()) {
        resolved_path = format!("{link}.img");
        npc = archive
            .root()
            .resolve(&resolved_path)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("linked NPC node {resolved_path:?} was not found"))?;
    }
    let info = npc.child("info").map_err(|error| error.to_string())?;
    let script = info
        .as_ref()
        .and_then(|info| info.child("script").ok().flatten());
    let script_present = script.is_some_and(|script| {
        script.children().is_ok_and(|children| !children.is_empty())
            || matches!(script.value(), Ok(NodeValue::String(ref value)) if !value.is_empty())
    });
    let has_shop = info
        .as_ref()
        .and_then(|info| integer_property(info, "shop"))
        .is_some_and(|value| value != 0);
    let traits = NpcTraits {
        scripted: script_present || has_shop,
    };
    let mut frames = Vec::new();
    let states = npc.children().map_err(|error| error.to_string())?;
    for state in states {
        let state_name = state.name().map_err(|error| error.to_string())?;
        if state_name == "info" {
            continue;
        }
        let motion = NpcMotion::from_state_name(state_name.clone());
        let mut frame_nodes = numeric_children(&state)?;
        if frame_nodes.is_empty() {
            frame_nodes.push(state);
        }
        for (index, frame_node) in frame_nodes.iter().enumerate() {
            let first_sprite = sprites.len();
            append_map_art_sprite(
                sprites,
                frame_node,
                &format!("npc:{resolved_path}/{state_name}/{index}"),
                512.0,
                GAMEPLAY_CAMERA_CENTER_Y,
                255,
                false,
                cache,
            )?;
            if sprites.len() > first_sprite {
                let delay_ms = integer_property(frame_node, "delay")
                    .and_then(|delay| u16::try_from(delay).ok())
                    .filter(|delay| *delay > 0)
                    .unwrap_or(100);
                frames.push(NpcFrameAsset {
                    motion: motion.clone(),
                    delay_ms,
                    sprites: first_sprite..sprites.len(),
                });
            }
        }
    }
    Ok((frames, traits))
}

fn append_hired_merchant_art(
    sprites: &mut Vec<DecodedSprite>,
    item_archive: &NxArchive,
    ui_archive: &NxArchive,
    item_id: i32,
    requested_skin: i8,
    owner_name: &str,
    description: &str,
    font: &fontdue::Font,
    cache: &mut MapArtCache,
) -> Result<(Vec<ProjectileFrameAsset>, usize, usize, u8), String> {
    let item_path = format!("Cash/0503.img/{item_id:08}/employee/say");
    let say = item_archive
        .root()
        .resolve(&item_path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("hired merchant animation {item_path:?} is missing"))?;
    let mut frame_nodes = numeric_children(&say)?;
    if frame_nodes.is_empty() {
        frame_nodes.push(say);
    }
    let mut frames = Vec::new();
    for (frame_index, frame_node) in frame_nodes.iter().enumerate() {
        let first_sprite = sprites.len();
        append_map_art_sprite(
            sprites,
            frame_node,
            &format!("merchant:{item_path}/{frame_index}"),
            512.0,
            GAMEPLAY_CAMERA_CENTER_Y,
            255,
            false,
            cache,
        )?;
        if sprites.len() > first_sprite {
            let delay_ms = integer_property(frame_node, "delay")
                .and_then(|delay| u16::try_from(delay).ok())
                .filter(|delay| *delay > 0)
                .unwrap_or(100);
            frames.push(ProjectileFrameAsset {
                delay_ms,
                sprites: first_sprite..sprites.len(),
            });
        }
    }
    if frames.is_empty() {
        return Err(format!(
            "hired merchant animation {item_path:?} has no frames"
        ));
    }

    let mut skin = u8::try_from(requested_skin)
        .ok()
        .filter(|skin| *skin < 7)
        .unwrap_or(0);
    let sign_path = |skin: u8| format!("ChatBalloon.img/miniroom/PSSkin/{skin}");
    let mut resolved_sign = ui_archive
        .root()
        .resolve(&sign_path(skin))
        .map_err(|error| error.to_string())?;
    if resolved_sign.is_none() && skin != 0 {
        skin = 0;
        resolved_sign = ui_archive
            .root()
            .resolve(&sign_path(skin))
            .map_err(|error| error.to_string())?;
    }
    let sign_node = resolved_sign.ok_or_else(|| "hired merchant sign art is missing".to_owned())?;
    let sign_index = sprites.len();
    append_map_art_sprite(
        sprites,
        &sign_node,
        &format!("merchant-sign:{skin}"),
        512.0,
        302.0,
        255,
        false,
        cache,
    )?;
    let sign_label_y = {
        let sign = sprites
            .get_mut(sign_index)
            .ok_or_else(|| format!("hired merchant sign {skin} has no bitmap"))?;
        let sign_width = sign.rect.width;
        let sign_height = sign.rect.height;
        sign.rect.x = 512.0 - sign_width / 2.0;
        sign.rect.y = GAMEPLAY_CAMERA_CENTER_Y - 82.0 - sign_height;
        sign.rect.y + if skin == 0 { 9.0 } else { 68.0 }
    };

    let label = if description.is_empty() {
        owner_name
    } else {
        description
    }
    .chars()
    .take(14)
    .collect::<String>();
    let label_index = sprites.len();
    append_text_sprite(
        sprites,
        font,
        &label,
        centered_text_x(font, &label, 12.0),
        sign_label_y,
        12.0,
    );
    Ok((frames, sign_index, label_index, skin))
}

fn append_summon_animation(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    skill_id: i32,
    cache: &mut MapArtCache,
) -> Result<Vec<ProjectileFrameAsset>, String> {
    let job_id = skill_id / 10_000;
    let skill_path = format!("{job_id:03}.img/skill/{skill_id}");
    let skill = archive
        .root()
        .resolve(&skill_path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("summon skill node {skill_path:?} was not found"))?;

    if let Some(summon) = skill.child("summon").map_err(|error| error.to_string())? {
        for state_name in ["stand", "fly"] {
            if let Some(state) = summon
                .child(state_name)
                .map_err(|error| error.to_string())?
            {
                let frames = append_summon_frames_from_node(
                    sprites,
                    &state,
                    &format!("skill:{skill_path}/summon/{state_name}"),
                    cache,
                )?;
                if !frames.is_empty() {
                    return Ok(frames);
                }
            }
        }
        let frames = append_summon_frames_from_node(
            sprites,
            &summon,
            &format!("skill:{skill_path}/summon"),
            cache,
        )?;
        if !frames.is_empty() {
            return Ok(frames);
        }
    }

    if let Some(effect) = skill.child("effect").map_err(|error| error.to_string())? {
        return append_summon_frames_from_node(
            sprites,
            &effect,
            &format!("skill:{skill_path}/effect"),
            cache,
        );
    }
    Ok(Vec::new())
}

fn append_pet_animations(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    item_id: i32,
    cache: &mut MapArtCache,
) -> Result<HashMap<u8, Vec<ProjectileFrameAsset>>, String> {
    let pet_path = format!("Pet/{item_id}.img");
    let Some(pet) = archive
        .root()
        .resolve(&pet_path)
        .map_err(|error| error.to_string())?
    else {
        return Ok(HashMap::new());
    };
    let mut animations = HashMap::new();
    for (state, state_name) in [
        (0, "move"),
        (1, "stand0"),
        (2, "jump"),
        (3, "alert"),
        (4, "prone"),
        (5, "fly"),
        (6, "hang"),
    ] {
        let Some(source) = pet.child(state_name).map_err(|error| error.to_string())? else {
            continue;
        };
        let frames = append_summon_frames_from_node(
            sprites,
            &source,
            &format!("pet:{pet_path}/{state_name}"),
            cache,
        )?;
        if !frames.is_empty() {
            animations.insert(state, frames);
        }
    }
    Ok(animations)
}

fn append_pet_interaction_animation(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    item_id: i32,
    feed: bool,
    index: u8,
    success: bool,
    cache: &mut MapArtCache,
) -> Result<Vec<ProjectileFrameAsset>, String> {
    let pet_path = format!("Pet/{item_id}.img");
    let Some(pet) = archive
        .root()
        .resolve(&pet_path)
        .map_err(|error| error.to_string())?
    else {
        return Ok(Vec::new());
    };
    let group = if feed {
        pet.child("food")
            .map_err(|error| error.to_string())?
            .map(|food| food.child("0"))
            .transpose()
            .map_err(|error| error.to_string())?
            .flatten()
    } else {
        pet.child("interact")
            .map_err(|error| error.to_string())?
            .map(|interactions| interactions.child(&index.to_string()))
            .transpose()
            .map_err(|error| error.to_string())?
            .flatten()
    };
    let Some(group) = group else {
        return Ok(Vec::new());
    };
    let result_name = if success { "success" } else { "fail" };
    let alternate_name = if success { "fail" } else { "success" };
    let branch = match group
        .child(result_name)
        .map_err(|error| error.to_string())?
    {
        Some(branch) => Some(branch),
        None => group
            .child(alternate_name)
            .map_err(|error| error.to_string())?,
    };
    let Some(branch) = branch else {
        return Ok(Vec::new());
    };
    let Some(first_frame) = branch.child("0").map_err(|error| error.to_string())? else {
        return Ok(Vec::new());
    };
    let Some(action) = string_property(&first_frame, "act") else {
        return Ok(Vec::new());
    };
    let Some(source) = pet.child(&action).map_err(|error| error.to_string())? else {
        return Ok(Vec::new());
    };
    append_summon_frames_from_node(sprites, &source, &format!("pet:{pet_path}/{action}"), cache)
}

fn append_summon_frames_from_node(
    sprites: &mut Vec<DecodedSprite>,
    node: &openstory_assets::NxNode,
    path: &str,
    cache: &mut MapArtCache,
) -> Result<Vec<ProjectileFrameAsset>, String> {
    append_animation_frames_from_node_at(
        sprites,
        node,
        path,
        512.0,
        GAMEPLAY_CAMERA_CENTER_Y,
        cache,
    )
}

fn append_animation_frames_from_node_at(
    sprites: &mut Vec<DecodedSprite>,
    node: &openstory_assets::NxNode,
    path: &str,
    anchor_x: f32,
    anchor_y: f32,
    cache: &mut MapArtCache,
) -> Result<Vec<ProjectileFrameAsset>, String> {
    let mut frame_nodes = numeric_children(node)?;
    if frame_nodes.is_empty() {
        if node.bitmap().map_err(|error| error.to_string())?.is_some() {
            frame_nodes.push(node.clone());
        } else if let Some(bitmap) = first_bitmap_node(node)? {
            frame_nodes.push(bitmap);
        }
    }
    let mut frames = Vec::new();
    for (index, frame_node) in frame_nodes.into_iter().enumerate() {
        let first_sprite = sprites.len();
        append_map_art_sprite(
            sprites,
            &frame_node,
            &format!("{path}/{index}"),
            anchor_x,
            anchor_y,
            255,
            false,
            cache,
        )?;
        if sprites.len() > first_sprite {
            let delay_ms = integer_property(&frame_node, "delay")
                .and_then(|delay| u16::try_from(delay).ok())
                .filter(|delay| *delay > 0)
                .unwrap_or(100);
            frames.push(ProjectileFrameAsset {
                delay_ms,
                sprites: first_sprite..sprites.len(),
            });
        }
    }
    Ok(frames)
}

fn append_item_aura_animations(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    item_id: i32,
    cache: &mut MapArtCache,
) -> Result<PersistentItemAuraAsset, String> {
    let path = format!("ItemEff.img/{item_id}");
    let Some(effect) = archive
        .root()
        .resolve(&path)
        .map_err(|error| error.to_string())?
    else {
        return Ok(PersistentItemAuraAsset::default());
    };
    let top = numeric_children(&effect)?;
    let mut aura = PersistentItemAuraAsset::default();
    let Some(first) = top.first() else {
        return Ok(aura);
    };
    if first.bitmap().map_err(|error| error.to_string())?.is_some() {
        aura.front = append_animation_frames_from_node_at(
            sprites,
            &effect,
            &format!("item-aura:{item_id}/flat"),
            512.0,
            GAMEPLAY_CAMERA_CENTER_Y,
            cache,
        )?;
        return Ok(aura);
    }

    let inner = numeric_children(first)?;
    let inner_is_bitmap = inner
        .first()
        .map(|node| node.bitmap().map(|bitmap| bitmap.is_some()))
        .transpose()
        .map_err(|error| error.to_string())?
        .unwrap_or(false);
    if !inner_is_bitmap {
        aura.front = append_animation_frames_from_node_at(
            sprites,
            first,
            &format!("item-aura:{item_id}/fallback"),
            512.0,
            GAMEPLAY_CAMERA_CENTER_Y,
            cache,
        )?;
    } else if top.len() <= 2 {
        if let Some(back) = effect.child("0").map_err(|error| error.to_string())? {
            aura.back = append_animation_frames_from_node_at(
                sprites,
                &back,
                &format!("item-aura:{item_id}/back"),
                512.0,
                GAMEPLAY_CAMERA_CENTER_Y,
                cache,
            )?;
        }
        if let Some(front) = effect.child("1").map_err(|error| error.to_string())? {
            aura.front = append_animation_frames_from_node_at(
                sprites,
                &front,
                &format!("item-aura:{item_id}/front"),
                512.0,
                GAMEPLAY_CAMERA_CENTER_Y,
                cache,
            )?;
        }
    } else {
        for (index, frame) in top.iter().enumerate() {
            let delay_ms = integer_property(frame, "delay")
                .and_then(|delay| u16::try_from(delay).ok())
                .filter(|delay| *delay > 0)
                .unwrap_or(100);
            for (layer_name, target) in [("0", &mut aura.back), ("1", &mut aura.front)] {
                let Some(layer) = frame.child(layer_name).map_err(|error| error.to_string())?
                else {
                    continue;
                };
                if layer.bitmap().map_err(|error| error.to_string())?.is_none() {
                    continue;
                }
                let mut layer_frames = append_animation_frames_from_node_at(
                    sprites,
                    &layer,
                    &format!("item-aura:{item_id}/{layer_name}/{index}"),
                    512.0,
                    GAMEPLAY_CAMERA_CENTER_Y,
                    cache,
                )?;
                for frame in &mut layer_frames {
                    frame.delay_ms = delay_ms;
                }
                target.extend(layer_frames);
            }
        }
    }

    aura.split = !aura.back.is_empty() && !aura.front.is_empty();
    if aura.front.is_empty() {
        aura.front = std::mem::take(&mut aura.back);
        aura.split = false;
    }
    Ok(aura)
}

fn append_skill_use_effects(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    skill_id: i32,
    skill_level: u8,
    character_level: u8,
    two_handed: bool,
    cache: &mut MapArtCache,
) -> Result<Vec<CharacterSkillEffectAsset>, String> {
    let skill_path = format!("{:03}.img/skill/{skill_id:07}", skill_id / 10_000);
    let Some(skill) = archive
        .root()
        .resolve(&skill_path)
        .map_err(|error| error.to_string())?
    else {
        return Ok(Vec::new());
    };
    let mut sources = Vec::new();
    if let Some(char_levels) = skill
        .child("CharLevel")
        .map_err(|error| error.to_string())?
    {
        let mut levels = numeric_children(&char_levels)?;
        levels.sort_by_key(|node| {
            node.name()
                .ok()
                .and_then(|name| name.parse::<u8>().ok())
                .unwrap_or(u8::MAX)
        });
        let selected = levels
            .iter()
            .filter_map(|node| {
                let required = node.name().ok()?.parse::<u8>().ok()?;
                (required <= character_level).then_some((required, node))
            })
            .max_by_key(|(required, _)| *required)
            .map(|(_, node)| node.clone())
            .or_else(|| levels.first().cloned());
        if let Some(level_node) = selected
            && let Some(effect) = level_node
                .child("effect")
                .map_err(|error| error.to_string())?
        {
            sources.push((format!("skill:{skill_path}/CharLevel/effect"), effect));
        }
    }

    if sources.is_empty()
        && skill
            .child("effect0")
            .map_err(|error| error.to_string())?
            .is_some()
    {
        for index in 0..8 {
            let name = if index == 0 {
                "effect".to_owned()
            } else {
                format!("effect{}", index - 1)
            };
            let Some(effect) = skill.child(&name).map_err(|error| error.to_string())? else {
                break;
            };
            sources.push((format!("skill:{skill_path}/{name}"), effect));
        }
    }

    if sources.is_empty()
        && let Some(effect) = skill.child("effect").map_err(|error| error.to_string())?
    {
        let zero = effect.child("0").map_err(|error| error.to_string())?;
        let zero_is_bitmap = zero
            .as_ref()
            .is_some_and(|node| node.bitmap().is_ok_and(|bitmap| bitmap.is_some()));
        let one = effect.child("1").map_err(|error| error.to_string())?;
        if !zero_is_bitmap && one.is_some() {
            let selected = if two_handed { one } else { zero };
            if let Some(selected) = selected {
                sources.push((format!("skill:{skill_path}/effect/variant"), selected));
            }
        } else {
            sources.push((format!("skill:{skill_path}/effect"), effect));
        }
    }

    let mut animations = Vec::new();
    for (path, source) in sources {
        let z = integer_property(&source, "z")
            .unwrap_or(0)
            .clamp(i64::from(i8::MIN), i64::from(i8::MAX)) as i8;
        let frames = append_summon_frames_from_node(sprites, &source, &path, cache)?;
        if !frames.is_empty() {
            animations.push(CharacterSkillEffectAsset { z, frames });
        }
    }
    if animations.is_empty()
        && skill_level > 0
        && let Some(effect) = skill
            .child("level")
            .map_err(|error| error.to_string())?
            .and_then(|levels| levels.child(&skill_level.to_string()).ok().flatten())
            .and_then(|level| level.child("effect").ok().flatten())
    {
        let z = integer_property(&effect, "z")
            .unwrap_or(0)
            .clamp(i64::from(i8::MIN), i64::from(i8::MAX)) as i8;
        let frames = append_summon_frames_from_node(
            sprites,
            &effect,
            &format!("skill:{skill_path}/level/{skill_level}/effect"),
            cache,
        )?;
        if !frames.is_empty() {
            animations.push(CharacterSkillEffectAsset { z, frames });
        }
    }
    Ok(animations)
}

fn append_skill_hit_effects(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    skill_id: i32,
    skill_level: u8,
    character_level: u8,
    two_handed: bool,
    cache: &mut MapArtCache,
) -> Result<Vec<SkillHitEffectAsset>, String> {
    let skill_path = format!("{:03}.img/skill/{skill_id:07}", skill_id / 10_000);
    let Some(skill) = archive
        .root()
        .resolve(&skill_path)
        .map_err(|error| error.to_string())?
    else {
        return Ok(Vec::new());
    };
    let base_hit = skill.child("hit").map_err(|error| error.to_string())?;
    let has_hand_variant = base_hit
        .as_ref()
        .and_then(|hit| hit.child("1").ok().flatten())
        .is_some();
    let hand = if two_handed && has_hand_variant {
        "1"
    } else {
        "0"
    };

    let mut selected = None;
    if let Some(char_levels) = skill
        .child("CharLevel")
        .map_err(|error| error.to_string())?
    {
        let levels = numeric_children(&char_levels)?;
        let selected_level = levels
            .iter()
            .filter_map(|node| {
                let required = node.name().ok()?.parse::<u8>().ok()?;
                let has_hit = node
                    .child("hit")
                    .ok()?
                    .and_then(|hit| hit.child("0").ok().flatten())
                    .is_some();
                (required <= character_level && has_hit).then_some((required, node))
            })
            .max_by_key(|(required, _)| *required)
            .map(|(_, node)| (*node).clone())
            .or_else(|| {
                levels.iter().find_map(|node| {
                    let has_hit = node
                        .child("hit")
                        .ok()?
                        .and_then(|hit| hit.child("0").ok().flatten())
                        .is_some();
                    has_hit.then(|| node.clone())
                })
            });
        if let Some(level_node) = selected_level {
            let level_name = level_node.name().unwrap_or_default();
            selected = level_node
                .child("hit")
                .map_err(|error| error.to_string())?
                .and_then(|hit| hit.child(hand).ok().flatten())
                .map(|hit| {
                    (
                        format!("skill:{skill_path}/CharLevel/{level_name}/hit/{hand}"),
                        hit,
                    )
                });
        }
    }

    if selected.is_none()
        && let Some(levels) = skill.child("level").map_err(|error| error.to_string())?
        && let Some(level) = levels
            .child(&skill_level.to_string())
            .map_err(|error| error.to_string())?
        && let Some(hit) = level.child("hit").map_err(|error| error.to_string())?
        && let Some(hit) = hit.child("0").map_err(|error| error.to_string())?
    {
        selected = Some((format!("skill:{skill_path}/level/{skill_level}/hit/0"), hit));
    }

    if selected.is_none()
        && let Some(hit) = base_hit
        && let Some(hit) = hit.child(hand).map_err(|error| error.to_string())?
    {
        selected = Some((format!("skill:{skill_path}/hit/{hand}"), hit));
    }

    let Some((path, source)) = selected else {
        return Ok(Vec::new());
    };
    let pos = integer_property(&source, "pos")
        .unwrap_or(0)
        .clamp(i64::from(i8::MIN), i64::from(i8::MAX)) as i8;
    let z = integer_property(&source, "z")
        .unwrap_or(0)
        .clamp(i64::from(i8::MIN), i64::from(i8::MAX)) as i8;
    let frames = append_summon_frames_from_node(sprites, &source, &path, cache)?;
    Ok((!frames.is_empty())
        .then_some(SkillHitEffectAsset { pos, z, frames })
        .into_iter()
        .collect())
}

fn append_skill_projectile_animation(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    skill_id: i32,
    skill_level: u8,
    cache: &mut MapArtCache,
) -> Result<Option<Vec<ProjectileFrameAsset>>, String> {
    let skill_path = format!("{:03}.img/skill/{skill_id:07}", skill_id / 10_000);
    let Some(skill) = archive
        .root()
        .resolve(&skill_path)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let levels = skill.child("level").map_err(|error| error.to_string())?;
    let level_balls = levels
        .as_ref()
        .and_then(|levels| levels.child("1").ok().flatten())
        .and_then(|level| level.child("ball").ok().flatten());
    let (source, source_path) = if level_balls.is_some() {
        let selected_ball = levels
            .and_then(|levels| levels.child(&skill_level.to_string()).ok().flatten())
            .and_then(|level| level.child("ball").ok().flatten());
        let Some(ball) = selected_ball else {
            return Ok(Some(Vec::new()));
        };
        (ball, format!("skill:{skill_path}/level/{skill_level}/ball"))
    } else {
        let Some(ball) = skill.child("ball").map_err(|error| error.to_string())? else {
            return Ok(None);
        };
        (ball, format!("skill:{skill_path}/ball"))
    };
    let frames = append_summon_frames_from_node(sprites, &source, &source_path, cache)?;
    Ok(Some(frames))
}

fn is_teleport_skill(skill_id: i32) -> bool {
    matches!(
        skill_id,
        2_101_002 | 2_201_002 | 2_301_001 | 9_001_002 | 9_101_002
    )
}

fn teleport_skill_range(archive: &NxArchive, skill_id: i32, skill_level: u8) -> i16 {
    let path = format!(
        "{:03}.img/skill/{skill_id:07}/level/{}",
        skill_id / 10_000,
        skill_level.max(1)
    );
    let range = archive
        .root()
        .resolve(&path)
        .ok()
        .flatten()
        .and_then(|level| number_property(&level, "range"))
        .filter(|range| range.is_finite() && *range > 0.0)
        .map(|range| range.round().clamp(1.0, f32::from(i16::MAX)) as i16);
    range.unwrap_or(130)
}

fn character_skill_action_stance(
    archive: &NxArchive,
    look: &openstory_protocol::CharacterLook,
    skill_id: i32,
    skill_level: u8,
) -> Result<Option<u8>, String> {
    let skill_path = format!("{:03}.img/skill/{skill_id:07}", skill_id / 10_000);
    let Some(skill) = archive
        .root()
        .resolve(&skill_path)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let two_handed = character_weapon_id(look).is_some_and(is_two_handed_weapon);
    let action = skill
        .child("action")
        .map_err(|error| error.to_string())?
        .and_then(|actions| {
            actions
                .child(if two_handed { "1" } else { "0" })
                .ok()
                .flatten()
                .and_then(|node| node.value().ok())
                .and_then(|value| match value {
                    NodeValue::String(action) => Some(action),
                    _ => None,
                })
        })
        .or_else(|| {
            skill
                .child("level")
                .ok()
                .flatten()?
                .child(&skill_level.max(1).to_string())
                .ok()
                .flatten()?
                .child("action")
                .ok()
                .flatten()?
                .value()
                .ok()
                .and_then(|value| match value {
                    NodeValue::String(action) => Some(action),
                    _ => None,
                })
        });
    let stances = character_weapon_attack_stances(archive, look)?;
    let action_stance = action.as_deref().and_then(|action| {
        stances
            .iter()
            .find_map(|(stance, name)| (*name == action).then_some(*stance))
    });
    if action_stance.is_some() {
        return Ok(action_stance);
    }
    let level_node = skill
        .child("level")
        .map_err(|error| error.to_string())?
        .and_then(|levels| levels.child(&skill_level.max(1).to_string()).ok().flatten());
    let attack_count = level_node
        .as_ref()
        .and_then(|node| integer_property(node, "attackCount"))
        .unwrap_or(0);
    Ok((is_known_attack_skill(skill_id) || attack_count > 0)
        .then(|| stances.first().map(|(stance, _)| *stance))
        .flatten())
}

fn append_reactor_frame(
    sprites: &mut Vec<DecodedSprite>,
    node: &openstory_assets::NxNode,
    path: &str,
    cache: &mut MapArtCache,
) -> Result<Option<ReactorFrameAsset>, String> {
    let first_sprite = sprites.len();
    append_map_art_sprite(
        sprites,
        node,
        path,
        512.0,
        GAMEPLAY_CAMERA_CENTER_Y,
        255,
        false,
        cache,
    )?;
    if sprites.len() == first_sprite {
        return Ok(None);
    }
    let delay_ms = integer_property(node, "delay")
        .and_then(|delay| u16::try_from(delay).ok())
        .filter(|delay| *delay > 0)
        .unwrap_or(100);
    Ok(Some(ReactorFrameAsset {
        delay_ms,
        sprites: first_sprite..sprites.len(),
    }))
}

fn append_reactor_animations(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    reactor_id: i32,
    cache: &mut MapArtCache,
) -> Result<ReactorArtAssets, String> {
    let image_path = format!("{reactor_id:07}.img");
    let reactor = archive
        .root()
        .resolve(&image_path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("reactor node {image_path:?} was not found"))?;
    let mut states = numeric_children(&reactor)?;
    states.sort_by_key(|state| {
        state
            .name()
            .ok()
            .and_then(|name| name.parse::<i8>().ok())
            .unwrap_or(i8::MAX)
    });
    let mut hittable = false;
    for state in &states {
        if state
            .child("event")
            .map_err(|error| error.to_string())?
            .is_some()
        {
            hittable = true;
            break;
        }
    }

    let idle_state = reactor
        .child("0")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("reactor {reactor_id} has no idle state"))?;
    let mut idle_nodes = numeric_children(&idle_state)?;
    if idle_nodes.is_empty() {
        idle_nodes.push(idle_state);
    }
    let idle = idle_nodes
        .first()
        .ok_or_else(|| format!("reactor {reactor_id} has no idle frame node"))?;
    let idle = append_reactor_frame(sprites, idle, &format!("reactor:{image_path}/0/0"), cache)?
        .ok_or_else(|| format!("reactor {reactor_id} has no drawable idle frame"))?;

    let mut hits = HashMap::new();
    for state in states {
        let Some(state_id) = state
            .name()
            .map_err(|error| error.to_string())?
            .parse::<i8>()
            .ok()
        else {
            continue;
        };
        let Some(hit) = state.child("hit").map_err(|error| error.to_string())? else {
            continue;
        };
        let mut frame_nodes = numeric_children(&hit)?;
        if frame_nodes.is_empty() {
            frame_nodes.push(hit);
        }
        let mut frames = Vec::new();
        for (index, frame_node) in frame_nodes.iter().enumerate() {
            if let Some(frame) = append_reactor_frame(
                sprites,
                frame_node,
                &format!("reactor:{image_path}/{state_id}/hit/{index}"),
                cache,
            )? {
                frames.push(frame);
            }
        }
        if !frames.is_empty() {
            hits.insert(state_id, frames);
        }
    }
    Ok(ReactorArtAssets {
        idle,
        hits,
        hittable,
    })
}

fn append_drop_icons(
    sprites: &mut Vec<DecodedSprite>,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    drop: openstory_protocol::MapDropSpawn,
    cache: &mut MapArtCache,
) -> Result<Vec<DropFrameAsset>, String> {
    let (archive, path) = if drop.meso {
        let icon = if drop.item_id > 999 {
            3
        } else if drop.item_id > 99 {
            2
        } else if drop.item_id > 49 {
            1
        } else {
            0
        };
        (
            item_archive,
            format!("Special/0900.img/0900000{icon}/iconRaw"),
        )
    } else {
        let item_id = drop.item_id;
        if item_id <= 0 {
            return Err(format!("invalid drop item id {item_id}"));
        }
        let item_file = format!("0{item_id}.img");
        let item_node = format!("0{item_id}");
        let item_prefix = item_id / 10_000;
        match item_id / 1_000_000 {
            1 => {
                let category = match item_prefix - 100 {
                    0 => "Cap",
                    1..=3 | 12..=14 => "Accessory",
                    4 => "Coat",
                    5 => "Longcoat",
                    6 => "Pants",
                    7 => "Shoes",
                    8 => "Glove",
                    9 => "Shield",
                    10 => "Cape",
                    11 => "Ring",
                    30..=70 => "Weapon",
                    _ => return Err(format!("unsupported equipment drop id {item_id}")),
                };
                (
                    character_archive,
                    format!("{category}/{item_file}/info/iconRaw"),
                )
            }
            2 => (
                item_archive,
                format!("Consume/0{item_prefix}.img/{item_node}/info/iconRaw"),
            ),
            3 => (
                item_archive,
                format!("Install/0{item_prefix}.img/{item_node}/info/iconRaw"),
            ),
            4 => (
                item_archive,
                format!("Etc/0{item_prefix}.img/{item_node}/info/iconRaw"),
            ),
            5 if item_prefix == 500 => (item_archive, format!("Pet/{item_id}.img/info/iconRaw")),
            5 => (
                item_archive,
                format!("Cash/0{item_prefix}.img/{item_node}/info/iconRaw"),
            ),
            _ => return Err(format!("unsupported drop item id {item_id}")),
        }
    };
    let icon = archive
        .root()
        .resolve(&path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("drop icon {path:?} was not found"))?;
    let mut frame_nodes = numeric_children(&icon)?;
    if frame_nodes.is_empty() {
        frame_nodes.push(icon);
    }
    let mut frames = Vec::new();
    for (frame_number, frame_node) in frame_nodes.iter().enumerate() {
        let sprite_index = sprites.len();
        append_map_art_sprite(
            sprites,
            frame_node,
            &format!("drop:{path}/{frame_number}"),
            512.0,
            GAMEPLAY_CAMERA_CENTER_Y,
            255,
            false,
            cache,
        )?;
        if sprites.len() > sprite_index {
            let delay_ms = integer_property(frame_node, "delay")
                .and_then(|delay| u16::try_from(delay).ok())
                .filter(|delay| *delay > 0)
                .unwrap_or(100);
            frames.push(DropFrameAsset {
                delay_ms,
                sprite_index,
            });
        }
    }
    Ok(frames)
}

fn chair_ground_offset(
    frame: &openstory_assets::NxNode,
) -> Result<Option<((f32, f32), f32)>, String> {
    let Some(art) = first_bitmap_node(frame)? else {
        return Ok(None);
    };
    let bitmap = art
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "chair frame has no bitmap".to_owned())?;
    let width = f32::from(bitmap.width());
    let height = f32::from(bitmap.height());
    let origin = vector_path(&art, "origin").unwrap_or((0.0, 0.0));
    Ok(Some(((origin.0 - width / 2.0, origin.1 - height), height)))
}

fn append_chair_animation(
    sprites: &mut Vec<DecodedSprite>,
    source: Option<&openstory_assets::NxNode>,
    item_id: i32,
    part_name: &str,
    offset: (f32, f32),
    cache: &mut MapArtCache,
) -> Result<Vec<ProjectileFrameAsset>, String> {
    let Some(source) = source else {
        return Ok(Vec::new());
    };
    let mut frame_nodes = numeric_children(source)?;
    if frame_nodes.is_empty() {
        frame_nodes.push(source.clone());
    }
    let mut frames = Vec::new();
    for (index, frame) in frame_nodes.iter().enumerate() {
        let first_sprite = sprites.len();
        append_map_art_sprite(
            sprites,
            frame,
            &format!("chair:{item_id}/{part_name}/{index}"),
            512.0 + offset.0,
            GAMEPLAY_CAMERA_CENTER_Y + offset.1,
            255,
            false,
            cache,
        )?;
        if sprites.len() > first_sprite {
            let delay_ms = integer_property(frame, "delay")
                .and_then(|delay| u16::try_from(delay).ok())
                .filter(|delay| *delay > 0)
                .unwrap_or(100);
            frames.push(ProjectileFrameAsset {
                delay_ms,
                sprites: first_sprite..sprites.len(),
            });
        }
    }
    Ok(frames)
}

fn append_local_chair(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    item_id: i32,
    cache: &mut MapArtCache,
) -> Result<LocalChairAsset, String> {
    let item_prefix = item_id / 10_000;
    let path = format!("Install/0{item_prefix}.img/0{item_id}");
    let item = archive
        .root()
        .resolve(&path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("chair item {path:?} was not found"))?;
    let info = item.child("info").map_err(|error| error.to_string())?;
    let behind = item.child("effect").map_err(|error| error.to_string())?;
    let front = item.child("effect2").map_err(|error| error.to_string())?;
    let auto_fit = info
        .as_ref()
        .and_then(|info| integer_property(info, "ai"))
        .is_some_and(|value| value != 0);

    let (behind_offset, front_offset, sit_offset_y) = if auto_fit {
        let behind_first = behind
            .as_ref()
            .map(numeric_children)
            .transpose()?
            .and_then(|frames| frames.into_iter().next())
            .or_else(|| behind.clone());
        let (behind_offset, height) = behind_first
            .as_ref()
            .map(chair_ground_offset)
            .transpose()?
            .flatten()
            .unwrap_or(((0.0, 0.0), 0.0));
        let chair_height = height;
        let front_first = front
            .as_ref()
            .map(numeric_children)
            .transpose()?
            .and_then(|frames| frames.into_iter().next())
            .or_else(|| front.clone());
        let front_offset = front_first
            .as_ref()
            .map(chair_ground_offset)
            .transpose()?
            .flatten()
            .map(|(offset, _)| (offset.0, offset.1 - chair_height))
            .unwrap_or((0.0, -chair_height));
        let seat_y = info
            .as_ref()
            .and_then(|info| info.child("seat").ok().flatten())
            .and_then(|seat| integer_property(&seat, "y"))
            .map(|y| y as f32);
        let sit_offset_y = if chair_height > 0.0 {
            seat_y.map_or(-(chair_height / 2.0), |seat_y| seat_y - chair_height) + 12.0
        } else {
            0.0
        };
        (behind_offset, front_offset, sit_offset_y)
    } else {
        let behind_offset = behind
            .as_ref()
            .and_then(|effect| vector_path(effect, "pos"))
            .unwrap_or((0.0, 0.0));
        let front_offset = front
            .as_ref()
            .and_then(|effect| vector_path(effect, "pos"))
            .unwrap_or((0.0, 0.0));
        (
            (behind_offset.0, behind_offset.1 - 50.0),
            (front_offset.0, front_offset.1 - 50.0),
            0.0,
        )
    };

    let behind_frames = append_chair_animation(
        sprites,
        behind.as_ref(),
        item_id,
        "effect",
        behind_offset,
        cache,
    )?;
    let front_frames = append_chair_animation(
        sprites,
        front.as_ref(),
        item_id,
        "effect2",
        front_offset,
        cache,
    )?;
    Ok(LocalChairAsset {
        behind: behind_frames,
        front: front_frames,
        sit_offset_y,
    })
}

fn append_projectile_animation(
    sprites: &mut Vec<DecodedSprite>,
    item_archive: &NxArchive,
    item_id: i32,
    cache: &mut MapArtCache,
) -> Result<Vec<ProjectileFrameAsset>, String> {
    let item_prefix = item_id / 10_000;
    let item_node = format!("0{item_id}");
    let path = format!("Consume/0{item_prefix}.img/{item_node}/bullet");
    let bullet = item_archive
        .root()
        .resolve(&path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("projectile animation {path:?} was not found"))?;
    let mut frame_nodes = numeric_children(&bullet)?;
    if frame_nodes.is_empty() {
        frame_nodes.push(bullet);
    }

    let mut frames = Vec::new();
    for (index, frame) in frame_nodes.iter().enumerate() {
        let first_sprite = sprites.len();
        append_map_art_sprite(
            sprites,
            frame,
            &format!("projectile:{path}/{index}"),
            0.0,
            0.0,
            255,
            false,
            cache,
        )?;
        if sprites.len() > first_sprite {
            let delay_ms = integer_property(frame, "delay")
                .and_then(|delay| u16::try_from(delay).ok())
                .filter(|delay| *delay > 0)
                .unwrap_or(100);
            frames.push(ProjectileFrameAsset {
                delay_ms,
                sprites: first_sprite..sprites.len(),
            });
        }
    }
    Ok(frames)
}

fn append_mob_animations(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    mob_id: i32,
    cache: &mut MapArtCache,
) -> Result<
    (
        Vec<MobFrameAsset>,
        bool,
        bool,
        MobTraits,
        HashMap<(u8, u8), (u8, i8)>,
    ),
    String,
> {
    let image_path = format!("{mob_id:07}.img");
    let mut resolved_path = image_path.clone();
    let mut mob = archive
        .root()
        .resolve(&image_path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("mob node {image_path:?} was not found"))?;
    let link = mob
        .child("info")
        .map_err(|error| error.to_string())?
        .and_then(|info| string_property(&info, "link"));
    if let Some(link) = link.filter(|link| !link.is_empty()) {
        resolved_path = format!("{link}.img");
        mob = archive
            .root()
            .resolve(&resolved_path)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("linked mob node {resolved_path:?} was not found"))?;
    }
    let info = mob.child("info").map_err(|error| error.to_string())?;
    let no_flip = info
        .as_ref()
        .and_then(|node| integer_property(node, "noFlip"))
        .unwrap_or(0)
        != 0;
    let faces_right = info
        .as_ref()
        .and_then(|node| integer_property(node, "ai"))
        .unwrap_or(0)
        != 0;
    let has_state_frames = |state: &str| -> Result<bool, String> {
        let Some(node) = mob.child(state).map_err(|error| error.to_string())? else {
            return Ok(false);
        };
        Ok(!numeric_children(&node)?.is_empty())
    };
    let can_fly = has_state_frames("fly")?;
    let can_jump = has_state_frames("jump")?;
    let can_move = can_fly || has_state_frames("move")?;
    let speed = info
        .as_ref()
        .and_then(|node| integer_property(node, "speed"))
        .unwrap_or(0)
        .saturating_add(100) as f64
        * 0.001;
    let fly_speed = info
        .as_ref()
        .and_then(|node| integer_property(node, "flySpeed"))
        .unwrap_or(0)
        .saturating_add(100) as f64
        * 0.0005;
    let traits = MobTraits {
        can_move,
        can_jump,
        can_fly,
        body_attack: info
            .as_ref()
            .and_then(|node| integer_property(node, "bodyAttack"))
            .unwrap_or(0)
            != 0,
        speed,
        fly_speed,
        physical_attack: info
            .as_ref()
            .and_then(|node| integer_property(node, "PADamage"))
            .unwrap_or(0)
            .clamp(0, i32::MAX as i64) as i32,
        level: info
            .as_ref()
            .and_then(|node| integer_property(node, "level"))
            .unwrap_or(1)
            .clamp(1, i32::MAX as i64) as i32,
        physical_defense: info
            .as_ref()
            .and_then(|node| integer_property(node, "PDDamage"))
            .unwrap_or(0)
            .max(0) as f64,
        magic_defense: info
            .as_ref()
            .and_then(|node| integer_property(node, "MDDamage"))
            .unwrap_or(0)
            .max(0) as f64,
        elemental_attributes: info
            .as_ref()
            .and_then(|node| string_property(node, "elemAttr"))
            .as_deref()
            .map(parse_elemental_attributes)
            .unwrap_or([1; 6]),
        avoid: info
            .as_ref()
            .and_then(|node| integer_property(node, "eva"))
            .unwrap_or(0)
            .max(0) as f64,
    };
    let mut sources = vec![
        (MobMotion::Stand, if can_fly { "fly" } else { "stand" }),
        (MobMotion::Move, if can_fly { "fly" } else { "move" }),
        (MobMotion::Jump, "jump"),
        (MobMotion::Hit, "hit1"),
        (MobMotion::Die, "die1"),
    ]
    .into_iter()
    .map(|(motion, state)| (motion, state.to_owned()))
    .collect::<Vec<_>>();
    for index in 1..=9 {
        sources.push((MobMotion::Attack(index), format!("attack{index}")));
        sources.push((MobMotion::Skill(index), format!("skill{index}")));
    }
    let mut frames = Vec::new();
    for (motion, state) in sources {
        let Some(state_node) = mob.child(&state).map_err(|error| error.to_string())? else {
            continue;
        };
        for frame_node in numeric_children(&state_node)? {
            let frame_name = frame_node.name().map_err(|error| error.to_string())?;
            let first_sprite = sprites.len();
            append_map_art_sprite(
                sprites,
                &frame_node,
                &format!("mob:{resolved_path}/{state}/{frame_name}"),
                512.0,
                GAMEPLAY_CAMERA_CENTER_Y,
                255,
                false,
                cache,
            )?;
            if sprites.len() > first_sprite {
                let delay_ms = integer_property(&frame_node, "delay")
                    .and_then(|delay| u16::try_from(delay).ok())
                    .filter(|delay| *delay > 0)
                    .unwrap_or(100);
                frames.push(MobFrameAsset {
                    motion,
                    delay_ms,
                    sprites: first_sprite..sprites.len(),
                });
            }
        }
    }
    let mut skill_actions = HashMap::new();
    if let Some(skill_list) = info
        .as_ref()
        .and_then(|node| node.child("skill").ok().flatten())
    {
        for (index, skill) in numeric_children(&skill_list)?.into_iter().enumerate() {
            if index >= 9 {
                break;
            }
            let Some(skill_id) =
                integer_property(&skill, "skill").and_then(|value| u8::try_from(value).ok())
            else {
                continue;
            };
            let Some(skill_level) =
                integer_property(&skill, "level").and_then(|value| u8::try_from(value).ok())
            else {
                continue;
            };
            let slot = index as u8 + 1;
            let action = 42 + (index as i8 * 2);
            skill_actions.insert((skill_id, skill_level), (slot, action));
        }
    }
    Ok((frames, no_flip, faces_right, traits, skill_actions))
}
