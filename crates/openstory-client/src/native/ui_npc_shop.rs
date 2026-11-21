// v83 shop art supplies every frame, control, selection and list background.
const NPC_SHOP_WIDTH: usize = 463;
const NPC_SHOP_HEIGHT: usize = 339;
const NPC_SHOP_X: f32 = 280.0;
const NPC_SHOP_Y: f32 = 170.0;
const NPC_SHOP_ROWS: usize = 5;
const NPC_SHOP_GRID_TOP: usize = 127;
const NPC_SHOP_ROW_STEP: usize = 40;
const NPC_SHOP_ROW_HEIGHT: usize = 35;
const NPC_SHOP_SCROLL_BOTTOM: usize =
    NPC_SHOP_GRID_TOP + (NPC_SHOP_ROWS - 1) * NPC_SHOP_ROW_STEP + NPC_SHOP_ROW_HEIGHT;
const NPC_SHOP_SCROLL_X: [usize; 2] = [209, 440];
const NPC_SHOP_SCROLL_WIDTH: usize = 15;
const NPC_SHOP_SCROLL_ARROW: usize = 13;
const NPC_SHOP_SCROLL_THUMB: usize = 25;

// Buy and sell share a baseline; the upper controls share their right edges.
// Both painting and hit testing use these anchors and the NX bitmap dimensions.
const NPC_SHOP_BUTTONS: [(&str, i32, i32); 4] = [
    ("BtExit", 138, 12),
    ("BtBuy", 138, 34),
    ("BtSell", 368, 34),
    ("BtRecharge", 368, 12),
];

struct NpcShopUi {
    shop: openstory_protocol::NpcShop,
    // Indices into the full server list. Zero-price / zero-pitch projectile
    // entries remain available to Recharge but are not buy offers.
    all_buy_item_indices: Vec<usize>,
    buy_item_indices: Vec<usize>,
    buy_recommended: bool,
    tab: openstory_protocol::InventoryType,
    buy_scroll: usize,
    sell_scroll: usize,
    buy_selection: Option<usize>,
    sell_selection: Option<u16>,
    pending: bool,
    quantity: Option<NpcShopQuantity>,
    quantity_texture: Option<GpuTexture>,
    hovered_button: Option<&'static str>,
    hovered_item: Option<NpcShopItemHover>,
    last_buy_click: Option<(usize, Instant)>,
    last_sell_click: Option<(u16, Instant)>,
    auto_recharge_slots: VecDeque<i16>,
    auto_recharge_in_flight: bool,
    auto_recharge_notice_sent: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NpcShopItemHover {
    Buy(usize),
    Sell(u16),
}

struct NpcShopQuantity {
    buy: bool,
    slot: i16,
    item_id: i32,
    maximum: i16,
    input: QuantityInput,
    invalid: bool,
}

fn composite_npc_shop_sprite(
    pixels: &mut [u8],
    source: &[u8],
    sw: usize,
    sh: usize,
    rect: SpriteRect,
) {
    let w = rect.width.round().max(1.0) as usize;
    let h = rect.height.round().max(1.0) as usize;
    for y in 0..h {
        for x in 0..w {
            let dx = rect.x.round() as i32 + x as i32;
            let dy = rect.y.round() as i32 + y as i32;
            if dx < 0 || dy < 0 || dx >= NPC_SHOP_WIDTH as i32 || dy >= NPC_SHOP_HEIGHT as i32 {
                continue;
            }
            let src = ((y * sh / h) * sw + x * sw / w) * 4;
            let dst = (dy as usize * NPC_SHOP_WIDTH + dx as usize) * 4;
            let alpha = u32::from(source[src + 3]);
            for c in 0..3 {
                pixels[dst + c] = ((u32::from(source[src + c]) * alpha
                    + u32::from(pixels[dst + c]) * (255 - alpha))
                    / 255) as u8;
            }
        }
    }
}

fn composite_npc_shop_tab(
    pixels: &mut [u8],
    ui: &NxArchive,
    x: usize,
    width: usize,
    selected: bool,
) -> Result<(), String> {
    composite_nx_ui_tab(
        pixels,
        NPC_SHOP_WIDTH,
        NPC_SHOP_HEIGHT,
        ui,
        ui_foundation::UiRect::new(x as f32, 95.0, width as f32, 21.0),
        selected,
    )?;
    Ok(())
}

impl NpcShopUi {
    fn new(shop: openstory_protocol::NpcShop) -> Self {
        let all_buy_item_indices: Vec<usize> = shop
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| (item.price > 0 || item.pitch > 0).then_some(index))
            .collect();
        Self {
            shop,
            buy_item_indices: all_buy_item_indices.clone(),
            all_buy_item_indices,
            buy_recommended: false,
            tab: openstory_protocol::InventoryType::Equip,
            buy_scroll: 0,
            sell_scroll: 0,
            buy_selection: None,
            sell_selection: None,
            pending: false,
            quantity: None,
            quantity_texture: None,
            hovered_button: None,
            hovered_item: None,
            last_buy_click: None,
            last_sell_click: None,
            auto_recharge_slots: VecDeque::new(),
            auto_recharge_in_flight: false,
            auto_recharge_notice_sent: false,
        }
    }

    fn update_buy_item_indices(
        &mut self,
        character: Option<&openstory_protocol::CharacterInfo>,
        inventory: &openstory_protocol::InventorySnapshot,
        character_archive: &NxArchive,
    ) {
        self.buy_item_indices = if self.buy_recommended {
            npc_shop_recommended_buy_indices(
                &self.shop,
                &self.all_buy_item_indices,
                character,
                inventory,
                character_archive,
            )
        } else {
            self.all_buy_item_indices.clone()
        };
    }
}

fn npc_shop_recommended_buy_indices(
    shop: &openstory_protocol::NpcShop,
    buy_item_indices: &[usize],
    character: Option<&openstory_protocol::CharacterInfo>,
    inventory: &openstory_protocol::InventorySnapshot,
    character_archive: &NxArchive,
) -> Vec<usize> {
    let Some(character) = character else {
        return Vec::new();
    };
    buy_item_indices
        .iter()
        .copied()
        .filter(|index| {
            let Some(item) = shop.items.get(*index) else {
                return false;
            };
            item.item_id / 1_000_000 == 1
                && equipment_block_reason(
                    item.item_id,
                    character,
                    inventory,
                    character_archive,
                )
                .is_none()
        })
        .collect()
}

fn npc_shop_buy_quantity(item: &openstory_protocol::NpcShopItem) -> NpcShopQuantity {
    NpcShopQuantity {
        buy: true,
        slot: item.slot,
        item_id: item.item_id,
        maximum: if npc_shop_item_is_equipment(item.item_id) {
            1
        } else if item.buyable == 0 {
            100
        } else {
            item.buyable.max(1)
        },
        input: QuantityInput::new(Some(1)),
        invalid: false,
    }
}

fn npc_shop_item_is_equipment(item_id: i32) -> bool {
    item_id / 1_000_000 == 1
}

fn npc_shop_sell_quantity(item: &openstory_protocol::InventoryItem) -> Option<NpcShopQuantity> {
    if item.count == 0 {
        return None;
    }
    let maximum = npc_shop_sell_maximum(item);
    Some(NpcShopQuantity {
        buy: false,
        slot: i16::try_from(item.slot).ok()?,
        item_id: item.item_id,
        // Rechargeable projectiles are stored as a stack of charges, but a
        // shop transaction sells one complete set at a time.
        maximum,
        input: QuantityInput::new(Some(maximum as u16)),
        invalid: false,
    })
}

fn npc_shop_sell_maximum(item: &openstory_protocol::InventoryItem) -> i16 {
    if matches!(item.item_id / 10_000, 207 | 233) {
        1
    } else {
        i16::try_from(item.count).unwrap_or(i16::MAX)
    }
}

fn npc_shop_recharge_slot(
    state: &NpcShopUi,
    inventory: &openstory_protocol::InventorySnapshot,
) -> Option<i16> {
    let slot = state.sell_selection?;
    let item = inventory
        .items
        .iter()
        .find(|item| item.inventory_type == state.tab && item.slot == slot && item.slot > 0)?;
    if !matches!(item.item_id / 10_000, 207 | 233)
        || !state
            .shop
            .items
            .iter()
            .any(|shop_item| shop_item.item_id == item.item_id && shop_item.rechargeable)
    {
        return None;
    }
    i16::try_from(item.slot).ok()
}

fn npc_shop_auto_recharge_slots(
    shop: &openstory_protocol::NpcShop,
    inventory: &openstory_protocol::InventorySnapshot,
) -> Vec<i16> {
    inventory
        .items
        .iter()
        .filter(|item| {
            item.inventory_type == openstory_protocol::InventoryType::Use
                && item.slot > 0
                && item.item_id / 10_000 == 207
                && shop
                    .items
                    .iter()
                    .any(|shop_item| shop_item.item_id == item.item_id && shop_item.rechargeable)
        })
        .filter_map(|item| i16::try_from(item.slot).ok())
        .collect()
}

fn npc_shop_tooltip_item(
    state: &NpcShopUi,
    inventory: &openstory_protocol::InventorySnapshot,
) -> Option<openstory_protocol::InventoryItem> {
    if state.quantity.is_some() {
        return None;
    }
    match state.hovered_item? {
        NpcShopItemHover::Buy(index) => state
            .buy_item_indices
            .get(index)
            .and_then(|shop_index| state.shop.items.get(*shop_index))
            .map(|item| {
                let inventory_type = match item.item_id / 1_000_000 {
                    1 => openstory_protocol::InventoryType::Equip,
                    2 => openstory_protocol::InventoryType::Use,
                    3 => openstory_protocol::InventoryType::Setup,
                    4 => openstory_protocol::InventoryType::Etc,
                    5 => openstory_protocol::InventoryType::Cash,
                    _ => openstory_protocol::InventoryType::Equip,
                };
                openstory_protocol::InventoryItem {
                    inventory_type,
                    item_id: item.item_id,
                    count: 1,
                    ..Default::default()
                }
            }),
        NpcShopItemHover::Sell(slot) => inventory
            .items
            .iter()
            .find(|item| item.inventory_type == state.tab && item.slot == slot)
            .cloned(),
    }
}

fn npc_shop_row_at(x: f32, y: f32, sell: bool) -> Option<usize> {
    let left = if sell { 237.0 } else { 7.0 };
    if !(left..left + 198.0).contains(&x)
        || !(NPC_SHOP_GRID_TOP as f32..NPC_SHOP_SCROLL_BOTTOM as f32).contains(&y)
    {
        return None;
    }
    let offset = (y - NPC_SHOP_GRID_TOP as f32) as usize;
    (offset % NPC_SHOP_ROW_STEP < NPC_SHOP_ROW_HEIGHT).then_some(offset / NPC_SHOP_ROW_STEP)
}

fn npc_shop_button_rect(archive: &NxArchive, name: &str, x: f32, y: f32) -> Option<SpriteRect> {
    let art_name = if name == "BtRecharge" { "BtExit" } else { name };
    let (width, height, ox, oy) =
        nx_dialogue_button_geometry(archive, &format!("UIWindow.img/Shop/{art_name}/normal/0"))?;
    Some(SpriteRect {
        x: x - ox as f32,
        y: y - oy as f32,
        width,
        height,
        blend_mode: BlendMode::Alpha,
    })
}

fn erase_npc_shop_button_label(pixels: &mut [u8], rect: ui_foundation::UiRect) {
    let x = rect.x.round() as usize;
    let y = rect.y.round() as usize;
    let width = rect.width.round() as usize;
    let height = rect.height.round() as usize;
    let left = x + 8;
    let right = x + width.saturating_sub(8);
    if right <= left + 1 || y + height > NPC_SHOP_HEIGHT || right >= NPC_SHOP_WIDTH {
        return;
    }
    for row in 3..height.saturating_sub(3) {
        let pixel_y = y + row;
        let left_index = (pixel_y * NPC_SHOP_WIDTH + left) * 4;
        let right_index = (pixel_y * NPC_SHOP_WIDTH + right) * 4;
        for pixel_x in left + 1..right {
            let distance = pixel_x - left;
            let span = right - left;
            let index = (pixel_y * NPC_SHOP_WIDTH + pixel_x) * 4;
            for channel in 0..3 {
                pixels[index + channel] = ((usize::from(pixels[left_index + channel])
                    * (span - distance)
                    + usize::from(pixels[right_index + channel]) * distance)
                    / span) as u8;
            }
        }
    }
}

fn draw_npc_shop_button_label(
    font: &fontdue::Font,
    pixels: &mut [u8],
    label: &str,
    x: i32,
    y: i32,
    width: usize,
    color: [u8; 3],
) {
    let outline = [62, 54, 47];
    for (offset_x, offset_y) in [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ] {
        draw_dialogue_text(
            font,
            pixels,
            NPC_SHOP_WIDTH,
            NPC_SHOP_HEIGHT,
            label,
            x + offset_x,
            y + offset_y,
            10.0,
            width,
            12,
            outline,
        );
    }
    draw_dialogue_text(
        font,
        pixels,
        NPC_SHOP_WIDTH,
        NPC_SHOP_HEIGHT,
        label,
        x,
        y,
        10.0,
        width,
        12,
        color,
    );
}

fn render_npc_shop_panel(
    font: &fontdue::Font,
    state: &NpcShopUi,
    inventory: &openstory_protocol::InventorySnapshot,
    character: Option<&openstory_protocol::CharacterInfo>,
    ui: &NxArchive,
    npcs: &NxArchive,
    items: &NxArchive,
    characters: &NxArchive,
    strings: &NxArchive,
    icons: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    names: &mut HashMap<i32, Option<String>>,
) -> Result<Vec<u8>, String> {
    let mut pixels = vec![0; NPC_SHOP_WIDTH * NPC_SHOP_HEIGHT * 4];
    NxDialog::new(&mut pixels, NPC_SHOP_WIDTH, NPC_SHOP_HEIGHT, ui).frame(
        "UIWindow.img/Shop/backgrnd",
        0,
        0,
    )?;
    draw_storage_keeper(
        &mut pixels,
        NPC_SHOP_WIDTH,
        NPC_SHOP_HEIGHT,
        npcs,
        state.shop.npc_id,
        52,
        76,
    );
    if let Some(character) = character {
        let mut sprites = Vec::new();
        append_character_preview(
            &mut sprites,
            characters,
            &character.look,
            288.0,
            76.0,
            0.85,
            false,
            &mut MapArtCache::default(),
        )?;
        for sprite in sprites {
            composite_npc_shop_sprite(
                &mut pixels,
                &sprite.pixels,
                sprite.texture_width as usize,
                sprite.texture_height as usize,
                sprite.rect,
            );
        }
    }
    let mut dialog = NxDialog::new(&mut pixels, NPC_SHOP_WIDTH, NPC_SHOP_HEIGHT, ui);
    let mut recharge_button = None;
    for (name, x, y) in NPC_SHOP_BUTTONS {
        let enabled = match name {
            "BtBuy" => state.buy_selection.is_some() && !state.pending,
            "BtSell" => state.sell_selection.is_some() && !state.pending,
            "BtRecharge" => !state.pending && npc_shop_recharge_slot(state, inventory).is_some(),
            _ => true,
        };
        let variant = if !enabled {
            "disabled"
        } else if state.hovered_button == Some(name) {
            "mouseOver"
        } else {
            "normal"
        };
        let art_name = if name == "BtRecharge" { "BtExit" } else { name };
        let rect =
            dialog.anchored_button(&format!("UIWindow.img/Shop/{art_name}/{variant}/0"), x, y)?;
        if name == "BtRecharge" {
            recharge_button = Some((rect, enabled));
        }
    }
    drop(dialog);
    if let Some((rect, enabled)) = recharge_button {
        erase_npc_shop_button_label(&mut pixels, rect);
        let label = "RECHARGE";
        let label_font = ui_foundation::bold_font().unwrap_or(font);
        let label_width = ui_foundation::measure_text(label_font, label, 10.0);
        let label_x = (rect.x + (rect.width - label_width) / 2.0).round() as i32;
        draw_npc_shop_button_label(
            label_font,
            &mut pixels,
            label,
            label_x,
            rect.y.round() as i32 + 3,
            rect.width.round() as usize,
            if enabled {
                [255, 255, 255]
            } else {
                [110, 110, 110]
            },
        );
    }
    composite_npc_shop_tab(&mut pixels, ui, 7, 54, !state.buy_recommended)?;
    ui_foundation::draw_tab_label(
        font,
        &mut pixels,
        NPC_SHOP_WIDTH,
        NPC_SHOP_HEIGHT,
        "All",
        ui_foundation::UiRect::new(7.0, 95.0, 54.0, 21.0),
    );
    composite_npc_shop_tab(&mut pixels, ui, 61, 100, state.buy_recommended)?;
    ui_foundation::draw_tab_label(
        font,
        &mut pixels,
        NPC_SHOP_WIDTH,
        NPC_SHOP_HEIGHT,
        "Recommended",
        ui_foundation::UiRect::new(61.0, 95.0, 100.0, 21.0),
    );
    let sell_tab_labels = ["Equip", "Use", "Setup", "Etc", "Cash"];
    for (index, tab) in storage_tabs().into_iter().enumerate() {
        let bounds = ui_foundation::npc_shop_sell_tab(index);
        composite_npc_shop_tab(
            &mut pixels,
            ui,
            bounds.x as usize,
            bounds.width as usize,
            state.tab == tab,
        )?;
        ui_foundation::draw_tab_label(
            font,
            &mut pixels,
            NPC_SHOP_WIDTH,
            NPC_SHOP_HEIGHT,
            sell_tab_labels[index],
            bounds,
        );
    }
    let sell: Vec<_> = inventory
        .items
        .iter()
        .filter(|i| i.inventory_type == state.tab && i.slot > 0)
        .collect();
    for (right, count, offset) in [
        (false, state.buy_item_indices.len(), state.buy_scroll),
        (true, sell.len(), state.sell_scroll),
    ] {
        let x = NPC_SHOP_SCROLL_X[usize::from(right)];
        let maximum = count.saturating_sub(NPC_SHOP_ROWS);
        let mode = if maximum > 0 { "enabled" } else { "disabled" };
        let track_top = NPC_SHOP_GRID_TOP + NPC_SHOP_SCROLL_ARROW;
        let track_bottom = NPC_SHOP_SCROLL_BOTTOM - NPC_SHOP_SCROLL_ARROW;
        // Tile the NX track and keep the last tile inside the arrow bounds.
        for y in (track_top..track_bottom).step_by(NPC_SHOP_SCROLL_ARROW) {
            composite_nx_ui_bitmap_scaled_to(
                &mut pixels,
                NPC_SHOP_WIDTH,
                NPC_SHOP_HEIGHT,
                ui,
                &format!("Basic.img/VScr4/{mode}/base"),
                x,
                y,
                NPC_SHOP_SCROLL_WIDTH,
                NPC_SHOP_SCROLL_ARROW.min(track_bottom - y),
            )?;
        }
        for (part, y) in [
            (
                if maximum > 0 { "prev0" } else { "prev" },
                NPC_SHOP_GRID_TOP,
            ),
            (if maximum > 0 { "next0" } else { "next" }, track_bottom),
        ] {
            composite_nx_ui_bitmap(
                &mut pixels,
                NPC_SHOP_WIDTH,
                NPC_SHOP_HEIGHT,
                ui,
                &format!("Basic.img/VScr4/{mode}/{part}"),
                x as i32,
                y as i32,
            )?;
        }
        if maximum > 0 {
            let travel = track_bottom - track_top - NPC_SHOP_SCROLL_THUMB;
            composite_nx_ui_bitmap(
                &mut pixels,
                NPC_SHOP_WIDTH,
                NPC_SHOP_HEIGHT,
                ui,
                "Basic.img/VScr4/enabled/thumb0",
                x as i32,
                (track_top + travel * offset.min(maximum) / maximum) as i32,
            )?;
        }
        for row in 0..NPC_SHOP_ROWS {
            let index = offset + row;
            let (id, quantity, price, selected) = if right {
                let Some(item) = sell.get(index) else {
                    break;
                };
                (
                    item.item_id,
                    item.count,
                    None,
                    state.sell_selection == Some(item.slot),
                )
            } else {
                let Some(shop_index) = state.buy_item_indices.get(index) else {
                    break;
                };
                let Some(item) = state.shop.items.get(*shop_index) else {
                    break;
                };
                (
                    item.item_id,
                    0,
                    Some(if item.price == 0 && item.charge_price > 0 {
                        i32::from(item.charge_price)
                    } else {
                        item.price
                    }),
                    state.buy_selection == Some(index),
                )
            };
            let x = if right { 237 } else { 7 };
            let y = NPC_SHOP_GRID_TOP + row * NPC_SHOP_ROW_STEP;
            if selected {
                // Fit the NX selection to the grey row's right edge (205/435),
                // preserving its border instead of painting into the gutter.
                composite_nx_ui_bitmap_scaled_to(
                    &mut pixels,
                    NPC_SHOP_WIDTH,
                    NPC_SHOP_HEIGHT,
                    ui,
                    "UIWindow.img/Shop/select",
                    x + 40,
                    y,
                    158,
                    NPC_SHOP_ROW_HEIGHT,
                )?;
            }
            if let Some((w, h, icon)) = inventory_item_icon(id, items, characters, icons) {
                composite_inventory_icon(
                    &mut pixels,
                    NPC_SHOP_WIDTH,
                    NPC_SHOP_HEIGHT,
                    &icon,
                    w as usize,
                    h as usize,
                    x,
                    y + 2,
                    32,
                );
            }
            let name = item_name(id, strings, names);
            let name = ui_foundation::ellipsize(font, &name, 12.0, 155.0);
            draw_dialogue_text(
                font,
                &mut pixels,
                NPC_SHOP_WIDTH,
                NPC_SHOP_HEIGHT,
                &name,
                (x + 41) as i32,
                (y + 1) as i32,
                12.0,
                155,
                17,
                [35, 35, 35],
            );
            if let Some(price) = price {
                if state.shop.currency_item_id > 0 {
                    if let Some((w, h, icon)) =
                        inventory_item_icon(state.shop.currency_item_id, items, characters, icons)
                    {
                        composite_inventory_icon(
                            &mut pixels,
                            NPC_SHOP_WIDTH,
                            NPC_SHOP_HEIGHT,
                            &icon,
                            w as usize,
                            h as usize,
                            x + 43,
                            y + 21,
                            12,
                        );
                    }
                } else {
                    composite_nx_ui_bitmap(
                        &mut pixels,
                        NPC_SHOP_WIDTH,
                        NPC_SHOP_HEIGHT,
                        ui,
                        "UIWindow.img/Shop/meso",
                        (x + 43) as i32,
                        (y + 21) as i32,
                    )?;
                }
                let price_label = if state.shop.currency_item_id > 0 {
                    price.to_string()
                } else {
                    format_meso_amount(i64::from(price))
                };
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    NPC_SHOP_WIDTH,
                    NPC_SHOP_HEIGHT,
                    &price_label,
                    (x + 59) as i32,
                    (y + 19) as i32,
                    12.0,
                    135,
                    17,
                    [35, 35, 35],
                );
            } else {
                let price = inventory_item_info_node(items, characters, id)
                    .and_then(|info| info.child("price").ok().flatten())
                    .and_then(|node| node.value().ok())
                    .and_then(|value| match value {
                        NodeValue::Integer(v) => Some(v.max(0)),
                        NodeValue::Real(v) if v.is_finite() && v >= 0.0 => Some(v.floor() as i64),
                        _ => None,
                    });
                if let Some(price) = price {
                    composite_nx_ui_bitmap(
                        &mut pixels,
                        NPC_SHOP_WIDTH,
                        NPC_SHOP_HEIGHT,
                        ui,
                        "UIWindow.img/Shop/meso",
                        (x + 43) as i32,
                        (y + 21) as i32,
                    )?;
                    let label = format_meso_amount(price);
                    draw_dialogue_text(
                        font,
                        &mut pixels,
                        NPC_SHOP_WIDTH,
                        NPC_SHOP_HEIGHT,
                        &label,
                        (x + 59) as i32,
                        (y + 19) as i32,
                        12.0,
                        135,
                        17,
                        [35, 35, 35],
                    );
                }
                if state.tab != openstory_protocol::InventoryType::Equip {
                    draw_item_stack_count(
                        &mut pixels,
                        NPC_SHOP_WIDTH,
                        NPC_SHOP_HEIGHT,
                        ui,
                        quantity,
                        x as i32,
                        (y + 22) as i32,
                        34,
                    )?;
                }
            }
        }
    }
    if state.buy_recommended && state.buy_item_indices.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            NPC_SHOP_WIDTH,
            NPC_SHOP_HEIGHT,
            "No recommended items",
            48,
            180,
            12.0,
            160,
            16,
            [105, 105, 105],
        );
    }
    let balance = if state.shop.currency_item_id > 0 {
        let currency_item_id = state.shop.currency_item_id;
        if let Some((w, h, icon)) = inventory_item_icon(currency_item_id, items, characters, icons)
        {
            composite_inventory_icon(
                &mut pixels,
                NPC_SHOP_WIDTH,
                NPC_SHOP_HEIGHT,
                &icon,
                w as usize,
                h as usize,
                349,
                64,
                12,
            );
        } else {
            composite_nx_ui_bitmap(
                &mut pixels,
                NPC_SHOP_WIDTH,
                NPC_SHOP_HEIGHT,
                ui,
                "UIWindow.img/Shop/token",
                349,
                64,
            )?;
        }
        inventory
            .items
            .iter()
            .filter(|item| item.item_id == currency_item_id)
            .map(|item| i64::from(item.count))
            .sum()
    } else {
        inventory.meso
    };
    draw_inventory_meso_amount(
        font,
        &mut pixels,
        NPC_SHOP_WIDTH,
        NPC_SHOP_HEIGHT,
        balance,
        449,
        64,
        86,
    );
    Ok(pixels)
}

fn render_npc_shop_quantity(
    font: &fontdue::Font,
    quantity: &NpcShopQuantity,
    ui: &NxArchive,
    items: &NxArchive,
    characters: &NxArchive,
    strings: &NxArchive,
    icons: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    names: &mut HashMap<i32, Option<String>>,
) -> Result<Vec<u8>, String> {
    let icon = inventory_item_icon(quantity.item_id, items, characters, icons);
    let title = if quantity.buy {
        "BUY ITEM"
    } else {
        "SELL ITEM"
    };
    let message = if icon.is_some() {
        String::new()
    } else {
        let name = item_name(quantity.item_id, strings, names);
        ui_foundation::ellipsize(font, &name, 10.0, (QUANTITY_WINDOW_WIDTH - 20) as f32)
    };
    let notice = quantity
        .invalid
        .then(|| format!("Enter a quantity from 1 to {}.", quantity.maximum));
    render_quantity_window(
        ui,
        font,
        title,
        &message,
        icon.as_ref().map(QuantityIcon::Item),
        Some((&quantity.input, 1, quantity.maximum as u16)),
        notice.as_deref(),
        InventoryDropStep::Quantity,
    )
}

impl ClientApp {
    fn start_next_npc_shop_auto_recharge(&mut self) {
        let slot = self.npc_shop.as_mut().and_then(|state| {
            if state.pending || state.auto_recharge_in_flight {
                return None;
            }
            let slot = state.auto_recharge_slots.pop_front()?;
            state.pending = true;
            state.auto_recharge_in_flight = true;
            Some(slot)
        });
        let Some(slot) = slot else {
            return;
        };
        let packet = match openstory_protocol::npc_shop_recharge(slot) {
            Ok(packet) => packet,
            Err(error) => {
                self.clear_npc_shop_auto_recharge();
                eprintln!("OpenStoryRS: could not build automatic shop recharge: {error}");
                return;
            }
        };
        let Some(session) = self.session.as_ref() else {
            self.clear_npc_shop_auto_recharge();
            return;
        };
        if let Err(error) = session.send_packet(packet) {
            self.clear_npc_shop_auto_recharge();
            eprintln!("OpenStoryRS: automatic shop recharge failed: {error}");
        }
    }

    fn clear_npc_shop_auto_recharge(&mut self) {
        if let Some(state) = self.npc_shop.as_mut() {
            state.pending = false;
            state.auto_recharge_in_flight = false;
            state.auto_recharge_slots.clear();
        }
    }

    fn update_npc_shop_hover(&mut self) {
        let Some(state) = self.npc_shop.as_ref() else {
            return;
        };
        let hovered_button = self.cursor.and_then(|cursor| {
            NPC_SHOP_BUTTONS
                .into_iter()
                .find(|(name, x, y)| {
                    npc_shop_button_rect(&self.legacy_ui_archive, name, *x as f32, *y as f32)
                        .is_some_and(|r| {
                            rect_contains(
                                r,
                                Point {
                                    x: cursor.x - NPC_SHOP_X,
                                    y: cursor.y - NPC_SHOP_Y,
                                },
                            )
                        })
                })
                .map(|(name, _, _)| name)
        });
        let hovered_item = if state.quantity.is_some() {
            None
        } else {
            self.cursor.and_then(|cursor| {
                let x = cursor.x - NPC_SHOP_X;
                let y = cursor.y - NPC_SHOP_Y;
                if let Some(row) = npc_shop_row_at(x, y, false) {
                    let index = state.buy_scroll + row;
                    (index < state.buy_item_indices.len()).then_some(NpcShopItemHover::Buy(index))
                } else if let Some(row) = npc_shop_row_at(x, y, true) {
                    self.inventory
                        .as_ref()
                        .and_then(|inventory| {
                            inventory
                                .items
                                .iter()
                                .filter(|item| item.inventory_type == state.tab && item.slot > 0)
                                .nth(state.sell_scroll + row)
                        })
                        .map(|item| NpcShopItemHover::Sell(item.slot))
                } else {
                    None
                }
            })
        };
        if hovered_button != state.hovered_button || hovered_item != state.hovered_item {
            if let Some(state) = self.npc_shop.as_mut() {
                state.hovered_button = hovered_button;
                state.hovered_item = hovered_item;
            }
            self.refresh_npc_shop_texture();
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }
    }

    fn refresh_npc_shop_texture(&mut self) {
        let (Some(state), Some(inventory)) = (self.npc_shop.as_mut(), self.inventory.as_ref())
        else {
            self.npc_shop_texture = None;
            return;
        };
        state.update_buy_item_indices(
            self.active_character.as_ref(),
            inventory,
            &self.character_archive,
        );
        state.buy_scroll = state
            .buy_scroll
            .min(state.buy_item_indices.len().saturating_sub(NPC_SHOP_ROWS));
        let sell: Vec<_> = inventory
            .items
            .iter()
            .filter(|i| i.inventory_type == state.tab && i.slot > 0)
            .collect();
        state.sell_scroll = state
            .sell_scroll
            .min(sell.len().saturating_sub(NPC_SHOP_ROWS));
        if state
            .sell_selection
            .is_some_and(|slot| !sell.iter().any(|i| i.slot == slot))
        {
            state.sell_selection = None;
        }
        if state
            .buy_selection
            .is_some_and(|index| index >= state.buy_item_indices.len())
        {
            state.buy_selection = None;
            state.last_buy_click = None;
        }
        if let Some(quantity) = state.quantity.as_mut().filter(|quantity| !quantity.buy) {
            let current = sell.iter().find(|item| {
                i16::try_from(item.slot).ok() == Some(quantity.slot)
                    && item.item_id == quantity.item_id
            });
            if let Some(item) = current.filter(|item| item.count > 0) {
                let maximum = npc_shop_sell_maximum(item);
                if quantity.maximum != maximum {
                    quantity.maximum = maximum;
                    quantity.input.clamp_to_max(maximum as u16);
                }
            } else {
                state.quantity = None;
            }
        }
        let pixels = render_npc_shop_panel(
            &self.font,
            state,
            inventory,
            self.active_character.as_ref(),
            &self.legacy_ui_archive,
            &self.npc_archive,
            &self.item_archive,
            &self.character_archive,
            &self.string_archive,
            &mut self.inventory_icon_cache,
            &mut self.item_name_cache,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = pixels.and_then(|pixels| {
            if let Some(texture) = self.npc_shop_texture.as_ref() {
                renderer.update_bgra(texture, &pixels).map(|()| None)
            } else {
                renderer
                    .upload_bgra(NPC_SHOP_WIDTH as u32, NPC_SHOP_HEIGHT as u32, &pixels)
                    .map(Some)
            }
            .map_err(|error| error.to_string())
        });
        match result {
            Ok(Some(texture)) => self.npc_shop_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not render NPC shop: {error}"),
        }
        if let Some(quantity) = state.quantity.as_ref() {
            let result = render_npc_shop_quantity(
                &self.font,
                quantity,
                &self.legacy_ui_archive,
                &self.item_archive,
                &self.character_archive,
                &self.string_archive,
                &mut self.inventory_icon_cache,
                &mut self.item_name_cache,
            )
            .and_then(|pixels| {
                update_quantity_window_texture(renderer, &mut state.quantity_texture, &pixels)
            });
            if let Err(error) = result {
                eprintln!("OpenStoryRS: could not render NPC shop quantity: {error}");
            }
        } else {
            state.quantity_texture = None;
        }
        self.refresh_npc_shop_tooltip_texture();
    }

    pub(super) fn refresh_npc_shop_tooltip_texture(&mut self) {
        let item = self.npc_shop.as_ref().and_then(|state| {
            let inventory = self.inventory.as_ref()?;
            npc_shop_tooltip_item(state, inventory)
        });
        let Some(item) = item else {
            self.inventory_tooltip_texture = None;
            return;
        };
        let pixels = render_inventory_tooltip(
            &self.font,
            &item,
            &self.legacy_ui_archive,
            &self.item_archive,
            &self.character_archive,
            &self.string_archive,
            self.active_character
                .as_ref()
                .map(|character| &character.stats),
            self.inventory.as_ref(),
            &mut self.inventory_icon_cache,
            &mut self.item_name_cache,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.inventory_tooltip_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(INVENTORY_TOOLTIP_WIDTH, INVENTORY_TOOLTIP_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.inventory_tooltip_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update NPC shop item tooltip: {error}"),
        }
    }

    fn close_npc_shop(&mut self) {
        if self.npc_shop.take().is_some()
            && let Some(session) = self.session.as_ref()
            && let Err(error) = session.send_packet(openstory_protocol::npc_shop_leave())
        {
            eprintln!("OpenStoryRS: could not leave NPC shop: {error}");
        }
        self.npc_shop_texture = None;
        self.inventory_tooltip_texture = None;
        self.refresh_inventory_texture();
    }

    fn click_npc_shop(&mut self, cursor: Point) {
        let x = cursor.x - NPC_SHOP_X;
        let y = cursor.y - NPC_SHOP_Y;
        let Some(state) = self.npc_shop.as_mut() else {
            return;
        };
        if let Some(quantity) = state.quantity.as_mut() {
            let local = quantity_window_local(cursor, npc_shop_quantity_rect());
            let (ok, cancel) = quantity_window_buttons(false);
            if ok.contains(local.x, local.y) {
                self.confirm_npc_shop_quantity();
            } else if cancel.contains(local.x, local.y) || QUANTITY_CLOSE.contains(local.x, local.y)
            {
                state.quantity = None;
                self.refresh_npc_shop_texture();
            } else if quantity
                .input
                .press(local.x, local.y, 1, quantity.maximum as u16)
            {
                quantity.invalid = false;
                self.refresh_npc_shop_texture();
            }
            return;
        }
        for (name, bx, by) in NPC_SHOP_BUTTONS {
            if !npc_shop_button_rect(&self.legacy_ui_archive, name, bx as f32, by as f32)
                .is_some_and(|r| rect_contains(r, Point { x, y }))
            {
                continue;
            }
            if name == "BtExit" {
                self.close_npc_shop();
                return;
            }
            if state.pending {
                return;
            }
            if name == "BtRecharge" {
                let packet = self
                    .inventory
                    .as_ref()
                    .and_then(|inventory| npc_shop_recharge_slot(state, inventory))
                    .and_then(|slot| openstory_protocol::npc_shop_recharge(slot).ok());
                if let (Some(packet), Some(session)) = (packet, self.session.as_ref()) {
                    match session.send_packet(packet) {
                        Ok(()) => state.pending = true,
                        Err(error) => eprintln!("OpenStoryRS: shop recharge failed: {error}"),
                    }
                }
                self.refresh_npc_shop_texture();
                return;
            }
            if name == "BtBuy" {
                let selected_item = state
                    .buy_selection
                    .and_then(|index| state.buy_item_indices.get(index))
                    .and_then(|shop_index| state.shop.items.get(*shop_index))
                    .map(|item| (item.slot, item.item_id));
                state.hovered_item = None;
                if let Some((slot, item_id)) = selected_item {
                    self.buy_npc_shop_item(slot, item_id);
                } else {
                    self.refresh_npc_shop_texture();
                }
                return;
            } else {
                state.quantity = self
                    .inventory
                    .as_ref()
                    .and_then(|inv| {
                        inv.items.iter().find(|i| {
                            i.inventory_type == state.tab && Some(i.slot) == state.sell_selection
                        })
                    })
                    .and_then(npc_shop_sell_quantity);
            }
            state.hovered_item = None;
            if state
                .quantity
                .as_ref()
                .is_some_and(|quantity| quantity.maximum == 1)
            {
                self.confirm_npc_shop_quantity();
                return;
            }
            self.refresh_npc_shop_texture();
            return;
        }
        if (96.0..115.0).contains(&y) && (7.0..161.0).contains(&x) {
            let recommended = x >= 61.0;
            if state.buy_recommended != recommended {
                state.buy_recommended = recommended;
                state.buy_scroll = 0;
                state.buy_selection = None;
                state.last_buy_click = None;
                state.hovered_item = None;
            }
        } else if (95.0..116.0).contains(&y) && (237.0..446.0).contains(&x) {
            if let Some(index) = (0..5).find(|index| {
                ui_foundation::npc_shop_sell_tab(*index).contains(x, y)
            }) {
                state.tab = storage_tabs()[index];
                state.sell_scroll = 0;
                state.sell_selection = None;
                state.last_sell_click = None;
                state.hovered_item = None;
            }
        } else if let Some(row) = npc_shop_row_at(x, y, false) {
            let index = state.buy_scroll + row;
            if index < state.buy_item_indices.len() {
                let now = Instant::now();
                let double_click = state.last_buy_click.is_some_and(|(previous, at)| {
                    previous == index
                        && now.saturating_duration_since(at) <= Duration::from_millis(500)
                });
                state.buy_selection = Some(index);
                state.last_buy_click = if double_click {
                    None
                } else {
                    Some((index, now))
                };
                if double_click {
                    let equipment = state
                        .buy_item_indices
                        .get(index)
                        .and_then(|shop_index| state.shop.items.get(*shop_index))
                        .filter(|item| npc_shop_item_is_equipment(item.item_id))
                        .map(|item| (item.slot, item.item_id));
                    if let Some((slot, item_id)) = equipment {
                        state.quantity = None;
                        state.hovered_item = None;
                        self.buy_npc_shop_item(slot, item_id);
                        return;
                    }
                    state.quantity = state
                        .buy_item_indices
                        .get(index)
                        .and_then(|shop_index| state.shop.items.get(*shop_index))
                        .map(npc_shop_buy_quantity);
                    state.hovered_item = None;
                }
            } else {
                state.buy_selection = None;
                state.last_buy_click = None;
            }
        } else if let Some(row) = npc_shop_row_at(x, y, true) {
            let slot = self
                .inventory
                .as_ref()
                .and_then(|inv| {
                    inv.items
                        .iter()
                        .filter(|i| i.inventory_type == state.tab && i.slot > 0)
                        .nth(state.sell_scroll + row)
                })
                .map(|i| i.slot);
            state.sell_selection = slot;
            if let Some(slot) = slot {
                let now = Instant::now();
                let double_click = state.last_sell_click.is_some_and(|(previous, at)| {
                    previous == slot
                        && now.saturating_duration_since(at) <= Duration::from_millis(500)
                });
                state.last_sell_click = if double_click {
                    None
                } else {
                    Some((slot, now))
                };
                if double_click {
                    state.quantity =
                        self.inventory
                            .as_ref()
                            .and_then(|inventory| {
                                inventory.items.iter().find(|item| {
                                    item.inventory_type == state.tab && item.slot == slot
                                })
                            })
                            .and_then(npc_shop_sell_quantity);
                    state.hovered_item = None;
                }
            } else {
                state.last_sell_click = None;
            }
        } else if (NPC_SHOP_GRID_TOP as f32..NPC_SHOP_SCROLL_BOTTOM as f32).contains(&y)
            && NPC_SHOP_SCROLL_X
                .iter()
                .any(|&left| (left as f32..(left + NPC_SHOP_SCROLL_WIDTH) as f32).contains(&x))
        {
            self.scroll_npc_shop(
                cursor,
                if y < (NPC_SHOP_GRID_TOP + NPC_SHOP_SCROLL_BOTTOM) as f32 / 2.0 {
                    1.0
                } else {
                    -1.0
                },
            );
            return;
        }
        if state
            .quantity
            .as_ref()
            .is_some_and(|quantity| quantity.maximum == 1)
        {
            self.confirm_npc_shop_quantity();
        }
        self.refresh_npc_shop_texture();
    }

    fn buy_npc_shop_item(&mut self, slot: i16, item_id: i32) {
        if self.npc_shop.as_ref().is_none_or(|state| state.pending) {
            return;
        }

        let unsupported_currency = self.npc_shop.as_ref().is_some_and(|state| {
            state
                .shop
                .items
                .iter()
                .find(|item| item.slot == slot && item.item_id == item_id)
                .is_some_and(|item| item.price <= 0 && item.pitch <= 0)
        });
        if unsupported_currency {
            self.show_server_notice_popup(
                "This item requires a special currency that's not supported.",
            );
            self.refresh_npc_shop_texture();
            return;
        }

        let Some(packet) = openstory_protocol::npc_shop_item_action(slot, item_id, 1, true).ok()
        else {
            return;
        };
        let Some(session) = self.session.as_ref() else {
            return;
        };
        match session.send_packet(packet) {
            Ok(()) => {
                if let Some(state) = self.npc_shop.as_mut() {
                    state.pending = true;
                    state.quantity = None;
                }
            }
            Err(error) => eprintln!("OpenStoryRS: shop action failed: {error}"),
        }
        self.refresh_npc_shop_texture();
    }

    fn confirm_npc_shop_quantity(&mut self) {
        let Some(state) = self.npc_shop.as_mut() else {
            return;
        };
        if state.pending {
            return;
        }
        let Some(quantity) = state.quantity.as_mut() else {
            return;
        };
        let Some(amount) = quantity
            .input
            .value()
            .and_then(|value| i16::try_from(value).ok())
            .filter(|value| (1..=quantity.maximum).contains(value))
        else {
            quantity.invalid = true;
            self.refresh_npc_shop_texture();
            return;
        };
        let unsupported_currency = quantity.buy
            && state
                .shop
                .items
                .iter()
                .find(|item| item.slot == quantity.slot && item.item_id == quantity.item_id)
                .is_some_and(|item| item.price <= 0 && item.pitch <= 0);
        if unsupported_currency {
            state.quantity = None;
            self.show_server_notice_popup(
                "This item requires a special currency that's not supported.",
            );
            self.refresh_npc_shop_texture();
            return;
        }
        // Validate the selected stack again after any intervening inventory updates.
        if !quantity.buy
            && !self.inventory.as_ref().is_some_and(|inv| {
                inv.items.iter().any(|i| {
                    i.inventory_type == state.tab
                        && i.slot == quantity.slot as u16
                        && i.item_id == quantity.item_id
                        && i.count >= amount as u16
                })
            })
        {
            return;
        }
        if let (Ok(packet), Some(session)) = (
            openstory_protocol::npc_shop_item_action(
                quantity.slot,
                quantity.item_id,
                amount,
                quantity.buy,
            ),
            self.session.as_ref(),
        ) {
            match session.send_packet(packet) {
                Ok(()) => {
                    state.pending = true;
                    state.quantity = None;
                }
                Err(error) => eprintln!("OpenStoryRS: shop action failed: {error}"),
            }
        }
        self.refresh_npc_shop_texture();
    }

    fn key_npc_shop(&mut self, event: &winit::event::KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }
        let editing = self.npc_shop.as_ref().is_some_and(|s| s.quantity.is_some());
        match event.physical_key {
            PhysicalKey::Code(KeyCode::Escape) if event.repeat => return,
            PhysicalKey::Code(KeyCode::Escape) if editing => {
                if let Some(s) = self.npc_shop.as_mut() {
                    s.quantity = None;
                }
            }
            PhysicalKey::Code(KeyCode::Escape) => {
                self.close_npc_shop();
                return;
            }
            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) if !event.repeat => {
                self.confirm_npc_shop_quantity();
                return;
            }
            _ => {
                if let Some(quantity) = self.npc_shop.as_mut().and_then(|s| s.quantity.as_mut())
                    && quantity.input.edit_key(event, quantity.maximum as u16)
                {
                    quantity.invalid = false;
                }
            }
        }
        self.refresh_npc_shop_texture();
    }

    fn scroll_npc_shop(&mut self, cursor: Point, amount: f32) {
        let x = cursor.x - NPC_SHOP_X;
        let y = cursor.y - NPC_SHOP_Y;
        if !(0.0..NPC_SHOP_WIDTH as f32).contains(&x)
            || !(NPC_SHOP_GRID_TOP as f32..NPC_SHOP_SCROLL_BOTTOM as f32).contains(&y)
        {
            return;
        }
        if let Some(state) = self.npc_shop.as_mut() {
            if state.quantity.is_some() {
                return;
            }
            let offset = if x < 230.0 {
                &mut state.buy_scroll
            } else {
                &mut state.sell_scroll
            };
            *offset = offset.saturating_add_signed(if amount > 0.0 { -1 } else { 1 });
            state.last_buy_click = None;
            state.last_sell_click = None;
            state.hovered_item = None;
        }
        self.refresh_npc_shop_texture();
        self.update_npc_shop_hover();
    }
}

#[cfg(test)]
mod npc_shop_tests {
    use super::*;
    #[test]
    fn rows_stay_inside_authored_slots() {
        assert_eq!(npc_shop_row_at(7.0, 127.0, false), Some(0));
        assert_eq!(npc_shop_row_at(204.0, 321.0, false), Some(4));
        assert_eq!(npc_shop_row_at(7.0, 162.0, false), None);
        assert_eq!(npc_shop_row_at(209.0, 127.0, false), None);
        assert_eq!(npc_shop_row_at(237.0, 287.0, true), Some(4));
        assert_eq!(npc_shop_row_at(237.0, 322.0, true), None);
    }
}
