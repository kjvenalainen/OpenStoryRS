fn load_chat_balloon_skin(archive: &NxArchive) -> Result<ChatBalloonSkin, String> {
    load_chat_balloon_skin_at(archive, "ChatBalloon.img/0")
}

fn load_pet_chat_balloon_skin(
    archive: &NxArchive,
    style_id: i32,
) -> Result<ChatBalloonSkin, String> {
    load_chat_balloon_skin_at(archive, &format!("ChatBalloon.img/pet/{style_id}"))
}

fn load_chat_balloon_skin_at(archive: &NxArchive, path: &str) -> Result<ChatBalloonSkin, String> {
    let source = archive
        .root()
        .resolve(path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("UI.nx is missing {path}"))?;
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
    marked_items: &HashSet<InventorySellMark>,
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
    let _ = dialog.authored_window(
        &[("UIWindow.img/Item/backgrnd", 0, 0)],
        ui_foundation::INVENTORY_CLOSE_BUTTON,
    );
    let _ = dialog.buttons(&[
        (
            "UIWindow.img/Item/BtGather/normal/0",
            ui_foundation::INVENTORY_GATHER_BUTTON,
        ),
        (
            "UIWindow.img/Item/BtSort/normal/0",
            ui_foundation::INVENTORY_SORT_BUTTON,
        ),
    ]);
    for (index, (tab, label)) in inventory_tabs()
        .into_iter()
        .zip(["Equip", "Use", "Setup", "Etc", "Cash"])
        .enumerate()
    {
        let bounds = ui_foundation::inventory_tab(index);
        let selected = tab == active_tab;
        let state = if selected { "enabled" } else { "disabled" };
        let _ = dialog.tab(bounds, selected);
        if dialog
            .buttons(&[(&format!("UIWindow.img/Item/Tab/{state}/{index}"), bounds)])
            .is_err()
        {
            let _ = dialog.labeled_tab(bounds, selected, font, label);
        }
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
                if marked_items.contains(&inventory_sell_mark(item)) {
                    draw_inventory_sell_mark(&mut pixels, WIDTH, HEIGHT, x as i32, y as i32);
                }
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

fn draw_inventory_sell_mark(pixels: &mut [u8], width: usize, height: usize, left: i32, top: i32) {
    let lines = [((3, 3), (29, 28)), ((29, 3), (3, 28))];
    for ((start_x, start_y), (end_x, end_y)) in lines {
        for step in 0..=26 {
            let progress = step as f32 / 26.0;
            let x = start_x + ((end_x - start_x) as f32 * progress).round() as i32;
            let y = start_y + ((end_y - start_y) as f32 * progress).round() as i32;
            for (radius, color) in [(2, [0, 0, 0, 255]), (1, [0, 0, 255, 255])] {
                for offset in -radius..=radius {
                    let (px, py) = if start_x < end_x {
                        (left + x + offset, top + y)
                    } else {
                        (left + x - offset, top + y)
                    };
                    if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                        continue;
                    }
                    let index = (py as usize * width + px as usize) * 4;
                    pixels[index..index + 4].copy_from_slice(&color);
                }
            }
        }
    }
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
mod pet_equip_destination_tests {
    use super::*;
    use openstory_protocol::{InventoryItem, InventorySnapshot, InventoryType};

    #[test]
    fn pet_equip_targets_use_their_dedicated_equipped_slots() {
        let empty = InventorySnapshot::default();
        assert_eq!(pet_equipment_destination(1_822_000, &empty), Some(114));
        let packet = openstory_protocol::move_item_packet(InventoryType::Equip, 3, -114, 1, 0);
        assert_eq!(&packet[9..11], &(-114_i16).to_le_bytes());
        assert_eq!(pet_equipment_destination(1_802_000, &empty), Some(121));
        assert_eq!(pet_equipment_destination(1_812_000, &empty), Some(122));
        assert_eq!(pet_equipment_destination(1_812_002, &empty), Some(124));
        assert_eq!(pet_equipment_destination(1_812_003, &empty), Some(125));
        assert_eq!(pet_equipment_destination(1_812_007, &empty), Some(129));
        assert_eq!(pet_equipment_destination(1_832_000, &empty), Some(113));
        let packet = openstory_protocol::move_item_packet(InventoryType::Equip, 3, -122, 1, 0);
        assert_eq!(&packet[9..11], &(-122_i16).to_le_bytes());
        let packet = openstory_protocol::move_item_packet(InventoryType::Equip, 3, -124, 1, 0);
        assert_eq!(&packet[9..11], &(-124_i16).to_le_bytes());
        let mut inventory = empty;
        inventory.items.push(InventoryItem {
            inventory_type: InventoryType::Equipped,
            slot: 114,
            item_id: 1_000_000,
            ..Default::default()
        });
        assert_eq!(pet_equipment_destination(1_822_000, &inventory), None);
        assert_eq!(pet_equipment_destination(1_812_000, &inventory), Some(122));
        inventory.items.push(InventoryItem {
            inventory_type: InventoryType::Equipped,
            slot: 122,
            item_id: 1_812_000,
            ..Default::default()
        });
        assert_eq!(pet_equipment_destination(1_812_000, &inventory), Some(122));
        assert_eq!(pet_equipment_destination(1_832_000, &inventory), Some(113));
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
