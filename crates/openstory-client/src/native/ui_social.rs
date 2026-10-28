fn render_chat_panel(
    ui_archive: &NxArchive,
    font: &fontdue::Font,
    lines: &VecDeque<ChatLine>,
    input: &str,
    open: bool,
    target: ChatTarget,
    composer_label: Option<&str>,
) -> Vec<u8> {
    let width = CHAT_WIDTH as usize;
    let height = CHAT_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    if open {
        // Keep this authored v83 frame at its native 295x240 size.
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            width,
            height,
            ui_archive,
            "UIWindow.img/Messenger/backgrnd2",
            0,
            0,
        );
    } else {
        // The collapsed chat surface follows the v83 status-bar chat strip.
        let _ = composite_nx_ui_bitmap_scaled_to(
            &mut pixels,
            width,
            height,
            ui_archive,
            "StatusBar.img/base/backgrnd2",
            0,
            0,
            width,
            71,
        );
    }
    let first = lines.len().saturating_sub(if open { 6 } else { 1 });
    for (row, line) in lines.iter().skip(first).enumerate() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &line.text,
            12,
            (if open { 52 } else { 27 }) + row as i32 * 15,
            12.0,
            width - 24,
            16,
            readable_chat_panel_color(line.color),
        );
    }
    if open {
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            width,
            height,
            ui_archive,
            "StatusBar.img/base/chat",
            6,
            height as i32 - 32,
        );
    }
    let prompt = if open {
        format!("[{}] >", composer_label.unwrap_or_else(|| target.label()))
    } else {
        "> Press Enter to chat".to_owned()
    };
    let input_x = 12 + dialogue_text_width(font, &prompt, 12.0).ceil() as i32 + 5;
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        &prompt,
        12,
        if open { height as i32 - 23 } else { 43 },
        12.0,
        width - 24,
        16,
        [76, 59, 43],
    );
    if !input.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            input,
            input_x,
            if open { height as i32 - 23 } else { 43 },
            12.0,
            (width as i32 - input_x - 12).max(1) as usize,
            16,
            [58, 47, 38],
        );
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
    input_focused: bool,
    target_focused: bool,
) -> Vec<u8> {
    let width = WHISPER_PANEL_WIDTH as usize;
    let height = WHISPER_PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    let _ = composite_nx_ui_layers(
        &mut pixels,
        width,
        height,
        ui_archive,
        &[
            ("UIWindow.img/Messenger/backgrnd", 0, 0),
            ("UIWindow.img/Messenger/backgrnd2", 0, 0),
            ("UIWindow.img/Messenger/backgrnd3", 14, 330),
            ("UIWindow.img/Messenger/BtEnter/normal/0", 205, 333),
        ],
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
    let row_start = 120;
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
    } else if !input_focused || target_focused || input.is_empty() {
        visible_input.clear();
    }
    if input_focused && !target_focused && !visible_input.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &visible_input,
            20,
            height as i32 - 30,
            12.0,
            input_width,
            18,
            [60, 49, 39],
        );
    }
    pixels
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
    let _ = composite_nx_ui_layers(
        &mut pixels,
        width,
        height,
        ui_archive,
        &[
            ("UIWindow.img/UserList/backgrnd", 0, 0),
            ("UIWindow.img/UserList/Tab/disabled/0", 0, 0),
            ("UIWindow.img/UserList/Tab/enabled/1", 0, 0),
            ("UIWindow.img/UserList/Tab/disabled/2", 0, 0),
            ("UIWindow.img/UserList/Tab/disabled/3", 0, 0),
            ("UIWindow.img/UserList/Party/party0", 10, 115),
        ],
    );
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
            let y = 142 + index as i32 * 18;
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
    owl: &OwlSearchUi,
    string_archive: &NxArchive,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<u8> {
    let width = OWL_PANEL_WIDTH as usize;
    let height = OWL_PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        height,
        [25, 23, 20, 246],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        36,
        [88, 68, 42, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        2,
        [206, 174, 112, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        height - 2,
        width,
        2,
        [206, 174, 112, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        2,
        height,
        [206, 174, 112, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        width - 2,
        0,
        2,
        height,
        [206, 174, 112, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Owl of Minerva",
        16,
        8,
        16.0,
        width - 72,
        22,
        [255, 229, 174],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "X",
        width as i32 - 28,
        8,
        15.0,
        16,
        20,
        [255, 238, 218],
    );

    if owl.result_item_id.is_none() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Search by item name or ID. Select a popular item below, or type a query.",
            16,
            44,
            11.5,
            width - 32,
            30,
            [211, 202, 184],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            16,
            78,
            326,
            32,
            [10, 10, 10, 255],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            16,
            78,
            326,
            1,
            [142, 128, 105, 255],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            16,
            109,
            326,
            1,
            [142, 128, 105, 255],
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
            25,
            84,
            13.0,
            306,
            20,
            if owl.query.is_empty() {
                [145, 140, 129]
            } else {
                [250, 244, 232]
            },
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            354,
            78,
            94,
            32,
            [103, 75, 37, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Search",
            374,
            84,
            13.0,
            62,
            20,
            [255, 233, 190],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Most searched items",
            16,
            123,
            13.0,
            width - 32,
            20,
            [229, 206, 160],
        );
        for (index, item_id) in owl.top_item_ids.iter().take(10).enumerate() {
            let y = 151 + index as i32 * 24;
            if index % 2 == 0 {
                fill_dialogue_rect(
                    &mut pixels,
                    width,
                    height,
                    12,
                    y as usize,
                    width - 24,
                    23,
                    [46, 42, 35, 230],
                );
            }
            let name = item_name(*item_id, string_archive, name_cache);
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!("{}. {name}", index + 1),
                22,
                y + 3,
                12.0,
                width - 44,
                18,
                [220, 217, 207],
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
                22,
                158,
                12.0,
                width - 44,
                18,
                [180, 176, 165],
            );
        }
    } else {
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
            16,
            47,
            13.0,
            width - 32,
            22,
            [229, 206, 160],
        );
        if owl.results.is_empty() {
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                "No open shops were found for this item.",
                20,
                90,
                12.0,
                width - 40,
                20,
                [210, 204, 190],
            );
        }
        for (visible_index, result) in owl
            .results
            .iter()
            .enumerate()
            .skip(owl.result_offset)
            .take(8)
        {
            let y = 78 + (visible_index - owl.result_offset) as i32 * 39;
            let selected = owl.selected_result == Some(visible_index);
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                12,
                y as usize,
                width - 24,
                37,
                if selected {
                    [102, 75, 38, 255]
                } else if visible_index % 2 == 0 {
                    [45, 42, 36, 238]
                } else {
                    [34, 32, 28, 238]
                },
            );
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
                20,
                y + 3,
                11.5,
                width - 40,
                17,
                [245, 240, 226],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!("{} · Map {}", result.description, result.map_id),
                20,
                y + 20,
                10.5,
                width - 40,
                15,
                [181, 177, 167],
            );
        }
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            12,
            410,
            126,
            30,
            [69, 62, 49, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Back",
            58,
            417,
            11.5,
            50,
            17,
            [238, 229, 207],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            146,
            410,
            74,
            30,
            [69, 62, 49, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "‹ Prev",
            157,
            417,
            11.5,
            58,
            17,
            [238, 229, 207],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            232,
            410,
            74,
            30,
            [69, 62, 49, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Next ›",
            247,
            417,
            11.5,
            58,
            17,
            [238, 229, 207],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            318,
            410,
            132,
            30,
            [103, 75, 37, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Go to shop",
            342,
            417,
            11.5,
            98,
            17,
            [255, 233, 190],
        );
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
    let _ = composite_nx_ui_layers(
        &mut pixels,
        width,
        height,
        ui_archive,
        &[
            ("UIWindow.img/UserList/backgrnd", 0, 0),
            ("UIWindow.img/UserList/backgrnd2", 0, 0),
            ("UIWindow.img/UserList/Tab/enabled/0", 0, 0),
            ("UIWindow.img/UserList/Tab/disabled/1", 0, 0),
            ("UIWindow.img/UserList/Tab/disabled/2", 0, 0),
            ("UIWindow.img/UserList/Tab/disabled/3", 0, 0),
            ("UIWindow.img/UserList/Friend/friend0", 0, 0),
            ("UIWindow.img/UserList/Friend/line", 132, 115),
        ],
    );
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
            116,
            10.0,
            width - 24,
            30,
            [50, 50, 50],
        );
    }
    for (row, buddy) in buddies.iter().skip(scroll).take(row_limit).enumerate() {
        let y = 116 + row as i32 * 24;
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
