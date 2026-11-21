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
    let cell_width = INVENTORY_SLOT_CELL_WIDTH;
    let cell_height = INVENTORY_SLOT_CELL_HEIGHT;
    let mut pixels = vec![0; cell_width * cell_height * 4];
    if let Some((icon_width, icon_height, icon_pixels)) =
        inventory_item_icon(item.item_id, item_archive, character_archive, icon_cache)
    {
        let (icon_x, icon_y) =
            inventory_item_icon_offset(item.item_id, item_archive, character_archive)
                .unwrap_or((1, 0));
        composite_bgra_pixels(
            &mut pixels,
            cell_width,
            cell_height,
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
        let _ = draw_item_stack_count(
            &mut pixels,
            cell_width,
            cell_height,
            ui_archive,
            item.count,
            0,
            20,
            cell_width,
        );
    }

    pixels
}

fn inventory_display_item<'a>(
    inventory: &'a openstory_protocol::InventorySnapshot,
    active_tab: openstory_protocol::InventoryType,
    slot: u16,
    pending_move: Option<(openstory_protocol::InventoryType, u16, u16)>,
) -> Option<&'a openstory_protocol::InventoryItem> {
    let lookup_slot = match pending_move {
        Some((inventory_type, source, destination)) if inventory_type == active_tab => {
            if slot == source {
                destination
            } else if slot == destination {
                source
            } else {
                slot
            }
        }
        _ => slot,
    };
    inventory
        .items
        .iter()
        .find(|item| item.inventory_type == active_tab && item.slot == lookup_slot)
}

fn render_inventory_panel(
    font: &fontdue::Font,
    inventory: &openstory_protocol::InventorySnapshot,
    active_tab: openstory_protocol::InventoryType,
    scroll_row: u16,
    hovered_slot: Option<u16>,
    drag_source: Option<(openstory_protocol::InventoryType, u16)>,
    pending_move: Option<(openstory_protocol::InventoryType, u16, u16)>,
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
    let mut dialog = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui_archive);
    let _ = dialog.frame("UIWindow.img/Item/backgrnd", 0, 0);
    let _ = dialog.buttons(&[
        (
            "UIWindow.img/Item/BtGather/normal/0",
            ui_foundation::INVENTORY_GATHER_BUTTON,
        ),
        (
            "UIWindow.img/Item/BtSort/normal/0",
            ui_foundation::INVENTORY_SORT_BUTTON,
        ),
        (
            ui_dialog::CLOSE_BUTTON_ART,
            ui_foundation::INVENTORY_CLOSE_BUTTON,
        ),
    ]);
    let tab_labels = ["Equip", "Use", "Setup", "Etc", "Cash"];
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
        ui_foundation::draw_tab_label(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            tab_labels[index],
            bounds,
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
            let Some(item) = inventory_display_item(inventory, active_tab, slot, pending_move)
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
                    INVENTORY_SLOT_CELL_WIDTH,
                    INVENTORY_SLOT_CELL_HEIGHT,
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
    player_stats: Option<&openstory_protocol::CharacterStats>,
    player_inventory: Option<&openstory_protocol::InventorySnapshot>,
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
        [91, 48, 40, 255],
    );
    for (x, y, w, h) in [
        (0, 0, WIDTH, 1),
        (0, 0, 1, HEIGHT),
        (WIDTH - 1, 0, 1, HEIGHT),
        (0, HEIGHT - 1, WIDTH, 1),
    ] {
        fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, x, y, w, h, [230, 225, 228, 255]);
    }
    let equipment = item.item_id / 1_000_000 == 1;
    let item_name = item_name(item.item_id, string_archive, item_name_cache);
    let name = if equipment {
        match item.item_id / 1_000 % 10 {
            0 => format!("{item_name} (Male)"),
            1 => format!("{item_name} (Female)"),
            _ => item_name,
        }
    } else {
        item_name
    };
    let name = ui_foundation::ellipsize(font, &name, 13.0, (WIDTH - 18) as f32);
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        8,
        12,
        4,
        4,
        [255, 223, 107, 255],
    );
    ui_foundation::draw_text_role(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &name,
        18,
        8,
        ui_foundation::UiTextRole::Title,
        WIDTH - 28,
        18,
        Some([255, 255, 255]),
    );
    let info = inventory_item_info_node(item_archive, character_archive, item.item_id);
    let icon_size = if equipment { 64 } else { 32 };
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        11,
        30,
        icon_size + 4,
        icon_size + 4,
        [220, 220, 220, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        12,
        31,
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
            13,
            32,
            icon_size,
        );
    }
    if equipment {
        let required_job = info
            .as_ref()
            .and_then(|node| integer_property(node, "reqJob"))
            .unwrap_or(0);
        let equipped_bonuses = [0, 1, 2, 3].map(|stat_index| {
            player_inventory.map_or(0, |inventory| {
                inventory
                    .equipped()
                    .filter_map(|item| {
                        item.equip
                            .as_ref()?
                            .stats
                            .get(stat_index)
                            .copied()
                            .map(i64::from)
                    })
                    .sum::<i64>()
            })
        });
        let player_values = player_stats.map(|stats| {
            [
                i64::from(stats.level),
                i64::from(stats.strength) + equipped_bonuses[0],
                i64::from(stats.dexterity) + equipped_bonuses[1],
                i64::from(stats.intelligence) + equipped_bonuses[2],
                i64::from(stats.luck) + equipped_bonuses[3],
                i64::from(stats.fame),
            ]
        });
        for (row, (label, key, stat_index)) in [
            ("LEV", "reqLevel", 0),
            ("STR", "reqSTR", 1),
            ("DEX", "reqDEX", 2),
            ("INT", "reqINT", 3),
            ("LUK", "reqLUK", 4),
            ("FAM", "reqPOP", 5),
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
            let insufficient = player_values.is_some_and(|values| value > values[stat_index]);
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &format!("REQ {label} : {value}"),
                93,
                35 + row as i32 * 11,
                10.0,
                WIDTH - 101,
                12,
                if insufficient {
                    [255, 76, 76]
                } else {
                    [255, 255, 255]
                },
            );
        }
        let class_labels = [
            ("BEGINNER", 32_i64),
            ("WARRIOR", 1),
            ("MAGICIAN", 2),
            ("BOWMAN", 4),
            ("THIEF", 8),
            ("PIRATE", 16),
        ];
        const CLASS_BUTTON_Y: usize = 107;
        const CLASS_BUTTON_HEIGHT: usize = 14;
        const CLASS_BUTTON_PADDING: i32 = 2;
        let available_class_width = WIDTH as i32 - 10;
        let class_gap = 2_i32;
        let mut class_font_size = 8.0;
        let class_widths = loop {
            let widths = class_labels
                .iter()
                .map(|(label, _)| dialogue_text_width(font, label, class_font_size).ceil() as i32)
                .collect::<Vec<_>>();
            let total_width = widths.iter().sum::<i32>()
                + CLASS_BUTTON_PADDING * 2 * class_labels.len() as i32
                + class_gap * class_labels.len().saturating_sub(1) as i32;
            if total_width <= available_class_width || class_font_size <= 6.5 {
                break widths;
            }
            class_font_size -= 0.25;
        };
        let mut class_x = 5_i32;
        for ((label, class_bit), label_width) in class_labels.into_iter().zip(class_widths) {
            let available = required_job == 0 || required_job & class_bit != 0;
            let button_width = label_width + CLASS_BUTTON_PADDING * 2;
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                class_x as usize,
                CLASS_BUTTON_Y,
                button_width as usize,
                CLASS_BUTTON_HEIGHT,
                [0, 0, 0, 255],
            );
            let border = [101, 106, 128, 255];
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                class_x as usize,
                CLASS_BUTTON_Y,
                button_width as usize,
                1,
                border,
            );
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                class_x as usize,
                CLASS_BUTTON_Y + CLASS_BUTTON_HEIGHT - 1,
                button_width as usize,
                1,
                border,
            );
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                class_x as usize,
                CLASS_BUTTON_Y,
                1,
                CLASS_BUTTON_HEIGHT,
                border,
            );
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                class_x as usize + button_width as usize - 1,
                CLASS_BUTTON_Y,
                1,
                CLASS_BUTTON_HEIGHT,
                border,
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                label,
                class_x + CLASS_BUTTON_PADDING,
                CLASS_BUTTON_Y as i32 + 2,
                class_font_size,
                label_width.max(1) as usize + 1,
                12,
                if available {
                    [255, 175, 72]
                } else {
                    [133, 136, 155]
                },
            );
            class_x += button_width + class_gap;
        }
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            5,
            123,
            WIDTH - 10,
            1,
            [220, 220, 220, 255],
        );
        let item_prefix = item.item_id / 10_000;
        let item_type = match item_prefix {
            100 => "Hat",
            101 => "Face Accessory",
            102 => "Eye Accessory",
            103 => "Earrings",
            104 => "Top",
            105 => "Overall",
            106 => "Bottom",
            107 => "Shoes",
            108 => "Gloves",
            109 => "Shield",
            110 => "Cape",
            111 => "Ring",
            112 => "Pendant",
            130 => "One-Handed Sword",
            131 => "One-Handed Axe",
            132 => "One-Handed Mace",
            133 => "Dagger",
            137 => "Wand",
            138 => "Staff",
            140 => "Two-Handed Sword",
            141 => "Two-Handed Axe",
            142 => "Two-Handed Mace",
            143 => "Spear",
            144 => "Polearm",
            145 => "Bow",
            146 => "Crossbow",
            147 => "Claw",
            148 => "Knuckle",
            149 => "Gun",
            170 => "Wand",
            _ => "Equipment",
        };
        let is_weapon = matches!(item_prefix, 130..=149 | 170);
        let equip = item.equip.as_ref();
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
                94,
                9.0,
                WIDTH - 18,
                12,
                [255, 175, 72],
            );
        }
        let mut detail_rows = vec![format!("Type: {item_type}")];
        if is_weapon {
            if let Some(speed) = info
                .as_ref()
                .and_then(|node| integer_property(node, "attackSpeed"))
            {
                let speed_name = match speed {
                    2 => "Fastest",
                    3 => "Faster",
                    4 => "Fast",
                    5 => "Fast (5)",
                    6 => "Average",
                    7 => "Slow",
                    8 => "Very Slow",
                    _ => "Average",
                };
                detail_rows.push(format!("Attack Speed: {speed_name} ({speed})"));
            }
        }
        let mut bonus_rows = Vec::new();
        for (index, property, label) in [
            (0, "incSTR", "STR"),
            (1, "incDEX", "DEX"),
            (2, "incINT", "INT"),
            (3, "incLUK", "LUK"),
            (4, "incMHP", "HP"),
            (5, "incMMP", "MP"),
            (6, "incPAD", "Weapon Attack"),
            (7, "incMAD", "Magic Attack"),
            (8, "incPDD", "Weapon Def."),
            (9, "incMDD", "Magic Def."),
            (10, "incACC", "Accuracy"),
            (11, "incEVA", "Avoidability"),
            (12, "incHands", "Hands"),
            (13, "incSpeed", "Speed"),
            (14, "incJump", "Jump"),
        ] {
            let value = equip
                .and_then(|data| data.stats.get(index).copied().map(i64::from))
                .filter(|value| *value != 0)
                .or_else(|| {
                    info.as_ref()
                        .and_then(|node| integer_property(node, property))
                        .filter(|value| *value != 0)
                });
            if let Some(value) = value {
                bonus_rows.push(format!("{label}: {value:+}"));
            }
        }
        detail_rows.append(&mut bonus_rows);
        let critical_rate = info.as_ref().and_then(|node| {
            ["incCr", "incCR", "incCriticalRate", "incCrit"]
                .into_iter()
                .find_map(|property| integer_property(node, property))
        });
        if let Some(value) = critical_rate.filter(|value| *value != 0) {
            detail_rows.push(format!("Critical Rate: {value:+}%"));
        }
        let slots = equip.map_or_else(
            || {
                info.as_ref()
                    .and_then(|node| integer_property(node, "tuc"))
                    .unwrap_or(0)
            },
            |data| i64::from(data.upgrade_slots),
        );
        detail_rows.push(format!("Remaining Enhancements: {slots}"));
        if detail_rows.len() > 6 {
            let enhancements = detail_rows.pop().unwrap_or_default();
            let overflow = detail_rows.drain(4..).collect::<Vec<_>>().join(", ");
            detail_rows.push(format!("Other: {overflow}"));
            detail_rows.push(enhancements);
        }
        for (row, detail) in detail_rows.iter().enumerate() {
            let y = 130 + row as i32 * 12;
            for (offset, x, width) in [(0, 1, 2), (1, 0, 4), (2, 0, 4), (3, 1, 2)] {
                fill_dialogue_rect(
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    9 + x,
                    y as usize + 3 + offset,
                    width,
                    1,
                    [70, 175, 255, 255],
                );
            }
            let detail = ui_foundation::ellipsize(font, detail, 9.0, (WIDTH - 29) as f32);
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &detail,
                19,
                y,
                9.0,
                WIDTH - 29,
                12,
                [239, 231, 215],
            );
        }
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            5,
            204,
            WIDTH - 10,
            1,
            [184, 190, 209, 255],
        );
        let sell_price = info
            .as_ref()
            .and_then(|node| integer_property(node, "price"))
            .unwrap_or(0)
            .max(0)
            / 2;
        let sell_price = format_meso_amount(sell_price);
        let sell_price = sell_price.strip_suffix(" Mesos").unwrap_or(&sell_price);
        let price_label = format!("Shop Sell Price: {sell_price}");
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &price_label,
            9,
            207,
            8.5,
            WIDTH - 25,
            12,
            [239, 231, 215],
        );
        let coin_x = 9 + dialogue_text_width(font, &price_label, 8.5).ceil() as usize + 2;
        for (row, left, width) in [
            (0, 2, 3),
            (1, 1, 5),
            (2, 0, 7),
            (3, 0, 7),
            (4, 0, 7),
            (5, 1, 5),
            (6, 2, 3),
        ] {
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                coin_x + left,
                209 + row,
                width,
                1,
                [20, 138, 224, 255],
            );
        }
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            coin_x + 1,
            210,
            5,
            5,
            [38, 192, 255, 255],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            coin_x + 2,
            211,
            3,
            3,
            [94, 231, 255, 255],
        );
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
    item_icon: Option<&(u32, u32, Arc<[u8]>)>,
    string_archive: &NxArchive,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Result<Vec<u8>, String> {
    let icon = if prompt.action == InventoryQuantityAction::DropMesos {
        Some(QuantityIcon::Mesos)
    } else {
        item_icon.map(QuantityIcon::Item)
    };
    let (title, message) = match prompt.step {
        InventoryDropStep::Quantity => {
            let title = match prompt.action {
                InventoryQuantityAction::Drop => "DROP ITEM",
                InventoryQuantityAction::DropMesos => "DROP MESOS",
                InventoryQuantityAction::StorageStore => "STORE ITEMS",
                InventoryQuantityAction::StorageTake => "TAKE ITEMS",
            };
            let message = if icon.is_some() {
                String::new()
            } else {
                let name = item_name(prompt.item_id, string_archive, name_cache);
                ui_foundation::ellipsize(font, &name, 10.0, (QUANTITY_WINDOW_WIDTH - 20) as f32)
            };
            (title, message)
        }
        InventoryDropStep::ConfirmUntradable => {
            let message = if icon.is_some() {
                "This item cannot be recovered once dropped. Continue?".to_owned()
            } else {
                let name = item_name(prompt.item_id, string_archive, name_cache);
                format!("{name} cannot be recovered once dropped. Continue?")
            };
            ("CONFIRM ITEM DROP", message)
        }
        InventoryDropStep::Blocked => (
            "ITEM CANNOT BE DROPPED",
            "Cash items cannot be dropped.".to_owned(),
        ),
    };
    render_quantity_window(
        ui_archive,
        font,
        title,
        &message,
        icon,
        (prompt.step == InventoryDropStep::Quantity).then_some((
            &prompt.quantity,
            prompt.minimum(),
            prompt.max_quantity,
        )),
        prompt.notice.as_deref(),
        prompt.step,
    )
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
    let _ = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui).frame("UIWindow.img/Notice/0", 369, 311);
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

#[cfg(test)]
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

fn pet_equipment_drawer_items(
    inventory: &openstory_protocol::InventorySnapshot,
) -> Vec<&openstory_protocol::InventoryItem> {
    let mut items: Vec<_> = inventory
        .items
        .iter()
        .filter(|item| {
            (180..=183).contains(&(item.item_id / 10_000))
                && matches!(
                    item.inventory_type,
                    openstory_protocol::InventoryType::Equip
                        | openstory_protocol::InventoryType::Cash
                        | openstory_protocol::InventoryType::Equipped
                )
        })
        .collect();
    items.sort_by_key(|item| (item.inventory_type as u8, item.slot));
    items
}

// The native pet panel labels identify these v83 item categories. Equipped
// entries are matched by item identity, not by guessed server slot numbers.
#[cfg(test)]
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

#[cfg(test)]
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

    let mut dialog = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui_archive);
    let _ = dialog.frame("UIWindow.img/Equip/backgrnd", 0, 0);
    let _ = dialog.buttons(&[
        (
            ui_dialog::CLOSE_BUTTON_ART,
            ui_foundation::INVENTORY_CLOSE_BUTTON,
        ),
        (
            "UIWindow.img/Equip/BtPetEquipShow/normal/0",
            ui_foundation::EQUIPMENT_PET_BUTTON,
        ),
    ]);

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
    const WIDTH: usize = PET_EQUIPMENT_PANEL_WIDTH as usize;
    const HEIGHT: usize = PET_EQUIPMENT_PANEL_HEIGHT as usize;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    // Keep the equipment window's title bar, border, and blue footer.
    // Replace the baked-in doll grid with the pet and gear rows.
    let mut dialog = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui_archive);
    let _ = dialog.scaled_frame("UIWindow.img/Equip/backgrnd", 0, 0, WIDTH, HEIGHT);
    let _ = dialog.buttons(&[(
        ui_dialog::CLOSE_BUTTON_ART,
        ui_foundation::PET_EQUIPMENT_CLOSE_BUTTON,
    )]);
    fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, 5, 5, WIDTH - 28, 14, [255, 255, 255, 255]);
    fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, 6, 26, WIDTH - 12, 246, [250, 247, 244, 255]);
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "PET EQUIPMENT",
        8,
        6,
        10.0,
        184,
        17,
        [48, 55, 57],
    );
    let pets: Vec<_> = inventory
        .items
        .iter()
        .filter(|item| item.pet.is_some())
        .take(3)
        .collect();
    if pets.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "No pets found in cash inventory.",
            14,
            38,
            9.0,
            WIDTH - 28,
            16,
            [90, 96, 96],
        );
    }
    for (index, item) in pets.iter().enumerate() {
        let y = 32 + index * 34;
        fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, 9, y, WIDTH - 18, 31, [235, 224, 211, 255]);
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
                12,
                y + 1,
                28,
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
        let pet_name = ui_foundation::ellipsize(font, &pet_name, 10.0, 190.0);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &pet_name,
            48,
            (y + 2) as i32,
            10.0,
            196,
            14,
            [43, 48, 48],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!("Lv {}   Fullness {}%", pet.level, pet.fullness),
            48,
            (y + 16) as i32,
            8.0,
            196,
            12,
            [72, 79, 78],
        );
    }
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        8,
        135,
        WIDTH - 16,
        1,
        [187, 161, 131, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Pet gear",
        16,
        139,
        10.0,
        180,
        12,
        [48, 55, 57],
    );
    let gear = pet_equipment_drawer_items(inventory);
    let scroll_row = scroll_row.min(gear.len().saturating_sub(PET_EQUIPMENT_VISIBLE_ROWS));
    if gear.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "No pet gear in your inventory.",
            14,
            164,
            11.0,
            WIDTH - 28,
            24,
            [90, 96, 96],
        );
    }
    for (index, item) in gear
        .iter()
        .skip(scroll_row)
        .take(PET_EQUIPMENT_VISIBLE_ROWS)
        .enumerate()
    {
        let y = PET_EQUIPMENT_GEAR_LIST_Y as usize
            + index * PET_EQUIPMENT_GEAR_ROW_HEIGHT as usize;
        fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, 9, y, WIDTH - 18, 37, [235, 224, 211, 255]);
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
                12,
                y + 4,
                32,
            );
        }
        let name = item_name(item.item_id, string_archive, name_cache);
        let name = ui_foundation::ellipsize(font, &name, 9.0, 119.0);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &name,
            52,
            (y + 5) as i32,
            9.0,
            119,
            13,
            [43, 48, 48],
        );
        let location = match item.inventory_type {
            openstory_protocol::InventoryType::Equipped => "Equipped".to_owned(),
            _ if pet_equipment_destination(item.item_id, inventory).is_some() => {
                "In inventory".to_owned()
            }
            _ => "Cannot equip".to_owned(),
        };
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &location,
            52,
            (y + 21) as i32,
            8.0,
            119,
            12,
            [80, 88, 87],
        );
        let (label, enabled) =
            if item.inventory_type == openstory_protocol::InventoryType::Equipped {
                ("Remove", true)
            } else if pet_equipment_destination(item.item_id, inventory).is_some() {
                ("Equip", true)
            } else {
                ("Unavailable", false)
            };
        let bounds = ui_foundation::UiRect::new(WIDTH as f32 - 82.0, y as f32 + 9.0, 70.0, 19.0);
        let _ = composite_nx_ui_tab(&mut pixels, WIDTH, HEIGHT, ui_archive, bounds, false);
        let label_width = dialogue_text_width(font, label, 11.0);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            (bounds.x + (bounds.width - label_width) / 2.0).round() as i32,
            bounds.y as i32 + 3,
            11.0,
            bounds.width as usize,
            16,
            if enabled { [48, 48, 48] } else { [103, 103, 103] },
        );
    }
    if !gear.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!("{}–{} / {}", scroll_row + 1, (scroll_row + PET_EQUIPMENT_VISIBLE_ROWS).min(gear.len()), gear.len()),
            16,
            HEIGHT as i32 - 21,
            9.0,
            150,
            12,
            [80, 88, 87],
        );
        if gear.len() > PET_EQUIPMENT_VISIBLE_ROWS {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                "Scroll",
                WIDTH as i32 - 58,
                HEIGHT as i32 - 21,
                9.0,
                45,
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
    fn pet_equip_targets_use_the_pet_equipment_slot() {
        let empty = InventorySnapshot::default();
        assert_eq!(pet_equipment_destination(1_822_000, &empty), Some(114));
        let packet = openstory_protocol::move_item_packet(InventoryType::Equip, 3, -114, 1, 0);
        assert_eq!(&packet[9..11], &(-114_i16).to_le_bytes());
        assert_eq!(pet_equipment_destination(1_812_000, &empty), Some(123));
        let packet = openstory_protocol::move_item_packet(InventoryType::Equip, 3, -123, 1, 0);
        assert_eq!(&packet[9..11], &(-123_i16).to_le_bytes());
        let mut inventory = empty;
        inventory.items.push(InventoryItem {
            inventory_type: InventoryType::Equipped,
            slot: 114,
            item_id: 1_000_000,
            ..Default::default()
        });
        assert_eq!(pet_equipment_destination(1_822_000, &inventory), None);
        assert_eq!(pet_equipment_destination(1_812_000, &inventory), Some(123));
        inventory.items.push(InventoryItem {
            inventory_type: InventoryType::Equipped,
            slot: 123,
            item_id: 1_812_000,
            ..Default::default()
        });
        assert_eq!(pet_equipment_destination(1_812_000, &inventory), Some(123));
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
