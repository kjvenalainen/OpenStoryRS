//! Cash Shop layout and presentation.
//!
//! The legacy client draws the Cash Shop from `CashShopGL.img` and keeps the
//! catalog, character preview, balances, and locker visible in one 1024x768
//! view. This module keeps that asset-backed presentation while giving the
//! catalog a predictable viewport and explicit hit regions for the client.

use std::collections::HashMap;
use std::sync::Arc;

use openstory_assets::{NodeValue, NxArchive};

const WIDTH: usize = 1024;
const HEIGHT: usize = 768;
const GRID_COLUMNS: usize = 2;
const GRID_ROWS: usize = 5;
pub(super) const CATALOG_PAGE_SIZE: usize = GRID_COLUMNS * GRID_ROWS;
const LOCKER_PAGE_SIZE: usize = 8;

/// A logical-coordinate region used by both the Cash Shop view and input code.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HitAction {
    Close,
    Help,
    Coupon,
    Search,
    CatalogView,
    LockerView,
    Category(usize),
    Product(usize),
    BuyProduct(usize),
    BuySelected,
    Currency(usize),
    PreviousPage,
    NextPage,
    LockerItem(usize),
    CouponInput,
    CouponSubmit,
    CouponCancel,
    ConfirmPurchase,
    CancelPurchase,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct HitRegion {
    pub bounds: Rect,
    pub action: HitAction,
}

#[derive(Clone, Debug)]
pub(super) struct Scene {
    pub pixels: Vec<u8>,
    pub hit_regions: Vec<HitRegion>,
}

pub(super) fn hit_test(regions: &[HitRegion], x: f32, y: f32) -> Option<HitAction> {
    regions
        .iter()
        .rev()
        .find(|region| region.bounds.contains(x, y))
        .map(|region| region.action)
}

/// Draws the catalog or locker using the legacy CashShopGL art and a stable
/// logical layout. The host owns input focus, network requests, and state
/// mutation; `hit_test` identifies the requested action.
pub(super) fn render(
    ui_archive: &NxArchive,
    font: &fontdue::Font,
    state: &super::CashShopUi,
    products: &[super::CashShopProduct],
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    character: Option<&openstory_protocol::CharacterInfo>,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Result<Scene, String> {
    const X: usize = 112;
    const Y: usize = 84;
    let mut pixels = vec![0; WIDTH * HEIGHT * 4];
    fill_layer(&mut pixels, 0, 0, WIDTH, HEIGHT, [25, 31, 42, 255]);
    let mut hit_regions = Vec::new();
    draw_asset(
        &mut pixels,
        ui_archive,
        "CashShop.img/Base/backgrnd",
        X as f32,
        Y as f32,
        1.0,
    )?
    .ok_or_else(|| "missing native CashShop.img/Base/backgrnd".to_owned())?;
    draw_asset(
        &mut pixels,
        ui_archive,
        "CashShop.img/Base/Preview/0",
        (X + 22) as f32,
        (Y + 41) as f32,
        1.0,
    )?;
    if let Some(character) = character {
        let mut avatar = Vec::new();
        super::append_character_preview(
            &mut avatar,
            character_archive,
            &character.look,
            43.0,
            156.0,
            1.0,
            false,
            &mut super::MapArtCache::default(),
        )?;
        let mut stage = vec![0; 212 * 165 * 4];
        for sprite in avatar {
            super::composite_bgra_pixels(
                &mut stage,
                212,
                165,
                &sprite.pixels,
                sprite.texture_width as usize,
                sprite.texture_height as usize,
                sprite.rect.x.round() as i32,
                sprite.rect.y.round() as i32,
            );
        }
        super::composite_bgra_pixels(
            &mut pixels,
            WIDTH,
            HEIGHT,
            &stage,
            212,
            165,
            (X + 22) as i32,
            (Y + 41) as i32,
        );
        let name = super::ui_foundation::ellipsize(font, &character.stats.name, 11.0, 130.0);
        add_text(
            font,
            &mut pixels,
            &name,
            (X + 100) as i32,
            (Y + 271) as i32,
            11.0,
            130,
            15,
            [30, 30, 30],
        );
    }
    for (index, label) in super::CASH_SHOP_CATEGORIES.iter().enumerate() {
        let left = X + 278 + index * 55;
        let selected = state.category == index;
        fill_layer(
            &mut pixels,
            left,
            Y + 36,
            53,
            31,
            if selected {
                [80, 160, 115, 255]
            } else {
                [145, 145, 145, 255]
            },
        );
        add_text(
            font,
            &mut pixels,
            label,
            left as i32 + 4,
            (Y + 45) as i32,
            9.0,
            48,
            14,
            [255, 255, 255],
        );
        hit_regions.push(logical_region(
            left as f32,
            (Y + 36) as f32,
            53.0,
            31.0,
            HitAction::Category(index),
        ));
    }
    let query = if state.query.is_empty() {
        "New Event"
    } else {
        &state.query
    };
    add_text(
        font,
        &mut pixels,
        query,
        (X + 282) as i32,
        (Y + 76) as i32,
        11.0,
        400,
        15,
        [20, 45, 30],
    );
    hit_regions.push(logical_region(
        (X + 278) as f32,
        (Y + 73) as f32,
        498.0,
        21.0,
        HitAction::Search,
    ));
    for (index, (path, x, y, action)) in [
        (
            "CashShop.img/CSChar/BtBuyAvatar/normal/0",
            22,
            240,
            HitAction::Help,
        ),
        (
            "CashShop.img/CSChar/BtDefaultAvatar/normal/0",
            101,
            240,
            HitAction::Help,
        ),
        (
            "CashShop.img/CSChar/BtTakeoffAvatar/normal/0",
            180,
            240,
            HitAction::Help,
        ),
        (
            "CashShop.img/CSStatus/BtCharge/normal/0",
            505,
            550,
            HitAction::Currency(0),
        ),
        (
            "CashShop.img/CSStatus/BtCheck/normal/0",
            549,
            550,
            HitAction::Currency(1),
        ),
        (
            "CashShop.img/CSStatus/BtCoupon/normal/0",
            593,
            550,
            HitAction::Coupon,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let _ = index;
        if let Some(bounds) = draw_asset(
            &mut pixels,
            ui_archive,
            path,
            (X + x) as f32,
            (Y + y) as f32,
            1.0,
        )? {
            hit_regions.push(HitRegion { bounds, action });
        }
    }
    hit_regions.push(logical_region(
        (X + 646) as f32,
        (Y + 548) as f32,
        148.0,
        48.0,
        HitAction::Close,
    ));
    hit_regions.push(logical_region(
        (X + 12) as f32,
        (Y + 322) as f32,
        220.0,
        24.0,
        HitAction::LockerView,
    ));
    hit_regions.push(logical_region(
        (X + 278) as f32,
        (Y + 96) as f32,
        410.0,
        8.0,
        HitAction::CatalogView,
    ));
    if let Some(inventory) = inventory {
        for (index, item) in inventory
            .items
            .iter()
            .filter(|item| item.slot > 0)
            .take(12)
            .enumerate()
        {
            if let Some((w, h, icon)) = super::inventory_item_icon(
                item.item_id,
                item_archive,
                character_archive,
                icon_cache,
            ) {
                super::composite_inventory_icon(
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    &icon,
                    w as usize,
                    h as usize,
                    X + 22 + index % 4 * 34,
                    Y + 484 + index / 4 * 34,
                    32,
                );
            }
        }
    }
    let filtered = super::cash_filtered_indices(products, state, string_archive, name_cache);
    let count = if state.view == super::CashShopView::Locker {
        state.locker.len()
    } else {
        filtered.len()
    };
    let page_size = if state.view == super::CashShopView::Locker {
        LOCKER_PAGE_SIZE
    } else {
        CATALOG_PAGE_SIZE
    };
    let pages = count.div_ceil(page_size).max(1);
    let page = state.page.min(pages - 1);
    for slot in 0..page_size {
        let index = page * page_size + slot;
        let left = X + 278 + slot % 2 * 206;
        let top = Y + 101 + slot / 2 * 81;
        let product = if state.view == super::CashShopView::Catalog {
            filtered.get(index).and_then(|i| products.get(*i))
        } else {
            None
        };
        let locker = if state.view == super::CashShopView::Locker {
            state.locker.get(index)
        } else {
            None
        };
        let Some(item_id) = product
            .map(|p| p.item_id)
            .or_else(|| locker.map(|item| item.item_id))
        else {
            continue;
        };
        draw_asset(
            &mut pixels,
            ui_archive,
            "CashShop.img/CSList/Base",
            left as f32,
            top as f32,
            1.0,
        )?;
        if let Some((w, h, icon)) =
            super::inventory_item_icon(item_id, item_archive, character_archive, icon_cache)
        {
            super::composite_inventory_icon(
                &mut pixels,
                WIDTH,
                HEIGHT,
                &icon,
                w as usize,
                h as usize,
                left + 5,
                top + 6,
                62,
            );
        }
        let name = super::item_name(item_id, string_archive, name_cache);
        let name = super::ui_foundation::ellipsize(font, &name, 11.0, 121.0);
        add_text(
            font,
            &mut pixels,
            &name,
            left as i32 + 76,
            top as i32 + 6,
            11.0,
            121,
            14,
            [25, 30, 35],
        );
        let value = product
            .map(|p| format!("{} NX", grouped_number(p.price)))
            .unwrap_or_else(|| "Cash locker".to_owned());
        add_text(
            font,
            &mut pixels,
            &value,
            left as i32 + 76,
            top as i32 + 30,
            11.0,
            121,
            15,
            [25, 30, 35],
        );
        let action = if product.is_some() {
            HitAction::Product(*filtered.get(index).unwrap())
        } else {
            HitAction::LockerItem(index)
        };
        hit_regions.push(logical_region(left as f32, top as f32, 200.0, 78.0, action));
        if let Some(product) = product {
            if let Some(bounds) = draw_asset(
                &mut pixels,
                ui_archive,
                "CashShop.img/CSList/BtBuy/normal/0",
                (left + 76) as f32,
                (top + 54) as f32,
                1.0,
            )? {
                let product_index = filtered[index];
                hit_regions.push(HitRegion {
                    bounds,
                    action: HitAction::BuyProduct(product_index),
                });
            }
            if state.selected_sn == Some(product.sn) {
                stroke_rect(&mut pixels, left, top, 200, 79, [70, 220, 245, 255]);
            }
        }
    }
    for (index, product) in products.iter().take(5).enumerate() {
        if let Some((w, h, icon)) =
            super::inventory_item_icon(product.item_id, item_archive, character_archive, icon_cache)
        {
            super::composite_inventory_icon(
                &mut pixels,
                WIDTH,
                HEIGHT,
                &icon,
                w as usize,
                h as usize,
                X + 738,
                Y + 173 + index * 70,
                32,
            );
        }
        add_text(
            font,
            &mut pixels,
            &format!("{} NX", grouped_number(product.price)),
            (X + 700) as i32,
            (Y + 206 + index * 70) as i32,
            9.0,
            76,
            12,
            [255, 255, 255],
        );
    }
    add_text(
        font,
        &mut pixels,
        &format!("{} / {pages}", page + 1),
        (X + 480) as i32,
        (Y + 516) as i32,
        11.0,
        80,
        15,
        [255, 255, 255],
    );
    hit_regions.push(logical_region(
        (X + 440) as f32,
        (Y + 511) as f32,
        35.0,
        22.0,
        HitAction::PreviousPage,
    ));
    hit_regions.push(logical_region(
        (X + 560) as f32,
        (Y + 511) as f32,
        35.0,
        22.0,
        HitAction::NextPage,
    ));
    for (row, balance) in state.balances.into_iter().enumerate() {
        let value = grouped_number(balance);
        let width = super::dialogue_text_width(font, &value, 11.0).ceil() as i32;
        add_text(
            font,
            &mut pixels,
            &value,
            (X + 482) as i32 - width,
            (Y + 551 + row * 14) as i32,
            11.0,
            145,
            15,
            [20, 20, 20],
        );
    }
    if let Some(product) = state
        .selected_sn
        .and_then(|sn| products.iter().find(|product| product.sn == sn))
    {
        let gender = match product.gender {
            0 => "Male",
            1 => "Female",
            _ => "All characters",
        };
        let duration = if product.period > 0 {
            format!("{} days", product.period)
        } else {
            "Permanent".to_owned()
        };
        let description = format!("{gender} · {duration}");
        add_text(
            font,
            &mut pixels,
            &description,
            (X + 278) as i32,
            (Y + 535) as i32,
            10.0,
            250,
            14,
            [30, 30, 30],
        );
        hit_regions.push(logical_region(
            (X + 278) as f32,
            (Y + 535) as f32,
            250.0,
            14.0,
            HitAction::BuySelected,
        ));
    }
    if let Some(notice) = state.notice.as_deref() {
        add_text(
            font,
            &mut pixels,
            notice,
            X as i32,
            (Y + 610) as i32,
            11.0,
            800,
            32,
            [255, 255, 255],
        );
    }
    if state.confirmation {
        hit_regions.clear();
        draw_purchase_confirmation(
            font,
            state,
            products,
            string_archive,
            name_cache,
            &mut pixels,
            &mut hit_regions,
        );
    } else if state.coupon_open || state.coupon_focused || !state.coupon_code.is_empty() {
        hit_regions.clear();
        draw_coupon_dialog(ui_archive, font, state, &mut pixels, &mut hit_regions)?;
    }
    Ok(Scene {
        pixels,
        hit_regions,
    })
}

fn logical_region(x: f32, y: f32, width: f32, height: f32, action: HitAction) -> HitRegion {
    HitRegion {
        bounds: Rect {
            x,
            y,
            width,
            height,
        },
        action,
    }
}

pub(super) fn render_preview(
    variant: &str,
    ui_archive: &NxArchive,
    font: &fontdue::Font,
    etc_archive: &NxArchive,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
) -> Result<Scene, String> {
    let mut products =
        super::load_cash_products(etc_archive, string_archive, character_archive, item_archive);
    let names = [
        "Orange Mushroom",
        "Pink Bow",
        "Bunny Ears",
        "Desert Fox",
        "Tilted Fedora",
        "Mini Crown",
        "Chocolate Heart",
        "Alphabet Glasses",
        "LED Sunglasses",
        "3D Glasses",
    ];
    let mut fixture_names = HashMap::new();
    products.sort_by_cached_key(|product| {
        let name = super::item_name(product.item_id, string_archive, &mut fixture_names);
        names
            .iter()
            .position(|wanted| name.contains(wanted))
            .unwrap_or(usize::MAX)
    });
    let mut seen = std::collections::HashSet::new();
    products.retain(|product| seen.insert(product.item_id));
    let mut state = super::CashShopUi::new();
    state.entered = true;
    state.balances = [2_145_354_472, 0, 0];
    state.locker_slots = 24;
    state.character_slots = 6;
    state.selected_sn = products.first().map(|product| product.sn);
    state.notice = Some("Offline preview data; no server is connected.".to_owned());

    match variant {
        "catalog" => state.category = 0,
        "locker" => {
            state.view = super::CashShopView::Locker;
            state.locker = products
                .iter()
                .take(8)
                .enumerate()
                .map(|(index, product)| openstory_protocol::CashShopItem {
                    instance_id: index as i64 + 1,
                    item_id: product.item_id,
                    commodity_sn: product.sn,
                    count: product.count,
                    expiration: 0,
                })
                .collect();
        }
        "coupon" => {
            state.coupon_focused = true;
            state.coupon_code = "MAPLE2026".to_owned();
        }
        "confirmation" => state.confirmation = true,
        _ => {
            return Err(format!(
                "unknown Cash Shop preview {variant:?}; choose catalog, locker, coupon, or confirmation"
            ));
        }
    }

    let mut icon_cache = HashMap::new();
    let mut name_cache = HashMap::new();
    render(
        ui_archive,
        font,
        &state,
        &products,
        item_archive,
        character_archive,
        string_archive,
        Some(&super::ui_preview::preview_character()),
        Some(&super::ui_preview::preview_inventory()),
        &mut icon_cache,
        &mut name_cache,
    )
}

fn draw_purchase_confirmation(
    font: &fontdue::Font,
    state: &super::CashShopUi,
    products: &[super::CashShopProduct],
    string_archive: &NxArchive,
    name_cache: &mut HashMap<i32, Option<String>>,
    pixels: &mut [u8],
    hit_regions: &mut Vec<HitRegion>,
) {
    blend_rect(pixels, 0, 0, WIDTH, HEIGHT, [0, 0, 0, 176]);
    blend_rect(pixels, 304, 250, 416, 228, [235, 240, 242, 255]);
    stroke_rect(pixels, 304, 250, 416, 228, [105, 130, 145, 255]);
    add_text(
        font,
        pixels,
        "Confirm purchase",
        336,
        278,
        18.0,
        350,
        28,
        [30, 30, 30],
    );
    let summary = state
        .selected_sn
        .and_then(|sn| products.iter().find(|product| product.sn == sn))
        .map(|product| {
            format!(
                "{} for {} NX?",
                super::item_name(product.item_id, string_archive, name_cache),
                grouped_number(product.price)
            )
        })
        .unwrap_or_else(|| "Select a product first".to_owned());
    add_text(
        font,
        pixels,
        &summary,
        336,
        321,
        13.0,
        350,
        42,
        [40, 40, 40],
    );
    draw_button(pixels, 380, 412, 124, 40, "CONFIRM", font, true);
    draw_button(pixels, 516, 412, 124, 40, "CANCEL", font, false);
    hit_regions.push(region(
        Rect::new(380.0, 412.0, 124.0, 40.0),
        HitAction::ConfirmPurchase,
    ));
    hit_regions.push(region(
        Rect::new(516.0, 412.0, 124.0, 40.0),
        HitAction::CancelPurchase,
    ));
}

fn draw_coupon_dialog(
    ui_archive: &NxArchive,
    font: &fontdue::Font,
    state: &super::CashShopUi,
    pixels: &mut [u8],
    hit_regions: &mut Vec<HitRegion>,
) -> Result<(), String> {
    blend_rect(pixels, 0, 0, WIDTH, HEIGHT, [0, 0, 0, 176]);
    let coupon = draw_asset(
        pixels,
        ui_archive,
        "CashShopGL.img/Popup/Cupon/background",
        374.0,
        352.0,
        1.0,
    )?;
    if coupon.is_none() {
        blend_rect(pixels, 350, 330, 324, 150, [42, 39, 34, 255]);
        stroke_rect(pixels, 350, 330, 324, 150, [215, 174, 103, 255]);
    }
    add_text(
        font,
        pixels,
        "COUPON CODE",
        386,
        358,
        13.0,
        252,
        18,
        if coupon.is_some() {
            [64, 46, 28]
        } else {
            [235, 220, 190]
        },
    );
    blend_rect(pixels, 386, 378, 252, 28, [250, 244, 226, 255]);
    add_text(
        font,
        pixels,
        &state.coupon_code,
        394,
        384,
        11.0,
        234,
        16,
        [41, 37, 30],
    );
    draw_button(
        pixels,
        452,
        415,
        84,
        32,
        if state.coupon_pending {
            "WAITING"
        } else {
            "REDEEM"
        },
        font,
        !state.coupon_pending,
    );
    draw_button(pixels, 542, 415, 84, 32, "CANCEL", font, false);
    hit_regions.push(region(
        Rect::new(386.0, 378.0, 252.0, 28.0),
        HitAction::CouponInput,
    ));
    hit_regions.push(region(
        Rect::new(452.0, 415.0, 84.0, 32.0),
        HitAction::CouponSubmit,
    ));
    hit_regions.push(region(
        Rect::new(542.0, 415.0, 84.0, 32.0),
        HitAction::CouponCancel,
    ));
    Ok(())
}

fn draw_button(
    pixels: &mut [u8],
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    label: &str,
    font: &fontdue::Font,
    active: bool,
) {
    blend_rect(
        pixels,
        x,
        y,
        width,
        height,
        if active {
            [116, 83, 45, 248]
        } else {
            [63, 61, 55, 238]
        },
    );
    stroke_rect(
        pixels,
        x,
        y,
        width,
        height,
        if active {
            [225, 185, 110, 255]
        } else {
            [124, 116, 100, 255]
        },
    );
    add_text(
        font,
        pixels,
        label,
        (x + 5) as i32,
        (y + ((height.saturating_sub(15)) / 2)) as i32,
        9.0,
        width.saturating_sub(10),
        16,
        if active {
            [255, 242, 218]
        } else {
            [197, 190, 176]
        },
    );
}

fn draw_asset(
    pixels: &mut [u8],
    archive: &NxArchive,
    path: &str,
    anchor_x: f32,
    anchor_y: f32,
    scale: f32,
) -> Result<Option<Rect>, String> {
    let node = archive
        .root()
        .resolve(path)
        .map_err(|error| error.to_string())?;
    let Some(node) = node else {
        return Ok(None);
    };
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("NX node {path:?} is not a bitmap"))?;
    let origin = match node
        .child("origin")
        .map_err(|error| error.to_string())?
        .map(|origin| origin.value())
        .transpose()
        .map_err(|error| error.to_string())?
    {
        Some(NodeValue::Vector { x, y }) => (x as f32, y as f32),
        Some(_) => return Err(format!("NX sprite {path:?} has a malformed origin")),
        None => (0.0, 0.0),
    };
    let width = usize::from(bitmap.width());
    let height = usize::from(bitmap.height());
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err(format!(
            "NX sprite {path:?} has invalid dimensions {width}x{height}"
        ));
    }
    let source = bitmap
        .decode_bgra_shared()
        .map_err(|error| error.to_string())?;
    let x = anchor_x - origin.0 * scale;
    let y = anchor_y - origin.1 * scale;
    composite_scaled(pixels, x, y, scale, width, height, &source);
    Ok(Some(Rect {
        x,
        y,
        width: width as f32 * scale,
        height: height as f32 * scale,
    }))
}

fn region(bounds: Rect, action: HitAction) -> HitRegion {
    HitRegion { bounds, action }
}

fn composite_scaled(
    destination: &mut [u8],
    x: f32,
    y: f32,
    scale: f32,
    source_width: usize,
    source_height: usize,
    source: &[u8],
) {
    if !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let left = x.floor() as i32;
    let top = y.floor() as i32;
    let draw_width = (source_width as f32 * scale).ceil() as i32;
    let draw_height = (source_height as f32 * scale).ceil() as i32;
    for dy in 0..draw_height {
        let target_y = top + dy;
        if target_y < 0 || target_y >= HEIGHT as i32 {
            continue;
        }
        let source_y = ((dy as f32 / scale).floor() as usize).min(source_height - 1);
        for dx in 0..draw_width {
            let target_x = left + dx;
            if target_x < 0 || target_x >= WIDTH as i32 {
                continue;
            }
            let source_x = ((dx as f32 / scale).floor() as usize).min(source_width - 1);
            let from = (source_y * source_width + source_x) * 4;
            let to = (target_y as usize * WIDTH + target_x as usize) * 4;
            blend_pixel(&mut destination[to..to + 4], &source[from..from + 4]);
        }
    }
}

fn fill_layer(pixels: &mut [u8], x: usize, y: usize, width: usize, height: usize, color: [u8; 4]) {
    for row in y.min(HEIGHT)..y.saturating_add(height).min(HEIGHT) {
        for column in x.min(WIDTH)..x.saturating_add(width).min(WIDTH) {
            let index = (row * WIDTH + column) * 4;
            pixels[index..index + 4].copy_from_slice(&[color[2], color[1], color[0], color[3]]);
        }
    }
}

fn blend_rect(pixels: &mut [u8], x: usize, y: usize, width: usize, height: usize, color: [u8; 4]) {
    for row in y.min(HEIGHT)..y.saturating_add(height).min(HEIGHT) {
        for column in x.min(WIDTH)..x.saturating_add(width).min(WIDTH) {
            let index = (row * WIDTH + column) * 4;
            blend_pixel(
                &mut pixels[index..index + 4],
                &[color[2], color[1], color[0], color[3]],
            );
        }
    }
}

fn stroke_rect(pixels: &mut [u8], x: usize, y: usize, width: usize, height: usize, color: [u8; 4]) {
    if width == 0 || height == 0 {
        return;
    }
    blend_rect(pixels, x, y, width, 1, color);
    blend_rect(pixels, x, y.saturating_add(height - 1), width, 1, color);
    blend_rect(pixels, x, y, 1, height, color);
    blend_rect(pixels, x.saturating_add(width - 1), y, 1, height, color);
}

fn blend_pixel(destination: &mut [u8], source: &[u8]) {
    let alpha = u32::from(source[3]);
    if alpha == 0 {
        return;
    }
    let inverse = 255 - alpha;
    for channel in 0..3 {
        destination[channel] = ((u32::from(source[channel]) * alpha
            + u32::from(destination[channel]) * inverse)
            / 255) as u8;
    }
    destination[3] = (alpha + u32::from(destination[3]) * inverse / 255) as u8;
}

fn add_text(
    font: &fontdue::Font,
    pixels: &mut [u8],
    text: &str,
    x: i32,
    y: i32,
    size: f32,
    max_width: usize,
    max_height: usize,
    color: [u8; 3],
) {
    super::draw_dialogue_text(
        font, pixels, WIDTH, HEIGHT, text, x, y, size, max_width, max_height, color,
    );
}

fn grouped_number(value: i32) -> String {
    let digits = value.max(0).to_string();
    let first_group = digits.len() % 3;
    let mut output = String::with_capacity(digits.len() + digits.len() / 3);
    if first_group > 0 {
        output.push_str(&digits[..first_group]);
    }
    for (index, group) in digits[first_group..].as_bytes().chunks(3).enumerate() {
        if first_group > 0 || index > 0 {
            output.push(',');
        }
        // `digits` contains ASCII decimal digits only.
        output.push_str(std::str::from_utf8(group).unwrap_or("000"));
    }
    output
}

impl Rect {
    const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buy_controls_take_priority_over_the_product_card() {
        let regions = [
            logical_region(10.0, 10.0, 200.0, 80.0, HitAction::Product(4)),
            logical_region(86.0, 64.0, 37.0, 19.0, HitAction::BuyProduct(4)),
        ];
        assert_eq!(
            hit_test(&regions, 90.0, 65.0),
            Some(HitAction::BuyProduct(4))
        );
        assert_eq!(hit_test(&regions, 20.0, 20.0), Some(HitAction::Product(4)));
        assert_eq!(hit_test(&regions, 210.0, 20.0), None);
    }
}
