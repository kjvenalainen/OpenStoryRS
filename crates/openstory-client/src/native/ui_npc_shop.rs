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
const NPC_SHOP_RECOMMENDED_LEVEL_STEP: i64 = 5;

#[derive(Clone, Copy)]
enum NpcShopScrollSide {
    Buy,
    Sell,
}

#[derive(Clone, Copy)]
struct NpcShopScrollDrag {
    side: NpcShopScrollSide,
    grab_offset: f32,
}

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
    scroll_dragging: Option<NpcShopScrollDrag>,
    last_buy_click: Option<(usize, Instant)>,
    last_sell_click: Option<(u16, Instant)>,
    auto_recharge_slots: VecDeque<i16>,
    auto_recharge_in_flight: bool,
    auto_recharge_notice_sent: bool,
    auto_sell_items: VecDeque<InventorySellMark>,
    auto_sell_in_flight: Option<InventorySellMark>,
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
        let mut all_buy_item_indices: Vec<usize> = shop
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| (item.price > 0 || item.pitch > 0).then_some(index))
            .collect();
        all_buy_item_indices.sort_by_key(|index| {
            npc_shop_item_is_throwing_star(shop.items[*index].item_id)
        });
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
            scroll_dragging: None,
            last_buy_click: None,
            last_sell_click: None,
            auto_recharge_slots: VecDeque::new(),
            auto_recharge_in_flight: false,
            auto_recharge_notice_sent: false,
            auto_sell_items: VecDeque::new(),
            auto_sell_in_flight: None,
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

    fn selected_buy_item(&self) -> Option<&openstory_protocol::NpcShopItem> {
        let index = *self.buy_item_indices.get(self.buy_selection?)?;
        self.shop.items.get(index)
    }

    fn selected_sell_item<'a>(
        &self,
        inventory: &'a openstory_protocol::InventorySnapshot,
    ) -> Option<&'a openstory_protocol::InventoryItem> {
        let slot = self.sell_selection?;
        inventory
            .items
            .iter()
            .find(|item| item.inventory_type == self.tab && item.slot == slot && item.slot > 0)
    }

    fn begin_item_action(&mut self, quantity: NpcShopQuantity) -> Option<NpcShopQuantity> {
        if self.pending {
            return None;
        }
        self.hovered_item = None;
        if quantity.maximum == 1 {
            return Some(quantity);
        }
        self.quantity = Some(quantity);
        None
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
    let current_level = i64::from(character.stats.level);
    let current_level_band =
        current_level.div_euclid(NPC_SHOP_RECOMMENDED_LEVEL_STEP) * NPC_SHOP_RECOMMENDED_LEVEL_STEP;
    let recommended_level_range = current_level_band..=current_level.saturating_add(10);
    buy_item_indices
        .iter()
        .copied()
        .filter(|index| {
            let Some(item) = shop.items.get(*index) else {
                return false;
            };
            npc_shop_item_is_equipment(item.item_id)
                && equipment_recommendation_block_reason(
                    item.item_id,
                    character,
                    inventory,
                    character_archive,
                )
                .is_none()
                && equipment_required_level(item.item_id, character_archive)
                    .is_some_and(|required_level| recommended_level_range.contains(&required_level))
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

fn npc_shop_item_is_throwing_star(item_id: i32) -> bool {
    item_id / 10_000 == 207
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
        maximum,
        input: QuantityInput::new(Some(maximum as u16)),
        invalid: false,
    })
}

fn npc_shop_sell_items(
    inventory: &openstory_protocol::InventorySnapshot,
    tab: openstory_protocol::InventoryType,
) -> Vec<&openstory_protocol::InventoryItem> {
    let mut items: Vec<_> = inventory
        .items
        .iter()
        .filter(|item| item.inventory_type == tab && item.slot > 0)
        .collect();
    items.sort_unstable_by_key(|item| item.slot);
    items
}

fn npc_shop_auto_sell_items(
    inventory: &openstory_protocol::InventorySnapshot,
    marked_items: &HashSet<InventorySellMark>,
) -> VecDeque<InventorySellMark> {
    let mut items: Vec<_> = inventory
        .items
        .iter()
        .filter(|item| item.slot > 0 && item.count > 0)
        .map(inventory_sell_mark)
        .filter(|mark| marked_items.contains(mark))
        .collect();
    items.sort_unstable_by_key(|mark| (mark.inventory_type as u8, mark.slot));
    items.into()
}

fn npc_shop_sell_maximum(item: &openstory_protocol::InventoryItem) -> i16 {
    // Rechargeable projectiles sell as a complete set of charges.
    if npc_shop_item_is_equipment(item.item_id) || matches!(item.item_id / 10_000, 207 | 233) {
        1
    } else {
        i16::try_from(item.count).unwrap_or(i16::MAX)
    }
}

fn npc_shop_recharge_slot(
    state: &NpcShopUi,
    inventory: &openstory_protocol::InventorySnapshot,
) -> Option<i16> {
    let item = state.selected_sell_item(inventory)?;
    if !npc_shop_item_needs_recharge(&state.shop, item) {
        return None;
    }
    i16::try_from(item.slot).ok()
}

fn npc_shop_item_needs_recharge(
    shop: &openstory_protocol::NpcShop,
    item: &openstory_protocol::InventoryItem,
) -> bool {
    item.inventory_type == openstory_protocol::InventoryType::Use
        && matches!(item.item_id / 10_000, 207 | 233)
        && shop.items.iter().any(|offer| {
            // For rechargeable items, buyable is the character's charge limit.
            // The server may omit a reply when a stack is already full.
            offer.item_id == item.item_id
                && offer.rechargeable
                && u16::try_from(offer.buyable).is_ok_and(|capacity| item.count < capacity)
        })
}

fn npc_shop_auto_recharge_slots(
    shop: &openstory_protocol::NpcShop,
    inventory: &openstory_protocol::InventorySnapshot,
) -> Vec<i16> {
    inventory
        .items
        .iter()
        .filter(|item| {
            item.slot > 0
                && item.item_id / 10_000 == 207
                && npc_shop_item_needs_recharge(shop, item)
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

fn npc_shop_button_art(name: &str) -> String {
    let art_name = if name == "BtRecharge" { "BtExit" } else { name };
    format!("UIWindow.img/Shop/{art_name}")
}

fn npc_shop_button_rect(archive: &NxArchive, name: &str, x: f32, y: f32) -> Option<SpriteRect> {
    let art = npc_shop_button_art(name);
    let rect = ui_dialog::NxButton::new(&art, ui_dialog::ButtonState::Normal)
        .anchored_bounds(archive, x, y)?;
    Some(SpriteRect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        blend_mode: BlendMode::Alpha,
    })
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
    use ui_dialog::{ButtonState, NxButton};

    let mut dialog = NxDialog::new(&mut pixels, NPC_SHOP_WIDTH, NPC_SHOP_HEIGHT, ui);
    for (name, x, y) in NPC_SHOP_BUTTONS {
        let enabled = match name {
            "BtBuy" => !state.pending && state.selected_buy_item().is_some(),
            "BtSell" => {
                !state.pending && state.selected_sell_item(inventory).is_some_and(|item| item.count > 0)
            }
            "BtRecharge" => !state.pending && npc_shop_recharge_slot(state, inventory).is_some(),
            _ => true,
        };
        let state = if !enabled {
            ButtonState::Disabled
        } else if state.hovered_button == Some(name) {
            ButtonState::Hovered
        } else {
            ButtonState::Normal
        };
        let art = npc_shop_button_art(name);
        let button = NxButton::new(&art, state);
        let bounds = NxButton::new(&art, ButtonState::Normal)
            .anchored_bounds(ui, x as f32, y as f32)
            .ok_or_else(|| format!("missing NX shop button {art}"))?;
        if name == "BtRecharge" {
            dialog.labeled_button(bounds, button, font, "Recharge")?;
        } else {
            dialog.button(bounds, button)?;
        }
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
    let sell = npc_shop_sell_items(inventory, state.tab);
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
    fn start_next_npc_shop_auto_sell(&mut self) {
        let (inventory, marked_items) = (&self.inventory, &self.marked_inventory_items);
        let mut next = None;
        if let (Some(inventory), Some(state)) = (inventory.as_ref(), self.npc_shop.as_mut())
            && !state.pending
            && state.auto_sell_in_flight.is_none()
        {
            while let Some(mark) = state.auto_sell_items.pop_front() {
                if !marked_items.contains(&mark) {
                    continue;
                }
                let Some(item) = inventory.items.iter().find(|item| {
                    item.inventory_type == mark.inventory_type
                        && item.slot == mark.slot
                        && item.item_id == mark.item_id
                        && item.cash_id == mark.cash_id
                        && item.count > 0
                }) else {
                    continue;
                };
                let Some(quantity) = npc_shop_sell_quantity(item) else {
                    continue;
                };
                state.pending = true;
                state.auto_sell_in_flight = Some(mark);
                next = Some((mark, quantity.maximum));
                break;
            }
        }
        let Some((mark, quantity)) = next else {
            self.start_next_npc_shop_auto_recharge();
            return;
        };
        let packet = i16::try_from(mark.slot).ok().and_then(|slot| {
            openstory_protocol::npc_shop_item_action(slot, mark.item_id, quantity, false).ok()
        });
        let Some(packet) = packet else {
            if let Some(state) = self.npc_shop.as_mut() {
                state.pending = false;
                state.auto_sell_in_flight = None;
            }
            self.start_next_npc_shop_auto_sell();
            return;
        };
        let Some(session) = self.session.as_ref() else {
            self.clear_npc_shop_auto_sell();
            return;
        };
        if let Err(error) = session.send_packet(packet) {
            self.clear_npc_shop_auto_sell();
            eprintln!("OpenStoryRS: automatic shop sale failed: {error}");
        }
    }

    fn clear_npc_shop_auto_sell(&mut self) {
        if let Some(state) = self.npc_shop.as_mut() {
            state.pending = false;
            state.auto_sell_in_flight = None;
            state.auto_sell_items.clear();
        }
    }

    fn start_next_npc_shop_auto_recharge(&mut self) {
        let inventory = self.inventory.as_ref();
        let slot = self.npc_shop.as_mut().and_then(|state| {
            if state.pending || state.auto_recharge_in_flight {
                return None;
            }
            let inventory = inventory?;
            let slot = std::iter::from_fn(|| state.auto_recharge_slots.pop_front()).find(|slot| {
                inventory.items.iter().any(|item| {
                    item.slot == *slot as u16
                        && item.item_id / 10_000 == 207
                        && npc_shop_item_needs_recharge(&state.shop, item)
                })
            })?;
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
        let cursor = self
            .cursor
            .filter(|cursor| {
                self.topmost_dialog_window_at(*cursor) == Some(DialogWindow::NpcShop)
            })
            .map(|cursor| self.dialog_window_input_cursor(cursor));
        let hovered_button = cursor.and_then(|cursor| {
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
            cursor.and_then(|cursor| {
                let x = cursor.x - NPC_SHOP_X;
                let y = cursor.y - NPC_SHOP_Y;
                if let Some(row) = npc_shop_row_at(x, y, false) {
                    let index = state.buy_scroll + row;
                    (index < state.buy_item_indices.len()).then_some(NpcShopItemHover::Buy(index))
                } else if let Some(row) = npc_shop_row_at(x, y, true) {
                    self.inventory
                        .as_ref()
                        .and_then(|inventory| {
                            npc_shop_sell_items(inventory, state.tab)
                                .into_iter()
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
        let sell = npc_shop_sell_items(inventory, state.tab);
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
        self.update_item_tooltip(item);
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
        self.inventory_comparison_tooltip_texture = None;
        self.refresh_inventory_texture();
    }

    fn click_npc_shop_scrollbar(&mut self, x: f32, y: f32) -> bool {
        let Some(side) = NPC_SHOP_SCROLL_X
            .iter()
            .enumerate()
            .find_map(|(index, &left)| {
                (left as f32..(left + NPC_SHOP_SCROLL_WIDTH) as f32)
                    .contains(&x)
                    .then_some(if index == 0 {
                        NpcShopScrollSide::Buy
                    } else {
                        NpcShopScrollSide::Sell
                    })
            })
        else {
            return false;
        };
        if !(NPC_SHOP_GRID_TOP as f32..NPC_SHOP_SCROLL_BOTTOM as f32).contains(&y) {
            return false;
        }
        let Some(state) = self.npc_shop.as_ref() else {
            return false;
        };
        if state.quantity.is_some() {
            return false;
        }
        let count = match side {
            NpcShopScrollSide::Buy => state.buy_item_indices.len(),
            NpcShopScrollSide::Sell => self
                .inventory
                .as_ref()
                .map_or(0, |inventory| npc_shop_sell_items(inventory, state.tab).len()),
        };
        let maximum = count.saturating_sub(NPC_SHOP_ROWS);
        let current = match side {
            NpcShopScrollSide::Buy => state.buy_scroll,
            NpcShopScrollSide::Sell => state.sell_scroll,
        }
        .min(maximum);
        if maximum == 0 {
            return true;
        }

        let track_top = NPC_SHOP_GRID_TOP + NPC_SHOP_SCROLL_ARROW;
        let track_bottom = NPC_SHOP_SCROLL_BOTTOM - NPC_SHOP_SCROLL_ARROW;
        let thumb_travel = track_bottom - track_top - NPC_SHOP_SCROLL_THUMB;
        let thumb_top = track_top + thumb_travel * current / maximum;
        let next = if y < track_top as f32 {
            Some(current.saturating_sub(1))
        } else if y >= track_bottom as f32 {
            Some(current.saturating_add(1).min(maximum))
        } else if (thumb_top as f32..(thumb_top + NPC_SHOP_SCROLL_THUMB) as f32).contains(&y) {
            None
        } else if y < thumb_top as f32 {
            Some(current.saturating_sub(NPC_SHOP_ROWS))
        } else {
            Some(current.saturating_add(NPC_SHOP_ROWS).min(maximum))
        };
        if let Some(state) = self.npc_shop.as_mut() {
            match side {
                NpcShopScrollSide::Buy => state.buy_scroll = current,
                NpcShopScrollSide::Sell => state.sell_scroll = current,
            }
            if let Some(next) = next {
                match side {
                    NpcShopScrollSide::Buy => state.buy_scroll = next,
                    NpcShopScrollSide::Sell => state.sell_scroll = next,
                }
                state.scroll_dragging = None;
            } else {
                state.scroll_dragging = Some(NpcShopScrollDrag {
                    side,
                    grab_offset: y - thumb_top as f32,
                });
            }
            state.last_buy_click = None;
            state.last_sell_click = None;
            state.hovered_item = None;
        }
        self.refresh_npc_shop_texture();
        self.update_npc_shop_hover();
        true
    }

    fn click_npc_shop(&mut self, cursor: Point) {
        let x = cursor.x - NPC_SHOP_X;
        let y = cursor.y - NPC_SHOP_Y;
        if self.click_npc_shop_scrollbar(x, y) {
            return;
        }
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
                    .selected_buy_item()
                    .map(|item| (item.slot, item.item_id));
                state.hovered_item = None;
                state.last_buy_click = None;
                if let Some((slot, item_id)) = selected_item {
                    self.send_npc_shop_item_action(slot, item_id, 1, true);
                }
            } else {
                let quantity = self
                    .inventory
                    .as_ref()
                    .and_then(|inventory| state.selected_sell_item(inventory))
                    .and_then(npc_shop_sell_quantity);
                state.last_sell_click = None;
                if let Some(quantity) = quantity {
                    self.begin_npc_shop_item_action(quantity);
                }
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
                if double_click && let Some(item) = state.selected_buy_item() {
                    let quantity = npc_shop_buy_quantity(item);
                    self.begin_npc_shop_item_action(quantity);
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
                    npc_shop_sell_items(inv, state.tab)
                        .into_iter()
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
                if double_click
                    && let Some(quantity) = self
                        .inventory
                        .as_ref()
                        .and_then(|inventory| state.selected_sell_item(inventory))
                        .and_then(npc_shop_sell_quantity)
                {
                    self.begin_npc_shop_item_action(quantity);
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
        self.refresh_npc_shop_texture();
    }

    fn begin_npc_shop_item_action(&mut self, quantity: NpcShopQuantity) {
        if let Some(quantity) = self
            .npc_shop
            .as_mut()
            .and_then(|state| state.begin_item_action(quantity))
        {
            self.send_npc_shop_item_action(quantity.slot, quantity.item_id, 1, quantity.buy);
        }
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
        let (slot, item_id, buy) = (quantity.slot, quantity.item_id, quantity.buy);
        self.send_npc_shop_item_action(slot, item_id, amount, buy);
        self.refresh_npc_shop_texture();
    }

    fn send_npc_shop_item_action(&mut self, slot: i16, item_id: i32, amount: i16, buy: bool) {
        let Some(state) = self.npc_shop.as_mut().filter(|state| !state.pending) else {
            return;
        };
        let unsupported_currency = buy
            && state
                .shop
                .items
                .iter()
                .find(|item| item.slot == slot && item.item_id == item_id)
                .is_some_and(|item| item.price <= 0 && item.pitch <= 0);
        if unsupported_currency {
            state.quantity = None;
            self.show_server_notice_popup(
                "This item requires a special currency that's not supported.",
            );
            return;
        }
        // Validate the selected stack again after any intervening inventory updates.
        if !buy
            && !self.inventory.as_ref().is_some_and(|inv| {
                inv.items.iter().any(|item| {
                    item.inventory_type == state.tab
                        && item.slot == slot as u16
                        && item.item_id == item_id
                        && item.count > 0
                        && npc_shop_sell_maximum(item) >= amount
                })
            })
        {
            return;
        }
        if let (Ok(packet), Some(session)) = (
            openstory_protocol::npc_shop_item_action(slot, item_id, amount, buy),
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
        let buy_side = x < 230.0;
        let maximum = self.npc_shop.as_ref().map_or(0, |state| {
            let count = if buy_side {
                state.buy_item_indices.len()
            } else {
                self.inventory
                    .as_ref()
                    .map_or(0, |inventory| npc_shop_sell_items(inventory, state.tab).len())
            };
            count.saturating_sub(NPC_SHOP_ROWS)
        });
        if let Some(state) = self.npc_shop.as_mut() {
            if state.quantity.is_some() {
                return;
            }
            let offset = if buy_side {
                &mut state.buy_scroll
            } else {
                &mut state.sell_scroll
            };
            *offset = offset
                .saturating_add_signed(if amount > 0.0 { -1 } else { 1 })
                .min(maximum);
            state.last_buy_click = None;
            state.last_sell_click = None;
            state.hovered_item = None;
        }
        self.refresh_npc_shop_texture();
        self.update_npc_shop_hover();
    }

    fn update_npc_shop_scroll_drag(&mut self, cursor: Point) -> bool {
        let Some(state) = self.npc_shop.as_ref() else {
            return false;
        };
        let Some(drag) = state.scroll_dragging else {
            return false;
        };
        if state.quantity.is_some() {
            return false;
        }
        let count = match drag.side {
            NpcShopScrollSide::Buy => state.buy_item_indices.len(),
            NpcShopScrollSide::Sell => self
                .inventory
                .as_ref()
                .map_or(0, |inventory| npc_shop_sell_items(inventory, state.tab).len()),
        };
        let maximum = count.saturating_sub(NPC_SHOP_ROWS);
        if maximum == 0 {
            return false;
        }
        let track_top = (NPC_SHOP_GRID_TOP + NPC_SHOP_SCROLL_ARROW) as f32;
        let track_bottom = (NPC_SHOP_SCROLL_BOTTOM - NPC_SHOP_SCROLL_ARROW) as f32;
        let travel = (track_bottom as usize)
            .saturating_sub(track_top as usize)
            .saturating_sub(NPC_SHOP_SCROLL_THUMB);
        if travel == 0 {
            return false;
        }
        let local_y = cursor.y - NPC_SHOP_Y;
        let thumb_top = (local_y - drag.grab_offset).clamp(track_top, track_top + travel as f32);
        let offset = (((thumb_top - track_top) / travel as f32) * maximum as f32).round() as usize;
        let Some(state) = self.npc_shop.as_mut() else {
            return false;
        };
        let scroll = match drag.side {
            NpcShopScrollSide::Buy => &mut state.buy_scroll,
            NpcShopScrollSide::Sell => &mut state.sell_scroll,
        };
        let next = offset.min(maximum);
        if *scroll == next {
            return false;
        }
        *scroll = next;
        state.last_buy_click = None;
        state.last_sell_click = None;
        state.hovered_item = None;
        self.refresh_npc_shop_texture();
        self.update_npc_shop_hover();
        true
    }
}

#[cfg(test)]
mod npc_shop_tests {
    use super::*;
    use openstory_protocol::{InventoryItem, InventorySnapshot, InventoryType, NpcShop, NpcShopItem};

    fn shop_with_item(item_id: i32, buyable: i16, rechargeable: bool) -> NpcShopUi {
        NpcShopUi::new(NpcShop {
            npc_id: 1_051_000,
            currency_item_id: 0,
            items: vec![NpcShopItem {
                item_id,
                price: if rechargeable { 0 } else { 3_000 },
                pitch: 0,
                time: 0,
                charge_price: 0,
                buyable,
                rechargeable,
                slot: 0,
            }],
        })
    }

    #[test]
    fn rows_stay_inside_authored_slots() {
        assert_eq!(npc_shop_row_at(7.0, 127.0, false), Some(0));
        assert_eq!(npc_shop_row_at(204.0, 321.0, false), Some(4));
        assert_eq!(npc_shop_row_at(7.0, 162.0, false), None);
        assert_eq!(npc_shop_row_at(209.0, 127.0, false), None);
        assert_eq!(npc_shop_row_at(237.0, 287.0, true), Some(4));
        assert_eq!(npc_shop_row_at(237.0, 322.0, true), None);
    }

    #[test]
    fn full_projectile_stacks_do_not_request_recharge() {
        let item_id = 2_070_000;
        let mut state = shop_with_item(item_id, 800, true);
        state.tab = InventoryType::Use;
        state.sell_selection = Some(1);
        let mut inventory = InventorySnapshot {
            items: vec![InventoryItem {
                inventory_type: InventoryType::Use,
                slot: 1,
                item_id,
                count: 800,
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(npc_shop_auto_recharge_slots(&state.shop, &inventory).is_empty());
        assert_eq!(npc_shop_recharge_slot(&state, &inventory), None);

        inventory.items[0].count -= 1;
        assert_eq!(npc_shop_auto_recharge_slots(&state.shop, &inventory), [1]);
        assert_eq!(npc_shop_recharge_slot(&state, &inventory), Some(1));
    }

    #[test]
    fn equipment_actions_submit_one_without_opening_a_quantity_picker() {
        let item_id = 1_472_000;
        let mut state = shop_with_item(item_id, 100, false);
        let buy = npc_shop_buy_quantity(&state.shop.items[0]);
        let sell = npc_shop_sell_quantity(&InventoryItem {
            slot: 1,
            item_id,
            count: 2,
            ..Default::default()
        })
        .unwrap();

        for (action, is_buy) in [(buy, true), (sell, false)] {
            let immediate = state.begin_item_action(action).unwrap();
            assert_eq!(immediate.maximum, 1);
            assert_eq!(immediate.buy, is_buy);
            assert!(state.quantity.is_none());
        }
    }

    #[test]
    fn pending_shop_requests_do_not_open_quantity_pickers() {
        let mut state = shop_with_item(2_000_000, 100, false);
        state.pending = true;
        let quantity = npc_shop_buy_quantity(&state.shop.items[0]);
        assert!(state.begin_item_action(quantity).is_none());
        assert!(state.quantity.is_none());

        state.pending = false;
        let quantity = npc_shop_buy_quantity(&state.shop.items[0]);
        assert!(state.begin_item_action(quantity).is_none());
        assert_eq!(state.quantity.unwrap().maximum, 100);
    }
}
