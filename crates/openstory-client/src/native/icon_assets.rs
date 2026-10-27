fn first_bitmap_node(
    node: &openstory_assets::NxNode,
) -> Result<Option<openstory_assets::NxNode>, String> {
    if node.bitmap().map_err(|error| error.to_string())?.is_some() {
        return Ok(Some(node.clone()));
    }
    for child in numeric_children(node)? {
        if child.bitmap().map_err(|error| error.to_string())?.is_some() {
            return Ok(Some(child));
        }
    }
    Ok(None)
}

fn decode_nx_icon(archive: &NxArchive, path: &str) -> Option<(u32, u32, Arc<[u8]>)> {
    let node = resolve_ui_path(&archive.root(), path).ok().flatten()?;
    let bitmap = first_bitmap_node(&node)
        .ok()
        .flatten()?
        .bitmap()
        .ok()
        .flatten()?;
    let width = u32::from(bitmap.width());
    let height = u32::from(bitmap.height());
    if width == 0 || height == 0 || width > 256 || height > 256 {
        return None;
    }
    Some((width, height, bitmap.decode_bgra_shared().ok()?))
}

fn quickslot_background_image(archive: &NxArchive) -> Option<(u32, u32, Arc<[u8]>)> {
    let node = archive
        .root()
        .resolve("StatusBar.img/base/quickSlot")
        .ok()
        .flatten()?;
    let bitmap = first_bitmap_node(&node)
        .ok()
        .flatten()?
        .bitmap()
        .ok()
        .flatten()?;
    let width = u32::from(bitmap.width());
    let height = u32::from(bitmap.height());
    if width == 0 || height == 0 || width > 512 || height > 512 {
        return None;
    }
    Some((width, height, bitmap.decode_bgra_shared().ok()?))
}

fn render_quickslot_panel_pixels(
    font: &fontdue::Font,
    background: Option<(u32, u32, Arc<[u8]>)>,
    keys: &[u8; 8],
    icons: &[Option<(u32, u32, Arc<[u8]>)>; 8],
) -> Vec<u8> {
    const WIDTH: usize = 151;
    const HEIGHT: usize = 80;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    if let Some((source_width, source_height, source)) = background {
        for y in 0..HEIGHT.min(source_height as usize) {
            let source_start = y * source_width as usize * 4;
            let destination_start = y * WIDTH * 4;
            let row_bytes = WIDTH.min(source_width as usize) * 4;
            pixels[destination_start..destination_start + row_bytes]
                .copy_from_slice(&source[source_start..source_start + row_bytes]);
        }
    } else {
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            0,
            0,
            WIDTH,
            HEIGHT,
            [42, 38, 31, 245],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            0,
            0,
            WIDTH,
            2,
            [196, 157, 91, 255],
        );
        for row in 0..2 {
            for column in 0..4 {
                fill_dialogue_rect(
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    7 + column * 35,
                    8 + row * 34,
                    32,
                    30,
                    [67, 61, 49, 255],
                );
            }
        }
    }

    for slot in 0..8 {
        let column = slot % 4;
        let row = slot / 4;
        let x = 7 + column * 35;
        let y = 8 + row * 34;
        if let Some((icon_width, icon_height, icon)) = &icons[slot] {
            composite_inventory_icon(
                &mut pixels,
                WIDTH,
                HEIGHT,
                icon,
                *icon_width as usize,
                *icon_height as usize,
                x + 5,
                y + 1,
                22,
            );
        }
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &quickslot_short_key_label(keys[slot]),
            x as i32 + 1,
            y as i32 + 20,
            8.0,
            30,
            10,
            [255, 247, 221],
        );
    }
    pixels
}

fn map_texture_key(path: &str, opacity: u8, flipped: bool) -> String {
    format!("{path}:opacity={opacity}:flip={flipped}")
}

fn prepare_map_pixels(
    pixels: &[u8],
    width: u32,
    height: u32,
    opacity: u8,
    flipped: bool,
) -> Arc<[u8]> {
    if width == 0 || height == 0 {
        return Arc::from(pixels);
    }
    if opacity == 255 && !flipped {
        return Arc::from(pixels);
    }
    let mut output = pixels.to_vec();
    if flipped {
        let row_bytes = width as usize * 4;
        for row in output.chunks_exact_mut(row_bytes).take(height as usize) {
            for x in 0..width as usize / 2 {
                let opposite = width as usize - 1 - x;
                for channel in 0..4 {
                    row.swap(x * 4 + channel, opposite * 4 + channel);
                }
            }
        }
    }
    if opacity != 255 {
        for pixel in output.chunks_exact_mut(4) {
            pixel[3] = ((u16::from(pixel[3]) * u16::from(opacity)) / 255) as u8;
        }
    }
    Arc::from(output)
}

fn append_map_background(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    entry: &openstory_assets::NxNode,
    camera_x: f32,
    camera_y: f32,
    cache: &mut MapArtCache,
    animations: &mut Vec<MapArtAnimationAsset>,
) -> Result<(), String> {
    let background_set = string_property(entry, "bS").unwrap_or_default();
    if background_set.is_empty() {
        return Ok(());
    }
    let art_index = integer_property(entry, "no").unwrap_or(0);
    let animated = integer_property(entry, "ani").unwrap_or(0) != 0;
    let folder = if animated { "ani" } else { "back" };
    let path = format!("Back/{background_set}.img/{folder}/{art_index}");
    let Some(art_root) = archive
        .root()
        .resolve(&path)
        .map_err(|error| error.to_string())?
    else {
        return Ok(());
    };
    let root_is_bitmap = art_root
        .bitmap()
        .map_err(|error| error.to_string())?
        .is_some();
    let mut frame_nodes = if animated && !root_is_bitmap {
        numeric_children(&art_root)?
    } else {
        Vec::new()
    };
    if frame_nodes.is_empty() {
        frame_nodes.push(art_root);
    }
    let mut frames = Vec::new();
    for frame_node in &frame_nodes {
        let first_sprite = sprites.len();
        let frame_path = if animated && !root_is_bitmap {
            format!(
                "{path}/{}",
                frame_node.name().map_err(|error| error.to_string())?
            )
        } else {
            path.clone()
        };
        append_map_background_frame(
            sprites,
            entry,
            frame_node,
            &frame_path,
            camera_x,
            camera_y,
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
    if frames.len() > 1 {
        animations.push(MapArtAnimationAsset {
            frames,
            environment_name: None,
            hold_last: false,
        });
    }
    Ok(())
}

fn append_map_background_frame(
    sprites: &mut Vec<DecodedSprite>,
    entry: &openstory_assets::NxNode,
    art: &openstory_assets::NxNode,
    path: &str,
    camera_x: f32,
    camera_y: f32,
    cache: &mut MapArtCache,
) -> Result<(), String> {
    let Some(bitmap) = art.bitmap().map_err(|error| error.to_string())? else {
        return Ok(());
    };
    let cache_entry = if let Some(cached) = cache.get(path) {
        cached
    } else {
        let origin = match art
            .child("origin")
            .map_err(|error| error.to_string())?
            .map(|node| node.value())
            .transpose()
            .map_err(|error| error.to_string())?
        {
            Some(NodeValue::Vector { x, y }) => (x as f32, y as f32),
            Some(_) => return Err(format!("map background {path:?} has a malformed origin")),
            None => (0.0, 0.0),
        };
        let pixels = bitmap
            .decode_bgra_shared()
            .map_err(|error| error.to_string())?;
        let cached = (
            u32::from(bitmap.width()),
            u32::from(bitmap.height()),
            origin,
            pixels,
        );
        cache.insert(path.to_owned(), cached.clone());
        cached
    };

    let kind = integer_property(entry, "type").unwrap_or(0);
    let rx = number_property(entry, "rx").unwrap_or(0.0);
    let ry = number_property(entry, "ry").unwrap_or(0.0);
    let horizontal_mobile = matches!(kind, 4 | 6) && rx != 0.0;
    let vertical_mobile = matches!(kind, 5 | 7) && ry != 0.0;
    let anchor_x = if horizontal_mobile {
        camera_x + number_property(entry, "x").unwrap_or(0.0)
    } else {
        512.0 + number_property(entry, "x").unwrap_or(0.0) + rx * (512.0 - camera_x) / 100.0
    };
    let anchor_y = if vertical_mobile {
        camera_y + number_property(entry, "y").unwrap_or(0.0)
    } else {
        GAMEPLAY_CAMERA_CENTER_Y
            + number_property(entry, "y").unwrap_or(0.0)
            + ry * (GAMEPLAY_CAMERA_CENTER_Y - camera_y) / 100.0
    };
    let tile_x = integer_property(entry, "cx")
        .filter(|value| *value > 0)
        .unwrap_or(i64::from(cache_entry.0)) as f32;
    let tile_y = integer_property(entry, "cy")
        .filter(|value| *value > 0)
        .unwrap_or(i64::from(cache_entry.1)) as f32;
    let horizontal = matches!(kind, 1 | 3 | 4 | 6);
    let vertical = matches!(kind, 2 | 3 | 5 | 7);
    let (count_x, count_y) = map_background_tile_counts(kind, tile_x, tile_y);
    let horizontal_scrolls = horizontal_mobile || rx != 0.0;
    let vertical_scrolls = vertical_mobile || ry != 0.0;
    let base_x = if horizontal {
        normalize_tiled_position(anchor_x, tile_x)
    } else {
        anchor_x
    };
    let base_y = if vertical {
        normalize_tiled_position(anchor_y, tile_y)
    } else {
        anchor_y
    };
    let opacity = integer_property(entry, "a").unwrap_or(255).clamp(0, 255) as u8;
    let flipped = integer_property(entry, "f").unwrap_or(0) != 0;
    let texture_key = map_texture_key(&path, opacity, flipped);
    let pixels = prepare_map_pixels(
        &cache_entry.3,
        cache_entry.0,
        cache_entry.1,
        opacity,
        flipped,
    );
    let additive = integer_property(&art, "blend").unwrap_or(0) != 0
        || art
            .child("0")
            .ok()
            .flatten()
            .and_then(|frame| integer_property(&frame, "blend"))
            .unwrap_or(0)
            != 0;
    let blend_mode = if additive {
        BlendMode::Additive
    } else {
        BlendMode::Alpha
    };
    for x_index in 0..count_x {
        for y_index in 0..count_y {
            let x = base_x + x_index as f32 * tile_x - cache_entry.2.0;
            let y = base_y + y_index as f32 * tile_y - cache_entry.2.1;
            // Keep offscreen copies that can scroll into view; tiled backgrounds recycle
            // their reserved copies as they leave the viewport.
            if (!horizontal
                && !horizontal_scrolls
                && (x + cache_entry.0 as f32 <= 0.0 || x >= 1024.0))
                || (!vertical
                    && !vertical_scrolls
                    && (y + cache_entry.1 as f32 <= 0.0 || y >= 768.0))
            {
                continue;
            }
            sprites.push(DecodedSprite {
                texture_key: texture_key.clone(),
                rect: SpriteRect {
                    x,
                    y,
                    width: cache_entry.0 as f32,
                    height: cache_entry.1 as f32,
                    blend_mode,
                },
                depth_order: 0,
                texture_width: cache_entry.0,
                texture_height: cache_entry.1,
                pixels: pixels.clone(),
            });
        }
    }
    Ok(())
}

fn normalize_tiled_position(position: f32, interval: f32) -> f32 {
    let interval = interval.max(1.0);
    let remainder = position.rem_euclid(interval);
    if remainder == 0.0 {
        0.0
    } else {
        remainder - interval
    }
}

fn map_spawn_point(map: &openstory_assets::NxNode, portal_id: u8) -> Result<(f32, f32), String> {
    let Some(portals) = map.child("portal").map_err(|error| error.to_string())? else {
        return Ok((0.0, 0.0));
    };
    let selected = portals
        .child(&portal_id.to_string())
        .map_err(|error| error.to_string())?;
    let portal = if let Some(selected) = selected {
        Some(selected)
    } else {
        numeric_children(&portals)?
            .into_iter()
            .find(|node| string_property(node, "pn").as_deref() == Some("sp"))
    };
    let Some(portal) = portal else {
        return Ok((0.0, 0.0));
    };
    Ok((
        number_property(&portal, "x").unwrap_or(0.0),
        // The legacy MapPortals::get_portal_by_id spawn point sits 30 px
        // above the portal anchor before snapping down to a foothold.
        number_property(&portal, "y").unwrap_or(0.0) - 30.0,
    ))
}

fn map_camera_bounds(
    map: &openstory_assets::NxNode,
    footholds: Option<&FootholdTree>,
) -> Option<MapCameraBounds> {
    let info = map.child("info").ok().flatten();
    let authored = info.and_then(|info| {
        Some(MapCameraBounds {
            left: integer_property(&info, "VRLeft")? as f32,
            right: integer_property(&info, "VRRight")? as f32,
            top: integer_property(&info, "VRTop")? as f32,
            bottom: integer_property(&info, "VRBottom")? as f32,
        })
    });
    if let Some(bounds) =
        authored.filter(|bounds| bounds.right > bounds.left && bounds.bottom > bounds.top)
    {
        return Some(bounds);
    }
    footholds.map(|footholds| {
        let bounds = footholds.bounds();
        MapCameraBounds {
            left: f32::from(bounds.left_wall),
            right: f32::from(bounds.right_wall),
            top: f32::from(bounds.top),
            bottom: f32::from(bounds.bottom),
        }
    })
}

fn load_map_footholds(map: &openstory_assets::NxNode) -> Result<Option<FootholdTree>, String> {
    let Some(root) = map.child("foothold").map_err(|error| error.to_string())? else {
        return Ok(None);
    };
    let mut footholds = Vec::new();
    for layer_node in numeric_children(&root)? {
        let layer = layer_node
            .name()
            .map_err(|error| error.to_string())?
            .parse::<u8>()
            .ok();
        let Some(layer) = layer else {
            continue;
        };
        for group in numeric_children(&layer_node)? {
            for segment in numeric_children(&group)? {
                let id = segment
                    .name()
                    .map_err(|error| error.to_string())?
                    .parse::<u16>()
                    .ok();
                let Some(id) = id.filter(|id| *id != 0) else {
                    continue;
                };
                let Some(previous) =
                    integer_property(&segment, "prev").and_then(|value| u16::try_from(value).ok())
                else {
                    continue;
                };
                let Some(next) =
                    integer_property(&segment, "next").and_then(|value| u16::try_from(value).ok())
                else {
                    continue;
                };
                let point = |name| {
                    integer_property(&segment, name).and_then(|value| i16::try_from(value).ok())
                };
                let (Some(x1), Some(y1), Some(x2), Some(y2)) =
                    (point("x1"), point("y1"), point("x2"), point("y2"))
                else {
                    continue;
                };
                footholds.push(Foothold {
                    id,
                    previous,
                    next,
                    layer,
                    x1,
                    y1,
                    x2,
                    y2,
                });
            }
        }
    }
    if footholds.is_empty() {
        return Ok(None);
    }
    FootholdTree::new(footholds)
        .map(Some)
        .map_err(|error| format!("invalid map footholds: {error}"))
}

fn numeric_children(
    node: &openstory_assets::NxNode,
) -> Result<Vec<openstory_assets::NxNode>, String> {
    let mut children = node
        .children()
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter_map(|child| {
            child
                .name()
                .ok()
                .and_then(|name| name.parse::<u32>().ok().map(|index| (index, child)))
        })
        .collect::<Vec<_>>();
    children.sort_by_key(|(index, _)| *index);
    Ok(children.into_iter().map(|(_, child)| child).collect())
}

fn learned_skill_level(
    archive: &NxArchive,
    skills: &HashMap<i32, openstory_protocol::SkillEntry>,
    skill_id: i32,
) -> Option<openstory_assets::NxNode> {
    let level = skills.get(&skill_id)?.level;
    if level <= 0 {
        return None;
    }
    let path = format!(
        "{:03}.img/skill/{skill_id:07}/level/{level}",
        skill_id / 10_000
    );
    resolve_ui_path(&archive.root(), &path).ok().flatten()
}

fn magic_amplification_skill(job: u16) -> Option<i32> {
    match job {
        210..=212 => Some(2_110_001),
        220..=222 => Some(2_210_001),
        1_111 | 1_112 => Some(12_110_001),
        2_215..=2_218 => Some(22_150_000),
        _ => None,
    }
}

fn legacy_y_below(footholds: &FootholdTree, x: i16, y: i16) -> i16 {
    footholds
        .find_below(f64::from(x), f64::from(y))
        .map(|(_, ground_y)| {
            (ground_y
                .trunc()
                .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16)
                .saturating_sub(1)
        })
        .unwrap_or_else(|| footholds.bounds().bottom.saturating_sub(1))
}

fn integer_property(node: &openstory_assets::NxNode, name: &str) -> Option<i64> {
    match node
        .child(name)
        .ok()?
        .and_then(|value| value.value().ok())?
    {
        NodeValue::Integer(value) => Some(value),
        _ => None,
    }
}

fn maple_tv_screen_anchors(archive: &NxArchive, npc_id: i32) -> Option<MapleTvAnchors> {
    let is_maple_tv_npc = npc_id == 9_201_066
        || (9_250_020..=9_250_100).contains(&npc_id)
        || (9_270_000..9_271_000).contains(&npc_id);
    if !is_maple_tv_npc {
        return None;
    }
    let path = format!("{npc_id:07}.img/info");
    let info = archive
        .root()
        .resolve(&path)
        .ok()?
        .and_then(|npc| npc.child("info").ok().flatten())?;
    let read_short = |name| integer_property(&info, name).map_or(0, |value| value as u16 as i16);
    let anchors = MapleTvAnchors {
        message_x: read_short("MapleTVmsgX"),
        message_y: read_short("MapleTVmsgY"),
        ad_x: read_short("MapleTVadX"),
        ad_y: read_short("MapleTVadY"),
    };
    (anchors.ad_x != 0 || anchors.ad_y != 0).then_some(anchors)
}

fn read_ui_bitmap(
    archive: &NxArchive,
    path: &str,
) -> Result<(u32, u32, Arc<[u8]>, (i16, i16)), String> {
    let node = resolve_ui_path(&archive.root(), path)?
        .ok_or_else(|| format!("UI bitmap {path:?} was not found"))?;
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("UI node {path:?} is not a bitmap"))?;
    let origin = match node
        .child("origin")
        .map_err(|error| error.to_string())?
        .map(|origin| origin.value())
        .transpose()
        .map_err(|error| error.to_string())?
    {
        Some(NodeValue::Vector { x, y }) => (x as i16, y as i16),
        _ => (0, 0),
    };
    Ok((
        u32::from(bitmap.width()),
        u32::from(bitmap.height()),
        bitmap
            .decode_bgra_shared()
            .map_err(|error| error.to_string())?,
        origin,
    ))
}

fn number_property(node: &openstory_assets::NxNode, name: &str) -> Option<f32> {
    match node
        .child(name)
        .ok()?
        .and_then(|value| value.value().ok())?
    {
        NodeValue::Integer(value) => Some(value as f32),
        NodeValue::Real(value) => Some(value as f32),
        _ => None,
    }
}

fn string_property(node: &openstory_assets::NxNode, name: &str) -> Option<String> {
    match node
        .child(name)
        .ok()?
        .and_then(|value| value.value().ok())?
    {
        NodeValue::String(value) => Some(value),
        _ => None,
    }
}
