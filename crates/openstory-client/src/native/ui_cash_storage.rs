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

fn render_storage_reference_panel(
    font: &fontdue::Font,
    storage: &openstory_protocol::StorageSnapshot,
    _active_tab: openstory_protocol::InventoryType,
    storage_scroll: usize,
    meso_input: &str,
    meso_focused: bool,
    notice: Option<&str>,
    ui_archive: &NxArchive,
    npc_archive: &NxArchive,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
) -> Vec<u8> {
    const WIDTH: usize = 206;
    const HEIGHT: usize = 355;
    const VISIBLE: usize = 30;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        HEIGHT,
        [95, 118, 137, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        1,
        1,
        WIDTH - 2,
        HEIGHT - 2,
        [250, 250, 250, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        3,
        3,
        WIDTH - 6,
        79,
        [225, 225, 220, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        3,
        84,
        WIDTH - 6,
        2,
        [245, 147, 183, 255],
    );
    // The banker portrait and three right-side actions occupy the same header
    // band as the v83 StoreBank window.
    if !draw_storage_keeper(
        &mut pixels,
        WIDTH,
        HEIGHT,
        npc_archive,
        storage.npc_id,
        39,
        70,
    ) {
        draw_storage_keeper_placeholder(&mut pixels, WIDTH, HEIGHT, 16, 15);
    }
    for (y, path, anchor_x, anchor_y, label, face) in [
        (7, "UIWindow.img/Trunk/BtExit/normal/0", 132, 7, "EXIT", [86, 190, 190, 255]),
        (29, "UIWindow.img/Trunk/BtGet/normal/0", 132, 29, "TAKE OUT", [80, 153, 225, 255]),
        (51, "UIWindow.img/Trunk/BtSort/normal/0", 167, 60, "ORGANIZE ITEM", [194, 167, 130, 255]),
    ] {
        if composite_nx_ui_bitmap(
            &mut pixels, WIDTH, HEIGHT, ui_archive, path, anchor_x, anchor_y,
        ).is_err() {
            fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, 132, y, 70, 18, face);
            draw_dialogue_text(
                font, &mut pixels, WIDTH, HEIGHT, label, 135, y as i32 + 4,
                7.0, 63, 12, [77, 67, 51],
            );
        }
    }

    // Keep the reference's compact All tab and pale, tightly packed item grid.
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        4,
        91,
        34,
        19,
        [136, 102, 238, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "All",
        11,
        94,
        11.0,
        24,
        14,
        [255, 255, 255],
    );
    let stored_items = storage.items.iter().collect::<Vec<_>>();
    for slot in 0..VISIBLE {
        let cell = ui_foundation::STORAGE_GRID.cell(slot);
        let (x, y) = (cell.x as usize, cell.y as usize);
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            y,
            34,
            34,
            [226, 230, 230, 255],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            y,
            34,
            1,
            [194, 205, 211, 255],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            y,
            1,
            34,
            [194, 205, 211, 255],
        );
        let Some(item) = stored_items.get(storage_scroll + slot).copied() else {
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
        if item.count > 1 {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &item.count.to_string(),
                x as i32 + 20,
                y as i32 + 23,
                9.0,
                13,
                11,
                [255, 255, 255],
            );
        }
    }

    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        5,
        332,
        195,
        1,
        [163, 177, 185, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        8,
        337,
        94,
        15,
        if meso_focused {
            [246, 247, 238, 255]
        } else {
            [231, 234, 231, 255]
        },
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &format!("{} mesos", storage.meso),
        10,
        338,
        10.0,
        84,
        13,
        [79, 77, 65],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        meso_input,
        106,
        338,
        9.0,
        43,
        13,
        [67, 76, 89],
    );
    for (x, label, face) in [
        (151, "IN", [82, 176, 92, 255]),
        (176, "OUT", [86, 156, 207, 255]),
    ] {
        fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, x, 337, 23, 15, face);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            x as i32 + 5,
            338,
            8.0,
            18,
            11,
            [255, 255, 255],
        );
    }
    if let Some(notice) = notice {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            notice,
            7,
            354 - 12,
            8.0,
            WIDTH - 14,
            11,
            [161, 70, 66],
        );
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

fn draw_storage_keeper_placeholder(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
) {
    let put = |pixels: &mut [u8], px: usize, py: usize, w: usize, h: usize, color: [u8; 4]| {
        fill_dialogue_rect(pixels, width, height, px, py, w, h, color);
    };
    put(pixels, x + 7, y, 22, 8, [27, 30, 38, 255]);
    put(pixels, x + 4, y + 4, 27, 9, [53, 56, 61, 255]);
    put(pixels, x + 7, y + 11, 21, 20, [190, 139, 92, 255]);
    put(pixels, x + 9, y + 16, 4, 4, [35, 35, 36, 255]);
    put(pixels, x + 22, y + 16, 4, 4, [35, 35, 36, 255]);
    put(pixels, x + 12, y + 24, 13, 4, [35, 27, 25, 255]);
    put(pixels, x + 4, y + 31, 27, 18, [59, 71, 71, 255]);
    put(pixels, x, y + 35, 6, 5, [190, 139, 92, 255]);
    put(pixels, x + 30, y + 35, 6, 5, [190, 139, 92, 255]);
    put(pixels, x + 8, y + 47, 7, 6, [46, 40, 38, 255]);
    put(pixels, x + 21, y + 47, 7, 6, [46, 40, 38, 255]);
}
