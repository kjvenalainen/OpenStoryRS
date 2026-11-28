use super::*;

pub(super) const WIDTH: u32 = 320;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SkillTooltipKey {
    id: i32,
    level: i32,
    cap: i32,
}

impl SkillTooltipKey {
    fn from_entry(skill: &SkillUiEntry) -> Self {
        Self {
            id: skill.id,
            level: skill.level.max(0),
            cap: if skill.master_level > 0 {
                skill.max_level.min(skill.master_level)
            } else {
                skill.max_level
            }
            .max(0),
        }
    }

    fn next_level(self) -> Option<i32> {
        (self.level < self.cap).then(|| self.level + 1)
    }
}

// Skill strings contain literal line breaks and the legacy text color codes.
// Keep their wording, using the tooltip's readable light text palette.
fn plain_text(text: &str) -> String {
    let normalized = text
        .replace("\\r\\n", "\n")
        .replace("\\n", "\n")
        .replace("\\r", "\n")
        .replace('\r', "");
    let mut chars = normalized.chars().peekable();
    let mut result = String::new();
    while let Some(character) = chars.next() {
        if character == '#' {
            if chars
                .peek()
                .is_some_and(|code| matches!(code, 'b' | 'k' | 'r' | 'c' | 'd' | 'e' | 'n' | 'g'))
            {
                chars.next();
            }
        } else {
            result.push(character);
        }
    }
    result.trim().to_owned()
}

fn description(text: &str) -> String {
    let mut text = plain_text(text);
    // The header uses the server's mastery cap, which can be lower than NX's.
    if let Some(label_start) = text.find("Master Level")
        && let Some(end) = text[label_start..].find(']')
    {
        let start = if text[..label_start].ends_with('[') {
            label_start - 1
        } else {
            label_start
        };
        text.replace_range(start..label_start + end + 1, "");
    }
    text.trim().to_owned()
}

fn level_text(
    skill_archive: &NxArchive,
    string_archive: &NxArchive,
    id: i32,
    level: i32,
) -> String {
    let stats = skill_archive
        .root()
        .resolve(&format!(
            "{:03}.img/skill/{id:07}/level/{level}",
            id / 10_000
        ))
        .ok()
        .flatten();
    let text_key = stats
        .as_ref()
        .and_then(|node| string_property(node, "hs"))
        .unwrap_or_else(|| format!("h{level}"));
    let effects = nx_string_value(string_archive, &format!("Skill.img/{id:07}/{text_key}"))
        .or_else(|| nx_string_value(string_archive, &format!("Skill.img/{id:07}/h{level}")))
        .map(|text| plain_text(&text))
        .filter(|text| !text.is_empty());
    let mut lines = Vec::new();
    if let Some(stats) = stats {
        let mut costs = Vec::new();
        for (key, label) in [
            ("hpCon", "HP"),
            ("mpCon", "MP"),
            ("moneyCon", "mesos"),
            ("bulletConsume", "ammo"),
        ] {
            if let Some(value) = integer_property(&stats, key).filter(|value| *value > 0) {
                costs.push(format!("{value} {label}"));
            }
        }
        if let Some(item) = integer_property(&stats, "itemCon").filter(|value| *value > 0) {
            let count = integer_property(&stats, "itemConNo").unwrap_or(1).max(1);
            let name = i32::try_from(item)
                .ok()
                .map(|id| item_name(id, string_archive, &mut HashMap::new()))
                .unwrap_or_else(|| format!("Item {item}"));
            costs.push(format!("{count} {name}"));
        }
        if !costs.is_empty() {
            lines.push(format!("Cost: {}", costs.join(", ")));
        }
        if let Some(cooldown) = integer_property(&stats, "cooltime").filter(|value| *value > 0) {
            lines.push(format!("Cooldown: {cooldown} sec"));
        }
    }
    if let Some(effects) = effects {
        lines.push(effects);
    }
    if lines.is_empty() {
        "Details unavailable in local skill data.".to_owned()
    } else {
        lines.join("\n")
    }
}

pub(super) fn render(
    font: &fontdue::Font,
    skill: &SkillUiEntry,
    skill_archive: &NxArchive,
    string_archive: &NxArchive,
) -> (u32, Vec<u8>) {
    render_with_remaining(font, skill, skill_archive, string_archive, None)
}

pub(super) fn render_with_remaining(
    font: &fontdue::Font,
    skill: &SkillUiEntry,
    skill_archive: &NxArchive,
    string_archive: &NxArchive,
    remaining: Option<&str>,
) -> (u32, Vec<u8>) {
    let key = SkillTooltipKey::from_entry(skill);
    let mut description =
        nx_string_value(string_archive, &format!("Skill.img/{:07}/desc", skill.id))
            .map(|text| description(&text))
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "Description unavailable in local skill data.".to_owned());
    if let Some(remaining) = remaining {
        description = format!("Time remaining: {remaining}\n\n{description}");
    }
    let mut sections = Vec::new();
    if key.level > 0 {
        sections.push((
            format!("Current Level: {}", key.level),
            level_text(skill_archive, string_archive, key.id, key.level),
        ));
    }
    if let Some(level) = key.next_level() {
        sections.push((
            format!("Next Level: {level}"),
            level_text(skill_archive, string_archive, key.id, level),
        ));
    }
    let width = WIDTH as usize;
    let text_height = |text: &str, width: usize| {
        dialogue_text_content_height(font, text, 11.0, width, &EMPTY_NPC_INLINE_ICONS)
    };
    let description_height = text_height(&description, width - 98);
    let divider_y = 51 + description_height.max(62) + 10;
    let section_heights = sections
        .iter()
        .map(|(_, text)| text_height(text, width - 24))
        .collect::<Vec<_>>();
    let height = (divider_y
        + 12
        + section_heights
            .iter()
            .map(|height| height + 27)
            .sum::<usize>())
    .min(748);
    let mut pixels = vec![0; width * height * 4];
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        height,
        [82, 54, 51, 218],
    );
    for (x, y, w, h) in [
        (0, 0, width, 1),
        (0, 0, 1, height),
        (width - 1, 0, 1, height),
        (0, height - 1, width, 1),
    ] {
        fill_dialogue_rect(&mut pixels, width, height, x, y, w, h, [230, 225, 228, 255]);
    }
    let name = ui_foundation::ellipsize(font, &skill.name, 13.0, (width - 24) as f32);
    ui_foundation::draw_text_role(
        font,
        &mut pixels,
        width,
        height,
        &name,
        12,
        9,
        ui_foundation::UiTextRole::Title,
        width - 24,
        19,
        Some([255, 255, 255]),
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        &format!(
            "Level {} / {}{}",
            key.level,
            key.cap,
            if skill.passive { "  (Passive)" } else { "" }
        ),
        12,
        30,
        11.0,
        width - 24,
        16,
        [255, 175, 72],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        12,
        52,
        66,
        66,
        [150, 150, 150, 255],
    );
    if let Some((w, h, icon)) = decode_nx_icon(
        skill_archive,
        &format!("{:03}.img/skill/{:07}/icon", skill.id / 10_000, skill.id),
    ) {
        composite_inventory_icon(
            &mut pixels,
            width,
            height,
            &icon,
            w as usize,
            h as usize,
            13,
            53,
            64,
        );
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        &description,
        86,
        51,
        11.0,
        width - 98,
        description_height,
        [255, 255, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        10,
        divider_y,
        width - 20,
        1,
        [220, 220, 220, 255],
    );
    let mut y = divider_y + 9;
    for ((title, text), text_height) in sections.iter().zip(section_heights) {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            title,
            12,
            y as i32,
            11.0,
            width - 24,
            16,
            [255, 175, 72],
        );
        y += 18;
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            text,
            12,
            y as i32,
            11.0,
            width - 24,
            text_height,
            [255, 255, 255],
        );
        y += text_height + 9;
    }
    (height as u32, pixels)
}

pub(super) fn position(cursor: Point, height: u32) -> Point {
    let width = WIDTH as f32;
    let height = height as f32;
    Point {
        x: if cursor.x + 18.0 + width > ui_foundation::LOGICAL_WIDTH {
            cursor.x - width - 18.0
        } else {
            cursor.x + 18.0
        }
        .clamp(0.0, ui_foundation::LOGICAL_WIDTH - width),
        y: if cursor.y + 18.0 + height > ui_foundation::LOGICAL_HEIGHT {
            cursor.y - height - 18.0
        } else {
            cursor.y + 18.0
        }
        .clamp(0.0, ui_foundation::LOGICAL_HEIGHT - height),
    }
}

impl ClientApp {
    // Re-evaluate before drawing so scrolling, server level updates, dialog
    // stacking, dragging, and resizing also work with a stationary pointer.
    pub(super) fn refresh_skill_tooltip(&mut self) {
        let buff_hover = self
            .cursor
            .filter(|cursor| {
                self.current_field.is_some()
                    && self.cash_shop.is_none()
                    && self.npc_shop.is_none()
                    && self.dialog_window_drag.is_none()
                    && self.skill_book_drag.is_none()
                    && self.inventory_drag_source.is_none()
                    && self.inventory_drop_prompt.is_none()
                    && !self.quit_confirmation
                    && !self.wedding_panel_open
                    && self.server_notice_popup.is_none()
                    && self.rps_game.is_none()
                    && (24.0..56.0).contains(&cursor.y)
            })
            .and_then(|cursor| {
                self.buff_list_skill_ids
                    .iter()
                    .enumerate()
                    .find(|(index, _)| {
                        let x = 1024.0 - 35.0 - 32.0 * *index as f32;
                        (x..x + 32.0).contains(&cursor.x)
                    })
                    .map(|(_, skill_id)| *skill_id)
            });
        let quickslot_skill_id = self
            .quickslot_hover_slot
            .filter(|_| {
                self.current_field.is_some()
                    && self.show_quickslots
                    && self.cash_shop.is_none()
                    && self.npc_shop.is_none()
                    && self.dialog_window_drag.is_none()
                    && self.skill_book_drag.is_none()
                    && self.inventory_drag_source.is_none()
                    && self.inventory_drop_prompt.is_none()
                    && !self.quit_confirmation
                    && !self.wedding_panel_open
                    && self.server_notice_popup.is_none()
                    && self.rps_game.is_none()
            })
            .and_then(|slot| self.current_quickslot_binding(slot))
            .filter(|binding| binding.kind == 1)
            .map(|binding| binding.action);
        self.quickslot_tooltip_skill_id = quickslot_skill_id;
        let now = Instant::now();
        let buff_remaining = buff_hover.and_then(|skill_id| {
            let buff = self
                .active_buffs
                .values()
                .copied()
                .find(|buff| buff.skill_id == skill_id && buff.is_active_at(now))?;
            Some(match buff.expires_at {
                Some(expires_at) => format_remaining(expires_at.saturating_duration_since(now)),
                None => "Unlimited".to_owned(),
            })
        });
        let buff_skill = buff_hover
            .filter(|_| buff_remaining.is_some())
            .and_then(|id| self.skill_ui_entry_by_id(id));
        let quickslot_skill = quickslot_skill_id.and_then(|id| self.skill_ui_entry_by_id(id));
        let skillbook_hover = if buff_skill.is_none() && quickslot_skill.is_none() {
            self.cursor
                .filter(|cursor| {
                    self.current_field.is_some()
                        && self.show_skill_book
                        && self.cash_shop.is_none()
                        && self.dialog_window_drag.is_none()
                        && self.skill_book_drag.is_none()
                        && self.inventory_drag_source.is_none()
                        && self.inventory_drop_prompt.is_none()
                        && !self.quit_confirmation
                        && !self.wedding_panel_open
                        && self.server_notice_popup.is_none()
                        && self.rps_game.is_none()
                        && self.dialog_window_hit_test(DialogWindow::SkillBook, *cursor)
                })
                .and_then(|cursor| {
                    let (x, y) = self.dialog_window_offset(DialogWindow::SkillBook);
                    (0..SKILL_BOOK_VISIBLE_ROWS).find(|row| {
                        ui_foundation::skill_row(*row).contains(
                            cursor.x - x - SKILL_BOOK_PANEL_X,
                            cursor.y - y - SKILL_BOOK_PANEL_Y,
                        )
                    })
                })
                .and_then(|row| {
                    self.skill_book_entries
                        .get(self.skill_book_scroll_row.saturating_add(row))
                        .cloned()
                })
        } else {
            None
        };
        let has_buff_tooltip = buff_skill.is_some();
        let hovered = buff_skill.or(quickslot_skill).or(skillbook_hover);
        let remaining = if has_buff_tooltip {
            buff_remaining
        } else {
            None
        };
        let key = hovered.as_ref().map(SkillTooltipKey::from_entry);
        if key == self.skill_tooltip_key && remaining == self.skill_tooltip_remaining {
            return;
        }
        self.skill_tooltip_key = key;
        self.skill_tooltip_remaining = remaining.clone();
        self.skill_tooltip_texture = None;
        let (Some(skill), Some(renderer)) = (hovered.as_ref(), self.renderer.as_ref()) else {
            return;
        };
        let (height, pixels) = render_with_remaining(
            &self.font,
            skill,
            &self.skill_archive,
            &self.string_archive,
            remaining.as_deref(),
        );
        match renderer.upload_bgra(WIDTH, height, &pixels) {
            Ok(texture) => {
                self.skill_tooltip_texture = Some(texture);
                self.skill_tooltip_height = height;
            }
            Err(error) => eprintln!("OpenStoryRS: could not update skill tooltip: {error}"),
        }
    }
}

fn format_remaining(remaining: Duration) -> String {
    let total_seconds = remaining
        .as_secs()
        .saturating_add(u64::from(remaining.subsec_nanos() > 0));
    let hours = total_seconds / 3_600;
    let minutes = (total_seconds / 60) % 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{}:{seconds:02}", total_seconds / 60)
    }
}
