fn render_family_panel(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    state: &FamilyUiState,
) -> Vec<u8> {
    let width = FAMILY_PANEL_WIDTH as usize;
    let height = FAMILY_PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        height,
        [18, 15, 12, 242],
    );
    draw_family_nx_background(
        &mut pixels,
        width,
        height,
        ui_archive,
        "UIWindow2.img/Family/backgrnd",
        1.28,
    );

    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Family",
        15,
        11,
        15.0,
        width - 52,
        20,
        [255, 235, 191],
    );
    draw_guild_button(font, &mut pixels, width, height, (242, 5, 24, 24), "X");

    if let Some(info) = state.info.as_ref() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            if info.name.trim().is_empty() {
                "No family"
            } else {
                info.name.as_str()
            },
            15,
            49,
            13.0,
            width - 30,
            20,
            [255, 226, 145],
        );
        for (index, (label, value)) in [
            ("Reputation", info.current_reputation),
            ("Total", info.total_reputation),
            ("Today", info.todays_reputation),
        ]
        .into_iter()
        .enumerate()
        {
            let y = 82 + index as i32 * 23;
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                label,
                18,
                y,
                9.0,
                94,
                15,
                [190, 181, 157],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &value.to_string(),
                122,
                y,
                9.0,
                130,
                15,
                [248, 242, 221],
            );
        }
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!(
                "Juniors: {} / {}",
                info.junior_count.max(0),
                info.juniors_allowed.max(0)
            ),
            18,
            158,
            9.0,
            width - 36,
            16,
            [214, 205, 181],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            14,
            185,
            width - 28,
            52,
            [30, 25, 20, 228],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Family motto · click to edit",
            21,
            187,
            7.0,
            width - 42,
            12,
            [178, 167, 143],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            if info.message.trim().is_empty() {
                "No motto set"
            } else {
                info.message.as_str()
            },
            21,
            201,
            9.0,
            width - 42,
            32,
            [229, 220, 197],
        );
    } else {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Requesting family information…",
            18,
            64,
            10.0,
            width - 36,
            30,
            [220, 211, 188],
        );
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
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        14,
        260,
        width - 28,
        91,
        [28, 24, 19, 232],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Family ability",
        20,
        270,
        8.0,
        100,
        14,
        [178, 167, 143],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        ability_name,
        20,
        287,
        11.0,
        width - 40,
        16,
        [255, 225, 144],
    );
    draw_guild_button(font, &mut pixels, width, height, (196, 299, 29, 25), "<");
    draw_guild_button(font, &mut pixels, width, height, (236, 299, 29, 25), ">");
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        ability_description,
        20,
        313,
        8.0,
        width - 40,
        20,
        [204, 195, 174],
    );
    if let Some(privilege) = privilege {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!(
                "Cost: {} rep  ·  Uses: {}",
                privilege.reputation_cost, privilege.usage_limit
            ),
            20,
            334,
            8.0,
            width - 40,
            14,
            [196, 185, 159],
        );
    }
    if let Some(notice) = state.notice.as_deref() {
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
            [255, 157, 132],
        );
    }
    draw_guild_button(font, &mut pixels, width, height, (12, 390, 60, 32), "Tree");
    draw_guild_button(font, &mut pixels, width, height, (76, 390, 50, 32), "Use");
    draw_guild_button(
        font,
        &mut pixels,
        width,
        height,
        (130, 390, 58, 32),
        "Invite",
    );
    draw_guild_button(
        font,
        &mut pixels,
        width,
        height,
        (192, 390, 68, 32),
        "Leave",
    );

    if let Some(input) = state.input.as_ref() {
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            0,
            0,
            width,
            height,
            [0, 0, 0, 150],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            24,
            124,
            226,
            166,
            [27, 23, 19, 255],
        );
        for (x, y, w, h) in [
            (24, 124, 226, 2),
            (24, 288, 226, 2),
            (24, 124, 2, 166),
            (248, 124, 2, 166),
        ] {
            fill_dialogue_rect(&mut pixels, width, height, x, y, w, h, [147, 121, 78, 255]);
        }
        let (heading, helper) = match input.kind {
            FamilyInputKind::InviteJunior => ("Invite a junior", "Enter a name (max 13)"),
            FamilyInputKind::EditPrecepts => ("Edit family motto", "Enter a motto (max 200)"),
        };
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            heading,
            38,
            140,
            12.0,
            198,
            18,
            [255, 229, 169],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            helper,
            38,
            164,
            8.0,
            198,
            14,
            [194, 183, 159],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            38,
            190,
            198,
            35,
            [12, 10, 8, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &input.text,
            45,
            199,
            10.0,
            184,
            20,
            [245, 236, 215],
        );
        draw_guild_button(font, &mut pixels, width, height, (53, 253, 79, 28), "OK");
        draw_guild_button(
            font,
            &mut pixels,
            width,
            height,
            (143, 253, 79, 28),
            "Cancel",
        );
    } else if state.confirm_leave {
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            0,
            0,
            width,
            height,
            [0, 0, 0, 150],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            24,
            124,
            226,
            166,
            [27, 23, 19, 255],
        );
        for (x, y, w, h) in [
            (24, 124, 226, 2),
            (24, 288, 226, 2),
            (24, 124, 2, 166),
            (248, 124, 2, 166),
        ] {
            fill_dialogue_rect(&mut pixels, width, height, x, y, w, h, [147, 121, 78, 255]);
        }
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Leave your family?",
            38,
            162,
            12.0,
            198,
            20,
            [255, 229, 169],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Your family relationship will end.",
            38,
            188,
            8.0,
            198,
            24,
            [210, 198, 175],
        );
        draw_guild_button(font, &mut pixels, width, height, (53, 253, 79, 28), "Leave");
        draw_guild_button(
            font,
            &mut pixels,
            width,
            height,
            (143, 253, 79, 28),
            "Cancel",
        );
    }
    pixels
}

fn render_family_tree_panel(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    state: &FamilyUiState,
) -> Vec<u8> {
    let width = FAMILY_TREE_PANEL_WIDTH as usize;
    let height = FAMILY_TREE_PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        height,
        [17, 15, 13, 243],
    );
    draw_family_nx_background(
        &mut pixels,
        width,
        height,
        ui_archive,
        "UIWindow2.img/FamilyTree/backgrnd",
        1.28,
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Family tree",
        18,
        11,
        15.0,
        width - 66,
        20,
        [255, 235, 191],
    );
    draw_guild_button(font, &mut pixels, width, height, (542, 5, 28, 25), "X");
    if let Some(chart) = state.chart.as_ref() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("{} family members", chart.entries.len()),
            18,
            29,
            9.0,
            width - 36,
            15,
            [197, 188, 167],
        );
        for (row, entry) in chart.entries.iter().take(10).enumerate() {
            let y = 44 + row as i32 * 30;
            let selected = state.selected_member == Some(entry.character_id);
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                12,
                y as usize,
                width - 24,
                27,
                if selected {
                    [78, 61, 37, 242]
                } else {
                    [29, 26, 22, 225]
                },
            );
            let indent = if entry.parent_id == 0 { 0 } else { 16 };
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!(
                    "{}{}",
                    if entry.parent_id == 0 { "★ " } else { "↳ " },
                    entry.name
                ),
                20 + indent,
                y + 4,
                9.0,
                248,
                15,
                if entry.online {
                    [255, 232, 173]
                } else {
                    [203, 194, 173]
                },
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!(
                    "Lv. {}  Job {}  Rep {}  {}",
                    entry.level,
                    entry.job,
                    entry.current_reputation,
                    if entry.online { "online" } else { "offline" }
                ),
                292,
                y + 4,
                8.0,
                width - 312,
                15,
                [205, 197, 179],
            );
        }
    } else {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Family pedigree data has not arrived yet.",
            18,
            56,
            10.0,
            width - 36,
            26,
            [221, 211, 188],
        );
    }
    pixels
}

fn draw_family_nx_background(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    archive: &NxArchive,
    path: &str,
    scale: f32,
) {
    let bitmap = archive
        .root()
        .resolve(path)
        .ok()
        .flatten()
        .and_then(|node| node.bitmap().ok().flatten());
    let Some(bitmap) = bitmap else {
        for (x, y, w, h) in [
            (0, 0, width, 2),
            (0, height - 2, width, 2),
            (0, 0, 2, height),
            (width - 2, 0, 2, height),
        ] {
            fill_dialogue_rect(pixels, width, height, x, y, w, h, [130, 105, 68, 255]);
        }
        return;
    };
    let Ok(source) = bitmap.decode_bgra_shared() else {
        return;
    };
    let source_width = usize::from(bitmap.width());
    let source_height = usize::from(bitmap.height());
    let dest_width = ((source_width as f32 * scale).round() as usize).min(width);
    let dest_height = ((source_height as f32 * scale).round() as usize).min(height);
    for dest_y in 0..dest_height {
        let source_y = ((dest_y as f32 / scale).floor() as usize).min(source_height - 1);
        for dest_x in 0..dest_width {
            let source_x = ((dest_x as f32 / scale).floor() as usize).min(source_width - 1);
            let source_index = (source_y * source_width + source_x) * 4;
            let alpha = u16::from(source[source_index + 3]);
            if alpha == 0 {
                continue;
            }
            let dest_index = (dest_y * width + dest_x) * 4;
            let inverse = 255 - alpha;
            for channel in 0..3 {
                pixels[dest_index + channel] = ((u16::from(source[source_index + channel]) * alpha
                    + u16::from(pixels[dest_index + channel]) * inverse)
                    / 255) as u8;
            }
            pixels[dest_index + 3] =
                (alpha + u16::from(pixels[dest_index + 3]) * inverse / 255).min(255) as u8;
        }
    }
}

fn render_guild_panel(
    font: &fontdue::Font,
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
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        height,
        [16, 14, 12, 225],
    );
    for (x, y, w, h) in [
        (0, 0, width, 2),
        (0, height - 2, width, 2),
        (0, 0, 2, height),
        (width - 2, 0, 2, height),
    ] {
        fill_dialogue_rect(&mut pixels, width, height, x, y, w, h, [112, 96, 72, 255]);
    }
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
        [226, 205, 158],
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
                [235, 224, 197],
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
                [180, 176, 164],
            );
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                14,
                140,
                width - 28,
                38,
                [42, 38, 32, 255],
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
                    [135, 130, 120]
                } else {
                    [245, 236, 215]
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
                width,
                height,
                (34, 344, 112, 34),
                "Create",
            );
            draw_guild_button(
                font,
                &mut pixels,
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
                [155, 151, 140],
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
                [235, 224, 197],
            );
            for (index, title) in titles.iter().enumerate() {
                let y = 88 + index as i32 * 46;
                fill_dialogue_rect(
                    &mut pixels,
                    width,
                    height,
                    14,
                    y as usize,
                    width - 28,
                    36,
                    if index == *selected {
                        [64, 52, 37, 255]
                    } else {
                        [35, 32, 28, 255]
                    },
                );
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
                    [180, 172, 152],
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
                    [245, 236, 215],
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
            draw_guild_button(font, &mut pixels, width, height, (34, 350, 112, 34), "Save");
            draw_guild_button(
                font,
                &mut pixels,
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
                [155, 151, 140],
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
                [235, 224, 197],
            );
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                132,
                78,
                72,
                72,
                [12, 10, 8, 255],
            );
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                137,
                83,
                62,
                62,
                [36, 32, 27, 255],
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
                fill_dialogue_rect(
                    &mut pixels,
                    width,
                    height,
                    14,
                    y as usize,
                    width - 28,
                    30,
                    if index == *selected {
                        [64, 52, 37, 255]
                    } else {
                        [35, 32, 28, 255]
                    },
                );
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
                    [210, 202, 184],
                );
                draw_guild_button(font, &mut pixels, width, height, (220, y + 3, 28, 24), "−");
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
                    [245, 236, 215],
                );
                draw_guild_button(font, &mut pixels, width, height, (290, y + 3, 28, 24), "+");
            }
            draw_guild_button(font, &mut pixels, width, height, (34, 350, 112, 34), "Save");
            draw_guild_button(
                font,
                &mut pixels,
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
                [155, 151, 140],
            );
            return pixels;
        }
        GuildPanelMode::Overview => {}
    }
    if let Some(guild) = guild {
        draw_guild_button(font, &mut pixels, width, height, (128, 5, 80, 21), "Ranks");
        draw_guild_button(
            font,
            &mut pixels,
            width,
            height,
            (216, 5, 108, 21),
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
            [210, 210, 210],
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
            [190, 190, 190],
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
                [255, 222, 112],
            );
        }
        let rows = if invite.is_some() || creation_invite.is_some() {
            10
        } else {
            12
        };
        let max_scroll = guild.members.len().saturating_sub(rows);
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
                [225, 240, 215]
            } else {
                [155, 155, 155]
            };
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!("{}  Lv.{}  Job {}", member.name, member.level, member.job),
                12,
                y,
                10.0,
                width - 24,
                14,
                color,
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!("{rank}  ·  {status}"),
                24,
                y + 11,
                8.0,
                width - 36,
                12,
                if member.online {
                    [145, 205, 145]
                } else {
                    [145, 145, 145]
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
                [210, 210, 210],
            );
        }
    } else if let Some((leader, guild_name)) = creation_invite {
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            10,
            height - 52,
            width - 20,
            1,
            [84, 76, 62, 255],
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
            [255, 222, 112],
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
            [210, 210, 210],
        );
        draw_guild_button(
            font,
            &mut pixels,
            width,
            height,
            (90, 104, 156, 34),
            "Create Guild",
        );
    }
    if let Some((_, inviter)) = invite {
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            10,
            height - 52,
            width - 20,
            1,
            [84, 76, 62, 255],
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
            [255, 222, 112],
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
            [170, 164, 148],
        );
    }
    pixels
}

fn draw_guild_button(
    font: &fontdue::Font,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    rect: (i32, i32, usize, usize),
    label: &str,
) {
    let (x, y, button_width, button_height) = rect;
    fill_dialogue_rect(
        pixels,
        width,
        height,
        x as usize,
        y as usize,
        button_width,
        button_height,
        [65, 54, 38, 255],
    );
    fill_dialogue_rect(
        pixels,
        width,
        height,
        x as usize,
        y as usize,
        button_width,
        1,
        [142, 120, 82, 255],
    );
    fill_dialogue_rect(
        pixels,
        width,
        height,
        x as usize,
        (y + button_height as i32 - 1) as usize,
        button_width,
        1,
        [142, 120, 82, 255],
    );
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
        [238, 226, 197],
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

fn render_alliance_panel(
    font: &fontdue::Font,
    guild: Option<&openstory_protocol::GuildInfo>,
    alliance: Option<&openstory_protocol::AllianceInfo>,
    invite: Option<&(i32, String)>,
) -> Vec<u8> {
    let width = ALLIANCE_PANEL_WIDTH as usize;
    let height = ALLIANCE_PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        height,
        [16, 14, 12, 232],
    );
    for (x, y, w, h) in [
        (0, 0, width, 2),
        (0, height - 2, width, 2),
        (0, 0, 2, height),
        (width - 2, 0, 2, height),
    ] {
        fill_dialogue_rect(&mut pixels, width, height, x, y, w, h, [112, 96, 72, 255]);
    }
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
        [226, 205, 158],
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
        [170, 164, 148],
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
            [225, 225, 215],
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
                [255, 222, 112],
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
            [190, 190, 190],
        );
        for (row, member_guild) in alliance.guilds.iter().take(8).enumerate() {
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
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!("{}{}", if local { "• " } else { "  " }, name),
                18,
                116 + row as i32 * 24,
                11.0,
                width - 36,
                18,
                if local {
                    [255, 222, 112]
                } else {
                    [220, 220, 210]
                },
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
                [220, 220, 210],
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
            [220, 220, 210],
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
            [170, 164, 148],
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
            [220, 220, 210],
        );
    }
    if let Some((_, inviter)) = invite {
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            12,
            height - 76,
            width - 24,
            1,
            [84, 76, 62, 255],
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
            [255, 222, 112],
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
        [170, 164, 148],
    );
    pixels
}

fn render_guild_bbs_panel(
    font: &fontdue::Font,
    bbs: &GuildBbsUi,
    guild: Option<&openstory_protocol::GuildInfo>,
) -> Vec<u8> {
    let width = GUILD_BBS_PANEL_WIDTH as usize;
    let height = GUILD_BBS_PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        height,
        [16, 14, 12, 245],
    );
    for (x, y, w, h) in [
        (0, 0, width, 2),
        (0, height - 2, width, 2),
        (0, 0, 2, height),
        (width - 2, 0, 2, height),
    ] {
        fill_dialogue_rect(&mut pixels, width, height, x, y, w, h, [124, 105, 76, 255]);
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Guild Bulletin Board",
        12,
        9,
        15.0,
        520,
        22,
        [238, 215, 168],
    );

    for (x, label) in [
        (604, "Refresh"),
        (660, "Prev"),
        (706, "Next"),
        (774, "Close"),
    ] {
        let button_width = match label {
            "Refresh" => 54,
            "Close" => 40,
            _ => 44,
        };
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            x,
            46,
            button_width,
            30,
            [58, 49, 37, 255],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            x,
            46,
            button_width,
            1,
            [142, 118, 80, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            label,
            x as i32 + 5,
            55,
            9.0,
            button_width - 10,
            16,
            [232, 220, 190],
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
        54,
        10.0,
        360,
        18,
        [190, 184, 168],
    );

    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        10,
        82,
        356,
        426,
        [31, 28, 23, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        374,
        82,
        width - 386,
        448,
        [31, 28, 23, 255],
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
        [208, 192, 153],
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
        [208, 192, 153],
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
            [175, 170, 156],
        );
    }
    for (index, (thread, is_notice)) in rows.iter().take(11).enumerate() {
        let y = 108 + index as i32 * 30;
        let selected = bbs.thread.as_ref().is_some_and(|view| view.id == thread.id);
        if selected {
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                14,
                (y - 3) as usize,
                348,
                28,
                [60, 50, 36, 255],
            );
        }
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
            if *is_notice {
                [255, 222, 112]
            } else {
                [230, 224, 208]
            },
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
            [160, 154, 140],
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
            [255, 224, 160],
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
            [165, 160, 146],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            386,
            170,
            width - 398,
            1,
            [80, 70, 54, 255],
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
            [222, 216, 200],
        );
        if !thread.replies.is_empty() {
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                386,
                310,
                width - 398,
                1,
                [80, 70, 54, 255],
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
                [208, 192, 153],
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
                    [195, 190, 176],
                );
            }
        }

        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            386,
            450,
            350,
            42,
            if bbs.reply_focused {
                [46, 42, 34, 255]
            } else {
                [39, 36, 30, 255]
            },
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            744,
            450,
            68,
            42,
            [70, 58, 42, 255],
        );
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
            463,
            9.0,
            338,
            24,
            if bbs.reply_input.is_empty() && !bbs.reply_focused {
                [145, 140, 128]
            } else {
                [235, 228, 208]
            },
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Reply",
            755,
            463,
            9.0,
            54,
            24,
            [240, 224, 190],
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
            [172, 166, 151],
        );
    }

    for (x, label, button_width) in [(14, "Write", 76), (98, "Notice", 86), (192, "Delete", 86)] {
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            x,
            462,
            button_width,
            36,
            [58, 49, 37, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            label,
            x as i32 + 9,
            473,
            9.0,
            button_width - 18,
            16,
            [232, 220, 190],
        );
    }
    if bbs.thread.is_none() {
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            386,
            450,
            426,
            42,
            [28, 25, 21, 255],
        );
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
            [145, 140, 128],
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
            [255, 210, 120],
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
        [170, 164, 148],
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
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            192,
            124,
            440,
            350,
            [19, 17, 14, 255],
        );
        for (x, y, w, h) in [
            (192, 124, 440, 2),
            (192, 472, 440, 2),
            (192, 124, 2, 350),
            (630, 124, 2, 350),
        ] {
            fill_dialogue_rect(&mut pixels, width, height, x, y, w, h, [126, 105, 73, 255]);
        }
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            if draft.is_notice {
                "Write Guild Notice"
            } else {
                "Write Guild Post"
            },
            210,
            142,
            14.0,
            404,
            22,
            [240, 217, 172],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("Subject · {}/25", draft.title.encode_utf16().count()),
            224,
            172,
            9.0,
            370,
            16,
            [195, 188, 170],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            224,
            192,
            376,
            30,
            if draft.focus == GuildBbsDraftFocus::Title {
                [55, 48, 37, 255]
            } else {
                [38, 35, 30, 255]
            },
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &draft.title,
            232,
            200,
            10.0,
            360,
            18,
            [238, 232, 216],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("Content · {}/600", draft.content.encode_utf16().count()),
            224,
            226,
            9.0,
            370,
            16,
            [195, 188, 170],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            224,
            246,
            376,
            156,
            if draft.focus == GuildBbsDraftFocus::Content {
                [55, 48, 37, 255]
            } else {
                [38, 35, 30, 255]
            },
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &draft.content,
            232,
            254,
            10.0,
            360,
            140,
            [238, 232, 216],
        );
        if let Some(feedback) = bbs.feedback.as_deref() {
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                feedback,
                224,
                408,
                8.0,
                376,
                14,
                [255, 190, 130],
            );
        }
        for (x, label) in [(410, "Cancel"), (510, "Publish")] {
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                x,
                422,
                90,
                36,
                [69, 57, 41, 255],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                label,
                x as i32 + 10,
                433,
                9.0,
                70,
                16,
                [240, 224, 190],
            );
        }
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Esc cancels · Enter moves to content; use Publish to submit",
            210,
            454,
            8.0,
            404,
            14,
            [160, 154, 140],
        );
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
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            260,
            220,
            304,
            124,
            [19, 17, 14, 255],
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
            [240, 217, 172],
        );
        for (x, label) in [(320, "Cancel"), (420, "Delete")] {
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                x,
                300,
                80,
                34,
                [69, 57, 41, 255],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                label,
                x as i32 + 8,
                310,
                9.0,
                64,
                16,
                [240, 224, 190],
            );
        }
    }
    pixels
}
