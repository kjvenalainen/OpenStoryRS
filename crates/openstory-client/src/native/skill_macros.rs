//! Native editor for the three v83 skill macros.

use super::{draw_dialogue_text, fill_dialogue_rect};
use openstory_protocol::{MAX_SKILL_MACRO_NAME_CHARS, SKILL_MACRO_COUNT, SkillMacro};

pub(super) const PANEL_X: f32 = 232.0;
pub(super) const PANEL_Y: f32 = 154.0;
pub(super) const PANEL_WIDTH: u32 = 560;
pub(super) const PANEL_HEIGHT: u32 = 450;

pub(super) const VISIBLE_SKILLS: usize = 5;
const SKILL_ROW_TOP: f32 = 198.0;
const SKILL_ROW_HEIGHT: f32 = 29.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    Close,
    Save,
    SelectMacro(usize),
    FocusName,
    ToggleShout,
    SelectSlot(usize),
    ClearSkill(usize),
    AssignSkill(i32),
}

#[derive(Clone, Debug, Default)]
pub(super) struct State {
    pub open: bool,
    pub name_focused: bool,
    pub selected_macro: usize,
    pub selected_slot: usize,
    pub skill_scroll: usize,
    pub key_text_for_ime_dedup: Option<String>,
    drafts: Vec<SkillMacro>,
}

impl State {
    pub fn open(&mut self, server_macros: &[SkillMacro]) {
        self.drafts = server_macros
            .iter()
            .take(SKILL_MACRO_COUNT)
            .cloned()
            .collect();
        self.drafts
            .resize_with(SKILL_MACRO_COUNT, SkillMacro::default);
        self.open = true;
        self.name_focused = true;
        self.selected_macro = 0;
        self.selected_slot = 0;
        self.skill_scroll = 0;
        self.key_text_for_ime_dedup = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.name_focused = false;
        self.key_text_for_ime_dedup = None;
    }

    pub fn drafts(&self) -> &[SkillMacro] {
        &self.drafts
    }

    pub fn select_macro(&mut self, index: usize) {
        if index < self.drafts.len() {
            self.selected_macro = index;
        }
    }

    pub fn select_slot(&mut self, index: usize) {
        if index < 3 {
            self.selected_slot = index;
            self.name_focused = false;
        }
    }

    pub fn toggle_shout(&mut self) {
        if let Some(macro_entry) = self.drafts.get_mut(self.selected_macro) {
            macro_entry.shout = !macro_entry.shout;
        }
        self.name_focused = false;
    }

    pub fn clear_skill(&mut self, slot: usize) {
        if let Some(skill_id) = self
            .drafts
            .get_mut(self.selected_macro)
            .and_then(|macro_entry| macro_entry.skills.get_mut(slot))
        {
            *skill_id = 0;
        }
        self.name_focused = false;
    }

    pub fn assign_skill(&mut self, skill_id: i32) {
        if skill_id <= 0 {
            return;
        }
        if let Some(skill_id_slot) = self
            .drafts
            .get_mut(self.selected_macro)
            .and_then(|macro_entry| macro_entry.skills.get_mut(self.selected_slot))
        {
            *skill_id_slot = skill_id;
        }
        self.name_focused = false;
    }

    pub fn insert_text(&mut self, text: &str) {
        if !self.name_focused {
            return;
        }
        let Some(name) = self
            .drafts
            .get_mut(self.selected_macro)
            .map(|macro_entry| &mut macro_entry.name)
        else {
            return;
        };
        for character in text.chars() {
            if !character.is_control() && name.chars().count() < MAX_SKILL_MACRO_NAME_CHARS {
                name.push(character);
            }
        }
    }

    pub fn backspace(&mut self) {
        if self.name_focused
            && let Some(name) = self
                .drafts
                .get_mut(self.selected_macro)
                .map(|macro_entry| &mut macro_entry.name)
        {
            name.pop();
        }
    }

    pub fn scroll_skills(&mut self, direction: isize, skill_count: usize) {
        self.skill_scroll = self
            .skill_scroll
            .saturating_add_signed(direction)
            .min(skill_count.saturating_sub(VISIBLE_SKILLS));
    }
}

pub(super) fn hit_test(
    local_x: f32,
    local_y: f32,
    state: &State,
    skills: &[(i32, String)],
) -> Option<Action> {
    if local_y < 42.0 && local_x >= PANEL_WIDTH as f32 - 44.0 {
        return Some(Action::Close);
    }
    if (48.0..80.0).contains(&local_y) && (18.0..542.0).contains(&local_x) {
        let index = ((local_x - 18.0) / 174.0).floor() as usize;
        return (index < SKILL_MACRO_COUNT).then_some(Action::SelectMacro(index));
    }
    if (88.0..120.0).contains(&local_y) && (56.0..432.0).contains(&local_x) {
        return Some(Action::FocusName);
    }
    if (124.0..153.0).contains(&local_y) && (16.0..190.0).contains(&local_x) {
        return Some(Action::ToggleShout);
    }
    if (190.0..322.0).contains(&local_y) && (16.0..300.0).contains(&local_x) {
        let slot = ((local_y - 190.0) / 44.0).floor() as usize;
        if slot < 3 {
            return Some(if local_x >= 250.0 {
                Action::ClearSkill(slot)
            } else {
                Action::SelectSlot(slot)
            });
        }
    }
    if (SKILL_ROW_TOP..SKILL_ROW_TOP + VISIBLE_SKILLS as f32 * SKILL_ROW_HEIGHT).contains(&local_y)
        && (316.0..546.0).contains(&local_x)
    {
        let row = ((local_y - SKILL_ROW_TOP) / SKILL_ROW_HEIGHT).floor() as usize;
        return skills
            .get(state.skill_scroll.saturating_add(row))
            .map(|(id, _)| Action::AssignSkill(*id));
    }
    if (410.0..440.0).contains(&local_y) {
        if (396.0..524.0).contains(&local_x) {
            return Some(Action::Save);
        }
        if (256.0..384.0).contains(&local_x) {
            return Some(Action::Close);
        }
    }
    None
}

pub(super) fn skill_list_contains(local_x: f32, local_y: f32) -> bool {
    (316.0..546.0).contains(&local_x)
        && (SKILL_ROW_TOP..SKILL_ROW_TOP + VISIBLE_SKILLS as f32 * SKILL_ROW_HEIGHT)
            .contains(&local_y)
}

pub(super) fn render(font: &fontdue::Font, state: &State, skills: &[(i32, String)]) -> Vec<u8> {
    let width = PANEL_WIDTH as usize;
    let height = PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        height,
        [27, 25, 23, 250],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        42,
        [91, 69, 42, 255],
    );
    for (x, y, rect_width, rect_height) in [
        (0, 0, width, 2),
        (0, height - 2, width, 2),
        (0, 0, 2, height),
        (width - 2, 0, 2, height),
    ] {
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            x,
            y,
            rect_width,
            rect_height,
            [192, 164, 112, 255],
        );
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Skill Macros",
        16,
        9,
        20.0,
        460,
        26,
        [250, 235, 202],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "X",
        width as i32 - 28,
        9,
        18.0,
        18,
        24,
        [255, 224, 178],
    );

    for index in 0..SKILL_MACRO_COUNT {
        let x = 18 + index * 174;
        let selected = index == state.selected_macro;
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            x,
            48,
            166,
            31,
            if selected {
                [92, 75, 48, 255]
            } else {
                [48, 48, 52, 255]
            },
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("Macro {}", index + 1),
            x as i32 + 9,
            55,
            13.0,
            146,
            19,
            if selected {
                [255, 231, 173]
            } else {
                [200, 200, 200]
            },
        );
    }

    let current = state.drafts.get(state.selected_macro);
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Name",
        20,
        96,
        13.0,
        48,
        18,
        [225, 218, 201],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        68,
        88,
        352,
        30,
        if state.name_focused {
            [244, 238, 221, 255]
        } else {
            [70, 67, 61, 255]
        },
    );
    let name = current.map_or("", |macro_entry| macro_entry.name.as_str());
    let displayed_name = if state.name_focused {
        format!("{name}|")
    } else {
        name.to_owned()
    };
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        &displayed_name,
        76,
        94,
        14.0,
        336,
        21,
        [25, 25, 25],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        20,
        130,
        18,
        18,
        if current.is_some_and(|macro_entry| macro_entry.shout) {
            [116, 151, 76, 255]
        } else {
            [66, 62, 55, 255]
        },
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        if current.is_some_and(|macro_entry| macro_entry.shout) {
            "✓"
        } else {
            " "
        },
        23,
        129,
        15.0,
        14,
        18,
        [255, 255, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Shout macro name in public chat",
        46,
        132,
        12.0,
        270,
        16,
        [220, 214, 199],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        16,
        158,
        width - 32,
        1,
        [101, 88, 66, 255],
    );

    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Macro skill order",
        20,
        168,
        13.0,
        270,
        18,
        [240, 224, 191],
    );
    for slot in 0..3 {
        let y = 190 + slot * 44;
        let selected = slot == state.selected_slot;
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            18,
            y,
            278,
            37,
            if selected {
                [75, 62, 43, 255]
            } else {
                [42, 39, 35, 255]
            },
        );
        let skill_id = current.map_or(0, |macro_entry| macro_entry.skills[slot]);
        let skill_name = skills.iter().find(|(id, _)| *id == skill_id).map_or_else(
            || {
                if skill_id == 0 {
                    "Empty slot".to_owned()
                } else {
                    format!("Skill #{skill_id}")
                }
            },
            |(_, name)| name.clone(),
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("{}. {}", slot + 1, skill_name),
            28,
            y as i32 + 5,
            12.0,
            210,
            17,
            if skill_id > 0 {
                [238, 221, 181]
            } else {
                [175, 175, 175]
            },
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            246,
            y + 6,
            42,
            24,
            [76, 55, 45, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Clear",
            251,
            y as i32 + 10,
            10.0,
            34,
            14,
            [230, 204, 181],
        );
    }

    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        &format!("Learned active skills ({})", skills.len()),
        320,
        168,
        12.0,
        220,
        18,
        [240, 224, 191],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        314,
        188,
        230,
        178,
        [34, 32, 29, 255],
    );
    for (row, (skill_id, skill_name)) in skills
        .iter()
        .skip(state.skill_scroll)
        .take(VISIBLE_SKILLS)
        .enumerate()
    {
        let y = SKILL_ROW_TOP as usize + row * SKILL_ROW_HEIGHT as usize;
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            318,
            y,
            222,
            27,
            if row % 2 == 0 {
                [49, 46, 41, 255]
            } else {
                [42, 40, 36, 255]
            },
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            skill_name,
            325,
            y as i32 + 6,
            11.0,
            208,
            16,
            [225, 218, 201],
        );
        let _ = skill_id;
    }
    if skills.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "No learned active skills.",
            324,
            201,
            11.0,
            210,
            32,
            [175, 175, 175],
        );
    }
    let visible_start = if skills.is_empty() {
        0
    } else {
        state.skill_scroll + 1
    };
    let visible_end = (state.skill_scroll + VISIBLE_SKILLS).min(skills.len());
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        &format!(
            "{visible_start}–{visible_end} / {}  (wheel to scroll)",
            skills.len()
        ),
        320,
        346,
        10.0,
        220,
        15,
        [177, 170, 157],
    );

    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        16,
        374,
        width - 32,
        1,
        [101, 88, 66, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Select a slot, then click a learned active skill to assign it.",
        20,
        385,
        11.0,
        width - 40,
        16,
        [201, 195, 181],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        256,
        410,
        128,
        28,
        [61, 55, 46, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        396,
        410,
        128,
        28,
        [76, 94, 57, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Cancel",
        292,
        417,
        12.0,
        62,
        16,
        [228, 218, 201],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Save macros",
        414,
        417,
        12.0,
        105,
        16,
        [228, 242, 204],
    );
    pixels
}
