fn map_string_category(map_id: i32) -> &'static str {
    if map_id < 100_000_000 {
        "maple"
    } else if map_id < 200_000_000 {
        "victoria"
    } else if map_id < 300_000_000 {
        "ossyria"
    } else if map_id < 540_000_000 {
        "elin"
    } else if map_id < 600_000_000 {
        "singapore"
    } else if map_id < 670_000_000 {
        "MasteriaGL"
    } else if map_id < 682_000_000 {
        let prefix3 = (map_id / 1_000_000) * 1_000_000;
        let prefix4 = (map_id / 100_000) * 100_000;
        if prefix3 == 674_000_000 || prefix4 == 680_100_000 || prefix4 == 889_100_000 {
            "etc"
        } else if prefix3 == 677_000_000 {
            "Episode1GL"
        } else {
            "weddingGL"
        }
    } else if map_id < 683_000_000 {
        "HalloweenGL"
    } else if map_id < 800_000_000 {
        "event"
    } else if map_id < 900_000_000 {
        "jp"
    } else {
        "etc"
    }
}

fn map_string_name(archive: &NxArchive, map_id: i32) -> Option<String> {
    let category = map_string_category(map_id);
    nx_string_value(archive, &format!("Map.img/{category}/{map_id}/mapName"))
}

fn map_string_street_name(archive: &NxArchive, map_id: i32) -> Option<String> {
    let category = map_string_category(map_id);
    nx_string_value(archive, &format!("Map.img/{category}/{map_id}/streetName"))
}

fn format_npc_dialogue_text(
    text: &str,
    player_name: &str,
    string_archive: &NxArchive,
    item_name_cache: &mut HashMap<i32, Option<String>>,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    skill_archive: &NxArchive,
    ui_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
) -> (String, NpcInlineIcons) {
    let normalized = text.replace("\\r\\n", "\n").replace("\\r", "\n");
    let characters = normalized.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(text.len());
    let mut item_macro_ids = Vec::new();
    let mut inline_icons = NpcInlineIcons::new();
    let mut icon_markers = HashMap::new();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if character == '\\' && characters.get(index + 1) == Some(&'n') {
            output.push('\n');
            index += 2;
            continue;
        }
        if character.is_control() && !matches!(character, '\n' | '\t') {
            index += 1;
            continue;
        }
        if character != '#' {
            output.push(character);
            index += 1;
            continue;
        }
        let Some(code) = characters.get(index + 1).copied() else {
            output.push('#');
            break;
        };
        if let Some(style) = match code {
            'b' => Some('\u{f0000}'),
            'k' => Some('\u{f0001}'),
            'r' => Some('\u{f0002}'),
            'd' => Some('\u{f0003}'),
            'e' => Some('\u{f0004}'),
            'n' => Some('\u{f0005}'),
            'g' => Some('\u{f0006}'),
            _ => None,
        } {
            output.push(style);
            index += 2;
            continue;
        }
        if code == 'c' {
            index += 2;
            continue;
        }
        if code == 'h' && characters.get(index + 2) == Some(&'#') {
            output.push_str(player_name);
            index += 3;
            continue;
        }
        if matches!(code, 'L' | 'l') {
            output.push('#');
            output.push(code);
            index += 2;
            continue;
        }
        if !code.is_ascii_alphabetic() {
            output.push('#');
            index += 1;
            continue;
        }
        let Some(end) = characters[index + 2..]
            .iter()
            .position(|value| *value == '#')
        else {
            output.push('#');
            index += 1;
            continue;
        };
        let end = index + 2 + end;
        let id = characters[index + 2..end].iter().collect::<String>();
        if !id.is_empty() && id.chars().all(|value| value.is_ascii_digit()) {
            if let Ok(id_number) = id.parse::<i32>() {
                match code {
                    'p' => output.push_str(
                        &nx_string_value(string_archive, &format!("Npc.img/{id}/name"))
                            .unwrap_or_else(|| format!("NPC {id}")),
                    ),
                    'm' => output.push_str(
                        &map_string_name(string_archive, id_number)
                            .unwrap_or_else(|| format!("Map {id}")),
                    ),
                    'o' => output.push_str(
                        &nx_string_value(string_archive, &format!("Mob.img/{id}/name"))
                            .unwrap_or_else(|| format!("Mob {id}")),
                    ),
                    't' | 'z' => {
                        item_macro_ids.push(id);
                        output.push_str(&item_name(id_number, string_archive, item_name_cache));
                    }
                    'v' | 'i' | 'q' | 's' | 'f' => {
                        let icon_kind = if code == 'i' { 'v' } else { code };
                        if matches!(code, 'v' | 'i') {
                            item_macro_ids.push(id);
                        }
                        if let Some(marker) = icon_markers.get(&(icon_kind, id_number)).copied() {
                            output.push(marker);
                        } else if inline_icons.len() < 0x18ff
                            && let Some(icon) = npc_dialogue_icon(
                                icon_kind,
                                id_number,
                                item_archive,
                                character_archive,
                                skill_archive,
                                ui_archive,
                                icon_cache,
                            )
                            && let Some(marker) = char::from_u32(0xe000 + inline_icons.len() as u32)
                        {
                            inline_icons.insert(marker, icon);
                            icon_markers.insert((icon_kind, id_number), marker);
                            output.push(marker);
                        }
                    }
                    _ => {}
                }
                index = end + 1;
                continue;
            }
        }
        output.push('#');
        index += 1;
    }

    for id in item_macro_ids.into_iter().filter(|id| id.len() >= 4) {
        let mut search_from = 0;
        while let Some(relative) = output.get(search_from..).and_then(|tail| tail.find(&id)) {
            let start = search_from + relative;
            let end = start + id.len();
            let left_is_digit = start > 0
                && output[..start]
                    .chars()
                    .next_back()
                    .is_some_and(|value| value.is_ascii_digit());
            let right_is_digit = output[end..]
                .chars()
                .next()
                .is_some_and(|value| value.is_ascii_digit());
            if left_is_digit || right_is_digit {
                search_from = end;
                continue;
            }
            let mut remove_start = start;
            let mut remove_end = end;
            if output[..start].ends_with(' ') {
                remove_start -= 1;
            } else if output[end..].starts_with(' ') {
                remove_end += 1;
            }
            output.replace_range(remove_start..remove_end, "");
            search_from = remove_start;
        }
    }
    (output, inline_icons)
}

fn npc_dialogue_icon(
    kind: char,
    id: i32,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    skill_archive: &NxArchive,
    ui_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
) -> Option<(u32, u32, Arc<[u8]>)> {
    if kind == 'v' {
        return inventory_item_icon(id, item_archive, character_archive, icon_cache);
    }
    let (archive, path) = match kind {
        'q' => (ui_archive, format!("UIWindow.img/QuestIcon/{id}")),
        's' => (
            skill_archive,
            format!("{:03}.img/skill/{id}/icon", id / 10_000),
        ),
        'f' => (
            character_archive,
            format!("Face/000{id:05}.img/default/face"),
        ),
        _ => return None,
    };
    let node = resolve_ui_path(&archive.root(), &path).ok().flatten()?;
    let art = first_bitmap_node(&node).ok().flatten()?;
    let bitmap = art.bitmap().ok().flatten()?;
    let (width, height) = (u32::from(bitmap.width()), u32::from(bitmap.height()));
    if width == 0 || height == 0 || width > 512 || height > 512 {
        return None;
    }
    Some((width, height, bitmap.decode_bgra_shared().ok()?))
}

fn parse_npc_selections(text: &str) -> (String, Vec<(i32, String)>) {
    let mut plain = String::new();
    let mut selections = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = text[cursor..].find("#L") {
        let start = cursor + relative_start;
        plain.push_str(&text[cursor..start]);
        let id_start = start + 2;
        let Some(id_end_relative) = text[id_start..].find('#') else {
            plain.push_str(&text[start..]);
            return (clean_npc_markup(&plain), selections);
        };
        let id_end = id_start + id_end_relative;
        let Ok(selection_id) = text[id_start..id_end].parse::<i32>() else {
            plain.push_str(&text[start..id_end + 1]);
            cursor = id_end + 1;
            continue;
        };
        let label_start = id_end + 1;
        // Several shipped island quiz pages omit #l, including Robin's last question.
        let rest = &text[label_start..];
        let terminator = rest.find("#l");
        let next_option = rest.find("#L");
        let label_length = [terminator, next_option]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(rest.len());
        let label_end = label_start + label_length;
        selections.push((
            selection_id,
            clean_npc_markup(&text[label_start..label_end])
                .trim()
                .to_owned(),
        ));
        cursor = label_end
            + if terminator == Some(label_length) {
                2
            } else {
                0
            };
    }
    plain.push_str(&text[cursor..]);
    (clean_npc_markup(&plain).trim().to_owned(), selections)
}

fn clean_npc_markup(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\\' && chars.peek() == Some(&'n') {
            chars.next();
            output.push('\n');
        } else if character == '#' {
            if let Some(next) = chars.next() {
                if next.is_ascii_alphabetic() {
                    if !matches!(next, 'b' | 'k' | 'r' | 'g' | 'e' | 'n' | 'h') {
                        while chars.next().is_some_and(|value| value != '#') {}
                    }
                } else {
                    output.push(character);
                    output.push(next);
                }
            }
        } else {
            output.push(character);
        }
    }
    output
}

fn npc_shop_sellable_items<'a>(
    inventory: &'a openstory_protocol::InventorySnapshot,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
) -> Vec<&'a openstory_protocol::InventoryItem> {
    inventory
        .items
        .iter()
        .filter(|item| {
            item.count > 0
                && item.item_id > 0
                && matches!(
                    item.inventory_type,
                    openstory_protocol::InventoryType::Equip
                        | openstory_protocol::InventoryType::Use
                        | openstory_protocol::InventoryType::Setup
                        | openstory_protocol::InventoryType::Etc
                )
                && !item.cash
                && !npc_shop_item_is_unsellable(item_archive, character_archive, item.item_id)
        })
        .collect()
}

fn npc_shop_item_info(
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    item_id: i32,
) -> Option<openstory_assets::NxNode> {
    let item_prefix = item_id / 10_000;
    let item_file = format!("0{item_id}.img");
    let item_node = format!("0{item_id}");
    let (archive, info_path) = match item_id / 1_000_000 {
        1 => {
            let category = match item_prefix {
                180..=183 => "PetEquip",
                190 | 191 => "TamingMob",
                _ => match item_prefix - 100 {
                    0 => "Cap",
                    1..=3 | 12..=14 => "Accessory",
                    4 => "Coat",
                    5 => "Longcoat",
                    6 => "Pants",
                    7 => "Shoes",
                    8 => "Glove",
                    9 => "Shield",
                    10 => "Cape",
                    11 => "Ring",
                    30..=70 => "Weapon",
                    _ => return None,
                },
            };
            (character_archive, format!("{category}/{item_file}/info"))
        }
        2 => (
            item_archive,
            format!("Consume/0{item_prefix}.img/{item_node}/info"),
        ),
        3 => (
            item_archive,
            format!("Install/0{item_prefix}.img/{item_node}/info"),
        ),
        4 => (
            item_archive,
            format!("Etc/0{item_prefix}.img/{item_node}/info"),
        ),
        _ => return None,
    };
    resolve_ui_path(&archive.root(), &info_path).ok().flatten()
}

fn npc_shop_item_is_unsellable(
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    item_id: i32,
) -> bool {
    npc_shop_item_info(item_archive, character_archive, item_id)
        .as_ref()
        .is_some_and(|info| item_flag_is_set(info, "notSale") || item_flag_is_set(info, "cash"))
}

fn merchant_stock_block_reason(
    item: &openstory_protocol::InventoryItem,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
) -> Option<&'static str> {
    if item.inventory_type == openstory_protocol::InventoryType::Cash || item.cash {
        return Some("Cash items cannot be listed in a player shop.");
    }
    let info = npc_shop_item_info(item_archive, character_archive, item.item_id)?;
    if item_flag_is_set(&info, "cash") {
        Some("Cash items cannot be listed in a player shop.")
    } else if item_flag_is_set(&info, "tradeBlock") {
        Some("This item cannot be traded or listed in a player shop.")
    } else {
        None
    }
}

fn item_flag_is_set(info: &openstory_assets::NxNode, name: &str) -> bool {
    integer_property(info, name).is_some_and(|value| value != 0)
}

fn npc_shop_item_price(
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    item_id: i32,
) -> Option<i64> {
    npc_shop_item_info(item_archive, character_archive, item_id)
        .as_ref()
        .and_then(|info| integer_property(info, "price"))
}

fn format_meso_amount(amount: i64) -> String {
    let digits = amount.max(0).to_string();
    let mut reversed = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            reversed.push(',');
        }
        reversed.push(digit);
    }
    format!("{} Mesos", reversed.chars().rev().collect::<String>())
}

const PERFECT_PITCH_ITEM_ID: i32 = 4_310_000;

fn inventory_item_count(
    inventory: &openstory_protocol::InventorySnapshot,
    inventory_type: openstory_protocol::InventoryType,
    item_id: i32,
) -> u32 {
    inventory
        .items
        .iter()
        .filter(|item| item.inventory_type == inventory_type && item.item_id == item_id)
        .map(|item| u32::from(item.count))
        .sum()
}

fn render_npc_shop_panel(
    font: &fontdue::Font,
    shop: &openstory_protocol::NpcShop,
    mode: NpcShopMode,
    selected: usize,
    first_visible: usize,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
    player_look: Option<&openstory_protocol::CharacterLook>,
    notice: Option<&str>,
    confirmation: Option<NpcShopConfirmation>,
    ui_archive: &NxArchive,
    npc_archive: &NxArchive,
    item_archive: &NxArchive,
    string_archive: &NxArchive,
    character_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> (Vec<u8>, SpriteRect) {
    const WIDTH: usize = NPC_SHOP_TEXTURE_WIDTH;
    const HEIGHT: usize = NPC_SHOP_TEXTURE_HEIGHT;
    const ITEM_ROWS_Y: usize = 173;
    const ITEM_ROW_STEP: usize = 54;
    const ITEM_ROW_HEIGHT: usize = 51;
    const ITEM_COLUMN_WIDTH: usize = 292;
    const LEFT_COLUMN_X: usize = 8;
    const RIGHT_COLUMN_X: usize = 316;
    let mut pixels = vec![0; WIDTH * HEIGHT * 4];
    if composite_nx_ui_bitmap_scaled_to(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "UIWindow.img/Shop/backgrnd",
        0,
        0,
        WIDTH,
        HEIGHT,
    )
    .is_err()
    {
        fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, 0, 0, WIDTH, HEIGHT, [230, 235, 238, 255]);
    }

    // This follows the v83 shop composition: the seller and customer share
    // the upper band, with the NPC stock and the player's inventory beneath.
    draw_npc_shopkeeper(&mut pixels, WIDTH, HEIGHT, npc_archive, shop.npc_id, 55, 68);
    if let Some(look) = player_look {
        let mut sprites = Vec::new();
        if append_character_preview(
            &mut sprites,
            character_archive,
            look,
            38.0,
            91.0,
            1.15,
            false,
            &mut MapArtCache::default(),
        )
        .is_ok()
        {
            let mut portrait = vec![0; 78 * 100 * 4];
            for sprite in sprites {
                composite_bgra_pixels(
                    &mut portrait,
                    78,
                    100,
                    &sprite.pixels,
                    sprite.texture_width as usize,
                    sprite.texture_height as usize,
                    sprite.rect.x.round() as i32,
                    sprite.rect.y.round() as i32,
                );
            }
            composite_bgra_pixels(&mut pixels, WIDTH, HEIGHT, &portrait, 78, 100, 348, 10);
        }
    }

    draw_npc_shop_tab(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        8,
        132,
        90,
        "All",
        true,
    );
    draw_npc_shop_tab(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        99,
        132,
        112,
        "Buy Back",
        false,
    );
    for (index, (label, tab_width)) in [
        ("Equip", 75),
        ("Use", 64),
        ("Set-up", 85),
        ("Etc.", 66),
    ]
    .into_iter()
    .enumerate()
    {
        draw_npc_shop_tab(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            RIGHT_COLUMN_X + [0, 76, 141, 227][index],
            132,
            tab_width,
            label,
            index == 0,
        );
    }

    for (path, x, y, width, height) in [
        (
            "UIWindow.img/Shop/BtExit/normal/0",
            202,
            18,
            92,
            23,
        ),
        (
            "UIWindow.img/Shop/BtBuy/normal/0",
            202,
            45,
            92,
            23,
        ),
        (
            "UIWindow.img/Shop/BtSell/normal/0",
            478,
            45,
            92,
            23,
        ),
    ] {
        let _ = composite_nx_ui_bitmap_scaled_to(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            path,
            x,
            y,
            width,
            height,
        );
    }
    if mode == NpcShopMode::Buy
        && shop
            .items
            .get(selected)
            .is_some_and(|item| item.rechargeable)
    {
        let _ = composite_nx_ui_bitmap_scaled_to(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            "UIWindow.img/Shop/BtRecharge/normal/0",
            296,
            45,
            52,
            23,
        );
    }

    if let Some(inventory) = inventory {
        let _ = composite_nx_ui_bitmap_scaled_to(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            "UIWindow.img/Shop/meso",
            456,
            91,
            17,
            17,
        );
        let balance = format_meso_amount(inventory.meso.max(0))
            .strip_suffix(" Mesos")
            .unwrap_or("0")
            .to_owned();
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &balance,
            477,
            89,
            15.0,
            135,
            20,
            [48, 48, 48],
        );
    }

    let sell_items = inventory
        .map(|i| npc_shop_sellable_items(i, item_archive, character_archive))
        .unwrap_or_default();
    for (column, column_mode) in [(0, NpcShopMode::Buy), (1, NpcShopMode::Sell)] {
        let x = if column == 0 {
            LEFT_COLUMN_X
        } else {
            RIGHT_COLUMN_X
        };
        let start = if mode == column_mode {
            first_visible
        } else {
            0
        };
        for row in 0..NPC_SHOP_VISIBLE_ROWS {
            let index = start + row;
            let data = if column_mode == NpcShopMode::Buy {
                shop.items.get(index).map(|item| {
                    let price = if item.price <= 0 && item.pitch > 0 {
                        format!(
                            "{} {}",
                            item.pitch,
                            item_name(PERFECT_PITCH_ITEM_ID, string_archive, name_cache)
                        )
                    } else if item.price == 0 && item.charge_price > 0 {
                        format_meso_amount(i64::from(item.charge_price))
                    } else if shop.currency_item_id > 0 {
                        format!(
                            "{} {}",
                            item.price.max(0),
                            item_name(shop.currency_item_id, string_archive, name_cache)
                        )
                    } else {
                        format_meso_amount(i64::from(item.price.max(0)))
                    };
                    (item.item_id, price)
                })
            } else {
                sell_items.get(index).map(|item| {
                    (
                        item.item_id,
                        npc_shop_item_price(item_archive, character_archive, item.item_id)
                            .map(|price| format_meso_amount(i64::from(price)))
                            .unwrap_or_else(|| "— mesos".to_owned()),
                    )
                })
            };
            let Some((item_id, price)) = data else {
                continue;
            };
            let y = ITEM_ROWS_Y + row * ITEM_ROW_STEP;
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                x,
                y,
                ITEM_COLUMN_WIDTH,
                ITEM_ROW_HEIGHT,
                if mode == column_mode && index == selected {
                    [229, 158, 45, 255]
                } else {
                    [202, 216, 226, 255]
                },
            );
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                x + 2,
                y + 2,
                ITEM_COLUMN_WIDTH - 4,
                1,
                [239, 245, 249, 255],
            );
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                x + 2,
                y + ITEM_ROW_HEIGHT - 2,
                ITEM_COLUMN_WIDTH - 4,
                1,
                [171, 191, 205, 255],
            );
            if mode == column_mode && index == selected {
                let _ = composite_nx_ui_bitmap_scaled_to(
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    ui_archive,
                    "UIWindow.img/Shop/select",
                    x,
                    y,
                    ITEM_COLUMN_WIDTH,
                    ITEM_ROW_HEIGHT,
                );
            }
            if let Some((w, h, icon)) =
                inventory_item_icon(item_id, item_archive, character_archive, icon_cache)
            {
                composite_inventory_icon(
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    &icon,
                    w as usize,
                    h as usize,
                    x + 5,
                    y + 4,
                    43,
                );
            }
            let name = item_name(item_id, string_archive, name_cache);
            let name = ui_foundation::ellipsize(font, &name, 15.0, 228.0);
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &name,
                (x + 52) as i32,
                (y + 4) as i32,
                15.0,
                228,
                19,
                [30, 30, 30],
            );
            let _ = composite_nx_ui_bitmap_scaled_to(
                &mut pixels,
                WIDTH,
                HEIGHT,
                ui_archive,
                "UIWindow.img/Shop/meso",
                x + 53,
                y + 29,
                15,
                15,
            );
            let details = ui_foundation::ellipsize(font, &price, 13.0, 208.0);
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &details,
                (x + 74) as i32,
                (y + 28) as i32,
                13.0,
                208,
                17,
                [60, 60, 60],
            );
        }
    }

    if let Some(confirmation) = confirmation {
        draw_npc_shop_modal(&mut pixels, WIDTH, HEIGHT, font, ui_archive, None);
        let question = if confirmation.recharge {
            "Recharge this item?"
        } else if confirmation.mode == NpcShopMode::Buy {
            "Buy this item?"
        } else {
            "Sell this item?"
        };
        draw_npc_shop_modal_text(font, &mut pixels, WIDTH, HEIGHT, question, 190, 219);
        let detail = format!(
            "{} ×{}",
            item_name(confirmation.item_id, string_archive, name_cache),
            confirmation.quantity
        );
        let detail = ui_foundation::ellipsize(font, &detail, 13.0, 270.0);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &detail,
            178,
            241,
            13.0,
            270,
            20,
            [242, 245, 250],
        );
        for (path, x) in [
            ("Basic.img/BtYes/normal/0", 195),
            ("Basic.img/BtNo/normal/0", 355),
        ] {
            let _ = composite_nx_ui_bitmap_scaled_to(
                &mut pixels,
                WIDTH,
                HEIGHT,
                ui_archive,
                path,
                x,
            298,
                66,
                24,
            );
        }
    } else if let Some(notice) = notice {
        draw_npc_shop_modal(&mut pixels, WIDTH, HEIGHT, font, ui_archive, Some(notice));
    }
    (
        pixels,
        SpriteRect {
            x: NPC_SHOP_PANEL_X,
            y: NPC_SHOP_PANEL_Y,
            width: NPC_SHOP_PANEL_WIDTH,
            height: NPC_SHOP_PANEL_HEIGHT,
            blend_mode: BlendMode::Alpha,
        },
    )
}

fn draw_npc_shopkeeper(
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
    let scale = (66.0 / source_width as f32)
        .min(88.0 / source_height as f32)
        .min(1.0);
    let draw_width = (source_width as f32 * scale).round().max(1.0) as usize;
    let draw_height = (source_height as f32 * scale).round().max(1.0) as usize;
    let left = anchor_x - (origin.0 as f32 * scale).round() as i32;
    let top = anchor_y - (origin.1 as f32 * scale).round() as i32;
    for y in 0..draw_height {
        let destination_y = top + y as i32;
        if !(0..canvas_height as i32).contains(&destination_y) {
            continue;
        }
        let source_y = (y as f32 / scale).floor() as usize;
        for x in 0..draw_width {
            let destination_x = left + x as i32;
            if !(0..canvas_width as i32).contains(&destination_x) {
                continue;
            }
            let source_x = (x as f32 / scale).floor() as usize;
            let source_index = (source_y * source_width + source_x) * 4;
            let destination_index =
                (destination_y as usize * canvas_width + destination_x as usize) * 4;
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

fn draw_npc_shop_tab(
    font: &fontdue::Font,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    ui_archive: &NxArchive,
    x: usize,
    y: usize,
    tab_width: usize,
    label: &str,
    selected: bool,
) {
    let tab_height = 33;
    let state = if selected { "1" } else { "0" };
    let _ = composite_nx_ui_bitmap_scaled_to(
        pixels,
        width,
        height,
        ui_archive,
        &format!("Basic.img/Tab2/left{state}"),
        x,
        y,
        6,
        tab_height,
    );
    let _ = composite_nx_ui_bitmap_scaled_to(
        pixels,
        width,
        height,
        ui_archive,
        &format!("Basic.img/Tab2/fill{state}"),
        x + 6,
        y,
        tab_width.saturating_sub(12),
        tab_height,
    );
    let _ = composite_nx_ui_bitmap_scaled_to(
        pixels,
        width,
        height,
        ui_archive,
        &format!("Basic.img/Tab2/right{state}"),
        x + tab_width.saturating_sub(6),
        y,
        6,
        tab_height,
    );
    let label_width = ui_foundation::measure_text(font, label, 14.0).round() as i32;
    draw_dialogue_text(
        font,
        pixels,
        width,
        height,
        label,
        x as i32 + (tab_width as i32 - label_width) / 2,
        y as i32 + 8,
        14.0,
        tab_width.saturating_sub(12),
        18,
        if selected { [255, 255, 255] } else { [55, 55, 55] },
    );
}

fn draw_npc_shop_modal(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    notice: Option<&str>,
) {
    for pixel in pixels.chunks_exact_mut(4) {
        pixel[0] = (u16::from(pixel[0]) * 175 / 255) as u8;
        pixel[1] = (u16::from(pixel[1]) * 175 / 255) as u8;
        pixel[2] = (u16::from(pixel[2]) * 175 / 255) as u8;
    }
    let _ = composite_nx_ui_bitmap_scaled_to(
        pixels,
        width,
        height,
        ui_archive,
        "UIWindow.img/Notice/0",
        133,
        180,
        354,
        156,
    );
    fill_dialogue_rect(
        pixels,
        width,
        height,
        143,
        194,
        334,
        94,
        [76, 145, 187, 255],
    );
    if let Some(notice) = notice {
        draw_npc_shop_modal_text(font, pixels, width, height, notice, 190, 219);
        let _ = composite_nx_ui_bitmap_scaled_to(
            pixels,
            width,
            height,
            ui_archive,
            "Basic.img/BtOK2/normal/0",
            414,
            298,
            60,
            24,
        );
    }
}

fn draw_npc_shop_modal_text(
    font: &fontdue::Font,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    message: &str,
    x: usize,
    y: usize,
) {
    let wrapped = wrap_dialogue_text(font, message, 16.0, 240, &EMPTY_NPC_INLINE_ICONS);
    let mut line_y = y as i32;
    for line in wrapped.lines() {
        let line_width = ui_foundation::measure_text(font, line, 16.0).round() as usize;
        let line_x = x + (240_usize.saturating_sub(line_width) / 2);
        draw_dialogue_text(
            font,
            pixels,
            width,
            height,
            line,
            line_x as i32,
            line_y,
            16.0,
            240,
            22,
            [245, 247, 250],
        );
        line_y += 22;
    }
}

// All NPC dialogue coordinates are NX-local: bitmap anchors, text and hitboxes
// share one 529x260 panel coordinate space. The panel itself is centered in the
// 1024x768 logical canvas (render scaling is applied later by the UI renderer).
struct NpcDialogueLayout;

impl NpcDialogueLayout {
    const WIDTH: usize = 529;
    const HEIGHT: usize = 260;
    const TOP_HEIGHT: usize = 28;
    const BOTTOM_Y: usize = 202;
    const BODY_HEIGHT: usize = Self::BOTTOM_Y - Self::TOP_HEIGHT;
    const PANEL_X: f32 = (1024.0 - Self::WIDTH as f32) / 2.0;
    const PANEL_Y: f32 = (768.0 - Self::HEIGHT as f32) / 2.0;

    const NPC_BAR_X: i32 = 22;
    const NPC_BAR_Y: i32 = 141;
    const NPC_BAR_WIDTH: i32 = 121;
    const NPC_CENTER_X: i32 = Self::NPC_BAR_X + Self::NPC_BAR_WIDTH / 2;
    const TEXT_X: i32 = 166;
    const TEXT_Y: i32 = 48;
    const TEXT_WIDTH: usize = 320;
    const SCROLL_X: usize = 490;
    const INPUT_X: i32 = Self::TEXT_X + 8;
    const INPUT_Y: i32 = 177;
    const INPUT_HEIGHT: usize = 20;
    const SELECTION_ROW_STEP: i32 = 25;
    const SELECTION_HEIGHT: usize = 23;
    const ADJUST_MINUS_X: i32 = 328;
    const ADJUST_PLUS_X: i32 = 365;
    const CLOSE_X: i32 = 9;
    const YES_X: i32 = 400;
    const NO_X: i32 = 465;
    const PREV_X: i32 = 389;
    const NEXT_X: i32 = 471;
    const OK_X: i32 = 471;
    const BUTTON_Y: i32 = 236;
}

fn append_npc_dialogue_input(input: &mut String, message_type: i8, text: &str) {
    let limit = match message_type {
        2 => 50,
        3 => 16,
        _ => return,
    };
    for character in text.chars() {
        if input.chars().count() >= limit {
            break;
        }
        let accepted = if message_type == 2 {
            !character.is_control()
        } else {
            character.is_ascii_digit() || (character == '-' && input.is_empty())
        };
        if accepted {
            input.push(character);
        }
    }
}

fn npc_dialogue_number(input: &str, bounds: Option<(i32, i32, i32)>) -> i32 {
    let value = input
        .parse::<i64>()
        .unwrap_or_else(|_| i64::from(bounds.map_or(0, |bounds| bounds.0)))
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    match bounds {
        Some((_, minimum, maximum)) if minimum <= maximum => value.clamp(minimum, maximum),
        _ => value,
    }
}

fn npc_dialogue_cancel_response(message_type: i8) -> i8 {
    match message_type {
        0 | 1 | 5 | 6 => -1,
        _ => 0,
    }
}

fn npc_dialogue_text_view_height(message_type: i8) -> usize {
    match message_type {
        2 | 3 => 112,
        4 => 106,
        _ => NpcDialogueLayout::BOTTOM_Y - NpcDialogueLayout::TEXT_Y as usize,
    }
}

#[cfg(test)]
mod npc_dialogue_viewport_tests {
    use super::*;

    #[test]
    fn long_dialogue_and_scrolled_text_stay_above_the_footer() {
        let font = fontdue::Font::from_bytes(include_bytes!("../../assets/fonts/Roboto-Regular.ttf").as_slice(), fontdue::FontSettings::default()).unwrap();
        let text = "A long line of dialogue that must remain inside the text viewport. ".repeat(30);
        let icons = NpcInlineIcons::new();
        let view_height = npc_dialogue_text_view_height(0);
        let size = ui_foundation::UiTextRole::Body.style().size;
        let content_height = dialogue_text_content_height(&font, &text, size, NpcDialogueLayout::TEXT_WIDTH, &icons);
        assert!(content_height > view_height);
        for offset in [0, content_height - view_height] {
            let mut pixels = vec![0; NpcDialogueLayout::WIDTH * NpcDialogueLayout::HEIGHT * 4];
            draw_dialogue_text_with_icons_scrolled(&font, &mut pixels, NpcDialogueLayout::WIDTH, NpcDialogueLayout::HEIGHT, &text, NpcDialogueLayout::TEXT_X, NpcDialogueLayout::TEXT_Y, size, NpcDialogueLayout::TEXT_WIDTH, view_height, [0, 0, 0], &icons, offset);
            assert!(pixels.iter().any(|&pixel| pixel != 0));
            assert!(pixels[NpcDialogueLayout::BOTTOM_Y * NpcDialogueLayout::WIDTH * 4..].iter().all(|&pixel| pixel == 0));
        }
    }
}

fn dialogue_text_content_height(
    font: &fontdue::Font,
    text: &str,
    size: f32,
    max_width: usize,
    inline_icons: &NpcInlineIcons,
) -> usize {
    let line_height = size.ceil() as usize + 4;
    let mut line_y = 0_usize;
    let mut pen_x = 0.0_f32;
    let mut previous = None;
    let mut bold = false;
    let mut has_content = false;
    let wrapped = wrap_dialogue_text(font, text, size, max_width, inline_icons);
    for character in wrapped.chars() {
        if character == '\u{f0004}' {
            bold = true;
            previous = None;
            continue;
        }
        if character == '\u{f0005}' {
            bold = false;
            previous = None;
            continue;
        }
        if (0xf0000..=0xf0006).contains(&(character as u32)) {
            continue;
        }
        let font = if bold {
            ui_foundation::bold_font().unwrap_or(font)
        } else {
            font
        };
        has_content = true;
        if character == '\n' {
            line_y = line_y.saturating_add(line_height);
            pen_x = 0.0;
            previous = None;
            continue;
        }
        if inline_icons.contains_key(&character) {
            let icon_size = (size.ceil() as usize + 4).clamp(16, 32);
            if pen_x + icon_size as f32 > max_width as f32 {
                line_y = line_y.saturating_add(line_height);
                pen_x = 0.0;
            }
            pen_x += icon_size as f32;
            previous = None;
            continue;
        }
        if let Some(previous) = previous {
            pen_x += font
                .horizontal_kern(previous, character, size)
                .unwrap_or(0.0);
        }
        let advance_width = font.metrics(character, size).advance_width;
        if pen_x + advance_width > max_width as f32 {
            line_y = line_y.saturating_add(line_height);
            pen_x = 0.0;
        }
        pen_x += advance_width;
        previous = Some(character);
    }
    if has_content {
        line_y.saturating_add(line_height)
    } else {
        0
    }
}

fn render_npc_dialogue(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    npc_archive: &NxArchive,
    string_archive: &NxArchive,
    npc_id: i32,
    speaker: i8,
    message_type: i8,
    navigation: Option<(bool, bool)>,
    text: &str,
    text_scroll: usize,
    inline_icons: &NpcInlineIcons,
    selections: &[(i32, String)],
    selected_selection: usize,
    first_visible_selection: usize,
    hovered_selection: Option<usize>,
    input: &str,
) -> (Vec<u8>, SpriteRect, Vec<NpcDialogueButton>, usize) {
    let (width, height) = (NpcDialogueLayout::WIDTH, NpcDialogueLayout::HEIGHT);
    let (panel_x, panel_y) = (NpcDialogueLayout::PANEL_X, NpcDialogueLayout::PANEL_Y);
    let mut pixels = vec![0_u8; width * height * 4];
    let nx_frame = composite_npc_dialogue_frame(&mut pixels, width, height, ui_archive);
    if !nx_frame {
        // Keep dialogue usable with incomplete custom UI archives.
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            0,
            0,
            width,
            height,
            [26, 22, 18, 235],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            0,
            0,
            width,
            36,
            [92, 70, 42, 255],
        );
    }
    if speaker == 0 {
        let mut npc_path = format!("{:07}.img", npc_id.max(0));
        for _ in 0..8 {
            let link = npc_archive
                .root()
                .resolve(&format!("{npc_path}/info/link"))
                .ok()
                .flatten()
                .and_then(|node| match node.value().ok()? {
                    NodeValue::String(link) => Some(link),
                    _ => None,
                });
            let Some(link) = link else {
                break;
            };
            npc_path = format!("{link}.img");
        }
        let npc_path = format!("{npc_path}/stand/0");
        let name = string_archive
            .root()
            .resolve(&format!("Npc.img/{npc_id}/name"))
            .ok()
            .flatten()
            .and_then(|node| match node.value().ok()? {
                NodeValue::String(name) if !name.trim().is_empty() => Some(name),
                _ => None,
            })
            .unwrap_or_else(|| format!("NPC {npc_id}"));
        if let Some(npc) = npc_archive.root().resolve(&npc_path).ok().flatten() {
            let _ = composite_nx_node_bitmap(
                &mut pixels,
                width,
                height,
                &npc,
                NpcDialogueLayout::NPC_CENTER_X,
                NpcDialogueLayout::NPC_BAR_Y,
            );
        }
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            width,
            height,
            ui_archive,
            "UIWindow.img/UtilDlgEx/bar",
            NpcDialogueLayout::NPC_BAR_X,
            NpcDialogueLayout::NPC_BAR_Y,
        );
        let name_style = ui_foundation::UiTextRole::Label.style();
        let name_width = (dialogue_text_width(font, &name, name_style.size).ceil() as i32) + 1;
        ui_foundation::draw_text_role(
            font,
            &mut pixels,
            width,
            height,
            &name,
            NpcDialogueLayout::NPC_BAR_X + (NpcDialogueLayout::NPC_BAR_WIDTH - name_width) / 2,
            NpcDialogueLayout::NPC_BAR_Y + 2,
            ui_foundation::UiTextRole::Label,
            NpcDialogueLayout::NPC_BAR_WIDTH as usize,
            17,
            Some([255, 255, 255]),
        );
    }
    let body_style = ui_foundation::UiTextRole::Body.style();
    let text_content_height = if text.trim().is_empty() {
        0
    } else {
        dialogue_text_content_height(
            font,
            text,
            body_style.size,
            NpcDialogueLayout::TEXT_WIDTH,
            inline_icons,
        )
    };
    let text_view_height = if message_type == 4 {
        if text.trim().is_empty() {
            0
        } else {
            text_content_height.min(npc_dialogue_text_view_height(4))
        }
    } else {
        npc_dialogue_text_view_height(message_type)
    };
    let text_max_scroll = text_content_height.saturating_sub(text_view_height);
    let text_scroll = text_scroll.min(text_max_scroll);
    draw_dialogue_text_with_icons_scrolled(
        font,
        &mut pixels,
        width,
        height,
        text,
        NpcDialogueLayout::TEXT_X,
        NpcDialogueLayout::TEXT_Y,
        body_style.size,
        NpcDialogueLayout::TEXT_WIDTH,
        text_view_height,
        [46, 46, 46],
        inline_icons,
        text_scroll,
    );
    ui_foundation::draw_scrollbar(
        &mut pixels,
        width,
        height,
        ui_archive,
        NpcDialogueLayout::SCROLL_X as i32,
        NpcDialogueLayout::TEXT_Y,
        text_view_height,
        ui_foundation::scroll_fraction(text_scroll, text_content_height, text_view_height),
    );

    let mut buttons = Vec::new();
    if message_type == 4 {
        for (row, (index, (selection, label))) in selections
            .iter()
            .enumerate()
            .skip(first_visible_selection)
            .take(5)
            .enumerate()
        {
            let local_y = NpcDialogueLayout::TEXT_Y
                + if text_view_height == 0 {
                    0
                } else {
                    text_view_height as i32 + 8
                }
                + row as i32 * NpcDialogueLayout::SELECTION_ROW_STEP;
            let body_size = body_style.size;
            let label_width = dialogue_text_width(font, label, body_size)
                .ceil()
                .clamp(12.0, (NpcDialogueLayout::TEXT_WIDTH - 24) as f32);
            let label_x = NpcDialogueLayout::TEXT_X + 24;
            let rect = SpriteRect {
                x: panel_x + (label_x - 14) as f32,
                y: panel_y + local_y as f32,
                width: (label_width + 20.0).min(NpcDialogueLayout::TEXT_WIDTH as f32 - 8.0),
                height: NpcDialogueLayout::SELECTION_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            };
            let hovered = hovered_selection == Some(index);
            let selected = index == selected_selection;
            let marker = if selected || hovered { "dot1" } else { "dot0" };
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                width,
                height,
                ui_archive,
                &format!("UIWindow.img/UtilDlgEx/{marker}"),
                NpcDialogueLayout::TEXT_X + 10,
                local_y + 7,
            );
            let link_color = if hovered {
                [132, 71, 168]
            } else {
                [108, 61, 145]
            };
            draw_dialogue_text_with_icons(
                font,
                &mut pixels,
                width,
                height,
                label,
                label_x,
                local_y + 2,
                body_size,
                NpcDialogueLayout::TEXT_WIDTH - 26,
                NpcDialogueLayout::SELECTION_HEIGHT,
                link_color,
                inline_icons,
            );
            let underline_width = label_width.ceil() as usize;
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                label_x as usize,
                (local_y + 20) as usize,
                underline_width,
                if hovered { 2 } else { 1 },
                [link_color[0], link_color[1], link_color[2], 255],
            );
            buttons.push(NpcDialogueButton {
                rect,
                action: NpcDialogueAction::Select(*selection),
            });
        }
        push_nx_dialogue_button(
            &mut pixels,
            &mut buttons,
            ui_archive,
            width,
            height,
            "UIWindow.img/UtilDlgEx/BtClose/normal/0",
            NpcDialogueLayout::CLOSE_X,
            NpcDialogueLayout::BUTTON_Y,
            NpcDialogueAction::More(0),
        );
    } else {
        if message_type == 2 || message_type == 3 {
            let mut visible = input;
            while dialogue_text_width(font, visible, 16.0) > 287.0 {
                let Some((index, _)) = visible.char_indices().nth(1) else {
                    break;
                };
                visible = &visible[index..];
            }
            let visible_with_caret = format!("{visible}|");
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &visible_with_caret,
                NpcDialogueLayout::INPUT_X,
                NpcDialogueLayout::INPUT_Y,
                16.0,
                NpcDialogueLayout::TEXT_WIDTH - 16,
                NpcDialogueLayout::INPUT_HEIGHT,
                [46, 46, 46],
            );
        }
        push_nx_dialogue_button(
            &mut pixels,
            &mut buttons,
            ui_archive,
            width,
            height,
            "UIWindow.img/UtilDlgEx/BtClose/normal/0",
            NpcDialogueLayout::CLOSE_X,
            NpcDialogueLayout::BUTTON_Y,
            NpcDialogueAction::More(npc_dialogue_cancel_response(message_type)),
        );
        match message_type {
            1 => {
                push_nx_yes_no_buttons(&mut pixels, &mut buttons, ui_archive, width, height);
            }
            2 => push_nx_dialogue_button(
                &mut pixels,
                &mut buttons,
                ui_archive,
                width,
                height,
                "UIWindow.img/UtilDlgEx/BtOK/normal/0",
                NpcDialogueLayout::OK_X,
                NpcDialogueLayout::BUTTON_Y,
                NpcDialogueAction::SubmitText,
            ),
            3 => {
                for (x, path, delta) in [
                    (
                        NpcDialogueLayout::ADJUST_MINUS_X,
                        "Basic.img/BtMin/normal/0",
                        -1,
                    ),
                    (
                        NpcDialogueLayout::ADJUST_PLUS_X,
                        "Basic.img/BtMax/normal/0",
                        1,
                    ),
                ] {
                    push_nx_dialogue_button(
                        &mut pixels,
                        &mut buttons,
                        ui_archive,
                        width,
                        height,
                        path,
                        x + 10,
                        NpcDialogueLayout::BUTTON_Y + 3,
                        NpcDialogueAction::AdjustNumber(delta),
                    );
                }
                push_nx_dialogue_button(
                    &mut pixels,
                    &mut buttons,
                    ui_archive,
                    width,
                    height,
                    "UIWindow.img/UtilDlgEx/BtOK/normal/0",
                    NpcDialogueLayout::OK_X,
                    NpcDialogueLayout::BUTTON_Y,
                    NpcDialogueAction::SubmitNumber,
                );
            }
            5 => push_nx_dialogue_button(
                &mut pixels,
                &mut buttons,
                ui_archive,
                width,
                height,
                "UIWindow.img/UtilDlgEx/BtNext/normal/0",
                NpcDialogueLayout::NEXT_X,
                NpcDialogueLayout::BUTTON_Y,
                NpcDialogueAction::More(1),
            ),
            6 => {
                push_nx_dialogue_button(
                    &mut pixels,
                    &mut buttons,
                    ui_archive,
                    width,
                    height,
                    "UIWindow.img/UtilDlgEx/BtPrev/normal/0",
                    NpcDialogueLayout::PREV_X,
                    NpcDialogueLayout::BUTTON_Y,
                    NpcDialogueAction::More(0),
                );
                push_nx_dialogue_button(
                    &mut pixels,
                    &mut buttons,
                    ui_archive,
                    width,
                    height,
                    "UIWindow.img/UtilDlgEx/BtNext/normal/0",
                    NpcDialogueLayout::NEXT_X,
                    NpcDialogueLayout::BUTTON_Y,
                    NpcDialogueAction::More(1),
                );
            }
            12 => {
                for (path, x, response) in [
                    (
                        "UIWindow.img/UtilDlgEx/BtQYes/normal/0",
                        NpcDialogueLayout::YES_X,
                        1,
                    ),
                    (
                        "UIWindow.img/UtilDlgEx/BtQNo/normal/0",
                        NpcDialogueLayout::NO_X,
                        0,
                    ),
                ] {
                    push_nx_dialogue_button(
                        &mut pixels,
                        &mut buttons,
                        ui_archive,
                        width,
                        height,
                        path,
                        x,
                        NpcDialogueLayout::BUTTON_Y,
                        NpcDialogueAction::More(response),
                    );
                }
            }
            0 if navigation.is_some_and(|(previous, next)| previous || next) => {
                let (previous, next) = navigation.unwrap_or_default();
                for (visible, path, x, response) in [
                    (
                        previous,
                        "UIWindow.img/UtilDlgEx/BtPrev/normal/0",
                        NpcDialogueLayout::PREV_X,
                        0,
                    ),
                    (
                        next,
                        "UIWindow.img/UtilDlgEx/BtNext/normal/0",
                        NpcDialogueLayout::NEXT_X,
                        1,
                    ),
                ] {
                    if visible {
                        push_nx_dialogue_button(
                            &mut pixels,
                            &mut buttons,
                            ui_archive,
                            width,
                            height,
                            path,
                            x,
                            NpcDialogueLayout::BUTTON_Y,
                            NpcDialogueAction::More(response),
                        );
                    }
                }
            }
            _ => push_nx_dialogue_button(
                &mut pixels,
                &mut buttons,
                ui_archive,
                width,
                height,
                "UIWindow.img/UtilDlgEx/BtOK/normal/0",
                NpcDialogueLayout::OK_X,
                NpcDialogueLayout::BUTTON_Y,
                NpcDialogueAction::More(1),
            ),
        }
    }
    (
        pixels,
        SpriteRect {
            x: panel_x,
            y: panel_y,
            width: width as f32,
            height: height as f32,
            blend_mode: BlendMode::Alpha,
        },
        buttons,
        text_max_scroll,
    )
}

fn push_nx_yes_no_buttons(
    pixels: &mut [u8],
    buttons: &mut Vec<NpcDialogueButton>,
    archive: &NxArchive,
    width: usize,
    height: usize,
) {
    push_nx_dialogue_button(
        pixels,
        buttons,
        archive,
        width,
        height,
        "UIWindow.img/UtilDlgEx/BtYes/normal/0",
        NpcDialogueLayout::YES_X,
        NpcDialogueLayout::BUTTON_Y,
        NpcDialogueAction::More(1),
    );
    push_nx_dialogue_button(
        pixels,
        buttons,
        archive,
        width,
        height,
        "UIWindow.img/UtilDlgEx/BtNo/normal/0",
        NpcDialogueLayout::NO_X,
        NpcDialogueLayout::BUTTON_Y,
        NpcDialogueAction::More(0),
    );
}

fn push_nx_dialogue_button(
    pixels: &mut [u8],
    buttons: &mut Vec<NpcDialogueButton>,
    archive: &NxArchive,
    width: usize,
    height: usize,
    path: &str,
    x: i32,
    y: i32,
    action: NpcDialogueAction,
) {
    let (button_width, button_height, origin_x, origin_y) =
        nx_dialogue_button_geometry(archive, path).unwrap_or((46.0, 18.0, 0, 0));
    let rect = SpriteRect {
        x: NpcDialogueLayout::PANEL_X + x as f32 - origin_x as f32,
        y: NpcDialogueLayout::PANEL_Y + y as f32 - origin_y as f32,
        width: button_width,
        height: button_height,
        blend_mode: BlendMode::Alpha,
    };
    let _ = composite_nx_ui_bitmap(pixels, width, height, archive, path, x, y);
    buttons.push(NpcDialogueButton { rect, action });
}

fn nx_dialogue_button_geometry(archive: &NxArchive, path: &str) -> Option<(f32, f32, i32, i32)> {
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
        f32::from(bitmap.width()),
        f32::from(bitmap.height()),
        origin.0,
        origin.1,
    ))
}

fn composite_npc_dialogue_frame(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    ui_archive: &NxArchive,
) -> bool {
    let mut frame = vec![0_u8; pixels.len()];
    let top = "UIWindow.img/UtilDlgEx/t";
    let fill = "UIWindow.img/UtilDlgEx/c";
    let bottom = "UIWindow.img/UtilDlgEx/s";
    let result =
        composite_nx_ui_bitmap(&mut frame, width, height, ui_archive, top, 0, 0).and_then(|_| {
            let node = resolve_ui_path(&ui_archive.root(), fill)?
                .ok_or_else(|| format!("missing v83 UI asset {fill:?}"))?;
            let bitmap = node
                .bitmap()
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("v83 UI asset {fill:?} is not a bitmap"))?;
            let step = usize::from(bitmap.height()).max(1);
            let body_end = NpcDialogueLayout::TOP_HEIGHT + NpcDialogueLayout::BODY_HEIGHT;
            for y in (NpcDialogueLayout::TOP_HEIGHT..body_end).step_by(step) {
                composite_nx_ui_bitmap(&mut frame, width, height, ui_archive, fill, 0, y as i32)?;
            }
            composite_nx_ui_bitmap(
                &mut frame,
                width,
                height,
                ui_archive,
                bottom,
                0,
                NpcDialogueLayout::BOTTOM_Y as i32,
            )
        });
    if result.is_ok() {
        pixels.copy_from_slice(&frame);
        true
    } else {
        false
    }
}

pub(super) fn render_npc_dialogue_preview(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    npc_archive: &NxArchive,
    string_archive: &NxArchive,
) -> (Vec<u8>, (f32, f32, f32, f32)) {
    let (pixels, panel, _, _) = render_npc_dialogue(
        font,
        ui_archive,
        npc_archive,
        string_archive,
        ui_preview::fixture_named_id(string_archive, "Npc.img", "Robin", 2007),
        0,
        0,
        None,
        "\u{f0004}Ascension Powers\u{f0005}\n\n\u{f0000}Upon reaching level 200, you have unlocked new command:\n\u{f0002}@ascension <option>\u{f0001}\n\n\u{f0004}Available Options:\u{f0005}\n0 - View your active ascensions\n1 - Hoarder Ascension: Gain 5x EXP and drop rates for all characters\n2 - Resilient Ascension: Characters become immune to debuffs\n\n\u{f0004}Please Note:\u{f0005} \u{f0002}Ascending will send your current character to level 1.",
        0,
        &NpcInlineIcons::new(),
        &[],
        0,
        0,
        None,
        "",
    );
    (pixels, (panel.x, panel.y, panel.width, panel.height))
}

fn dialogue_text_width(font: &fontdue::Font, text: &str, size: f32) -> f32 {
    ui_foundation::measure_text(font, text, size)
}

fn rasterize_maple_tv_text(
    font: &fontdue::Font,
    lines: &[String],
    start_x: i32,
    color: [u8; 3],
) -> Vec<u8> {
    const WIDTH: usize = 240;
    const HEIGHT: usize = 90;
    const FONT_SIZE: f32 = 11.0;
    const LINE_HEIGHT: i32 = 16;
    let mut pixels = vec![0; WIDTH * HEIGHT * 4];
    let right = WIDTH as f32 - 5.0;
    let mut y = 8_i32;

    for line in lines.iter().take(5) {
        let mut x = start_x as f32;
        let mut previous = None;
        for character in line.chars() {
            if y + LINE_HEIGHT > HEIGHT as i32 {
                break;
            }
            if character == '\r' {
                continue;
            }
            if character == '\n' {
                x = start_x as f32;
                y += LINE_HEIGHT;
                previous = None;
                continue;
            }
            let kern = previous.map_or(0.0, |previous| {
                font.horizontal_kern(previous, character, FONT_SIZE)
                    .unwrap_or(0.0)
            });
            let (metrics, coverage) = rasterize_ui_glyph(font, character, FONT_SIZE);
            let mut glyph_kern = kern;
            if x + glyph_kern + metrics.advance_width > right {
                x = start_x as f32;
                y += LINE_HEIGHT;
                glyph_kern = 0.0;
            }
            if y + LINE_HEIGHT > HEIGHT as i32 {
                break;
            }
            x += glyph_kern;
            let left = x.round() as i32 + metrics.xmin;
            let baseline = y + FONT_SIZE.ceil() as i32 + 1;
            let top = baseline - metrics.ymin - metrics.height as i32;
            for row in 0..metrics.height {
                for column in 0..metrics.width {
                    let px = left + column as i32;
                    let py = top + row as i32;
                    if px < start_x || px >= WIDTH as i32 || py < 0 || py >= HEIGHT as i32 {
                        continue;
                    }
                    let alpha = coverage[row * metrics.width + column];
                    if alpha == 0 {
                        continue;
                    }
                    let index = (py as usize * WIDTH + px as usize) * 4;
                    pixels[index..index + 4]
                        .copy_from_slice(&[color[2], color[1], color[0], alpha]);
                }
            }
            x += metrics.advance_width;
            previous = Some(character);
        }
        y += LINE_HEIGHT;
    }
    pixels
}

fn draw_dialogue_text(
    font: &fontdue::Font,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    text: &str,
    x: i32,
    y: i32,
    size: f32,
    max_width: usize,
    max_height: usize,
    color: [u8; 3],
) {
    let x = ui_foundation::snap(x as f32) as i32;
    let y = ui_foundation::snap(y as f32) as i32;
    draw_dialogue_text_with_icons(
        font,
        pixels,
        width,
        height,
        text,
        x,
        y,
        size,
        max_width,
        max_height,
        color,
        &EMPTY_NPC_INLINE_ICONS,
    );
}

fn draw_dialogue_text_with_icons(
    font: &fontdue::Font,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    text: &str,
    x: i32,
    y: i32,
    size: f32,
    max_width: usize,
    max_height: usize,
    color: [u8; 3],
    inline_icons: &NpcInlineIcons,
) {
    draw_dialogue_text_with_icons_scrolled(
        font,
        pixels,
        width,
        height,
        text,
        x,
        y,
        size,
        max_width,
        max_height,
        color,
        inline_icons,
        0,
    );
}

fn draw_dialogue_text_with_icons_scrolled(
    font: &fontdue::Font,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    text: &str,
    x: i32,
    y: i32,
    size: f32,
    max_width: usize,
    max_height: usize,
    color: [u8; 3],
    inline_icons: &NpcInlineIcons,
    vertical_offset: usize,
) {
    let line_height = size.ceil() as i32 + 4;
    let mut pen_x = x as f32;
    let mut line_y = y;
    let mut previous = None;
    let mut text_color = color;
    let mut bold = false;
    let wrapped_text = wrap_dialogue_text(font, text, size, max_width, inline_icons);
    for character in wrapped_text.chars() {
        match character {
            '\u{f0000}' => {
                text_color = [108, 61, 145];
                previous = None;
                continue;
            }
            '\u{f0001}' => {
                text_color = color;
                previous = None;
                continue;
            }
            '\u{f0002}' => {
                text_color = [225, 45, 36];
                previous = None;
                continue;
            }
            '\u{f0003}' => {
                text_color = [80, 80, 80];
                previous = None;
                continue;
            }
            '\u{f0004}' => {
                bold = true;
                previous = None;
                continue;
            }
            '\u{f0005}' => {
                bold = false;
                previous = None;
                continue;
            }
            '\u{f0006}' => {
                text_color = [35, 130, 45];
                previous = None;
                continue;
            }
            _ => {}
        }
        let font = if bold {
            ui_foundation::bold_font().unwrap_or(font)
        } else {
            font
        };
        if character == '\n' {
            pen_x = x as f32;
            line_y += line_height;
            previous = None;
            continue;
        }
        if let Some((icon_width, icon_height, icon_pixels)) = inline_icons.get(&character) {
            let icon_size = (size.ceil() as usize + 4).clamp(16, 32);
            if pen_x + icon_size as f32 > x as f32 + max_width as f32 {
                pen_x = x as f32;
                line_y += line_height;
            }
            let icon_y = line_y - vertical_offset as i32;
            if icon_y >= y && icon_y + icon_size as i32 <= y + max_height as i32 {
                composite_inventory_icon(
                    pixels,
                    width,
                    height,
                    icon_pixels,
                    *icon_width as usize,
                    *icon_height as usize,
                    pen_x.round().max(0.0) as usize,
                    icon_y.max(0) as usize,
                    icon_size,
                );
            }
            pen_x += icon_size as f32;
            previous = None;
            continue;
        }
        if let Some(previous) = previous {
            pen_x += font
                .horizontal_kern(previous, character, size)
                .unwrap_or(0.0);
        }
        let (metrics, coverage) = rasterize_ui_glyph(font, character, size);
        if pen_x + metrics.advance_width > x as f32 + max_width as f32 {
            pen_x = x as f32;
            line_y += line_height;
        }
        let display_line_y = line_y - vertical_offset as i32;
        let left = pen_x.round() as i32 + metrics.xmin;
        let baseline = display_line_y + size.ceil() as i32 + 1;
        let top = baseline - metrics.ymin - metrics.height as i32;
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                let px = left + column as i32;
                let py = top + row as i32;
                if px < x.max(0)
                    || px >= x.saturating_add(max_width.min(i32::MAX as usize) as i32)
                    || py < y.max(0)
                    || px >= width as i32
                    || py >= (y + max_height as i32).min(height as i32)
                {
                    continue;
                }
                let alpha = coverage[row * metrics.width + column];
                if alpha == 0 {
                    continue;
                }
                let index = (py as usize * width + px as usize) * 4;
                let source_alpha = u32::from(alpha);
                let destination_alpha = u32::from(pixels[index + 3]);
                let inverse = 255 - source_alpha;
                let output_alpha = source_alpha + destination_alpha * inverse / 255;
                if output_alpha == 0 {
                    continue;
                }
                for (channel, foreground) in [text_color[2], text_color[1], text_color[0]]
                    .into_iter()
                    .enumerate()
                {
                    let destination = u32::from(pixels[index + channel]);
                    let premultiplied = u32::from(foreground) * source_alpha
                        + destination * destination_alpha * inverse / 255;
                    pixels[index + channel] = (premultiplied / output_alpha) as u8;
                }
                pixels[index + 3] = output_alpha as u8;
            }
        }
        pen_x += metrics.advance_width;
        previous = Some(character);
    }
}

fn wrap_dialogue_text(
    font: &fontdue::Font,
    text: &str,
    size: f32,
    max_width: usize,
    inline_icons: &NpcInlineIcons,
) -> String {
    let mut output = String::new();
    for (paragraph_index, paragraph) in text.split('\n').enumerate() {
        if paragraph_index > 0 {
            output.push('\n');
        }
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            let mut width = 0.0_f32;
            let mut previous = None;
            for character in candidate.chars() {
                if (0xf0000..=0xf0006).contains(&(character as u32)) {
                    continue;
                }
                if inline_icons.contains_key(&character) {
                    width += (size.ceil() as usize + 4).clamp(16, 32) as f32;
                    previous = None;
                } else {
                    if let Some(previous) = previous {
                        width += font
                            .horizontal_kern(previous, character, size)
                            .unwrap_or(0.0);
                    }
                    width += font.metrics(character, size).advance_width;
                    previous = Some(character);
                }
            }
            if !line.is_empty() && width > max_width as f32 {
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str(&line);
                line.clear();
                line.push_str(word);
            } else {
                line = candidate;
            }
        }
        if !line.is_empty() {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(&line);
        }
    }
    output
}

fn fill_dialogue_rect(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: [u8; 4],
) {
    let right = x.saturating_add(width).min(canvas_width);
    let bottom = y.saturating_add(height).min(canvas_height);
    for row in y.min(canvas_height)..bottom {
        for column in x.min(canvas_width)..right {
            let index = (row * canvas_width + column) * 4;
            pixels[index..index + 4].copy_from_slice(&color);
        }
    }
}

fn composite_nx_ui_bitmap(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    archive: &NxArchive,
    path: &str,
    anchor_x: i32,
    anchor_y: i32,
) -> Result<(), String> {
    let node = resolve_ui_path(&archive.root(), path)?
        .ok_or_else(|| format!("missing v83 UI asset {path:?}"))?;
    composite_nx_node_bitmap(
        pixels,
        canvas_width,
        canvas_height,
        &node,
        anchor_x,
        anchor_y,
    )
}

fn composite_nx_node_bitmap(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    node: &openstory_assets::NxNode,
    anchor_x: i32,
    anchor_y: i32,
) -> Result<(), String> {
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "NX sprite node is not a bitmap".to_owned())?;
    let source = bitmap
        .decode_bgra_shared()
        .map_err(|error| error.to_string())?;
    let origin = node
        .child("origin")
        .map_err(|error| error.to_string())?
        .map(|origin| origin.value())
        .transpose()
        .map_err(|error| error.to_string())?;
    let (origin_x, origin_y) = match origin {
        Some(NodeValue::Vector { x, y }) => (x, y),
        Some(_) => return Err("NX sprite has a malformed origin".to_owned()),
        None => (0, 0),
    };
    let left = anchor_x - origin_x;
    let top = anchor_y - origin_y;
    let source_width = usize::from(bitmap.width());
    let source_height = usize::from(bitmap.height());
    for source_y in 0..source_height {
        let destination_y = top + source_y as i32;
        if !(0..canvas_height as i32).contains(&destination_y) {
            continue;
        }
        for source_x in 0..source_width {
            let destination_x = left + source_x as i32;
            if !(0..canvas_width as i32).contains(&destination_x) {
                continue;
            }
            let source_index = (source_y * source_width + source_x) * 4;
            let destination_index =
                (destination_y as usize * canvas_width + destination_x as usize) * 4;
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
