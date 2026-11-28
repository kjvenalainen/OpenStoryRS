use super::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use ui_foundation::{UiRect, UiTextRole, draw_text_role};

const WIDTH: usize = PET_ITEM_FILTER_PANEL_WIDTH as usize;
const HEIGHT: usize = PET_ITEM_FILTER_PANEL_HEIGHT as usize;
const SEARCH_FIELD: UiRect = UiRect::new(9.0, 29.0, (WIDTH - 18) as f32, 22.0);
const CONTENT: UiRect = UiRect::new(9.0, 51.0, (WIDTH - 18) as f32, (HEIGHT - 92) as f32);
const ROW_HEIGHT: f32 = 22.0;
const SEARCH_RESULT_ROW_HEIGHT: f32 = 20.0;
const SECTION_HEADER_HEIGHT: f32 = 16.0;
const SECTION_GAP: f32 = 2.0;
const MAX_SEARCH_RESULTS: usize = 5;
const SEARCH_DEBOUNCE_DELAY: Duration = Duration::from_millis(500);
const SCROLLBAR_WIDTH: f32 = 15.0;
const SCROLLBAR_ARROW_HEIGHT: f32 = 13.0;
const SCROLLBAR_THUMB_HEIGHT: f32 = 25.0;
const FOOTER_BUTTONS: [UiRect; 4] =
    ui_dialog::right_aligned_footer(WIDTH as f32 - 8.0, HEIGHT as f32, [42.0, 50.0, 68.0, 50.0]);
const MESO_BUTTON: UiRect = FOOTER_BUTTONS[0];
const LOW_LEVEL_BUTTON: UiRect = FOOTER_BUTTONS[1];
const REGISTER_BUTTON: UiRect = FOOTER_BUTTONS[2];
const DELETE_BUTTON: UiRect = FOOTER_BUTTONS[3];
pub(super) const CLOSE: UiRect = ui_dialog::inventory_close(WIDTH as f32);

#[derive(Clone, Debug)]
pub(super) struct State {
    pub(super) open: bool,
    pub(super) ignored_item_ids: Vec<i32>,
    pub(super) ignored_item_lookup: Arc<HashSet<i32>>,
    pub(super) ignore_meso: bool,
    pub(super) ignore_below_level_equipment: bool,
    pub(super) query: String,
    pub(super) scroll: usize,
    pub(super) selected_item_id: Option<i32>,
    pub(super) search_focused: bool,
    pub(super) key_text_for_ime_dedup: Option<String>,
    pub(super) notice: Option<String>,
    pub(super) search_due_at: Option<Instant>,
    pub(super) search_results: Vec<i32>,
    pub(super) search_result_count: usize,
    pub(super) item_labels: HashMap<i32, String>,
    pub(super) item_labels_lower: HashMap<i32, String>,
    pub(super) item_labels_loaded: bool,
    pub(super) display_order: Vec<i32>,
}

impl State {
    pub(super) fn load() -> Self {
        let saved = settings::load_pet_item_filter();
        let ignored_item_lookup = Arc::new(saved.ignored_item_ids.iter().copied().collect());
        Self {
            open: false,
            ignored_item_ids: saved.ignored_item_ids,
            ignored_item_lookup,
            ignore_meso: saved.ignore_meso,
            ignore_below_level_equipment: saved.ignore_below_level_equipment,
            query: String::new(),
            scroll: 0,
            selected_item_id: None,
            search_focused: false,
            key_text_for_ime_dedup: None,
            notice: None,
            search_due_at: None,
            search_results: Vec::new(),
            search_result_count: 0,
            item_labels: HashMap::new(),
            item_labels_lower: HashMap::new(),
            item_labels_loaded: false,
            display_order: Vec::new(),
        }
    }

    pub(super) fn loot_snapshot(&self, player_level: u8) -> PetLootFilterSnapshot {
        PetLootFilterSnapshot {
            ignored_item_ids: self.ignored_item_lookup.clone(),
            ignore_meso: self.ignore_meso,
            ignore_below_level_equipment: self.ignore_below_level_equipment,
            player_level,
        }
    }

    pub(super) fn schedule_search(&mut self) {
        self.search_results.clear();
        self.search_result_count = 0;
        self.search_due_at = if self.query.trim().is_empty() {
            None
        } else {
            Some(Instant::now() + SEARCH_DEBOUNCE_DELAY)
        };
    }

    pub(super) fn clear_query(&mut self) {
        self.query.clear();
        self.search_due_at = None;
        self.search_results.clear();
        self.search_result_count = 0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    Meso,
    LowLevel,
    Register,
    Delete,
}

pub(super) fn search_field() -> UiRect {
    SEARCH_FIELD
}

fn dropdown_open(state: &State) -> bool {
    !state.query.trim().is_empty() && state.search_due_at.is_none()
}

fn dropdown_row_count(state: &State) -> usize {
    state.search_results.len().max(1).min(MAX_SEARCH_RESULTS)
}

fn dropdown_bounds(state: &State) -> UiRect {
    let height =
        SECTION_HEADER_HEIGHT + dropdown_row_count(state) as f32 * SEARCH_RESULT_ROW_HEIGHT;
    UiRect::new(CONTENT.x, CONTENT.y, CONTENT.width, height)
}

fn ignored_header_y(state: &State) -> f32 {
    if dropdown_open(state) {
        dropdown_bounds(state).y + dropdown_bounds(state).height + SECTION_GAP
    } else {
        CONTENT.y
    }
}

pub(super) fn list_bounds(state: &State) -> UiRect {
    let header_y = ignored_header_y(state);
    let list_y = header_y + SECTION_HEADER_HEIGHT + SECTION_GAP;
    UiRect::new(
        CONTENT.x,
        list_y,
        CONTENT.width,
        (CONTENT.y + CONTENT.height - list_y).max(ROW_HEIGHT),
    )
}

pub(super) fn scrollbar_click_position(x: f32, y: f32, state: &State) -> Option<usize> {
    if !scrollbar_bounds(state).contains(x, y) {
        return None;
    }
    let maximum = max_scroll(state);
    let list = list_bounds(state);
    let top = list.y + SCROLLBAR_ARROW_HEIGHT;
    let bottom = list.y + list.height - SCROLLBAR_ARROW_HEIGHT;
    if y < top {
        return Some(state.scroll.saturating_sub(1));
    }
    if y >= bottom {
        return Some(state.scroll.saturating_add(1).min(maximum));
    }
    let fraction = ((y - top) / (bottom - top).max(1.0)).clamp(0.0, 1.0);
    Some((fraction * maximum as f32).round() as usize)
}

fn scrollbar_bounds(state: &State) -> UiRect {
    let list = list_bounds(state);
    UiRect::new(
        list.x + list.width - SCROLLBAR_WIDTH,
        list.y,
        SCROLLBAR_WIDTH,
        list.height,
    )
}

pub(super) fn hitboxes() -> [UiRect; 7] {
    [
        CLOSE,
        SEARCH_FIELD,
        CONTENT,
        MESO_BUTTON,
        LOW_LEVEL_BUTTON,
        REGISTER_BUTTON,
        DELETE_BUTTON,
    ]
}

pub(super) fn visible_item_ids(state: &State) -> Vec<i32> {
    let order = if state.display_order.len() == state.ignored_item_ids.len() {
        &state.display_order
    } else {
        &state.ignored_item_ids
    };
    order.to_vec()
}

pub(super) fn search_item_ids(
    query: &str,
    index: &[(String, i32)],
    ignored_item_ids: &HashSet<i32>,
) -> (Vec<i32>, usize) {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return (Vec::new(), 0);
    }
    let numeric_query = query.chars().all(|character| character.is_ascii_digit());
    let mut seen = HashSet::new();
    let mut matches = index
        .iter()
        .filter_map(|(name, item_id)| {
            if ignored_item_ids.contains(item_id) {
                return None;
            }
            let name_matches = name.contains(&query);
            let id_matches = numeric_query && item_id.to_string().starts_with(&query);
            if (!name_matches && !id_matches) || !seen.insert(*item_id) {
                return None;
            }
            let rank = if name == &query {
                0
            } else if name.starts_with(&query) {
                1
            } else if id_matches {
                2
            } else {
                3
            };
            Some((rank, name.chars().count(), name.clone(), *item_id))
        })
        .collect::<Vec<_>>();
    matches.sort();
    let total = matches.len();
    let results = matches
        .into_iter()
        .take(MAX_SEARCH_RESULTS)
        .map(|(_, _, _, item_id)| item_id)
        .collect();
    (results, total)
}

pub(super) fn search_result_at(x: f32, y: f32, state: &State) -> Option<i32> {
    if !dropdown_open(state) {
        return None;
    }
    let bounds = dropdown_bounds(state);
    let rows = UiRect::new(
        bounds.x + 1.0,
        bounds.y + SECTION_HEADER_HEIGHT,
        bounds.width - 2.0,
        bounds.height - SECTION_HEADER_HEIGHT,
    );
    if !rows.contains(x, y) {
        return None;
    }
    let row = ((y - rows.y) / SEARCH_RESULT_ROW_HEIGHT).floor() as usize;
    state.search_results.get(row).copied()
}

pub(super) fn search_dropdown_contains(x: f32, y: f32, state: &State) -> bool {
    dropdown_open(state) && dropdown_bounds(state).contains(x, y)
}

pub(super) fn wildcard_item_ids(pattern: &str, index: &[(String, i32)]) -> Vec<i32> {
    let pattern = pattern.trim().to_lowercase().chars().collect::<Vec<_>>();
    let mut seen = HashSet::new();
    index
        .iter()
        .filter_map(|(name, item_id)| {
            (wildcard_match_chars(&pattern, &name.chars().collect::<Vec<_>>())
                && seen.insert(*item_id))
            .then_some(*item_id)
        })
        .collect()
}

fn wildcard_match_chars(pattern: &[char], value: &[char]) -> bool {
    let (mut pattern_index, mut value_index) = (0, 0);
    let (mut last_star, mut star_value_index) = (None, 0);

    while value_index < value.len() {
        if pattern.get(pattern_index) == Some(&value[value_index]) {
            pattern_index += 1;
            value_index += 1;
        } else if pattern.get(pattern_index) == Some(&'*') {
            last_star = Some(pattern_index);
            pattern_index += 1;
            star_value_index = value_index;
        } else if let Some(star_index) = last_star {
            star_value_index += 1;
            value_index = star_value_index;
            pattern_index = star_index + 1;
        } else {
            return false;
        }
    }

    while pattern.get(pattern_index) == Some(&'*') {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

pub(super) fn sort_item_order(state: &mut State) {
    let mut order = state.ignored_item_ids.clone();
    order.sort_by(|left, right| {
        let left_name = state
            .item_labels_lower
            .get(left)
            .map_or_else(|| format!("item #{left}"), Clone::clone);
        let right_name = state
            .item_labels_lower
            .get(right)
            .map_or_else(|| format!("item #{right}"), Clone::clone);
        left_name.cmp(&right_name).then_with(|| left.cmp(right))
    });
    state.display_order = order;
}

pub(super) fn visible_rows(state: &State) -> usize {
    (list_bounds(state).height / ROW_HEIGHT).floor() as usize
}

pub(super) fn action_at(x: f32, y: f32) -> Option<Action> {
    [
        (Action::Meso, MESO_BUTTON),
        (Action::LowLevel, LOW_LEVEL_BUTTON),
        (Action::Register, REGISTER_BUTTON),
        (Action::Delete, DELETE_BUTTON),
    ]
    .into_iter()
    .find_map(|(action, bounds)| bounds.contains(x, y).then_some(action))
}

pub(super) fn row_at(x: f32, y: f32, state: &State) -> Option<i32> {
    let list = list_bounds(state);
    let content = UiRect::new(list.x, list.y, list.width - SCROLLBAR_WIDTH, list.height);
    if !content.contains(x, y) {
        return None;
    }
    let row = ((y - list.y) / ROW_HEIGHT).floor() as usize;
    visible_item_ids(state)
        .get(state.scroll.saturating_add(row))
        .copied()
}

pub(super) fn max_scroll(state: &State) -> usize {
    visible_item_ids(state)
        .len()
        .saturating_sub(visible_rows(state))
}

pub(super) fn closest_item(query: &str, index: &[(String, i32)]) -> Option<(i32, String)> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return None;
    }
    if let Some((name, item_id)) = index.iter().find(|(name, _)| name == &needle) {
        return Some((*item_id, name.clone()));
    }

    let substring = index
        .iter()
        .filter(|(name, _)| name.contains(&needle))
        .min_by(|left, right| {
            left.0
                .chars()
                .count()
                .cmp(&right.0.chars().count())
                .then_with(|| left.0.cmp(&right.0))
                .then_with(|| left.1.cmp(&right.1))
        });
    if let Some((name, item_id)) = substring {
        return Some((*item_id, name.clone()));
    }

    let max_distance = needle.chars().count().div_ceil(3).max(2);
    index
        .iter()
        .map(|(name, item_id)| {
            (
                edit_distance_with_limit(&needle, name, max_distance),
                name,
                item_id,
            )
        })
        .filter(|(distance, _, _)| *distance <= max_distance)
        .min_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.chars().count().cmp(&right.1.chars().count()))
                .then_with(|| left.1.cmp(right.1))
                .then_with(|| left.2.cmp(right.2))
        })
        .map(|(_, name, item_id)| (*item_id, name.clone()))
}

fn edit_distance_with_limit(left: &str, right: &str, limit: usize) -> usize {
    let left = left.chars().collect::<Vec<_>>();
    let right = right.chars().collect::<Vec<_>>();
    if left.len().abs_diff(right.len()) > limit {
        return limit.saturating_add(1);
    }
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];
    for (left_index, left_char) in left.iter().enumerate() {
        current[0] = left_index + 1;
        let mut row_min = current[0];
        for (right_index, right_char) in right.iter().enumerate() {
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + usize::from(left_char != right_char));
            row_min = row_min.min(current[right_index + 1]);
        }
        if row_min > limit {
            return limit.saturating_add(1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

pub(super) fn render(
    font: &fontdue::Font,
    ui: &NxArchive,
    items: &NxArchive,
    characters: &NxArchive,
    strings: &NxArchive,
    state: &State,
    name_cache: &mut HashMap<i32, Option<String>>,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
) -> Vec<u8> {
    let mut pixels = vec![0; WIDTH * HEIGHT * 4];
    let mut dialog = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui);
    let _ = dialog.inventory_window(
        UiRect::new(0.0, 0.0, WIDTH as f32, HEIGHT as f32),
        font,
        "Exception List",
        CLOSE,
    );

    rect_outline(
        &mut pixels,
        WIDTH,
        HEIGHT,
        SEARCH_FIELD,
        if state.search_focused {
            [178, 137, 85, 255]
        } else {
            [204, 195, 181, 255]
        },
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        SEARCH_FIELD.x as usize + 1,
        SEARCH_FIELD.y as usize + 1,
        SEARCH_FIELD.width as usize - 2,
        SEARCH_FIELD.height as usize - 2,
        if state.search_focused {
            [255, 255, 255, 255]
        } else {
            [237, 237, 232, 255]
        },
    );
    let search_text = if state.query.is_empty() {
        "Type an item name to search".to_owned()
    } else {
        state.query.clone()
    };
    let search_text = ui_foundation::ellipsize(font, &search_text, 10.0, SEARCH_FIELD.width - 14.0);
    super::draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &search_text,
        SEARCH_FIELD.x as i32 + 6,
        SEARCH_FIELD.y as i32 + 4,
        10.0,
        SEARCH_FIELD.width as usize - 14,
        SEARCH_FIELD.height as usize - 7,
        if state.query.is_empty() {
            [120, 112, 98]
        } else {
            [45, 38, 29]
        },
    );

    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        CONTENT.x as usize,
        CONTENT.y as usize,
        CONTENT.width as usize,
        CONTENT.height as usize,
        [224, 231, 238, 255],
    );
    rect_outline(&mut pixels, WIDTH, HEIGHT, CONTENT, [153, 164, 175, 255]);

    if dropdown_open(state) {
        draw_search_dropdown(
            font,
            items,
            characters,
            strings,
            state,
            &mut pixels,
            name_cache,
            icon_cache,
        );
    }

    let header_y = ignored_header_y(state);
    let header = UiRect::new(CONTENT.x, header_y, CONTENT.width, SECTION_HEADER_HEIGHT);
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        header.x as usize,
        header.y as usize,
        header.width as usize,
        header.height as usize,
        [214, 222, 229, 255],
    );
    let header_label = if state.search_due_at.is_some() && !state.query.trim().is_empty() {
        "Ignored items  ·  Searching…"
    } else {
        "Ignored items"
    };
    draw_text_role(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        header_label,
        header.x as i32 + 4,
        header.y as i32 + 2,
        UiTextRole::Small,
        header.width as usize - 8,
        12,
        Some([65, 77, 89]),
    );

    let list = list_bounds(state);
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        list.x as usize,
        list.y as usize,
        list.width as usize,
        list.height as usize,
        [224, 231, 238, 255],
    );
    rect_outline(&mut pixels, WIDTH, HEIGHT, list, [153, 164, 175, 255]);
    let visible = visible_item_ids(state);
    for row in 0..visible_rows(state) {
        let row_bounds = UiRect::new(
            list.x + 1.0,
            list.y + 1.0 + row as f32 * ROW_HEIGHT,
            list.width - SCROLLBAR_WIDTH - 2.0,
            ROW_HEIGHT - 1.0,
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            row_bounds.x as usize,
            row_bounds.y as usize,
            row_bounds.width as usize,
            row_bounds.height as usize,
            if row % 2 == 0 {
                [236, 241, 246, 255]
            } else {
                [226, 233, 240, 255]
            },
        );
        let Some(item_id) = visible.get(state.scroll.saturating_add(row)).copied() else {
            continue;
        };
        let selected = state.selected_item_id == Some(item_id);
        if selected {
            let _ = composite_nx_ui_bitmap_scaled_to(
                &mut pixels,
                WIDTH,
                HEIGHT,
                ui,
                "UIWindow.img/Shop/select",
                row_bounds.x as usize,
                row_bounds.y as usize,
                row_bounds.width as usize,
                row_bounds.height as usize,
            );
        }
        let label = state
            .item_labels
            .get(&item_id)
            .cloned()
            .unwrap_or_else(|| super::item_name(item_id, strings, name_cache));
        if let Some((icon_width, icon_height, icon)) =
            super::inventory_item_icon(item_id, items, characters, icon_cache)
        {
            draw_scaled_item_icon(
                &mut pixels,
                &icon,
                icon_width as usize,
                icon_height as usize,
                row_bounds.x as usize + 3,
                row_bounds.y as usize + 2,
                18,
            );
        }
        let label = ui_foundation::ellipsize(font, &label, 10.0, row_bounds.width - 34.0);
        super::draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &label,
            row_bounds.x as i32 + 25,
            row_bounds.y as i32 + 4,
            10.0,
            row_bounds.width as usize - 32,
            row_bounds.height as usize - 5,
            [45, 38, 29],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            row_bounds.x as usize,
            row_bounds.y as usize + row_bounds.height as usize - 1,
            row_bounds.width as usize,
            1,
            [193, 202, 211, 255],
        );
    }

    if visible.is_empty() {
        super::draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "No ignored items",
            list.x as i32 + 8,
            list.y as i32 + 10,
            10.0,
            list.width as usize - SCROLLBAR_WIDTH as usize - 16,
            16,
            [105, 96, 82],
        );
    }
    if let Some(notice) = state.notice.as_deref() {
        draw_text_role(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            notice,
            CONTENT.x as i32 + 2,
            (CONTENT.y + CONTENT.height + 1.0) as i32,
            UiTextRole::Small,
            CONTENT.width as usize - 4,
            12,
            Some([114, 81, 53]),
        );
    }
    draw_scrollbar(&mut pixels, ui, &visible, state.scroll, list);

    let mut dialog = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui);
    draw_button(&mut dialog, font, MESO_BUTTON, "MESO", "Basic.img/BtYes2");
    draw_button(
        &mut dialog,
        font,
        LOW_LEVEL_BUTTON,
        "LOW LVL",
        "Basic.img/BtYes2",
    );
    draw_button(
        &mut dialog,
        font,
        REGISTER_BUTTON,
        "REGISTER",
        "Basic.img/BtYes2",
    );
    draw_button(
        &mut dialog,
        font,
        DELETE_BUTTON,
        "DELETE",
        "Basic.img/BtNo2",
    );
    drop(dialog);
    tint_button_background(
        &mut pixels,
        MESO_BUTTON,
        if state.ignore_meso {
            ([249, 157, 145], [190, 56, 48])
        } else {
            ([155, 214, 132], [52, 133, 74])
        },
    );
    tint_button_background(
        &mut pixels,
        LOW_LEVEL_BUTTON,
        if state.ignore_below_level_equipment {
            ([249, 157, 145], [190, 56, 48])
        } else {
            ([155, 214, 132], [52, 133, 74])
        },
    );
    tint_button_background(
        &mut pixels,
        REGISTER_BUTTON,
        ([155, 214, 132], [52, 133, 74]),
    );
    tint_button_background(&mut pixels, DELETE_BUTTON, ([247, 174, 126], [196, 79, 56]));
    pixels
}

#[allow(clippy::too_many_arguments)]
fn draw_search_dropdown(
    font: &fontdue::Font,
    items: &NxArchive,
    characters: &NxArchive,
    strings: &NxArchive,
    state: &State,
    pixels: &mut [u8],
    name_cache: &mut HashMap<i32, Option<String>>,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
) {
    let bounds = dropdown_bounds(state);
    fill_dialogue_rect(
        pixels,
        WIDTH,
        HEIGHT,
        bounds.x as usize,
        bounds.y as usize,
        bounds.width as usize,
        bounds.height as usize,
        [255, 249, 230, 255],
    );
    rect_outline(pixels, WIDTH, HEIGHT, bounds, [180, 145, 83, 255]);
    let header = UiRect::new(
        bounds.x + 1.0,
        bounds.y + 1.0,
        bounds.width - 2.0,
        SECTION_HEADER_HEIGHT - 1.0,
    );
    fill_dialogue_rect(
        pixels,
        WIDTH,
        HEIGHT,
        header.x as usize,
        header.y as usize,
        header.width as usize,
        header.height as usize,
        [242, 225, 185, 255],
    );
    let header_label = format!(
        "Search results · click to add ({})",
        state.search_result_count
    );
    draw_text_role(
        font,
        pixels,
        WIDTH,
        HEIGHT,
        &header_label,
        header.x as i32 + 4,
        header.y as i32 + 2,
        UiTextRole::Small,
        header.width as usize - 8,
        12,
        Some([100, 74, 33]),
    );

    if state.search_results.is_empty() {
        super::draw_dialogue_text(
            font,
            pixels,
            WIDTH,
            HEIGHT,
            "No new matches",
            bounds.x as i32 + 8,
            (bounds.y + SECTION_HEADER_HEIGHT + 3.0) as i32,
            9.0,
            bounds.width as usize - 16,
            14,
            [113, 98, 70],
        );
        return;
    }

    for (row, item_id) in state.search_results.iter().copied().enumerate() {
        let row_bounds = UiRect::new(
            bounds.x + 1.0,
            bounds.y + SECTION_HEADER_HEIGHT + row as f32 * SEARCH_RESULT_ROW_HEIGHT,
            bounds.width - 2.0,
            SEARCH_RESULT_ROW_HEIGHT,
        );
        fill_dialogue_rect(
            pixels,
            WIDTH,
            HEIGHT,
            row_bounds.x as usize,
            row_bounds.y as usize,
            row_bounds.width as usize,
            row_bounds.height as usize,
            if row % 2 == 0 {
                [255, 253, 246, 255]
            } else {
                [250, 244, 228, 255]
            },
        );
        if let Some((icon_width, icon_height, icon)) =
            super::inventory_item_icon(item_id, items, characters, icon_cache)
        {
            draw_scaled_item_icon(
                pixels,
                &icon,
                icon_width as usize,
                icon_height as usize,
                row_bounds.x as usize + 2,
                row_bounds.y as usize + 1,
                18,
            );
        }
        let label = state
            .item_labels
            .get(&item_id)
            .cloned()
            .unwrap_or_else(|| super::item_name(item_id, strings, name_cache));
        let label = ui_foundation::ellipsize(font, &label, 9.5, row_bounds.width - 47.0);
        super::draw_dialogue_text(
            font,
            pixels,
            WIDTH,
            HEIGHT,
            &label,
            row_bounds.x as i32 + 24,
            row_bounds.y as i32 + 3,
            9.5,
            row_bounds.width as usize - 50,
            15,
            [54, 43, 25],
        );
        super::draw_dialogue_text(
            font,
            pixels,
            WIDTH,
            HEIGHT,
            "+ Add",
            (row_bounds.x + row_bounds.width - 34.0) as i32,
            row_bounds.y as i32 + 3,
            8.5,
            31,
            15,
            [135, 93, 32],
        );
        fill_dialogue_rect(
            pixels,
            WIDTH,
            HEIGHT,
            row_bounds.x as usize,
            row_bounds.y as usize + row_bounds.height as usize - 1,
            row_bounds.width as usize,
            1,
            [235, 221, 190, 255],
        );
    }
}

fn draw_scaled_item_icon(
    canvas: &mut [u8],
    source: &[u8],
    source_width: usize,
    source_height: usize,
    x: usize,
    y: usize,
    max_size: usize,
) {
    if source_width == 0 || source_height == 0 || max_size == 0 {
        return;
    }
    let scale = (max_size as f32 / source_width as f32)
        .min(max_size as f32 / source_height as f32)
        .min(1.0);
    let width = (source_width as f32 * scale).round().max(1.0) as usize;
    let height = (source_height as f32 * scale).round().max(1.0) as usize;
    let left = x + (max_size - width) / 2;
    let top = y + (max_size - height) / 2;
    for target_y in 0..height {
        let source_y = target_y * source_height / height;
        for target_x in 0..width {
            let source_x = target_x * source_width / width;
            let source_index = (source_y * source_width + source_x) * 4;
            let destination_index = ((top + target_y) * WIDTH + left + target_x) * 4;
            let Some(source_pixel) = source.get(source_index..source_index + 4) else {
                continue;
            };
            let Some(destination_pixel) = canvas.get_mut(destination_index..destination_index + 4)
            else {
                continue;
            };
            let alpha = u16::from(source_pixel[3]);
            let inverse = 255 - alpha;
            for channel in 0..3 {
                destination_pixel[channel] = ((u16::from(source_pixel[channel]) * alpha
                    + u16::from(destination_pixel[channel]) * inverse)
                    / 255) as u8;
            }
            let destination_alpha = u16::from(destination_pixel[3]);
            destination_pixel[3] = (alpha + destination_alpha * inverse / 255) as u8;
        }
    }
}

fn draw_scrollbar(pixels: &mut [u8], ui: &NxArchive, visible: &[i32], scroll: usize, list: UiRect) {
    let bounds = scrollbar_bounds_for(list);
    let maximum = visible
        .len()
        .saturating_sub((list.height / ROW_HEIGHT).floor() as usize);
    let mode = if maximum > 0 { "enabled" } else { "disabled" };
    let track_top = bounds.y + SCROLLBAR_ARROW_HEIGHT;
    let track_bottom = bounds.y + bounds.height - SCROLLBAR_ARROW_HEIGHT;
    let track_x = bounds.x as usize;
    let track_top = track_top as usize;
    let track_bottom = track_bottom as usize;
    for y in (track_top..track_bottom).step_by(SCROLLBAR_ARROW_HEIGHT as usize) {
        let height = (track_bottom - y).min(SCROLLBAR_ARROW_HEIGHT as usize);
        let _ = composite_nx_ui_bitmap_scaled_to(
            pixels,
            WIDTH,
            HEIGHT,
            ui,
            &format!("Basic.img/VScr4/{mode}/base"),
            track_x,
            y,
            SCROLLBAR_WIDTH as usize,
            height,
        );
    }
    let _ = composite_nx_ui_bitmap(
        pixels,
        WIDTH,
        HEIGHT,
        ui,
        &format!(
            "Basic.img/VScr4/{mode}/{}",
            if maximum > 0 { "prev0" } else { "prev" }
        ),
        bounds.x as i32,
        bounds.y as i32,
    );
    let _ = composite_nx_ui_bitmap(
        pixels,
        WIDTH,
        HEIGHT,
        ui,
        &format!(
            "Basic.img/VScr4/{mode}/{}",
            if maximum > 0 { "next0" } else { "next" }
        ),
        bounds.x as i32,
        (bounds.y + bounds.height - SCROLLBAR_ARROW_HEIGHT) as i32,
    );
    if maximum > 0 {
        let travel = track_bottom - track_top - SCROLLBAR_THUMB_HEIGHT as usize;
        let thumb_y = track_top + travel * scroll.min(maximum) / maximum;
        let _ = composite_nx_ui_bitmap(
            pixels,
            WIDTH,
            HEIGHT,
            ui,
            "Basic.img/VScr4/enabled/thumb0",
            bounds.x as i32,
            thumb_y as i32,
        );
    }
}

fn scrollbar_bounds_for(list: UiRect) -> UiRect {
    UiRect::new(
        list.x + list.width - SCROLLBAR_WIDTH,
        list.y,
        SCROLLBAR_WIDTH,
        list.height,
    )
}

fn draw_button(
    dialog: &mut NxDialog<'_>,
    font: &fontdue::Font,
    bounds: UiRect,
    label: &str,
    art: &'static str,
) {
    let _ = dialog.labeled_button(
        bounds,
        ui_dialog::NxButton::new(art, ui_dialog::ButtonState::Normal),
        font,
        label,
    );
}

fn tint_button_background(pixels: &mut [u8], bounds: UiRect, tint: ([u8; 3], [u8; 3])) {
    let (light, dark) = tint;
    for y in bounds.y as usize..(bounds.y + bounds.height) as usize {
        for x in bounds.x as usize..(bounds.x + bounds.width) as usize {
            let offset = (y * WIDTH + x) * 4;
            let pixel = &mut pixels[offset..offset + 4];
            let [red, green, blue] = [pixel[2], pixel[1], pixel[0]];
            let brightest = red.max(green).max(blue);
            let darkest = red.min(green).min(blue);
            if brightest <= 90 || brightest.saturating_sub(darkest) <= 24 {
                continue;
            }
            let luminance = red as f32 * 0.299 + green as f32 * 0.587 + blue as f32 * 0.114;
            let shade = ((luminance - 78.0) / 110.0).clamp(0.0, 1.0);
            for channel in 0..3 {
                pixel[2 - channel] = (dark[channel] as f32
                    + (light[channel] as f32 - dark[channel] as f32) * shade)
                    .round() as u8;
            }
        }
    }
}

fn rect_outline(pixels: &mut [u8], width: usize, height: usize, rect: UiRect, color: [u8; 4]) {
    let left = rect.x as usize;
    let top = rect.y as usize;
    let right = left.saturating_add(rect.width as usize).saturating_sub(1);
    let bottom = top.saturating_add(rect.height as usize).saturating_sub(1);
    for x in left..=right.min(width.saturating_sub(1)) {
        set_pixel(pixels, width, height, x, top, color);
        set_pixel(pixels, width, height, x, bottom, color);
    }
    for y in top..=bottom.min(height.saturating_sub(1)) {
        set_pixel(pixels, width, height, left, y, color);
        set_pixel(pixels, width, height, right, y, color);
    }
}

fn set_pixel(pixels: &mut [u8], width: usize, height: usize, x: usize, y: usize, color: [u8; 4]) {
    if x >= width || y >= height {
        return;
    }
    let offset = (y * width + x) * 4;
    pixels[offset..offset + 4].copy_from_slice(&color);
}
