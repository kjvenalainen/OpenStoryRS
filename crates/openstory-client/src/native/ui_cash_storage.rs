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

fn storage_tabs() -> [openstory_protocol::InventoryType; 5] {
    [
        openstory_protocol::InventoryType::Equip,
        openstory_protocol::InventoryType::Use,
        openstory_protocol::InventoryType::Setup,
        openstory_protocol::InventoryType::Etc,
        openstory_protocol::InventoryType::Cash,
    ]
}

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
