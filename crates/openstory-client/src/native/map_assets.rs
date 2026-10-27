fn load_map_portals(map: &openstory_assets::NxNode) -> Result<Vec<MapPortalAsset>, String> {
    let Some(portal_root) = map.child("portal").map_err(|error| error.to_string())? else {
        return Ok(Vec::new());
    };
    let mut portals = Vec::new();
    for portal in numeric_children(&portal_root)? {
        let Some(portal_type) =
            integer_property(&portal, "pt").and_then(|value| u8::try_from(value).ok())
        else {
            continue;
        };
        portals.push(MapPortalAsset {
            portal_type,
            name: string_property(&portal, "pn").unwrap_or_default(),
            target_name: string_property(&portal, "tn").unwrap_or_default(),
            target_map_id: integer_property(&portal, "tm")
                .and_then(|value| i32::try_from(value).ok())
                .unwrap_or(999_999_999),
            x: f64::from(number_property(&portal, "x").unwrap_or(0.0)),
            y: f64::from(number_property(&portal, "y").unwrap_or(0.0)),
        });
    }
    Ok(portals)
}

fn load_map_seats(map: &openstory_assets::NxNode) -> Result<Vec<(i16, i16)>, String> {
    let Some(seat_root) = map.child("seat").map_err(|error| error.to_string())? else {
        return Ok(Vec::new());
    };
    let seats = numeric_children(&seat_root)?
        .into_iter()
        .filter_map(|seat| {
            let x = integer_property(&seat, "x").and_then(|value| i16::try_from(value).ok())?;
            let y = integer_property(&seat, "y").and_then(|value| i16::try_from(value).ok())?;
            Some((x, y))
        })
        .collect();
    Ok(seats)
}

fn load_map_ladders(map: &openstory_assets::NxNode) -> Result<Vec<MapLadder>, String> {
    let Some(ladder_root) = map.child("ladderRope").map_err(|error| error.to_string())? else {
        return Ok(Vec::new());
    };
    let ladders = numeric_children(&ladder_root)?
        .into_iter()
        .filter_map(|ladder| {
            let x = integer_property(&ladder, "x").and_then(|value| i16::try_from(value).ok())?;
            let y1 = integer_property(&ladder, "y1").and_then(|value| i16::try_from(value).ok())?;
            let y2 = integer_property(&ladder, "y2").and_then(|value| i16::try_from(value).ok())?;
            Some(MapLadder {
                x,
                y1: y1.min(y2),
                y2: y1.max(y2),
                is_ladder: integer_property(&ladder, "l").unwrap_or(0) != 0,
            })
        })
        .collect();
    Ok(ladders)
}

fn append_portal_animation(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    node_path: &str,
    cache: &mut MapArtCache,
) -> Result<Vec<SpriteFrameAsset>, String> {
    let Some(animation) = archive
        .root()
        .resolve(node_path)
        .map_err(|error| error.to_string())?
    else {
        return Ok(Vec::new());
    };
    let mut frame_nodes = numeric_children(&animation)?;
    if frame_nodes.is_empty() {
        frame_nodes.push(animation);
    }
    let mut frames = Vec::new();
    for (index, frame_node) in frame_nodes.iter().enumerate() {
        let first_sprite = sprites.len();
        append_map_art_sprite(
            sprites,
            frame_node,
            &format!("portal:{node_path}/{index}"),
            512.0,
            GAMEPLAY_CAMERA_CENTER_Y,
            255,
            false,
            cache,
        )?;
        let delay_ms = integer_property(frame_node, "delay")
            .and_then(|delay| u16::try_from(delay).ok())
            .filter(|delay| *delay > 0)
            .unwrap_or(100);
        frames.push(SpriteFrameAsset {
            delay_ms,
            sprites: first_sprite..sprites.len(),
        });
    }
    Ok(frames)
}

fn vector_path(node: &openstory_assets::NxNode, path: &str) -> Option<(f32, f32)> {
    match node
        .resolve(path)
        .ok()?
        .and_then(|node| node.value().ok())?
    {
        NodeValue::Vector { x, y } => Some((x as f32, y as f32)),
        _ => None,
    }
}

fn append_map_layers(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    map: &openstory_assets::NxNode,
    camera_x: f32,
    camera_y: f32,
    cache: &mut MapArtCache,
    animations: &mut Vec<MapArtAnimationAsset>,
) -> Result<[usize; 8], String> {
    let mut layer_insert_indices = [0; 8];
    let mut next_layer = 0;
    for layer in numeric_children(map)? {
        let layer_id = layer
            .name()
            .map_err(|error| error.to_string())?
            .parse::<usize>()
            .map_err(|error| error.to_string())?;
        while next_layer < 8 && next_layer < layer_id {
            layer_insert_indices[next_layer] = sprites.len();
            next_layer += 1;
        }
        append_map_layer(
            sprites, archive, &layer, camera_x, camera_y, cache, animations,
        )?;
        if layer_id < 8 {
            layer_insert_indices[layer_id] = sprites.len();
            next_layer = layer_id + 1;
        }
    }
    while next_layer < 8 {
        layer_insert_indices[next_layer] = sprites.len();
        next_layer += 1;
    }
    Ok(layer_insert_indices)
}

fn append_map_layer(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    layer: &openstory_assets::NxNode,
    camera_x: f32,
    camera_y: f32,
    cache: &mut MapArtCache,
    animations: &mut Vec<MapArtAnimationAsset>,
) -> Result<(), String> {
    let tile_set = layer
        .child("info")
        .map_err(|error| error.to_string())?
        .and_then(|info| string_property(&info, "tS"));

    if let Some(objects) = layer.child("obj").map_err(|error| error.to_string())? {
        let mut objects = numeric_children(&objects)?;
        objects.sort_by_key(|object| integer_property(object, "z").unwrap_or(0));
        for object in objects {
            let Some(set) = string_property(&object, "oS") else {
                continue;
            };
            let Some(layer0) = string_property(&object, "l0") else {
                continue;
            };
            let Some(layer1) = string_property(&object, "l1") else {
                continue;
            };
            let Some(layer2) = string_property(&object, "l2") else {
                continue;
            };
            let path = format!("Obj/{set}.img/{layer0}/{layer1}/{layer2}");
            let Some(art) = archive
                .root()
                .resolve(&path)
                .map_err(|error| error.to_string())?
            else {
                continue;
            };
            let anchor_x = number_property(&object, "x").unwrap_or(0.0) + camera_x;
            let anchor_y = number_property(&object, "y").unwrap_or(0.0) + camera_y;
            let flipped = integer_property(&object, "f").unwrap_or(0) != 0;
            if let Some(animation) = append_map_art_animation(
                sprites, &art, &path, anchor_x, anchor_y, 255, flipped, cache,
            )? {
                animations.push(animation);
            }
        }
    }

    let (Some(tile_set), Some(tiles)) = (
        tile_set,
        layer.child("tile").map_err(|error| error.to_string())?,
    ) else {
        return Ok(());
    };
    let mut tiles_with_art = Vec::new();
    for tile in numeric_children(&tiles)? {
        let Some(category) = string_property(&tile, "u") else {
            continue;
        };
        let tile_number = integer_property(&tile, "no").unwrap_or(0);
        let path = format!("Tile/{tile_set}.img/{category}/{tile_number}");
        let Some(art) = archive
            .root()
            .resolve(&path)
            .map_err(|error| error.to_string())?
        else {
            continue;
        };
        let depth = integer_property(&art, "z")
            .filter(|depth| *depth != 0)
            .or_else(|| integer_property(&art, "zM"))
            .unwrap_or(0);
        tiles_with_art.push((depth, tile, path, art));
    }
    tiles_with_art.sort_by_key(|(depth, _, _, _)| *depth);
    for (_, tile, path, art) in tiles_with_art {
        let anchor_x = number_property(&tile, "x").unwrap_or(0.0) + camera_x;
        let anchor_y = number_property(&tile, "y").unwrap_or(0.0) + camera_y;
        append_map_art_sprite(sprites, &art, &path, anchor_x, anchor_y, 255, false, cache)?;
    }
    Ok(())
}

fn append_map_art_animation(
    sprites: &mut Vec<DecodedSprite>,
    source: &openstory_assets::NxNode,
    path: &str,
    anchor_x: f32,
    anchor_y: f32,
    opacity: u8,
    flipped: bool,
    cache: &mut MapArtCache,
) -> Result<Option<MapArtAnimationAsset>, String> {
    let source_is_bitmap = source
        .bitmap()
        .map_err(|error| error.to_string())?
        .is_some();
    let mut frame_nodes = numeric_children(source)?;
    if frame_nodes.is_empty() {
        frame_nodes.push(source.clone());
    }
    let mut frames = Vec::new();
    for frame_node in &frame_nodes {
        let first_sprite = sprites.len();
        let frame_name = frame_node.name().map_err(|error| error.to_string())?;
        let frame_path = if source_is_bitmap {
            path.to_owned()
        } else {
            format!("{path}/{frame_name}")
        };
        append_map_art_sprite(
            sprites,
            frame_node,
            &frame_path,
            anchor_x,
            anchor_y,
            opacity,
            flipped,
            cache,
        )?;
        let delay_ms = integer_property(frame_node, "delay")
            .and_then(|delay| u16::try_from(delay).ok())
            .filter(|delay| *delay > 0)
            .unwrap_or(100);
        frames.push(SpriteFrameAsset {
            delay_ms,
            sprites: first_sprite..sprites.len(),
        });
    }
    Ok((frames.len() > 1).then_some(MapArtAnimationAsset {
        frames,
        environment_name: None,
        hold_last: false,
    }))
}

fn append_map_art_sprite(
    sprites: &mut Vec<DecodedSprite>,
    source: &openstory_assets::NxNode,
    path: &str,
    anchor_x: f32,
    anchor_y: f32,
    opacity: u8,
    flipped: bool,
    cache: &mut MapArtCache,
) -> Result<(), String> {
    let Some(art) = first_bitmap_node(source)? else {
        return Ok(());
    };
    let bitmap = art
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("map art {path:?} has no bitmap"))?;
    let (width, height) = (u32::from(bitmap.width()), u32::from(bitmap.height()));
    let origin = match art
        .child("origin")
        .map_err(|error| error.to_string())?
        .map(|node| node.value())
        .transpose()
        .map_err(|error| error.to_string())?
    {
        Some(NodeValue::Vector { x, y }) => (x as f32, y as f32),
        Some(_) => return Err(format!("map art {path:?} has a malformed origin")),
        None => (0.0, 0.0),
    };
    let x = anchor_x - origin.0;
    let y = anchor_y - origin.1;
    const CAMERA_CULL_MARGIN: f32 = 512.0;
    if x + width as f32 <= -CAMERA_CULL_MARGIN
        || y + height as f32 <= -CAMERA_CULL_MARGIN
        || x >= 1024.0 + CAMERA_CULL_MARGIN
        || y >= 768.0 + CAMERA_CULL_MARGIN
    {
        return Ok(());
    }

    let cached = if let Some(cached) = cache.get(path) {
        cached
    } else {
        let pixels = bitmap
            .decode_bgra_shared()
            .map_err(|error| error.to_string())?;
        let cached = (width, height, origin, pixels);
        cache.insert(path.to_owned(), cached.clone());
        cached
    };
    let texture_key = map_texture_key(path, opacity, flipped);
    let pixels = prepare_map_pixels(&cached.3, width, height, opacity, flipped);
    sprites.push(DecodedSprite {
        texture_key,
        rect: SpriteRect {
            x,
            y,
            width: width as f32,
            height: height as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: width,
        texture_height: height,
        pixels,
    });
    Ok(())
}

fn append_local_death_art(
    sprites: &mut Vec<DecodedSprite>,
    effect_archive: &NxArchive,
    character_archive: &NxArchive,
    look: &openstory_protocol::CharacterLook,
    cache: &mut MapArtCache,
) -> Result<Option<LocalDeathArtAsset>, String> {
    let Some(tomb) = effect_archive
        .root()
        .resolve("Tomb.img")
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let mut fall_frames = Vec::new();
    if let Some(fall) = tomb.child("fall").map_err(|error| error.to_string())? {
        for frame in numeric_children(&fall)? {
            let first_sprite = sprites.len();
            let frame_name = frame.name().map_err(|error| error.to_string())?;
            append_map_art_sprite(
                sprites,
                &frame,
                &format!("effect:Tomb.img/fall/{frame_name}"),
                512.0,
                GAMEPLAY_CAMERA_CENTER_Y,
                255,
                false,
                cache,
            )?;
            if sprites.len() > first_sprite {
                fall_frames.push(LocalDeathFrameAsset {
                    delay_ms: 30,
                    sprites: first_sprite..sprites.len(),
                });
            }
        }
    }

    let land_frame = match tomb.child("land").map_err(|error| error.to_string())? {
        Some(land) => land.child("0").map_err(|error| error.to_string())?,
        None => None,
    };
    let landed_sprites = if let Some(land) = land_frame {
        let first_sprite = sprites.len();
        append_map_art_sprite(
            sprites,
            &land,
            "effect:Tomb.img/land/0",
            512.0,
            GAMEPLAY_CAMERA_CENTER_Y,
            255,
            false,
            cache,
        )?;
        first_sprite..sprites.len()
    } else {
        sprites.len()..sprites.len()
    };

    let mut ghost_frames = Vec::new();
    if let Some(ghost_stand) = character_archive
        .root()
        .resolve("00002000.img/ghoststand/1")
        .map_err(|error| error.to_string())?
    {
        let ghost_anchor_y = GAMEPLAY_CAMERA_CENTER_Y - 30.0;
        let normal_body_path = format!("000020{:02}.img/stand1/0", look.skin);
        let normal_body = character_archive
            .root()
            .resolve(&format!("{normal_body_path}/body"))
            .map_err(|error| error.to_string())?;
        let normal_body_neck = normal_body
            .as_ref()
            .and_then(|body| vector_path(body, "map/neck"))
            .unwrap_or((0.0, 0.0));
        let head_path = format!("000120{:02}.img/stand1/0/head", look.skin);
        let head = character_archive
            .root()
            .resolve(&head_path)
            .map_err(|error| error.to_string())?;
        let head_neck = head
            .as_ref()
            .and_then(|head| vector_path(head, "map/neck"))
            .unwrap_or((0.0, 0.0));
        let head_brow = head
            .as_ref()
            .and_then(|head| vector_path(head, "map/brow"))
            .unwrap_or((0.0, 0.0));
        let head_offset = (
            normal_body_neck.0 - head_neck.0,
            normal_body_neck.1 - head_neck.1,
        );
        let face_offset = (head_offset.0 + head_brow.0, head_offset.1 + head_brow.1);
        let hair_path = format!("Hair/000{:05}.img/stand1/0", look.hair_id);
        let hair_frame = character_archive
            .root()
            .resolve(&hair_path)
            .map_err(|error| error.to_string())?;
        let face_path = format!("Face/000{:05}.img/default/face", look.face_id);
        let face = character_archive
            .root()
            .resolve(&face_path)
            .map_err(|error| error.to_string())?;

        for frame in numeric_children(&ghost_stand)? {
            let Some(body) = frame.child("body").map_err(|error| error.to_string())? else {
                continue;
            };
            let first_sprite = sprites.len();
            let frame_name = frame.name().map_err(|error| error.to_string())?;
            let body_path = format!("00002000.img/ghoststand/1/{frame_name}/body");
            append_map_art_sprite(
                sprites,
                &body,
                &format!("character:{body_path}"),
                512.0,
                ghost_anchor_y,
                255,
                false,
                cache,
            )?;

            let ghost_neck = vector_path(&body, "map/neck").unwrap_or((0.0, 0.0));
            let fake_origin = (
                512.0 + ghost_neck.0 - normal_body_neck.0,
                ghost_anchor_y + ghost_neck.1 - normal_body_neck.1,
            );
            if let Some(hair_frame) = hair_frame.as_ref() {
                append_hair_part(
                    sprites,
                    hair_frame,
                    "hairBelowBody",
                    &hair_path,
                    fake_origin,
                    face_offset,
                    cache,
                )?;
            }
            if let Some(head) = head.as_ref() {
                append_avatar_node(
                    sprites,
                    head,
                    &head_path,
                    (fake_origin.0 + head_offset.0, fake_origin.1 + head_offset.1),
                    cache,
                )?;
            }
            if let Some(hair_frame) = hair_frame.as_ref() {
                for part in ["hairShade", "hair"] {
                    append_hair_part(
                        sprites,
                        hair_frame,
                        part,
                        &hair_path,
                        fake_origin,
                        face_offset,
                        cache,
                    )?;
                }
            }
            if let Some(face) = face.as_ref() {
                let face_brow = vector_path(face, "map/brow").unwrap_or((0.0, 0.0));
                append_avatar_node(
                    sprites,
                    face,
                    &face_path,
                    (
                        fake_origin.0 + face_offset.0 - face_brow.0,
                        fake_origin.1 + face_offset.1 - face_brow.1,
                    ),
                    cache,
                )?;
            }
            if let Some(hair_frame) = hair_frame.as_ref() {
                append_hair_part(
                    sprites,
                    hair_frame,
                    "hairOverHead",
                    &hair_path,
                    fake_origin,
                    face_offset,
                    cache,
                )?;
            }
            if sprites.len() > first_sprite {
                ghost_frames.push(LocalDeathFrameAsset {
                    delay_ms: integer_property(&frame, "delay")
                        .and_then(|delay| u16::try_from(delay).ok())
                        .filter(|delay| *delay > 0)
                        .unwrap_or(500),
                    sprites: first_sprite..sprites.len(),
                });
            }
        }
    }
    if fall_frames.is_empty() && landed_sprites.is_empty() && ghost_frames.is_empty() {
        return Ok(None);
    }
    Ok(Some(LocalDeathArtAsset {
        fall_frames,
        landed_sprites,
        ghost_frames,
    }))
}
