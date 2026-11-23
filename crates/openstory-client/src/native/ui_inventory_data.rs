fn inventory_tabs() -> [openstory_protocol::InventoryType; 5] {
    [
        openstory_protocol::InventoryType::Equip,
        openstory_protocol::InventoryType::Use,
        openstory_protocol::InventoryType::Setup,
        openstory_protocol::InventoryType::Etc,
        openstory_protocol::InventoryType::Cash,
    ]
}

fn inventory_slot_max(
    inventory: &openstory_protocol::InventorySnapshot,
    tab: openstory_protocol::InventoryType,
) -> u16 {
    match tab {
        openstory_protocol::InventoryType::Equip => u16::from(inventory.slot_maxima[0]),
        openstory_protocol::InventoryType::Use => u16::from(inventory.slot_maxima[1]),
        openstory_protocol::InventoryType::Setup => u16::from(inventory.slot_maxima[2]),
        openstory_protocol::InventoryType::Etc => u16::from(inventory.slot_maxima[3]),
        openstory_protocol::InventoryType::Cash => u16::from(inventory.slot_maxima[4]),
        // Includes the sparse v83 bonus slots (medal, belt, pocket, pendant,
        // shoulder, android, and newer gear); INVENTORY_GROW can update it.
        openstory_protocol::InventoryType::Equipped => u16::from(inventory.equipped_slot_maximum),
    }
}

fn inventory_tab_at(cursor: Point) -> Option<openstory_protocol::InventoryType> {
    inventory_tabs()
        .into_iter()
        .enumerate()
        .find_map(|(index, tab)| {
            ui_foundation::inventory_tab(index)
                .contains(cursor.x - INVENTORY_PANEL_X, cursor.y - INVENTORY_PANEL_Y)
                .then_some(tab)
        })
}

fn inventory_slot_at(cursor: Point, scroll_row: u16) -> Option<u16> {
    let index = ui_foundation::INVENTORY_GRID
        .hit(cursor.x - INVENTORY_PANEL_X, cursor.y - INVENTORY_PANEL_Y)?;
    Some(
        scroll_row
            .saturating_mul(INVENTORY_SLOT_COLUMNS as u16)
            .saturating_add(index as u16 + 1),
    )
}

fn first_free_equip_inventory_slot(
    inventory: &openstory_protocol::InventorySnapshot,
) -> Option<i16> {
    (1..=u16::from(inventory.slot_maxima[0]))
        .find(|slot| {
            !inventory.items.iter().any(|item| {
                item.inventory_type == openstory_protocol::InventoryType::Equip
                    && item.slot == *slot
            })
        })
        .and_then(|slot| i16::try_from(slot).ok())
}

// Pet equipment uses dedicated equipped positions. The 1812000-1812007
// ability items occupy slots 122-129; the name and quote rings use 114 and
// 113. Sending every PetEquip item to -123 makes each one replace the last.
fn pet_equipment_destination(
    item_id: i32,
    inventory: &openstory_protocol::InventorySnapshot,
) -> Option<i16> {
    let (slot, preserve_existing) = match item_id {
        1_812_000..=1_812_007 => (122 + (item_id - 1_812_000) as i16, false),
        id if id / 10_000 == 180 => (121, false),
        id if id / 10_000 == 182 => (114, true),
        id if id / 10_000 == 183 => (113, true),
        _ => return None,
    };
    if preserve_existing {
        if let Some(equipped_slot) = inventory
            .equipped()
            .find(|item| item.item_id == item_id)
            .and_then(|item| i16::try_from(item.slot).ok())
            .filter(|equipped_slot| *equipped_slot > 0)
        {
            return Some(equipped_slot);
        }
        if inventory
            .equipped()
            .any(|item| i32::from(item.slot) == i32::from(slot))
        {
            return None;
        }
    }
    Some(slot)
}

fn equip_slot_for_item(
    item_id: i32,
    cash: bool,
    inventory: &openstory_protocol::InventorySnapshot,
) -> Option<i16> {
    if item_id <= 0 {
        return None;
    }
    let category = item_id / 10_000;
    let offset = if cash { 100 } else { 0 };
    let slot = match category {
        100 => 1,
        101 => 2,
        102 => 3,
        103 => 4,
        104 | 105 => 5,
        106 => 6,
        107 => 7,
        108 => 8,
        109 => 10,
        110 => 9,
        111 => [13_u16, 15, 16]
            .into_iter()
            .find(|slot| !inventory.equipped().any(|item| item.slot == *slot + offset))
            .unwrap_or(12),
        112 => 17,
        113 => 50,
        114 => 49,
        130..=149 => 11,
        170 if cash => 11,
        181 => return None,
        190 => 18,
        191 => 19,
        _ => return None,
    };
    Some((slot + offset) as i16)
}

fn equipment_block_reason(
    item_id: i32,
    character: &openstory_protocol::CharacterInfo,
    inventory: &openstory_protocol::InventorySnapshot,
    archive: &NxArchive,
) -> Option<&'static str> {
    equipment_block_reason_with_level_check(item_id, character, inventory, archive, true)
}

fn equipment_recommendation_block_reason(
    item_id: i32,
    character: &openstory_protocol::CharacterInfo,
    inventory: &openstory_protocol::InventorySnapshot,
    archive: &NxArchive,
) -> Option<&'static str> {
    equipment_block_reason_with_level_check(item_id, character, inventory, archive, false)
}

fn equipment_block_reason_with_level_check(
    item_id: i32,
    character: &openstory_protocol::CharacterInfo,
    inventory: &openstory_protocol::InventorySnapshot,
    archive: &NxArchive,
    check_level: bool,
) -> Option<&'static str> {
    let category = character_equip_category(item_id)?;
    let item_prefix = item_id / 10_000;
    let gender =
        if (item_id / 1_000_000 != 1 && item_prefix != 254) || matches!(item_prefix, 119 | 168) {
            2
        } else {
            let gender_digit = item_id / 1_000 % 10;
            if gender_digit > 1 { 2 } else { gender_digit }
        };
    if (gender == 0 && character.stats.female) || (gender == 1 && !character.stats.female) {
        return Some("This equipment is for a different character gender.");
    }

    if matches!(character.stats.job, 900 | 910) {
        return None;
    }

    let info_path = format!("{category}/0{item_id}.img/info");
    let Some(info) = archive.root().resolve(&info_path).ok().flatten() else {
        return Some("Equipment data is unavailable for this item.");
    };
    let required_job = integer_property(&info, "reqJob")
        .map_or(0, |value| u16::try_from(value).unwrap_or(u16::MAX));
    if !job_can_equip(character.stats.job, required_job) {
        return Some("Your current job cannot equip this item.");
    }

    let required_level = integer_property(&info, "reqLevel").unwrap_or(0);
    let equipped_bonus = |index: usize| -> i64 {
        inventory
            .equipped()
            .filter_map(|item| {
                let item = item.equip.as_ref()?;
                item.stats.get(index).copied().map(i64::from)
            })
            .sum()
    };
    let total_strength = i64::from(character.stats.strength) + equipped_bonus(0);
    let total_dexterity = i64::from(character.stats.dexterity) + equipped_bonus(1);
    let total_intelligence = i64::from(character.stats.intelligence) + equipped_bonus(2);
    let total_luck = i64::from(character.stats.luck) + equipped_bonus(3);
    let requirements_met = (!check_level || i64::from(character.stats.level) >= required_level)
        && total_dexterity >= integer_property(&info, "reqDEX").unwrap_or(0)
        && total_strength >= integer_property(&info, "reqSTR").unwrap_or(0)
        && total_luck >= integer_property(&info, "reqLUK").unwrap_or(0)
        && total_intelligence >= integer_property(&info, "reqINT").unwrap_or(0)
        && i64::from(character.stats.fame) >= integer_property(&info, "reqFAME").unwrap_or(0);
    (!requirements_met).then_some("Your level or stats are too low to equip this item.")
}

fn equipment_required_level(item_id: i32, archive: &NxArchive) -> Option<i64> {
    let category = character_equip_category(item_id)?;
    let info_path = format!("{category}/0{item_id}.img/info");
    let info = archive.root().resolve(&info_path).ok().flatten()?;
    Some(integer_property(&info, "reqLevel").unwrap_or(0))
}

fn job_can_equip(job_id: u16, required_job: u16) -> bool {
    match required_job {
        0 => true,
        1 => job_id / 100 == 1 || job_id / 100 >= 20,
        2 => job_id / 100 == 2,
        4 => job_id / 100 == 3,
        8 => job_id / 100 == 4,
        16 => job_id / 100 == 5,
        _ => false,
    }
}

fn inventory_item_icon(
    item_id: i32,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
) -> Option<(u32, u32, Arc<[u8]>)> {
    if let Some(cached) = cache.get(&item_id) {
        return cached.clone();
    }
    let resolved = inventory_icon_path(item_id).and_then(|(is_character, path)| {
        let archive = if is_character {
            character_archive
        } else {
            item_archive
        };
        let icon = archive.root().resolve(&path).ok().flatten()?;
        let art = first_bitmap_node(&icon).ok().flatten()?;
        let bitmap = art.bitmap().ok().flatten()?;
        let (width, height) = (u32::from(bitmap.width()), u32::from(bitmap.height()));
        if width == 0 || height == 0 || width > 256 || height > 256 {
            return None;
        }
        let pixels = bitmap.decode_bgra_shared().ok()?;
        Some((width, height, pixels))
    });
    if cache.len() >= 256 {
        cache.clear();
    }
    cache.insert(item_id, resolved.clone());
    resolved
}

fn inventory_item_icon_offset(
    item_id: i32,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
) -> Option<(i32, i32)> {
    let (is_character, path) = inventory_icon_path(item_id)?;
    let archive = if is_character {
        character_archive
    } else {
        item_archive
    };
    let icon = archive.root().resolve(&path).ok().flatten()?;
    let origin = icon.child("origin").ok().flatten()?.value().ok()?;
    let NodeValue::Vector { x, y } = origin else {
        return None;
    };
    // Item icons are drawn bottom-aligned to a 32 px box by the legacy client.
    Some((-x, 32 - y))
}

fn load_cash_products(
    etc_archive: &NxArchive,
    string_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
) -> Vec<CashShopProduct> {
    let Ok(Some(commodity_root)) = etc_archive.root().resolve("Commodity.img") else {
        return Vec::new();
    };
    let Ok(entries) = commodity_root.children() else {
        return Vec::new();
    };
    let mut products = Vec::new();
    for entry in entries {
        let Some(sn) = cash_commodity_integer(&entry, "SN") else {
            continue;
        };
        let Some(item_id) = cash_commodity_integer(&entry, "ItemId") else {
            continue;
        };
        if sn <= 0 || item_id <= 0 || cash_commodity_integer(&entry, "OnSale") != Some(1) {
            continue;
        }
        let Some(price) = cash_commodity_integer(&entry, "Price") else {
            continue;
        };
        let Some(count) = cash_commodity_integer(&entry, "Count")
            .and_then(|count| u16::try_from(count).ok())
            .filter(|count| *count > 0)
        else {
            continue;
        };
        let Some((is_character_item, icon_path)) = inventory_icon_path(item_id) else {
            continue;
        };
        let icon_archive = if is_character_item {
            character_archive
        } else {
            item_archive
        };
        if icon_archive
            .root()
            .resolve(&icon_path)
            .ok()
            .flatten()
            .is_none()
            || item_string_node(item_id, string_archive, "name").is_none()
        {
            continue;
        }
        products.push(CashShopProduct {
            sn,
            item_id,
            price: price.max(0),
            count,
            period: cash_commodity_integer(&entry, "Period").unwrap_or(0),
            gender: cash_commodity_integer(&entry, "Gender").unwrap_or(2),
            priority: cash_commodity_integer(&entry, "Priority").unwrap_or(i32::MAX),
        });
    }
    products.sort_by_key(|product| (product.priority, product.sn));
    products
}

fn cash_commodity_integer(node: &openstory_assets::NxNode, name: &str) -> Option<i32> {
    let value = node.child(name).ok().flatten()?.value().ok()?;
    match value {
        NodeValue::Integer(value) => i32::try_from(value).ok(),
        _ => None,
    }
}

fn inventory_icon_path(item_id: i32) -> Option<(bool, String)> {
    if item_id <= 0 {
        return None;
    }
    let item_file = format!("0{item_id}.img");
    let item_node = format!("0{item_id}");
    let item_prefix = item_id / 10_000;
    let icon_node = if matches!(item_prefix, 238 | 239) {
        "iconRaw"
    } else {
        "icon"
    };
    let (is_character, path) = match item_id / 1_000_000 {
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
            (true, format!("{category}/{item_file}/info/{icon_node}"))
        }
        2 => (
            false,
            format!("Consume/0{item_prefix}.img/{item_node}/info/{icon_node}"),
        ),
        3 => (
            false,
            format!("Install/0{item_prefix}.img/{item_node}/info/{icon_node}"),
        ),
        4 => (
            false,
            format!("Etc/0{item_prefix}.img/{item_node}/info/{icon_node}"),
        ),
        5 if item_prefix == 500 => (false, format!("Pet/{item_id}.img/info/{icon_node}")),
        5 => (
            false,
            format!("Cash/0{item_prefix}.img/{item_node}/info/{icon_node}"),
        ),
        _ => return None,
    };
    Some((is_character, path))
}

fn inventory_item_info_node(
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    item_id: i32,
) -> Option<openstory_assets::NxNode> {
    let (is_character, icon_path) = inventory_icon_path(item_id)?;
    let info_path = icon_path.rsplit_once('/')?.0;
    let archive = if is_character {
        character_archive
    } else {
        item_archive
    };
    archive.root().resolve(info_path).ok().flatten()
}

fn inventory_item_property_truthy(node: &openstory_assets::NxNode, field: &str) -> bool {
    let Some(value) = node
        .child(field)
        .ok()
        .flatten()
        .and_then(|value| value.value().ok())
    else {
        return false;
    };
    match value {
        NodeValue::Integer(value) => value != 0,
        NodeValue::Real(value) => value != 0.0,
        NodeValue::String(value) => {
            !value.trim().is_empty() && !matches!(value.trim(), "0" | "false" | "False")
        }
        _ => false,
    }
}

fn item_string_path(item_id: i32) -> Option<String> {
    if item_id <= 0 {
        return None;
    }
    let item_prefix = item_id / 10_000;
    Some(match item_id / 1_000_000 {
        1 => {
            let category = match item_prefix {
                180..=183 => "PetEquip",
                190 | 191 => "Taming",
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
            format!("Eqp.img/Eqp/{category}/{item_id}")
        }
        2 => format!("Consume.img/{item_id}"),
        3 => format!("Ins.img/{item_id}"),
        4 => format!("Etc.img/Etc/{item_id}"),
        5 if item_prefix == 500 => format!("Pet.img/{item_id}"),
        5 => format!("Cash.img/{item_id}"),
        _ => return None,
    })
}

fn item_string_node(item_id: i32, archive: &NxArchive, property: &str) -> Option<NxNode> {
    let path = item_string_path(item_id)?;
    let root = archive.root();
    root.resolve(&format!("{path}/{property}"))
        .ok()
        .flatten()
        .or_else(|| {
            // Mount strings use Taming in v83. Also accept the Character.nx category name.
            let mount_id = path.strip_prefix("Eqp.img/Eqp/Taming/")?;
            root.resolve(&format!("Eqp.img/Eqp/TamingMob/{mount_id}/{property}"))
                .ok()
                .flatten()
        })
}

fn item_string_property(item_id: i32, archive: &NxArchive, property: &str) -> Option<String> {
    match item_string_node(item_id, archive, property)?.value().ok()? {
        NodeValue::String(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

fn item_name(
    item_id: i32,
    string_archive: &NxArchive,
    cache: &mut HashMap<i32, Option<String>>,
) -> String {
    if let Some(name) = cache.get(&item_id) {
        return name.clone().unwrap_or_else(|| format!("Item #{item_id}"));
    }
    let resolved = item_string_property(item_id, string_archive, "name");
    if cache.len() >= 512 {
        cache.clear();
    }
    cache.insert(item_id, resolved.clone());
    resolved.unwrap_or_else(|| format!("Item #{item_id}"))
}

fn append_owl_item_names(items: &NxNode, output: &mut Vec<(String, i32)>) {
    for item in items.children().unwrap_or_default() {
        if output.len() >= 100_000 {
            return;
        }
        let Some(item_id) = item.name().ok().and_then(|name| name.parse::<i32>().ok()) else {
            continue;
        };
        if item_id <= 0 {
            continue;
        }
        let name = item
            .child("name")
            .ok()
            .flatten()
            .and_then(|name| name.value().ok())
            .and_then(|value| match value {
                NodeValue::String(name) if !name.trim().is_empty() => Some(name),
                _ => None,
            });
        if let Some(name) = name {
            output.push((name.to_lowercase(), item_id));
        }
    }
}

fn build_owl_item_name_index(string_archive: &NxArchive) -> Vec<(String, i32)> {
    let root = string_archive.root();
    let mut index = Vec::with_capacity(32_000);
    for path in [
        "Consume.img",
        "Ins.img",
        "Cash.img",
        "Pet.img",
        "Etc.img/Etc",
    ] {
        if let Ok(Some(items)) = root.resolve(path) {
            append_owl_item_names(&items, &mut index);
        }
    }
    if let Ok(Some(equip_categories)) = root.resolve("Eqp.img/Eqp") {
        for category in equip_categories.children().unwrap_or_default() {
            append_owl_item_names(&category, &mut index);
            if index.len() >= 100_000 {
                break;
            }
        }
    }
    index
}

fn composite_inventory_icon(
    canvas: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    source: &[u8],
    source_width: usize,
    source_height: usize,
    x: usize,
    y: usize,
    target_size: usize,
) {
    if source_width == 0 || source_height == 0 || target_size == 0 {
        return;
    }
    for target_y in 0..target_size {
        let destination_y = y + target_y;
        if destination_y >= canvas_height {
            break;
        }
        let source_y = target_y * source_height / target_size;
        for target_x in 0..target_size {
            let destination_x = x + target_x;
            if destination_x >= canvas_width {
                break;
            }
            let source_x = target_x * source_width / target_size;
            let source_index = (source_y * source_width + source_x) * 4;
            let destination_index = (destination_y * canvas_width + destination_x) * 4;
            let alpha = u16::from(source[source_index + 3]);
            let inverse = 255 - alpha;
            for channel in 0..3 {
                canvas[destination_index + channel] = ((u16::from(source[source_index + channel])
                    * alpha
                    + u16::from(canvas[destination_index + channel]) * inverse)
                    / 255) as u8;
            }
            let destination_alpha = u16::from(canvas[destination_index + 3]);
            canvas[destination_index + 3] = (alpha + destination_alpha * inverse / 255) as u8;
        }
    }
}

fn nx_string_value(archive: &NxArchive, path: &str) -> Option<String> {
    let node = archive.root().resolve(path).ok().flatten()?;
    match node.value().ok()? {
        NodeValue::String(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}
