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

/// Stack counts use the game's outlined bitmap font at the icon's lower-left.
/// Compose all digits before fitting unusually large counts so no digits are
/// clipped and no system-font fallback changes the appearance.
fn draw_item_stack_count(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    ui: &NxArchive,
    count: impl Into<u32>,
    left: i32,
    top: i32,
    max_width: usize,
) -> Result<(), String> {
    let count = count.into();
    if count == 0 || max_width == 0 {
        return Ok(());
    }
    let mut glyphs = Vec::new();
    for digit in count.to_string().chars() {
        let path = format!("Basic.img/ItemNo/{digit}");
        let node = resolve_ui_path(&ui.root(), &path)?
            .ok_or_else(|| format!("missing item-count glyph {path}"))?;
        let bitmap = node
            .bitmap()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("invalid item-count glyph {path}"))?;
        glyphs.push((
            node,
            usize::from(bitmap.width()),
            usize::from(bitmap.height()),
        ));
    }
    let natural_width = glyphs.iter().map(|(_, w, _)| w + 1).sum::<usize>() - 1;
    let glyph_height = glyphs.iter().map(|(_, _, h)| *h).max().unwrap_or(0);
    if natural_width == 0 || glyph_height == 0 {
        return Ok(());
    }
    let mut strip = vec![0; natural_width * glyph_height * 4];
    let mut x = 0;
    for (node, w, _) in glyphs {
        composite_nx_node_bitmap(&mut strip, natural_width, glyph_height, &node, x as i32, 0)?;
        x += w + 1;
    }
    let draw_width = natural_width.min(max_width);
    if draw_width < natural_width {
        // Keep the native 11px height and outlined glyphs; only condense the
        // horizontal advance when a full five-digit count exceeds its icon.
        let mut fitted = vec![0; draw_width * glyph_height * 4];
        for y in 0..glyph_height {
            for x in 0..draw_width {
                let source = (y * natural_width + x * natural_width / draw_width) * 4;
                let dest = (y * draw_width + x) * 4;
                fitted[dest..dest + 4].copy_from_slice(&strip[source..source + 4]);
            }
        }
        composite_bgra_pixels(
            pixels,
            width,
            height,
            &fitted,
            draw_width,
            glyph_height,
            left,
            top,
        );
    } else {
        composite_bgra_pixels(
            pixels,
            width,
            height,
            &strip,
            natural_width,
            glyph_height,
            left,
            top,
        );
    }
    Ok(())
}

fn render_inventory_item_cell(
    item: &openstory_protocol::InventoryItem,
    ui_archive: &NxArchive,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
) -> Vec<u8> {
    let mut pixels = vec![0; 34 * 33 * 4];
    if let Some((icon_width, icon_height, icon_pixels)) =
        inventory_item_icon(item.item_id, item_archive, character_archive, icon_cache)
    {
        let (icon_x, icon_y) =
            inventory_item_icon_offset(item.item_id, item_archive, character_archive)
                .unwrap_or((1, 0));
        composite_bgra_pixels(
            &mut pixels,
            34,
            33,
            &icon_pixels,
            icon_width as usize,
            icon_height as usize,
            icon_x - 2,
            icon_y - 2,
        );
    }
    if !matches!(
        item.inventory_type,
        openstory_protocol::InventoryType::Equip | openstory_protocol::InventoryType::Equipped
    ) {
        let _ = draw_item_stack_count(&mut pixels, 34, 33, ui_archive, item.count, 0, 20, 34);
    }

    pixels
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
    // Native NX layers remain the source of truth; no reference screenshot
    // pixels participate in the live window.
    for (path, x, y) in [
        ("UIWindow.img/Item/backgrnd", 0, 0),
        ("UIWindow.img/Item/BtGather/normal/0", 132, 12),
        ("UIWindow.img/Item/BtSort/normal/0", 147, 12),
        ("Basic.img/BtClose/normal/0", 156, 6),
    ] {
        let _ = composite_nx_ui_bitmap(&mut pixels, WIDTH, HEIGHT, ui_archive, path, x, y);
    }
    for (index, tab) in inventory_tabs().into_iter().enumerate() {
        let bounds = ui_foundation::inventory_tab(index);
        let _ = composite_nx_ui_tab(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            bounds,
            tab == active_tab,
        );
        let state = if tab == active_tab {
            "enabled"
        } else {
            "disabled"
        };
        let path = format!("UIWindow.img/Item/Tab/{state}/{index}");
        let _ = composite_nx_ui_bitmap_centered(
            &mut pixels, WIDTH, HEIGHT, ui_archive, &path, bounds,
        );
    }
    let maximum_slot = inventory_slot_max(inventory, active_tab);
    for row in 0..INVENTORY_SLOT_ROWS {
        for column in 0..INVENTORY_SLOT_COLUMNS {
            let slot = scroll_row
                .saturating_mul(INVENTORY_SLOT_COLUMNS as u16)
                .saturating_add((row * INVENTORY_SLOT_COLUMNS + column + 1) as u16);
            let cell = ui_foundation::INVENTORY_GRID.cell(row * INVENTORY_SLOT_COLUMNS + column);
            let (x, y) = (cell.x as usize, cell.y as usize);
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
            if drag_source != Some((active_tab, slot)) {
                let cell_pixels = render_inventory_item_cell(
                    item,
                    ui_archive,
                    item_archive,
                    character_archive,
                    icon_cache,
                );
                composite_bgra_pixels(
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    &cell_pixels,
                    34,
                    33,
                    x as i32,
                    y as i32,
                );
            }
        }
    }
    let rows = usize::from(maximum_slot).div_ceil(INVENTORY_SLOT_COLUMNS);
    let fraction =
        ui_foundation::scroll_fraction(usize::from(scroll_row), rows, INVENTORY_SLOT_ROWS);
    ui_foundation::draw_scrollbar(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        152,
        51,
        210,
        fraction,
    );
    // Hover details are displayed by the separate tooltip, never over the meso field.
    let _ = (hovered_slot, string_archive, item_name_cache);
    draw_inventory_meso_amount(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        inventory.meso,
        138,
        266,
        112,
    );
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
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        HEIGHT,
        [82, 54, 51, 218],
    );
    for (x, y, w, h) in [
        (0, 0, WIDTH, 1),
        (0, 0, 1, HEIGHT),
        (WIDTH - 1, 0, 1, HEIGHT),
        (0, HEIGHT - 1, WIDTH, 1),
    ] {
        fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, x, y, w, h, [230, 225, 228, 255]);
    }
    let name = item_name(item.item_id, string_archive, item_name_cache);
    let name = ui_foundation::ellipsize(font, &name, 13.0, (WIDTH - 28) as f32);
    ui_foundation::draw_text_role(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &name,
        17,
        8,
        ui_foundation::UiTextRole::Title,
        WIDTH - 28,
        18,
        Some([255, 255, 255]),
    );
    let info = inventory_item_info_node(item_archive, character_archive, item.item_id);
    let equipment = item.item_id / 1_000_000 == 1;
    let icon_size = if equipment { 64 } else { 32 };
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        9,
        32,
        icon_size + 4,
        icon_size + 4,
        [220, 220, 220, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        10,
        33,
        icon_size + 2,
        icon_size + 2,
        [150, 150, 150, 255],
    );
    if let Some((w, h, icon)) =
        inventory_item_icon(item.item_id, item_archive, character_archive, icon_cache)
    {
        composite_inventory_icon(
            &mut pixels,
            WIDTH,
            HEIGHT,
            &icon,
            w as usize,
            h as usize,
            11,
            34,
            icon_size,
        );
    }
    if equipment {
        for (row, (label, key)) in [
            ("LEV", "reqLevel"),
            ("STR", "reqSTR"),
            ("DEX", "reqDEX"),
            ("INT", "reqINT"),
            ("LUK", "reqLUK"),
            ("FAM", "reqPOP"),
        ]
        .into_iter()
        .enumerate()
        {
            let value = info
                .as_ref()
                .and_then(|node| node.child(key).ok().flatten())
                .and_then(|node| match node.value().ok()? {
                    NodeValue::Integer(value) => Some(value),
                    _ => None,
                })
                .unwrap_or(0);
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &format!("REQ {label} : {value}"),
                93,
                29 + row as i32 * 11,
                10.0,
                WIDTH - 101,
                12,
                [255, 255, 255],
            );
        }
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "BEGINNER WARRIOR MAGICIAN BOWMAN THIEF PIRATE",
            9,
            110,
            8.0,
            WIDTH - 18,
            12,
            [255, 175, 72],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            5,
            127,
            WIDTH - 10,
            1,
            [220, 220, 220, 255],
        );
        let category = match item.item_id / 10_000 {
            100 => "HAT",
            101..=103 => "ACCESSORY",
            104 => "TOP",
            105 => "OVERALL",
            106 => "BOTTOM",
            107 => "SHOES",
            108 => "GLOVES",
            109 => "SHIELD",
            110 => "CAPE",
            _ => "WEAPON",
        };
        let equip = item.equip.as_ref();
        let (stat_label, stat_value) = if let Some(equip) = equip {
            if equip.stats[6] > 0 {
                ("WEAPON ATT.", equip.stats[6])
            } else if equip.stats[7] > 0 {
                ("MAGIC ATT.", equip.stats[7])
            } else {
                ("WEAPON DEF.", equip.stats[8])
            }
        } else {
            ("WEAPON DEF.", 0)
        };
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
                9,
                99,
                9.0,
                WIDTH - 18,
                12,
                [255, 175, 72],
            );
        }
        let slots = equip.map(|data| data.upgrade_slots).unwrap_or(0);
        for (row, value) in [
            format!("- CATEGORY : {category}"),
            format!("- {stat_label} : {stat_value}"),
            format!("- NUMBER OF UPGRADES AVAILABLE : {slots}"),
        ]
        .into_iter()
        .enumerate()
        {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &value,
                9,
                134 + row as i32 * 14,
                10.0,
                WIDTH - 18,
                13,
                [255, 255, 255],
            );
        }
    } else {
        let summary = inventory_item_stat_summary(item, item_archive, character_archive)
            .or_else(|| item_description(item.item_id, string_archive))
            .unwrap_or_default();
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &summary,
            53,
            34,
            11.0,
            WIDTH - 62,
            112,
            [255, 255, 255],
        );
        if item.count > 1 {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &format!("Quantity: {}", item.count),
                10,
                153,
                11.0,
                WIDTH - 20,
                15,
                [255, 255, 255],
            );
        }
    }
    if !equipment
        && (item.cash
            || info
                .as_ref()
                .is_some_and(|node| inventory_item_property_truthy(node, "tradeBlock")))
    {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "Untradable",
            10,
            169,
            9.0,
            WIDTH - 20,
            12,
            [255, 175, 72],
        );
    }
    pixels
}

fn render_inventory_drop_prompt(
    ui_archive: &NxArchive,
    font: &fontdue::Font,
    prompt: &InventoryDropPrompt,
    string_archive: &NxArchive,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Result<Vec<u8>, String> {
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
    let (background_path, button_paths): (&str, &[&str]) = match prompt.step {
        InventoryDropStep::ConfirmUntradable => (
            "Basic.img/YesNo/backgrnd",
            &["Basic.img/BtYes/normal/0", "Basic.img/BtNo/normal/0"],
        ),
        InventoryDropStep::Quantity => (
            "Basic.img/YesNo/backgrnd",
            &["Basic.img/BtOK/normal/0", "Basic.img/BtCancel/normal/0"],
        ),
        InventoryDropStep::Blocked => ("Basic.img/Notice/backgrnd", &["Basic.img/BtOK/normal/0"]),
    };
    let modal = composite_nx_ui_modal(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        background_path,
        button_paths,
    )?;
    let left = modal.bounds.x.round() as i32 + 18;
    let top = modal.bounds.y.round() as i32 + 12;
    let text_width = (modal.bounds.width - 36.0).max(0.0) as usize;
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
        left,
        top,
        14.0,
        text_width,
        18,
        [35, 35, 35],
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
        left,
        top + 26,
        11.0,
        text_width,
        28,
        [44, 44, 44],
    );
    if prompt.step == InventoryDropStep::Quantity {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!("Quantity: {}  (1 to {})", prompt.quantity, prompt.max_quantity),
            left,
            top + 61,
            11.0,
            text_width,
            16,
            [44, 44, 44],
        );
    }
    if let Some(notice) = prompt.notice.as_deref() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            notice,
            left,
            top + 78,
            10.0,
            text_width,
            14,
            [125, 40, 35],
        );
    }
    Ok(pixels)
}

fn render_death_notice(font: &fontdue::Font, ui: &NxArchive, respawn_pending: bool) -> Vec<u8> {
    const WIDTH: usize = 1024;
    const HEIGHT: usize = 768;
    let mut pixels = vec![0; WIDTH * HEIGHT * 4];
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
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui,
        "UIWindow.img/Notice/0",
        369,
        311,
    );
    let button = ui_foundation::REVIVE_BUTTON;
    let path = if respawn_pending {
        "Basic.img/BtOK/disabled/0"
    } else {
        "Basic.img/BtOK/normal/0"
    };
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui,
        path,
        button.x as i32,
        button.y as i32,
    );
    if respawn_pending {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "...",
            505,
            button.y as i32 + 2,
            11.0,
            20,
            14,
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
        .filter(|item| {
            (180..=183).contains(&(item.item_id / 10_000))
                && (matches!(
                    item.inventory_type,
                    openstory_protocol::InventoryType::Equip
                        | openstory_protocol::InventoryType::Cash
                ) || (item.inventory_type == openstory_protocol::InventoryType::Equipped
                    && pet_equipment_grid_position(item.item_id).is_none()))
        })
        .collect();
    items.sort_by_key(|item| (item.inventory_type as u8, item.slot));
    items
}

// The native pet panel labels identify these v83 item categories. Equipped
// entries are matched by item identity, not by guessed server slot numbers.
fn pet_equipment_grid_position(item_id: i32) -> Option<(usize, usize)> {
    let (column, row) = match item_id {
        1_812_000 => (1, 1),                // Meso Magnet
        1_812_001 => (0, 1),                // Item Pouch
        1_812_002 => (0, 0),                // HP Potion Pouch
        1_812_003 => (2, 0),                // MP Potion Pouch
        1_812_004 => (0, 3),                // Wing Boots
        1_812_007 => (0, 2),                // Item Ignore Pendant
        id if id / 10_000 == 180 => (3, 2), // pet equipment
        id if id / 10_000 == 182 => (2, 3), // name ring
        id if id / 10_000 == 183 => (3, 3), // quote ring
        _ => return None,
    };
    Some((12 + column * 33, 34 + row * 33))
}

fn pet_equipment_grid_item_at(
    inventory: &openstory_protocol::InventorySnapshot,
    x: f32,
    y: f32,
) -> Option<&openstory_protocol::InventoryItem> {
    inventory.equipped().find(|item| {
        pet_equipment_grid_position(item.item_id).is_some_and(|(left, top)| {
            (left as f32..left as f32 + 32.0).contains(&x)
                && (top as f32..top as f32 + 32.0).contains(&y)
        })
    })
}

fn equipment_inventory_slot_position(slot: u16) -> Option<(usize, usize)> {
    let (x, y) = match slot {
        1 => (43, 27),    // hat
        2 => (43, 60),    // face accessory
        3 => (76, 93),    // eye accessory
        4 => (109, 93),   // ear accessory
        5 => (43, 126),   // top
        6 => (43, 159),   // bottom
        7 => (76, 192),   // shoes
        8 => (10, 159),   // gloves
        9 => (10, 126),   // cape
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
    };
    // These positions were authored for UIWindow2; the v83 doll grid starts
    // five pixels left and eight pixels lower.
    Some((x - 5, y + 8))
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
        "Basic.img/BtClose/normal/0",
        156,
        6,
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
    ui_archive: &NxArchive,
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
    let _ = composite_nx_ui_nine_slice_frame(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "UIWindow.img/EventWindow/backgrnd",
        0,
        0,
        WIDTH,
        HEIGHT,
    );
    // The pet grid covers only the top of the equipment frame; clip its
    // unused doll section before compositing the original art over the frame.
    // The pet overlay is 177px wide, two pixels wider than the equipment doll.
    // Preserve its authored right border rather than clipping it to the doll.
    let mut equipment = vec![0_u8; 177 * 204 * 4];
    let _ = composite_nx_ui_bitmap(&mut equipment, 177, 204, ui_archive, "UIWindow.img/Equip/backgrnd", 0, 0);
    let _ = composite_nx_ui_bitmap(&mut equipment, 177, 204, ui_archive, "UIWindow.img/Equip/pet", 0, 23);
    composite_bgra_pixels(&mut pixels, WIDTH, HEIGHT, &equipment, 177, 204, 0, 0);
    for item in inventory.equipped() {
        let Some((x, y)) = pet_equipment_grid_position(item.item_id) else {
            continue;
        };
        if let Some((w, h, icon)) =
            inventory_item_icon(item.item_id, item_archive, character_archive, icon_cache)
        {
            composite_inventory_icon(
                &mut pixels,
                WIDTH,
                HEIGHT,
                &icon,
                w as usize,
                h as usize,
                x,
                y,
                32,
            );
        }
    }
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "Basic.img/BtClose/normal/0",
        358,
        8,
    );

    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Pets",
        188,
        7,
        12.0,
        160,
        17,
        [48, 55, 57],
    );
    let pets: Vec<_> = inventory
        .items
        .iter()
        .filter(|item| item.pet.is_some())
        .take(3)
        .collect();
    for index in 0..3 {
        let y = 42 + index * 51;
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
                    190,
                    y + 5,
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
            let pet_name = ui_foundation::ellipsize(font, &pet_name, 11.0, 135.0);
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &pet_name,
                232,
                (y + 4) as i32,
                11.0,
                135,
                15,
                [43, 48, 48],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &format!("Lv {}   Fullness {}%", pet.level, pet.fullness),
                232,
                (y + 23) as i32,
                9.0,
                135,
                14,
                [72, 79, 78],
            );
        }
    }
    let _ = composite_nx_ui_bitmap_scaled_to(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "UIWindow.img/UtilDlgEx/line",
        8,
        206,
        WIDTH - 16,
        2,
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Other pet gear",
        16,
        208,
        10.0,
        190,
        12,
        [48, 55, 57],
    );
    let gear = pet_equipment_items(inventory);
    let scroll_row = scroll_row.min(gear.len().saturating_sub(5));
    if gear.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "No other pet equipment.",
            24,
            256,
            11.0,
            WIDTH - 48,
            24,
            [90, 96, 96],
        );
    }
    for (index, item) in gear.iter().skip(scroll_row).take(5).enumerate() {
        let y = 220 + index * 49;
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
        let name = ui_foundation::ellipsize(font, &name, 11.0, 220.0);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &name,
            68,
            (y + 7) as i32,
            11.0,
            220,
            17,
            [43, 48, 48],
        );
        let location = match item.inventory_type {
            openstory_protocol::InventoryType::Equipped => {
                "Equipped · click to remove".to_owned()
            }
            openstory_protocol::InventoryType::Cash => "Cash inventory".to_owned(),
            _ if pet_equipment_destination(item.item_id, inventory).is_some() => {
                "Click to equip".to_owned()
            }
            _ => "Cannot equip".to_owned(),
        };
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &location,
            68,
            (y + 25) as i32,
            9.0,
            220,
            13,
            [80, 88, 87],
        );
        if item.inventory_type != openstory_protocol::InventoryType::Equipped {
            let path = if item.inventory_type == openstory_protocol::InventoryType::Equip
                && pet_equipment_destination(item.item_id, inventory).is_none()
            {
                "UIWindow.img/ShortCut/BtEquip/disabled/0"
            } else {
                "UIWindow.img/ShortCut/BtEquip/normal/0"
            };
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                WIDTH,
                HEIGHT,
                ui_archive,
                path,
                294,
                y as i32 + 9,
            );
        }
    }
    if !gear.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!(
                "{}–{} / {}",
                scroll_row + 1,
                (scroll_row + 5).min(gear.len()),
                gear.len()
            ),
            16,
            465,
            9.0,
            150,
            12,
            [80, 88, 87],
        );
        if gear.len() > 5 {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                "Scroll or Page Up/Down",
                236,
                465,
                9.0,
                145,
                12,
                [80, 88, 87],
            );
        }
    }
    pixels
}

#[cfg(test)]
mod pet_equipment_grid_tests {
    use super::*;
    use openstory_protocol::{InventoryItem, InventorySnapshot, InventoryType};

    #[test]
    fn pet_equip_targets_are_positive_and_do_not_replace_unrelated_items() {
        let empty = InventorySnapshot::default();
        assert_eq!(pet_equipment_destination(1_822_000, &empty), Some(114));
        let packet = openstory_protocol::move_item_packet(InventoryType::Equip, 3, -114, 1, 0);
        assert_eq!(&packet[9..11], &(-114_i16).to_le_bytes());
        assert_eq!(pet_equipment_destination(1_812_000, &empty), None);
        let mut inventory = empty;
        inventory.items.push(InventoryItem {
            inventory_type: InventoryType::Equipped,
            slot: 114,
            item_id: 1_000_000,
            ..Default::default()
        });
        assert_eq!(pet_equipment_destination(1_822_000, &inventory), None);
        inventory.items.push(InventoryItem {
            inventory_type: InventoryType::Equipped,
            slot: 120,
            item_id: 1_812_000,
            ..Default::default()
        });
        assert_eq!(pet_equipment_destination(1_812_000, &inventory), Some(120));
        assert_eq!(pet_equipment_destination(1_832_000, &inventory), None);
    }

    #[test]
    fn worn_items_map_to_the_labeled_cells_and_do_not_appear_as_available() {
        let inventory = InventorySnapshot {
            items: vec![
                InventoryItem {
                    inventory_type: InventoryType::Equipped,
                    slot: 114,
                    item_id: 1_812_000,
                    ..Default::default()
                },
                InventoryItem {
                    inventory_type: InventoryType::Equip,
                    slot: 1,
                    item_id: 1_812_001,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert_eq!(
            pet_equipment_grid_item_at(&inventory, 46.0, 68.0).map(|item| item.item_id),
            Some(1_812_000)
        );
        assert!(pet_equipment_grid_item_at(&inventory, 44.0, 68.0).is_none());
        assert_eq!(pet_equipment_items(&inventory).len(), 1);
        assert_eq!(pet_equipment_items(&inventory)[0].item_id, 1_812_001);
        assert_eq!(pet_equipment_grid_position(1_812_002), Some((12, 34)));
        assert_eq!(pet_equipment_grid_position(1_822_000), Some((78, 133)));
        assert_eq!(
            inventory_icon_path(1_802_000),
            Some((true, "PetEquip/01802000.img/info/icon".to_owned()))
        );
        assert_eq!(
            item_name_path(1_832_000).as_deref(),
            Some("Eqp.img/Eqp/PetEquip/1832000/name")
        );
    }
}

// Shared currency typography for NX inventory and NPC shop balance fields.
fn draw_inventory_meso_amount(
    font: &fontdue::Font,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    amount: i64,
    right: i32,
    top: i32,
    field_width: usize,
) {
    let meso = format_meso_amount(amount);
    let meso = meso.strip_suffix(" Mesos").unwrap_or(&meso);
    let meso = ui_foundation::ellipsize(font, meso, 11.0, field_width as f32);
    let text_width = dialogue_text_width(font, &meso, 11.0).ceil() as i32;
    draw_dialogue_text(
        font,
        pixels,
        width,
        height,
        &meso,
        (right - text_width).max(right - field_width as i32),
        top,
        11.0,
        field_width,
        15,
        [0, 0, 0],
    );
}
