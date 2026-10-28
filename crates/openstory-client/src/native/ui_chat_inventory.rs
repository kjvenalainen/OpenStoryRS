fn load_chat_balloon_skin(archive: &NxArchive) -> Result<ChatBalloonSkin, String> {
    let source = archive
        .root()
        .resolve("ChatBalloon.img/0")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "UI.nx is missing ChatBalloon.img/0".to_owned())?;
    let mut parts = HashMap::new();
    for name in ["arrow", "c", "e", "ne", "n", "nw", "s", "se", "sw", "w"] {
        let node = source
            .child(name)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("chat balloon is missing its {name} image"))?;
        let bitmap = node
            .bitmap()
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("chat balloon {name} entry is not a bitmap"))?;
        let (origin_x, origin_y) = match node
            .child("origin")
            .map_err(|error| error.to_string())?
            .map(|origin| origin.value())
            .transpose()
            .map_err(|error| error.to_string())?
        {
            Some(NodeValue::Vector { x, y }) => (x, y),
            Some(_) => return Err(format!("chat balloon {name} has an invalid origin")),
            None => (0, 0),
        };
        let width = usize::from(bitmap.width());
        let height = usize::from(bitmap.height());
        if width == 0 || height == 0 || width > 128 || height > 128 {
            return Err(format!(
                "chat balloon {name} has unsupported size {width}x{height}"
            ));
        }
        let pixels = bitmap
            .decode_bgra_shared()
            .map_err(|error| error.to_string())?;
        parts.insert(
            name,
            ChatBalloonPart {
                width,
                height,
                origin_x,
                origin_y,
                pixels,
            },
        );
    }
    Ok(ChatBalloonSkin { parts })
}

fn render_speech_bubble(font: &fontdue::Font, skin: &ChatBalloonSkin, text: &str) -> Vec<u8> {
    const FONT_SIZE: f32 = 11.0;
    const MAX_LINE_WIDTH: usize = 260;
    const MAX_LINES: usize = 5;
    let lines = wrap_speech_text(font, text, FONT_SIZE, MAX_LINE_WIDTH, MAX_LINES);
    let text_width = lines
        .iter()
        .map(|line| dialogue_text_width(font, line, FONT_SIZE).ceil() as usize)
        .max()
        .unwrap_or(1)
        .max(1);
    let line_height = FONT_SIZE.ceil() as usize + 4;
    let text_height = lines.len().max(1) * line_height;
    let north = &skin.parts["n"];
    let west = &skin.parts["w"];
    let tile_width = north.width.max(1);
    let tile_height = west.height.max(1);
    let frame_width = (text_width / tile_width + 2) * tile_width;
    let frame_height = (text_height / tile_height).max(1) * tile_height;
    let (anchor_x, anchor_y) = SPEECH_BUBBLE_ANCHOR;
    let left = anchor_x - frame_width as i32 / 2;
    let top = anchor_y - frame_height as i32;
    let right = left + frame_width as i32;
    let bottom = top + frame_height as i32;
    let mut pixels = vec![0; SPEECH_BUBBLE_WIDTH as usize * SPEECH_BUBBLE_HEIGHT as usize * 4];

    for (name, x, y) in [
        ("nw", left, top),
        ("sw", left, bottom),
        ("ne", right, top),
        ("se", right, bottom),
    ] {
        blit_chat_balloon_part(&mut pixels, &skin.parts[name], x, y, None);
    }
    blit_chat_balloon_part(
        &mut pixels,
        &skin.parts["c"],
        left,
        top,
        Some((frame_width, frame_height)),
    );
    let mut x = left;
    while x < right {
        blit_chat_balloon_part(&mut pixels, &skin.parts["n"], x, top, None);
        blit_chat_balloon_part(&mut pixels, &skin.parts["s"], x, bottom, None);
        x += tile_width as i32;
    }
    let mut y = top;
    while y < bottom {
        blit_chat_balloon_part(&mut pixels, &skin.parts["w"], left, y, None);
        blit_chat_balloon_part(&mut pixels, &skin.parts["e"], right, y, None);
        y += tile_height as i32;
    }
    blit_chat_balloon_part(&mut pixels, &skin.parts["arrow"], anchor_x, anchor_y, None);
    let text_top = anchor_y - text_height as i32 - 4;
    for (index, line) in lines.iter().enumerate() {
        let width = dialogue_text_width(font, line, FONT_SIZE);
        let text_x = anchor_x - (width / 2.0).round() as i32;
        draw_dialogue_text(
            font,
            &mut pixels,
            SPEECH_BUBBLE_WIDTH as usize,
            SPEECH_BUBBLE_HEIGHT as usize,
            line,
            text_x,
            text_top + index as i32 * line_height as i32,
            FONT_SIZE,
            text_width + 2,
            line_height,
            [16, 16, 16],
        );
    }
    pixels
}

fn render_chalkboard(font: &fontdue::Font, text: &str) -> Vec<u8> {
    let width = CHALKBOARD_WIDTH as usize;
    let height = CHALKBOARD_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let edge = x < 3 || x >= width - 3 || y < 3 || y >= height - 3;
            let color = if edge {
                [74, 112, 174, 255]
            } else {
                [31, 49, 35, 246]
            };
            let offset = (y * width + x) * 4;
            pixels[offset..offset + 4].copy_from_slice(&color);
        }
    }
    let clipped_text = text.chars().take(256).collect::<String>();
    let label = rasterize_label_color(
        font,
        &clipped_text,
        ui_foundation::UiTextRole::Title.style().size,
        CHALKBOARD_WIDTH - 24,
        22,
        [242, 238, 208],
    );
    let label_width = (CHALKBOARD_WIDTH - 24) as usize;
    for y in 0..22_usize {
        for x in 0..label_width {
            let source = (y * label_width + x) * 4;
            let alpha = u32::from(label[source + 3]);
            if alpha == 0 {
                continue;
            }
            let target = ((y + 15) * width + x + 12) * 4;
            for channel in 0..3 {
                let foreground = u32::from(label[source + channel]);
                let background = u32::from(pixels[target + channel]);
                pixels[target + channel] =
                    ((foreground * alpha + background * (255 - alpha)) / 255) as u8;
            }
            pixels[target + 3] = 255;
        }
    }
    pixels
}

fn wrap_speech_text(
    font: &fontdue::Font,
    text: &str,
    size: f32,
    max_width: usize,
    max_lines: usize,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut truncated = false;
    for character in text.chars() {
        if character.is_control() {
            continue;
        }
        if character.is_whitespace() && current.is_empty() {
            continue;
        }
        let mut candidate = current.clone();
        candidate.push(character);
        if !current.is_empty() && dialogue_text_width(font, &candidate, size) > max_width as f32 {
            lines.push(std::mem::take(&mut current));
            if lines.len() >= max_lines {
                truncated = true;
                break;
            }
            if !character.is_whitespace() {
                current.push(character);
            }
        } else {
            current.push(character);
        }
    }
    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current.trim_end().to_owned());
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    if truncated {
        if let Some(last) = lines.last_mut() {
            while !last.is_empty()
                && dialogue_text_width(font, &format!("{last}…"), size) > max_width as f32
            {
                last.pop();
            }
            last.push('…');
        }
    }
    lines
}

fn blit_chat_balloon_part(
    canvas: &mut [u8],
    part: &ChatBalloonPart,
    anchor_x: i32,
    anchor_y: i32,
    stretch: Option<(usize, usize)>,
) {
    let canvas_width = SPEECH_BUBBLE_WIDTH as usize;
    let canvas_height = SPEECH_BUBBLE_HEIGHT as usize;
    let (draw_width, draw_height) = stretch.unwrap_or((part.width, part.height));
    let left = anchor_x - part.origin_x;
    let top = anchor_y - part.origin_y;
    for y in 0..draw_height {
        let target_y = top + y as i32;
        if !(0..canvas_height as i32).contains(&target_y) {
            continue;
        }
        let source_y = (y * part.height / draw_height.max(1)).min(part.height - 1);
        for x in 0..draw_width {
            let target_x = left + x as i32;
            if !(0..canvas_width as i32).contains(&target_x) {
                continue;
            }
            let source_x = (x * part.width / draw_width.max(1)).min(part.width - 1);
            let source_index = (source_y * part.width + source_x) * 4;
            let target_index = (target_y as usize * canvas_width + target_x as usize) * 4;
            let alpha = u16::from(part.pixels[source_index + 3]);
            if alpha == 0 {
                continue;
            }
            let inverse = 255 - alpha;
            for channel in 0..3 {
                canvas[target_index + channel] = ((u16::from(part.pixels[source_index + channel])
                    * alpha
                    + u16::from(canvas[target_index + channel]) * inverse)
                    / 255) as u8;
            }
            canvas[target_index + 3] =
                (alpha + (u16::from(canvas[target_index + 3]) * inverse) / 255).min(255) as u8;
        }
    }
}

fn render_inventory_panel(
    font: &fontdue::Font,
    inventory: &openstory_protocol::InventorySnapshot,
    active_tab: openstory_protocol::InventoryType,
    scroll_row: u16,
    hovered_slot: Option<u16>,
    drag_source: Option<(openstory_protocol::InventoryType, u16)>,
    pending_scroll_slot: Option<u16>,
    ui_archive: &NxArchive,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    item_name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<u8> {
    const WIDTH: usize = INVENTORY_PANEL_WIDTH as usize;
    const HEIGHT: usize = INVENTORY_PANEL_HEIGHT as usize;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    let reference_frame = inventory_reference_frame().is_some_and(|frame| {
        pixels.copy_from_slice(frame);
        true
    });
    if reference_frame {
        // Keep the reference-derived frame art free of test items; the actual
        // slot field is composed from the original NX window layer at runtime.
        let mut nx_slots = vec![0_u8; pixels.len()];
        let _ = composite_nx_ui_bitmap(
            &mut nx_slots,
            WIDTH,
            HEIGHT,
            ui_archive,
            "UIWindow.img/Item/backgrnd",
            0,
            0,
        );
        for y in 45..260 {
            for x in 4..152 {
                let offset = (y * WIDTH + x) * 4;
                pixels[offset..offset + 4].copy_from_slice(&nx_slots[offset..offset + 4]);
            }
        }
    }
    if !reference_frame {
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            "UIWindow.img/Item/backgrnd",
            0,
            0,
        );
        // The supplied reference crops include the final right and bottom border
        // pixels beyond the NX bitmap's 175x289 storage. Extend the border without
        // scaling the source artwork or moving any controls.
        for y in 0..289 {
            let source = (y * WIDTH + 174) * 4;
            let destination = (y * WIDTH + 175) * 4;
            pixels.copy_within(source..source + 4, destination);
        }
        for y in [289, 290] {
            let source = (288 * WIDTH) * 4;
            let destination = (y * WIDTH) * 4;
            pixels.copy_within(source..source + 175 * 4, destination);
            let edge = (y * WIDTH + 174) * 4;
            let edge_destination = (y * WIDTH + 175) * 4;
            pixels.copy_within(edge..edge + 4, edge_destination);
        }
        for (path, anchor_x, anchor_y) in [
            ("UIWindow.img/Item/BtFull/normal/0", 132, 12),
            ("UIWindow.img/Item/BtSmall/normal/0", 144, 12),
            ("Basic.img/BtClose/normal/0", 156, 6),
        ] {
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                WIDTH,
                HEIGHT,
                ui_archive,
                path,
                anchor_x,
                anchor_y,
            );
        }
    }

    let native_tabs = [
        openstory_protocol::InventoryType::Equip,
        openstory_protocol::InventoryType::Use,
        openstory_protocol::InventoryType::Setup,
        openstory_protocol::InventoryType::Etc,
        openstory_protocol::InventoryType::Cash,
    ];
    for (index, tab) in native_tabs.into_iter().enumerate() {
        if !reference_frame && tab == active_tab {
            let x = 10 + index * 31;
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                x + 4,
                26,
                29,
                15,
                // Pixel buffers are BGRA. This produces the reference's
                // rose active-tab fill (RGB 238, 102, 136).
                [136, 102, 238, 255],
            );
        }
        if reference_frame {
            continue;
        }
        let state = if tab == active_tab {
            "enabled"
        } else {
            "disabled"
        };
        let path = format!("UIWindow.img/Item/Tab/{state}/{index}");
        composite_inventory_tab_shadow(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            &path,
            (10 + index * 31) as i32,
            29,
        );
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            &path,
            (10 + index * 31) as i32,
            29,
        );
    }
    if reference_frame && active_tab != openstory_protocol::InventoryType::Etc {
        render_reference_inventory_tab(&mut pixels, WIDTH, HEIGHT, ui_archive, active_tab);
    }
    let maximum_slot = inventory_slot_max(inventory, active_tab);
    for row in 0..INVENTORY_SLOT_ROWS {
        for column in 0..INVENTORY_SLOT_COLUMNS {
            let slot = scroll_row
                .saturating_mul(INVENTORY_SLOT_COLUMNS as u16)
                .saturating_add((row * INVENTORY_SLOT_COLUMNS + column + 1) as u16);
            let x = INVENTORY_SLOT_X + column * INVENTORY_SLOT_STEP_X;
            let y = INVENTORY_SLOT_Y + row * INVENTORY_SLOT_STEP_Y;
            if slot > maximum_slot {
                let _ = composite_nx_ui_bitmap(
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    ui_archive,
                    "UIWindow.img/Item/disabled",
                    x as i32,
                    y as i32,
                );
                continue;
            }
            let Some(item) = inventory
                .items
                .iter()
                .find(|item| item.inventory_type == active_tab && item.slot == slot)
            else {
                continue;
            };
            if let Some((icon_width, icon_height, icon_pixels)) =
                inventory_item_icon(item.item_id, item_archive, character_archive, icon_cache)
            {
                let (icon_x, icon_y) = inventory_item_icon_offset(
                    item.item_id,
                    item_archive,
                    character_archive,
                )
                .unwrap_or((1, 0));
                let icon_x = icon_x - 2;
                let icon_y = icon_y - 2;
                composite_inventory_icon_native(
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    &icon_pixels,
                    icon_width as usize,
                    icon_height as usize,
                    x.saturating_add_signed(icon_x as isize),
                    y.saturating_add_signed(icon_y as isize),
                );
            }
            if active_tab != openstory_protocol::InventoryType::Equip && item.count > 0 {
                // MapleStory draws stack quantities with the pixel glyphs in
                // Basic.img/ItemNo, anchored at the lower-left of the slot.
                let mut digit_x = x;
                for digit in item.count.to_string().chars() {
                    let path = format!("Basic.img/ItemNo/{digit}");
                    let digit_width = resolve_ui_path(&ui_archive.root(), &path)
                        .ok()
                        .flatten()
                        .and_then(|node| node.bitmap().ok().flatten())
                        .map(|bitmap| usize::from(bitmap.width()))
                        .unwrap_or(8);
                    let _ = composite_nx_ui_bitmap(
                        &mut pixels,
                        WIDTH,
                        HEIGHT,
                        ui_archive,
                        &path,
                        digit_x as i32,
                        (y + 20) as i32,
                    );
                    digit_x += digit_width;
                }
            }
            if drag_source == Some((active_tab, slot)) {
                fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, x, y, 34, 33, [90, 136, 182, 72]);
            }
        }
    }
    // The reference inventory displays the blue scrollbar thumb centered in
    // its rail, with the rail's arrows already present in the NX background.
    if !reference_frame {
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            "Basic.img/VScr5/enabled/thumb0",
            152,
            111,
        );
    }
    if let Some(slot) = hovered_slot {
        if let Some(item) = inventory
            .items
            .iter()
            .find(|item| item.inventory_type == active_tab && item.slot == slot)
        {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &item_name(item.item_id, string_archive, item_name_cache),
                10,
                260,
                9.0,
                WIDTH - 20,
                13,
                [47, 82, 116],
            );
            let details = item_description(item.item_id, string_archive).unwrap_or_else(|| {
                format!(
                    "Item {}{}",
                    item.item_id,
                    if item.count > 1 {
                        format!(" · Quantity {}", item.count)
                    } else {
                        String::new()
                    }
                )
            });
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &details,
                10,
                274,
                7.0,
                WIDTH - 20,
                11,
                [78, 74, 66],
            );
        }
    } else {
        let formatted_meso = format_meso_amount(inventory.meso);
        let meso_text = formatted_meso
            .strip_suffix(" mesos")
            .unwrap_or(&formatted_meso);
        // A11M is an 11-pixel regular face in the legacy client. Use regular
        // text here and tune tracking to match the reference's value width.
        let size = 11.0;
        let tracking = 0.5;
        let characters: Vec<_> = meso_text.chars().collect();
        let advances: Vec<_> = characters
            .iter()
            .map(|character| dialogue_text_width(font, &character.to_string(), size))
            .collect();
        let run_width = advances.iter().sum::<f32>() + tracking * advances.len().saturating_sub(1) as f32;
        // Keep a visible buffer before the fixed "mesos" label so the final
        // digit cannot be clipped by the narrow value field.
        let mut cursor_x = 138.0 - run_width;
        for (character, advance) in characters.iter().zip(advances) {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &character.to_string(),
                cursor_x.round() as i32,
                266,
                size,
                16,
                16,
                [0, 0, 0],
            );
            cursor_x += advance + tracking;
        }
    }
    if pending_scroll_slot.is_some() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "Select gear to scroll · Esc cancels",
            10,
            257,
            7.0,
            WIDTH - 20,
            11,
            [136, 83, 39],
        );
    }
    pixels
}

fn render_reference_inventory_tab(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    ui_archive: &NxArchive,
    active_tab: openstory_protocol::InventoryType,
) {
    let active_index = match active_tab {
        openstory_protocol::InventoryType::Equipped => 0,
        openstory_protocol::InventoryType::Equip => 0,
        openstory_protocol::InventoryType::Use => 1,
        openstory_protocol::InventoryType::Setup => 2,
        openstory_protocol::InventoryType::Etc => 3,
        openstory_protocol::InventoryType::Cash => 4,
    };
    // The reference frame already contains all five correctly positioned
    // labels. Recolor only the ETC tab's rose fill so its original label stays
    // clean when another tab is selected.
    for y in 26..41 {
        for x in 107..136 {
            let index = (y * width + x) * 4;
            let red = pixels[index + 2];
            let green = pixels[index + 1];
            let blue = pixels[index];
            if red > green.saturating_add(25) && red > 170 && blue > 90 {
                pixels[index..index + 4].copy_from_slice(&[204, 204, 204, 255]);
            }
        }
    }

    let active_x = 10 + active_index * 31;
    fill_dialogue_rect(
        pixels,
        width,
        height,
        active_x + 1,
        26,
        30,
        15,
        [136, 102, 238, 255],
    );
    let active_path = format!("UIWindow.img/Item/Tab/enabled/{active_index}");
    composite_inventory_tab_shadow(
        pixels,
        width,
        height,
        ui_archive,
        &active_path,
        active_x as i32,
        29,
    );
    let _ = composite_nx_ui_bitmap(
        pixels,
        width,
        height,
        ui_archive,
        &active_path,
        active_x as i32,
        29,
    );
}

fn inventory_reference_frame() -> Option<&'static [u8]> {
    static FRAME: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();
    FRAME.get_or_init(|| {
            let bytes = include_bytes!("../../assets/ui/item-inventory-reference-frame.png");
            let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes.as_slice()));
            decoder.set_transformations(png::Transformations::EXPAND);
            let mut reader = decoder.read_info().ok()?;
            let mut decoded = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut decoded).ok()?;
            if info.width != INVENTORY_PANEL_WIDTH
                || info.height != INVENTORY_PANEL_HEIGHT
                || info.color_type != png::ColorType::Rgba
                || info.bit_depth != png::BitDepth::Eight
            {
                return None;
            }
            let source = &decoded[..info.buffer_size()];
            let mut bgra = Vec::with_capacity(source.len());
            for rgba in source.chunks_exact(4) {
                bgra.extend_from_slice(&[rgba[2], rgba[1], rgba[0], rgba[3]]);
            }
            Some(bgra)
        })
        .as_ref()
        .map(Vec::as_slice)
}

fn composite_inventory_tab_shadow(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    archive: &NxArchive,
    path: &str,
    anchor_x: i32,
    anchor_y: i32,
) {
    let Ok(Some(node)) = resolve_ui_path(&archive.root(), path) else {
        return;
    };
    let Ok(Some(bitmap)) = node.bitmap() else {
        return;
    };
    let Ok(source) = bitmap.decode_bgra_shared() else {
        return;
    };
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
    let width = usize::from(bitmap.width());
    let height = usize::from(bitmap.height());
    for (offset_x, offset_y) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let left = anchor_x + offset_x - origin.0;
        let top = anchor_y + offset_y - origin.1;
        for source_y in 0..height {
            let destination_y = top + source_y as i32;
            if !(0..canvas_height as i32).contains(&destination_y) {
                continue;
            }
            for source_x in 0..width {
                let destination_x = left + source_x as i32;
                if !(0..canvas_width as i32).contains(&destination_x) {
                    continue;
                }
                let source_index = (source_y * width + source_x) * 4;
                let destination_index = (destination_y as usize * canvas_width
                    + destination_x as usize)
                    * 4;
                let alpha = u32::from(source[source_index + 3]) * 3 / 5;
                let inverse = 255 - alpha;
                for channel in 0..3 {
                    let background = u32::from(pixels[destination_index + channel]);
                    pixels[destination_index + channel] =
                        (([42, 34, 28][channel] * alpha + background * inverse) / 255) as u8;
                }
                let background_alpha = u32::from(pixels[destination_index + 3]);
                pixels[destination_index + 3] = (alpha + background_alpha * inverse / 255) as u8;
            }
        }
    }
}

pub(super) fn render_inventory_tooltip(
    font: &fontdue::Font,
    item: &openstory_protocol::InventoryItem,
    _ui_archive: &NxArchive,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    item_name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<u8> {
    const WIDTH: usize = INVENTORY_TOOLTIP_WIDTH as usize;
    const HEIGHT: usize = INVENTORY_TOOLTIP_HEIGHT as usize;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, 0, 0, WIDTH, HEIGHT, [61, 34, 37, 220]);
    fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, 0, 0, WIDTH, 1, [138, 177, 224, 255]);
    fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, 0, HEIGHT - 1, WIDTH, 1, [93, 122, 174, 255]);
    fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, 0, 0, 1, HEIGHT, [138, 177, 224, 255]);
    fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, WIDTH - 1, 0, 1, HEIGHT, [93, 122, 174, 255]);
    fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, 7, 35, WIDTH - 14, 1, [92, 123, 175, 190]);
    if let Some((icon_width, icon_height, icon_pixels)) =
        inventory_item_icon(item.item_id, item_archive, character_archive, icon_cache)
    {
        composite_inventory_icon(
            &mut pixels,
            WIDTH,
            HEIGHT,
            &icon_pixels,
            icon_width as usize,
            icon_height as usize,
            15,
            79,
            44,
        );
    }
    let name = item_name(item.item_id, string_archive, item_name_cache);
    let name_width = ui_foundation::measure_text(font, &name, 13.0).ceil() as usize;
    ui_foundation::draw_text_role(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &name,
        (WIDTH.saturating_sub(name_width) / 2) as i32,
        13,
        ui_foundation::UiTextRole::Title,
        WIDTH - 20,
        18,
        Some([143, 193, 255]),
    );
    if item.count > 1 {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!("Quantity: {}", item.count),
            68,
            47,
            9.5,
            WIDTH - 78,
            12,
            [171, 202, 239],
        );
    }
    let info = inventory_item_info_node(item_archive, character_archive, item.item_id);
    if let Some(summary) = inventory_item_stat_summary(item, item_archive, character_archive) {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &summary,
            68,
            67,
            9.5,
            WIDTH - 78,
            48,
            [175, 207, 244],
        );
        let description_y = 116_usize;
        if let Some(description) = item_description(item.item_id, string_archive) {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &description,
                14,
                description_y as i32,
                9.5,
                WIDTH - 28,
                HEIGHT - description_y - 8,
                [184, 205, 232],
            );
        }
    } else if let Some(description) = item_description(item.item_id, string_archive) {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &description,
            68,
            67,
            9.5,
            WIDTH - 78,
            HEIGHT - 60,
            [184, 205, 232],
        );
    }
    let untradable = item.cash
        || info
            .as_ref()
            .is_some_and(|node| inventory_item_property_truthy(node, "tradeBlock"));
    if untradable {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "Untradable",
            12,
            HEIGHT as i32 - 16,
            7.0,
            WIDTH - 24,
            10,
            [255, 174, 110],
        );
    }
    pixels
}

fn render_inventory_drop_prompt(
    font: &fontdue::Font,
    prompt: &InventoryDropPrompt,
    string_archive: &NxArchive,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<u8> {
    const WIDTH: usize = 1024;
    const HEIGHT: usize = 768;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        HEIGHT,
        [0, 0, 0, 175],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        312,
        240,
        400,
        288,
        [177, 145, 92, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        316,
        244,
        392,
        280,
        [33, 31, 30, 255],
    );
    let title = match (prompt.action, prompt.step) {
        (_, InventoryDropStep::Quantity) => "Choose item quantity",
        (InventoryQuantityAction::Drop, InventoryDropStep::ConfirmUntradable) => {
            "Confirm item drop"
        }
        (InventoryQuantityAction::Drop, InventoryDropStep::Blocked) => "Item cannot be dropped",
        _ => "Choose item quantity",
    };
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        title,
        344,
        270,
        19.0,
        336,
        28,
        [246, 226, 185],
    );
    let name = item_name(prompt.item_id, string_archive, name_cache);
    let message = match prompt.step {
        InventoryDropStep::ConfirmUntradable => {
            format!("{name} cannot be recovered once dropped. Continue?")
        }
        InventoryDropStep::Quantity => {
            let action = match prompt.action {
                InventoryQuantityAction::Drop => "drop",
                InventoryQuantityAction::StorageStore => "store",
                InventoryQuantityAction::StorageTake => "take out",
            };
            format!("How many {name} would you like to {action}?")
        }
        InventoryDropStep::Blocked => "Cash items cannot be dropped.".to_owned(),
    };
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &message,
        344,
        318,
        13.0,
        336,
        36,
        [229, 226, 217],
    );
    if prompt.step == InventoryDropStep::Quantity {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!("Quantity: 1 to {}", prompt.max_quantity),
            344,
            366,
            12.0,
            336,
            20,
            [206, 201, 188],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            344,
            392,
            336,
            36,
            [53, 53, 53, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &prompt.quantity,
            356,
            400,
            15.0,
            312,
            20,
            [250, 246, 235],
        );
    }
    if let Some(notice) = prompt.notice.as_deref() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            notice,
            344,
            438,
            11.0,
            336,
            18,
            [242, 132, 115],
        );
    }
    let buttons = if prompt.step == InventoryDropStep::Blocked {
        vec![(452, "Close")]
    } else {
        let action = match prompt.action {
            InventoryQuantityAction::Drop => "Drop",
            InventoryQuantityAction::StorageStore => "Store",
            InventoryQuantityAction::StorageTake => "Take out",
        };
        vec![(365, action), (539, "Cancel")]
    };
    for (x, label) in buttons {
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            472,
            120,
            40,
            if x == 365 {
                [93, 68, 43, 255]
            } else {
                [63, 57, 49, 255]
            },
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            x as i32 + 14,
            482,
            13.0,
            92,
            18,
            [246, 231, 199],
        );
    }
    pixels
}

fn render_death_notice(font: &fontdue::Font, respawn_pending: bool) -> Vec<u8> {
    const WIDTH: usize = 1024;
    const HEIGHT: usize = 768;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        HEIGHT,
        [0, 0, 0, 160],
    );

    // This 1:1 art asset is the panel shown by reference-images/revive-character.png.
    // Keep it at native resolution so the frame, text raster, grave illustration,
    // and button bevel stay pixel aligned with the v83 client.
    static REVIVE_PANEL: std::sync::OnceLock<Option<(usize, usize, Vec<u8>)>> =
        std::sync::OnceLock::new();
    if let Some((panel_width, panel_height, panel_bgra)) = REVIVE_PANEL
        .get_or_init(|| {
            let bytes = include_bytes!("../../assets/ui/revive-character.png");
            let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes.as_slice()));
            decoder.set_transformations(png::Transformations::EXPAND);
            let mut reader = decoder.read_info().ok()?;
            let mut decoded = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut decoded).ok()?;
            let source = &decoded[..info.buffer_size()];
            if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
                return None;
            }
            let mut bgra = Vec::with_capacity(source.len());
            for rgba in source.chunks_exact(4) {
                bgra.extend_from_slice(&[rgba[2], rgba[1], rgba[0], rgba[3]]);
            }
            Some((info.width as usize, info.height as usize, bgra))
        })
        .as_ref()
    {
        let left = (WIDTH - *panel_width) / 2;
        let top = (HEIGHT - *panel_height) / 2;
        for source_y in 0..*panel_height {
            for source_x in 0..*panel_width {
                let source_index = (source_y * *panel_width + source_x) * 4;
                let source_alpha = u32::from(panel_bgra[source_index + 3]);
                if source_alpha == 0 {
                    continue;
                }
                let destination_index = ((top + source_y) * WIDTH + left + source_x) * 4;
                let destination_alpha = u32::from(pixels[destination_index + 3]);
                let inverse = 255 - source_alpha;
                let output_alpha = source_alpha + destination_alpha * inverse / 255;
                for channel in 0..3 {
                    let foreground = u32::from(panel_bgra[source_index + channel]);
                    let background = u32::from(pixels[destination_index + channel]);
                    let premultiplied = foreground * source_alpha
                        + background * destination_alpha * inverse / 255;
                    pixels[destination_index + channel] =
                        (premultiplied / output_alpha.max(1)) as u8;
                }
                pixels[destination_index + 3] = output_alpha as u8;
            }
        }
    }

    // Indicate an in-flight respawn request without changing the panel art or
    // its resting-state reference appearance.
    if respawn_pending {
        let panel_width = 285;
        let panel_height = 139;
        let left = (WIDTH - panel_width) / 2;
        let top = (HEIGHT - panel_height) / 2;
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            left + 131,
            top + 113,
            26,
            10,
            [237, 148, 62, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "...",
            (left + 139) as i32,
            (top + 112) as i32,
            10.0,
            18,
            13,
            [255, 255, 255],
        );
    }
    pixels
}

fn pet_equipment_items(
    inventory: &openstory_protocol::InventorySnapshot,
) -> Vec<&openstory_protocol::InventoryItem> {
    let mut items: Vec<_> = inventory
        .items
        .iter()
        .filter(|item| item.item_id / 10_000 == 181)
        .collect();
    items.sort_by_key(|item| (item.inventory_type as u8, item.slot));
    items
}

fn equipment_inventory_slot_position(slot: u16) -> Option<(usize, usize)> {
    Some(match slot {
        1 => (43, 27),   // hat
        2 => (43, 60),   // face accessory
        3 => (43, 93),   // eye accessory
        4 => (109, 93),  // ear accessory
        5 => (43, 126),  // top
        6 => (43, 159),  // bottom
        7 => (76, 192),  // shoes
        8 => (10, 159),  // gloves
        9 => (10, 126),  // cape
        10 => (142, 126), // shield
        11 => (109, 126), // weapon
        12 => (109, 60),  // ring 1
        13 => (142, 60),  // ring 2
        15 => (109, 159), // ring 3
        16 => (142, 159), // ring 4
        17 => (76, 126),  // pendant
        18 => (10, 225),  // tamed monster
        19 => (43, 225),  // saddle
        49 => (10, 60),   // medal
        50 => (76, 159),  // belt
        52 => (10, 27),   // book
        54 => (142, 93),  // shoulder
        55 => (109, 27),  // android
        57 => (76, 27),   // badge
        59 => (142, 27),  // android heart
        _ => return None,
    })
}

fn render_equipment_inventory_panel(
    inventory: &openstory_protocol::InventorySnapshot,
    ui_archive: &NxArchive,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
) -> Vec<u8> {
    const WIDTH: usize = EQUIPMENT_INVENTORY_PANEL_WIDTH as usize;
    const HEIGHT: usize = EQUIPMENT_INVENTORY_PANEL_HEIGHT as usize;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];

    let _ = composite_nx_ui_layers(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        &[("UIWindow.img/Equip/backgrnd", 0, 0)],
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "UIWindow.img/BtUIClose/normal/0",
        139,
        5,
    );

    for item in inventory.equipped() {
        let Some((x, y)) = equipment_inventory_slot_position(item.slot) else {
            continue;
        };
        if let Some((icon_width, icon_height, icon_pixels)) =
            inventory_item_icon(item.item_id, item_archive, character_archive, icon_cache)
        {
            composite_inventory_icon(
                &mut pixels,
                WIDTH,
                HEIGHT,
                &icon_pixels,
                icon_width as usize,
                icon_height as usize,
                x + 1,
                y + 1,
                32,
            );
        }
    }
    pixels
}

fn render_pet_equipment_panel(
    font: &fontdue::Font,
    inventory: &openstory_protocol::InventorySnapshot,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
    scroll_row: usize,
) -> Vec<u8> {
    const WIDTH: usize = 390;
    const HEIGHT: usize = 480;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        HEIGHT,
        [25, 27, 31, 248],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        2,
        [192, 164, 112, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        HEIGHT - 2,
        WIDTH,
        2,
        [192, 164, 112, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        2,
        HEIGHT,
        [192, 164, 112, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        WIDTH - 2,
        0,
        2,
        HEIGHT,
        [192, 164, 112, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Pet equipment",
        16,
        12,
        20.0,
        WIDTH - 60,
        25,
        [246, 226, 185],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        358,
        8,
        24,
        24,
        [79, 48, 42, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "X",
        366,
        10,
        14.0,
        14,
        18,
        [255, 225, 210],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Pet details and gear from your Equip inventory",
        16,
        42,
        12.0,
        WIDTH - 32,
        18,
        [205, 204, 197],
    );

    let pets: Vec<_> = inventory
        .items
        .iter()
        .filter(|item| item.pet.is_some())
        .take(3)
        .collect();
    for index in 0..3 {
        let x = 16 + index * 119;
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            68,
            112,
            94,
            if index < pets.len() {
                [39, 41, 46, 255]
            } else {
                [31, 32, 36, 255]
            },
        );
        if let Some(item) = pets.get(index) {
            if let Some((icon_width, icon_height, icon_pixels)) =
                inventory_item_icon(item.item_id, item_archive, character_archive, icon_cache)
            {
                composite_inventory_icon(
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    &icon_pixels,
                    icon_width as usize,
                    icon_height as usize,
                    x + 7,
                    78,
                    36,
                );
            }
            let Some(pet) = item.pet.as_ref() else {
                continue;
            };
            let pet_name = if pet.name.trim().is_empty() {
                item_name(item.item_id, string_archive, name_cache)
            } else {
                pet.name.clone()
            };
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &pet_name,
                (x + 47) as i32,
                77,
                11.0,
                60,
                29,
                [245, 235, 214],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &format!("Lv {}", pet.level),
                (x + 8) as i32,
                121,
                10.0,
                90,
                14,
                [201, 199, 191],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &format!("Fullness {}%", pet.fullness),
                (x + 8) as i32,
                139,
                10.0,
                96,
                14,
                [201, 199, 191],
            );
        }
    }

    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Available pet gear",
        16,
        178,
        15.0,
        WIDTH - 32,
        19,
        [246, 226, 185],
    );
    let gear = pet_equipment_items(inventory);
    let maximum_scroll = gear.len().saturating_sub(5);
    let scroll_row = scroll_row.min(maximum_scroll);
    let browse_hint = if gear.is_empty() {
        "No pet gear available.".to_owned()
    } else {
        format!(
            "Items {}-{} of {} | Scroll or Page Up/Down",
            scroll_row + 1,
            (scroll_row + 5).min(gear.len()),
            gear.len()
        )
    };
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &browse_hint,
        16,
        198,
        10.0,
        WIDTH - 32,
        15,
        [190, 190, 185],
    );
    if gear.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "No pet equipment in your Equip inventory.",
            24,
            256,
            13.0,
            WIDTH - 48,
            24,
            [184, 184, 184],
        );
    }
    for (index, item) in gear.iter().skip(scroll_row).take(5).enumerate() {
        let y = 220 + index * 49;
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            16,
            y,
            358,
            44,
            [39, 41, 46, 255],
        );
        if let Some((icon_width, icon_height, icon_pixels)) =
            inventory_item_icon(item.item_id, item_archive, character_archive, icon_cache)
        {
            composite_inventory_icon(
                &mut pixels,
                WIDTH,
                HEIGHT,
                &icon_pixels,
                icon_width as usize,
                icon_height as usize,
                23,
                y + 4,
                36,
            );
        }
        let name = item_name(item.item_id, string_archive, name_cache);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &name,
            68,
            (y + 7) as i32,
            12.0,
            220,
            17,
            [238, 232, 220],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!(
                "{} slot {}",
                if item.inventory_type == openstory_protocol::InventoryType::Cash {
                    "Cash"
                } else {
                    "Equip"
                },
                item.slot
            ),
            68,
            (y + 25) as i32,
            9.0,
            120,
            13,
            [173, 172, 166],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            319,
            y + 8,
            45,
            27,
            [93, 68, 41, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "USE",
            326,
            (y + 13) as i32,
            11.0,
            34,
            16,
            [255, 236, 192],
        );
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Press P to open this window",
        16,
        464,
        10.0,
        WIDTH - 32,
        14,
        [170, 170, 165],
    );
    pixels
}
