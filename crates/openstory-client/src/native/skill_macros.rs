//! Native editor for the three v83 skill macros.

use super::{NxArchive, composite_nx_ui_bitmap, draw_dialogue_text};
use openstory_protocol::{MAX_SKILL_MACRO_NAME_CHARS, SKILL_MACRO_COUNT, SkillMacro};

pub(super) const PANEL_X: f32 = 408.0;
pub(super) const PANEL_Y: f32 = 239.0;
pub(super) const PANEL_WIDTH: u32 = 207;
pub(super) const PANEL_HEIGHT: u32 = 289;

pub(super) const VISIBLE_SKILLS: usize = 3;
const SKILL_ROW_TOP: f32 = 188.0;
const SKILL_ROW_HEIGHT: f32 = 19.0;

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
    if local_y < 24.0 && local_x >= PANEL_WIDTH as f32 - 32.0 {
        return Some(Action::Close);
    }
    if (27.0..54.0).contains(&local_y) && (6.0..201.0).contains(&local_x) {
        let index = ((local_x - 6.0) / 65.0).floor() as usize;
        return (index < SKILL_MACRO_COUNT).then_some(Action::SelectMacro(index));
    }
    if (56.0..80.0).contains(&local_y) && (42.0..201.0).contains(&local_x) {
        return Some(Action::FocusName);
    }
    if (81.0..103.0).contains(&local_y) && (6.0..201.0).contains(&local_x) {
        return Some(Action::ToggleShout);
    }
    if (107.0..182.0).contains(&local_y) && (6.0..201.0).contains(&local_x) {
        let slot = ((local_y - 107.0) / 25.0).floor() as usize;
        if slot < 3 {
            return Some(if local_x >= 170.0 {
                Action::ClearSkill(slot)
            } else {
                Action::SelectSlot(slot)
            });
        }
    }
    if (SKILL_ROW_TOP..SKILL_ROW_TOP + VISIBLE_SKILLS as f32 * SKILL_ROW_HEIGHT).contains(&local_y)
        && (6.0..201.0).contains(&local_x)
    {
        let row = ((local_y - SKILL_ROW_TOP) / SKILL_ROW_HEIGHT).floor() as usize;
        return skills
            .get(state.skill_scroll.saturating_add(row))
            .map(|(id, _)| Action::AssignSkill(*id));
    }
    if (258.0..286.0).contains(&local_y) {
        if (108.0..201.0).contains(&local_x) {
            return Some(Action::Save);
        }
        if (6.0..101.0).contains(&local_x) {
            return Some(Action::Close);
        }
    }
    None
}

pub(super) fn skill_list_contains(local_x: f32, local_y: f32) -> bool {
    (6.0..201.0).contains(&local_x)
        && (SKILL_ROW_TOP..SKILL_ROW_TOP + VISIBLE_SKILLS as f32 * SKILL_ROW_HEIGHT)
            .contains(&local_y)
}

pub(super) fn render(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    state: &State,
    skills: &[(i32, String)],
) -> Vec<u8> {
    let width = PANEL_WIDTH as usize;
    let height = PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        width,
        height,
        ui_archive,
        "UIWindow.img/SkillMacro/backgrnd",
        0,
        0,
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        width,
        height,
        ui_archive,
        "UIWindow.img/BtUIClose/normal/0",
        width as i32 - 32,
        6,
    );

    // The NX background already supplies the macro tabs and their frames.
    // Overlay only the selected tab's number instead of scaling a slot texture
    // across the baked tab artwork.
    for index in 0..SKILL_MACRO_COUNT {
        let x = 6 + index * 65;
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("{}", index + 1),
            x as i32 + 26,
            33,
            11.0,
            14,
            15,
            if index == state.selected_macro {
                [172, 89, 36]
            } else {
                [72, 50, 31]
            },
        );
    }

    let current = state.drafts.get(state.selected_macro);
    let name = current.map_or("", |entry| entry.name.as_str());
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        &if state.name_focused {
            format!("{name}|")
        } else {
            name.to_owned()
        },
        47,
        62,
        10.0,
        148,
        14,
        [56, 42, 31],
    );

    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        width,
        height,
        ui_archive,
        if current.is_some_and(|entry| entry.shout) {
            "Basic.img/CheckBox/1"
        } else {
            "Basic.img/CheckBox/0"
        },
        8,
        83,
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Shout in chat",
        27,
        85,
        8.0,
        112,
        12,
        [70, 48, 33],
    );

    for slot in 0..3 {
        let y = 108 + slot * 25;
        let selected = slot == state.selected_slot;
        let skill_id = current.map_or(0, |entry| entry.skills[slot]);
        let skill_name = skills.iter().find(|(id, _)| *id == skill_id).map_or_else(
            || {
                if skill_id == 0 {
                    "Empty".to_owned()
                } else {
                    format!("#{skill_id}")
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
            11,
            y as i32 + 5,
            9.0,
            151,
            13,
            if selected { [40, 63, 95] } else { [61, 46, 34] },
        );
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            width,
            height,
            ui_archive,
            "Basic.img/BtClose/normal/0",
            182,
            y as i32 + 5,
        );
    }

    for (row, (skill_id, skill_name)) in skills
        .iter()
        .skip(state.skill_scroll)
        .take(VISIBLE_SKILLS)
        .enumerate()
    {
        let y = SKILL_ROW_TOP as usize + row * SKILL_ROW_HEIGHT as usize;
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("{}  {}", skill_id, skill_name),
            9,
            y as i32,
            9.0,
            188,
            14,
            [70, 48, 33],
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
        &format!("{visible_start}–{visible_end}/{}", skills.len()),
        9,
        246,
        8.0,
        65,
        12,
        [88, 68, 50],
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        width,
        height,
        ui_archive,
        "Basic.img/BtCancel/normal/0",
        6,
        259,
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        width,
        height,
        ui_archive,
        "Basic.img/BtOK/normal/0",
        108,
        259,
    );
    pixels
}
