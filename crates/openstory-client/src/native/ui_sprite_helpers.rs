/// Resolves UI window art across the v83 and newer NX layouts. The v83 archive
/// puts these windows in `UIWindow.img`; newer archives split them into
/// `UIWindow2.img`/`UIWindow4.img` and rename several window groups.
fn resolve_ui_path(root: &NxNode, path: &str) -> Result<Option<NxNode>, String> {
    if let Some(node) = root.resolve(path).map_err(|error| error.to_string())? {
        return Ok(Some(node));
    }

    let Some(suffix) = ["UIWindow2.img/", "UIWindow3.img/", "UIWindow4.img/"]
        .iter()
        .find_map(|prefix| path.strip_prefix(prefix))
    else {
        return Ok(None);
    };

    let aliases = [
        ("Equip/character/", "Equip/"),
        ("Skill/main/", "Skill/"),
        ("Shop2/", "Shop/"),
        ("OptionMenu/", "GameOpt/"),
        ("Stat/main/", "Stat/"),
        ("UserList/Main/Party/partyOn", "UserList/Party/party0"),
        ("UserList/Main/Friend/title", "UserList/Friend/friend0"),
        ("UserList/Sheet2/0", "UserList/Party/party0"),
        ("UserList/Sheet2/1", "UserList/Party/party1"),
        ("UserList/line", "UserList/Friend/line"),
        ("UserList/Main/", "UserList/"),
    ];
    for (from, to) in aliases {
        if let Some(rest) = suffix.strip_prefix(from) {
            let candidate = format!("UIWindow.img/{to}{rest}");
            if let Some(node) = root
                .resolve(&candidate)
                .map_err(|error| error.to_string())?
            {
                return Ok(Some(node));
            }
        }
    }

    let candidate = format!("UIWindow.img/{suffix}");
    root.resolve(&candidate).map_err(|error| error.to_string())
}

/// Composites authored NX window layers in order. Callers provide each layer's
/// legacy anchor; `composite_nx_ui_bitmap` applies the bitmap's NX origin and
/// `resolve_ui_path` handles aliases between newer and v83 path layouts.
fn composite_nx_ui_layers(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    archive: &NxArchive,
    layers: &[(&str, i32, i32)],
) -> Result<(), String> {
    for (path, anchor_x, anchor_y) in layers {
        composite_nx_ui_bitmap(
            pixels,
            canvas_width,
            canvas_height,
            archive,
            path,
            *anchor_x,
            *anchor_y,
        )?;
    }
    Ok(())
}

/// Draws a v83 NX bitmap into a panel-sized rectangle using nearest-neighbor
/// scaling so pixel-art edges remain crisp. The requested position is the
/// destination's top-left; source NX origins are intentionally ignored because
/// this is for authored panel backdrops, not individual controls.
fn composite_nx_ui_bitmap_scaled_to(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    archive: &NxArchive,
    path: &str,
    left: usize,
    top: usize,
    target_width: usize,
    target_height: usize,
) -> Result<(), String> {
    if target_width == 0 || target_height == 0 {
        return Ok(());
    }
    let node = resolve_ui_path(&archive.root(), path)?
        .ok_or_else(|| format!("missing v83 UI asset {path:?}"))?;
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("v83 UI asset {path:?} is not a bitmap"))?;
    let source = bitmap
        .decode_bgra_shared()
        .map_err(|error| error.to_string())?;
    let source_width = usize::from(bitmap.width());
    let source_height = usize::from(bitmap.height());
    if source_width == 0 || source_height == 0 {
        return Err(format!("v83 UI asset {path:?} has empty dimensions"));
    }

    for destination_y in 0..target_height {
        let canvas_y = top.saturating_add(destination_y);
        if canvas_y >= canvas_height {
            continue;
        }
        let source_y = destination_y * source_height / target_height;
        for destination_x in 0..target_width {
            let canvas_x = left.saturating_add(destination_x);
            if canvas_x >= canvas_width {
                continue;
            }
            let source_x = destination_x * source_width / target_width;
            let source_index = (source_y * source_width + source_x) * 4;
            let destination_index = (canvas_y * canvas_width + canvas_x) * 4;
            let source_alpha = u32::from(source[source_index + 3]);
            if source_alpha == 0 {
                continue;
            }
            let destination_alpha = u32::from(pixels[destination_index + 3]);
            let inverse = 255 - source_alpha;
            let output_alpha = source_alpha + destination_alpha * inverse / 255;
            for channel in 0..3 {
                let foreground = u32::from(source[source_index + channel]);
                let background = u32::from(pixels[destination_index + channel]);
                let premultiplied = foreground * source_alpha
                    + background * destination_alpha * inverse / 255;
                pixels[destination_index + channel] =
                    (premultiplied / output_alpha.max(1)) as u8;
            }
            pixels[destination_index + 3] = output_alpha as u8;
        }
    }
    Ok(())
}

fn append_login_button(
    sprites: &mut Vec<DecodedSprite>,
    actions: &mut Vec<LoginActionRegion>,
    buttons: &mut Vec<LoginButtonAsset>,
    archive: &NxArchive,
    button_name: &str,
    anchor_x: f32,
    anchor_y: f32,
    action: Option<LoginAction>,
) -> Result<(), String> {
    let normal_draw = sprites.len();
    append_sprite(
        sprites,
        archive,
        &format!("Login.img/Title/{button_name}/normal/0"),
        anchor_x,
        anchor_y,
        1.0,
    )?;
    let hit_rect = sprites[normal_draw].rect;
    if let Some(action) = action {
        actions.push(LoginActionRegion {
            rect: hit_rect,
            action,
        });
    }
    let mut append_state = |state: &str| -> Result<Option<usize>, String> {
        let path = format!("Login.img/Title/{button_name}/{state}/0");
        let index = sprites.len();
        append_optional_sprite(sprites, archive, &path, anchor_x, anchor_y, 1.0)?;
        Ok((sprites.len() > index).then_some(index))
    };
    let mouse_over_draw = append_state("mouseOver")?;
    let pressed_draw = append_state("pressed")?;
    let disabled_draw = append_state("disabled")?;
    let key_focused_draw = append_state("keyFocused")?;
    buttons.push(LoginButtonAsset {
        hit_rect,
        action,
        normal_draw,
        mouse_over_draw,
        pressed_draw,
        disabled_draw,
        key_focused_draw,
    });
    Ok(())
}

fn append_sprite(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    path: &str,
    anchor_x: f32,
    anchor_y: f32,
    scale: f32,
) -> Result<(), String> {
    let node = resolve_ui_path(&archive.root(), path)?
        .ok_or_else(|| format!("NX node {path:?} was not found"))?;
    append_node_sprite(sprites, &node, path, anchor_x, anchor_y, scale)
}

fn append_optional_sprite(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    path: &str,
    anchor_x: f32,
    anchor_y: f32,
    scale: f32,
) -> Result<Option<SpriteRect>, String> {
    let node = resolve_ui_path(&archive.root(), path)?;
    let Some(node) = node else {
        return Ok(None);
    };
    let index = sprites.len();
    append_node_sprite(sprites, &node, path, anchor_x, anchor_y, scale)?;
    Ok(Some(sprites[index].rect))
}

fn append_scaled_ui_animation(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    path: &str,
    anchor_x: f32,
    anchor_y: f32,
    scale: f32,
) -> Result<Option<MapArtAnimationAsset>, String> {
    let Some(source) = resolve_ui_path(&archive.root(), path)? else {
        return Ok(None);
    };
    let mut frame_nodes = numeric_children(&source)?;
    if frame_nodes.is_empty() {
        frame_nodes.push(source);
    }
    let mut frames = Vec::new();
    for frame_node in frame_nodes {
        let frame_name = frame_node.name().map_err(|error| error.to_string())?;
        let frame_path = format!("{path}/{frame_name}");
        let first_sprite = sprites.len();
        if append_optional_sprite(sprites, archive, &frame_path, anchor_x, anchor_y, scale)?
            .is_none()
        {
            continue;
        }
        let delay_ms = integer_property(&frame_node, "delay")
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

fn append_optional_sprite_at_top_left(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    path: &str,
    left: f32,
    top: f32,
    scale: f32,
) -> Result<Option<SpriteRect>, String> {
    let node = resolve_ui_path(&archive.root(), path)?;
    let Some(node) = node else {
        return Ok(None);
    };
    let origin = match node
        .child("origin")
        .map_err(|error| error.to_string())?
        .map(|origin| origin.value())
        .transpose()
        .map_err(|error| error.to_string())?
    {
        Some(NodeValue::Vector { x, y }) => (x as f32, y as f32),
        Some(_) => return Err(format!("NX sprite {path:?} has a malformed origin")),
        None => (0.0, 0.0),
    };
    let index = sprites.len();
    append_node_sprite(
        sprites,
        &node,
        path,
        left + origin.0 * scale,
        top + origin.1 * scale,
        scale,
    )?;
    Ok(Some(sprites[index].rect))
}

fn build_quit_confirmation_sprites(
    archive: &NxArchive,
) -> Result<(Vec<DecodedSprite>, [SpriteRect; 2]), String> {
    // Prefer the v83 login modal. The larger v83 frame uses different
    // proportions than the modern login modal, so center its text and
    // button group from their actual NX dimensions instead of reusing the
    // modern panel's fixed 17/70/130 offsets.
    let background_path = "Login.img/Notice/backgrnd/0";
    let background = archive
        .root()
        .resolve(background_path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("missing native quit-dialog asset {background_path:?}"))?;
    let bitmap = background
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("native quit-dialog asset {background_path:?} is not a bitmap"))?;
    let panel_width = f32::from(bitmap.width());
    let panel_height = f32::from(bitmap.height());
    let left = (1024.0 - panel_width) / 2.0;
    let top = (768.0 - panel_height) / 2.0;
    let mut sprites = Vec::with_capacity(4);
    let mut append_required = |path: &str, x, y| {
        append_optional_sprite_at_top_left(&mut sprites, archive, path, x, y, 1.0)?
            .ok_or_else(|| format!("missing native quit-dialog asset {path:?}"))
    };
    append_required(background_path, left, top)?;

    let text_path = "Login.img/Notice/text/53";
    let text_node = archive
        .root()
        .resolve(text_path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("missing native quit-dialog asset {text_path:?}"))?;
    let text_bitmap = text_node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("native quit-dialog asset {text_path:?} is not a bitmap"))?;
    let text_width = f32::from(text_bitmap.width());
    let text_left = left + (panel_width - text_width) / 2.0;
    append_required(text_path, text_left, top + 13.0)?;
    drop(append_required);

    // The prompt bitmap has an opaque paper-colored area below its glyphs, so
    // it hides the mushroom artwork baked into the notice background. Restore
    // just that part of the background after the text; the crop starts below
    // the prompt glyphs and keeps the mushroom face in front.
    let mushroom_crop = (32_u32, 45_u32, 72_u32, 70_u32);
    let (crop_x, crop_y, crop_width, crop_height) = mushroom_crop;
    if crop_x.saturating_add(crop_width) <= u32::from(bitmap.width())
        && crop_y.saturating_add(crop_height) <= u32::from(bitmap.height())
    {
        let background_pixels = bitmap
            .decode_bgra_shared()
            .map_err(|error| error.to_string())?;
        let source_width = usize::from(bitmap.width());
        let mut crop_pixels = vec![0_u8; crop_width as usize * crop_height as usize * 4];
        for row in 0..crop_height as usize {
            let source_start =
                ((crop_y as usize + row) * source_width + crop_x as usize) * 4;
            let destination_start = row * crop_width as usize * 4;
            let row_bytes = crop_width as usize * 4;
            crop_pixels[destination_start..destination_start + row_bytes]
                .copy_from_slice(&background_pixels[source_start..source_start + row_bytes]);
        }
        sprites.push(DecodedSprite {
            texture_key: format!("{background_path}#mushroom-foreground"),
            rect: SpriteRect {
                x: left + crop_x as f32,
                y: top + crop_y as f32,
                width: crop_width as f32,
                height: crop_height as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: crop_width,
            texture_height: crop_height,
            pixels: crop_pixels.into(),
        });
    }

    let mut append_required = |path: &str, x, y| {
        append_optional_sprite_at_top_left(&mut sprites, archive, path, x, y, 1.0)?
            .ok_or_else(|| format!("missing native quit-dialog asset {path:?}"))
    };
    let yes_path = "Login.img/Notice/BtYes/normal/0";
    let no_path = "Login.img/Notice/BtNo/normal/0";
    let button_width = |path: &str| -> Result<f32, String> {
        let node = archive
            .root()
            .resolve(path)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("missing native quit-dialog asset {path:?}"))?;
        let bitmap = node
            .bitmap()
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("native quit-dialog asset {path:?} is not a bitmap"))?;
        Ok(f32::from(bitmap.width()))
    };
    let yes_width = button_width(yes_path)?;
    let no_width = button_width(no_path)?;
    let gap = 8.0_f32;
    let group_width = yes_width + gap + no_width;
    let buttons_left = left + (panel_width - group_width) / 2.0;
    let button_y = top + panel_height - 56.0;
    let yes = append_required(yes_path, buttons_left, button_y)?;
    let no = append_required(no_path, buttons_left + yes_width + gap, button_y)?;
    Ok((sprites, [yes, no]))
}

fn build_server_notice_art(
    archive: &NxArchive,
    font: &fontdue::Font,
    message: &str,
) -> Result<(u32, u32, Vec<u8>, SpriteRect, SpriteRect), String> {
    let background_path = "Login.img/Notice/backgrnd/0";
    let background = archive
        .root()
        .resolve(background_path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("missing v83 server-notice asset {background_path:?}"))?;
    let bitmap = background
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("v83 server-notice asset {background_path:?} is not a bitmap"))?;
    let width = u32::from(bitmap.width());
    let height = u32::from(bitmap.height());
    let mut pixels = bitmap
        .decode_bgra_shared()
        .map_err(|error| error.to_string())?
        .to_vec();
    let panel = SpriteRect {
        x: (1024.0 - width as f32) / 2.0,
        y: (768.0 - height as f32) / 2.0,
        width: width as f32,
        height: height as f32,
        blend_mode: BlendMode::Alpha,
    };
    let button_width = 76_u32.min(width.saturating_sub(24));
    let button_height = 34_u32.min(height.saturating_sub(24));
    let button = SpriteRect {
        x: panel.x + (width - button_width) as f32 / 2.0,
        y: panel.y + height as f32 - button_height as f32 - 22.0,
        width: button_width as f32,
        height: button_height as f32,
        blend_mode: BlendMode::Alpha,
    };

    let font_size = 14.0;
    let text_left = 104_u32.min(width.saturating_sub(24));
    let content_width = width.saturating_sub(text_left + 24);
    let lines = wrap_server_notice(font, message, font_size, content_width as f32, 4);
    let line_height = font_size.ceil() as u32 + 5;
    let text_top = 30;
    for (line_index, line) in lines.iter().enumerate() {
        let line_width = measured_text_width(font, line, font_size)
            .ceil()
            .clamp(1.0, content_width as f32) as u32
            + 2;
        let label = rasterize_label_color(
            font,
            line,
            font_size,
            line_width,
            line_height,
            [55, 40, 29],
        );
        let top = text_top + line_index as u32 * line_height;
        blend_notice_label(
            &mut pixels,
            width,
            height,
            text_left,
            top,
            &label,
            line_width,
            line_height,
        );
    }

    let button_x = (button.x - panel.x) as u32;
    let button_y = (button.y - panel.y) as u32;
    fill_notice_rect(
        &mut pixels,
        width,
        height,
        button_x,
        button_y,
        button_width,
        button_height,
        [165, 194, 221, 255],
    );
    stroke_notice_rect(
        &mut pixels,
        width,
        height,
        button_x,
        button_y,
        button_width,
        button_height,
        [55, 93, 130, 255],
    );
    let label_width = measured_text_width(font, "OK", font_size)
        .ceil()
        .max(1.0) as u32
        + 2;
    let label_height = font_size.ceil() as u32 + 5;
    let label = rasterize_label_color(
        font,
        "OK",
        font_size,
        label_width,
        label_height,
        [35, 48, 69],
    );
    let label_left = button_x + button_width.saturating_sub(label_width) / 2;
    let label_top = button_y + button_height.saturating_sub(label_height) / 2;
    blend_notice_label(
        &mut pixels,
        width,
        height,
        label_left,
        label_top,
        &label,
        label_width,
        label_height,
    );
    Ok((width, height, pixels, panel, button))
}

fn wrap_server_notice(
    font: &fontdue::Font,
    message: &str,
    font_size: f32,
    max_width: f32,
    max_lines: usize,
) -> Vec<String> {
    let bounded_message = message.chars().take(512).collect::<String>();
    let mut lines = Vec::new();
    for paragraph in bounded_message.replace('\r', "").split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if measured_text_width(font, &candidate, font_size) <= max_width {
                line = candidate;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                if lines.len() >= max_lines {
                    return lines;
                }
            }
            for character in word.chars() {
                let candidate = format!("{line}{character}");
                if !line.is_empty() && measured_text_width(font, &candidate, font_size) > max_width {
                    lines.push(std::mem::take(&mut line));
                    if lines.len() >= max_lines {
                        return lines;
                    }
                }
                line.push(character);
            }
        }
        if !line.is_empty() || paragraph.is_empty() {
            lines.push(line);
            if lines.len() >= max_lines {
                return lines;
            }
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn blend_notice_label(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    left: u32,
    top: u32,
    label: &[u8],
    label_width: u32,
    label_height: u32,
) {
    for y in 0..label_height.min(height.saturating_sub(top)) {
        for x in 0..label_width.min(width.saturating_sub(left)) {
            let source = ((y * label_width + x) * 4) as usize;
            let destination = (((top + y) * width + left + x) * 4) as usize;
            let alpha = u16::from(label[source + 3]);
            if alpha == 0 {
                continue;
            }
            for channel in 0..3 {
                pixels[destination + channel] = ((u16::from(label[source + channel]) * alpha
                    + u16::from(pixels[destination + channel]) * (255 - alpha))
                    / 255) as u8;
            }
            pixels[destination + 3] = 255;
        }
    }
}

fn fill_notice_rect(
    pixels: &mut [u8],
    canvas_width: u32,
    canvas_height: u32,
    left: u32,
    top: u32,
    width: u32,
    height: u32,
    color: [u8; 4],
) {
    for y in top..top.saturating_add(height).min(canvas_height) {
        for x in left..left.saturating_add(width).min(canvas_width) {
            let offset = ((y * canvas_width + x) * 4) as usize;
            pixels[offset..offset + 4].copy_from_slice(&color);
        }
    }
}

fn stroke_notice_rect(
    pixels: &mut [u8],
    canvas_width: u32,
    canvas_height: u32,
    left: u32,
    top: u32,
    width: u32,
    height: u32,
    color: [u8; 4],
) {
    for x in left..left.saturating_add(width).min(canvas_width) {
        for y in [top, top.saturating_add(height.saturating_sub(1))] {
            if y < canvas_height {
                let offset = ((y * canvas_width + x) * 4) as usize;
                pixels[offset..offset + 4].copy_from_slice(&color);
            }
        }
    }
    for y in top..top.saturating_add(height).min(canvas_height) {
        for x in [left, left.saturating_add(width.saturating_sub(1))] {
            if x < canvas_width {
                let offset = ((y * canvas_width + x) * 4) as usize;
                pixels[offset..offset + 4].copy_from_slice(&color);
            }
        }
    }
}

fn centered_text_x(font: &fontdue::Font, text: &str, font_size: f32) -> f32 {
    512.0 - measured_text_width(font, text, font_size) / 2.0
}

fn measured_text_width(font: &fontdue::Font, text: &str, font_size: f32) -> f32 {
    let mut advance = 0.0_f32;
    let mut previous = None;
    for character in text.chars() {
        if let Some(previous) = previous {
            advance += font
                .horizontal_kern(previous, character, font_size)
                .unwrap_or(0.0);
        }
        let (metrics, _) = rasterize_ui_glyph(font, character, font_size);
        advance += metrics.advance_width;
        previous = Some(character);
    }
    advance
}

fn append_text_sprite(
    sprites: &mut Vec<DecodedSprite>,
    font: &fontdue::Font,
    text: &str,
    x: f32,
    y: f32,
    font_size: f32,
) {
    let width = 360_u32;
    let height = 28_u32;
    let pixels = rasterize_label(font, text, font_size, width, height);
    sprites.push(DecodedSprite {
        texture_key: format!("label:{font_size}:{text}"),
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
        pixels: pixels.into(),
    });
}

fn rasterize_label(
    font: &fontdue::Font,
    text: &str,
    font_size: f32,
    width: u32,
    height: u32,
) -> Vec<u8> {
    rasterize_label_color(font, text, font_size, width, height, [255, 255, 255])
}

fn rasterize_label_color(
    font: &fontdue::Font,
    text: &str,
    font_size: f32,
    width: u32,
    height: u32,
    color: [u8; 3],
) -> Vec<u8> {
    let mut pixels = vec![0; width as usize * height as usize * 4];
    let mut pen_x = 0.0_f32;
    let baseline = font_size.ceil() as i32 + 2;
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
                let x = left + column as i32;
                let y = top + row as i32;
                if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                    continue;
                }
                let src = row * metrics.width + column;
                let dst = (y as usize * width as usize + x as usize) * 4;
                pixels[dst..dst + 4]
                    .copy_from_slice(&[color[0], color[1], color[2], coverage[src]]);
            }
        }
        pen_x += metrics.advance_width;
        if pen_x >= width as f32 {
            break;
        }
        previous = Some(character);
    }
    pixels
}

fn append_tiled_background(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    path: &str,
    width: u32,
    height: u32,
    scale: f32,
) -> Result<(), String> {
    let node = archive
        .root()
        .resolve(path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("NX node {path:?} was not found"))?;
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("NX node {path:?} is not a bitmap"))?;
    let source = bitmap
        .decode_bgra_shared()
        .map_err(|error| error.to_string())?;
    let source_width = usize::from(bitmap.width());
    let source_height = usize::from(bitmap.height());
    let mut pixels = vec![0; width as usize * height as usize * 4];
    for y in 0..height as usize {
        let source_y = ((y as f32 / scale).floor() as usize).min(source_height - 1);
        for x in 0..width as usize {
            let tile_width = (source_width as f32 * scale).max(1.0);
            let source_x = ((x as f32 % tile_width) / scale).floor() as usize;
            let source_x = source_x.min(source_width - 1);
            let src = (source_y * source_width + source_x) * 4;
            let dst = (y * width as usize + x) * 4;
            pixels[dst..dst + 4].copy_from_slice(&source[src..src + 4]);
        }
    }
    sprites.push(DecodedSprite {
        texture_key: format!("tiled:{path}:{width}x{height}:{scale}"),
        rect: SpriteRect {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: width,
        texture_height: height,
        pixels: pixels.into(),
    });
    Ok(())
}

fn append_node_sprite(
    sprites: &mut Vec<DecodedSprite>,
    node: &openstory_assets::NxNode,
    path: &str,
    anchor_x: f32,
    anchor_y: f32,
    scale: f32,
) -> Result<(), String> {
    let bitmap: Bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("NX node {path:?} is not a bitmap"))?;
    let origin = match node
        .child("origin")
        .map_err(|error| error.to_string())?
        .map(|origin| origin.value())
        .transpose()
        .map_err(|error| error.to_string())?
    {
        Some(NodeValue::Vector { x, y }) => (x as f32, y as f32),
        Some(_) => return Err(format!("NX sprite {path:?} has a malformed origin")),
        None => (0.0, 0.0),
    };
    let width = f32::from(bitmap.width()) * scale;
    let height = f32::from(bitmap.height()) * scale;
    let pixels = bitmap
        .decode_bgra_shared()
        .map_err(|error| error.to_string())?;
    sprites.push(DecodedSprite {
        texture_key: path.to_owned(),
        rect: SpriteRect {
            x: anchor_x - origin.0 * scale,
            y: anchor_y - origin.1 * scale,
            width,
            height,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: u32::from(bitmap.width()),
        texture_height: u32::from(bitmap.height()),
        pixels: pixels.into(),
    });
    Ok(())
}

fn load_damage_glyph_atlas(
    archive: &NxArchive,
    renderer: &GpuRenderer,
) -> Result<(GpuTexture, Vec<DamageGlyph>), String> {
    struct DecodedGlyph {
        width: u32,
        height: u32,
        origin_x: f32,
        origin_y: f32,
        pixels: Arc<[u8]>,
    }

    let load_glyph = |path: String| -> Result<DecodedGlyph, String> {
        let node = archive
            .root()
            .resolve(&path)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("NX damage glyph {path:?} is missing"))?;
        let bitmap = node
            .bitmap()
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("NX damage glyph {path:?} is not a bitmap"))?;
        let (width, height) = (u32::from(bitmap.width()), u32::from(bitmap.height()));
        if width == 0 || height == 0 || width > 128 || height > 128 {
            return Err(format!(
                "NX damage glyph {path:?} has unsupported dimensions {width}x{height}"
            ));
        }
        let origin = match node
            .child("origin")
            .map_err(|error| error.to_string())?
            .map(|origin| origin.value())
            .transpose()
            .map_err(|error| error.to_string())?
        {
            Some(NodeValue::Vector { x, y }) => (x as f32, y as f32),
            Some(_) => return Err(format!("NX damage glyph {path:?} has invalid origin")),
            None => (0.0, 0.0),
        };
        Ok(DecodedGlyph {
            width,
            height,
            origin_x: origin.0,
            origin_y: origin.1,
            pixels: bitmap
                .decode_bgra_shared()
                .map_err(|error| error.to_string())?,
        })
    };

    let mut decoded = Vec::with_capacity(81);
    for state in [
        "NoRed1",
        "NoRed0",
        "NoCri1",
        "NoCri0",
        "NoBlue1",
        "NoBlue0",
        "NoViolet1",
        "NoViolet0",
    ] {
        for digit in 0..10 {
            let path = format!("BasicEff.img/{state}/{digit}");
            decoded.push(load_glyph(path)?);
        }
    }
    decoded.push(load_glyph("BasicEff.img/NoRed0/Miss".to_owned())?);

    let cell_width = decoded.iter().map(|glyph| glyph.width).max().unwrap_or(0) + 2;
    let cell_height = decoded.iter().map(|glyph| glyph.height).max().unwrap_or(0) + 2;
    let atlas_width = cell_width * 5;
    let atlas_rows = u32::try_from(decoded.len().div_ceil(5))
        .map_err(|_| "NX damage number atlas row count overflow".to_owned())?;
    let atlas_height = cell_height * atlas_rows;
    let pixel_length = usize::try_from(atlas_width)
        .ok()
        .and_then(|width| {
            usize::try_from(atlas_height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "NX damage number atlas dimensions overflow".to_owned())?;
    let mut atlas = vec![0; pixel_length];
    let mut glyphs = Vec::with_capacity(decoded.len());
    for (index, glyph) in decoded.into_iter().enumerate() {
        let x = (index as u32 % 5) * cell_width + 1;
        let y = (index as u32 / 5) * cell_height + 1;
        for row in 0..glyph.height as usize {
            let source = row * glyph.width as usize * 4;
            let destination = ((y as usize + row) * atlas_width as usize + x as usize) * 4;
            let length = glyph.width as usize * 4;
            atlas[destination..destination + length]
                .copy_from_slice(&glyph.pixels[source..source + length]);
        }
        glyphs.push(DamageGlyph {
            uv: [
                (x as f32 + 0.5) / atlas_width as f32,
                (y as f32 + 0.5) / atlas_height as f32,
                (x as f32 + glyph.width as f32 - 0.5) / atlas_width as f32,
                (y as f32 + glyph.height as f32 - 0.5) / atlas_height as f32,
            ],
            width: glyph.width as f32,
            height: glyph.height as f32,
            origin_x: glyph.origin_x,
            origin_y: glyph.origin_y,
        });
    }
    let texture = renderer
        .upload_bgra(atlas_width, atlas_height, &atlas)
        .map_err(|error| error.to_string())?;
    Ok((texture, glyphs))
}
