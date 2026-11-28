//! Docked skill macro panel. Slots are cast from left to right.

use super::ui_foundation::UiRect;
use super::*;
use openstory_protocol::{SKILL_MACRO_COUNT, SKILL_MACRO_SKILL_COUNT, SkillMacro};

pub(super) const DOCK_OVERLAP: f32 = 6.0;
pub(super) const PANEL_X: f32 = SKILL_BOOK_PANEL_X + SKILL_BOOK_PANEL_WIDTH as f32 - DOCK_OVERLAP;
pub(super) const PANEL_Y: f32 = SKILL_BOOK_PANEL_Y;
pub(super) const PANEL_WIDTH: u32 = 166;
pub(super) const PANEL_HEIGHT: u32 = 149;
pub(super) const NAMES: [&str; SKILL_MACRO_COUNT] = ["Sword", "Book", "Shield"];

fn cell(row: usize, column: usize) -> UiRect {
    let x = if column == 0 {
        9
    } else {
        49 + (column - 1) * 36
    };
    UiRect::new(x as f32, 29.0 + row as f32 * 38.0, 32.0, 32.0)
}

pub(super) fn emblem_at(x: f32, y: f32) -> Option<usize> {
    (0..SKILL_MACRO_COUNT).find(|&row| cell(row, 0).contains(x, y))
}

pub(super) fn slot_at(x: f32, y: f32) -> Option<(usize, usize)> {
    for row in 0..SKILL_MACRO_COUNT {
        for slot in 0..SKILL_MACRO_SKILL_COUNT {
            if cell(row, slot + 1).contains(x, y) {
                return Some((row, slot));
            }
        }
    }
    None
}

pub(super) fn render(
    font: &fontdue::Font,
    ui: &NxArchive,
    skills: &NxArchive,
    macros: &[SkillMacro],
) -> Vec<u8> {
    let (width, height) = (PANEL_WIDTH as usize, PANEL_HEIGHT as usize);
    let mut pixels = vec![0; width * height * 4];
    let _ = NxDialog::new(&mut pixels, width, height, ui).window_chrome_with_footer(
        UiRect::new(0.0, 0.0, PANEL_WIDTH as f32, PANEL_HEIGHT as f32),
        font,
        "Skill Macros",
        UiRect::new(8.0, 5.0, 150.0, 12.0),
        8,
    );

    for row in 0..SKILL_MACRO_COUNT {
        for column in 0..=SKILL_MACRO_SKILL_COUNT {
            let rect = cell(row, column);
            let (x, y) = (rect.x as usize, rect.y as usize);
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                x,
                y,
                32,
                32,
                [145, 132, 113, 255],
            );
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                x + 1,
                y + 1,
                30,
                30,
                [189, 176, 158, 255],
            );
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                x + 2,
                y + 2,
                28,
                28,
                [225, 218, 203, 255],
            );

            let icon = if column == 0 {
                decode_nx_icon(ui, &format!("UIWindow.img/SkillMacro/Macroicon/{row}/icon"))
            } else {
                let id = macros.get(row).map_or(0, |entry| entry.skills[column - 1]);
                (id > 0)
                    .then(|| {
                        decode_nx_icon(
                            skills,
                            &format!("{:03}.img/skill/{id:07}/icon", id / 10_000),
                        )
                    })
                    .flatten()
            };
            if let Some((w, h, icon)) = icon {
                composite_inventory_icon(
                    &mut pixels,
                    width,
                    height,
                    &icon,
                    w as usize,
                    h as usize,
                    x + 2,
                    y + 2,
                    28,
                );
            }
        }
    }
    pixels
}
