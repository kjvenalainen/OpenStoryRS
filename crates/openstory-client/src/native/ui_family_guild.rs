fn render_family_panel(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    state: &FamilyUiState,
) -> Vec<u8> {
    let width = FAMILY_PANEL_WIDTH as usize;
    let height = FAMILY_PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    let _ = NxDialog::new(&mut pixels, width, height, ui_archive).scaled_frame(
        "UIWindow.img/Family/backgrnd",
        0, 0, width, height,
    );

    let _ = NxDialog::new(&mut pixels, width, height, ui_archive)
        .anchored_button("UIWindow.img/Family/BtClose/normal/0", 250, 10);

    if let Some(info) = state.info.as_ref() {
        let name = if info.name.trim().is_empty() { "No family" } else { info.name.as_str() };
        let name = ui_foundation::ellipsize(font, name, 11.0, (width - 32) as f32);
        draw_dialogue_text(font, &mut pixels, width, height, &name, 16, 36, 11.0, width - 32, 16, [255, 255, 255]);
        let motto = if info.message.trim().is_empty() { "Click to edit family precept" } else { info.message.as_str() };
        let motto = ui_foundation::ellipsize(font, motto, 10.0, (width - 36) as f32);
        draw_dialogue_text(font, &mut pixels, width, height, &motto, 17, 66, 10.0, width - 36, 15, [45, 45, 45]);
        draw_dialogue_text(font, &mut pixels, width, height, &format!("{} / {}", info.junior_count.max(0), info.juniors_allowed.max(0)), 70, 91, 10.0, width - 85, 14, [45, 45, 45]);
        draw_dialogue_text(font, &mut pixels, width, height, &format!("{} / {}", info.current_reputation, info.total_reputation), 65, 122, 10.0, width - 78, 14, [45, 45, 45]);
        draw_dialogue_text(font, &mut pixels, width, height, &info.todays_reputation.to_string(), 130, 141, 10.0, width - 145, 14, [45, 45, 45]);
    } else {
        draw_dialogue_text(font, &mut pixels, width, height, "Requesting family information…", 17, 66, 10.0, width - 36, 15, [70, 74, 80]);
    }

    let default_abilities = [
        ("Teleport", "Teleport to a selected family member."),
        ("Summon", "Summon a selected family member."),
        ("Drop buff", "Increase item drop rate for a short time."),
        ("EXP buff", "Increase EXP gained for a short time."),
        ("Drop + EXP", "Increase drop rate and EXP for a short time."),
    ];
    let ability_index = state.selected_ability.min(default_abilities.len() - 1);
    let fallback = default_abilities[ability_index];
    let privilege = state
        .privileges
        .iter()
        .find(|privilege| usize::from(privilege.kind) == ability_index);
    let ability_name = privilege.map_or(fallback.0, |privilege| privilege.name.as_str());
    let ability_description =
        privilege.map_or(fallback.1, |privilege| privilege.description.as_str());
    let ability_name = ui_foundation::ellipsize(font, ability_name, 10.0, (width - 40) as f32);
    draw_dialogue_text(font, &mut pixels, width, height, &ability_name, 20, 201, 10.0, width - 40, 15, [45, 45, 45]);
    let cost = privilege.map_or_else(|| "—".to_owned(), |item| item.reputation_cost.to_string());
    let uses = privilege.map_or_else(|| "—".to_owned(), |item| {
        let used = state.info.as_ref().and_then(|info| info.entitlement_usage.iter().find(|usage| usage.ordinal == i32::from(item.kind))).map_or(0, |usage| usage.used);
        format!("{} / {}", used, item.usage_limit)
    });
    draw_dialogue_text(font, &mut pixels, width, height, &cost, 22, 240, 10.0, 98, 15, [45, 45, 45]);
    draw_dialogue_text(font, &mut pixels, width, height, &uses, 155, 240, 10.0, 90, 15, [45, 45, 45]);
    let icon = format!("UIWindow.img/Family/RightIcon/{ability_index}");
    let _ = composite_nx_ui_bitmap(&mut pixels, width, height, ui_archive, &icon, 215, 300);
    for (path, x) in [("UIWindow.img/Family/BtLeft/normal/0", 204), ("UIWindow.img/Family/BtRight/normal/0", 244)] {
        let _ = composite_nx_ui_bitmap(&mut pixels, width, height, ui_archive, path, x, 305);
    }
    draw_dialogue_text(font, &mut pixels, width, height, ability_description, 20, 270, 9.0, 175, 65, [58, 58, 58]);
    if state.input.is_none() && let Some(notice) = state.notice.as_deref() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            notice,
            18,
            360,
            8.0,
            width - 36,
            23,
            [150, 35, 35],
        );
    }
    for (path, x) in [
        ("UIWindow.img/Family/BtTree/normal/0", 10),
        ("UIWindow.img/Family/BtSpecial/normal/0", 69),
        ("UIWindow.img/Family/BtJuniorEntry/normal/0", 128),
    ] {
        let _ = composite_nx_ui_bitmap(&mut pixels, width, height, ui_archive, path, x, 397);
    }
    let _ = composite_nx_ui_bitmap(
        &mut pixels, width, height, ui_archive,
        "UIWindow.img/UserList/Guild/GuildInfo/BtWithdraw/normal/0", 201, 397,
    );

    if state.input.is_some() || state.confirm_leave {
        for pixel in pixels.chunks_exact_mut(4) {
            for channel in &mut pixel[..3] { *channel = (u16::from(*channel) * 105 / 255) as u8; }
        }
        let _ = NxDialog::new(&mut pixels, width, height, ui_archive)
            .frame("Basic.img/YesNo/backgrnd", 5, 145);
        if let Some(input) = state.input.as_ref() {
            let heading = match input.kind {
                FamilyInputKind::InviteJunior => "Enter the junior's name:",
                FamilyInputKind::EditPrecepts => "Enter the family precept:",
            };
            draw_dialogue_text(font, &mut pixels, width, height, heading, 30, 166, 11.0, 214, 17, [35, 35, 35]);
            let text = family_input_visible_text(font, &input.text, 208.0);
            draw_dialogue_text(font, &mut pixels, width, height, &text, 33, 196, 10.0, 208, 15, [35, 35, 35]);
            if let Some(notice) = state.notice.as_deref() {
                let notice = ui_foundation::ellipsize(font, notice, 9.0, 218.0);
                draw_dialogue_text(font, &mut pixels, width, height, &notice, 28, 219, 9.0, 218, 13, [150, 35, 35]);
            }
        } else {
            draw_dialogue_text(font, &mut pixels, width, height, "Leave your family?", 30, 169, 12.0, 214, 18, [35, 35, 35]);
            draw_dialogue_text(font, &mut pixels, width, height, "Your family relationship will end.", 30, 193, 10.0, 214, 24, [55, 55, 55]);
        }
        let (accept, cancel) = if state.input.is_some() { ("Basic.img/BtOK/normal/0", "Basic.img/BtCancel/normal/0") } else { ("Basic.img/BtYes/normal/0", "Basic.img/BtNo/normal/0") };
        NxDialog::new(&mut pixels, width, height, ui_archive)
            .anchored_buttons(&[(accept, 64, 239), (cancel, 145, 239)]);
    }

    pixels
}

// Keep the caret end visible when a long family precept exceeds the field.
fn family_input_visible_text(font: &fontdue::Font, text: &str, max_width: f32) -> String {
    if dialogue_text_width(font, text, 10.0) <= max_width { return text.to_owned(); }
    for (index, _) in text.char_indices().skip(1) {
        let candidate = format!("…{}", &text[index..]);
        if dialogue_text_width(font, &candidate, 10.0) <= max_width { return candidate; }
    }
    "…".to_owned()
}

#[cfg(test)]
mod family_input_layout_tests {
    use super::*;

    #[test]
    fn long_unicode_input_keeps_its_end_visible_within_the_field() {
        let font = fontdue::Font::from_bytes(include_bytes!("../../assets/fonts/Roboto-Regular.ttf").as_slice(), fontdue::FontSettings::default()).unwrap();
        let text = "Long family message with accented characters éé and a visible ending";
        let visible = family_input_visible_text(&font, text, 100.0);
        assert!(visible.starts_with('…'));
        assert!(visible.ends_with("ending"));
        assert!(dialogue_text_width(&font, &visible, 10.0) <= 100.0);
        assert_eq!(family_input_visible_text(&font, "short", 100.0), "short");
    }
}

fn render_family_tree_panel(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    state: &FamilyUiState,
) -> Vec<u8> {
    let width = FAMILY_TREE_PANEL_WIDTH as usize;
    let height = FAMILY_TREE_PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    draw_family_nx_background(
        &mut pixels,
        width,
        height,
        ui_archive,
        "UIWindow.img/FamilyTree/backgrnd",
        1.0,
    );
    if let Some(chart) = state.chart.as_ref() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &chart.member_counters.iter().find(|counter| counter.id_or_flag == -1 && counter.count >= 0).map(|counter| counter.count.to_string()).unwrap_or_else(|| {
                chart.entries.iter().map(|entry| entry.character_id).collect::<std::collections::HashSet<_>>().len().to_string()
            }),
            25,
            40,
            9.0,
            width - 36,
            15,
            [45, 40, 32],
        );
        for (index, x, y) in family_tree_slots(chart) {
            let entry = &chart.entries[index];
            let path = if entry.parent_id == 0 {
                "UIWindow.img/FamilyTree/PlateLeader/0"
            } else {
                "UIWindow.img/FamilyTree/PlateOthers/0"
            };
            let _ = composite_nx_ui_bitmap(&mut pixels, width, height, ui_archive, path, x, y);
            if state.selected_member == Some(entry.character_id) {
                let _ = composite_nx_ui_bitmap(
                    &mut pixels,
                    width,
                    height,
                    ui_archive,
                    "UIWindow.img/FamilyTree/selected",
                    x - 3,
                    y - 3,
                );
            }
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &entry.name,
                x + 7,
                y + 3,
                10.0,
                120,
                14,
                if entry.online {
                    [35, 31, 27]
                } else {
                    [92, 88, 82]
                },
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!(
                    "Lv. {}  Job {}  Rep {}",
                    entry.level, entry.job, entry.current_reputation
                ),
                x + 7,
                y + 17,
                8.0,
                120,
                12,
                [58, 54, 49],
            );
        }
    } else {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Family pedigree data has not arrived yet.",
            170,
            345,
            10.0,
            width - 36,
            26,
            [221, 211, 188],
        );
    }
    pixels
}

// The leader has a separate plate above the two senior positions. Clicking a
// plate requests that member's own chart.
fn family_tree_slots(chart: &openstory_protocol::FamilyChart) -> Vec<(usize, i32, i32)> {
    let Some(viewed) = chart
        .entries
        .iter()
        .position(|entry| entry.character_id == chart.viewed_character_id)
        .or_else(|| (!chart.entries.is_empty()).then_some(0))
    else {
        return Vec::new();
    };
    let mut slots = vec![(viewed, 223, 200)];
    let leader = chart.entries.iter().position(|entry| entry.parent_id == 0);
    if let Some(index) = leader.filter(|&index| chart.entries[index].character_id != chart.entries[viewed].character_id) {
        slots.push((index, 223, 42));
    }
    let mut senior = viewed;
    for y in [147, 95] {
        let parent = chart.entries[senior].parent_id;
        let Some(index) = chart
            .entries
            .iter()
            .position(|entry| entry.character_id == parent && parent != 0)
        else {
            break;
        };
        if slots.iter().any(|(seen, _, _)| chart.entries[*seen].character_id == chart.entries[index].character_id) {
            break;
        }
        slots.push((index, 223, y));
        senior = index;
    }
    let parent_id = chart.entries[viewed].parent_id;
    if parent_id != 0 {
        if let Some((index, _)) = chart.entries.iter().enumerate().find(|(_, entry)| {
            entry.parent_id == parent_id && entry.character_id != chart.entries[viewed].character_id
        }) {
            slots.push((index, 365, 200));
        }
    }
    let children: Vec<usize> = chart
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            entry.parent_id == chart.entries[viewed].character_id
                && entry.character_id != chart.entries[viewed].character_id
        })
        .map(|(index, _)| index)
        .take(2)
        .collect();
    for (side, child) in children.into_iter().enumerate() {
        if slots.iter().any(|(seen, _, _)| *seen == child) {
            continue;
        }
        slots.push((child, [80, 365][side], 253));
        for (branch, (index, _)) in chart
            .entries
            .iter()
            .enumerate()
            .filter(|(index, entry)| {
                entry.parent_id == chart.entries[child].character_id
                    && !slots.iter().any(|(seen, _, _)| *seen == *index)
            })
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .enumerate()
        {
            slots.push((index, [14, 149, 298, 433][side * 2 + branch], 305));
        }
    }
    slots
}

#[cfg(test)]
mod family_tree_tests {
    use super::*;

    #[test]
    fn shows_leader_ancestors_and_sibling_in_distinct_plates() {
        let entries = [(1, 0), (2, 1), (3, 2), (4, 3), (5, 3), (6, 4)]
            .into_iter()
            .map(|(character_id, parent_id)| openstory_protocol::FamilyChartEntry {
                character_id,
                parent_id,
                job: 0,
                level: 1,
                online: false,
                current_reputation: 0,
                total_reputation: 0,
                reputation_to_senior: 0,
                todays_reputation: 0,
                channel: 0,
                time_online: 0,
                name: String::new(),
            })
            .collect();
        let chart = openstory_protocol::FamilyChart {
            viewed_character_id: 4,
            entries,
            member_counters: Vec::new(),
            entitlement_usage: Vec::new(),
            add_button: None,
        };
        let positions: Vec<_> = family_tree_slots(&chart)
            .into_iter()
            .map(|(index, x, y)| (chart.entries[index].character_id, x, y))
            .collect();
        assert_eq!(positions, [(4, 223, 200), (1, 223, 42), (3, 223, 147), (2, 223, 95), (5, 365, 200), (6, 80, 253)]);
    }
}

fn draw_family_nx_background(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    archive: &NxArchive,
    path: &str,
    scale: f32,
) {
    let Some((source_width, source_height, _, _)) = nx_ui_bitmap_geometry(archive, path) else {
        return;
    };
    let dest_width = ((source_width as f32 * scale).round() as usize).min(width);
    let dest_height = ((source_height as f32 * scale).round() as usize).min(height);
    let _ = NxDialog::new(pixels, width, height, archive)
        .scaled_frame(path, 0, 0, dest_width, dest_height);
}

const GUILD_VISIBLE_ROWS: usize = 10;

fn guild_scrollbar_position(x: f32, y: f32, count: usize, offset: usize) -> Option<usize> {
    if !(306.0..318.0).contains(&x) || !(92.0..332.0).contains(&y) { return None; }
    let maximum = count.saturating_sub(GUILD_VISIBLE_ROWS);
    let next = if y < 104.0 { offset.saturating_sub(1) }
        else if y >= 320.0 { offset.saturating_add(1) }
        else { (((y - 110.0) / 204.0).clamp(0.0, 1.0) * maximum as f32).round() as usize };
    Some(next.min(maximum))
}

#[cfg(test)]
mod guild_roster_layout_tests {
    use super::*;

    #[test]
    fn scrollbar_exposes_every_member_without_entering_the_footer() {
        assert_eq!(guild_scrollbar_position(306.0, 92.0, 12, 0), Some(0));
        assert_eq!(guild_scrollbar_position(306.0, 331.0, 12, 0), Some(1));
        assert_eq!(guild_scrollbar_position(306.0, 314.0, 12, 0), Some(2));
        assert_eq!(guild_scrollbar_position(306.0, 332.0, 12, 0), None);
        assert!(92 + (GUILD_VISIBLE_ROWS - 1) * 24 + 23 < 344);
    }
}

fn render_guild_panel(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    guild: Option<&openstory_protocol::GuildInfo>,
    invite: Option<&(i32, String)>,
    creation_invite: Option<&(String, String)>,
    scroll: usize,
    mode: &GuildPanelMode,
    background_preview: Option<&Bitmap>,
    mark_preview: Option<&Bitmap>,
) -> Vec<u8> {
    let width = GUILD_PANEL_WIDTH as usize;
    let height = GUILD_PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    draw_family_nx_background(
        &mut pixels,
        width,
        height,
        ui_archive,
        "UIWindow.img/UserList/Guild/backgrnd",
        height as f32 / 304.0,
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Guild",
        12,
        7,
        15.0,
        width - 24,
        18,
        [80, 65, 42],
    );
    match mode {
        GuildPanelMode::Create { name, notice } => {
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                "Create a guild",
                14,
                56,
                14.0,
                width - 28,
                20,
                [70, 60, 45],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                "Choose a 3–12 letter name. Nearby players will confirm.",
                14,
                88,
                9.0,
                width - 28,
                28,
                [92, 83, 69],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                if name.is_empty() { "Guild name" } else { name },
                24,
                150,
                12.0,
                width - 48,
                18,
                if name.is_empty() {
                    [120, 112, 98]
                } else {
                    [45, 38, 29]
                },
            );
            if let Some(notice) = notice {
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    width,
                    height,
                    notice,
                    14,
                    195,
                    9.0,
                    width - 28,
                    36,
                    [255, 148, 130],
                );
            }
            draw_guild_button(
                font,
                &mut pixels,
                ui_archive,
                width,
                height,
                (34, 344, 112, 34),
                "Create",
            );
            draw_guild_button(
                font,
                &mut pixels,
                ui_archive,
                width,
                height,
                (190, 344, 112, 34),
                "Cancel",
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                "Enter creates · Esc cancels",
                14,
                390,
                8.0,
                width - 28,
                12,
                [75, 70, 60],
            );
            return pixels;
        }
        GuildPanelMode::RankTitles {
            titles,
            selected,
            notice,
        } => {
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                "Edit guild rank titles",
                14,
                50,
                13.0,
                width - 28,
                18,
                [70, 60, 45],
            );
            for (index, title) in titles.iter().enumerate() {
                let y = 88 + index as i32 * 46;
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    width,
                    height,
                    &format!("Rank {}", index + 1),
                    22,
                    y + 3,
                    8.0,
                    70,
                    12,
                    if index == *selected { [20, 104, 174] } else { [92, 83, 69] },
                );
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    width,
                    height,
                    if title.is_empty() { "(empty)" } else { title },
                    98,
                    y + 9,
                    11.0,
                    width - 124,
                    16,
                    if index == *selected { [20, 104, 174] } else { [45, 38, 29] },
                );
            }
            if let Some(notice) = notice {
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    width,
                    height,
                    notice,
                    14,
                    322,
                    8.0,
                    width - 28,
                    14,
                    [255, 148, 130],
                );
            }
            draw_guild_button(font, &mut pixels, ui_archive, width, height, (34, 350, 112, 34), "Save");
            draw_guild_button(
                font,
                &mut pixels,
                ui_archive,
                width,
                height,
                (190, 350, 112, 34),
                "Cancel",
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                "Click a row or use Tab; type to edit",
                14,
                394,
                8.0,
                width - 28,
                12,
                [75, 70, 60],
            );
            return pixels;
        }
        GuildPanelMode::Emblem {
            emblem,
            selected,
            backgrounds,
            marks,
        } => {
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                "Edit guild emblem",
                14,
                50,
                13.0,
                width - 28,
                18,
                [70, 60, 45],
            );
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                width,
                height,
                ui_archive,
                "UIWindow.img/UserList/Guild/MakeMark/inner",
                177,
                114,
            );
            if let Some(bitmap) = background_preview {
                draw_guild_bitmap(&mut pixels, width, height, 137, 83, 4, bitmap);
            }
            if let Some(bitmap) = mark_preview {
                draw_guild_bitmap(&mut pixels, width, height, 137, 83, 4, bitmap);
            }
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!("{} / {}", emblem.background, emblem.logo),
                150,
                149,
                8.0,
                40,
                12,
                [40, 35, 30],
            );
            let fields = [
                (
                    format!(
                        "Background · {}",
                        backgrounds
                            .iter()
                            .find(|choice| choice.id == emblem.background)
                            .map_or("None", |choice| choice.label.as_str())
                    ),
                    emblem.background.to_string(),
                ),
                (
                    "Background color".to_owned(),
                    emblem.background_color.to_string(),
                ),
                (
                    format!(
                        "Mark · {}",
                        marks
                            .iter()
                            .find(|choice| choice.id == emblem.logo)
                            .map_or("None", |choice| choice.label.as_str())
                    ),
                    emblem.logo.to_string(),
                ),
                ("Mark color".to_owned(), emblem.logo_color.to_string()),
            ];
            for (index, (label, value)) in fields.iter().enumerate() {
                let y = 176 + index as i32 * 38;
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    width,
                    height,
                    label,
                    22,
                    y + 8,
                    9.0,
                    150,
                    14,
                    if index == *selected { [20, 104, 174] } else { [92, 83, 69] },
                );
                draw_guild_button(font, &mut pixels, ui_archive, width, height, (220, y + 3, 28, 24), "−");
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    width,
                    height,
                    value,
                    255,
                    y + 8,
                    10.0,
                    34,
                    14,
                    if index == *selected { [20, 104, 174] } else { [45, 38, 29] },
                );
                draw_guild_button(font, &mut pixels, ui_archive, width, height, (290, y + 3, 28, 24), "+");
            }
            draw_guild_button(font, &mut pixels, ui_archive, width, height, (34, 350, 112, 34), "Save");
            draw_guild_button(
                font,
                &mut pixels,
                ui_archive,
                width,
                height,
                (190, 350, 112, 34),
                "Cancel",
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                "Choose a field and adjust with ←/→",
                14,
                394,
                8.0,
                width - 28,
                12,
                [75, 70, 60],
            );
            return pixels;
        }
        GuildPanelMode::Overview => {}
    }
    if let Some(guild) = guild {
        draw_guild_button(font, &mut pixels, ui_archive, width, height, (128, 5, 80, 21), "Edit");
        draw_guild_button(
            font,
            &mut pixels,
            ui_archive,
            width,
            height,
            (216, 5, 102, 21),
            "Emblem",
        );
        let online = guild.members.iter().filter(|member| member.online).count();
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!(
                "{}  ·  {online}/{} online  ·  GP {}",
                guild.name,
                guild.members.len(),
                guild.gp
            ),
            12,
            28,
            10.0,
            width - 24,
            15,
            [48, 48, 48],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!(
                "Capacity: {}/{}",
                guild.members.len(),
                guild.capacity.max(0)
            ),
            12,
            46,
            10.0,
            width - 24,
            15,
            [72, 68, 60],
        );
        if !guild.notice.trim().is_empty() {
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!("Notice: {}", guild.notice),
                12,
                64,
                10.0,
                width - 24,
                28,
                [128, 83, 0],
            );
        }
        let rows = GUILD_VISIBLE_ROWS;
        let max_scroll = guild.members.len().saturating_sub(rows);
        if max_scroll > 0 {
            for (path, y) in [("Basic.img/VScr/enabled/prev0", 92), ("Basic.img/VScr/enabled/next0", 320)] {
                let _ = composite_nx_ui_bitmap(&mut pixels, width, height, ui_archive, path, 306, y);
            }
            let thumb_y = 104 + 204 * scroll.min(max_scroll) / max_scroll;
            let _ = composite_nx_ui_bitmap(&mut pixels, width, height, ui_archive, "Basic.img/VScr/enabled/thumb0", 306, thumb_y as i32);
        }
        for (row, member) in guild
            .members
            .iter()
            .skip(scroll.min(max_scroll))
            .take(rows)
            .enumerate()
        {
            let y = 92 + row as i32 * 24;
            let rank_index = member.rank.saturating_sub(1).clamp(0, 4) as usize;
            let rank = guild.rank_titles[rank_index].as_str();
            let status = if member.online { "Online" } else { "Offline" };
            let color = if member.online {
                [38, 70, 38]
            } else {
                [90, 90, 90]
            };
            let member_line = ui_foundation::ellipsize(font, &format!("{}  Lv.{}  Job {}", member.name, member.level, member.job), 10.0, (width - 40) as f32);
            let rank_line = ui_foundation::ellipsize(font, &format!("{rank}  ·  {status}"), 8.0, (width - 50) as f32);
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &member_line,
                12,
                y,
                10.0,
                width - 40,
                14,
                color,
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &rank_line,
                24,
                y + 11,
                8.0,
                width - 50,
                12,
                if member.online {
                    [55, 100, 55]
                } else {
                    [100, 100, 100]
                },
            );
        }
        if guild.members.is_empty() {
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                "No guild members were returned.",
                12,
                100,
                11.0,
                width - 24,
                18,
                [60, 60, 60],
            );
        }
    } else if let Some((leader, guild_name)) = creation_invite {
        let _ = composite_nx_ui_bitmap_scaled_to(
            &mut pixels,
            width,
            height,
            ui_archive,
            "UIWindow.img/UtilDlgEx/line",
            10,
            height - 52,
            width.saturating_sub(20),
            2,
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("{leader} wants to form {guild_name} · Y agree / N decline"),
            12,
            height as i32 - 42,
            8.0,
            width - 24,
            14,
            [128, 83, 0],
        );
    } else {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "You are not in a guild.",
            12,
            48,
            12.0,
            width - 24,
            20,
            [60, 60, 60],
        );
        draw_guild_button(
            font,
            &mut pixels,
            ui_archive,
            width,
            height,
            (90, 104, 156, 34),
            "Create Guild",
        );
    }
    if let Some((_, inviter)) = invite {
        let _ = composite_nx_ui_bitmap_scaled_to(
            &mut pixels,
            width,
            height,
            ui_archive,
            "UIWindow.img/UtilDlgEx/line",
            10,
            height - 52,
            width.saturating_sub(20),
            2,
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("{inviter} invited you · Y accept / N decline"),
            12,
            height as i32 - 42,
            9.0,
            width - 24,
            14,
            [128, 83, 0],
        );
    } else {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "G · /guild create, invite, leave, expel, rank, notice",
            12,
            height as i32 - 22,
            9.0,
            width - 24,
            13,
            [80, 74, 62],
        );
    }
    pixels
}

fn draw_guild_button(
    font: &fontdue::Font,
    pixels: &mut [u8],
    ui_archive: &NxArchive,
    width: usize,
    height: usize,
    rect: (i32, i32, usize, usize),
    label: &str,
) {
    let (x, y, button_width, button_height) = rect;
    let path = match label {
        "Create" => Some("UIWindow.img/UserList/Guild/Make/BtAgree/normal/0"),
        "Save" => Some("UIWindow.img/UserList/Guild/GuildEdit/BtSave/normal/0"),
        "Cancel" => Some("UIWindow.img/UserList/Guild/GuildBoard/BtCancel/normal/0"),
        "Edit" => Some("UIWindow.img/UserList/Guild/GuildGrade/BtModf/normal/0"),
        "−" => Some("UIWindow.img/UserList/Guild/GuildRank/BtLeft/normal/0"),
        "+" => Some("UIWindow.img/UserList/Guild/GuildRank/BtRight/normal/0"),
        _ => None,
    };
    if let Some(path) = path {
        let bounds = ui_foundation::UiRect::new(
            x as f32, y as f32, button_width as f32, button_height as f32,
        );
        let _ = NxDialog::new(pixels, width, height, ui_archive).buttons(&[(path, bounds)]);
        return;
    }
    let text_width = dialogue_text_width(font, label, 9.0);
    draw_dialogue_text(
        font,
        pixels,
        width,
        height,
        label,
        x + ((button_width as f32 - text_width) / 2.0).max(3.0) as i32,
        y + ((button_height as i32 - 14) / 2).max(1),
        9.0,
        button_width.saturating_sub(8),
        14,
        [45, 38, 29],
    );
}

fn draw_guild_bitmap(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    x: usize,
    y: usize,
    scale: usize,
    bitmap: &Bitmap,
) {
    let Ok(source) = bitmap.decode_bgra_shared() else {
        return;
    };
    let source_width = usize::from(bitmap.width());
    let source_height = usize::from(bitmap.height());
    for source_y in 0..source_height {
        for source_x in 0..source_width {
            let source_index = (source_y * source_width + source_x) * 4;
            let Some(source_pixel) = source.get(source_index..source_index + 4) else {
                continue;
            };
            let alpha = u16::from(source_pixel[3]);
            if alpha == 0 {
                continue;
            }
            for dy in 0..scale {
                for dx in 0..scale {
                    let dest_x = x + source_x * scale + dx;
                    let dest_y = y + source_y * scale + dy;
                    if dest_x >= canvas_width || dest_y >= canvas_height {
                        continue;
                    }
                    let dest_index = (dest_y * canvas_width + dest_x) * 4;
                    let inverse = 255 - alpha;
                    for channel in 0..3 {
                        pixels[dest_index + channel] = ((u16::from(source_pixel[channel]) * alpha
                            + u16::from(pixels[dest_index + channel]) * inverse)
                            / 255) as u8;
                    }
                    pixels[dest_index + 3] =
                        (alpha + u16::from(pixels[dest_index + 3]) * inverse / 255).min(255) as u8;
                }
            }
        }
    }
}

const ALLIANCE_VISIBLE_ROWS: usize = 8;

fn alliance_scrollbar_position(x: f32, y: f32, count: usize, offset: usize) -> Option<usize> {
    if !(279.0..291.0).contains(&x) || !(116.0..308.0).contains(&y) { return None; }
    let maximum = count.saturating_sub(ALLIANCE_VISIBLE_ROWS);
    let next = if y < 128.0 { offset.saturating_sub(1) }
        else if y >= 296.0 { offset.saturating_add(1) }
        else { (((y - 134.0) / 156.0).clamp(0.0, 1.0) * maximum as f32).round() as usize };
    Some(next.min(maximum))
}

#[cfg(test)]
mod alliance_roster_layout_tests {
    use super::*;

    #[test]
    fn scrollbar_reaches_later_guilds_and_clamps_at_both_ends() {
        assert_eq!(alliance_scrollbar_position(279.0, 116.0, 12, 0), Some(0));
        assert_eq!(alliance_scrollbar_position(279.0, 307.0, 12, 0), Some(1));
        assert_eq!(alliance_scrollbar_position(279.0, 290.0, 12, 0), Some(4));
        assert_eq!(alliance_scrollbar_position(279.0, 307.0, 3, 0), Some(0));
        assert_eq!(alliance_scrollbar_position(278.0, 150.0, 12, 0), None);
    }
}

fn render_alliance_panel(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    guild: Option<&openstory_protocol::GuildInfo>,
    alliance: Option<&openstory_protocol::AllianceInfo>,
    invite: Option<&(i32, String)>,
    scroll: usize,
) -> Vec<u8> {
    let width = ALLIANCE_PANEL_WIDTH as usize;
    let height = ALLIANCE_PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    draw_family_nx_background(
        &mut pixels,
        width,
        height,
        ui_archive,
        "UIWindow.img/UserList/GuildUnion/backgrnd",
        height as f32 / 304.0,
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Alliance",
        14,
        9,
        16.0,
        width - 58,
        20,
        [80, 65, 42],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "U / Esc: close",
        width as i32 - 126,
        13,
        9.0,
        108,
        14,
        [80, 74, 62],
    );
    if let Some(alliance) = alliance {
        let title = if alliance.name.starts_with("Alliance ") {
            format!("{}  ·  ID {}", alliance.name, alliance.id)
        } else {
            format!(
                "{}  ·  {}/{} guilds",
                alliance.name,
                alliance.guilds.len(),
                alliance.capacity.max(0)
            )
        };
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &title,
            14,
            36,
            11.0,
            width - 28,
            16,
            [50, 50, 50],
        );
        if !alliance.notice.trim().is_empty() {
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!("Notice: {}", alliance.notice),
                14,
                58,
                10.0,
                width - 28,
                30,
                [128, 83, 0],
            );
        }
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Alliance guilds",
            14,
            96,
            10.0,
            width - 28,
            14,
            [72, 68, 60],
        );
        let maximum = alliance.guilds.len().saturating_sub(ALLIANCE_VISIBLE_ROWS);
        let scroll = scroll.min(maximum);
        if maximum > 0 {
            for (path, y) in [("Basic.img/VScr/enabled/prev0", 116), ("Basic.img/VScr/enabled/next0", 296)] {
                let _ = composite_nx_ui_bitmap(&mut pixels, width, height, ui_archive, path, 279, y);
            }
            let thumb_y = 128 + 156 * scroll / maximum;
            let _ = composite_nx_ui_bitmap(&mut pixels, width, height, ui_archive, "Basic.img/VScr/enabled/thumb0", 279, thumb_y as i32);
        }
        for (row, member_guild) in alliance.guilds.iter().skip(scroll).take(ALLIANCE_VISIBLE_ROWS).enumerate() {
            let local = guild.is_some_and(|guild| guild.id == member_guild.id);
            let name = if member_guild.name.is_empty() {
                if local {
                    guild.map_or_else(
                        || format!("Guild {}", member_guild.id),
                        |guild| guild.name.clone(),
                    )
                } else {
                    format!("Guild {}", member_guild.id)
                }
            } else {
                member_guild.name.clone()
            };
            let name = ui_foundation::ellipsize(font, &format!("{}{}", if local { "• " } else { "  " }, name), 11.0, (width - 54) as f32);
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &name,
                18,
                116 + row as i32 * 24,
                11.0,
                width - 54,
                18,
                if local { [128, 83, 0] } else { [52, 52, 52] },
            );
        }
        if alliance.guilds.is_empty() {
            let local_name = guild
                .filter(|guild| guild.alliance_id == alliance.id)
                .map_or("No guild roster has been sent yet.", |guild| {
                    guild.name.as_str()
                });
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                local_name,
                18,
                118,
                11.0,
                width - 36,
                18,
                [52, 52, 52],
            );
        }
    } else if let Some(guild) = guild.filter(|guild| guild.alliance_id > 0) {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("{} belongs to alliance {}.", guild.name, guild.alliance_id),
            14,
            42,
            11.0,
            width - 28,
            18,
            [52, 52, 52],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Waiting for the server's alliance roster...",
            14,
            66,
            10.0,
            width - 28,
            18,
            [80, 74, 62],
        );
    } else {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Your guild is not in an alliance.",
            14,
            42,
            11.0,
            width - 28,
            18,
            [52, 52, 52],
        );
    }
    if let Some((_, inviter)) = invite {
        let _ = composite_nx_ui_bitmap_scaled_to(
            &mut pixels,
            width,
            height,
            ui_archive,
            "UIWindow.img/UtilDlgEx/line",
            12,
            height - 76,
            width - 24,
            2,
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("{inviter} invited your guild · Y accept / N decline"),
            14,
            height as i32 - 64,
            10.0,
            width - 28,
            16,
            [128, 83, 0],
        );
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Commands: invite · expel · leave · notice · ranks · list",
        14,
        height as i32 - 34,
        9.0,
        width - 28,
        14,
        [80, 74, 62],
    );
    pixels
}

const GUILD_BBS_CLOSE: ui_foundation::UiRect =
    ui_dialog::inventory_close(GUILD_BBS_PANEL_WIDTH as f32);

const GUILD_BBS_DRAFT_FRAME: ui_foundation::UiRect =
    ui_foundation::UiRect::new(192.0, 124.0, 440.0, 350.0);
const GUILD_BBS_DRAFT_CLOSE: ui_foundation::UiRect = ui_dialog::close_button(
    GUILD_BBS_DRAFT_FRAME.x + GUILD_BBS_DRAFT_FRAME.width - 19.0,
    GUILD_BBS_DRAFT_FRAME.y + 6.0,
);
const GUILD_BBS_DRAFT_SUBJECT: ui_foundation::UiRect =
    ui_foundation::UiRect::new(224.0, 192.0, 376.0, 30.0);
const GUILD_BBS_DRAFT_CONTENT: ui_foundation::UiRect =
    ui_foundation::UiRect::new(224.0, 246.0, 376.0, 162.0);
const GUILD_BBS_DRAFT_BUTTONS: [ui_foundation::UiRect; 2] = ui_dialog::right_aligned_footer(
    GUILD_BBS_DRAFT_FRAME.x + GUILD_BBS_DRAFT_FRAME.width - 8.0,
    GUILD_BBS_DRAFT_FRAME.y + GUILD_BBS_DRAFT_FRAME.height,
    [40.0, 40.0],
);

fn render_guild_bbs_panel(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    bbs: &GuildBbsUi,
    guild: Option<&openstory_protocol::GuildInfo>,
) -> Vec<u8> {
    let width = GUILD_BBS_PANEL_WIDTH as usize;
    let height = GUILD_BBS_PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    let _ = NxDialog::new(&mut pixels, width, height, ui_archive).inventory_window(
        ui_foundation::UiRect::new(0.0, 0.0, width as f32, height as f32),
        font,
        "Guild Board",
        GUILD_BBS_CLOSE,
    );
    for (x, button_width, path) in [
        (604, 54, "UIWindow.img/UserList/Guild/GuildBoard/BtList/normal/0"),
        (660, 44, "UIWindow.img/UserList/Guild/GuildRank/BtLeft/normal/0"),
        (706, 44, "UIWindow.img/UserList/Guild/GuildRank/BtRight/normal/0"),
    ] {
        let _ = composite_nx_ui_bitmap_centered(
            &mut pixels,
            width,
            height,
            ui_archive,
            path,
            ui_foundation::UiRect::new(x as f32, 46.0, button_width as f32, 30.0),
        );
    }

    let page_count = bbs.total_threads.div_ceil(10);
    let page_label = if page_count == 0 {
        "Page 0 / 0".to_owned()
    } else {
        format!("Page {} / {page_count}", bbs.page.saturating_add(1))
    };
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        &format!("{} posts · {page_label}", bbs.total_threads),
        12,
        76,
        10.0,
        360,
        18,
        [92, 83, 69],
    );

    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Threads",
        18,
        88,
        10.0,
        340,
        16,
        [70, 60, 45],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Thread details",
        384,
        88,
        10.0,
        420,
        16,
        [70, 60, 45],
    );

    let mut rows: Vec<(&openstory_protocol::BbsThread, bool)> = Vec::new();
    if let Some(notice) = bbs.notice.as_ref() {
        rows.push((notice, true));
    }
    rows.extend(bbs.threads.iter().map(|thread| (thread, false)));
    if rows.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "No guild posts on this page.",
            20,
            120,
            10.0,
            330,
            32,
            [92, 83, 69],
        );
    }
    for (index, (thread, is_notice)) in rows.iter().take(11).enumerate() {
        let y = 108 + index as i32 * 30;
        let selected = bbs.thread.as_ref().is_some_and(|view| view.id == thread.id);
        let author = guild
            .and_then(|guild| {
                guild
                    .members
                    .iter()
                    .find(|member| member.character_id == thread.poster_id)
            })
            .map_or_else(
                || format!("#{}", thread.poster_id),
                |member| member.name.clone(),
            );
        let title = if *is_notice {
            format!("[Notice] {}", thread.title)
        } else {
            thread.title.clone()
        };
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &title,
            20,
            y,
            10.0,
            336,
            14,
            if selected { [20, 104, 174] } else if *is_notice { [128, 83, 0] } else { [45, 38, 29] },
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("{author}  ·  {} replies", thread.reply_count.max(0)),
            20,
            y + 12,
            8.0,
            336,
            12,
            [92, 83, 69],
        );
    }

    if let Some(thread) = bbs.thread.as_ref() {
        let author = guild
            .and_then(|guild| {
                guild
                    .members
                    .iter()
                    .find(|member| member.character_id == thread.poster_id)
            })
            .map_or_else(
                || format!("#{}", thread.poster_id),
                |member| member.name.clone(),
            );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &thread.title,
            388,
            110,
            13.0,
            416,
            34,
            [45, 38, 29],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("Posted by {author}  ·  {} replies", thread.replies.len()),
            388,
            146,
            9.0,
            416,
            18,
            [92, 83, 69],
        );
        let _ = composite_nx_ui_bitmap_scaled_to(
            &mut pixels,
            width,
            height,
            ui_archive,
            "UIWindow.img/UtilDlgEx/line",
            386,
            170,
            width - 398,
            2,
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &thread.content,
            388,
            180,
            10.0,
            416,
            176,
            [45, 38, 29],
        );
        if !thread.replies.is_empty() {
            let _ = composite_nx_ui_bitmap_scaled_to(
                &mut pixels,
                width,
                height,
                ui_archive,
                "UIWindow.img/UtilDlgEx/line",
                386,
                310,
                width - 398,
                2,
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!("Replies ({})", thread.replies.len()),
                388,
                318,
                10.0,
                416,
                16,
                [70, 60, 45],
            );
            let start = thread.replies.len().saturating_sub(4);
            for (index, reply) in thread.replies[start..].iter().enumerate() {
                let y = 338 + index as i32 * 25;
                let reply_author = guild
                    .and_then(|guild| {
                        guild
                            .members
                            .iter()
                            .find(|member| member.character_id == reply.poster_id)
                    })
                    .map_or_else(
                        || format!("#{}", reply.poster_id),
                        |member| member.name.clone(),
                    );
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    width,
                    height,
                    &format!("{reply_author}: {}", reply.text),
                    388,
                    y,
                    9.0,
                    416,
                    22,
                    [80, 74, 62],
                );
            }
        }

        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            if bbs.reply_input.is_empty() && !bbs.reply_focused {
                "Write a reply (max 25 chars)"
            } else {
                &bbs.reply_input
            },
            394,
            453,
            9.0,
            338,
            24,
            if bbs.reply_input.is_empty() && !bbs.reply_focused {
                [120, 112, 98]
            } else {
                [45, 38, 29]
            },
        );
        let _ = composite_nx_ui_bitmap_centered(
            &mut pixels,
            width,
            height,
            ui_archive,
            "UIWindow.img/UserList/Guild/GuildBoard/BtComment/normal/0",
            ui_foundation::UiRect::new(744.0, 450.0, 68.0, 42.0),
        );
    } else {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Select a thread to read it.",
            388,
            120,
            11.0,
            416,
            24,
            [92, 83, 69],
        );
    }

    for (x, button_width, path) in [
        (14, 76, "UIWindow.img/UserList/Guild/GuildBoard/BtWrite/normal/0"),
        (98, 86, "UIWindow.img/UserList/Guild/GuildBoard/BtComment/normal/0"),
        (192, 86, "UIWindow.img/UserList/Guild/GuildBoard/BtDelete/normal/0"),
    ] {
        let _ = composite_nx_ui_bitmap_centered(
            &mut pixels,
            width,
            height,
            ui_archive,
            path,
            ui_foundation::UiRect::new(x as f32, 462.0, button_width as f32, 36.0),
        );
    }
    if bbs.thread.is_none() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Open a thread to reply.",
            398,
            463,
            9.0,
            402,
            20,
            [120, 112, 98],
        );
    }
    if let Some(feedback) = bbs.feedback.as_deref() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            feedback,
            294,
            500,
            8.0,
            width - 306,
            14,
            [150, 35, 35],
        );
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "V opens the guild board · Esc closes · ← / → change pages",
        12,
        520,
        9.0,
        width - 24,
        15,
        [92, 83, 69],
    );

    if let Some(draft) = bbs.draft.as_ref() {
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            0,
            0,
            width,
            height,
            [0, 0, 0, 176],
        );
        use ui_dialog::{ButtonState, NxButton};
        use ui_foundation::{UiTextRole, draw_text_role};

        let title = if draft.is_notice {
            "Write Guild Notice"
        } else {
            "Write Guild Post"
        };
        let mut dialog = NxDialog::new(&mut pixels, width, height, ui_archive);
        let _ = dialog.inventory_window(GUILD_BBS_DRAFT_FRAME, font, title, GUILD_BBS_DRAFT_CLOSE);
        let [cancel, save] = GUILD_BBS_DRAFT_BUTTONS;
        let _ = dialog.button(
            cancel,
            NxButton::new(
                "UIWindow.img/UserList/Guild/GuildBoard/BtCancel",
                ButtonState::Normal,
            ),
        );
        let _ = dialog.button(
            save,
            NxButton::new(
                "UIWindow.img/UserList/Guild/GuildBoard/BtSave",
                ButtonState::Normal,
            ),
        );
        for (field, focus, label, value, limit) in [
            (
                GUILD_BBS_DRAFT_SUBJECT,
                GuildBbsDraftFocus::Title,
                "Subject",
                &draft.title,
                GUILD_BBS_TITLE_LIMIT,
            ),
            (
                GUILD_BBS_DRAFT_CONTENT,
                GuildBbsDraftFocus::Content,
                "Content",
                &draft.content,
                GUILD_BBS_CONTENT_LIMIT,
            ),
        ] {
            let (x, y, w, h) = (
                field.x as usize,
                field.y as usize,
                field.width as usize,
                field.height as usize,
            );
            let border = if draft.focus == focus {
                [178, 137, 85, 255]
            } else {
                [204, 195, 181, 255]
            };
            fill_dialogue_rect(&mut pixels, width, height, x, y, w, h, border);
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                x + 1,
                y + 1,
                w - 2,
                h - 2,
                [255, 255, 255, 255],
            );
            draw_text_role(
                font,
                &mut pixels,
                width,
                height,
                &format!("{label} · {}/{limit}", value.encode_utf16().count()),
                x as i32,
                y as i32 - 17,
                UiTextRole::Label,
                w,
                14,
                None,
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                value,
                x as i32 + 7,
                y as i32 + 6,
                UiTextRole::Value.style().size,
                w - 14,
                h - 12,
                UiTextRole::Value.style().color,
            );
        }
        if let Some(feedback) = bbs.feedback.as_deref() {
            draw_text_role(
                font,
                &mut pixels,
                width,
                height,
                feedback,
                224,
                418,
                UiTextRole::Value,
                376,
                14,
                Some([150, 35, 35]),
            );
        }
    } else if bbs.confirm_delete {
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            0,
            0,
            width,
            height,
            [0, 0, 0, 176],
        );
        let _ = NxDialog::new(&mut pixels, width, height, ui_archive).frame(
            "Basic.img/YesNo/backgrnd",
            260,
            220,
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Delete this guild post?",
            282,
            242,
            12.0,
            260,
            20,
            [45, 38, 29],
        );
        for (x, path) in [
            (320, "Basic.img/BtNo/normal/0"),
            (420, "Basic.img/BtYes/normal/0"),
        ] {
            let _ = composite_nx_ui_bitmap_centered(
                &mut pixels,
                width,
                height,
                ui_archive,
                path,
                ui_foundation::UiRect::new(x as f32, 300.0, 80.0, 34.0),
            );
        }
    }
    pixels
}
