const MAP_NAME_TAG_FONT_SIZE: f32 = 13.0;
const MAP_NAME_TAG_MAX_WIDTH: usize = 360;
const MAP_NAME_TAG_TEXT_BASELINE_NUDGE: i32 = -2;
const CHARACTER_NAME_TAG_Y_OFFSET: f32 = 3.0;

#[derive(Clone, Copy)]
enum MapNameTagKind {
    Character,
    Pet,
    Npc,
}

struct MapNameTagBitmap {
    width: usize,
    height: usize,
    origin_x: i32,
    origin_y: i32,
    pixels: Arc<[u8]>,
}

fn append_map_name_tag_sprite(
    sprites: &mut Vec<DecodedSprite>,
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    character_archive: Option<&NxArchive>,
    look: Option<&openstory_protocol::CharacterLook>,
    name: &str,
    center_x: f32,
    anchor_y: f32,
    kind: MapNameTagKind,
) -> Option<usize> {
    append_map_name_tag_sprite_scaled(
        sprites,
        font,
        ui_archive,
        character_archive,
        look,
        name,
        center_x,
        anchor_y,
        kind,
        1.0,
    )
}

fn append_map_name_tag_sprite_scaled(
    sprites: &mut Vec<DecodedSprite>,
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    character_archive: Option<&NxArchive>,
    look: Option<&openstory_protocol::CharacterLook>,
    name: &str,
    center_x: f32,
    anchor_y: f32,
    kind: MapNameTagKind,
    scale: f32,
) -> Option<usize> {
    append_map_name_tag_sprite_scaled_with_style(
        sprites,
        font,
        ui_archive,
        character_archive,
        look,
        name,
        center_x,
        anchor_y,
        kind,
        scale,
        None,
    )
}

fn append_pet_name_tag_sprite_scaled(
    sprites: &mut Vec<DecodedSprite>,
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    item_archive: &NxArchive,
    pet_id: i32,
    use_pet_style: bool,
    name: &str,
    center_x: f32,
    anchor_y: f32,
    scale: f32,
) -> Option<usize> {
    let style = use_pet_style
        .then(|| pet_name_tag_style_id(item_archive, pet_id))
        .flatten();
    append_map_name_tag_sprite_scaled_with_style(
        sprites,
        font,
        ui_archive,
        None,
        None,
        name,
        center_x,
        anchor_y,
        MapNameTagKind::Pet,
        scale,
        style,
    )
}

fn append_map_name_tag_sprite_scaled_with_style(
    sprites: &mut Vec<DecodedSprite>,
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    character_archive: Option<&NxArchive>,
    look: Option<&openstory_protocol::CharacterLook>,
    name: &str,
    center_x: f32,
    anchor_y: f32,
    kind: MapNameTagKind,
    scale: f32,
    style_override: Option<i32>,
) -> Option<usize> {
    let name = name.trim();
    if name.is_empty() || !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let font_size = MAP_NAME_TAG_FONT_SIZE * scale;
    let measured_width = measured_text_width(font, name, font_size)
        .ceil()
        .clamp(1.0, (MAP_NAME_TAG_MAX_WIDTH as f32 * scale - 16.0 * scale).max(1.0))
        as usize;

    let style = style_override
        .and_then(|style| load_name_tag_style(ui_archive, style, kind))
        .or_else(|| {
            look.zip(character_archive)
                .map(|(look, archive)| equipped_name_tag_styles(look, archive))
                .unwrap_or_default()
                .into_iter()
                .find_map(|style| load_name_tag_style(ui_archive, style, kind))
        });
    let (width, height, min_x, min_y, mut pixels, text_color, style_key, text_baseline) =
        if let Some(style) = style {
            let half_text_width = measured_width as f32 / 2.0;
            let name_center_x = 0.0_f32;
            let tag_y = 5.0_f32 * scale;
            let w_x = name_center_x - half_text_width - style.w.origin_x as f32 * scale;
            let c_x = name_center_x - half_text_width - style.c.origin_x as f32 * scale;
            let e_x = name_center_x + half_text_width - style.e.origin_x as f32 * scale;
            let w_y = tag_y - style.w.origin_y as f32 * scale;
            let c_y = tag_y - style.c.origin_y as f32 * scale;
            let e_y = tag_y - style.e.origin_y as f32 * scale;
            let min_x = w_x.min(c_x).min(e_x).floor() as i32;
            let min_y = w_y.min(c_y).min(e_y).floor() as i32;
            let max_x = (w_x + style.w.width as f32 * scale)
                .max(c_x + measured_width as f32)
                .max(e_x + style.e.width as f32 * scale)
                .ceil() as i32;
            let max_y = (w_y + style.w.height as f32 * scale)
                .max(c_y + style.c.height as f32 * scale)
                .max(e_y + style.e.height as f32 * scale)
                .ceil() as i32;
            let max_width = (MAP_NAME_TAG_MAX_WIDTH as f32 * scale).round() as i32;
            let max_height = (40.0 * scale).round() as i32;
            let width = (max_x - min_x).clamp(1, max_width.max(1)) as usize;
            let height = (max_y - min_y).clamp(1, max_height.max(1)) as usize;
            let mut pixels = vec![0_u8; width * height * 4];
            blit_map_name_tag_bitmap(
                &mut pixels,
                width,
                height,
                &style.w,
                w_x.floor() as i32 - min_x,
                w_y.floor() as i32 - min_y,
                (style.w.width as f32 * scale).round().max(1.0) as usize,
                (style.w.height as f32 * scale).round().max(1.0) as usize,
            );
            blit_map_name_tag_bitmap(
                &mut pixels,
                width,
                height,
                &style.c,
                c_x.floor() as i32 - min_x,
                c_y.floor() as i32 - min_y,
                measured_width.max(1),
                (style.c.height as f32 * scale).round().max(1.0) as usize,
            );
            blit_map_name_tag_bitmap(
                &mut pixels,
                width,
                height,
                &style.e,
                e_x.floor() as i32 - min_x,
                e_y.floor() as i32 - min_y,
                (style.e.width as f32 * scale).round().max(1.0) as usize,
                (style.e.height as f32 * scale).round().max(1.0) as usize,
            );
            let text_baseline = map_name_tag_text_baseline(height, font_size);
            (
                width,
                height,
                min_x,
                min_y,
                pixels,
                style.color,
                style.key,
                text_baseline,
            )
        } else {
            let (r, g, b) = match kind {
                MapNameTagKind::Character | MapNameTagKind::Pet => (255, 255, 255),
                MapNameTagKind::Npc => (255, 255, 0),
            };
            let width = (measured_width + (8.0 * scale).round() as usize)
                .min((MAP_NAME_TAG_MAX_WIDTH as f32 * scale).round() as usize);
            let height = (22.0 * scale).round().max(1.0) as usize;
            let mut pixels = vec![0_u8; width * height * 4];
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.copy_from_slice(&[0, 0, 0, 150]);
            }
            let baseline = map_name_tag_text_baseline(height, font_size);
            (
                width,
                height,
                -(width as i32 / 2),
                (2.0 * scale).round() as i32,
                pixels,
                (r, g, b, 255),
                "default".to_owned(),
                baseline,
            )
        };

    draw_map_name_tag_text(
        &mut pixels,
        width,
        height,
        font,
        name,
        font_size,
        text_color,
        text_baseline,
    );
    let panel_y_offset = match kind {
        MapNameTagKind::Character | MapNameTagKind::Pet => CHARACTER_NAME_TAG_Y_OFFSET * scale,
        MapNameTagKind::Npc => 0.0,
    };
    let texture_key = format!(
        "map-name-tag:{style_key}:{}-{}-{}-{}:{scale}:{name}",
        text_color.0, text_color.1, text_color.2, text_color.3,
    );
    let index = sprites.len();
    sprites.push(DecodedSprite {
        texture_key,
        rect: SpriteRect {
            x: center_x + min_x as f32,
            y: anchor_y + min_y as f32 + panel_y_offset,
            width: width as f32,
            height: height as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: width as u32,
        texture_height: height as u32,
        pixels: pixels.into(),
    });
    Some(index)
}

fn map_name_tag_text_baseline(height: usize, font_size: f32) -> i32 {
    ((height as f32 + font_size) / 2.0).round() as i32
        + (MAP_NAME_TAG_TEXT_BASELINE_NUDGE as f32 * font_size / MAP_NAME_TAG_FONT_SIZE).round() as i32
}

struct LoadedMapNameTagStyle {
    w: MapNameTagBitmap,
    c: MapNameTagBitmap,
    e: MapNameTagBitmap,
    color: (u8, u8, u8, u8),
    key: String,
}

pub(super) fn pet_name_tag_style_id(item_archive: &NxArchive, pet_id: i32) -> Option<i32> {
    pet_info_style_id(item_archive, pet_id, "nameTag")
}

pub(super) fn pet_chat_balloon_style_id(item_archive: &NxArchive, pet_id: i32) -> Option<i32> {
    pet_info_style_id(item_archive, pet_id, "chatBalloon")
}

fn pet_info_style_id(item_archive: &NxArchive, pet_id: i32, field: &str) -> Option<i32> {
    let info = item_archive
        .root()
        .resolve(&format!("Pet/{pet_id}.img/info"))
        .ok()??;
    integer_property(&info, field).and_then(|style| i32::try_from(style).ok())
}

fn equipped_name_tag_styles(
    look: &openstory_protocol::CharacterLook,
    character_archive: &NxArchive,
) -> Vec<i32> {
    look.equips
        .iter()
        .chain(look.masked_equips.iter())
        .filter(|(slot, _)| matches!(slot.unsigned_abs(), 12 | 13 | 15 | 16))
        .filter_map(|(_, item_id)| {
            let path = format!("Ring/0{item_id:07}.img/info");
            let info = character_archive.root().resolve(&path).ok().flatten()?;
            let style = integer_property(&info, "nameTag")?;
            (style > 0).then(|| i32::try_from(style).ok()).flatten()
        })
        .collect()
}

fn load_name_tag_style(
    ui_archive: &NxArchive,
    style_id: i32,
    kind: MapNameTagKind,
) -> Option<LoadedMapNameTagStyle> {
    let style_path = match kind {
        MapNameTagKind::Character => format!("NameTag.img/{style_id}"),
        MapNameTagKind::Pet => format!("NameTag.img/pet/{style_id}"),
        MapNameTagKind::Npc => return None,
    };
    let root = ui_archive
        .root()
        .resolve(&style_path)
        .ok()??;
    let w = load_map_name_tag_bitmap(ui_archive, &format!("{style_path}/w"))?;
    let c = load_map_name_tag_bitmap(ui_archive, &format!("{style_path}/c"))?;
    let e = load_map_name_tag_bitmap(ui_archive, &format!("{style_path}/e"))?;
    let packed = integer_property(&root, "clr").unwrap_or(-1) as u32;
    let mut alpha = (packed >> 24) as u8;
    if alpha == 0 {
        alpha = 255;
    }
    Some(LoadedMapNameTagStyle {
        w,
        c,
        e,
        color: (
            (packed >> 16) as u8,
            (packed >> 8) as u8,
            packed as u8,
            alpha,
        ),
        key: format!("{}:{style_id}", if matches!(kind, MapNameTagKind::Pet) { "pet-styled" } else { "styled" }),
    })
}

fn load_map_name_tag_bitmap(archive: &NxArchive, path: &str) -> Option<MapNameTagBitmap> {
    let node = archive.root().resolve(path).ok()??;
    let bitmap = node.bitmap().ok()??;
    let (width, height) = (usize::from(bitmap.width()), usize::from(bitmap.height()));
    if width == 0 || height == 0 {
        return None;
    }
    let origin = node
        .child("origin")
        .ok()??
        .value()
        .ok()
        .and_then(|value| match value {
            NodeValue::Vector { x, y } => Some((x, y)),
            _ => None,
        })
        .unwrap_or((0, 0));
    Some(MapNameTagBitmap {
        width,
        height,
        origin_x: origin.0,
        origin_y: origin.1,
        pixels: bitmap.decode_bgra_shared().ok()?,
    })
}

fn blit_map_name_tag_bitmap(
    canvas: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    bitmap: &MapNameTagBitmap,
    x: i32,
    y: i32,
    target_width: usize,
    target_height: usize,
) {
    let target_width = target_width.max(1);
    let target_height = target_height.max(1);
    for dest_x in 0..target_width {
        let source_x = dest_x * bitmap.width / target_width;
        for dest_y in 0..target_height {
            let source_y = dest_y * bitmap.height / target_height;
            let canvas_x = x + dest_x as i32;
            let canvas_y = y + dest_y as i32;
            if canvas_x < 0
                || canvas_y < 0
                || canvas_x >= canvas_width as i32
                || canvas_y >= canvas_height as i32
            {
                continue;
            }
            let source_index = (source_y * bitmap.width + source_x) * 4;
            let target_index = (canvas_y as usize * canvas_width + canvas_x as usize) * 4;
            blend_map_name_tag_pixel(canvas, target_index, &bitmap.pixels[source_index..source_index + 4]);
        }
    }
}

fn draw_map_name_tag_text(
    canvas: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    font: &fontdue::Font,
    text: &str,
    font_size: f32,
    color: (u8, u8, u8, u8),
    baseline: i32,
) {
    let text_width = measured_text_width(font, text, font_size);
    let mut pen_x = ((canvas_width as f32 - text_width) / 2.0).max(0.0);
    let mut previous = None;
    for character in text.chars() {
        if let Some(previous) = previous {
            pen_x += font
                .horizontal_kern(previous, character, font_size)
                .unwrap_or(0.0);
        }
        let (metrics, coverage) = rasterize_ui_glyph(font, character, font_size);
        let left = pen_x.round() as i32 + metrics.xmin;
        let top = baseline - metrics.ymin - metrics.height as i32;
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                let canvas_x = left + column as i32;
                let canvas_y = top + row as i32;
                if canvas_x < 0
                    || canvas_y < 0
                    || canvas_x >= canvas_width as i32
                    || canvas_y >= canvas_height as i32
                {
                    continue;
                }
                let alpha = (u16::from(coverage[row * metrics.width + column])
                    * u16::from(color.3)
                    / 255) as u8;
                if alpha == 0 {
                    continue;
                }
                let target_index = (canvas_y as usize * canvas_width + canvas_x as usize) * 4;
                blend_map_name_tag_pixel(
                    canvas,
                    target_index,
                    &[color.2, color.1, color.0, alpha],
                );
            }
        }
        pen_x += metrics.advance_width;
        if pen_x >= canvas_width as f32 {
            break;
        }
        previous = Some(character);
    }
}

fn blend_map_name_tag_pixel(canvas: &mut [u8], target_index: usize, source: &[u8]) {
    let alpha = u32::from(source[3]);
    let inverse = 255 - alpha;
    let destination_alpha = u32::from(canvas[target_index + 3]);
    let output_alpha = alpha + destination_alpha * inverse / 255;
    if output_alpha == 0 {
        return;
    }
    for channel in 0..3 {
        let source_channel = u32::from(source[channel]);
        let destination_channel = u32::from(canvas[target_index + channel]);
        let destination_contribution = destination_channel * destination_alpha * inverse / 255;
        canvas[target_index + channel] = ((source_channel * alpha + destination_contribution)
            / output_alpha) as u8;
    }
    canvas[target_index + 3] = output_alpha as u8;
}

fn append_map_name_tag_draws<'a>(
    scene: &MapSceneRuntime,
    textures: &'a [GpuTexture],
    draws: &mut Vec<SpriteDraw<'a>>,
) {
    let camera_bias_x = scene.camera_offset.0 + scene.player.x as f32 - 512.0;
    let camera_bias_y = scene.camera_offset.1 + scene.player.y as f32 - GAMEPLAY_CAMERA_CENTER_Y;

    let mut append = |label: Option<(usize, SpriteRect)>, offset_x: f32, offset_y: f32, opacity| {
        let Some((texture_index, rect)) = label else {
            return;
        };
        let Some(texture) = textures.get(texture_index) else {
            return;
        };
        draws.push(SpriteDraw {
            texture,
            x: rect.x + offset_x + camera_bias_x,
            y: rect.y + offset_y + camera_bias_y,
            width: rect.width,
            height: rect.height,
            uv: [0.0, 0.0, 1.0, 1.0],
            tint: [1.0, 1.0, 1.0],
            blend_mode: rect.blend_mode,
            flip_x: false,
            opacity,
        });
    };

    for npc in scene.npcs.values() {
        let offset_x = (npc.x - scene.player.x) as f32;
        let offset_y = (npc.y - scene.player.y) as f32;
        append(npc.name_label, offset_x, offset_y, 1.0);
        append(npc.function_label, offset_x, offset_y, 1.0);
    }
    for pet in scene.pets.values() {
        let offset_x = (pet.x - scene.player.x) as f32;
        let offset_y = (pet.y - scene.player.y) as f32;
        append(pet.name_label, offset_x, offset_y, 1.0);
    }
    for player in scene.remote_players.values() {
        let offset_x = (player.x - scene.player.x + player.recoil_offset_x) as f32;
        let offset_y = (player.y - scene.player.y) as f32;
        append(
            player.name_label,
            offset_x,
            offset_y,
            if player.hidden { 0.5 } else { 1.0 },
        );
    }
    append(
        scene.local_name_label,
        0.0,
        scene.local_chair.as_ref().map_or(0.0, |chair| chair.sit_offset_y),
        if scene.gm_hidden { 0.5 } else { 1.0 },
    );
}
