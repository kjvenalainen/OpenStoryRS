fn render_character_info_panel(
    font: &fontdue::Font,
    ui: &CharacterInfoUi,
    look: Option<&openstory_protocol::CharacterLook>,
    ui_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    string_archive: &NxArchive,
    name_cache: &mut HashMap<i32, Option<String>>,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
) -> Vec<u8> {
    const WIDTH: usize = CHARACTER_INFO_PANEL_WIDTH as usize;
    const HEIGHT: usize = CHARACTER_INFO_PANEL_HEIGHT as usize;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "UIWindow.img/UserInfo/backgrnd2",
        0,
        0,
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "UIWindow.img/UserInfo/backgrnd3",
        286,
        0,
    );
    for y in [165, 300] {
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            "Basic.img/Notice/backgrnd",
            286,
            y,
        );
    }
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "Basic.img/BtClose/normal/0",
        255,
        5,
    );
    if let Some(look) = look {
        let mut sprites = Vec::new();
        if append_character_preview(
            &mut sprites,
            character_archive,
            look,
            45.0,
            104.0,
            1.0,
            false,
            &mut MapArtCache::default(),
        )
        .is_ok()
        {
            let mut portrait = vec![0; 90 * 130 * 4];
            for sprite in sprites {
                composite_bgra_pixels(
                    &mut portrait,
                    90,
                    130,
                    &sprite.pixels,
                    sprite.texture_width as usize,
                    sprite.texture_height as usize,
                    sprite.rect.x.round() as i32,
                    sprite.rect.y.round() as i32,
                );
            }
            composite_bgra_pixels(&mut pixels, WIDTH, HEIGHT, &portrait, 90, 130, 6, 25);
        }
    }
    let info = &ui.info;
    let name = ui_foundation::ellipsize(font, &ui.name, 11.0, 154.0);
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &name,
        110,
        28,
        11.0,
        154,
        16,
        [255, 255, 255],
    );
    let marry_path = if info.married {
        "UIWindow.img/UserInfo/Marry/married/0"
    } else {
        "UIWindow.img/UserInfo/Marry/unmarried/0"
    };
    let _ = composite_nx_ui_bitmap(&mut pixels, WIDTH, HEIGHT, ui_archive, marry_path, 110, 45);
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        if info.married { "Married" } else { "Single" },
        134,
        47,
        9.0,
        100,
        14,
        [255, 255, 255],
    );
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
        let value = ui_foundation::ellipsize(font, value, 10.0, 110.0);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &value,
            154,
            75 + index as i32 * 18,
            10.0,
            110,
            15,
            [49, 45, 42],
        );
    }
    for index in 0..3 {
        let state = if index >= info.pets.len() {
            "disabled"
        } else if index == ui.selected_pet {
            "pressed"
        } else {
            "normal"
        };
        let path = format!("UIWindow.img/UserInfo/BtPet{}/{state}/0", index + 1);
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            &path,
            182 + index as i32 * 34,
            192,
        );
    }
    for (index, pet) in info.pets.iter().take(3).enumerate() {
        let name = if pet.name.is_empty() {
            item_name(pet.item_id, string_archive, name_cache)
        } else {
            pet.name.clone()
        };
        let name = ui_foundation::ellipsize(font, &name, 10.0, 105.0);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &name,
            140,
            220 + index as i32 * 38,
            10.0,
            105,
            15,
            [35, 35, 35],
        );
    }
    if let Some(pet) = info.pets.get(ui.selected_pet) {
        let mut portrait = vec![0; 90 * 100 * 4];
        for stance in ["stand0", "stand1"] {
            let path = format!("Pet/{}.img/{stance}/0", pet.item_id);
            if composite_nx_ui_bitmap(&mut portrait, 90, 100, item_archive, &path, 45, 82).is_ok() {
                break;
            }
        }
        composite_bgra_pixels(&mut pixels, WIDTH, HEIGHT, &portrait, 90, 100, 6, 214);
        let kind = item_name(pet.item_id, string_archive, name_cache);
        for (value, x, y, width) in [
            (
                ui_foundation::ellipsize(font, &kind, 9.0, 76.0),
                53,
                329,
                76,
            ),
            (pet.level.to_string(), 53, 347, 76),
            (pet.closeness.to_string(), 190, 329, 75),
            (pet.fullness.to_string(), 190, 347, 75),
        ] {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &value,
                x,
                y,
                9.0,
                width,
                14,
                [35, 35, 35],
            );
        }
    }
    for (row, item_id) in info
        .wishlist
        .iter()
        .skip(ui.wishlist_offset)
        .take(3)
        .enumerate()
    {
        let y = 30 + row * 40;
        if let Some((w, h, icon)) =
            inventory_item_icon(*item_id, item_archive, character_archive, icon_cache)
        {
            composite_inventory_icon(
                &mut pixels,
                WIDTH,
                HEIGHT,
                &icon,
                w as usize,
                h as usize,
                302,
                y,
                32,
            );
        }
        let name = item_name(*item_id, string_archive, name_cache);
        let name = ui_foundation::ellipsize(font, &name, 10.0, 155.0);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &name,
            338,
            y as i32 + 4,
            10.0,
            155,
            15,
            [35, 35, 35],
        );
    }
    if info.wishlist.len() > 3 {
        for (path, x) in [
            ("Basic.img/BtUP/normal/0", 488),
            ("Basic.img/BtDown/normal/0", 506),
        ] {
            let _ = composite_nx_ui_bitmap(&mut pixels, WIDTH, HEIGHT, ui_archive, path, x, 146);
        }
    }
    // The notice artwork has an inset button well; give the detail lines a
    // plain backing so text remains legible across that decoration.
    for (x, y, w, h) in [
        (312, 190, 214, 25),
        (312, 219, 214, 25),
        (312, 248, 214, 29),
        (312, 351, 120, 17),
        (312, 379, 120, 17),
        (312, 404, 214, 16),
    ] {
        fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, x, y, w, h, [245, 245, 245, 255]);
    }
    let mount = info.mount.map_or_else(
        || "Mount: none".to_owned(),
        |m| {
            format!(
                "Mount: Lv {} · EXP {} · tiredness {}",
                m.level, m.experience, m.tiredness
            )
        },
    );
    let book = format!(
        "Monster Book: Lv {} · {} cards ({}/{})",
        info.monster_book.level,
        info.monster_book.total_cards,
        info.monster_book.normal_cards,
        info.monster_book.special_cards
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &mount,
        316,
        193,
        10.0,
        208,
        25,
        [35, 35, 35],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &book,
        316,
        222,
        10.0,
        208,
        25,
        [35, 35, 35],
    );
    if let Some(medal) = &info.medal {
        let name = item_name(medal.item_id, string_archive, name_cache);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!("Medal: {name} · {} quests", medal.quest_ids.len()),
            316,
            251,
            10.0,
            208,
            28,
            [35, 35, 35],
        );
    }
    for (path, x, y) in [
        ("UIWindow.img/UserInfo/BtParty/normal/0", 306, 326),
        ("UIWindow.img/UserInfo/BtTrade/normal/0", 372, 326),
        ("Basic.img/BtOK/normal/0", 443, 348),
        (
            if ui.fame_pending {
                "Basic.img/BtUP/disabled/0"
            } else {
                "Basic.img/BtUP/normal/0"
            },
            443,
            382,
        ),
        (
            if ui.fame_pending {
                "Basic.img/BtDown/disabled/0"
            } else {
                "Basic.img/BtDown/normal/0"
            },
            461,
            382,
        ),
    ] {
        let _ = composite_nx_ui_bitmap(&mut pixels, WIDTH, HEIGHT, ui_archive, path, x, y);
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Guild invite",
        316,
        354,
        10.0,
        120,
        15,
        [35, 35, 35],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Change fame",
        316,
        382,
        10.0,
        120,
        15,
        [35, 35, 35],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Level 15 · once per day",
        316,
        407,
        9.0,
        208,
        14,
        [60, 60, 60],
    );
    pixels
}

// Coordinates are local to the trade texture and match the right-hand NX grid.
fn trade_own_grid_slot(x: f32, y: f32) -> Option<u8> {
    let x = x - 247.0;
    let y = y - 148.0;
    if !(0.0..114.0).contains(&x)
        || !(0.0..114.0).contains(&y)
        || x % 38.0 >= 36.0
        || y % 38.0 >= 36.0
    {
        return None;
    }
    Some((y / 38.0) as u8 * 3 + (x / 38.0) as u8)
}

fn render_trade_room_panel(
    font: &fontdue::Font,
    ui: &TradeRoomUi,
    ui_archive: &NxArchive,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<u8> {
    const WIDTH: usize = TRADE_PANEL_WIDTH as usize;
    const HEIGHT: usize = TRADE_PANEL_HEIGHT as usize;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "UIWindow.img/TradingRoom/backgrnd",
        78,
        20,
    );
    for (items, base_x) in [(&ui.partner_items, 84), (&ui.own_items, 247)] {
        for (slot, item) in items.iter().take(9).enumerate() {
            let Some(item) = item else { continue };
            let x = base_x + slot % 3 * 38;
            let y = 148 + slot / 3 * 38;
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
                    x + 2,
                    y + 2,
                    32,
                );
            }
            if item.quantity > 1 {
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    &item.quantity.to_string(),
                    x as i32 + 2,
                    y as i32 + 24,
                    9.0,
                    32,
                    12,
                    [35, 35, 35],
                );
            }
        }
    }
    let partner = ui
        .partner
        .as_ref()
        .map_or("Waiting...", |p| p.name.as_str());
    for (text, x) in [(partner, 99), ("Your offer", 264)] {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            text,
            x,
            128,
            10.0,
            82,
            15,
            [255, 255, 255],
        );
    }
    for (text, x) in [
        (ui.partner_meso.to_string(), 101),
        (
            if ui.meso_focused {
                ui.meso_input.clone()
            } else {
                ui.own_meso.to_string()
            },
            264,
        ),
    ] {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &text,
            x,
            265,
            10.0,
            91,
            14,
            [35, 35, 35],
        );
    }
    for (confirmed, x) in [(ui.partner_confirmed, 100), (ui.own_confirmed, 264)] {
        if confirmed {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                "Confirmed",
                x,
                91,
                10.0,
                90,
                14,
                [35, 110, 45],
            );
        }
    }
    for (index, line) in ui
        .chat_lines
        .iter()
        .rev()
        .take(11)
        .collect::<Vec<_>>()
        .iter()
        .rev()
        .enumerate()
    {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            line,
            375,
            46 + index as i32 * 18,
            10.0,
            235,
            17,
            [35, 35, 35],
        );
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &ui.chat_input,
        377,
        266,
        10.0,
        185,
        14,
        [35, 35, 35],
    );
    for (path, x, y) in [
        ("UIWindow.img/TradingRoom/BtCoin/normal/0", 247, 264),
        ("UIWindow.img/TradingRoom/BtTrade/normal/0", 385, 290),
        ("UIWindow.img/TradingRoom/BtCancel/normal/0", 451, 290),
    ] {
        let _ = composite_nx_ui_bitmap(&mut pixels, WIDTH, HEIGHT, ui_archive, path, x, y);
    }
    if let Some(notice) = ui.notice.as_deref() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            notice,
            375,
            238,
            10.0,
            235,
            18,
            [160, 55, 35],
        );
    }
    if let Some(source) = ui.pending_item {
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            "Basic.img/Notice/backgrnd",
            228,
            210,
        );
        let name = item_name(source.item_id, string_archive, name_cache);
        let prompt = ui
            .notice
            .clone()
            .unwrap_or_else(|| format!("How many {name}? (max {})", source.max_quantity));
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &prompt,
            247,
            235,
            11.0,
            226,
            28,
            [45, 45, 45],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            251,
            270,
            218,
            22,
            [255, 255, 255, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &ui.quantity_input,
            257,
            273,
            11.0,
            206,
            16,
            [35, 35, 35],
        );
        for (path, x) in [
            ("Basic.img/BtOK/normal/0", 284),
            ("Basic.img/BtCancel/normal/0", 357),
        ] {
            let _ = composite_nx_ui_bitmap(&mut pixels, WIDTH, HEIGHT, ui_archive, path, x, 309);
        }
    }
    pixels
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MerchantControl {
    Start,
    Arrange,
    Collect,
    Exit,
    Close,
    Buy,
}

fn merchant_controls(
    room: &openstory_protocol::HiredMerchantRoom,
) -> Vec<(MerchantControl, &'static str, usize, usize)> {
    let actions: &[(MerchantControl, &str)] = if room.my_slot != 0 {
        &[
            (
                MerchantControl::Buy,
                "UIWindow.img/PersonalShop/BtBuy/normal/0",
            ),
            (
                MerchantControl::Exit,
                "UIWindow.img/PersonalShop/BtExit/normal/0",
            ),
        ]
    } else if room.room_type == 4 {
        &[
            (
                MerchantControl::Start,
                "UIWindow.img/PersonalShop/BtStart/normal/0",
            ),
            (
                MerchantControl::Collect,
                "UIWindow.img/EntrustedShop/BtCoin/normal/0",
            ),
            (
                MerchantControl::Exit,
                "UIWindow.img/PersonalShop/BtExit/normal/0",
            ),
        ]
    } else if room.first_time {
        &[
            (
                MerchantControl::Start,
                "UIWindow.img/PersonalShop/BtStart/normal/0",
            ),
            (
                MerchantControl::Arrange,
                "UIWindow.img/EntrustedShop/BtArrange/normal/0",
            ),
            (
                MerchantControl::Collect,
                "UIWindow.img/EntrustedShop/BtCoin/normal/0",
            ),
            (
                MerchantControl::Exit,
                "UIWindow.img/PersonalShop/BtExit/normal/0",
            ),
        ]
    } else {
        &[
            (
                MerchantControl::Arrange,
                "UIWindow.img/EntrustedShop/BtArrange/normal/0",
            ),
            (
                MerchantControl::Collect,
                "UIWindow.img/EntrustedShop/BtCoin/normal/0",
            ),
            (
                MerchantControl::Exit,
                "UIWindow.img/PersonalShop/BtExit/normal/0",
            ),
            (
                MerchantControl::Close,
                "UIWindow.img/PersonalShop/BtClose/normal/0",
            ),
        ]
    };
    actions
        .iter()
        .enumerate()
        .map(|(i, (action, path))| (*action, *path, 330 + (i % 3) * 86, 320 + (i / 3) * 22))
        .collect()
}

fn merchant_listing_row(x: f32, y: f32) -> Option<usize> {
    if !(93.0..294.0).contains(&x) || !(177.0..377.0).contains(&y) {
        return None;
    }
    Some(((y - 177.0) / 40.0) as usize)
}

fn render_hired_merchant_room_panel(
    font: &fontdue::Font,
    ui: &HiredMerchantRoomUi,
    ui_archive: &NxArchive,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<u8> {
    const WIDTH: usize = HIRED_MERCHANT_PANEL_WIDTH as usize;
    const HEIGHT: usize = HIRED_MERCHANT_PANEL_HEIGHT as usize;
    let room = &ui.state;
    let mut pixels = vec![0; WIDTH * HEIGHT * 4];
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "UIWindow.img/PersonalShop/backgrnd",
        88,
        20,
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &room.description,
        106,
        27,
        11.0,
        482,
        16,
        [35, 35, 35],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &room.owner_name,
        100,
        158,
        10.0,
        82,
        15,
        [255, 255, 255],
    );
    for visitor in &room.visitors {
        if (1..=3).contains(&visitor.slot) {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &visitor.name,
                329 + (visitor.slot as i32 - 1) * 92,
                158,
                10.0,
                84,
                15,
                [255, 255, 255],
            );
        }
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &room.meso.to_string(),
        219,
        158,
        10.0,
        89,
        15,
        [35, 35, 35],
    );
    for row in 0..5 {
        let index = ui.item_scroll.saturating_add(row);
        let Some(item) = room.items.get(index) else {
            continue;
        };
        let y = 177 + row * 40;
        if ui.selected_item == Some(index) {
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                WIDTH,
                HEIGHT,
                ui_archive,
                "UIWindow.img/PersonalShop/select",
                129,
                y as i32,
            );
        }
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
                95,
                y + 2,
                32,
            );
        }
        let name = item_name(item.item_id, string_archive, name_cache);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!("{name} x{}", item.per_bundle.max(0)),
            132,
            y as i32 + 2,
            10.0,
            154,
            15,
            [35, 35, 35],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!("{} meso · {} left", item.price, item.bundles.max(0)),
            132,
            y as i32 + 19,
            9.0,
            154,
            14,
            [35, 65, 140],
        );
        if item.sold_out {
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                WIDTH,
                HEIGHT,
                ui_archive,
                "UIWindow.img/PersonalShop/soldout",
                93,
                y as i32,
            );
        }
    }
    for (i, message) in room
        .messages
        .iter()
        .rev()
        .take(7)
        .collect::<Vec<_>>()
        .iter()
        .rev()
        .enumerate()
    {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &message.text,
            330,
            181 + i as i32 * 18,
            10.0,
            255,
            17,
            [35, 35, 35],
        );
    }
    if let Some(notice) = ui.notice.as_deref() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            notice,
            330,
            302,
            10.0,
            255,
            16,
            [160, 55, 35],
        );
    }
    for (_, path, x, y) in merchant_controls(room) {
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            path,
            x as i32,
            y as i32,
        );
    }
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        329,
        365,
        202,
        18,
        [255, 255, 255, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &ui.chat_input,
        333,
        367,
        10.0,
        194,
        14,
        [35, 35, 35],
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "UIWindow.img/PersonalShop/BtEnter/normal/0",
        536,
        365,
    );
    if let Some(source) = ui.stock_source {
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            "Basic.img/Notice/backgrnd",
            218,
            214,
        );
        let name = item_name(source.item_id, string_archive, name_cache);
        let prompt = ui
            .notice
            .clone()
            .unwrap_or_else(|| format!("Price for {name} x{}?", source.count.max(1)));
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &prompt,
            237,
            239,
            11.0,
            226,
            28,
            [45, 45, 45],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            241,
            274,
            218,
            22,
            [255, 255, 255, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &ui.stock_price,
            247,
            277,
            11.0,
            206,
            16,
            [35, 35, 35],
        );
        for (path, x) in [
            ("Basic.img/BtOK/normal/0", 274),
            ("Basic.img/BtCancel/normal/0", 347),
        ] {
            let _ = composite_nx_ui_bitmap(&mut pixels, WIDTH, HEIGHT, ui_archive, path, x, 313);
        }
    }
    pixels
}

#[cfg(test)]
mod trade_grid_tests {
    use super::{merchant_listing_row, trade_own_grid_slot};

    #[test]
    fn merchant_rows_follow_the_five_original_artwork_rows() {
        assert_eq!(merchant_listing_row(93.0, 177.0), Some(0));
        assert_eq!(merchant_listing_row(293.0, 376.0), Some(4));
        assert_eq!(merchant_listing_row(93.0, 217.0), Some(1));
        assert_eq!(merchant_listing_row(92.0, 177.0), None);
        assert_eq!(merchant_listing_row(294.0, 177.0), None);
        assert_eq!(merchant_listing_row(93.0, 377.0), None);
    }

    #[test]
    fn trade_grid_hit_targets_match_cells_and_gaps() {
        assert_eq!(trade_own_grid_slot(247.0, 148.0), Some(0));
        assert_eq!(trade_own_grid_slot(358.0, 259.0), Some(8));
        assert_eq!(trade_own_grid_slot(283.0, 148.0), None);
        assert_eq!(trade_own_grid_slot(247.0, 184.0), None);
        assert_eq!(trade_own_grid_slot(361.0, 148.0), None);
    }
}
