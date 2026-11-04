//! Native editor for the three v83 skill macros.

use super::{NxArchive, composite_nx_ui_bitmap, draw_dialogue_text};
use openstory_protocol::{MAX_SKILL_MACRO_NAME_CHARS, SKILL_MACRO_COUNT, SkillMacro};

pub(super) const PANEL_X: f32 = 408.0;
pub(super) const PANEL_Y: f32 = 195.0;
pub(super) const PANEL_WIDTH: u32 = 207;
pub(super) const PANEL_HEIGHT: u32 = 375;

pub(super) const VISIBLE_SKILLS: usize = 3;
const SKILL_ROW_TOP: f32 = 294.0;
const SKILL_ROW_HEIGHT: f32 = 19.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    Close,
    Save,
    SelectMacro(usize),
    FocusName,
    ToggleShout,
    SelectSlot(usize, usize),
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
    for row in 0..SKILL_MACRO_COUNT {
        let top = 46.0 + row as f32 * 44.0;
        if (top..top + 32.0).contains(&local_y) {
            for slot in 0..3 {
                let left = 14.0 + slot as f32 * 36.0;
                if (left..left + 32.0).contains(&local_x) {
                    if row == state.selected_macro && local_x >= left + 24.0 && local_y < top + 9.0
                    {
                        return Some(Action::ClearSkill(slot));
                    }
                    return Some(Action::SelectSlot(row, slot));
                }
            }
            if (138.0..170.0).contains(&local_x) {
                return Some(Action::SelectMacro(row));
            }
        }
    }
    if (60.0..180.0).contains(&local_x) && (200.0..220.0).contains(&local_y) {
        return Some(Action::FocusName);
    }
    if (144.0..170.0).contains(&local_x) && (228.0..246.0).contains(&local_y) {
        return Some(Action::ToggleShout);
    }
    if (SKILL_ROW_TOP..SKILL_ROW_TOP + VISIBLE_SKILLS as f32 * SKILL_ROW_HEIGHT).contains(&local_y)
        && (6.0..201.0).contains(&local_x)
    {
        let row = ((local_y - SKILL_ROW_TOP) / SKILL_ROW_HEIGHT).floor() as usize;
        return skills
            .get(state.skill_scroll.saturating_add(row))
            .map(|(id, _)| Action::AssignSkill(*id));
    }
    if (350.0..375.0).contains(&local_y) {
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
    skill_archive: &NxArchive,
    state: &State,
    skills: &[(i32, String)],
) -> Vec<u8> {
    let width = PANEL_WIDTH as usize;
    let height = PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    let native = composite_nx_ui_bitmap(
        &mut pixels,
        width,
        height,
        ui_archive,
        "UIWindow2.img/Skill/macro/backgrnd",
        1,
        0,
    )
    .is_ok();
    if native {
        for layer in ["backgrnd2", "backgrnd3"] {
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                width,
                height,
                ui_archive,
                &format!("UIWindow2.img/Skill/macro/{layer}"),
                0,
                0,
            );
        }
    } else {
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            width,
            height,
            ui_archive,
            "UIWindow.img/SkillMacro/backgrnd",
            0,
            0,
        );
        for row in 0..SKILL_MACRO_COUNT {
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                width,
                height,
                ui_archive,
                "UIWindow.img/SkillMacro/macroslot2",
                12,
                40 + row as i32 * 44,
            );
        }
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            width,
            height,
            ui_archive,
            "UIWindow.img/SkillMacro/macroname",
            14,
            185,
        );
    }
    // The original panel edits three macro rows. The additional list below it
    // retains click-to-assign for clients without skill-icon dragging.
    super::fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        289,
        width,
        height - 289,
        [242, 242, 237, 255],
    );
    super::fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        289,
        width,
        1,
        [100, 118, 133, 255],
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
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        width,
        height,
        ui_archive,
        "UIWindow2.img/Skill/macro/select",
        13,
        40 + state.selected_macro as i32 * 44,
    );
    for row in 0..SKILL_MACRO_COUNT {
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            width,
            height,
            ui_archive,
            &format!("UIWindow.img/SkillMacro/Macroicon/{row}/icon"),
            138,
            72 + row as i32 * 44,
        );
        for slot in 0..3 {
            let x = 14 + slot as i32 * 36;
            let y = 46 + row as i32 * 44;
            let skill_id = state.drafts.get(row).map_or(0, |entry| entry.skills[slot]);
            if skill_id > 0
                && composite_nx_ui_bitmap(
                    &mut pixels,
                    width,
                    height,
                    skill_archive,
                    &format!("{:03}.img/skill/{skill_id}/icon", skill_id / 10_000),
                    x + 1,
                    y + 30,
                )
                .is_err()
            {
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    width,
                    height,
                    "?",
                    x + 10,
                    y + 8,
                    12.0,
                    14,
                    16,
                    [80, 60, 40],
                );
            }
            if row == state.selected_macro && slot == state.selected_slot {
                for (dx, dy, w, h) in [(0, 0, 32, 1), (0, 31, 32, 1), (0, 0, 1, 32), (31, 0, 1, 32)]
                {
                    super::fill_dialogue_rect(
                        &mut pixels,
                        width,
                        height,
                        (x + dx) as usize,
                        (y + dy) as usize,
                        w,
                        h,
                        [55, 110, 180, 255],
                    );
                }
            }
            if row == state.selected_macro && skill_id > 0 {
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    width,
                    height,
                    "×",
                    x + 24,
                    y,
                    9.0,
                    8,
                    10,
                    [130, 30, 30],
                );
            }
        }
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
        60,
        200,
        10.0,
        120,
        20,
        [56, 42, 31],
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        width,
        height,
        ui_archive,
        &format!(
            "UIWindow.img/SkillMacro/Macroicon/{}/icon",
            state.selected_macro
        ),
        15,
        221,
    );
    if current.is_some_and(|entry| entry.shout) {
        if native {
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                width,
                height,
                ui_archive,
                "UIWindow2.img/Skill/macro/check",
                0,
                0,
            );
        }
    }
    if !native {
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
            144,
            230,
        );
    }
    for (row, (_, name)) in skills
        .iter()
        .skip(state.skill_scroll)
        .take(VISIBLE_SKILLS)
        .enumerate()
    {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            name,
            9,
            SKILL_ROW_TOP as i32 + row as i32 * SKILL_ROW_HEIGHT as i32,
            9.0,
            188,
            16,
            [70, 48, 33],
        );
    }
    for (path, x) in [
        ("Basic.img/BtCancel/normal/0", 6),
        ("Basic.img/BtOK/normal/0", 108),
    ] {
        let _ = composite_nx_ui_bitmap(&mut pixels, width, height, ui_archive, path, x, 351);
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_and_skill_list_hit_the_drawn_cells() {
        let mut state = State::default();
        state.open(&[]);
        let skills = vec![(1001004, "Power Strike".to_owned())];
        assert_eq!(
            hit_test(15.0, 47.0, &state, &skills),
            Some(Action::SelectSlot(0, 0))
        );
        assert_eq!(
            hit_test(51.0, 91.0, &state, &skills),
            Some(Action::SelectSlot(1, 1))
        );
        assert_eq!(
            hit_test(140.0, 47.0, &state, &skills),
            Some(Action::SelectMacro(0))
        );
        assert_eq!(
            hit_test(39.0, 47.0, &state, &skills),
            Some(Action::ClearSkill(0))
        );
        assert_eq!(
            hit_test(65.0, 205.0, &state, &skills),
            Some(Action::FocusName)
        );
        assert_eq!(
            hit_test(150.0, 235.0, &state, &skills),
            Some(Action::ToggleShout)
        );
        assert_eq!(
            hit_test(10.0, 295.0, &state, &skills),
            Some(Action::AssignSkill(1001004))
        );
        assert_eq!(hit_test(110.0, 355.0, &state, &skills), Some(Action::Save));
    }
}
