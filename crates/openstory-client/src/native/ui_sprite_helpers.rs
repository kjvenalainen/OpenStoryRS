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

/// Returns native bitmap dimensions and its NX anchor origin when a path is a
/// bitmap. Window composition uses the visible top-left while the compositor
/// accepts the original NX anchor point.
fn nx_ui_bitmap_geometry(archive: &NxArchive, path: &str) -> Option<(usize, usize, i32, i32)> {
    let node = resolve_ui_path(&archive.root(), path).ok().flatten()?;
    let bitmap = node.bitmap().ok().flatten()?;
    let origin = node
        .child("origin")
        .ok()
        .flatten()
        .and_then(|origin| origin.value().ok())
        .and_then(|value| match value {
            NodeValue::Vector { x, y } => Some((x, y)),
            _ => None,
        })
        .unwrap_or((0, 0));
    Some((
        usize::from(bitmap.width()),
        usize::from(bitmap.height()),
        origin.0,
        origin.1,
    ))
}

fn nx_dialogue_button_geometry(archive: &NxArchive, path: &str) -> Option<(f32, f32, i32, i32)> {
    nx_ui_bitmap_geometry(archive, path).map(|(width, height, origin_x, origin_y)| {
        (width as f32, height as f32, origin_x, origin_y)
    })
}

struct NxUiModalLayout {
    bounds: ui_foundation::UiRect,
    buttons: Vec<ui_foundation::UiRect>,
}

/// Composes an NX modal and centers its authored buttons as a group along the
/// bottom edge. The returned visible bounds also drive text layout and hits.
fn composite_nx_ui_modal(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    archive: &NxArchive,
    background_path: &str,
    button_paths: &[&str],
) -> Result<NxUiModalLayout, String> {
    let (background_width, background_height, origin_x, origin_y) =
        nx_dialogue_button_geometry(archive, background_path)
            .ok_or_else(|| format!("missing NX modal bitmap {background_path:?}"))?;
    let left = ((canvas_width as f32 - background_width) * 0.5).floor();
    let top = ((canvas_height as f32 - background_height) * 0.5).floor();
    let mut dialog = NxDialog::new(pixels, canvas_width, canvas_height, archive);
    dialog.frame(
        background_path,
        left as i32 + origin_x,
        top as i32 + origin_y,
    )?;

    let button_geometry = button_paths
        .iter()
        .map(|path| {
            nx_dialogue_button_geometry(archive, path)
                .ok_or_else(|| format!("missing NX modal button {path:?}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let gap = if button_geometry.len() > 1 { 18.0 } else { 0.0 };
    let buttons_width = button_geometry.iter().map(|button| button.0).sum::<f32>()
        + gap * button_geometry.len().saturating_sub(1) as f32;
    let mut button_left = left + ((background_width - buttons_width) * 0.5).floor();
    let mut buttons = Vec::with_capacity(button_geometry.len());
    for (path, (button_width, button_height, button_origin_x, button_origin_y)) in
        button_paths.iter().zip(button_geometry)
    {
        let button_top = top + background_height - button_height - 14.0;
        let button = dialog.anchored_button(
            path,
            button_left.round() as i32 + button_origin_x,
            button_top.round() as i32 + button_origin_y,
        )?;
        buttons.push(button);
        button_left += button_width + gap;
    }
    Ok(NxUiModalLayout {
        bounds: ui_foundation::UiRect::new(left, top, background_width, background_height),
        buttons,
    })
}

/// Composes a native v83 nine-piece frame by tiling its authored edges and
/// center. The frame remains crisp at arbitrary window dimensions; no Rust
/// color blocks are used to imitate missing window art.
fn composite_nx_ui_nine_slice_frame(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    archive: &NxArchive,
    base_path: &str,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
) -> Result<(), String> {
    let paths = [
        ["top/left", "top/center", "top/right"],
        ["center/left", "center/center", "center/right"],
        ["bottom/left", "bottom/center", "bottom/right"],
    ];
    let mut geometry = [[(0_usize, 0_usize, 0_i32, 0_i32); 3]; 3];
    for (row, parts) in paths.iter().enumerate() {
        for (column, part) in parts.iter().enumerate() {
            let path = format!("{base_path}/{part}");
            geometry[row][column] = nx_ui_bitmap_geometry(archive, &path)
                .ok_or_else(|| format!("missing NX frame bitmap {path:?}"))?;
            if geometry[row][column].0 == 0 || geometry[row][column].1 == 0 {
                return Err(format!("NX frame bitmap {path:?} has empty dimensions"));
            }
        }
    }

    let tile_width = geometry[1][1].0;
    let tile_height = geometry[1][1].1;
    let top_height = geometry[0][1].1;
    let bottom_y = height.saturating_sub(geometry[2][1].1);
    let body_start = top_height.min(height);
    let body_end = bottom_y.max(body_start).min(height);
    let path_at = |row: usize, column: usize| format!("{base_path}/{}", paths[row][column]);
    let composite_at = |pixels: &mut [u8], row: usize, column: usize, x: usize, y: usize| {
        let path = path_at(row, column);
        let (_, _, origin_x, origin_y) = geometry[row][column];
        composite_nx_ui_bitmap(
            pixels,
            canvas_width,
            canvas_height,
            archive,
            &path,
            left as i32 + x as i32 + origin_x,
            top as i32 + y as i32 + origin_y,
        )
    };

    for x in (0..width).step_by(tile_width) {
        let column = if x == 0 {
            0
        } else if x + tile_width >= width {
            2
        } else {
            1
        };
        composite_at(pixels, 0, column, x, 0)?;
        composite_at(pixels, 2, column, x, bottom_y)?;
    }
    for y in (body_start..body_end).step_by(tile_height) {
        for x in (0..width).step_by(tile_width) {
            let column = if x == 0 {
                0
            } else if x + tile_width >= width {
                2
            } else {
                1
            };
            composite_at(pixels, 1, column, x, y)?;
        }
    }
    Ok(())
}

/// Composes a window made from a fixed header, repeating center strip and
/// footer, preserving the three source bitmaps at native pixel size.
fn composite_nx_ui_vertical_frame(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    archive: &NxArchive,
    top_path: &str,
    center_path: &str,
    bottom_path: &str,
    left: i32,
    top: i32,
    bottom: i32,
) -> Result<(), String> {
    let (_, center_height, _, _) = nx_ui_bitmap_geometry(archive, center_path)
        .ok_or_else(|| format!("missing NX frame bitmap {center_path:?}"))?;
    if center_height == 0 {
        return Err(format!("NX frame bitmap {center_path:?} has empty height"));
    }
    composite_nx_ui_bitmap(
        pixels,
        canvas_width,
        canvas_height,
        archive,
        top_path,
        left,
        top,
    )?;
    let mut y = top.saturating_add(
        nx_ui_bitmap_geometry(archive, top_path)
            .ok_or_else(|| format!("missing NX frame bitmap {top_path:?}"))?
            .1 as i32,
    );
    while y < bottom {
        composite_nx_ui_bitmap(
            pixels,
            canvas_width,
            canvas_height,
            archive,
            center_path,
            left,
            y,
        )?;
        y = y.saturating_add(center_height as i32);
    }
    composite_nx_ui_bitmap(
        pixels,
        canvas_width,
        canvas_height,
        archive,
        bottom_path,
        left,
        bottom,
    )
}

/// Draws a standard Basic.img Tab2 composition into a tab cell. Inventory,
/// NPC shop and other lists share the same v83 cap and fill artwork.
fn composite_nx_ui_tab(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    archive: &NxArchive,
    bounds: ui_foundation::UiRect,
    selected: bool,
) -> Result<(), String> {
    let state = usize::from(selected);
    let x = bounds.x.max(0.0) as usize;
    let y = bounds.y.max(0.0) as usize;
    let width = bounds.width.max(0.0) as usize;
    let height = bounds.height.max(0.0) as usize;
    if width < 8 || height == 0 {
        return Ok(());
    }
    for (part, left, part_width) in [
        ("left", x, 4),
        ("fill", x + 4, width - 8),
        ("right", x + width - 4, 4),
    ] {
        composite_nx_ui_bitmap_scaled_to(
            pixels,
            canvas_width,
            canvas_height,
            archive,
            &format!("Basic.img/Tab2/{part}{state}"),
            left,
            y,
            part_width,
            height,
        )?;
    }
    Ok(())
}

/// Centers native artwork in a layout cell without stretching its pixels.
/// Convert the desired bitmap position back to an NX anchor so nonzero origins
/// (including focused button art) do not shift the visible image.
fn composite_nx_ui_bitmap_centered(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    archive: &NxArchive,
    path: &str,
    bounds: ui_foundation::UiRect,
) -> Result<(), String> {
    let (w, h, ox, oy) = nx_dialogue_button_geometry(archive, path)
        .ok_or_else(|| format!("missing NX bitmap {path:?}"))?;
    composite_nx_ui_bitmap(
        pixels,
        width,
        height,
        archive,
        path,
        (bounds.x + (bounds.width - w) * 0.5).floor() as i32 + ox,
        (bounds.y + (bounds.height - h) * 0.5).floor() as i32 + oy,
    )
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
                let premultiplied =
                    foreground * source_alpha + background * destination_alpha * inverse / 255;
                pixels[destination_index + channel] = (premultiplied / output_alpha.max(1)) as u8;
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

#[derive(Clone, Copy)]
enum MushroomPopupButtons {
    Ok,
    OkCancel,
}

struct MushroomPopupArt {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    panel: SpriteRect,
    buttons: Vec<SpriteRect>,
    page_count: usize,
}

fn build_mushroom_popup_art(
    archive: &NxArchive,
    font: &fontdue::Font,
    message: &str,
    page: usize,
    buttons: MushroomPopupButtons,
) -> Result<MushroomPopupArt, String> {
    let background_path = "Login.img/Notice/backgrnd/0";
    let background = archive
        .root()
        .resolve(background_path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("missing native mushroom dialog asset {background_path:?}"))?;
    let bitmap = background
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!("native mushroom dialog asset {background_path:?} is not a bitmap")
        })?;
    let width = u32::from(bitmap.width());
    let height = u32::from(bitmap.height());
    let mut pixels = bitmap
        .decode_bgra_shared()
        .map_err(|error| error.to_string())?
        .to_vec();
    let panel = SpriteRect {
        x: (1024.0 - width as f32) * 0.5,
        y: (768.0 - height as f32) * 0.5,
        width: width as f32,
        height: height as f32,
        blend_mode: BlendMode::Alpha,
    };

    let button_paths: &[&str] = match buttons {
        MushroomPopupButtons::Ok => &["Login.img/Notice/BtYes/normal/0"],
        MushroomPopupButtons::OkCancel => &[
            "Login.img/Notice/BtYes/normal/0",
            "Login.img/Notice/BtNo/normal/0",
        ],
    };
    let button_geometry = button_paths
        .iter()
        .map(|path| {
            nx_dialogue_button_geometry(archive, path)
                .ok_or_else(|| format!("missing native mushroom dialog button {path:?}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let button_gap = if button_geometry.len() > 1 { 8.0 } else { 0.0 };
    let buttons_width = button_geometry.iter().map(|button| button.0).sum::<f32>()
        + button_gap * button_geometry.len().saturating_sub(1) as f32;
    let mut button_x = ((width as f32 - buttons_width) * 0.5).floor();
    let mut button_bounds = Vec::with_capacity(button_geometry.len());
    for (path, (button_width, button_height, origin_x, origin_y)) in
        button_paths.iter().zip(button_geometry)
    {
        let button_y = height as f32 - button_height - 33.0;
        composite_nx_ui_bitmap(
            &mut pixels,
            width as usize,
            height as usize,
            archive,
            path,
            button_x.round() as i32 + origin_x,
            button_y.round() as i32 + origin_y,
        )?;
        button_bounds.push(SpriteRect {
            x: panel.x + button_x,
            y: panel.y + button_y,
            width: button_width,
            height: button_height,
            blend_mode: BlendMode::Alpha,
        });
        button_x += button_width + button_gap;
    }

    let font_size = 14.0;
    let text_left = 104_u32.min(width.saturating_sub(24));
    let content_width = width.saturating_sub(text_left + 24);
    let lines = wrap_server_notice(font, message, font_size, content_width as f32, usize::MAX);
    let page_count = lines.len().div_ceil(4).max(1);
    let page = page.min(page_count - 1);
    for (index, line) in lines.iter().skip(page * 4).take(4).enumerate() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width as usize,
            height as usize,
            line,
            text_left as i32,
            35 + index as i32 * 19,
            font_size,
            content_width as usize,
            19,
            [55, 40, 29],
        );
    }
    if page_count > 1 {
        let label = if page + 1 < page_count {
            format!("{} / {} · OK to continue", page + 1, page_count)
        } else {
            format!("{} / {}", page + 1, page_count)
        };
        draw_dialogue_text(
            font,
            &mut pixels,
            width as usize,
            height as usize,
            &label,
            text_left as i32,
            126,
            10.0,
            content_width as usize,
            16,
            [70, 50, 35],
        );
    }

    Ok(MushroomPopupArt {
        width,
        height,
        pixels,
        panel,
        buttons: button_bounds,
        page_count,
    })
}

fn build_mushroom_popup_sprite(art: MushroomPopupArt, texture_key: String) -> DecodedSprite {
    DecodedSprite {
        texture_key,
        rect: art.panel,
        depth_order: 0,
        texture_width: art.width,
        texture_height: art.height,
        pixels: art.pixels.into(),
    }
}

fn build_quit_confirmation_sprites(
    archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<(Vec<DecodedSprite>, [SpriteRect; 2]), String> {
    let art = build_mushroom_popup_art(
        archive,
        font,
        "Are you sure you want to exit?",
        0,
        MushroomPopupButtons::OkCancel,
    )?;
    let [ok, cancel] = art
        .buttons
        .as_slice()
        .try_into()
        .map_err(|_| "mushroom quit dialog did not render both buttons".to_owned())?;
    let sprite = build_mushroom_popup_sprite(art, "native-mushroom-popup:quit".to_owned());
    Ok((vec![sprite], [ok, cancel]))
}

fn build_character_delete_confirmation_sprites(
    archive: &NxArchive,
    font: &fontdue::Font,
    message: &str,
) -> Result<(Vec<DecodedSprite>, [SpriteRect; 2]), String> {
    let art = build_mushroom_popup_art(archive, font, message, 0, MushroomPopupButtons::OkCancel)?;
    let [ok, cancel] =
        art.buttons.as_slice().try_into().map_err(|_| {
            "mushroom character-delete dialog did not render both buttons".to_owned()
        })?;
    let sprite = build_mushroom_popup_sprite(
        art,
        format!("native-mushroom-popup:character-delete:{message}"),
    );
    Ok((vec![sprite], [ok, cancel]))
}

fn build_server_notice_art(
    archive: &NxArchive,
    font: &fontdue::Font,
    message: &str,
    page: usize,
) -> Result<(u32, u32, Vec<u8>, SpriteRect, SpriteRect, usize), String> {
    let art = build_mushroom_popup_art(archive, font, message, page, MushroomPopupButtons::Ok)?;
    let button = *art
        .buttons
        .first()
        .ok_or_else(|| "mushroom notice did not render an OK button".to_owned())?;
    Ok((
        art.width,
        art.height,
        art.pixels,
        art.panel,
        button,
        art.page_count,
    ))
}

fn wrap_server_notice(
    font: &fontdue::Font,
    message: &str,
    font_size: f32,
    max_width: f32,
    max_lines: usize,
) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in message.replace('\r', "").split('\n') {
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
                if !line.is_empty() && measured_text_width(font, &candidate, font_size) > max_width
                {
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

fn centered_text_x(font: &fontdue::Font, text: &str, font_size: f32) -> f32 {
    512.0 - measured_text_width(font, text, font_size) / 2.0
}

fn measured_text_width(font: &fontdue::Font, text: &str, font_size: f32) -> f32 {
    ui_foundation::measure_text(font, text, font_size)
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
                pixels[dst..dst + 4].copy_from_slice(&[
                    color[0],
                    color[1],
                    color[2],
                    coverage[src],
                ]);
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

    let mut decoded = Vec::with_capacity(82);
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
    decoded.push(load_glyph("BasicEff.img/NoViolet0/Miss".to_owned())?);

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

#[cfg(test)]
mod server_notice_tests {
    use super::*;

    #[test]
    fn notice_wrapping_preserves_long_messages_and_bounds_each_line() {
        let font = fontdue::Font::from_bytes(
            include_bytes!("../../assets/fonts/Roboto-Regular.ttf").as_slice(),
            fontdue::FontSettings::default(),
        )
        .unwrap();
        let message = format!("{} {}", "maintenance ".repeat(80), "X".repeat(70));
        let lines = wrap_server_notice(&font, &message, 14.0, 230.0, usize::MAX);
        assert!(lines.len() > 4);
        assert_eq!(lines.join("").replace(' ', ""), message.replace(' ', ""));
        assert!(
            lines
                .iter()
                .all(|line| measured_text_width(&font, line, 14.0) <= 230.0)
        );
    }
}
