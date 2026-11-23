fn render_chat_panel(
    ui_archive: &NxArchive,
    font: &fontdue::Font,
    lines: &VecDeque<ChatLine>,
    input: &str,
    open: bool,
    scroll: usize,
    input_focused: bool,
    target: ChatTarget,
    composer_label: Option<&str>,
) -> Vec<u8> {
    let width = CHAT_WIDTH as usize;
    let height = CHAT_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    let history_height = CHAT_HISTORY_HEIGHT;
    let visible_rows = if open { CHAT_ROWS } else { 1 };
    let max_scroll = lines.len().saturating_sub(visible_rows);
    let first = if open { max_scroll.saturating_sub(scroll) } else { max_scroll };
    let panel_top = if open { 0 } else { history_height - 16 };
    fill_dialogue_rect(&mut pixels, width, height, 0, panel_top, width, history_height - panel_top, [0, 0, 0, 165]);
    if open {
        let fraction = if max_scroll == 0 { 1.0 } else { first as f32 / max_scroll as f32 };
        ui_foundation::draw_scrollbar(&mut pixels, width, height, ui_archive, width as i32 - 16, 0, history_height, fraction);
    }
    let first_y = history_height - 2 - visible_rows.min(lines.len()) * 13;
    for (row, line) in lines.iter().skip(first).take(visible_rows).enumerate() {
        let text = ui_foundation::ellipsize(font, &line.text, 11.0, (width - 22) as f32);
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &text,
            3,
            (first_y + row * 13) as i32,
            11.0,
            width - 14,
            13,
            line.color,
        );
    }
    let _ = composite_nx_ui_bitmap(&mut pixels, width, height, ui_archive,
        "StatusBar.img/base/iconBlue", CHAT_TOGGLE_X as i32, CHAT_TOGGLE_Y as i32);
    fill_dialogue_rect(&mut pixels, width, height, CHAT_TOGGLE_X + 3, CHAT_TOGGLE_Y + 5, 6, 2, [255; 4]);
    if !open {
        fill_dialogue_rect(&mut pixels, width, height, CHAT_TOGGLE_X + 5, CHAT_TOGGLE_Y + 3, 2, 6, [255; 4]);
    }
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        width,
        height,
        ui_archive,
        "StatusBar.img/base/chatTarget",
        0,
        history_height as i32 + 6,
    );
    let prompt = composer_label.unwrap_or_else(|| target.label());
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        prompt,
        5,
        history_height as i32 + 8,
        10.0,
        70,
        14,
        [255, 255, 255],
    );
    let text = if input_focused {
        input
    } else {
        "Press Enter to chat"
    };
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        text,
        88,
        history_height as i32 + 8,
        11.0,
        width - 110,
        15,
        [35, 35, 35],
    );
    if input_focused {
        let caret_x = 88 + dialogue_text_width(font, input, 11.0).ceil() as usize;
        if caret_x < width - 20 {
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                caret_x,
                history_height + 9,
                1,
                12,
                [35, 35, 35, 255],
            );
        }
    }
    pixels
}

fn readable_chat_panel_color(color: [u8; 3]) -> [u8; 3] {
    let luminance =
        (u16::from(color[0]) * 54 + u16::from(color[1]) * 183 + u16::from(color[2]) * 19) / 256;
    if luminance > 145 {
        color.map(|channel| ((u16::from(channel) * 2) / 5) as u8)
    } else {
        color
    }
}

fn render_whisper_panel(
    ui_archive: &NxArchive,
    font: &fontdue::Font,
    conversation: &WhisperConversation,
    _input_focused: bool,
    _target_focused: bool,
) -> Vec<u8> {
    let width = WHISPER_PANEL_WIDTH as usize;
    let height = WHISPER_PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    let mut dialog = NxDialog::new(&mut pixels, width, height, ui_archive);
    let _ = dialog.authored_window(&[("UIWindow.img/Messenger/backgrnd", 0, 0)], WHISPER_CLOSE);
    dialog.replace_title(
        font,
        "Whisper",
        ui_foundation::UiRect::new(7.0, 5.0, 203.0, 12.0),
    );
    let state = if whisper_can_send(conversation) {
        ui_dialog::ButtonState::Normal
    } else {
        ui_dialog::ButtonState::Disabled
    };
    let _ = dialog.button(
        WHISPER_SEND,
        ui_dialog::NxButton::new("UIWindow.img/Messenger/BtEnter", state),
    );
    let recipient = if conversation.target.is_empty() {
        "To: enter a character name".to_owned()
    } else {
        format!("To: {}", conversation.target)
    };
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        &recipient,
        16,
        34,
        12.0,
        width - 32,
        18,
        [56, 48, 39],
    );

    let visible_lines = 9;
    let first = conversation.lines.len().saturating_sub(visible_lines);
    let visible_messages = conversation.lines.iter().skip(first).collect::<Vec<_>>();
    let row_start = 154;
    for (row, line) in visible_messages.into_iter().enumerate() {
        let wrapped = wrap_speech_text(font, &line.text, 12.0, width - 24, 1);
        if let Some(text) = wrapped.first() {
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                text,
                12,
                row_start + row as i32 * 14,
                12.0,
                width - 24,
                16,
                readable_chat_panel_color(line.color),
            );
        }
    }
    let input = conversation.input.trim_end();
    let input_width = 176;
    let mut visible_input = input.to_owned();
    let full_input_width = dialogue_text_width(font, &format!("> {input}"), 12.0);
    if full_input_width > input_width as f32 {
        while !visible_input.is_empty()
            && dialogue_text_width(font, &format!("…{visible_input}"), 12.0) > input_width as f32
        {
            visible_input.remove(0);
        }
        visible_input.insert(0, '…');
    }
    if !visible_input.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &visible_input,
            20,
            319,
            12.0,
            input_width,
            18,
            [60, 49, 39],
        );
    }
    pixels
}

fn whisper_can_send(conversation: &WhisperConversation) -> bool {
    !conversation.target.trim().is_empty() && !conversation.input.trim().is_empty()
}

fn render_party_panel(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    party: Option<&openstory_protocol::PartySnapshot>,
    invite: Option<&(i32, String)>,
    local_id: Option<i32>,
) -> Vec<u8> {
    let width = PARTY_PANEL_WIDTH as usize;
    let height = PARTY_PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    let _ = NxDialog::new(&mut pixels, width, height, ui_archive).layers(&[
        ("UIWindow.img/UserList/backgrnd", 0, 0),
        ("UIWindow.img/UserList/Tab/disabled/0", 15, 32),
        ("UIWindow.img/UserList/Tab/enabled/1", 65, 32),
        ("UIWindow.img/UserList/Tab/disabled/2", 112, 32),
        ("UIWindow.img/UserList/Tab/disabled/3", 160, 32),
        ("UIWindow.img/UserList/Party/party0", 10, 115),
    ]);
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Party",
        22,
        61,
        12.0,
        width - 24,
        18,
        [37, 37, 37],
    );
    if let Some(party) = party {
        let leader = party
            .members
            .iter()
            .find(|member| member.character_id == party.leader_id)
            .map(|member| member.name.as_str())
            .unwrap_or("Unknown");
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("Party {} · Leader {leader}", party.party_id),
            20,
            92,
            9.0,
            width - 40,
            14,
            [65, 65, 65],
        );
        if party.members.is_empty() {
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                "Waiting for member details…",
                37,
                139,
                11.0,
                width - 24,
                18,
                [45, 45, 45],
            );
        }
        for (index, member) in party.members.iter().take(6).enumerate() {
            let y = 142 + index as i32 * 30;
            let leader_mark = if member.character_id == party.leader_id {
                "★ "
            } else {
                "  "
            };
            let local_mark = if local_id == Some(member.character_id) {
                " (You)"
            } else {
                ""
            };
            let channel = if member.channel < 0 {
                "Offline".to_owned()
            } else {
                format!("Ch {}", member.channel.saturating_add(1))
            };
            let line = format!(
                "{leader_mark}{} Lv.{}  {channel}{local_mark}",
                member.name, member.level
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &line,
                27,
                y - 2,
                10.0,
                width - 55,
                15,
                if member.channel < 0 {
                    [130, 130, 130]
                } else {
                    [35, 35, 35]
                },
            );
            if let Some((hp, max_hp)) = member.hp.filter(|(_, max_hp)| *max_hp > 0) {
                let bar_width = 63 * hp.clamp(0, max_hp) as usize / max_hp as usize;
                let _ = composite_nx_ui_bitmap(
                    &mut pixels,
                    width,
                    height,
                    ui_archive,
                    "UIWindow.img/UserList/Party/PartyHP/GaugeBar/bar",
                    27,
                    y + 18,
                );
                if bar_width > 0 {
                    let _ = composite_nx_ui_bitmap_scaled_to(
                        &mut pixels,
                        width,
                        height,
                        ui_archive,
                        "UIWindow.img/UserList/Party/PartyHP/GaugeBar/gauge",
                        27,
                        (y + 18) as usize,
                        bar_width,
                        7,
                    );
                }
            }
        }
    } else {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "You are not in a party.",
            27,
            136,
            11.0,
            width - 24,
            18,
            [45, 45, 45],
        );
    }
    if let Some((party_id, inviter)) = invite {
        let display_name = inviter.strip_prefix("PS: ").unwrap_or(inviter);
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("Invite from {display_name} (#{party_id})"),
            20,
            height as i32 - 46,
            11.0,
            width - 24,
            16,
            [255, 222, 112],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "/party accept  ·  /party deny",
            20,
            height as i32 - 28,
            10.0,
            width - 24,
            14,
            [205, 194, 168],
        );
    } else {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "O toggles this panel · /party controls",
            20,
            height as i32 - 27,
            10.0,
            width - 24,
            14,
            [170, 164, 148],
        );
    }
    pixels
}

fn render_owl_panel(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    owl: &OwlSearchUi,
    string_archive: &NxArchive,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<u8> {
    let width = OWL_PANEL_WIDTH as usize;
    let height = OWL_PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    if owl.result_item_id.is_none() {
        let _ = NxDialog::new(&mut pixels, width, height, ui_archive).frame(
            "UIWindow.img/itemSearch/backgrnd",
            0,
            0,
        );
        if !owl.top_item_ids.is_empty() || owl.top_items_loaded {
            let _ = NxDialog::new(&mut pixels, width, height, ui_archive).frame(
                "UIWindow.img/itemSearch/searchback",
                248,
                0,
            );
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                width,
                height,
                ui_archive,
                "Basic.img/BtHide/normal/0",
                430,
                6,
            );
            for (path, x) in [
                ("UIWindow.img/itemSearch/BtLeft/normal/0", 414),
                ("UIWindow.img/itemSearch/BtRight/normal/0", 430),
                ("UIWindow.img/itemSearch/BtSearch2/normal/0", 381),
                ("UIWindow.img/itemSearch/BtGo/normal/0", 196),
            ] {
                let y = if x == 196 { 218 } else if x == 381 { 248 } else { 24 };
                let _ = composite_nx_ui_bitmap(
                    &mut pixels,
                    width,
                    height,
                    ui_archive,
                    path,
                    x,
                    y,
                );
            }
        }
        let _ = NxDialog::new(&mut pixels, width, height, ui_archive)
        .anchored_button("Basic.img/BtClose3/normal/0", 230, 5);
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            width,
            height,
            ui_archive,
            "UIWindow.img/itemSearch/BtSearch/normal/0",
            178,
            77,
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            if owl.query.is_empty() {
                "Type an item name or ID…"
            } else {
                &owl.query
            },
            22,
            73,
            11.0,
            146,
            16,
            [35, 35, 35],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Most searched items",
            260,
            14,
            11.0,
            width - 270,
            16,
            [35, 35, 35],
        );
        for (index, item_id) in owl.top_item_ids.iter().take(10).enumerate() {
            let y = 38 + index as i32 * 16;
            let name = item_name(*item_id, string_archive, name_cache);
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!("{}. {name}", index + 1),
                262,
                y,
                11.0,
                width - 274,
                15,
                [35, 35, 35],
            );
        }
        if owl.top_item_ids.is_empty() {
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                if owl.top_items_loaded {
                    "No popular searches are available."
                } else {
                    "Loading popular searches…"
                },
                262,
                38,
                11.0,
                width - 274,
                16,
                [60, 60, 60],
            );
        }
    } else {
        let _ = NxDialog::new(&mut pixels, width, height, ui_archive).frame(
            "UIWindow.img/itemSearch/resultback",
            0,
            0,
        );
        for (path, x) in [
            ("Basic.img/BtClose3/normal/0", 230),
            ("UIWindow.img/itemSearch/BtRetry/normal/0", 160),
            ("UIWindow.img/itemSearch/BtLink/normal/0", 232),
            ("UIWindow.img/itemSearch/BtCancel/normal/0", 300),
        ] {
            let y = if x == 230 { 5 } else { 403 };
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                width,
                height,
                ui_archive,
                path,
                x,
                y,
            );
        }
        let searched_name = item_name(
            owl.result_item_id.unwrap_or_default(),
            string_archive,
            name_cache,
        );
        let start = if owl.results.is_empty() {
            0
        } else {
            owl.result_offset.min(owl.results.len() - 1) + 1
        };
        let end = (owl.result_offset + 8).min(owl.results.len());
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!(
                "Listings for {searched_name} · {start}–{end} / {}",
                owl.results.len()
            ),
            26,
            66,
            12.0,
            width - 52,
            17,
            [35, 35, 35],
        );
        if owl.results.is_empty() {
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                "No open shops were found for this item.",
                26,
                100,
                12.0,
                width - 40,
                20,
                [35, 35, 35],
            );
        }
        for (visible_index, result) in owl
            .results
            .iter()
            .enumerate()
            .skip(owl.result_offset)
            .take(8)
        {
            let y = 174 + (visible_index - owl.result_offset) as i32 * 23;
            let units = i64::from(result.bundles).saturating_mul(i64::from(result.per_bundle));
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!(
                    "{} · Ch {} · {} each · {} units",
                    result.owner,
                    i16::from(result.channel).saturating_add(1),
                    result.price.max(0),
                    units.max(0)
                ),
                16,
                y,
                10.0,
                width - 32,
                13,
                [35, 35, 35],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!("{} · Map {}", result.description, result.map_id),
                16,
                y + 13,
                9.0,
                width - 32,
                12,
                [80, 80, 80],
            );
        }
    }
    if let Some(notice) = owl.notice.as_deref() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            notice,
            16,
            height as i32 - 19,
            10.5,
            width - 32,
            18,
            [248, 185, 130],
        );
    }
    pixels
}

fn render_buddy_panel(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    buddies: &[openstory_protocol::BuddyEntry],
    capacity: u8,
    total_buddies: usize,
    group_filter: Option<&str>,
    invite: Option<&(i32, String)>,
    scroll: usize,
) -> Vec<u8> {
    let width = BUDDY_PANEL_WIDTH as usize;
    let height = BUDDY_PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    let _ = NxDialog::new(&mut pixels, width, height, ui_archive).layers(&[
        ("UIWindow.img/UserList/backgrnd", 0, 0),
        ("UIWindow.img/UserList/backgrnd2", 0, 0),
        ("UIWindow.img/UserList/Tab/enabled/0", 15, 32),
        ("UIWindow.img/UserList/Tab/disabled/1", 65, 32),
        ("UIWindow.img/UserList/Tab/disabled/2", 112, 32),
        ("UIWindow.img/UserList/Tab/disabled/3", 160, 32),
        ("UIWindow.img/UserList/Friend/friend0", 10, 115),
        ("UIWindow.img/UserList/Friend/line", 132, 142),
    ]);
    let online = buddies.iter().filter(|buddy| buddy.channel >= 0).count();
    let title = if let Some(group) = group_filter {
        format!("Buddy List · {group}  {online}/{} online", buddies.len())
    } else {
        format!("Buddy List  {online}/{} online", buddies.len())
    };
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        &title,
        22,
        61,
        11.0,
        width - 24,
        18,
        [37, 37, 37],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        &format!("Capacity: {total_buddies}/{capacity}"),
        20,
        78,
        9.0,
        width - 24,
        14,
        [70, 70, 70],
    );
    let row_limit = if invite.is_some() { 5 } else { 7 };
    if buddies.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            if group_filter.is_some() {
                "No buddies in this group."
            } else {
                "No buddies yet. Use /buddy add <name>."
            },
            20,
            146,
            10.0,
            width - 24,
            30,
            [50, 50, 50],
        );
    }
    for (row, buddy) in buddies.iter().skip(scroll).take(row_limit).enumerate() {
        let y = 146 + row as i32 * 24;
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            width,
            height,
            ui_archive,
            if buddy.channel >= 0 {
                "UIWindow.img/UserList/Friend/icon0"
            } else {
                "UIWindow.img/UserList/Friend/icon1"
            },
            13,
            y + 4,
        );
        let status = if buddy.channel < 0 {
            "Offline".to_owned()
        } else {
            format!("Channel {}", buddy.channel.saturating_add(1))
        };
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &buddy.name,
            40,
            y + 4,
            10.0,
            112,
            14,
            if buddy.channel < 0 {
                [125, 125, 125]
            } else {
                [35, 35, 35]
            },
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &status,
            160,
            y + 4,
            9.0,
            100,
            15,
            [70, 70, 70],
        );
    }
    if let Some((character_id, name)) = invite {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("{name} invited you (#{character_id})"),
            12,
            height as i32 - 51,
            10.0,
            width - 24,
            14,
            [255, 222, 112],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "/buddy accept  ·  /buddy dismiss",
            12,
            height as i32 - 35,
            10.0,
            width - 24,
            14,
            [205, 194, 168],
        );
    } else {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "B · /buddy add/group · /buddy filter all|group",
            20,
            height as i32 - 27,
            9.0,
            width - 24,
            13,
            [70, 70, 70],
        );
    }
    pixels
}

fn load_guild_mark_catalog(
    archive: &NxArchive,
) -> Result<(Vec<GuildMarkChoice>, Vec<GuildMarkChoice>), String> {
    fn child_colors(node: &NxNode) -> Result<Vec<i8>, String> {
        let mut colors = Vec::new();
        for child in node.children().map_err(|error| error.to_string())? {
            let Ok(color) = child
                .name()
                .map_err(|error| error.to_string())?
                .parse::<i8>()
            else {
                continue;
            };
            if child.bitmap().map_err(|error| error.to_string())?.is_some() {
                colors.push(color);
            }
        }
        colors.sort_unstable();
        colors.dedup();
        Ok(colors)
    }

    fn display_name(node: &NxNode, fallback: String) -> Result<String, String> {
        let Some(name) = node.child("name").map_err(|error| error.to_string())? else {
            return Ok(fallback);
        };
        match name.value().map_err(|error| error.to_string())? {
            NodeValue::String(value) if !value.is_empty() => Ok(value),
            _ => Ok(fallback),
        }
    }

    let root = archive.root();
    let backgrounds_root = root
        .resolve("GuildMark.img/BackGround")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "GuildMark.img/BackGround was not found".to_owned())?;
    let marks_root = root
        .resolve("GuildMark.img/Mark")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "GuildMark.img/Mark was not found".to_owned())?;

    let mut backgrounds = vec![GuildMarkChoice {
        id: 0,
        category: "BackGround".to_owned(),
        label: "None".to_owned(),
        colors: vec![0],
    }];
    for node in backgrounds_root
        .children()
        .map_err(|error| error.to_string())?
    {
        let name = node.name().map_err(|error| error.to_string())?;
        let Ok(id) = name.parse::<i16>() else {
            continue;
        };
        let colors = child_colors(&node)?;
        if colors.is_empty() {
            continue;
        }
        backgrounds.push(GuildMarkChoice {
            id,
            category: "BackGround".to_owned(),
            label: display_name(&node, format!("Background {id}"))
                .unwrap_or_else(|_| format!("Background {id}")),
            colors,
        });
    }

    let mut marks = vec![GuildMarkChoice {
        id: 0,
        category: "".to_owned(),
        label: "None".to_owned(),
        colors: vec![0],
    }];
    for category_name in ["Animal", "Etc", "Letter", "Pattern", "Plant"] {
        let Some(category) = marks_root
            .child(category_name)
            .map_err(|error| error.to_string())?
        else {
            continue;
        };
        for node in category.children().map_err(|error| error.to_string())? {
            let name = node.name().map_err(|error| error.to_string())?;
            let Ok(id) = name.parse::<i16>() else {
                continue;
            };
            let colors = child_colors(&node)?;
            if colors.is_empty() {
                continue;
            }
            marks.push(GuildMarkChoice {
                id,
                category: category_name.to_owned(),
                label: display_name(&node, format!("{category_name} {id}"))
                    .unwrap_or_else(|_| format!("{category_name} {id}")),
                colors,
            });
        }
    }
    if backgrounds.len() <= 1 || marks.len() <= 1 {
        return Err("GuildMark.img did not contain usable background and mark choices".into());
    }
    Ok((backgrounds, marks))
}

fn step_guild_mark_id(choices: &[GuildMarkChoice], current: i16, delta: i16) -> i16 {
    let Some(last) = choices.len().checked_sub(1) else {
        return current;
    };
    let Some(index) = choices.iter().position(|choice| choice.id == current) else {
        return if delta < 0 {
            choices[last].id
        } else {
            choices[0].id
        };
    };
    choices[index.saturating_add_signed(delta as isize).min(last)].id
}

fn nearest_guild_mark_color(choices: &[GuildMarkChoice], id: i16, current: i8) -> i8 {
    choices
        .iter()
        .find(|choice| choice.id == id)
        .and_then(|choice| {
            choice
                .colors
                .iter()
                .copied()
                .find(|color| *color == current)
                .or_else(|| choice.colors.first().copied())
        })
        .unwrap_or(current)
}

fn step_guild_mark_color(choices: &[GuildMarkChoice], id: i16, current: i8, delta: i16) -> i8 {
    let Some(choice) = choices.iter().find(|choice| choice.id == id) else {
        return current;
    };
    let Some(last) = choice.colors.len().checked_sub(1) else {
        return current;
    };
    let Some(index) = choice.colors.iter().position(|color| *color == current) else {
        return if delta < 0 {
            choice.colors[last]
        } else {
            choice.colors[0]
        };
    };
    choice.colors[index.saturating_add_signed(delta as isize).min(last)]
}
