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
    let characters = text.chars().collect::<Vec<_>>();
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
        if matches!(code, 'b' | 'k' | 'r' | 'd' | 'c' | 'e' | 'n' | 'g') {
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
    let node = archive.root().resolve(&path).ok().flatten()?;
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
        let Some(label_end_relative) = text[label_start..].find("#l") else {
            plain.push_str(&text[start..]);
            return (clean_npc_markup(&plain), selections);
        };
        let label_end = label_start + label_end_relative;
        selections.push((
            selection_id,
            clean_npc_markup(&text[label_start..label_end]),
        ));
        cursor = label_end + 2;
    }
    plain.push_str(&text[cursor..]);
    (clean_npc_markup(&plain), selections)
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
                181 => "PetEquip",
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
    archive.root().resolve(&info_path).ok().flatten()
}

fn npc_shop_item_is_unsellable(
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    item_id: i32,
) -> bool {
    npc_shop_item_info(item_archive, character_archive, item_id)
        .as_ref()
        .and_then(|info| integer_property(info, "notSale"))
        .is_some_and(|value| value != 0)
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
    format!("{} mesos", reversed.chars().rev().collect::<String>())
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
    quantity: i16,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
    notice: Option<&str>,
    confirmation: Option<NpcShopConfirmation>,
    item_archive: &NxArchive,
    string_archive: &NxArchive,
    character_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> (Vec<u8>, SpriteRect) {
    const WIDTH: usize = 620;
    const HEIGHT: usize = 520;
    const PANEL_X: f32 = 202.0;
    const PANEL_Y: f32 = 124.0;
    let sell_items = inventory
        .map(|inventory| npc_shop_sellable_items(inventory, item_archive, character_archive))
        .unwrap_or_default();
    let item_count = match mode {
        NpcShopMode::Buy => shop.items.len(),
        NpcShopMode::Sell => sell_items.len(),
    };
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
        42,
        [83, 63, 40, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        2,
        [207, 176, 119, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        HEIGHT - 2,
        WIDTH,
        2,
        [207, 176, 119, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        2,
        HEIGHT,
        [207, 176, 119, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        WIDTH - 2,
        0,
        2,
        HEIGHT,
        [207, 176, 119, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &format!("Shop · NPC {}", shop.npc_id),
        18,
        10,
        20.0,
        330,
        26,
        [246, 226, 185],
    );
    for (x, label, active) in [
        (420, "Buy", mode == NpcShopMode::Buy),
        (490, "Sell", mode == NpcShopMode::Sell),
    ] {
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            9,
            60,
            26,
            if active {
                [112, 81, 45, 255]
            } else {
                [53, 51, 47, 255]
            },
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            (x + 14) as i32,
            13,
            12.0,
            36,
            16,
            [246, 231, 199],
        );
    }
    let currency = if mode == NpcShopMode::Sell {
        "Choose an item from your inventory to sell".to_owned()
    } else if shop.currency_item_id > 0 {
        format!(
            "Currency: {}",
            item_name(shop.currency_item_id, string_archive, name_cache)
        )
    } else {
        "Currency: mesos".to_owned()
    };
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &currency,
        20,
        48,
        12.0,
        WIDTH - 40,
        17,
        [210, 207, 196],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        584,
        9,
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
        593,
        12,
        14.0,
        14,
        18,
        [255, 225, 210],
    );
    for visible_row in 0..9 {
        let index = first_visible.saturating_add(visible_row);
        let y = 66 + visible_row * 40;
        let row_data = match mode {
            NpcShopMode::Buy => shop.items.get(index).map(|item| {
                let pitch_owned = if item.price <= 0 && item.pitch > 0 {
                    Some(inventory.map_or(0, |inventory| {
                        inventory_item_count(
                            inventory,
                            openstory_protocol::InventoryType::Etc,
                            PERFECT_PITCH_ITEM_ID,
                        )
                    }))
                } else {
                    None
                };
                let display_price = if item.charge_price > 0 && item.price == 0 {
                    i32::from(item.charge_price)
                } else {
                    item.price
                };
                let price = if item.price <= 0 && item.pitch > 0 {
                    format!(
                        "{} {}",
                        item.pitch,
                        item_name(PERFECT_PITCH_ITEM_ID, string_archive, name_cache)
                    )
                } else {
                    let currency = if shop.currency_item_id > 0 {
                        item_name(shop.currency_item_id, string_archive, name_cache)
                    } else {
                        "meso".to_owned()
                    };
                    format!("{} {currency}", display_price.max(0))
                };
                let stock = if let Some(pitch_owned) = pitch_owned {
                    let stock = if item.buyable == 0 {
                        "unlimited".to_owned()
                    } else {
                        item.buyable.max(0).to_string()
                    };
                    format!("Stock: {stock} · Have {pitch_owned}")
                } else if item.buyable == 0 {
                    "Stock: unlimited".to_owned()
                } else {
                    format!("Stock: {}", item.buyable.max(0))
                };
                (
                    item.item_id,
                    if item.rechargeable {
                        format!(
                            "{} ({})  (Rechargeable)",
                            item_name(item.item_id, string_archive, name_cache),
                            item.item_id
                        )
                    } else {
                        format!(
                            "{} ({})",
                            item_name(item.item_id, string_archive, name_cache),
                            item.item_id
                        )
                    },
                    price,
                    stock,
                )
            }),
            NpcShopMode::Sell => sell_items.get(index).map(|item| {
                (
                    item.item_id,
                    format!(
                        "{} ({})  ·  Slot {}",
                        item_name(item.item_id, string_archive, name_cache),
                        item.item_id,
                        item.slot
                    ),
                    npc_shop_item_price(item_archive, character_archive, item.item_id)
                        .map(format_meso_amount)
                        .unwrap_or_else(|| "Price unavailable".to_owned()),
                    format!("Count: {}", item.count),
                )
            }),
        };
        if let Some((item_id, label, price, stock)) = row_data {
            let is_selected = index == selected;
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                12,
                y,
                WIDTH - 24,
                36,
                if is_selected {
                    [83, 67, 45, 255]
                } else {
                    [38, 40, 44, 255]
                },
            );
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                12,
                y + 35,
                WIDTH - 24,
                1,
                [77, 74, 67, 255],
            );
            if let Some((icon_width, icon_height, icon_pixels)) =
                inventory_item_icon(item_id, item_archive, character_archive, icon_cache)
            {
                composite_inventory_icon(
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    &icon_pixels,
                    icon_width as usize,
                    icon_height as usize,
                    18,
                    y + 3,
                    30,
                );
            }
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &label,
                58,
                (y + 3) as i32,
                13.0,
                270,
                17,
                [238, 237, 229],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &price,
                354,
                (y + 4) as i32,
                12.0,
                240,
                16,
                [232, 216, 171],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &stock,
                58,
                (y + 20) as i32,
                10.0,
                250,
                13,
                [177, 177, 171],
            );
        }
    }
    if item_count == 0 {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            if mode == NpcShopMode::Buy {
                "This shop has no items."
            } else {
                "You have no items to sell."
            },
            160,
            250,
            16.0,
            300,
            22,
            [205, 205, 201],
        );
    }
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        16,
        434,
        WIDTH - 32,
        1,
        [108, 99, 80, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        notice.unwrap_or("Select an item, then buy or sell. Scroll to browse."),
        18,
        436,
        10.0,
        WIDTH - 36,
        14,
        [218, 203, 166],
    );
    let selected_buy = if mode == NpcShopMode::Buy {
        shop.items.get(selected)
    } else {
        None
    };
    let selected_sell = if mode == NpcShopMode::Sell {
        sell_items.get(selected).copied()
    } else {
        None
    };
    let max_quantity = match (selected_buy, selected_sell) {
        (Some(item), _) if item.buyable == 0 => 100,
        (Some(item), _) => item.buyable.max(1),
        (_, Some(item)) => i16::try_from(item.count.max(1)).unwrap_or(i16::MAX),
        _ => 1,
    };
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &format!(
            "Quantity: {} / {}",
            quantity.clamp(1, max_quantity),
            max_quantity
        ),
        150,
        466,
        13.0,
        112,
        18,
        [233, 231, 221],
    );
    for (x, label) in [(270, "−"), (310, "+")] {
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            454,
            36,
            32,
            [65, 63, 57, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            (x + 12) as i32,
            461,
            16.0,
            14,
            20,
            [245, 236, 215],
        );
    }
    for (x, width, label, enabled) in [
        (
            354,
            80,
            "Recharge",
            selected_buy.is_some_and(|item| item.rechargeable),
        ),
        (
            442,
            78,
            if mode == NpcShopMode::Buy {
                "Buy"
            } else {
                "Sell"
            },
            selected_buy.is_some_and(|item| !item.rechargeable) || selected_sell.is_some(),
        ),
        (528, 80, "Leave", true),
    ] {
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            454,
            width,
            32,
            if enabled {
                [88, 68, 45, 255]
            } else {
                [47, 47, 46, 255]
            },
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            (x + 7) as i32,
            461,
            11.0,
            width - 14,
            18,
            if enabled {
                [246, 231, 199]
            } else {
                [132, 132, 127]
            },
        );
    }
    if let Some(confirmation) = confirmation {
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            0,
            0,
            WIDTH,
            HEIGHT,
            [0, 0, 0, 190],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            90,
            160,
            440,
            190,
            [177, 145, 92, 255],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            94,
            164,
            432,
            182,
            [33, 31, 30, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "Confirm transaction",
            120,
            184,
            18.0,
            380,
            24,
            [246, 231, 199],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            if confirmation.recharge {
                "Are you sure you want to recharge this item?"
            } else if confirmation.mode == NpcShopMode::Buy {
                "Are you sure you want to buy this item?"
            } else {
                "Are you sure you want to sell this item?"
            },
            118,
            222,
            13.0,
            384,
            20,
            [231, 226, 214],
        );
        for (x, label) in [(150, "Yes"), (340, "No")] {
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                x,
                280,
                130,
                36,
                [88, 68, 45, 255],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                label,
                x as i32 + 46,
                287,
                14.0,
                40,
                20,
                [246, 231, 199],
            );
        }
    }
    (
        pixels,
        SpriteRect {
            x: PANEL_X,
            y: PANEL_Y,
            width: WIDTH as f32,
            height: HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        },
    )
}

fn npc_dialogue_text_view_height(message_type: i8) -> usize {
    match message_type {
        2 | 3 => 112,
        4 => 58,
        _ => 145,
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
    let mut has_content = false;
    for character in text.chars() {
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
    npc_id: i32,
    speaker: i8,
    message_type: i8,
    text: &str,
    text_scroll: usize,
    inline_icons: &NpcInlineIcons,
    selections: &[(i32, String)],
    selected_selection: usize,
    first_visible_selection: usize,
    input: &str,
) -> (Vec<u8>, SpriteRect, Vec<NpcDialogueButton>, usize) {
    const WIDTH: usize = 600;
    const HEIGHT: usize = 260;
    const PANEL_X: f32 = 212.0;
    const PANEL_Y: f32 = 490.0;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        HEIGHT,
        [26, 22, 18, 235],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        36,
        [92, 70, 42, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        2,
        [202, 177, 126, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        HEIGHT - 2,
        WIDTH,
        2,
        [202, 177, 126, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        2,
        HEIGHT,
        [202, 177, 126, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        WIDTH - 2,
        0,
        2,
        HEIGHT,
        [202, 177, 126, 255],
    );
    let title = if speaker == 0 {
        format!("NPC {npc_id}")
    } else {
        "Dialogue".to_owned()
    };
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &title,
        22,
        8,
        18.0,
        246,
        26,
        [236, 223, 192],
    );
    let keyboard_hint = match message_type {
        1 => "SPACE: REVEAL / YES   ENTER: YES",
        2 => "SPACE: REVEAL / TYPE   ENTER: SUBMIT",
        3 => "SPACE: REVEAL / SUBMIT   ENTER: SUBMIT",
        4 => "UP/DOWN SELECT   SPACE: CHOOSE",
        12 => "SPACE: REVEAL / ACCEPT   ENTER: ACCEPT",
        _ => "SPACE: REVEAL / NEXT   ENTER: NEXT",
    };
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        keyboard_hint,
        286,
        11,
        12.0,
        292,
        18,
        [229, 208, 166],
    );
    let text_view_height = npc_dialogue_text_view_height(message_type);
    let text_content_height = dialogue_text_content_height(font, text, 16.0, 548, inline_icons);
    let text_max_scroll = if message_type == 4 {
        0
    } else {
        text_content_height.saturating_sub(text_view_height)
    };
    let text_scroll = text_scroll.min(text_max_scroll);
    draw_dialogue_text_with_icons_scrolled(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        text,
        26,
        50,
        16.0,
        548,
        text_view_height,
        [235, 235, 235],
        inline_icons,
        text_scroll,
    );
    if text_max_scroll > 0 {
        const SCROLL_TRACK_X: usize = 582;
        const SCROLL_TRACK_Y: usize = 50;
        const SCROLL_TRACK_WIDTH: usize = 5;
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            SCROLL_TRACK_X,
            SCROLL_TRACK_Y,
            SCROLL_TRACK_WIDTH,
            text_view_height,
            [56, 53, 48, 255],
        );
        let content_height = text_view_height.saturating_add(text_max_scroll);
        let thumb_height =
            (text_view_height * text_view_height / content_height).clamp(18, text_view_height);
        let thumb_range = text_view_height.saturating_sub(thumb_height);
        let thumb_y = SCROLL_TRACK_Y + thumb_range * text_scroll / text_max_scroll;
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            SCROLL_TRACK_X,
            thumb_y,
            SCROLL_TRACK_WIDTH,
            thumb_height,
            [198, 169, 116, 255],
        );
    }

    let mut buttons = Vec::new();
    if message_type == 4 {
        for (row, (index, (selection, label))) in selections
            .iter()
            .enumerate()
            .skip(first_visible_selection)
            .take(5)
            .enumerate()
        {
            let local_y = 112 + row as i32 * 25;
            let rect = SpriteRect {
                x: PANEL_X + 28.0,
                y: PANEL_Y + local_y as f32,
                width: 544.0,
                height: 23.0,
                blend_mode: BlendMode::Alpha,
            };
            draw_dialogue_button_with_icons(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                rect,
                PANEL_X,
                PANEL_Y,
                label,
                inline_icons,
            );
            if index == selected_selection {
                let x = 28_usize;
                let y = local_y as usize;
                let color = [244, 210, 124, 255];
                fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, x, y, 544, 2, color);
                fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, x, y + 21, 544, 2, color);
                fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, x, y, 2, 23, color);
                fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, x + 542, y, 2, 23, color);
            }
            buttons.push(NpcDialogueButton {
                rect,
                action: NpcDialogueAction::Select(*selection),
            });
        }
        push_dialogue_button(
            &mut pixels,
            &mut buttons,
            font,
            WIDTH,
            HEIGHT,
            PANEL_X,
            PANEL_Y,
            "Close",
            0,
            234,
        );
    } else {
        if message_type == 2 || message_type == 3 {
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                26,
                172,
                548,
                28,
                [72, 64, 52, 255],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                input,
                36,
                177,
                16.0,
                530,
                20,
                [255, 255, 255],
            );
        }
        let (labels, actions): (&[&str], &[NpcDialogueAction]) = match message_type {
            1 => (
                &["Yes", "No"],
                &[NpcDialogueAction::More(1), NpcDialogueAction::More(0)],
            ),
            2 => (
                &["OK", "Cancel"],
                &[NpcDialogueAction::SubmitText, NpcDialogueAction::More(0)],
            ),
            3 => (
                &["−", "+", "OK", "Cancel"],
                &[
                    NpcDialogueAction::AdjustNumber(-1),
                    NpcDialogueAction::AdjustNumber(1),
                    NpcDialogueAction::SubmitNumber,
                    NpcDialogueAction::More(0),
                ],
            ),
            5 => (&["Next"], &[NpcDialogueAction::More(1)]),
            6 => (
                &["Back", "Next"],
                &[NpcDialogueAction::More(0), NpcDialogueAction::More(1)],
            ),
            12 => (
                &["Accept", "Decline"],
                &[NpcDialogueAction::More(1), NpcDialogueAction::More(0)],
            ),
            _ => (&["OK"], &[NpcDialogueAction::More(1)]),
        };
        let width = if message_type == 3 { 58.0 } else { 104.0 };
        let gap = 12.0;
        let total = labels.len() as f32 * width + labels.len().saturating_sub(1) as f32 * gap;
        let start_x = (WIDTH as f32 - total) / 2.0;
        for (index, (label, action)) in labels.iter().zip(actions).enumerate() {
            let local_x = start_x + index as f32 * (width + gap);
            let rect = SpriteRect {
                x: PANEL_X + local_x,
                y: PANEL_Y + 214.0,
                width,
                height: 30.0,
                blend_mode: BlendMode::Alpha,
            };
            draw_dialogue_button(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                rect,
                PANEL_X,
                PANEL_Y,
                label,
            );
            buttons.push(NpcDialogueButton {
                rect,
                action: *action,
            });
        }
    }
    (
        pixels,
        SpriteRect {
            x: PANEL_X,
            y: PANEL_Y,
            width: WIDTH as f32,
            height: HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        },
        buttons,
        text_max_scroll,
    )
}

fn push_dialogue_button(
    pixels: &mut [u8],
    buttons: &mut Vec<NpcDialogueButton>,
    font: &fontdue::Font,
    width: usize,
    height: usize,
    panel_x: f32,
    panel_y: f32,
    label: &str,
    response: i8,
    local_y: i32,
) {
    let rect = SpriteRect {
        x: panel_x + 248.0,
        y: panel_y + local_y as f32,
        width: 104.0,
        height: 24.0,
        blend_mode: BlendMode::Alpha,
    };
    draw_dialogue_button(font, pixels, width, height, rect, panel_x, panel_y, label);
    buttons.push(NpcDialogueButton {
        rect,
        action: NpcDialogueAction::More(response),
    });
}

fn draw_dialogue_button(
    font: &fontdue::Font,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    rect: SpriteRect,
    panel_x: f32,
    panel_y: f32,
    label: &str,
) {
    draw_dialogue_button_with_icons(
        font,
        pixels,
        width,
        height,
        rect,
        panel_x,
        panel_y,
        label,
        &EMPTY_NPC_INLINE_ICONS,
    );
}

fn draw_dialogue_button_with_icons(
    font: &fontdue::Font,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    rect: SpriteRect,
    panel_x: f32,
    panel_y: f32,
    label: &str,
    inline_icons: &NpcInlineIcons,
) {
    let x = (rect.x - panel_x).round().max(0.0) as usize;
    let y = (rect.y - panel_y).round().max(0.0) as usize;
    let button_width = rect.width.round().max(0.0) as usize;
    let button_height = rect.height.round().max(0.0) as usize;
    fill_dialogue_rect(
        pixels,
        width,
        height,
        x,
        y,
        button_width,
        button_height,
        [108, 91, 64, 255],
    );
    let text_width = dialogue_text_width(font, label, 15.0);
    let text_x = x as i32 + ((button_width as f32 - text_width) / 2.0).max(4.0) as i32;
    let text_y = y as i32 + ((button_height as f32 - 18.0) / 2.0).max(0.0) as i32;
    draw_dialogue_text_with_icons(
        font,
        pixels,
        width,
        height,
        label,
        text_x,
        text_y,
        15.0,
        button_width.saturating_sub(8),
        button_height,
        [255, 255, 255],
        inline_icons,
    );
}

fn dialogue_text_width(font: &fontdue::Font, text: &str, size: f32) -> f32 {
    let mut width = 0.0;
    let mut previous = None;
    for character in text.chars() {
        if let Some(previous) = previous {
            width += font
                .horizontal_kern(previous, character, size)
                .unwrap_or(0.0);
        }
        width += font.metrics(character, size).advance_width;
        previous = Some(character);
    }
    width
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
    for character in text.chars() {
        if character == '\n' {
            pen_x = x as f32;
            line_y += line_height;
            previous = None;
            continue;
        }
        if let Some((icon_width, icon_height, icon_pixels)) = inline_icons.get(&character) {
            let icon_size = (size.ceil() as usize + 4).clamp(16, 32);
            if pen_x + icon_size as f32 > (x as usize + max_width) as f32 {
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
        if pen_x + metrics.advance_width > (x as usize + max_width) as f32 {
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
                if px < 0
                    || py < y
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
                for (channel, foreground) in [color[2], color[1], color[0]].into_iter().enumerate()
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
    let node = archive
        .root()
        .resolve(path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("missing v83 UI asset {path:?}"))?;
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("v83 UI asset {path:?} is not a bitmap"))?;
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
        Some(_) => return Err(format!("v83 UI asset {path:?} has a malformed origin")),
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
