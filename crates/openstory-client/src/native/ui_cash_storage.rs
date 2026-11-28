fn cash_filtered_indices(
    products: &[CashShopProduct],
    state: &CashShopUi,
    string_archive: &NxArchive,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<usize> {
    let query = state.query.trim().to_lowercase();
    products
        .iter()
        .enumerate()
        .filter_map(|(index, product)| {
            if !cash_product_matches_category(product.item_id, state.category) {
                return None;
            }
            if !query.is_empty()
                && !item_name(product.item_id, string_archive, name_cache)
                    .to_lowercase()
                    .contains(&query)
            {
                return None;
            }
            Some(index)
        })
        .collect()
}

fn cash_product_matches_category(item_id: i32, category: usize) -> bool {
    let item_type = item_id / 1_000_000;
    let item_prefix = item_id / 10_000;
    match category {
        0 => true,
        1 => item_type == 1,
        2 => item_type == 2,
        3 => item_type == 3,
        4 => item_type == 4,
        5 => item_type >= 5,
        6 => item_prefix == 500,
        7 => item_prefix == 180 || item_prefix == 181,
        8 => (182..190).contains(&item_prefix) || item_prefix == 212,
        _ => false,
    }
}

#[allow(dead_code)] // Kept by the legacy two-pane renderer below for older snapshots.
fn storage_tabs() -> [openstory_protocol::InventoryType; 5] {
    [
        openstory_protocol::InventoryType::Equip,
        openstory_protocol::InventoryType::Use,
        openstory_protocol::InventoryType::Setup,
        openstory_protocol::InventoryType::Etc,
        openstory_protocol::InventoryType::Cash,
    ]
}

#[allow(dead_code)] // Retained as a layout fallback for old storage packet captures.
fn render_storage_panel(
    font: &fontdue::Font,
    storage: &openstory_protocol::StorageSnapshot,
    inventory: &openstory_protocol::InventorySnapshot,
    active_tab: openstory_protocol::InventoryType,
    storage_scroll: usize,
    inventory_scroll: usize,
    meso_input: &str,
    meso_focused: bool,
    notice: Option<&str>,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<u8> {
    const WIDTH: usize = 660;
    const HEIGHT: usize = 560;
    const ROW_HEIGHT: usize = 38;
    const FIRST_ROW_Y: usize = 125;
    const VISIBLE_ROWS: usize = 8;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        HEIGHT,
        [24, 26, 30, 248],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        2,
        [202, 168, 108, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        HEIGHT - 2,
        WIDTH,
        2,
        [202, 168, 108, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        2,
        HEIGHT,
        [202, 168, 108, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        WIDTH - 2,
        0,
        2,
        HEIGHT,
        [202, 168, 108, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Storage",
        18,
        11,
        20.0,
        230,
        26,
        [246, 226, 185],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &format!(
            "{} / {} slots     NPC {}",
            storage.items.len(),
            storage.slot_capacity,
            storage.npc_id
        ),
        180,
        17,
        12.0,
        420,
        18,
        [206, 203, 194],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        620,
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
        628,
        9,
        14.0,
        14,
        18,
        [255, 225, 210],
    );

    let labels = ["EQUIP", "USE", "SETUP", "ETC", "CASH"];
    for (index, (tab, label)) in storage_tabs().into_iter().zip(labels).enumerate() {
        let x = 14 + index * 128;
        let selected = tab == active_tab;
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            56,
            120,
            30,
            if selected {
                [104, 76, 49, 255]
            } else {
                [45, 47, 51, 255]
            },
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            56,
            120,
            1,
            if selected {
                [230, 190, 112, 255]
            } else {
                [91, 86, 76, 255]
            },
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            (x + 36) as i32,
            63,
            10.0,
            70,
            14,
            if selected {
                [255, 236, 192]
            } else {
                [204, 202, 194]
            },
        );
    }

    for (x, title) in [
        (14, "IN STORAGE · CLICK TO TAKE"),
        (336, "YOUR INVENTORY · CLICK TO STORE"),
    ] {
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            96,
            310,
            26,
            [38, 40, 44, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            title,
            x as i32 + 8,
            102,
            10.0,
            294,
            14,
            [227, 214, 184],
        );
    }
    let stored_items = storage
        .items
        .iter()
        .filter(|item| item.inventory_type == active_tab)
        .collect::<Vec<_>>();
    let inventory_items = inventory
        .items
        .iter()
        .filter(|item| item.inventory_type == active_tab)
        .collect::<Vec<_>>();
    for row in 0..VISIBLE_ROWS {
        let y = FIRST_ROW_Y + row * ROW_HEIGHT;
        for (panel_x, items, first_visible) in [
            (14, &stored_items, storage_scroll),
            (336, &inventory_items, inventory_scroll),
        ] {
            let index = first_visible + row;
            let enabled = index < items.len();
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                panel_x,
                y,
                310,
                ROW_HEIGHT - 2,
                if enabled {
                    [34, 36, 40, 255]
                } else {
                    [28, 29, 32, 255]
                },
            );
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                panel_x,
                y + ROW_HEIGHT - 2,
                310,
                1,
                [71, 68, 62, 255],
            );
            let Some(item) = items.get(index).copied() else {
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
                    panel_x + 5,
                    y + 2,
                    32,
                );
            }
            let name = item_name(item.item_id, string_archive, name_cache);
            let clipped_name = name.chars().take(27).collect::<String>();
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &clipped_name,
                (panel_x + 44) as i32,
                (y + 5) as i32,
                11.0,
                216,
                16,
                [231, 229, 220],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &format!("x{}", item.count.max(1)),
                (panel_x + 260) as i32,
                (y + 20) as i32,
                9.0,
                42,
                13,
                [202, 193, 171],
            );
        }
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &format!(
            "Storage mesos: {}     Character mesos: {}",
            storage.meso, inventory.meso
        ),
        18,
        442,
        12.0,
        624,
        18,
        [232, 224, 205],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        14,
        470,
        146,
        38,
        [75, 56, 39, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        190,
        470,
        210,
        38,
        if meso_focused {
            [57, 64, 73, 255]
        } else {
            [35, 37, 41, 255]
        },
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        500,
        470,
        146,
        38,
        [55, 65, 45, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "DEPOSIT",
        50,
        481,
        11.0,
        90,
        15,
        [255, 236, 192],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        meso_input,
        204,
        481,
        12.0,
        178,
        15,
        [245, 245, 239],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "WITHDRAW",
        535,
        481,
        11.0,
        100,
        15,
        [231, 242, 209],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        notice
            .unwrap_or("Click a stack to choose its quantity; click Deposit or Withdraw for mesos. Esc closes storage."),
        18,
        522,
        10.0,
        624,
        22,
        if notice.is_some() {
            [255, 190, 115]
        } else {
            [185, 183, 175]
        },
    );
    pixels
}

const STORAGE_PANEL_WIDTH: usize = 463;
const STORAGE_PANEL_HEIGHT: usize = 318;
const STORAGE_LEFT_ROWS: usize = 5;
const STORAGE_RIGHT_ROWS: usize = 4;

fn storage_row_at(x: f32, y: f32, inventory: bool) -> Option<usize> {
    let (left, top, rows) = if inventory { (234.0, 126.0, STORAGE_RIGHT_ROWS) } else { (4.0, 86.0, STORAGE_LEFT_ROWS) };
    if !(left..left + 200.0).contains(&x) || y < top { return None; }
    let offset = y - top;
    let row = (offset / 40.0) as usize;
    (row < rows && offset % 40.0 < 36.0).then_some(row)
}

fn storage_scrollbar_position(x: f32, y: f32, inventory: bool, count: usize, offset: usize) -> Option<usize> {
    let (left, top, rows) = if inventory { (440.0, 126.0, STORAGE_RIGHT_ROWS) } else { (209.0, 86.0, STORAGE_LEFT_ROWS) };
    let height = rows as f32 * 40.0;
    if !(left..left + 12.0).contains(&x) || !(top..top + height).contains(&y) { return None; }
    let maximum = count.saturating_sub(rows);
    let next = if y < top + 12.0 { offset.saturating_sub(1) }
        else if y >= top + height - 12.0 { offset.saturating_add(1) }
        else { (((y - top - 18.0) / (height - 36.0)).clamp(0.0, 1.0) * maximum as f32).round() as usize };
    Some(next.min(maximum))
}

fn render_storage_reference_panel(
    font: &fontdue::Font,
    storage: &openstory_protocol::StorageSnapshot,
    inventory: &openstory_protocol::InventorySnapshot,
    active_tab: openstory_protocol::InventoryType,
    storage_scroll: usize,
    inventory_scroll: usize,
    selected_storage: Option<(openstory_protocol::InventoryType, u16, i32)>,
    selected_inventory: Option<(openstory_protocol::InventoryType, u16, i32)>,
    meso_input: &str,
    meso_focused: bool,
    notice: Option<&str>,
    ui_archive: &NxArchive,
    npc_archive: &NxArchive,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<u8> {
    const WIDTH: usize = STORAGE_PANEL_WIDTH;
    const HEIGHT: usize = STORAGE_PANEL_HEIGHT;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    let _ = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui_archive).authored_window_with_controls(
        &[("UIWindow.img/Trunk/backgrnd", 0, 0)],
        &[
            ("UIWindow.img/Trunk/BtGet/normal/0", 151, 12),
            ("UIWindow.img/Trunk/BtSort/disabled/0", 186, 43),
            ("UIWindow.img/Trunk/BtPut/normal/0", 383, 12),
            ("UIWindow.img/Trunk/BtExit/normal/0", 383, 56),
            ("UIWindow.img/Trunk/BtInCoin/normal/0", 3, 293),
            ("UIWindow.img/Trunk/BtOutCoin/normal/0", 424, 293),
        ],
    );
    draw_storage_keeper(&mut pixels, WIDTH, HEIGHT, npc_archive, storage.npc_id, 53, 73);
    let tabs = [openstory_protocol::InventoryType::Equip, openstory_protocol::InventoryType::Use, openstory_protocol::InventoryType::Setup, openstory_protocol::InventoryType::Etc, openstory_protocol::InventoryType::Cash];
    let tab_labels = ["Equip", "Use", "Setup", "Etc", "Cash"];
    for (index, tab) in tabs.into_iter().enumerate() {
        let bounds = ui_foundation::storage_tab(index);
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
    let available: Vec<_> = inventory.items.iter().filter(|item| item.inventory_type == active_tab && item.slot > 0).collect();
    for (x, top, rows, count, offset) in [(209, 86, STORAGE_LEFT_ROWS, storage.items.len(), storage_scroll), (440, 126, STORAGE_RIGHT_ROWS, available.len(), inventory_scroll)] {
        let maximum = count.saturating_sub(rows);
        let enabled = maximum > 0;
        let (up, down) = if enabled { ("Basic.img/VScr/enabled/prev0", "Basic.img/VScr/enabled/next0") } else { ("Basic.img/VScr/disabled/prev", "Basic.img/VScr/disabled/next") };
        let height = (rows * 40) as i32;
        let _ = composite_nx_ui_bitmap(&mut pixels, WIDTH, HEIGHT, ui_archive, up, x, top);
        let _ = composite_nx_ui_bitmap(&mut pixels, WIDTH, HEIGHT, ui_archive, down, x, top + height - 12);
        if enabled {
            let y = top + 12 + ((height - 36) as usize * offset.min(maximum) / maximum) as i32;
            let _ = composite_nx_ui_bitmap(&mut pixels, WIDTH, HEIGHT, ui_archive, "Basic.img/VScr/enabled/thumb0", x, y);
        }
    }
    for (items, start, rows, x, top, selected) in [
        (storage.items.iter().collect::<Vec<_>>(), storage_scroll, STORAGE_LEFT_ROWS, 5_usize, 87_usize, selected_storage),
        (available, inventory_scroll, STORAGE_RIGHT_ROWS, 235_usize, 127_usize, selected_inventory),
    ] {
        for (row, item) in items.into_iter().skip(start).take(rows).enumerate() {
            let y = top + row * 40;
            if selected == Some((item.inventory_type, item.slot, item.item_id)) {
                let _ = composite_nx_ui_bitmap(&mut pixels, WIDTH, HEIGHT, ui_archive, "UIWindow.img/Trunk/select", (x + 40) as i32, y as i32);
            }
            if let Some((iw, ih, icon)) = inventory_item_icon(item.item_id, item_archive, character_archive, icon_cache) {
                composite_inventory_icon(&mut pixels, WIDTH, HEIGHT, &icon, iw as usize, ih as usize, x + 2, y + 2, 32);
            }
            let name = item_name(item.item_id, string_archive, name_cache);
            let label = ui_foundation::ellipsize(font, &name, 10.0, 155.0);
            draw_dialogue_text(font, &mut pixels, WIDTH, HEIGHT, &label, (x + 40) as i32, (y + 3) as i32, 10.0, 155, 14, [35, 38, 40]);
            if item.count > 1 { draw_dialogue_text(font, &mut pixels, WIDTH, HEIGHT, &format!("x{}", item.count), (x + 40) as i32, (y + 19) as i32, 9.0, 155, 12, [60, 65, 68]); }
        }
    }
    for (value, right, width) in [
        (i64::from(storage.meso), 193, 150),
        (inventory.meso, 420, 146),
    ] {
        draw_inventory_meso_amount(font, &mut pixels, WIDTH, HEIGHT, value, right, 297, width);
    }
    // The amount field is shared by the two native transfer buttons.
    draw_dialogue_text(font, &mut pixels, WIDTH, HEIGHT, "Amount", 263, 35, 10.0, 104, 14, [55, 55, 55]);
    fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, 263, 52, 104, 17, if meso_focused { [255, 255, 255, 255] } else { [237, 237, 232, 255] });
    let amount = ui_foundation::ellipsize(font, meso_input, 10.0, 98.0);
    draw_dialogue_text(font, &mut pixels, WIDTH, HEIGHT, &amount, 266, 54, 10.0, 98, 13, [40, 40, 40]);
    if let Some(notice) = notice {
        let notice = ui_foundation::ellipsize(font, notice, 9.0, 210.0);
        draw_dialogue_text(font, &mut pixels, WIDTH, HEIGHT, &notice, 238, 72, 9.0, 210, 13, [150, 35, 35]);
    }
    pixels
}

fn draw_storage_keeper(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    archive: &NxArchive,
    npc_id: i32,
    anchor_x: i32,
    anchor_y: i32,
) -> bool {
    let path = format!("{:07}.img/stand/0", npc_id.max(0));
    let Some(node) = archive.root().resolve(&path).ok().flatten() else {
        return false;
    };
    let Ok(Some(bitmap)) = node.bitmap() else {
        return false;
    };
    let Ok(source) = bitmap.decode_bgra_shared() else {
        return false;
    };
    let origin = node
        .child("origin")
        .ok()
        .flatten()
        .and_then(|origin| origin.value().ok())
        .and_then(|value| match value {
            openstory_assets::NodeValue::Vector { x, y } => Some((x, y)),
            _ => None,
        })
        .unwrap_or((0, 0));
    let source_width = usize::from(bitmap.width());
    let source_height = usize::from(bitmap.height());
    let scale = (46.0 / source_width as f32)
        .min(58.0 / source_height as f32)
        .min(1.0);
    let width = (source_width as f32 * scale).round().max(1.0) as usize;
    let height = (source_height as f32 * scale).round().max(1.0) as usize;
    let left = anchor_x - (origin.0 as f32 * scale).round() as i32;
    let top = anchor_y - (origin.1 as f32 * scale).round() as i32;
    for y in 0..height {
        let dy = top + y as i32;
        if !(0..canvas_height as i32).contains(&dy) {
            continue;
        }
        for x in 0..width {
            let dx = left + x as i32;
            if !(0..canvas_width as i32).contains(&dx) {
                continue;
            }
            let sx = (x as f32 / scale).floor() as usize;
            let sy = (y as f32 / scale).floor() as usize;
            let source_index = (sy * source_width + sx) * 4;
            let destination_index = (dy as usize * canvas_width + dx as usize) * 4;
            let alpha = u32::from(source[source_index + 3]);
            let inverse = 255 - alpha;
            for channel in 0..3 {
                let foreground = u32::from(source[source_index + channel]);
                let background = u32::from(pixels[destination_index + channel]);
                pixels[destination_index + channel] =
                    ((foreground * alpha + background * inverse) / 255) as u8;
            }
            let background_alpha = u32::from(pixels[destination_index + 3]);
            pixels[destination_index + 3] =
                (alpha + background_alpha * inverse / 255).min(255) as u8;
        }
    }
    true
}

#[cfg(test)]
mod storage_layout_tests {
    use super::*;

    #[test]
    fn row_hit_testing_matches_both_native_lists_and_their_gaps() {
        assert_eq!(storage_row_at(5.0, 86.0, false), Some(0));
        assert_eq!(storage_row_at(5.0, 121.0, false), Some(0));
        assert_eq!(storage_row_at(5.0, 122.0, false), None);
        assert_eq!(storage_row_at(5.0, 246.0, false), Some(4));
        assert_eq!(storage_row_at(5.0, 286.0, false), None);
        assert_eq!(storage_row_at(235.0, 126.0, true), Some(0));
        assert_eq!(storage_row_at(235.0, 246.0, true), Some(3));
        assert_eq!(storage_row_at(235.0, 286.0, true), None);
    }

    #[test]
    fn scrollbar_clicks_clamp_and_respect_list_lengths() {
        assert_eq!(storage_scrollbar_position(209.0, 86.0, false, 20, 0), Some(0));
        assert_eq!(storage_scrollbar_position(209.0, 285.0, false, 20, 0), Some(1));
        assert_eq!(storage_scrollbar_position(209.0, 268.0, false, 20, 0), Some(15));
        assert_eq!(storage_scrollbar_position(440.0, 285.0, true, 2, 0), Some(0));
        assert_eq!(storage_scrollbar_position(439.0, 200.0, true, 20, 0), None);
    }
}

