fn render_character_info_panel(
    font: &fontdue::Font,
    ui: &CharacterInfoUi,
    ui_archive: &NxArchive,
    string_archive: &NxArchive,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<u8> {
    const WIDTH: usize = CHARACTER_INFO_PANEL_WIDTH as usize;
    const HEIGHT: usize = CHARACTER_INFO_PANEL_HEIGHT as usize;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    let _ = composite_nx_ui_bitmap_scaled_to(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "UIWindow.img/UserInfo/backgrnd2",
        0,
        0,
        WIDTH,
        HEIGHT,
    );
    // Keep the legacy character card visible above the supplemental details.
    // The latter occupy a plain area so long server-provided text stays legible.
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        5,
        216,
        WIDTH - 10,
        261,
        [241, 242, 240, 255],
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "Basic.img/BtClose/normal/0",
        368,
        8,
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &ui.name,
        157,
        38,
        12.0,
        210,
        17,
        [255, 255, 255],
    );
    let info = &ui.info;
    for (index, value) in [
        info.level.to_string(),
        character_job_name(info.job),
        info.fame.to_string(),
        if info.guild.is_empty() {
            "-".to_owned()
        } else {
            info.guild.clone()
        },
        if info.alliance.is_empty() {
            "-".to_owned()
        } else {
            info.alliance.clone()
        },
    ]
    .iter()
    .enumerate()
    {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            value,
            222,
            95 + index as i32 * 22,
            10.0,
            152,
            17,
            [49, 45, 42],
        );
    }
    let pet_text = if info.pets.is_empty() {
        "No active pets".to_owned()
    } else {
        info.pets
            .iter()
            .take(3)
            .map(|pet| {
                let name = if pet.name.is_empty() {
                    item_name(pet.item_id, string_archive, name_cache)
                } else {
                    pet.name.clone()
                };
                format!(
                    "{name} Lv {} ({}/{})",
                    pet.level, pet.closeness, pet.fullness
                )
            })
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let mount_text = info.mount.map_or_else(
        || "Mount: none".to_owned(),
        |mount| {
            format!(
                "Mount: Lv {} · EXP {} · tiredness {}",
                mount.level, mount.experience, mount.tiredness
            )
        },
    );
    let wishlist = if info.wishlist.is_empty() {
        "No wish-list items".to_owned()
    } else {
        info.wishlist
            .iter()
            .take(4)
            .map(|id| item_name(*id, string_archive, name_cache))
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let details = [
        ("PETS", pet_text),
        ("MOUNT", mount_text),
        ("WISHLIST", wishlist),
        (
            "MONSTER BOOK",
            format!(
                "Lv {} · {} cards ({}/{})",
                info.monster_book.level,
                info.monster_book.total_cards,
                info.monster_book.normal_cards,
                info.monster_book.special_cards
            ),
        ),
    ];
    for (index, (label, value)) in details.iter().enumerate() {
        let y = 226 + index as i32 * 40;
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            20,
            y,
            9.0,
            150,
            14,
            [125, 56, 86],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            value,
            20,
            y + 14,
            10.0,
            350,
            23,
            [49, 45, 42],
        );
    }
    if let Some(medal) = info.medal.as_ref() {
        let medal_name = item_name(medal.item_id, string_archive, name_cache);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!("Medal: {medal_name} · {} quests", medal.quest_ids.len()),
            20,
            389,
            9.0,
            350,
            13,
            [49, 45, 42],
        );
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        if info.married { "Married" } else { "Single" },
        157,
        58,
        9.0,
        180,
        14,
        [255, 255, 255],
    );
    for (x, path) in [
        (20, "UIWindow.img/UserInfo/BtParty/normal/0"),
        (118, "UIWindow.img/UserInfo/BtTrade/normal/0"),
    ] {
        let _ = composite_nx_ui_bitmap_scaled_to(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            path,
            x,
            402,
            92,
            30,
        );
    }
    draw_guild_button(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        (216, 402, 92, 30),
        "GUILD",
    );
    for (x, label) in [
        (
            20,
            if ui.fame_pending {
                "WAITING…"
            } else {
                "RAISE FAME"
            },
        ),
        (
            118,
            if ui.fame_pending {
                "WAITING…"
            } else {
                "LOWER FAME"
            },
        ),
    ] {
        draw_guild_button(font, &mut pixels, WIDTH, HEIGHT, (x, 440, 92, 30), label);
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Level 15 · once per day",
        218,
        448,
        8.0,
        160,
        15,
        [90, 84, 78],
    );

    pixels
}

fn render_trade_room_panel(
    font: &fontdue::Font,
    ui: &TradeRoomUi,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<u8> {
    const WIDTH: usize = TRADE_PANEL_WIDTH as usize;
    const HEIGHT: usize = TRADE_PANEL_HEIGHT as usize;
    const CELL: usize = 42;
    const PARTNER_X: usize = 18;
    const OWN_X: usize = 174;
    const GRID_Y: usize = 136;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        HEIGHT,
        [22, 25, 31, 252],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        46,
        [75, 64, 91, 255],
    );
    for (x, y, width, height) in [
        (0, 0, WIDTH, 2),
        (0, HEIGHT - 2, WIDTH, 2),
        (0, 0, 2, HEIGHT),
        (WIDTH - 2, 0, 2, HEIGHT),
    ] {
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            y,
            width,
            height,
            [184, 163, 217, 255],
        );
    }
    let partner_name = ui
        .partner
        .as_ref()
        .map_or("Waiting for partner…", |partner| partner.name.as_str());
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &format!("Trade · {partner_name}"),
        18,
        11,
        18.0,
        610,
        24,
        [245, 233, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        680,
        8,
        28,
        28,
        [63, 43, 52, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "X",
        690,
        11,
        14.0,
        16,
        18,
        [255, 221, 222],
    );

    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "PARTNER OFFER",
        PARTNER_X as i32,
        86,
        10.0,
        140,
        16,
        [210, 188, 237],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "YOUR OFFER",
        OWN_X as i32,
        86,
        10.0,
        140,
        16,
        [178, 220, 188],
    );
    for (items, base_x, tint) in [
        (&ui.partner_items, PARTNER_X, [89, 58, 73, 255]),
        (&ui.own_items, OWN_X, [47, 76, 59, 255]),
    ] {
        for slot in 0..9 {
            let column = slot % 3;
            let row = slot / 3;
            let x = base_x + column * (CELL + 4);
            let y = GRID_Y + row * (CELL + 4);
            fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, x, y, CELL, CELL, tint);
            if let Some(item) = items.get(slot).and_then(Option::as_ref) {
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
                        x + 4,
                        y + 4,
                        34,
                    );
                }
                if item.quantity > 1 {
                    draw_dialogue_text(
                        font,
                        &mut pixels,
                        WIDTH,
                        HEIGHT,
                        &format!("x{}", item.quantity),
                        (x + 2) as i32,
                        (y + 29) as i32,
                        9.0,
                        38,
                        12,
                        [255, 255, 255],
                    );
                }
            }
        }
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &format!("Partner mesos: {}", ui.partner_meso),
        PARTNER_X as i32,
        284,
        10.0,
        145,
        16,
        [224, 215, 221],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &format!("Your mesos: {}", ui.own_meso),
        OWN_X as i32,
        284,
        10.0,
        145,
        16,
        [221, 232, 219],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        18,
        306,
        294,
        40,
        if ui.meso_focused {
            [46, 58, 57, 255]
        } else {
            [33, 38, 41, 255]
        },
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        if ui.meso_input.is_empty() {
            "Click here to enter your meso offer"
        } else {
            &ui.meso_input
        },
        30,
        319,
        11.0,
        270,
        16,
        [237, 235, 220],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        if ui.own_confirmed {
            "YOU CONFIRMED"
        } else {
            "NOT CONFIRMED"
        },
        OWN_X as i32,
        310,
        9.0,
        138,
        14,
        if ui.own_confirmed {
            [134, 231, 160]
        } else {
            [202, 183, 156]
        },
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        if ui.partner_confirmed {
            "PARTNER CONFIRMED"
        } else {
            "PARTNER NOT CONFIRMED"
        },
        PARTNER_X as i32,
        310,
        9.0,
        142,
        14,
        if ui.partner_confirmed {
            [134, 231, 160]
        } else {
            [202, 183, 156]
        },
    );

    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        330,
        76,
        374,
        422,
        [30, 33, 39, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "TRADE CHAT",
        346,
        86,
        11.0,
        336,
        16,
        [214, 198, 229],
    );
    let visible = ui.chat_lines.iter().rev().take(18).collect::<Vec<_>>();
    for (index, line) in visible.iter().rev().enumerate() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            line,
            346,
            112 + index as i32 * 19,
            10.0,
            340,
            17,
            [224, 224, 220],
        );
    }
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        330,
        512,
        292,
        36,
        if ui.chat_focused {
            [48, 55, 64, 255]
        } else {
            [38, 41, 47, 255]
        },
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        if ui.chat_input.is_empty() {
            "Click to write a message…"
        } else {
            &ui.chat_input
        },
        340,
        522,
        11.0,
        272,
        17,
        [243, 239, 228],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        638,
        512,
        66,
        36,
        [67, 57, 79, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "SEND",
        654,
        522,
        10.0,
        38,
        16,
        [240, 229, 255],
    );
    if let Some(notice) = ui.notice.as_deref() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            notice,
            18,
            472,
            10.0,
            685,
            18,
            [255, 197, 128],
        );
    }
    let buttons = [
        (
            18,
            552,
            114,
            if ui.own_confirmed {
                "WAITING…"
            } else {
                "CONFIRM"
            },
            if ui.own_confirmed {
                [48, 63, 51, 255]
            } else {
                [51, 89, 61, 255]
            },
        ),
        (140, 552, 114, "CANCEL", [88, 48, 51, 255]),
        (258, 552, 114, "OFFER MESO", [57, 61, 86, 255]),
    ];
    for (x, y, width, label, color) in buttons {
        fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, x, y, width, 34, color);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            (x + 9) as i32,
            (y + 9) as i32,
            10.0,
            width - 18,
            16,
            [243, 235, 255],
        );
    }
    if let Some(source) = ui.pending_item {
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            0,
            0,
            WIDTH,
            HEIGHT,
            [0, 0, 0, 128],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            206,
            216,
            310,
            170,
            [29, 31, 37, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "HOW MANY TO TRADE?",
            230,
            234,
            14.0,
            270,
            20,
            [243, 223, 177],
        );
        let name = item_name(source.item_id, string_archive, name_cache);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!("{} · max {}", name, source.max_quantity),
            230,
            269,
            11.0,
            270,
            17,
            [226, 223, 213],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            230,
            300,
            246,
            34,
            [44, 49, 58, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            if ui.quantity_input.is_empty() {
                "Enter a quantity"
            } else {
                &ui.quantity_input
            },
            242,
            309,
            12.0,
            224,
            18,
            [245, 241, 227],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            504,
            347,
            120,
            34,
            [57, 83, 61, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "TRADE",
            541,
            357,
            10.0,
            48,
            14,
            [237, 248, 230],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            632,
            347,
            72,
            34,
            [82, 48, 52, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "CANCEL",
            640,
            357,
            9.0,
            56,
            14,
            [255, 226, 225],
        );
    }
    pixels
}

fn render_hired_merchant_room_panel(
    font: &fontdue::Font,
    ui: &HiredMerchantRoomUi,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<u8> {
    const WIDTH: usize = HIRED_MERCHANT_PANEL_WIDTH as usize;
    const HEIGHT: usize = HIRED_MERCHANT_PANEL_HEIGHT as usize;
    const ROWS: usize = 8;
    const ROW_HEIGHT: usize = 42;
    const FIRST_ROW_Y: usize = 142;
    let room = &ui.state;
    let owner = room.my_slot == 0;
    let personal_shop = room.room_type == 4;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        HEIGHT,
        [24, 26, 30, 250],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        44,
        [83, 63, 40, 255],
    );
    for (x, y, width, height) in [
        (0, 0, WIDTH, 2),
        (0, HEIGHT - 2, WIDTH, 2),
        (0, 0, 2, HEIGHT),
        (WIDTH - 2, 0, 2, HEIGHT),
    ] {
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            y,
            width,
            height,
            [207, 176, 119, 255],
        );
    }
    let title = if personal_shop {
        format!("{}'s Personal Shop", room.owner_name)
    } else {
        format!("{}'s Hired Merchant", room.owner_name)
    };
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &title,
        18,
        9,
        19.0,
        500,
        26,
        [246, 226, 185],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        660,
        8,
        26,
        26,
        [65, 43, 37, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "X",
        669,
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
        &room.description,
        18,
        50,
        12.0,
        420,
        18,
        [223, 212, 188],
    );
    if !personal_shop {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!("{} meso", room.meso),
            520,
            50,
            12.0,
            145,
            18,
            [231, 225, 205],
        );
    }

    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        14,
        76,
        672,
        48,
        [34, 36, 40, 255],
    );
    let mut attendees = Vec::with_capacity(4);
    attendees.push((0, room.owner_name.as_str()));
    attendees.extend(
        room.visitors
            .iter()
            .map(|visitor| (visitor.slot, visitor.name.as_str())),
    );
    for (index, (slot, name)) in attendees.into_iter().enumerate() {
        let x = 22 + index * 164;
        let is_self = if owner {
            slot == 0
        } else {
            slot == room.my_slot as i8
        };
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            83,
            154,
            34,
            if is_self {
                [84, 68, 46, 255]
            } else {
                [43, 45, 49, 255]
            },
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            name,
            (x + 8) as i32,
            92,
            11.0,
            138,
            17,
            [238, 231, 212],
        );
    }

    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        if owner {
            "ITEMS · DOUBLE CLICK TO WITHDRAW"
        } else {
            "SHOP LISTINGS · SELECT TO BUY"
        },
        18,
        128,
        10.0,
        348,
        14,
        [222, 207, 177],
    );
    for row in 0..ROWS {
        let index = ui.item_scroll.saturating_add(row);
        let y = FIRST_ROW_Y + row * ROW_HEIGHT;
        let item = room.items.get(index);
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            14,
            y,
            354,
            ROW_HEIGHT - 2,
            if Some(index) == ui.selected_item {
                [78, 62, 42, 255]
            } else if item.is_some() {
                [34, 36, 40, 255]
            } else {
                [28, 29, 32, 255]
            },
        );
        if let Some(item) = item {
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
                    20,
                    y + 3,
                    34,
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
                60,
                (y + 4) as i32,
                11.0,
                292,
                15,
                [235, 232, 221],
            );
            let details = format!(
                "{} bundle(s) × {}  ·  {} meso{}",
                item.bundles.max(0),
                item.per_bundle.max(0),
                item.price,
                if item.sold_out { "  ·  SOLD OUT" } else { "" }
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &details,
                60,
                (y + 21) as i32,
                9.0,
                292,
                13,
                if item.sold_out {
                    [240, 143, 126]
                } else {
                    [202, 193, 171]
                },
            );
        }
    }

    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        382,
        128,
        304,
        370,
        [31, 33, 37, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "SHOP CHAT",
        394,
        134,
        10.0,
        275,
        15,
        [222, 207, 177],
    );
    let first_message = room.messages.len().saturating_sub(16);
    let mut message_y = 158;
    for message in room.messages.iter().skip(first_message) {
        if message_y > 476 {
            break;
        }
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &message.text,
            394,
            message_y,
            10.0,
            278,
            18,
            match message.speaker {
                1 => [137, 195, 230],
                2 => [162, 220, 154],
                3 => [245, 185, 105],
                _ => [220, 218, 209],
            },
        );
        message_y += 20;
    }
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        390,
        500,
        226,
        34,
        if ui.chat_focused {
            [57, 64, 73, 255]
        } else {
            [39, 41, 45, 255]
        },
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        if ui.chat_input.is_empty() {
            "Click to chat…"
        } else {
            &ui.chat_input
        },
        400,
        509,
        11.0,
        206,
        16,
        [241, 238, 225],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        622,
        500,
        64,
        34,
        [75, 56, 39, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "SEND",
        637,
        509,
        10.0,
        38,
        16,
        [255, 236, 192],
    );
    if let Some(notice) = ui.notice.as_deref() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            notice,
            18,
            532,
            10.0,
            650,
            15,
            [255, 190, 115],
        );
    }

    let personal_owner_buttons: &[(usize, usize, usize, &str, [u8; 4])] = &[
        (18, 552, 108, "START SHOP", [83, 67, 42, 255]),
        (134, 552, 100, "COLLECT", [63, 65, 58, 255]),
        (242, 552, 108, "EXIT ROOM", [63, 57, 48, 255]),
    ];
    let hired_owner_buttons: &[(usize, usize, usize, &str, [u8; 4])] = &[
        (
            18,
            552,
            108,
            if room.first_time {
                "START STORE"
            } else {
                "END SESSION"
            },
            [83, 67, 42, 255],
        ),
        (134, 552, 100, "ORGANIZE", [63, 65, 58, 255]),
        (242, 552, 100, "COLLECT", [63, 65, 58, 255]),
        (350, 552, 108, "EXIT", [63, 57, 48, 255]),
        (466, 552, 120, "CLOSE STORE", [83, 48, 42, 255]),
    ];
    let visitor_buttons: &[(usize, usize, usize, &str, [u8; 4])] = &[
        (18, 552, 116, "BUY SELECTED", [83, 67, 42, 255]),
        (142, 552, 116, "LEAVE SHOP", [63, 57, 48, 255]),
    ];
    let buttons = if owner && personal_shop {
        personal_owner_buttons
    } else if owner {
        hired_owner_buttons
    } else {
        visitor_buttons
    };
    for (x, y, width, label, color) in buttons {
        fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, *x, *y, *width, 32, *color);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            (*x + 8) as i32,
            (*y + 8) as i32,
            9.5,
            width.saturating_sub(16),
            14,
            [246, 231, 199],
        );
    }
    let scroll_notice = if room.items.len() > ROWS {
        format!(
            "Items {}–{} of {} · scroll the item list",
            ui.item_scroll + 1,
            (ui.item_scroll + ROWS).min(room.items.len()),
            room.items.len()
        )
    } else if owner && personal_shop {
        "Double-click to withdraw; drag an inventory item here to stock, then start the shop."
            .to_owned()
    } else if owner {
        "Double-click to withdraw; press I and drag inventory items here to stock.".to_owned()
    } else {
        "Select a listing and choose Buy Selected. Escape leaves the shop.".to_owned()
    };
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &scroll_notice,
        18,
        582,
        9.0,
        660,
        14,
        [180, 180, 174],
    );
    if let Some(source) = ui.stock_source {
        let name = item_name(source.item_id, string_archive, name_cache);
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            44,
            208,
            330,
            186,
            [20, 21, 24, 255],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            44,
            208,
            330,
            2,
            [230, 190, 112, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "STOCK ITEM",
            60,
            224,
            16.0,
            280,
            22,
            [246, 226, 185],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!("{} × {} per bundle", name, source.count.max(1)),
            60,
            257,
            11.0,
            296,
            16,
            [223, 218, 204],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "Price per bundle",
            60,
            285,
            10.0,
            132,
            15,
            [190, 188, 180],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            192,
            279,
            160,
            30,
            [42, 44, 48, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            if ui.stock_price.is_empty() {
                "Enter price"
            } else {
                &ui.stock_price
            },
            202,
            286,
            12.0,
            140,
            16,
            [245, 245, 239],
        );
        for (x, label) in [(60, "ADD TO STORE"), (222, "CANCEL")] {
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                x,
                331,
                130,
                34,
                if x == 60 {
                    [83, 67, 42, 255]
                } else {
                    [57, 57, 55, 255]
                },
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                label,
                (x + 10) as i32,
                341,
                10.0,
                110,
                15,
                [246, 231, 199],
            );
        }
    }
    pixels
}
