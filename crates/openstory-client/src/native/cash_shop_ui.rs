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
const GRID_COLUMNS: usize = 3;
const GRID_ROWS: usize = 4;
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

impl Scene {
    pub fn hit_test(&self, x: f32, y: f32) -> Option<HitAction> {
        self.hit_regions
            .iter()
            .rev()
            .find(|region| region.bounds.contains(x, y))
            .map(|region| region.action)
    }
}

/// Draws the catalog or locker using the legacy CashShopGL art and a stable
/// logical layout. The host owns input focus, network requests, and state
/// mutation; `Scene::hit_test` identifies the requested action.
pub(super) fn render(
    ui_archive: &NxArchive,
    font: &fontdue::Font,
    state: &super::CashShopUi,
    products: &[super::CashShopProduct],
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Result<Scene, String> {
    if state.view == super::CashShopView::Catalog
        && !state.confirmation
        && state.coupon_code.is_empty()
    {
        return render_reference_catalog(
            font,
            state,
            products,
            item_archive,
            character_archive,
            string_archive,
            icon_cache,
            name_cache,
        );
    }
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    let mut hit_regions = Vec::with_capacity(48);

    // Keep the original full-screen Cash Shop composition as the visual base.
    let background = draw_asset(
        &mut pixels,
        ui_archive,
        "CashShopGL.img/BaseFrame/background",
        0.0,
        0.0,
        1.0,
    )?;
    if background.is_none() {
        fill_layer(&mut pixels, 0, 0, WIDTH, HEIGHT, [28, 31, 36, 255]);
    }

    // This center frame is the legacy Home panel; extend its dark work surface
    // to cover the full catalog viewport so the fourth row is not clipped.
    let _ = draw_asset(
        &mut pixels,
        ui_archive,
        "CashShopGL.img/MainMenu/Home/background",
        170.0,
        31.0,
        1.0,
    )?;
    blend_rect(&mut pixels, 174, 74, 573, 548, [24, 27, 32, 238]);
    stroke_rect(&mut pixels, 174, 74, 573, 548, [181, 143, 83, 255]);

    let _ = draw_asset(
        &mut pixels,
        ui_archive,
        "CashShopGL.img/BaseFrame/CSItemSearch/background",
        0.0,
        0.0,
        1.0,
    )?;
    add_text(
        font,
        &mut pixels,
        "MAPLE CASH SHOP",
        184,
        9,
        17.0,
        250,
        24,
        [255, 246, 222],
    );
    add_text(
        font,
        &mut pixels,
        if state.query.is_empty() && !state.search_focused {
            "Search items"
        } else {
            &state.query
        },
        486,
        9,
        11.0,
        171,
        16,
        if state.query.is_empty() && !state.search_focused {
            [136, 132, 123]
        } else {
            [44, 39, 32]
        },
    );
    let search_button = draw_asset(
        &mut pixels,
        ui_archive,
        "CashShopGL.img/BaseFrame/CSItemSearch/BtGo/normal/0",
        0.0,
        0.0,
        1.0,
    )?;
    hit_regions.push(region(
        Rect::new(483.0, 6.0, 198.0, 23.0),
        HitAction::Search,
    ));

    let help_button = draw_asset(
        &mut pixels,
        ui_archive,
        "CashShopGL.img/BaseFrame/BtHelp/normal/0",
        0.0,
        0.0,
        1.0,
    )?;
    let coupon_button = draw_asset(
        &mut pixels,
        ui_archive,
        "CashShopGL.img/BaseFrame/BtCoupon/normal/0",
        0.0,
        0.0,
        1.0,
    )?;
    let exit_button = draw_asset(
        &mut pixels,
        ui_archive,
        "CashShopGL.img/BaseFrame/BtExit/normal/0",
        0.0,
        0.0,
        1.0,
    )?;
    hit_regions.push(region(
        help_button.unwrap_or(Rect::new(876.0, 7.0, 40.0, 24.0)),
        HitAction::Help,
    ));
    hit_regions.push(region(
        coupon_button.unwrap_or(Rect::new(777.0, 6.0, 60.0, 22.0)),
        HitAction::Coupon,
    ));
    hit_regions.push(region(
        exit_button.unwrap_or(Rect::new(958.0, 5.0, 36.0, 24.0)),
        HitAction::Close,
    ));
    if let Some(bounds) = search_button {
        hit_regions.push(region(bounds, HitAction::Search));
    }

    // Small navigation tabs preserve the newer Rust locker view while keeping
    // the underlying CashShopGL layout and palette.
    draw_tab(
        font,
        &mut pixels,
        "CATALOG",
        190,
        state.view == super::CashShopView::Catalog,
    );
    draw_tab(
        font,
        &mut pixels,
        "MY LOCKER",
        282,
        state.view == super::CashShopView::Locker,
    );
    hit_regions.push(region(
        Rect::new(184.0, 37.0, 88.0, 29.0),
        HitAction::CatalogView,
    ));
    hit_regions.push(region(
        Rect::new(276.0, 37.0, 112.0, 29.0),
        HitAction::LockerView,
    ));

    if state.view == super::CashShopView::Catalog {
        draw_catalog(
            ui_archive,
            font,
            state,
            products,
            item_archive,
            character_archive,
            string_archive,
            icon_cache,
            name_cache,
            &mut pixels,
            &mut hit_regions,
        )?;
    } else {
        draw_locker(
            font,
            state,
            item_archive,
            character_archive,
            string_archive,
            icon_cache,
            name_cache,
            &mut pixels,
            &mut hit_regions,
        );
    }

    draw_balances(font, state, &mut pixels);

    if state.confirmation {
        draw_purchase_confirmation(
            font,
            state,
            products,
            string_archive,
            name_cache,
            &mut pixels,
            &mut hit_regions,
        );
    } else if state.coupon_focused || state.coupon_pending {
        draw_coupon_dialog(ui_archive, font, state, &mut pixels, &mut hit_regions)?;
    }

    Ok(Scene {
        pixels,
        hit_regions,
    })
}

fn render_reference_catalog(
    font: &fontdue::Font,
    state: &super::CashShopUi,
    products: &[super::CashShopProduct],
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Result<Scene, String> {
    const X: usize = 223;
    const Y: usize = 151;
    const PANEL_W: usize = 578;
    const PANEL_H: usize = 465;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    fill_layer(&mut pixels, 0, 0, WIDTH, HEIGHT, [14, 25, 49, 255]);
    let mut hit_regions = Vec::with_capacity(40);
    fill_layer(&mut pixels, X, Y, PANEL_W, PANEL_H, [83, 104, 135, 255]);
    fill_layer(
        &mut pixels,
        X + 3,
        Y + 3,
        PANEL_W - 6,
        PANEL_H - 6,
        [224, 229, 234, 255],
    );
    fill_layer(
        &mut pixels,
        X + 6,
        Y + 6,
        PANEL_W - 12,
        PANEL_H - 12,
        [190, 205, 220, 255],
    );

    // Left character-preview, cash inventory, and regular inventory columns.
    fill_layer(&mut pixels, X + 9, Y + 8, 157, 169, [34, 68, 108, 255]);
    fill_layer(&mut pixels, X + 12, Y + 11, 151, 163, [203, 225, 239, 255]);
    fill_layer(&mut pixels, X + 17, Y + 20, 141, 139, [213, 231, 243, 255]);
    for row in 0..8 {
        fill_layer(
            &mut pixels,
            X + 21 + row * 17,
            Y + 24,
            2,
            128,
            [224, 237, 245, 255],
        );
    }
    // A compact MapleStory-style avatar silhouette stands in the preview stage.
    fill_layer(&mut pixels, X + 48, Y + 83, 33, 24, [76, 54, 46, 255]);
    fill_layer(&mut pixels, X + 51, Y + 77, 28, 15, [224, 185, 142, 255]);
    fill_layer(&mut pixels, X + 48, Y + 73, 33, 7, [78, 45, 37, 255]);
    fill_layer(&mut pixels, X + 53, Y + 96, 24, 34, [53, 88, 138, 255]);
    fill_layer(&mut pixels, X + 49, Y + 126, 12, 20, [39, 43, 53, 255]);
    fill_layer(&mut pixels, X + 68, Y + 126, 12, 20, [39, 43, 53, 255]);
    fill_layer(&mut pixels, X + 59, Y + 82, 3, 3, [39, 33, 30, 255]);
    fill_layer(&mut pixels, X + 70, Y + 82, 3, 3, [39, 33, 30, 255]);
    add_text(
        font,
        &mut pixels,
        "Preview",
        (X + 12) as i32,
        (Y + 3) as i32,
        9.0,
        55,
        12,
        [255, 255, 255],
    );
    add_text(
        font,
        &mut pixels,
        "ON",
        (X + 138) as i32,
        (Y + 3) as i32,
        8.0,
        20,
        12,
        [245, 248, 250],
    );
    for (index, label) in ["BUY EQUIPPED ITEM", "RETURN TO DEFAULT", "REMOVE ALL"]
        .into_iter()
        .enumerate()
    {
        let button_x = X + 10 + index * 51;
        fill_layer(&mut pixels, button_x, Y + 178, 49, 16, [181, 102, 27, 255]);
        fill_layer(
            &mut pixels,
            button_x + 1,
            Y + 179,
            47,
            14,
            [240, 172, 66, 255],
        );
        add_text(
            font,
            &mut pixels,
            label,
            button_x as i32 + 2,
            (Y + 181) as i32,
            5.5,
            45,
            10,
            [255, 255, 245],
        );
    }
    add_text(
        font,
        &mut pixels,
        "Character ID",
        (X + 12) as i32,
        (Y + 201) as i32,
        8.0,
        68,
        12,
        [58, 87, 119],
    );
    add_text(
        font,
        &mut pixels,
        "Nexon ID",
        (X + 12) as i32,
        (Y + 218) as i32,
        8.0,
        68,
        12,
        [58, 87, 119],
    );
    add_text(
        font,
        &mut pixels,
        "PreviewHero",
        (X + 83) as i32,
        (Y + 201) as i32,
        8.0,
        70,
        12,
        [48, 65, 82],
    );
    add_text(
        font,
        &mut pixels,
        "An**",
        (X + 83) as i32,
        (Y + 218) as i32,
        8.0,
        70,
        12,
        [48, 65, 82],
    );

    fill_layer(&mut pixels, X + 9, Y + 239, 157, 64, [37, 74, 115, 255]);
    fill_layer(&mut pixels, X + 11, Y + 241, 153, 60, [229, 236, 241, 255]);
    add_text(
        font,
        &mut pixels,
        "CashInventory",
        (X + 12) as i32,
        (Y + 241) as i32,
        11.0,
        110,
        14,
        [37, 50, 61],
    );
    for row in 0..2 {
        for col in 0..5 {
            stroke_rect(
                &mut pixels,
                X + 14 + col * 28,
                Y + 258 + row * 18,
                27,
                18,
                [170, 191, 208, 255],
            );
        }
    }
    fill_layer(&mut pixels, X + 9, Y + 307, 157, 149, [35, 73, 115, 255]);
    fill_layer(&mut pixels, X + 11, Y + 309, 153, 145, [230, 235, 239, 255]);
    add_text(
        font,
        &mut pixels,
        "Item Inventory",
        (X + 12) as i32,
        (Y + 309) as i32,
        11.0,
        130,
        14,
        [37, 50, 61],
    );
    for (index, label) in ["Equip", "Use", "Set-up", "Etc", "Cash", "+ Equip"]
        .into_iter()
        .enumerate()
    {
        let tab_x = X + 12 + index * 24;
        fill_layer(
            &mut pixels,
            tab_x,
            Y + 325,
            23,
            16,
            if index == 4 {
                [204, 61, 101, 255]
            } else {
                [145, 160, 175, 255]
            },
        );
        add_text(
            font,
            &mut pixels,
            label,
            tab_x as i32 + 2,
            (Y + 328) as i32,
            5.5,
            21,
            10,
            [255, 255, 255],
        );
    }
    for row in 0..3 {
        for col in 0..6 {
            stroke_rect(
                &mut pixels,
                X + 13 + col * 23,
                Y + 342 + row * 34,
                23,
                33,
                [177, 193, 207, 255],
            );
        }
    }

    // Blue catalog frame and the reference's compact tab strip.
    fill_layer(&mut pixels, X + 170, Y + 8, 399, 408, [18, 54, 98, 255]);
    fill_layer(&mut pixels, X + 173, Y + 11, 393, 402, [231, 238, 245, 255]);
    let tabs = [
        "MAIN",
        "EVENT",
        "EQUIP",
        "USE",
        "SET-UP",
        "ETC",
        "PET",
        "PACKAGE",
        "WISH LIST",
    ];
    let tab_widths = [38, 47, 39, 32, 47, 31, 29, 47, 59];
    let mut tab_x = X + 176;
    for (index, (label, tab_width)) in tabs.into_iter().zip(tab_widths).enumerate() {
        let active = (index == 0 && state.category == 0) || (index == 1 && state.category != 0);
        fill_layer(
            &mut pixels,
            tab_x,
            Y + 17,
            tab_width,
            25,
            if active {
                if index == 1 {
                    [213, 75, 123, 255]
                } else {
                    [40, 74, 113, 255]
                }
            } else {
                [162, 177, 192, 255]
            },
        );
        add_text(
            font,
            &mut pixels,
            label,
            tab_x as i32 + 2,
            (Y + 24) as i32,
            7.0,
            tab_width - 4,
            11,
            [255, 255, 255],
        );
        hit_regions.push(region(
            Rect {
                x: tab_x as f32,
                y: (Y + 17) as f32,
                width: tab_width as f32,
                height: 25.0,
            },
            if index == 8 {
                HitAction::LockerView
            } else if index == 0 {
                HitAction::CatalogView
            } else {
                HitAction::Category(index - 1)
            },
        ));
        tab_x += tab_width;
    }
    add_text(
        font,
        &mut pixels,
        "New Event",
        (X + 177) as i32,
        (Y + 50) as i32,
        8.0,
        120,
        14,
        [59, 70, 82],
    );
    fill_layer(&mut pixels, X + 174, Y + 65, 323, 15, [32, 89, 47, 255]);
    add_text(
        font,
        &mut pixels,
        "Search items",
        (X + 180) as i32,
        (Y + 66) as i32,
        8.0,
        105,
        12,
        [245, 248, 245],
    );
    hit_regions.push(region(
        Rect {
            x: (X + 174) as f32,
            y: (Y + 65) as f32,
            width: 323.0,
            height: 15.0,
        },
        HitAction::Search,
    ));

    let visible = super::cash_filtered_indices(products, state, string_archive, name_cache);
    let page_count = visible.len().max(1).div_ceil(10);
    let page = state.page.min(page_count.saturating_sub(1));
    for (slot, product_index) in visible.iter().skip(page * 10).take(10).enumerate() {
        let product = &products[*product_index];
        let col = slot % 2;
        let row = slot / 2;
        let card_x = X + 175 + col * 162;
        let card_y = Y + 82 + row * 57;
        fill_layer(&mut pixels, card_x, card_y, 158, 54, [42, 85, 132, 255]);
        fill_layer(
            &mut pixels,
            card_x + 2,
            card_y + 2,
            154,
            50,
            [238, 243, 247, 255],
        );
        stroke_rect(
            &mut pixels,
            card_x + 2,
            card_y + 2,
            154,
            50,
            [111, 143, 172, 255],
        );
        if let Some((icon_width, icon_height, icon_pixels)) =
            super::inventory_item_icon(product.item_id, item_archive, character_archive, icon_cache)
        {
            super::composite_inventory_icon(
                &mut pixels,
                WIDTH,
                HEIGHT,
                &icon_pixels,
                icon_width as usize,
                icon_height as usize,
                card_x + 7,
                card_y + 9,
                36,
            );
        }
        let item_name = super::item_name(product.item_id, string_archive, name_cache);
        add_text(
            font,
            &mut pixels,
            &item_name,
            (card_x + 49) as i32,
            (card_y + 6) as i32,
            8.0,
            102,
            12,
            [56, 70, 85],
        );
        add_text(
            font,
            &mut pixels,
            &format!("{} NX", grouped_number(product.price)),
            (card_x + 49) as i32,
            (card_y + 20) as i32,
            8.0,
            90,
            12,
            [60, 78, 96],
        );
        // Add the card region first so the more specific BUY region wins hit
        // testing where it overlaps the card.
        hit_regions.push(region(
            Rect {
                x: card_x as f32,
                y: card_y as f32,
                width: 158.0,
                height: 54.0,
            },
            HitAction::Product(*product_index),
        ));
        for (button, label) in [(0, "BUY"), (1, "GIFT"), (2, "GET")] {
            let bx = card_x + 49 + button * 34;
            fill_layer(&mut pixels, bx, card_y + 35, 31, 14, [44, 95, 147, 255]);
            fill_layer(
                &mut pixels,
                bx + 1,
                card_y + 36,
                29,
                12,
                [207, 224, 238, 255],
            );
            add_text(
                font,
                &mut pixels,
                label,
                bx as i32 + 6,
                (card_y + 38) as i32,
                6.0,
                20,
                9,
                [49, 72, 95],
            );
            if button == 0 {
                hit_regions.push(region(
                    Rect {
                        x: bx as f32,
                        y: (card_y + 35) as f32,
                        width: 31.0,
                        height: 14.0,
                    },
                    HitAction::BuyProduct(*product_index),
                ));
            }
        }
    }

    // Wish-list column and balance footer.
    fill_layer(&mut pixels, X + 500, Y + 47, 65, 368, [30, 56, 102, 255]);
    add_text(
        font,
        &mut pixels,
        "SERAPHITEM",
        (X + 504) as i32,
        (Y + 52) as i32,
        6.0,
        58,
        10,
        [255, 242, 249],
    );
    add_text(
        font,
        &mut pixels,
        "Best item",
        (X + 505) as i32,
        (Y + 73) as i32,
        8.0,
        58,
        12,
        [230, 235, 242],
    );
    for rank in 0..5 {
        let ry = Y + 92 + rank * 57;
        fill_layer(&mut pixels, X + 504, ry, 56, 53, [34, 70, 120, 255]);
        add_text(
            font,
            &mut pixels,
            &(rank + 1).to_string(),
            (X + 508) as i32,
            (ry + 1) as i32,
            24.0,
            25,
            27,
            [214, 229, 247],
        );
        add_text(
            font,
            &mut pixels,
            &format!("{} NX", 10_000 - rank * 2_000),
            (X + 507) as i32,
            (ry + 32) as i32,
            7.0,
            50,
            11,
            [222, 226, 239],
        );
    }
    fill_layer(&mut pixels, X + 170, Y + 419, 399, 35, [217, 226, 234, 255]);
    add_text(
        font,
        &mut pixels,
        "NX Credit",
        (X + 178) as i32,
        (Y + 422) as i32,
        7.0,
        58,
        10,
        [65, 82, 99],
    );
    add_text(
        font,
        &mut pixels,
        &grouped_number(state.balances[0]),
        (X + 178) as i32,
        (Y + 435) as i32,
        8.0,
        72,
        12,
        [52, 69, 85],
    );
    add_text(
        font,
        &mut pixels,
        "Maple Point",
        (X + 262) as i32,
        (Y + 422) as i32,
        7.0,
        66,
        10,
        [65, 82, 99],
    );
    add_text(
        font,
        &mut pixels,
        &grouped_number(state.balances[1]),
        (X + 262) as i32,
        (Y + 435) as i32,
        8.0,
        72,
        12,
        [52, 69, 85],
    );
    add_text(
        font,
        &mut pixels,
        "NX Prepaid",
        (X + 356) as i32,
        (Y + 422) as i32,
        7.0,
        64,
        10,
        [65, 82, 99],
    );
    add_text(
        font,
        &mut pixels,
        &grouped_number(state.balances[2]),
        (X + 356) as i32,
        (Y + 435) as i32,
        8.0,
        72,
        12,
        [52, 69, 85],
    );
    add_text(
        font,
        &mut pixels,
        &format!("{} | {} of {}", page + 1, visible.len(), page_count),
        (X + 453) as i32,
        (Y + 430) as i32,
        7.0,
        48,
        12,
        [60, 75, 88],
    );
    if page > 0 {
        hit_regions.push(region(
            Rect {
                x: (X + 446) as f32,
                y: (Y + 419) as f32,
                width: 45.0,
                height: 35.0,
            },
            HitAction::PreviousPage,
        ));
    }
    if page + 1 < page_count {
        hit_regions.push(region(
            Rect {
                x: (X + 491) as f32,
                y: (Y + 419) as f32,
                width: 45.0,
                height: 35.0,
            },
            HitAction::NextPage,
        ));
    }
    add_text(
        font,
        &mut pixels,
        "EXIT",
        (X + 526) as i32,
        (Y + 422) as i32,
        15.0,
        40,
        24,
        [44, 50, 57],
    );
    hit_regions.push(region(
        Rect {
            x: (X + 518) as f32,
            y: (Y + 419) as f32,
            width: 48.0,
            height: 35.0,
        },
        HitAction::Close,
    ));
    Ok(Scene {
        pixels,
        hit_regions,
    })
}

/// Builds an offline scene with representative server data for the native UI
/// preview runner. `variant` accepts `catalog`, `locker`, `coupon`, or
/// `confirmation`.
pub(super) fn render_preview(
    variant: &str,
    ui_archive: &NxArchive,
    font: &fontdue::Font,
    etc_archive: &NxArchive,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
) -> Result<Scene, String> {
    let products =
        super::load_cash_products(etc_archive, string_archive, character_archive, item_archive);
    let mut state = super::CashShopUi::new();
    state.entered = true;
    state.balances = [250_000, 12_500, 80_000];
    state.locker_slots = 24;
    state.character_slots = 6;
    state.selected_sn = products.first().map(|product| product.sn);
    state.notice = Some("Offline preview data; no server is connected.".to_owned());

    match variant {
        "catalog" => state.category = 1,
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
        &mut icon_cache,
        &mut name_cache,
    )
}

fn draw_catalog(
    ui_archive: &NxArchive,
    font: &fontdue::Font,
    state: &super::CashShopUi,
    products: &[super::CashShopProduct],
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
    pixels: &mut [u8],
    hit_regions: &mut Vec<HitRegion>,
) -> Result<(), String> {
    let visible = super::cash_filtered_indices(products, state, string_archive, name_cache);
    let page_count = visible.len().max(1).div_ceil(CATALOG_PAGE_SIZE);
    let page = state.page.min(page_count.saturating_sub(1));
    let start = page * CATALOG_PAGE_SIZE;

    for (index, product_index) in visible
        .iter()
        .skip(start)
        .take(CATALOG_PAGE_SIZE)
        .enumerate()
    {
        let product = &products[*product_index];
        let rect = catalog_card_rect(index);
        let x = rect.x as usize;
        let y = rect.y as usize;
        if draw_asset(
            pixels,
            ui_archive,
            "CashShopGL.img/MainMenu/MainItem/background",
            rect.x,
            rect.y,
            1.0,
        )?
        .is_none()
        {
            blend_rect(
                pixels,
                x,
                y,
                rect.width as usize,
                rect.height as usize,
                [43, 44, 44, 248],
            );
            stroke_rect(
                pixels,
                x,
                y,
                rect.width as usize,
                rect.height as usize,
                [139, 118, 83, 255],
            );
        }
        if state.selected_sn == Some(product.sn) {
            stroke_rect(
                pixels,
                x + 2,
                y + 2,
                rect.width as usize - 4,
                rect.height as usize - 4,
                [250, 205, 117, 255],
            );
        }

        if let Some((icon_width, icon_height, icon_pixels)) =
            super::inventory_item_icon(product.item_id, item_archive, character_archive, icon_cache)
        {
            super::composite_inventory_icon(
                pixels,
                WIDTH,
                HEIGHT,
                &icon_pixels,
                icon_width as usize,
                icon_height as usize,
                x + 9,
                y + 12,
                54,
            );
        }

        let name = super::item_name(product.item_id, string_archive, name_cache);
        add_text(
            font,
            pixels,
            &name,
            (x + 72) as i32,
            (y + 13) as i32,
            11.0,
            105,
            38,
            [238, 232, 216],
        );
        add_text(
            font,
            pixels,
            &format!("{} NX", grouped_number(product.price)),
            (x + 72) as i32,
            (y + 58) as i32,
            11.0,
            105,
            18,
            [247, 204, 127],
        );
        let quantity = if product.count > 1 {
            format!("Bundle of {}", product.count)
        } else if product.period > 0 {
            format!("{} days", product.period)
        } else {
            "Permanent".to_owned()
        };
        add_text(
            font,
            pixels,
            &quantity,
            (x + 72) as i32,
            (y + 78) as i32,
            9.0,
            105,
            15,
            [196, 187, 169],
        );

        let buy_button = draw_asset(
            pixels,
            ui_archive,
            "CashShopGL.img/MainMenu/MainItem/BtBuy/normal/0",
            (x + 7) as f32,
            (y + 39) as f32,
            1.0,
        )?;
        hit_regions.push(region(rect, HitAction::Product(*product_index)));
        if let Some(bounds) = buy_button {
            hit_regions.push(region(bounds, HitAction::BuyProduct(*product_index)));
        }
    }

    if visible.is_empty() {
        add_text(
            font,
            pixels,
            "No products match this category or search.",
            292,
            302,
            14.0,
            360,
            28,
            [230, 224, 211],
        );
    }

    draw_categories(font, state, pixels, hit_regions);
    draw_page_controls(
        font,
        page,
        page_count,
        visible.len(),
        CATALOG_PAGE_SIZE,
        pixels,
        hit_regions,
    );
    draw_product_details(
        font,
        state,
        products,
        item_archive,
        character_archive,
        string_archive,
        icon_cache,
        name_cache,
        pixels,
        hit_regions,
    );
    draw_currency_options(font, state, pixels, hit_regions);
    Ok(())
}

fn draw_categories(
    font: &fontdue::Font,
    state: &super::CashShopUi,
    pixels: &mut [u8],
    hit_regions: &mut Vec<HitRegion>,
) {
    add_text(
        font,
        pixels,
        "BROWSE CATEGORIES",
        18,
        76,
        10.0,
        142,
        15,
        [226, 208, 177],
    );
    for (index, label) in super::CASH_SHOP_CATEGORIES.iter().enumerate() {
        let y = 98 + index * 32;
        let active = state.category == index;
        if active {
            blend_rect(pixels, 12, y, 144, 25, [117, 88, 50, 236]);
            stroke_rect(pixels, 12, y, 144, 25, [222, 179, 99, 255]);
        } else if index % 2 == 0 {
            blend_rect(pixels, 12, y, 144, 25, [43, 44, 43, 222]);
        }
        add_text(
            font,
            pixels,
            label,
            22,
            (y + 5) as i32,
            11.0,
            126,
            16,
            if active {
                [255, 242, 215]
            } else {
                [233, 224, 207]
            },
        );
        hit_regions.push(region(
            Rect::new(12.0, y as f32, 144.0, 25.0),
            HitAction::Category(index),
        ));
    }
}

fn draw_page_controls(
    font: &fontdue::Font,
    page: usize,
    page_count: usize,
    item_count: usize,
    page_size: usize,
    pixels: &mut [u8],
    hit_regions: &mut Vec<HitRegion>,
) {
    let y = 603;
    draw_button(pixels, 190, y, 64, 27, "PREV", font, page > 0);
    draw_button(pixels, 640, y, 64, 27, "NEXT", font, page + 1 < page_count);
    let first = if item_count == 0 {
        0
    } else {
        page * page_size + 1
    };
    let last = item_count.min((page + 1) * page_size);
    add_text(
        font,
        pixels,
        &format!(
            "{first}-{last} of {item_count}   ·   page {}/{}",
            page + 1,
            page_count
        ),
        278,
        (y + 7) as i32,
        10.0,
        350,
        16,
        [226, 216, 197],
    );
    hit_regions.push(region(
        Rect::new(190.0, y as f32, 64.0, 27.0),
        HitAction::PreviousPage,
    ));
    hit_regions.push(region(
        Rect::new(640.0, y as f32, 64.0, 27.0),
        HitAction::NextPage,
    ));
}

fn draw_product_details(
    font: &fontdue::Font,
    state: &super::CashShopUi,
    products: &[super::CashShopProduct],
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
    pixels: &mut [u8],
    hit_regions: &mut Vec<HitRegion>,
) {
    add_text(
        font,
        pixels,
        "SELECTED ITEM",
        778,
        111,
        11.0,
        210,
        16,
        [226, 208, 177],
    );
    let selected = state
        .selected_sn
        .and_then(|sn| products.iter().find(|product| product.sn == sn));
    if let Some(product) = selected {
        blend_rect(pixels, 850, 128, 80, 80, [39, 39, 36, 240]);
        stroke_rect(pixels, 850, 128, 80, 80, [157, 129, 84, 255]);
        if let Some((icon_width, icon_height, icon_pixels)) =
            super::inventory_item_icon(product.item_id, item_archive, character_archive, icon_cache)
        {
            super::composite_inventory_icon(
                pixels,
                WIDTH,
                HEIGHT,
                &icon_pixels,
                icon_width as usize,
                icon_height as usize,
                854,
                132,
                72,
            );
        }
        let name = super::item_name(product.item_id, string_archive, name_cache);
        add_text(
            font,
            pixels,
            &name,
            778,
            218,
            12.0,
            216,
            44,
            [242, 235, 218],
        );
        add_text(
            font,
            pixels,
            &format!("Price  {} NX", grouped_number(product.price)),
            778,
            264,
            11.0,
            216,
            18,
            [247, 204, 127],
        );
        add_text(
            font,
            pixels,
            &format!("Quantity  {}", product.count),
            778,
            285,
            10.0,
            216,
            15,
            [203, 193, 174],
        );
        add_text(
            font,
            pixels,
            &format!(
                "Duration  {}",
                if product.period > 0 {
                    format!("{} days", product.period)
                } else {
                    "Permanent".to_owned()
                }
            ),
            778,
            305,
            10.0,
            216,
            15,
            [203, 193, 174],
        );
        add_text(
            font,
            pixels,
            &format!(
                "Gender  {}",
                match product.gender {
                    0 => "Male",
                    1 => "Female",
                    _ => "All",
                }
            ),
            778,
            325,
            10.0,
            216,
            15,
            [203, 193, 174],
        );
        draw_button(
            pixels,
            778,
            354,
            214,
            36,
            "BUY SELECTED ITEM",
            font,
            state.entered,
        );
        hit_regions.push(region(
            Rect::new(778.0, 354.0, 214.0, 36.0),
            HitAction::BuySelected,
        ));
    } else {
        add_text(
            font,
            pixels,
            "Choose a catalog item to see its price and details.",
            778,
            142,
            11.0,
            216,
            52,
            [207, 198, 179],
        );
    }
}

fn draw_currency_options(
    font: &fontdue::Font,
    state: &super::CashShopUi,
    pixels: &mut [u8],
    hit_regions: &mut Vec<HitRegion>,
) {
    for (index, label) in ["CREDIT", "POINTS", "PREPAID"].into_iter().enumerate() {
        let x = 775 + index * 74;
        let selected = state.currency == index;
        draw_button(pixels, x, 402, 70, 38, label, font, selected);
        add_text(
            font,
            pixels,
            &grouped_number(state.balances[index]),
            (x + 5) as i32,
            425,
            9.0,
            62,
            12,
            [247, 235, 208],
        );
        hit_regions.push(region(
            Rect::new(x as f32, 402.0, 70.0, 38.0),
            HitAction::Currency(index),
        ));
    }
}

fn draw_locker(
    font: &fontdue::Font,
    state: &super::CashShopUi,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
    pixels: &mut [u8],
    hit_regions: &mut Vec<HitRegion>,
) {
    add_text(
        font,
        pixels,
        "MY CASH SHOP LOCKER",
        195,
        79,
        13.0,
        440,
        20,
        [246, 228, 193],
    );
    add_text(
        font,
        pixels,
        &format!(
            "{} items  ·  {} slots",
            state.locker.len(),
            state.locker_slots
        ),
        195,
        101,
        10.0,
        440,
        16,
        [201, 188, 163],
    );
    let mut items = state.locker.iter().collect::<Vec<_>>();
    items.sort_by_key(|item| item.instance_id);
    let page_count = items.len().max(1).div_ceil(LOCKER_PAGE_SIZE);
    let page = state.page.min(page_count.saturating_sub(1));
    for (index, item) in items
        .iter()
        .skip(page * LOCKER_PAGE_SIZE)
        .take(LOCKER_PAGE_SIZE)
        .enumerate()
    {
        let x = 195;
        let y = 128 + index * 57;
        blend_rect(pixels, x, y, 512, 50, [45, 45, 43, 240]);
        stroke_rect(pixels, x, y, 512, 50, [117, 99, 71, 255]);
        if let Some((icon_width, icon_height, icon_pixels)) =
            super::inventory_item_icon(item.item_id, item_archive, character_archive, icon_cache)
        {
            super::composite_inventory_icon(
                pixels,
                WIDTH,
                HEIGHT,
                &icon_pixels,
                icon_width as usize,
                icon_height as usize,
                x + 7,
                y + 5,
                40,
            );
        }
        let name = super::item_name(item.item_id, string_archive, name_cache);
        add_text(
            font,
            pixels,
            &name,
            (x + 58) as i32,
            (y + 8) as i32,
            11.0,
            322,
            17,
            [242, 235, 219],
        );
        add_text(
            font,
            pixels,
            &format!("Count {}  ·  SN {}", item.count, item.commodity_sn),
            (x + 58) as i32,
            (y + 27) as i32,
            9.0,
            322,
            14,
            [186, 175, 153],
        );
        let pending = state.pending_withdrawal == Some(item.instance_id);
        draw_button(
            pixels,
            x + 404,
            y + 9,
            100,
            32,
            if pending { "WAITING" } else { "WITHDRAW" },
            font,
            !pending,
        );
        hit_regions.push(region(
            Rect::new((x + 404) as f32, (y + 9) as f32, 100.0, 32.0),
            HitAction::LockerItem(page * LOCKER_PAGE_SIZE + index),
        ));
    }
    if items.is_empty() {
        add_text(
            font,
            pixels,
            "Your Cash Shop locker is empty.",
            325,
            322,
            15.0,
            300,
            28,
            [215, 204, 182],
        );
    }
    draw_page_controls(
        font,
        page,
        page_count,
        items.len(),
        LOCKER_PAGE_SIZE,
        pixels,
        hit_regions,
    );
}

fn draw_balances(font: &fontdue::Font, state: &super::CashShopUi, pixels: &mut [u8]) {
    for (index, (label, balance)) in ["NX Credit", "Maple Points", "NX Prepaid"]
        .into_iter()
        .zip(state.balances)
        .enumerate()
    {
        let x = 205 + index * 176;
        add_text(
            font,
            pixels,
            &format!("{label}: {}", grouped_number(balance)),
            x as i32,
            662,
            10.0,
            166,
            16,
            [247, 229, 193],
        );
    }
    add_text(
        font,
        pixels,
        state.notice.as_deref().unwrap_or(if state.entered {
            "Purchases and locker changes are confirmed by the server."
        } else {
            "Waiting for Cash Shop data from the server…"
        }),
        205,
        687,
        10.0,
        780,
        28,
        [232, 222, 204],
    );
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
    blend_rect(pixels, 304, 250, 416, 228, [40, 38, 35, 255]);
    stroke_rect(pixels, 304, 250, 416, 228, [219, 181, 111, 255]);
    add_text(
        font,
        pixels,
        "Confirm purchase",
        336,
        278,
        18.0,
        350,
        28,
        [255, 237, 198],
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
        [231, 223, 209],
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
        [64, 46, 28],
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

fn draw_tab(font: &fontdue::Font, pixels: &mut [u8], label: &str, x: usize, selected: bool) {
    blend_rect(
        pixels,
        x,
        37,
        if label == "CATALOG" { 88 } else { 112 },
        29,
        if selected {
            [116, 85, 48, 248]
        } else {
            [49, 48, 44, 238]
        },
    );
    stroke_rect(
        pixels,
        x,
        37,
        if label == "CATALOG" { 88 } else { 112 },
        29,
        if selected {
            [223, 181, 102, 255]
        } else {
            [120, 106, 81, 255]
        },
    );
    add_text(
        font,
        pixels,
        label,
        (x + 9) as i32,
        45,
        10.0,
        100,
        16,
        [255, 241, 211],
    );
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

fn catalog_card_rect(index: usize) -> Rect {
    let column = index % GRID_COLUMNS;
    let row = index / GRID_COLUMNS;
    Rect::new(
        (176 + column * 190) as f32,
        (82 + row * 128) as f32,
        185.0,
        124.0,
    )
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
