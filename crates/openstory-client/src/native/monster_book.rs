//! Monster Book panel rendering from the v83 NX UI and item data.
//!
//! The old client draws an interactive book with animated monster details, set
//! rewards, and a text search dialog. This module renders its cover, card pages,
//! category filters, set progress, search dialog, and animated detail view. The
//! caller owns interaction state and uploads the returned software-composited
//! panel.

use fontdue::Font;
use openstory_assets::{Bitmap, NodeValue, NxArchive, NxNode};
use std::collections::HashMap;
use std::sync::Arc;

use super::{GpuTexture, rasterize_ui_glyph};

const VIEW_WIDTH: f32 = 1024.0;
const UI_POSITION_X: f32 = 300.0;
const UI_POSITION_Y: f32 = 20.0;
const CARD_COLUMNS: usize = 5;
const CARDS_PER_SIDE: usize = 25;
const CARDS_PER_PAGE: usize = 50;
const DROP_COLUMNS: usize = 4;
const DROP_VISIBLE_ROWS: usize = 3;
const DROP_CELL_SIZE: f32 = 40.0;
const MAX_CARD_DROPS: usize = 256;

pub(super) fn set_scroll_max() -> usize {
    CARD_SET_DEFINITIONS.len().div_ceil(3).saturating_sub(3)
}

pub(super) fn drop_scroll_max(drop_count: usize) -> usize {
    drop_count
        .div_ceil(DROP_COLUMNS)
        .saturating_sub(DROP_VISIBLE_ROWS)
}

#[derive(Clone, Copy, Debug)]
pub(super) struct HitRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl HitRect {
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct CardHit {
    pub bounds: HitRect,
    pub item_id: i32,
    pub mob_id: i32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct CategoryHit {
    pub bounds: HitRect,
    pub category: u8,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct SetHit {
    pub bounds: HitRect,
    pub index: usize,
}

pub(super) struct Panel {
    pub width: u32,
    pub height: u32,
    pub x: f32,
    pub y: f32,
    pub pixels: Vec<u8>,
    pub close: HitRect,
    pub previous_page: Option<HitRect>,
    pub next_page: Option<HitRect>,
    pub search: Option<HitRect>,
    pub set_effect: Option<HitRect>,
    pub set_cells: Vec<SetHit>,
    pub categories: Vec<CategoryHit>,
    pub cards: Vec<CardHit>,
    pub detail_back: Option<HitRect>,
    pub detail_animation_area: Option<HitRect>,
    pub drop_area: Option<HitRect>,
    pub drop_count: usize,
    pub page: usize,
}

pub(super) struct State {
    pub open: bool,
    pub texture: Option<GpuTexture>,
    pub x: f32,
    pub y: f32,
    pub width: u32,
    pub height: u32,
    pub close: Option<HitRect>,
    pub previous_page: Option<HitRect>,
    pub next_page: Option<HitRect>,
    pub search: Option<HitRect>,
    pub set_effect: Option<HitRect>,
    pub set_cells: Vec<SetHit>,
    pub categories: Vec<CategoryHit>,
    pub cards: Vec<CardHit>,
    pub detail_back: Option<HitRect>,
    pub detail_animation_area: Option<HitRect>,
    pub drop_area: Option<HitRect>,
    pub drop_count: usize,
    pub drop_scroll: usize,
    pub page: usize,
    pub category: Option<u8>,
    pub selected_card: Option<i32>,
    pub search_open: bool,
    pub search_query: String,
    pub search_notice: Option<String>,
    pub set_effect_open: bool,
    pub set_scroll: usize,
    pub selected_set: Option<usize>,
    pub cover_item_id: i32,
    pub card_updates: Vec<(i32, i8)>,
    pub set_card_icons: Option<std::collections::HashMap<i32, NxNode>>,
    detail_animation: DetailAnimation,
    detail_sprite: Option<AnimationSpriteBounds>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            open: false,
            texture: None,
            x: 0.0,
            y: 0.0,
            width: 0,
            height: 0,
            close: None,
            previous_page: None,
            next_page: None,
            search: None,
            set_effect: None,
            set_cells: Vec::new(),
            categories: Vec::new(),
            cards: Vec::new(),
            detail_back: None,
            detail_animation_area: None,
            drop_area: None,
            drop_count: 0,
            drop_scroll: 0,
            page: 0,
            category: None,
            selected_card: None,
            search_open: false,
            search_query: String::new(),
            search_notice: None,
            set_effect_open: false,
            set_scroll: 0,
            selected_set: None,
            cover_item_id: 0,
            card_updates: Vec::new(),
            set_card_icons: None,
            detail_animation: DetailAnimation::default(),
            detail_sprite: None,
        }
    }
}

impl State {
    pub fn set_layout(&mut self, panel: Panel) {
        self.x = panel.x;
        self.y = panel.y;
        self.width = panel.width;
        self.height = panel.height;
        self.close = Some(panel.close);
        self.previous_page = panel.previous_page;
        self.next_page = panel.next_page;
        self.search = panel.search;
        self.set_effect = panel.set_effect;
        self.set_cells = panel.set_cells;
        self.categories = panel.categories;
        self.cards = panel.cards;
        self.detail_back = panel.detail_back;
        self.detail_animation_area = panel.detail_animation_area;
        self.drop_area = panel.drop_area;
        self.drop_count = panel.drop_count;
        self.page = panel.page;
    }

    pub fn select_card(&mut self, item_id: i32, mob_id: i32, mob_archive: &NxArchive) {
        self.selected_card = Some(item_id);
        self.detail_animation = DetailAnimation::for_mob(mob_archive, mob_id);
        self.detail_sprite = None;
    }

    pub fn clear_selected_card(&mut self) {
        self.selected_card = None;
        self.detail_animation = DetailAnimation::default();
        self.detail_sprite = None;
    }

    pub fn cycle_detail_animation(&mut self) {
        self.detail_animation.cycle();
        self.detail_sprite = None;
    }

    pub fn advance_detail_animation(&mut self, elapsed_ms: u64) -> bool {
        let changed = self.selected_card.is_some() && self.detail_animation.advance(elapsed_ms);
        if changed {
            self.detail_sprite = None;
        }
        changed
    }

    pub fn current_detail_animation(&self) -> &DetailAnimation {
        &self.detail_animation
    }

    pub fn refresh_detail_animation_sprite(
        &mut self,
        mob_archive: &NxArchive,
    ) -> Result<Option<AnimationSpritePixels>, String> {
        let Some(sprite) = self.detail_animation.current_sprite() else {
            self.detail_sprite = None;
            return Ok(None);
        };
        let bitmap = resolved_bitmap(mob_archive, sprite, "Monster Book detail animation")?;
        let pixels = bitmap
            .decode_bgra_shared()
            .map_err(|error| error.to_string())?;
        let origin = vector_origin(sprite)
            .or_else(|| bitmap_origin(mob_archive, sprite, "Monster Book detail animation").ok())
            .unwrap_or((0, 0));
        self.detail_sprite = Some(AnimationSpriteBounds {
            width: u32::from(bitmap.width()),
            height: u32::from(bitmap.height()),
            origin_x: origin.0 as f32,
            origin_y: origin.1 as f32,
        });
        Ok(Some(AnimationSpritePixels {
            width: u32::from(bitmap.width()),
            height: u32::from(bitmap.height()),
            pixels,
        }))
    }

    pub fn current_detail_animation_draw(&self) -> Option<AnimationSpriteDraw> {
        let sprite = self.detail_sprite?;
        let (opacity, scale) = self.detail_animation.current_transform();
        Some(AnimationSpriteDraw {
            x: UI_POSITION_X + 137.0 - sprite.origin_x * scale,
            y: UI_POSITION_Y + 155.0 - sprite.origin_y * scale,
            width: sprite.width as f32 * scale,
            height: sprite.height as f32 * scale,
            opacity: opacity / 255.0,
        })
    }

    pub fn add_card(&mut self, item_id: i32, level: i8) {
        if item_id <= 0 {
            return;
        }
        if let Some((_, current_level)) = self
            .card_updates
            .iter_mut()
            .find(|(current_item_id, _)| *current_item_id == item_id)
        {
            *current_level = level;
        } else {
            self.card_updates.push((item_id, level));
        }
        self.card_updates.sort_unstable_by_key(|(id, _)| *id);
    }

    pub fn set_cover(&mut self, item_id: i32) {
        self.cover_item_id = item_id.max(0);
    }
}

#[derive(Clone)]
struct DetailFrame {
    sprite: Option<NxNode>,
    delay_ms: u64,
    opacity_start: f32,
    opacity_end: f32,
    scale_start: f32,
    scale_end: f32,
}

#[derive(Clone, Copy)]
struct AnimationSpriteBounds {
    width: u32,
    height: u32,
    origin_x: f32,
    origin_y: f32,
}

pub(super) struct AnimationSpritePixels {
    pub width: u32,
    pub height: u32,
    pub pixels: Arc<[u8]>,
}

pub(super) struct AnimationSpriteDraw {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub opacity: f32,
}

#[derive(Clone)]
struct AnimationClip {
    frames: Vec<DetailFrame>,
    zigzag: bool,
}

impl AnimationClip {
    fn blank() -> Self {
        Self {
            frames: vec![DetailFrame {
                sprite: None,
                delay_ms: 100,
                opacity_start: 255.0,
                opacity_end: 255.0,
                scale_start: 100.0,
                scale_end: 100.0,
            }],
            zigzag: false,
        }
    }
}

pub(super) struct DetailAnimation {
    modes: [AnimationClip; 3],
    mode: usize,
    frame: usize,
    frame_step: isize,
    elapsed_ms: u64,
}

impl Default for DetailAnimation {
    fn default() -> Self {
        Self {
            modes: [
                AnimationClip::blank(),
                AnimationClip::blank(),
                AnimationClip::blank(),
            ],
            mode: 0,
            frame: 0,
            frame_step: 1,
            elapsed_ms: 0,
        }
    }
}

impl DetailAnimation {
    fn for_mob(mob_archive: &NxArchive, mob_id: i32) -> Self {
        if mob_id <= 0 {
            return Self::default();
        }
        let mob_path = format!("{mob_id:07}.img");
        let Ok(Some(mob)) = mob_archive.root().resolve(&mob_path) else {
            return Self::default();
        };

        let mut stand = load_animation_clip(&mob, "stand");
        if stand.frames.is_empty() {
            stand = load_animation_clip(&mob, "fly");
        }
        if stand.frames.is_empty() {
            stand = AnimationClip::blank();
        }
        let mut movement = load_animation_clip(&mob, "move");
        if movement.frames.is_empty() {
            movement = stand.clone();
        }
        let mut die = load_animation_clip(&mob, "die1");
        if die.frames.is_empty() {
            die = AnimationClip::blank();
        }
        Self {
            modes: [stand, movement, die],
            mode: 0,
            frame: 0,
            frame_step: 1,
            elapsed_ms: 0,
        }
    }

    fn cycle(&mut self) {
        self.mode = (self.mode + 1) % self.modes.len();
        self.frame = 0;
        self.frame_step = 1;
        self.elapsed_ms = 0;
    }

    fn advance(&mut self, elapsed_ms: u64) -> bool {
        let current_clip = &self.modes[self.mode];
        let has_single_frame_curve = current_clip.frames.first().is_some_and(|frame| {
            frame.opacity_start != frame.opacity_end || frame.scale_start != frame.scale_end
        });
        if self.mode != 2 && current_clip.frames.len() <= 1 && !has_single_frame_curve {
            self.elapsed_ms = 0;
            return false;
        }
        let starting_frame = (self.mode, self.frame);
        let mut remaining = self.elapsed_ms.saturating_add(elapsed_ms.min(1_000));
        loop {
            let clip = &self.modes[self.mode];
            let frame_count = clip.frames.len();
            let zigzag = clip.zigzag && frame_count > 1;
            let delay_ms = clip
                .frames
                .get(self.frame)
                .map_or(100, |frame| frame.delay_ms.max(1));
            if remaining < delay_ms {
                break;
            }
            remaining -= delay_ms;
            let ended = if zigzag {
                if self.frame_step == 1 && self.frame + 1 == frame_count {
                    self.frame_step = -1;
                    false
                } else if self.frame_step == -1 && self.frame == 0 {
                    self.frame_step = 1;
                    true
                } else {
                    false
                }
            } else {
                self.frame + 1 == frame_count
            };
            if self.mode == 2 && ended {
                self.mode = 0;
                self.frame = 0;
                self.frame_step = 1;
                remaining = 0;
                break;
            }
            if zigzag {
                self.frame = self.frame.saturating_add_signed(self.frame_step);
            } else {
                self.frame = (self.frame + 1) % frame_count;
            }
        }
        self.elapsed_ms = remaining;
        starting_frame != (self.mode, self.frame)
    }

    fn current_sprite(&self) -> Option<&NxNode> {
        self.modes
            .get(self.mode)?
            .frames
            .get(self.frame)?
            .sprite
            .as_ref()
    }

    fn current_transform(&self) -> (f32, f32) {
        let Some(frame) = self
            .modes
            .get(self.mode)
            .and_then(|clip| clip.frames.get(self.frame))
        else {
            return (255.0, 1.0);
        };
        let fraction = (self.elapsed_ms as f32 / frame.delay_ms.max(1) as f32).clamp(0.0, 1.0);
        let mut opacity =
            frame.opacity_start + (frame.opacity_end - frame.opacity_start) * fraction;
        let scale = frame.scale_start + (frame.scale_end - frame.scale_start) * fraction;
        if scale < 0.0 {
            opacity = 0.0;
        }
        (opacity.clamp(0.0, 255.0), (scale.max(0.0) / 100.0))
    }
}

fn load_animation_clip(mob: &NxNode, stance: &str) -> AnimationClip {
    let Ok(Some(animation)) = mob.child(stance) else {
        return AnimationClip {
            frames: Vec::new(),
            zigzag: false,
        };
    };
    let zigzag = integer_value(
        animation
            .child("zigzag")
            .ok()
            .flatten()
            .as_ref()
            .unwrap_or(&animation),
    )
    .is_some_and(|value| value != 0);
    if animation.bitmap().ok().flatten().is_some() {
        let (opacity_start, opacity_end, scale_start, scale_end) = animation_values(&animation);
        return AnimationClip {
            frames: vec![DetailFrame {
                delay_ms: animation_delay(&animation),
                sprite: Some(animation),
                opacity_start,
                opacity_end,
                scale_start,
                scale_end,
            }],
            zigzag,
        };
    }
    let Ok(children) = animation.children() else {
        return AnimationClip {
            frames: Vec::new(),
            zigzag,
        };
    };
    let mut frames = children
        .into_iter()
        .filter_map(|frame| {
            let index = frame.name().ok()?.parse::<usize>().ok()?;
            let sprite = if frame.bitmap().ok().flatten().is_some()
                || frame.child("source").ok().flatten().is_some()
            {
                Some(frame.clone())
            } else {
                frame.child("0").ok().flatten().filter(|nested| {
                    nested.bitmap().ok().flatten().is_some()
                        || nested.child("source").ok().flatten().is_some()
                })
            }?;
            let (opacity_start, opacity_end, scale_start, scale_end) = animation_values(&frame);
            Some((
                index,
                DetailFrame {
                    sprite: Some(sprite),
                    delay_ms: animation_delay(&frame),
                    opacity_start,
                    opacity_end,
                    scale_start,
                    scale_end,
                },
            ))
        })
        .collect::<Vec<_>>();
    frames.sort_unstable_by_key(|(index, _)| *index);
    AnimationClip {
        frames: frames.into_iter().map(|(_, frame)| frame).collect(),
        zigzag,
    }
}

fn animation_delay(frame: &NxNode) -> u64 {
    integer_value(
        frame
            .child("delay")
            .ok()
            .flatten()
            .as_ref()
            .unwrap_or(frame),
    )
    .filter(|delay| *delay > 0)
    .map_or(100, |delay| delay as u64)
}

fn animation_values(frame: &NxNode) -> (f32, f32, f32, f32) {
    let (opacity_start, opacity_end) = animation_property_range(frame, "a0", "a1", 255.0);
    let (scale_start, scale_end) = animation_property_range(frame, "z0", "z1", 100.0);
    (opacity_start, opacity_end, scale_start, scale_end)
}

fn animation_property_range(
    frame: &NxNode,
    start_name: &str,
    end_name: &str,
    default: f32,
) -> (f32, f32) {
    let start = integer_value(
        frame
            .child(start_name)
            .ok()
            .flatten()
            .as_ref()
            .unwrap_or(frame),
    )
    .map(|value| value as f32);
    let end = integer_value(
        frame
            .child(end_name)
            .ok()
            .flatten()
            .as_ref()
            .unwrap_or(frame),
    )
    .map(|value| value as f32);
    match (start, end) {
        (Some(start), Some(end)) => (start, end),
        (Some(value), None) | (None, Some(value)) => (value, value),
        (None, None) => (default, default),
    }
}

pub(super) fn render(
    ui_archive: &NxArchive,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    mob_archive: &NxArchive,
    string_archive: &NxArchive,
    cards: &[(i32, i8)],
    set_card_icons: &std::collections::HashMap<i32, NxNode>,
    cover_item_id: i32,
    page: usize,
    category: Option<u8>,
    selected_card: Option<i32>,
    detail_animation: Option<&DetailAnimation>,
    search_open: bool,
    search_query: &str,
    search_notice: Option<&str>,
    set_effect_open: bool,
    set_scroll: usize,
    selected_set: Option<usize>,
    drop_scroll: usize,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    font: &Font,
) -> Result<Panel, String> {
    let ui_root = ui_archive.root();
    let book = required(&ui_root, "UIWindow2.img/MonsterBook")?;
    let background_node = required(&book, "backgrnd")?;
    let background = bitmap_info(ui_archive, &background_node, "MonsterBook/backgrnd")?;
    let background_width = background.width();
    let background_height = background.height();
    if background_width == 0
        || background_height == 0
        || background_width > 2048
        || background_height > 2048
    {
        return Err(format!(
            "Monster Book background has invalid dimensions {}x{}",
            background_width, background_height
        ));
    }
    let width = background_width;
    let height = background_height;
    let panel_x = (VIEW_WIDTH - width as f32) / 2.0;
    let panel_y = UI_POSITION_Y;
    let ui_anchor_x = UI_POSITION_X - panel_x;
    let mut canvas = Canvas::new(width, height);
    let mut categories = Vec::new();
    let mut card_hits = Vec::new();

    canvas.draw_node(
        ui_archive,
        &background_node,
        "MonsterBook/backgrnd",
        ui_anchor_x,
        0.0,
    )?;

    let all_entries = load_cards(item_archive, mob_archive, string_archive, cards);
    let mut entries = all_entries.clone();
    entries.retain(|entry| category.is_none_or(|filter| entry.category == filter));
    let category_totals = count_categories(item_archive);
    let mut category_collected = [0_usize; 9];
    for entry in &all_entries {
        category_collected[usize::from(entry.category)] += 1;
    }
    let total_cards = cards.iter().filter(|(id, _)| *id > 0).count();
    let total_possible: usize = category_totals.iter().sum();
    let num_pages = if entries.len() <= CARDS_PER_SIDE {
        1
    } else {
        1 + (entries.len() - CARDS_PER_SIDE).div_ceil(CARDS_PER_PAGE)
    };
    let page = page.min(num_pages.saturating_sub(1));
    let detail_open =
        selected_card.is_some_and(|selected| entries.iter().any(|entry| entry.item_id == selected));

    let mut set_cells = Vec::new();
    let mut drop_area = None;
    let mut drop_count = 0;
    let mut detail_animation_area = None;
    if set_effect_open {
        draw_set_effect(
            &mut canvas,
            &book,
            ui_archive,
            item_archive,
            mob_archive,
            &all_entries,
            set_card_icons,
            selected_set,
            set_scroll,
            font,
            ui_anchor_x,
            panel_x,
            panel_y,
            &mut set_cells,
        )?;
    } else if let Some(selected) = selected_card
        && let Some(entry) = entries.iter().find(|entry| entry.item_id == selected)
    {
        let count = draw_detail(
            &mut canvas,
            &book,
            ui_archive,
            item_archive,
            character_archive,
            mob_archive,
            string_archive,
            entry,
            drop_scroll,
            detail_animation,
            icon_cache,
            font,
            ui_anchor_x,
        )?;
        drop_area = Some(HitRect {
            x: panel_x + ui_anchor_x + 255.0,
            y: panel_y + 165.0,
            width: DROP_COLUMNS as f32 * DROP_CELL_SIZE,
            height: DROP_VISIBLE_ROWS as f32 * DROP_CELL_SIZE,
        });
        drop_count = count;
        detail_animation_area = Some(HitRect {
            x: panel_x + ui_anchor_x + 80.0,
            y: panel_y + 67.0,
            width: 114.0,
            height: 130.0,
        });
    } else {
        draw_book_pages(
            &mut canvas,
            &book,
            ui_archive,
            item_archive,
            &entries,
            cover_item_id,
            page,
            num_pages,
            total_cards,
            total_possible,
            &category_collected,
            &category_totals,
            font,
            ui_anchor_x,
            panel_x,
            panel_y,
            &mut card_hits,
        )?;
    }

    let close = draw_button(
        &mut canvas,
        ui_archive,
        &book,
        "BtClose/normal/0",
        ui_anchor_x + 440.0,
        6.0,
        panel_x,
        panel_y,
    )?
    .unwrap_or(HitRect {
        x: panel_x + width as f32 - 58.0,
        y: panel_y + 6.0,
        width: 28.0,
        height: 20.0,
    });

    let previous_page = if page > 0 {
        draw_button(
            &mut canvas,
            ui_archive,
            &book,
            "arrowLeft/normal/0",
            ui_anchor_x + 280.0,
            290.0,
            panel_x,
            panel_y,
        )?
    } else {
        None
    };
    let next_page = if page + 1 < num_pages {
        draw_button(
            &mut canvas,
            ui_archive,
            &book,
            "arrowRight/normal/0",
            ui_anchor_x + 400.0,
            290.0,
            panel_x,
            panel_y,
        )?
    } else {
        // Draw the disabled right arrow if present. It remains non-interactive.
        let _ = draw_button(
            &mut canvas,
            ui_archive,
            &book,
            "arrowRight/disabled/0",
            ui_anchor_x + 400.0,
            290.0,
            panel_x,
            panel_y,
        )?;
        None
    };

    let search = draw_button(
        &mut canvas,
        ui_archive,
        &book,
        "BtSearch/normal/0",
        ui_anchor_x,
        0.0,
        panel_x,
        panel_y,
    )?;
    let set_effect = draw_button(
        &mut canvas,
        ui_archive,
        &book,
        "BtSetEffect/normal/0",
        ui_anchor_x,
        0.0,
        panel_x,
        panel_y,
    )?;

    if search_open {
        draw_search_dialog(&mut canvas, font, search_query, search_notice);
    }

    for side in ["LeftTab", "RightTab"] {
        for category_index in 0..9 {
            let state = if category == Some(category_index) {
                "selected/0"
            } else {
                "normal/0"
            };
            let path = format!("{side}/{category_index}/{state}");
            if let Some(bounds) = draw_button(
                &mut canvas,
                ui_archive,
                &book,
                &path,
                ui_anchor_x,
                0.0,
                panel_x,
                panel_y,
            )? {
                categories.push(CategoryHit {
                    bounds,
                    category: category_index as u8,
                });
            }
        }
    }

    let detail_back = detail_open.then_some(HitRect {
        x: panel_x + 18.0,
        y: panel_y + 32.0,
        width: width as f32 - 36.0,
        height: height as f32 - 42.0,
    });

    Ok(Panel {
        width: width as u32,
        height: height as u32,
        x: panel_x,
        y: panel_y,
        pixels: canvas.pixels,
        close,
        previous_page,
        next_page,
        search,
        set_effect,
        set_cells,
        categories,
        cards: card_hits,
        detail_back,
        detail_animation_area,
        drop_area,
        drop_count,
        page,
    })
}

#[derive(Clone)]
struct CardEntry {
    item_id: i32,
    level: i8,
    mob_id: i32,
    mob_name: String,
    category: u8,
}

fn load_cards(
    item_archive: &NxArchive,
    mob_archive: &NxArchive,
    string_archive: &NxArchive,
    cards: &[(i32, i8)],
) -> Vec<CardEntry> {
    let mut cards = cards
        .iter()
        .filter(|(item_id, _)| *item_id > 0)
        .map(|(item_id, level)| {
            let card_path = card_item_path(*item_id);
            let mob_id = item_archive
                .root()
                .resolve(&format!("{card_path}/info/mob"))
                .ok()
                .flatten()
                .and_then(|node| integer_value(&node));
            let mob_name = mob_id
                .and_then(|mob_id| {
                    string_archive
                        .root()
                        .resolve(&format!("Mob.img/{mob_id}/name"))
                        .ok()
                        .flatten()
                        .and_then(|node| string_value(&node))
                })
                .or_else(|| item_name(string_archive, *item_id))
                .unwrap_or_else(|| format!("Monster Card {item_id}"));
            let mob_present = mob_id.is_some_and(|mob_id| {
                mob_archive
                    .root()
                    .resolve(&format!("{mob_id:07}.img"))
                    .ok()
                    .flatten()
                    .is_some()
            });
            CardEntry {
                item_id: *item_id,
                level: *level,
                mob_id: if mob_present {
                    i32::try_from(mob_id.unwrap_or_default()).unwrap_or_default()
                } else {
                    0
                },
                mob_name,
                category: category_for(*item_id),
            }
        })
        .collect::<Vec<_>>();
    cards.sort_unstable_by_key(|entry| entry.item_id);
    cards
}

pub(super) fn find_card_page(
    item_archive: &NxArchive,
    mob_archive: &NxArchive,
    string_archive: &NxArchive,
    cards: &[(i32, i8)],
    category: Option<u8>,
    query: &str,
) -> Option<usize> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return None;
    }
    let entries = load_cards(item_archive, mob_archive, string_archive, cards);
    let index = entries.iter().enumerate().find_map(|(index, entry)| {
        if category.is_some_and(|category| category != entry.category) {
            return None;
        }
        let name_matches = entry.mob_name.to_lowercase().contains(&query);
        let id_matches = entry.item_id.to_string().contains(&query);
        (name_matches || id_matches).then_some(index)
    })?;
    Some(if index < CARDS_PER_SIDE {
        0
    } else {
        1 + (index - CARDS_PER_SIDE) / CARDS_PER_PAGE
    })
}

struct CardSetDefinition {
    name: &'static str,
    bonus: &'static str,
    region: u8,
    mobs: &'static [i32],
}

const CARD_SET_DEFINITIONS: &[CardSetDefinition] = &[
    CardSetDefinition {
        name: "Victoria Island",
        bonus: "+50 HP",
        region: 0,
        mobs: &[100100, 100101, 120100, 130100],
    },
    CardSetDefinition {
        name: "Kerning City",
        bonus: "+20 Avoid",
        region: 0,
        mobs: &[
            2230100, 2230101, 2230102, 2230103, 2230104, 2230105, 2230106, 2230107, 2230108,
            2230109, 2230110,
        ],
    },
    CardSetDefinition {
        name: "Ellinia",
        bonus: "+50 MP",
        region: 0,
        mobs: &[
            210100, 100130, 100131, 100132, 100133, 100134, 1210100, 1210101, 1210102, 1210103,
        ],
    },
    CardSetDefinition {
        name: "Sleepywood",
        bonus: "+5 ATK, +50 DEF",
        region: 0,
        mobs: &[
            4130100, 4130101, 4130102, 4130103, 4130104, 4130105, 6130100, 6130101, 6130102,
        ],
    },
    CardSetDefinition {
        name: "Perion",
        bonus: "+50 HP",
        region: 0,
        mobs: &[
            130101, 1110100, 1110101, 1110130, 1120100, 1130100, 1140100, 1140130, 2100100,
            2100101, 2100102, 2100103, 2100104, 2100105, 2100106, 2100107, 2100108,
        ],
    },
    CardSetDefinition {
        name: "Henesys",
        bonus: "+20 ACC",
        region: 0,
        mobs: &[
            1210100, 2110200, 2110300, 2110301, 2130100, 2130103, 3000000, 3000001, 3000005,
            3000006, 3100101, 3100102,
        ],
    },
    CardSetDefinition {
        name: "Orbis",
        bonus: "+3 Speed, +10 Jump",
        region: 1,
        mobs: &[
            3210100, 3210200, 3210201, 3210202, 3210203, 3210204, 3210205, 3210206, 3210207,
            3210208,
        ],
    },
    CardSetDefinition {
        name: "El Nath",
        bonus: "+5 ATK, +10 DEX",
        region: 2,
        mobs: &[
            5100100, 5100101, 5100102, 5100103, 5100104, 5105100, 5105101, 5105102, 5120100,
            5120101,
        ],
    },
    CardSetDefinition {
        name: "Dead Mine",
        bonus: "+6 ATK%, +10 MATK",
        region: 2,
        mobs: &[
            5130100, 5130101, 5130102, 5130103, 5130104, 5130105, 5130106, 5130107, 5130108,
        ],
    },
    CardSetDefinition {
        name: "Ludibrium",
        bonus: "+100 HP, +100 DEF",
        region: 3,
        mobs: &[
            3210450, 3210800, 3220000, 3230100, 3230101, 3230102, 3230200, 3230300, 3230301,
            3230302,
        ],
    },
    CardSetDefinition {
        name: "Aqua Road",
        bonus: "+6 Magic%, +100 MDEF",
        region: 3,
        mobs: &[
            2230131, 2230200, 2300100, 3110100, 3110101, 3110102, 3110300, 3110301, 3110302,
            3110303,
        ],
    },
    CardSetDefinition {
        name: "Mu Lung",
        bonus: "+15% Ignore DEF",
        region: 4,
        mobs: &[
            5090100, 6090000, 6090001, 6090002, 6090003, 6090004, 6130200, 6230100, 6230200,
            6300100,
        ],
    },
    CardSetDefinition {
        name: "Leafre",
        bonus: "+1 All Skills",
        region: 5,
        mobs: &[
            8140100, 8140101, 8140102, 8140103, 8140104, 8140105, 8140110, 8140111, 8140112,
            8143000,
        ],
    },
    CardSetDefinition {
        name: "Temple of Time",
        bonus: "+47 HP/MP Recovery",
        region: 5,
        mobs: &[
            8200000, 8200001, 8200002, 8200003, 8200004, 8200005, 8200006, 8200007, 8200008,
            8200009,
        ],
    },
    CardSetDefinition {
        name: "Nihal Desert",
        bonus: "+50 DEF, +50 MDEF",
        region: 6,
        mobs: &[
            7130000, 7130001, 7130002, 7130003, 7130004, 7130005, 7130100, 7130101, 7130200,
            7130300,
        ],
    },
    CardSetDefinition {
        name: "Magatia",
        bonus: "+10 INT, +5 MATK",
        region: 6,
        mobs: &[
            7130400, 7130401, 7130402, 7130500, 7130501, 7130600, 7130601, 7140000, 7140100,
        ],
    },
    CardSetDefinition {
        name: "Mushroom Shrine",
        bonus: "+85 MP Recovery",
        region: 7,
        mobs: &[
            9400000, 9400001, 9400002, 9400003, 9400004, 9400005, 9400006, 9400007, 9400008,
        ],
    },
    CardSetDefinition {
        name: "Ninja Castle",
        bonus: "+6 Speed, +6 Jump",
        region: 7,
        mobs: &[
            9400100, 9400101, 9400102, 9400103, 9400104, 9400105, 9400106,
        ],
    },
    CardSetDefinition {
        name: "Showa Village",
        bonus: "+5 All Stats",
        region: 7,
        mobs: &[
            9400110, 9400111, 9400112, 9400113, 9400114, 9400115, 9400120, 9400121,
        ],
    },
];

pub(super) fn load_set_card_icons(
    item_archive: &NxArchive,
) -> std::collections::HashMap<i32, NxNode> {
    let wanted_mobs = CARD_SET_DEFINITIONS
        .iter()
        .flat_map(|definition| definition.mobs.iter().copied())
        .collect::<std::collections::HashSet<_>>();
    let mut icons = std::collections::HashMap::new();
    let root = item_archive.root();
    let Ok(Some(consume)) = root.resolve("Consume") else {
        return icons;
    };
    let Ok(images) = consume.children() else {
        return icons;
    };
    for image in images {
        let Ok(items) = image.children() else {
            continue;
        };
        for item in items {
            let Ok(Some(info)) = item.child("info") else {
                continue;
            };
            let Ok(Some(mob_node)) = info.child("mob") else {
                continue;
            };
            let Some(mob_id) =
                integer_value(&mob_node).and_then(|mob_id| i32::try_from(mob_id).ok())
            else {
                continue;
            };
            if !wanted_mobs.contains(&mob_id) || icons.contains_key(&mob_id) {
                continue;
            }
            if let Ok(Some(icon)) = info.child("iconRaw") {
                icons.insert(mob_id, icon);
            }
        }
        if icons.len() == wanted_mobs.len() {
            break;
        }
    }
    icons
}

fn draw_set_effect(
    canvas: &mut Canvas,
    book: &NxNode,
    ui_archive: &NxArchive,
    item_archive: &NxArchive,
    mob_archive: &NxArchive,
    cards: &[CardEntry],
    card_icons_by_mob: &std::collections::HashMap<i32, NxNode>,
    selected_set: Option<usize>,
    set_scroll: usize,
    font: &Font,
    ui_anchor_x: f32,
    panel_x: f32,
    panel_y: f32,
    set_hits: &mut Vec<SetHit>,
) -> Result<(), String> {
    if let Some(info) = optional(book, "pageInfo/setInfo0")? {
        canvas.draw_node(ui_archive, &info, "pageInfo/setInfo0", ui_anchor_x, 0.0)?;
    }
    if let Some(info) = optional(book, "pageInfo/setInfo1")? {
        canvas.draw_node(ui_archive, &info, "pageInfo/setInfo1", ui_anchor_x, 0.0)?;
    }

    let collected_mobs = cards
        .iter()
        .filter(|card| card.mob_id > 0)
        .map(|card| card.mob_id)
        .collect::<std::collections::HashSet<_>>();
    let set_progress = CARD_SET_DEFINITIONS
        .iter()
        .map(|definition| {
            let collected = definition
                .mobs
                .iter()
                .filter(|mob_id| collected_mobs.contains(mob_id))
                .count();
            (definition, collected)
        })
        .collect::<Vec<_>>();
    let score = set_progress
        .iter()
        .map(|(_, collected)| collected * 10)
        .sum::<usize>();
    let complete = set_progress
        .iter()
        .filter(|(definition, collected)| *collected >= definition.mobs.len())
        .count();

    let left_page_x = ui_anchor_x + 50.0;
    let right_page_x = ui_anchor_x + 250.0;
    let score_width = score.to_string().len() as f32 * 18.0;
    draw_number(
        canvas,
        ui_archive,
        book,
        font,
        "numberLarge",
        left_page_x + 87.0 - score_width / 2.0,
        135.0,
        score,
        true,
    )?;
    canvas.draw_text(
        font,
        &format!("{complete}/{}", set_progress.len()),
        right_page_x + 108.0,
        38.0,
        9.0,
        [255, 255, 255],
        Align::Left,
        56.0,
    );

    if let Some(index) = selected_set
        && let Some((definition, collected)) = set_progress.get(index)
    {
        draw_set_icon(
            canvas,
            book,
            ui_archive,
            item_archive,
            mob_archive,
            card_icons_by_mob,
            definition.mobs.first().copied(),
            left_page_x + 87.0,
            195.0,
        )?;
        canvas.draw_text(
            font,
            definition.name,
            left_page_x + 87.0,
            255.0,
            10.0,
            [24, 19, 12],
            Align::Center,
            170.0,
        );
        canvas.draw_text(
            font,
            definition.bonus,
            left_page_x + 87.0,
            275.0,
            10.0,
            [24, 19, 12],
            Align::Center,
            170.0,
        );
        canvas.draw_text(
            font,
            &format!("{collected} / {} cards", definition.mobs.len()),
            left_page_x + 87.0,
            294.0,
            9.0,
            [255, 255, 255],
            Align::Center,
            170.0,
        );
    }

    const GRID_COLUMNS: usize = 3;
    const VISIBLE_ROWS: usize = 3;
    const CELL_SIZE: f32 = 52.0;
    const CELL_STEP_X: f32 = 59.0;
    const CELL_STEP_Y: f32 = 68.0;
    let grid_x = right_page_x + 2.5;
    let grid_y = 57.0;
    let rows = set_progress.len().div_ceil(GRID_COLUMNS);
    let start_row = set_scroll.min(rows.saturating_sub(VISIBLE_ROWS));
    for (index, (definition, collected)) in set_progress.iter().enumerate() {
        let row = index / GRID_COLUMNS;
        if row < start_row || row >= start_row + VISIBLE_ROWS {
            continue;
        }
        let column = index % GRID_COLUMNS;
        let x = grid_x + column as f32 * CELL_STEP_X;
        let y = grid_y + (row - start_row) as f32 * CELL_STEP_Y;
        let path = "pageInfo/setIconBack";
        if let Some(icon_back) = optional(book, path)? {
            canvas.draw_node(
                ui_archive,
                &icon_back,
                path,
                x + CELL_SIZE / 2.0,
                y + CELL_SIZE / 2.0,
            )?;
        }
        let region_path = format!("icon/{}", definition.region);
        if let Some(region_icon) = optional(book, &region_path)? {
            canvas.draw_node(ui_archive, &region_icon, &region_path, x + 5.0, y + 5.0)?;
        }
        draw_set_icon(
            canvas,
            book,
            ui_archive,
            item_archive,
            mob_archive,
            card_icons_by_mob,
            definition.mobs.first().copied(),
            x + CELL_SIZE / 2.0,
            y + CELL_SIZE / 2.0,
        )?;
        canvas.draw_text(
            font,
            &format!("{collected}/{}", definition.mobs.len()),
            x + CELL_SIZE / 2.0,
            y + CELL_SIZE + 2.0,
            8.0,
            [255, 255, 255],
            Align::Center,
            CELL_SIZE,
        );
        set_hits.push(SetHit {
            bounds: HitRect {
                x: panel_x + x,
                y: panel_y + y,
                width: CELL_SIZE,
                height: CELL_SIZE,
            },
            index,
        });
    }

    if rows > VISIBLE_ROWS {
        canvas.draw_text(
            font,
            &format!(
                "Sets {}–{} of {}  •  Scroll to browse",
                start_row + 1,
                (start_row + VISIBLE_ROWS).min(rows),
                rows
            ),
            right_page_x + 87.0,
            286.0,
            8.0,
            [255, 255, 255],
            Align::Center,
            190.0,
        );
    }
    Ok(())
}

fn draw_set_icon(
    canvas: &mut Canvas,
    book: &NxNode,
    ui_archive: &NxArchive,
    item_archive: &NxArchive,
    mob_archive: &NxArchive,
    card_icons_by_mob: &std::collections::HashMap<i32, NxNode>,
    mob_id: Option<i32>,
    center_x: f32,
    center_y: f32,
) -> Result<(), String> {
    let Some(mob_id) = mob_id else {
        return Ok(());
    };
    if let Some(icon) = card_icons_by_mob.get(&mob_id)
        && canvas
            .draw_node(
                item_archive,
                icon,
                "monster-card-set/iconRaw",
                center_x,
                center_y,
            )
            .is_ok()
    {
        return Ok(());
    }

    let path = format!("{mob_id:07}.img");
    let Some(mob) = mob_archive
        .root()
        .resolve(&path)
        .map_err(|error| error.to_string())?
    else {
        if let Some(cover) = optional(book, "cover")? {
            canvas.draw_node(ui_archive, &cover, "cover", center_x, center_y)?;
        }
        return Ok(());
    };
    let sprite_path = if optional(&mob, "stand/0")?.is_some() {
        "stand/0"
    } else {
        "fly/0"
    };
    if let Some(sprite) = optional(&mob, sprite_path)? {
        canvas.draw_node(
            mob_archive,
            &sprite,
            &format!("{path}/{sprite_path}"),
            center_x,
            center_y,
        )?;
    }
    Ok(())
}

fn draw_search_dialog(canvas: &mut Canvas, font: &Font, query: &str, notice: Option<&str>) {
    let width = (canvas.width as f32 - 72.0).min(380.0);
    let x = (canvas.width as f32 - width) / 2.0;
    let y = (canvas.height as f32 - 94.0) / 2.0;
    canvas.fill_rect(x, y, width, 94.0, [18, 15, 11, 245]);
    canvas.fill_rect(x, y, width, 2.0, [181, 137, 56, 255]);
    canvas.fill_rect(x, y + 92.0, width, 2.0, [181, 137, 56, 255]);
    canvas.draw_text(
        font,
        "Search monster cards",
        x + 12.0,
        y + 8.0,
        12.0,
        [255, 226, 164],
        Align::Left,
        width - 24.0,
    );
    canvas.fill_rect(x + 12.0, y + 32.0, width - 24.0, 24.0, [36, 31, 23, 255]);
    canvas.draw_text(
        font,
        if query.is_empty() {
            "Type a monster name…"
        } else {
            query
        },
        x + 18.0,
        y + 36.0,
        12.0,
        if query.is_empty() {
            [160, 151, 133]
        } else {
            [255, 255, 255]
        },
        Align::Left,
        width - 36.0,
    );
    let help = notice.unwrap_or("Enter to search   •   Backspace to edit   •   Esc to close");
    canvas.draw_text(
        font,
        help,
        x + 12.0,
        y + 64.0,
        9.0,
        if notice.is_some() {
            [255, 173, 120]
        } else {
            [202, 194, 174]
        },
        Align::Left,
        width - 24.0,
    );
}

fn draw_book_pages(
    canvas: &mut Canvas,
    book: &NxNode,
    ui_archive: &NxArchive,
    item_archive: &NxArchive,
    entries: &[CardEntry],
    cover_item_id: i32,
    page: usize,
    num_pages: usize,
    total_cards: usize,
    total_possible: usize,
    category_collected: &[usize; 9],
    category_totals: &[usize; 9],
    font: &Font,
    ui_anchor_x: f32,
    panel_x: f32,
    panel_y: f32,
    card_hits: &mut Vec<CardHit>,
) -> Result<(), String> {
    let left_x = ui_anchor_x + 50.0;
    let page_y = 27.0;
    let right_x = ui_anchor_x + 247.0;

    if page == 0 {
        if let Some(info) = optional(book, "pageInfo/bookInfo0")? {
            canvas.draw_node(ui_archive, &info, "pageInfo/bookInfo0", ui_anchor_x, 0.0)?;
        }
        draw_number(
            canvas,
            ui_archive,
            book,
            font,
            "numberLarge",
            left_x + 50.0,
            page_y + 88.0,
            total_cards,
            true,
        )?;
        draw_number(
            canvas,
            ui_archive,
            book,
            font,
            "numberLarge",
            left_x + 110.0,
            page_y + 88.0,
            total_possible,
            true,
        )?;
        canvas.draw_text(
            font,
            "Cards",
            left_x + 145.0,
            page_y + 100.0,
            11.0,
            [15, 15, 15],
            Align::Center,
            90.0,
        );

        let order = [1_usize, 2, 3, 4, 5, 6, 7, 8, 0];
        for (index, category) in order.into_iter().enumerate() {
            let column = index % 2;
            let row = index / 2;
            let x = left_x + column as f32 * 87.0;
            let y = page_y + 132.0 + row as f32 * 27.0;
            let image_index = if category == 0 { 9 } else { category };
            let path = format!("pageInfo/cardCategory/{image_index}");
            if let Some(node) = optional(book, &path)? {
                canvas.draw_node(ui_archive, &node, &path, x, y)?;
            }
            draw_number(
                canvas,
                ui_archive,
                book,
                font,
                "numberSmall",
                x + 48.0,
                y + 8.0,
                category_collected[category],
                false,
            )?;
            draw_number(
                canvas,
                ui_archive,
                book,
                font,
                "numberSmall",
                x + 68.0,
                y + 8.0,
                category_totals[category],
                false,
            )?;
        }
        for index in 0..CARDS_PER_SIDE.min(entries.len()) {
            draw_card(
                canvas,
                ui_archive,
                item_archive,
                book,
                &entries[index],
                index,
                right_x,
                page_y,
                panel_x,
                panel_y,
                card_hits,
            )?;
        }
    } else {
        let first_index = CARDS_PER_SIDE + (page - 1) * CARDS_PER_PAGE;
        for side_index in 0..CARDS_PER_PAGE {
            let index = first_index + side_index;
            if index >= entries.len() {
                break;
            }
            let origin_x = if side_index < CARDS_PER_SIDE {
                left_x
            } else {
                right_x
            };
            let page_index = side_index % CARDS_PER_SIDE;
            draw_card(
                canvas,
                ui_archive,
                item_archive,
                book,
                &entries[index],
                page_index,
                origin_x,
                page_y,
                panel_x,
                panel_y,
                card_hits,
            )?;
        }
    }

    let _ = cover_item_id;
    canvas.draw_text(
        font,
        &format!("{} / {}", page + 1, num_pages),
        right_x + 87.0,
        page_y + 290.0,
        11.0,
        [255, 255, 255],
        Align::Center,
        80.0,
    );
    Ok(())
}

fn draw_card(
    canvas: &mut Canvas,
    ui_archive: &NxArchive,
    item_archive: &NxArchive,
    book: &NxNode,
    entry: &CardEntry,
    local_index: usize,
    page_origin_x: f32,
    page_origin_y: f32,
    panel_x: f32,
    panel_y: f32,
    card_hits: &mut Vec<CardHit>,
) -> Result<(), String> {
    let column = local_index % CARD_COLUMNS;
    let row = local_index / CARD_COLUMNS;
    let x = page_origin_x + column as f32 * 35.0;
    let y = page_origin_y + row as f32 * 43.0 + 50.0;
    let item_root = item_archive.root();
    let card_path = card_item_path(entry.item_id);
    let icon_node = item_root
        .resolve(&format!("{card_path}/info/iconRaw"))
        .map_err(|error| error.to_string())?;
    let mut drew_icon = false;
    if let Some(icon) = icon_node {
        if canvas
            .draw_node(
                item_archive,
                &icon,
                &format!("{card_path}/info/iconRaw"),
                x + 15.0,
                y + 21.0,
            )
            .is_ok()
        {
            drew_icon = true;
        }
    }
    if !drew_icon {
        if let Some(cover) = optional(book, "cover")? {
            let origin_x = vector_origin(&cover).map_or(0.0, |origin| origin.0 as f32);
            let origin_y = vector_origin(&cover).map_or(0.0, |origin| origin.1 as f32);
            canvas.draw_node(ui_archive, &cover, "cover", x + origin_x, y + origin_y)?;
        }
    }
    // The legacy card geometry is 31x42 inside a 35x43 grid cell.
    card_hits.push(CardHit {
        bounds: HitRect {
            x: panel_x + x,
            y: panel_y + y,
            width: 31.0,
            height: 42.0,
        },
        item_id: entry.item_id,
        mob_id: entry.mob_id,
    });
    Ok(())
}

fn draw_detail(
    canvas: &mut Canvas,
    book: &NxNode,
    ui_archive: &NxArchive,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    mob_archive: &NxArchive,
    string_archive: &NxArchive,
    entry: &CardEntry,
    drop_scroll: usize,
    detail_animation: Option<&DetailAnimation>,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    font: &Font,
    ui_anchor_x: f32,
) -> Result<usize, String> {
    if let Some(info) = optional(book, "pageInfo/monsterInfo")? {
        canvas.draw_node(ui_archive, &info, "pageInfo/monsterInfo", ui_anchor_x, 0.0)?;
    }
    let mob_id = entry.mob_id;
    let mob_path = format!("{mob_id:07}.img");
    let mob_root = mob_archive.root();
    let mob = if mob_id > 0 {
        mob_root
            .resolve(&mob_path)
            .map_err(|error| error.to_string())?
    } else {
        None
    };
    if detail_animation.is_none()
        && let Some(mob) = mob.as_ref()
    {
        let stand = optional(mob, "stand/0")?.or(optional(mob, "fly/0")?);
        if let Some(sprite) = stand {
            canvas.draw_node(
                mob_archive,
                &sprite,
                &format!("{mob_path}/stand/0"),
                ui_anchor_x + 137.0,
                155.0,
            )?;
        }
    }
    canvas.draw_text(
        font,
        &entry.mob_name,
        ui_anchor_x + 137.0,
        195.0,
        12.0,
        [20, 20, 20],
        Align::Center,
        160.0,
    );
    canvas.draw_text(
        font,
        &format!("Card level {}", entry.level),
        ui_anchor_x + 137.0,
        105.0,
        9.0,
        [91, 82, 67],
        Align::Center,
        120.0,
    );

    let info = mob
        .as_ref()
        .and_then(|mob| optional(mob, "info").ok().flatten());
    let stat = |name: &str| -> i64 {
        info.as_ref()
            .and_then(|info| optional(info, name).ok().flatten())
            .and_then(|node| integer_value(&node))
            .unwrap_or(0)
    };
    let hp = stat("maxHP");
    let mp = stat("maxMP");
    let level = stat("level");
    let exp = stat("exp");
    canvas.draw_text(
        font,
        &hp.to_string(),
        ui_anchor_x + 137.0,
        218.0,
        10.0,
        [225, 58, 48],
        Align::Center,
        150.0,
    );
    canvas.draw_text(
        font,
        &mp.to_string(),
        ui_anchor_x + 137.0,
        236.0,
        10.0,
        [72, 135, 225],
        Align::Center,
        150.0,
    );
    canvas.draw_text(
        font,
        &format!("Level {level}    EXP {exp}"),
        ui_anchor_x + 137.0,
        255.0,
        10.0,
        [62, 58, 49],
        Align::Center,
        170.0,
    );
    let reward = string_archive
        .root()
        .resolve(&format!("MonsterBook.img/{mob_id}/reward"))
        .map_err(|error| error.to_string())?
        .map(|node| node.children().unwrap_or_default())
        .unwrap_or_default();
    let mut drops = Vec::new();
    for drop in reward.into_iter().take(MAX_CARD_DROPS) {
        let Some(item_id) = integer_value(&drop)
            .and_then(|value| i32::try_from(value).ok())
            .filter(|item_id| *item_id > 0)
        else {
            continue;
        };
        let Some((icon_width, icon_height, icon_pixels)) =
            super::inventory_item_icon(item_id, item_archive, character_archive, icon_cache)
        else {
            continue;
        };
        drops.push((icon_width, icon_height, icon_pixels));
    }

    let drop_count = drops.len();
    let total_rows = drop_count.div_ceil(DROP_COLUMNS);
    let max_scroll = total_rows.saturating_sub(DROP_VISIBLE_ROWS);
    let first_row = drop_scroll.min(max_scroll);
    let grid_x = ui_anchor_x + 255.0;
    let grid_y = 165.0;
    for row in 0..DROP_VISIBLE_ROWS {
        for column in 0..DROP_COLUMNS {
            let index = (first_row + row) * DROP_COLUMNS + column;
            let Some((icon_width, icon_height, icon_pixels)) = drops.get(index) else {
                continue;
            };
            let cell_x = grid_x + column as f32 * DROP_CELL_SIZE;
            let cell_y = grid_y + row as f32 * DROP_CELL_SIZE;
            super::composite_inventory_icon(
                &mut canvas.pixels,
                canvas.width,
                canvas.height,
                icon_pixels,
                *icon_width as usize,
                *icon_height as usize,
                (cell_x + 7.0) as usize,
                (cell_y + 7.0) as usize,
                26,
            );
        }
    }
    Ok(drop_count)
}

fn draw_number(
    canvas: &mut Canvas,
    ui_archive: &NxArchive,
    book: &NxNode,
    font: &Font,
    folder: &str,
    x: f32,
    y: f32,
    value: usize,
    large: bool,
) -> Result<(), String> {
    let text = value.to_string();
    let digit_width = if large { 18.0 } else { 7.0 };
    let mut anchor_x = x;
    for digit in text.chars() {
        let path = format!("{folder}/{digit}");
        if let Some(node) = optional(book, &path)? {
            canvas.draw_node(ui_archive, &node, &path, anchor_x, y)?;
            let (width, _) = dimensions(ui_archive, &node, &path)?;
            anchor_x += width as f32;
        } else {
            canvas.draw_text(
                font,
                &digit.to_string(),
                anchor_x,
                y,
                if large { 20.0 } else { 9.0 },
                [255, 255, 255],
                Align::Left,
                digit_width,
            );
            anchor_x += digit_width;
        }
    }
    Ok(())
}

fn draw_button(
    canvas: &mut Canvas,
    archive: &NxArchive,
    book: &NxNode,
    path: &str,
    anchor_x: f32,
    anchor_y: f32,
    panel_x: f32,
    panel_y: f32,
) -> Result<Option<HitRect>, String> {
    let Some(node) = optional(book, path)? else {
        return Ok(None);
    };
    let (width, height) = dimensions(archive, &node, path)?;
    let origin = vector_origin(&node).unwrap_or((0, 0));
    canvas.draw_node(archive, &node, path, anchor_x, anchor_y)?;
    Ok(Some(HitRect {
        x: panel_x + anchor_x - origin.0 as f32,
        y: panel_y + anchor_y - origin.1 as f32,
        width: width as f32,
        height: height as f32,
    }))
}

fn count_categories(item_archive: &NxArchive) -> [usize; 9] {
    let mut counts = [0_usize; 9];
    let Ok(Some(consume)) = item_archive.root().resolve("Consume") else {
        return counts;
    };
    let Ok(images) = consume.children() else {
        return counts;
    };
    for image in images {
        let Ok(items) = image.children() else {
            continue;
        };
        for item in items {
            let Ok(name) = item.name() else {
                continue;
            };
            let Ok(item_id) = name.parse::<i32>() else {
                continue;
            };
            if (2_380_000..2_390_000).contains(&item_id) {
                counts[usize::from(category_for(item_id))] += 1;
            }
        }
    }
    counts
}

fn category_for(item_id: i32) -> u8 {
    if item_id / 1000 >= 2388 {
        8
    } else {
        ((item_id - 2_380_000) / 1000).clamp(0, 7) as u8
    }
}

fn card_item_path(item_id: i32) -> String {
    format!("Consume/0{}.img/0{item_id}", item_id / 10_000)
}

fn item_name(string_archive: &NxArchive, item_id: i32) -> Option<String> {
    string_archive
        .root()
        .resolve(&format!("Consume.img/{item_id}/name"))
        .ok()
        .flatten()
        .and_then(|node| string_value(&node))
}

fn required(root: &NxNode, path: &str) -> Result<NxNode, String> {
    optional(root, path)?.ok_or_else(|| format!("Monster Book NX node {path:?} was not found"))
}

fn optional(root: &NxNode, path: &str) -> Result<Option<NxNode>, String> {
    root.resolve(path).map_err(|error| error.to_string())
}

fn integer_value(node: &NxNode) -> Option<i64> {
    match node.value().ok()? {
        NodeValue::Integer(value) => Some(value),
        _ => None,
    }
}

fn string_value(node: &NxNode) -> Option<String> {
    match node.value().ok()? {
        NodeValue::String(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

fn vector_origin(node: &NxNode) -> Option<(i32, i32)> {
    match node
        .child("origin")
        .ok()?
        .and_then(|origin| origin.value().ok())?
    {
        NodeValue::Vector { x, y } => Some((x, y)),
        _ => None,
    }
}

fn dimensions(archive: &NxArchive, node: &NxNode, path: &str) -> Result<(u16, u16), String> {
    let bitmap = resolved_bitmap(archive, node, path)?;
    Ok((bitmap.width(), bitmap.height()))
}

fn bitmap_info(archive: &NxArchive, node: &NxNode, path: &str) -> Result<Bitmap, String> {
    resolved_bitmap(archive, node, path)
}

fn resolved_bitmap(archive: &NxArchive, node: &NxNode, path: &str) -> Result<Bitmap, String> {
    if let Some(bitmap) = node.bitmap().map_err(|error| error.to_string())? {
        return Ok(bitmap);
    }
    let source = node
        .child("source")
        .map_err(|error| error.to_string())?
        .and_then(|source| string_value(&source))
        .ok_or_else(|| format!("NX node {path:?} is not a bitmap and has no source"))?;
    let source = source
        .split_once('/')
        .map_or(source.as_str(), |(_, path)| path);
    let source_node = archive
        .root()
        .resolve(source)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("NX bitmap source {source:?} for {path:?} was not found"))?;
    source_node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("NX bitmap source {source:?} for {path:?} is not a bitmap"))
}

#[derive(Clone, Copy)]
enum Align {
    Left,
    Center,
}

struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl Canvas {
    fn new(width: u16, height: u16) -> Self {
        Self {
            width: usize::from(width),
            height: usize::from(height),
            pixels: vec![0; usize::from(width) * usize::from(height) * 4],
        }
    }

    fn fill_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [u8; 4]) {
        if !x.is_finite()
            || !y.is_finite()
            || !width.is_finite()
            || !height.is_finite()
            || width <= 0.0
            || height <= 0.0
        {
            return;
        }
        let left = (x.floor() as i32).clamp(0, self.width as i32);
        let top = (y.floor() as i32).clamp(0, self.height as i32);
        let right = ((x + width).ceil() as i32).clamp(0, self.width as i32);
        let bottom = ((y + height).ceil() as i32).clamp(0, self.height as i32);
        let source = [color[2], color[1], color[0], color[3]];
        for row in top..bottom {
            for column in left..right {
                let index = (row as usize * self.width + column as usize) * 4;
                blend_bgra(&mut self.pixels[index..index + 4], &source);
            }
        }
    }

    fn draw_node(
        &mut self,
        archive: &NxArchive,
        node: &NxNode,
        path: &str,
        anchor_x: f32,
        anchor_y: f32,
    ) -> Result<(), String> {
        let bitmap = resolved_bitmap(archive, node, path)?;
        let source = bitmap
            .decode_bgra_shared()
            .map_err(|error| error.to_string())?;
        let origin = vector_origin(node)
            .or_else(|| bitmap_origin(archive, node, path).ok())
            .unwrap_or((0, 0));
        let left = (anchor_x - origin.0 as f32).round() as i32;
        let top = (anchor_y - origin.1 as f32).round() as i32;
        let source_width = usize::from(bitmap.width());
        let source_height = usize::from(bitmap.height());
        for sy in 0..source_height {
            let dy = top + sy as i32;
            if dy < 0 || dy >= self.height as i32 {
                continue;
            }
            for sx in 0..source_width {
                let dx = left + sx as i32;
                if dx < 0 || dx >= self.width as i32 {
                    continue;
                }
                let src = (sy * source_width + sx) * 4;
                let dst = (dy as usize * self.width + dx as usize) * 4;
                blend_bgra(&mut self.pixels[dst..dst + 4], &source[src..src + 4]);
            }
        }
        Ok(())
    }

    fn draw_text(
        &mut self,
        font: &Font,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: [u8; 3],
        align: Align,
        max_width: f32,
    ) {
        let mut advance = 0.0;
        let mut previous = None;
        for character in text.chars() {
            if let Some(previous) = previous {
                advance += font
                    .horizontal_kern(previous, character, size)
                    .unwrap_or(0.0);
            }
            advance += rasterize_ui_glyph(font, character, size).0.advance_width;
            previous = Some(character);
        }
        let start_x = match align {
            Align::Left => x,
            Align::Center => x - advance.min(max_width) / 2.0,
        };
        let baseline = y.round() as i32 + size.ceil() as i32;
        let mut pen_x = start_x;
        previous = None;
        for character in text.chars() {
            if let Some(previous) = previous {
                pen_x += font
                    .horizontal_kern(previous, character, size)
                    .unwrap_or(0.0);
            }
            let (metrics, coverage) = rasterize_ui_glyph(font, character, size);
            let left = pen_x.round() as i32 + metrics.xmin;
            let top = baseline - metrics.ymin - metrics.height as i32;
            for row in 0..metrics.height {
                for column in 0..metrics.width {
                    let px = left + column as i32;
                    let py = top + row as i32;
                    if px < 0 || py < 0 || px >= self.width as i32 || py >= self.height as i32 {
                        continue;
                    }
                    let alpha = coverage[row * metrics.width + column];
                    if alpha == 0 {
                        continue;
                    }
                    let dest = (py as usize * self.width + px as usize) * 4;
                    let source = [color[2], color[1], color[0], alpha];
                    blend_bgra(&mut self.pixels[dest..dest + 4], &source);
                }
            }
            pen_x += metrics.advance_width;
            if pen_x - start_x > max_width {
                break;
            }
            previous = Some(character);
        }
    }
}

fn bitmap_origin(archive: &NxArchive, node: &NxNode, path: &str) -> Result<(i32, i32), String> {
    if let Some(origin) = vector_origin(node) {
        return Ok(origin);
    }
    let source = node
        .child("source")
        .map_err(|error| error.to_string())?
        .and_then(|source| string_value(&source));
    let Some(source) = source else {
        let _ = path;
        return Ok((0, 0));
    };
    let source = source
        .split_once('/')
        .map_or(source.as_str(), |(_, path)| path);
    let Some(source_node) = archive
        .root()
        .resolve(source)
        .map_err(|error| error.to_string())?
    else {
        return Ok((0, 0));
    };
    Ok(vector_origin(&source_node).unwrap_or((0, 0)))
}

fn blend_bgra(destination: &mut [u8], source: &[u8]) {
    let alpha = u32::from(source[3]);
    if alpha == 0 {
        return;
    }
    let destination_alpha = u32::from(destination[3]);
    let inverse = 255 - alpha;
    let output_alpha = alpha + (destination_alpha * inverse + 127) / 255;
    for channel in 0..3 {
        let premultiplied = u32::from(source[channel]) * alpha
            + (u32::from(destination[channel]) * destination_alpha * inverse + 127) / 255;
        destination[channel] = if output_alpha == 0 {
            0
        } else {
            ((premultiplied + output_alpha / 2) / output_alpha).min(255) as u8
        };
    }
    destination[3] = output_alpha.min(255) as u8;
}
