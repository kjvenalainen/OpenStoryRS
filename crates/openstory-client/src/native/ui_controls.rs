const COMBO_COUNTER_WIDTH: u32 = 140;
const COMBO_COUNTER_HEIGHT: u32 = 44;
const COMBO_COUNTER_X: f32 = 864.0;
const COMBO_COUNTER_Y: f32 = 150.0;

fn render_combo_counter(font: &fontdue::Font, count: i32) -> Vec<u8> {
    let width = COMBO_COUNTER_WIDTH as usize;
    let height = COMBO_COUNTER_HEIGHT as usize;
    let mut pixels = vec![0_u8; width * height * 4];
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        height,
        [0, 0, 0, 154],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "COMBO",
        47,
        4,
        11.0,
        66,
        15,
        [0, 0, 0],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "COMBO",
        46,
        3,
        11.0,
        66,
        15,
        [255, 235, 96],
    );
    let count_text = count.to_string();
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        &count_text,
        35,
        24,
        18.0,
        70,
        20,
        [0, 0, 0],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        &count_text,
        34,
        23,
        18.0,
        70,
        20,
        [255, 255, 255],
    );
    pixels
}

#[allow(clippy::too_many_arguments)]
fn render_key_config_panel(
    font: &fontdue::Font,
    ui: &NxArchive,
    skills: &NxArchive,
    items: &NxArchive,
    characters: &NxArchive,
    bindings: &[openstory_protocol::KeyBinding],
    quickslot_keys: &[u8; 8],
    capture: Option<usize>,
    quickslot_capture: Option<usize>,
    first_row: usize,
    actions: &[KeyConfigAction],
    active_skill_ids: &HashSet<i32>,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    counts: &HashMap<i32, u32>,
) -> Vec<u8> {
    keyboard_settings::render(
        font,
        ui,
        skills,
        items,
        characters,
        bindings,
        quickslot_keys,
        capture,
        quickslot_capture,
        first_row,
        actions,
        active_skill_ids,
        icon_cache,
        counts,
    )
}

pub(super) fn unbound_action_cell(kind: u8, action: i32) -> Option<(i32, i32)> {
    let slot = match kind {
        4 => match action {
            0 => (6, 0),       // Equipment
            1 => (7, 0),       // Items
            2 => (6, 1),       // Stats
            3 => (7, 1),       // Skills
            4 => (13, 1),      // Friends
            5 => (14, 1),      // World map
            6 => (0, 0),       // Chat
            7 => (15, 1),      // Mini map
            8 => (8, 1),       // Quest log
            9 => (16, 1),      // Key bindings
            10 => (0, 1),      // Say
            11 => (2, 0),      // Whisper
            12 => (1, 1),      // Party chat
            13 => (2, 1),      // Friends chat
            14 => (9, 0),      // Menu
            15 => (10, 0),     // Quick slots
            16 => (1, 0),      // Toggle chat
            17 => (10, 1),     // Guild
            18 => (0, 2),      // Guild chat
            19 => (11, 1),     // Party
            21 => (8, 0),      // Character info
            22 => (16, 0),     // Monster book
            23 => (2, 2),      // Cash shop
            24 => (1, 2),      // Alliance chat
            25 => (9, 1),      // Change channel
            26 | 42 => (5, 2), // Family shortcut
            27 => (4, 0),      // Boss party
            28 => (3, 2),      // Main menu / settings
            _ => return None,
        },
        5 => match action {
            50 => (11, 0), // Pickup
            51 => (12, 0), // Sit
            52 => (13, 0), // Attack
            53 => (14, 0), // Jump
            54 => (15, 0), // NPC chat
            _ => return None,
        },
        6 if (100..=106).contains(&action) => (action - 100, 3),
        _ => return None,
    };
    Some(slot)
}

fn draw_nx_slider(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    ui: &NxArchive,
    (left, center_y, track_width): (usize, usize, usize),
    thumb_offset: usize,
    thumb_state: &str,
) -> Result<(), String> {
    composite_nx_ui_bitmap_scaled_to(
        pixels,
        width,
        height,
        ui,
        "Basic.img/Slider/mid",
        left,
        center_y.saturating_sub(3),
        track_width,
        6,
    )?;
    composite_nx_ui_bitmap(
        pixels,
        width,
        height,
        ui,
        &format!("Basic.img/Slider/{thumb_state}"),
        (left + thumb_offset).saturating_sub(11) as i32,
        center_y.saturating_sub(7) as i32,
    )
}

fn draw_option_checkbox(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    ui_archive: &NxArchive,
    x: usize,
    y: usize,
    selected: bool,
    checked: bool,
) {
    let state = match (checked, selected) {
        (false, false) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (true, true) => 3,
    };
    let _ = composite_nx_ui_bitmap(
        pixels,
        width,
        height,
        ui_archive,
        &format!("Basic.img/CheckBox/{state}"),
        x as i32 + 4,
        y as i32 + 4,
    );
}

fn maple_key_from_physical(key: PhysicalKey) -> Option<u8> {
    let PhysicalKey::Code(key) = key else {
        return None;
    };
    Some(match key {
        KeyCode::Digit0 => 2,
        KeyCode::Digit1 => 3,
        KeyCode::Digit2 => 4,
        KeyCode::Digit3 => 5,
        KeyCode::Digit4 => 6,
        KeyCode::Digit5 => 7,
        KeyCode::Digit6 => 8,
        KeyCode::Digit7 => 9,
        KeyCode::Digit8 => 10,
        KeyCode::Digit9 => 11,
        KeyCode::Minus => 12,
        KeyCode::Equal => 13,
        KeyCode::KeyQ => 16,
        KeyCode::KeyW => 17,
        KeyCode::KeyE => 18,
        KeyCode::KeyR => 19,
        KeyCode::KeyT => 20,
        KeyCode::KeyY => 21,
        KeyCode::KeyU => 22,
        KeyCode::KeyI => 23,
        KeyCode::KeyO => 24,
        KeyCode::KeyP => 25,
        KeyCode::BracketLeft => 26,
        KeyCode::BracketRight => 27,
        KeyCode::ControlLeft => 29,
        KeyCode::KeyA => 30,
        KeyCode::KeyS => 31,
        KeyCode::KeyD => 32,
        KeyCode::KeyF => 33,
        KeyCode::KeyG => 34,
        KeyCode::KeyH => 35,
        KeyCode::KeyJ => 36,
        KeyCode::KeyK => 37,
        KeyCode::KeyL => 38,
        KeyCode::Semicolon => 39,
        KeyCode::Quote => 40,
        KeyCode::Backquote => 41,
        KeyCode::ShiftLeft => 42,
        KeyCode::Backslash => 43,
        KeyCode::KeyZ => 44,
        KeyCode::KeyX => 45,
        KeyCode::KeyC => 46,
        KeyCode::KeyV => 47,
        KeyCode::KeyB => 48,
        KeyCode::KeyN => 49,
        KeyCode::KeyM => 50,
        KeyCode::Comma => 51,
        KeyCode::Period => 52,
        KeyCode::Slash => 53,
        KeyCode::AltLeft | KeyCode::SuperLeft => 56,
        KeyCode::Space => 57,
        KeyCode::F1 => 59,
        KeyCode::F2 => 60,
        KeyCode::F3 => 61,
        KeyCode::F4 => 62,
        KeyCode::F5 => 63,
        KeyCode::F6 => 64,
        KeyCode::F7 => 65,
        KeyCode::F8 => 66,
        KeyCode::F9 => 67,
        KeyCode::F10 => 68,
        KeyCode::F11 => 69,
        KeyCode::F12 => 70,
        KeyCode::Home => 71,
        KeyCode::PageUp => 73,
        KeyCode::End => 79,
        KeyCode::PageDown => 81,
        KeyCode::Insert => 82,
        KeyCode::Delete => 83,
        KeyCode::Escape => 84,
        KeyCode::ControlRight => 85,
        KeyCode::ShiftRight => 86,
        KeyCode::AltRight | KeyCode::SuperRight => 87,
        KeyCode::ScrollLock => 88,
        _ => return None,
    })
}

fn maple_key_label(key: u8) -> String {
    match key {
        2..=11 => char::from(b'0' + (key - 2)).to_string(),
        12 => "-".to_owned(),
        13 => "=".to_owned(),
        16..=25 => {
            ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"][usize::from(key - 16)].to_owned()
        }
        26 => "[".to_owned(),
        27 => "]".to_owned(),
        29 => "L Ctrl".to_owned(),
        30..=38 => char::from(b'A' + (key - 30)).to_string(),
        39 => ";".to_owned(),
        40 => "'".to_owned(),
        41 => "`".to_owned(),
        42 => "L Shift".to_owned(),
        43 => "\\".to_owned(),
        44..=50 => ["Z", "X", "C", "V", "B", "N", "M"][usize::from(key - 44)].to_owned(),
        51 => ",".to_owned(),
        52 => ".".to_owned(),
        53 => "/".to_owned(),
        56 => "Alt / Cmd".to_owned(),
        57 => "Space".to_owned(),
        59..=70 => format!("F{}", key - 58),
        71 => "Home".to_owned(),
        73 => "Page Up".to_owned(),
        79 => "End".to_owned(),
        81 => "Page Down".to_owned(),
        82 => "Insert".to_owned(),
        83 => "Delete".to_owned(),
        84 => "Escape".to_owned(),
        85 => "R Ctrl".to_owned(),
        86 => "R Shift".to_owned(),
        87 => "R Alt / Cmd".to_owned(),
        88 => "Scroll Lock".to_owned(),
        _ => format!("Key {key}"),
    }
}

fn quickslot_short_key_label(key: u8) -> String {
    match key {
        14 => "Bksp".to_owned(),
        15 => "Tab".to_owned(),
        29 | 85 => "Ctl".to_owned(),
        42 | 86 => "Shft".to_owned(),
        56 | 87 => "Alt".to_owned(),
        57 => "Spc".to_owned(),
        71 => "Hme".to_owned(),
        73 => "PgU".to_owned(),
        79 => "End".to_owned(),
        81 => "PgD".to_owned(),
        82 => "Ins".to_owned(),
        83 => "Del".to_owned(),
        84 => "Esc".to_owned(),
        88 => "Scr".to_owned(),
        _ => maple_key_label(key),
    }
}

fn render_skill_book_panel(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    skill_archive: &NxArchive,
    entries: &[SkillUiEntry],
    book_title: &str,
    book_icon: Option<(u32, u32, Arc<[u8]>)>,
    skill_points: u16,
    skills: &HashMap<i32, openstory_protocol::SkillEntry>,
    active_skill_ids: &HashSet<i32>,
    selected_tab: u8,
    visible_tabs: u8,
    scroll_row: usize,
    macro_panel_open: bool,
) -> Vec<u8> {
    const WIDTH: usize = SKILL_BOOK_PANEL_WIDTH as usize;
    const HEIGHT: usize = SKILL_BOOK_PANEL_HEIGHT as usize;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    let mut dialog = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui_archive);
    let _ = dialog.authored_window(
        &[("UIWindow.img/Skill/backgrnd", 0, 0)],
        ui_foundation::SKILL_CLOSE_BUTTON,
    );
    for tab in 0..visible_tabs.min(5) {
        let active = tab == selected_tab;
        let bounds = ui_foundation::skill_tab(usize::from(tab));
        let _ = dialog.tab(bounds, active);
        let path = format!(
            "UIWindow.img/Skill/Tab/{}/{tab}",
            if active { "enabled" } else { "disabled" }
        );
        let _ = dialog.buttons(&[(&path, bounds)]);
    }
    if let Some((icon_width, icon_height, icon)) = book_icon {
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            9,
            59,
            28,
            27,
            [220, 220, 220, 255],
        );
        composite_inventory_icon(
            &mut pixels,
            WIDTH,
            HEIGHT,
            &icon,
            icon_width as usize,
            icon_height as usize,
            10,
            59,
            27,
        );
    }
    let title = ui_foundation::ellipsize(font, book_title, 11.0, 111.0);
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &title,
        48,
        66,
        11.0,
        111,
        15,
        [255, 255, 255],
    );
    let first = scroll_row.min(entries.len().saturating_sub(SKILL_BOOK_VISIBLE_ROWS));
    for (row, skill) in entries
        .iter()
        .skip(first)
        .take(SKILL_BOOK_VISIBLE_ROWS)
        .enumerate()
    {
        let y = ui_foundation::skill_row(row).y as usize;
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            if skill.level <= 0 || skill.max_level == 0 {
                "UIWindow.img/Skill/skill0"
            } else {
                "UIWindow.img/Skill/skill1"
            },
            6,
            y as i32,
        );
        let skill_disabled = skill.level <= 0 || skill.max_level == 0;
        let icon_path = |variant| format!("{:03}.img/skill/{:07}/{variant}", skill.id / 10_000, skill.id);
        let (icon, grayscale_fallback) = if skill_disabled {
            decode_nx_icon(skill_archive, &icon_path("iconDisabled"))
                .map(|icon| (Some(icon), false))
                .unwrap_or_else(|| {
                    (
                        decode_nx_icon(skill_archive, &icon_path("icon")),
                        true,
                    )
                })
        } else {
            (decode_nx_icon(skill_archive, &icon_path("icon")), false)
        };
        if let Some((w, h, icon)) = icon {
            if grayscale_fallback {
                let mut grayscale = icon.to_vec();
                for pixel in grayscale.chunks_exact_mut(4) {
                    let gray = ((u16::from(pixel[0])
                        + u16::from(pixel[1])
                        + u16::from(pixel[2]))
                        / 3) as u8;
                    pixel[..3].fill(gray);
                }
                composite_inventory_icon(
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    &grayscale,
                    w as usize,
                    h as usize,
                    11,
                    y + 3,
                    30,
                );
            } else {
                composite_inventory_icon(
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    &icon,
                    w as usize,
                    h as usize,
                    11,
                    y + 3,
                    30,
                );
            }
        }
        if active_skill_ids.contains(&skill.id) {
            draw_auto_buff_sparkle(&mut pixels, WIDTH, HEIGHT, 41, y + 33);
        }
        let name = ui_foundation::ellipsize(font, &skill.name, 11.0, 99.0);
        let text_color = if skill_disabled {
            [110, 110, 110]
        } else {
            [40, 40, 40]
        };
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &name,
            47,
            y as i32 + 2,
            11.0,
            99,
            14,
            text_color,
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &skill.level.to_string(),
            47,
            y as i32 + 20,
            11.0,
            72,
            14,
            text_color,
        );
        let state = if client_cash_skills::can_spend_skill_point(skill, skill_points, skills) {
            "normal"
        } else {
            "disabled"
        };
        let path = format!("UIWindow.img/Skill/BtSpUp/{state}/0");
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            &path,
            ui_foundation::skill_sp_button(row).x as i32,
            ui_foundation::skill_sp_button(row).y as i32,
        );
    }
    if entries.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "No skills for this job.",
            12,
            104,
            11.0,
            132,
            16,
            [60, 60, 60],
        );
    }
    let fraction = ui_foundation::scroll_fraction(first, entries.len(), SKILL_BOOK_VISIBLE_ROWS);
    ui_foundation::draw_scrollbar(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        152,
        99,
        155,
        fraction,
    );
    let value = skill_points.to_string();
    let value_width = dialogue_text_width(font, &value, 11.0).ceil() as i32;
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &value,
        108 - value_width,
        266,
        11.0,
        28,
        16,
        [30, 30, 30],
    );
    let macro_button = ui_foundation::SKILL_MACRO_BUTTON;
    let macro_button_drawn = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "Basic.img/BtMacro/normal/0",
        macro_button.x as i32,
        macro_button.y as i32,
    );
    if macro_panel_open && macro_button_drawn.is_ok() {
        // Mirror only the arrow on the 48×18 NX button, keeping the label and bevel intact.
        for y in 5..14 {
            let row = ((macro_button.y as usize + y) * WIDTH + macro_button.x as usize) * 4;
            for (left, right) in [(38, 44), (39, 43), (40, 42)] {
                for channel in 0..4 {
                    pixels.swap(row + left * 4 + channel, row + right * 4 + channel);
                }
            }
        }
    }
    pixels
}

fn render_options_panel(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    settings: AudioSettings,
    input_settings: InputSettings,
    gameplay_settings: GameplaySettings,
    social_settings: SocialSettings,
    selected: OptionControl,
    notice: Option<&str>,
) -> Vec<u8> {
    const WIDTH: usize = SETTINGS_PANEL_WIDTH as usize;
    const HEIGHT: usize = SETTINGS_PANEL_HEIGHT as usize;
    const GAME_X: usize = 300;
    const GAME_Y: usize = 0;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];

    // Keep the rows above Monster Info and join the original footer below them.
    let mut dialog = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui_archive);
    if let Some((source_width, _, source)) = dialog.frame_bitmap("UIWindow.img/SysOpt/backgrnd") {
        let _ = dialog.frame_rows(
            &source,
            source_width,
            0,
            &[
                (0..SYSTEM_OPTIONS_FOOTER_Y, 0),
                (330..366, SYSTEM_OPTIONS_FOOTER_Y as i32),
            ],
        );
        dialog.replace_title(
            font,
            "System Options",
            ui_foundation::UiRect::new(7.0, 5.0, 163.0, 12.0),
        );
    }
    // Move the baked Mute labels and Scroll Lock badge inward. The blank
    // pixels to their left wrap around to fill the space along the row edge.
    for (x, first_y, end_y, inset) in [
        (SYSTEM_OPTIONS_MUTE_CELL_X, 63, 117, 12),
        (248, 121, 150, 8),
    ] {
        for row in pixels.chunks_exact_mut(WIDTH * 4).take(end_y).skip(first_y) {
            row[x * 4..SYSTEM_OPTIONS_ROW_RIGHT * 4].rotate_left(inset * 4);
        }
    }
    let mut dialog = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui_archive);
    let _ = dialog.authored_window(
        &[(
            "UIWindow.img/GameOpt/backgrnd",
            GAME_X as i32,
            GAME_Y as i32,
        )],
        SETTINGS_CLOSE_BUTTON,
    );
    dialog.replace_title(
        font,
        "Game Options",
        ui_foundation::UiRect::new(GAME_X as f32 + 7.0, 5.0, 125.0, 12.0),
    );

    // The SysOpt artwork carries its own labels and layout. Overlay only live
    // values and controls in the authored row fields.
    for (slider, value) in [
        (
            OptionControl::GraphicsQuality,
            gameplay_settings.graphics_quality,
        ),
        (OptionControl::Music, settings.music_volume),
        (OptionControl::SoundEffects, settings.effects_volume),
        (
            OptionControl::EffectsQuality,
            gameplay_settings.effects_quality,
        ),
        (OptionControl::MouseSpeed, input_settings.mouse_speed),
        (OptionControl::HpWarning, gameplay_settings.hp_warning),
        (OptionControl::MpWarning, gameplay_settings.mp_warning),
    ] {
        let y = slider.track_y().expect("sliders have a track position");
        let track_left = slider.track_left();
        let track_width = slider.track_width();
        let filled_width = usize::from(value) * track_width / 100;
        let thumb_state = if slider == selected {
            "thumbMouseOver"
        } else {
            "thumbNormal"
        };
        let _ = draw_nx_slider(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            (track_left, y, track_width),
            filled_width,
            thumb_state,
        );
    }

    draw_dialogue_text(font, &mut pixels, WIDTH, HEIGHT, "Effects", 307, 262, 9.0, 48, 13, [38, 48, 55]);

    for (control, checked) in [
        (OptionControl::MuteMusic, settings.mute_music),
        (OptionControl::MuteEffects, settings.mute_effects),
        (OptionControl::ScreenShake, gameplay_settings.screen_shake_enabled),
    ] {
        draw_option_checkbox(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            SYSTEM_OPTIONS_CHECKBOX_RIGHT - 12 - 4,
            control.system_checkbox_y() - 10,
            selected == control,
            checked,
        );
    }

    // GameOpt owns the social labels. A checkbox replaces its baked ALLOW cell
    // with the current state while leaving every authored label visible.
    let permissions = [
        (OptionControl::AllowWhisper, social_settings.allow_whisper),
        (
            OptionControl::AllowFriendChat,
            social_settings.allow_friend_chat,
        ),
        (
            OptionControl::AllowChatInvite,
            social_settings.allow_chat_invite,
        ),
        (
            OptionControl::AllowTradeRequest,
            social_settings.allow_trade_request,
        ),
        (
            OptionControl::AllowPartyInvite,
            social_settings.allow_party_invite,
        ),
        (
            OptionControl::AllowExpeditionInvite,
            social_settings.allow_expedition_invite,
        ),
        (OptionControl::AllowFollow, social_settings.allow_follow),
        (
            OptionControl::AllowGuildChat,
            social_settings.allow_guild_chat,
        ),
        (
            OptionControl::AllowGuildInvite,
            social_settings.allow_guild_invite,
        ),
        (
            OptionControl::AllowAllianceChat,
            social_settings.allow_alliance_chat,
        ),
        (
            OptionControl::AllowAllianceInvite,
            social_settings.allow_alliance_invite,
        ),
        (
            OptionControl::AllowFamilyInvite,
            social_settings.allow_family_invite,
        ),
    ];
    for (index, (control, allowed)) in permissions.into_iter().enumerate() {
        let y = GAME_Y + 31 + index * 18;
        draw_option_checkbox(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            GAME_X + 124,
            y,
            selected == control,
            allowed,
        );
    }

    let _ = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui_archive).buttons(&[
        ("Basic.img/BtOK2/normal/0", SYSTEM_OPTIONS_OK_BUTTON),
        ("Basic.img/BtCancel2/normal/0", SYSTEM_OPTIONS_CANCEL_BUTTON),
    ]);
    if let Some(notice) = notice {
        let notice_width = SYSTEM_OPTIONS_OK_BUTTON.x as usize - 12;
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            4,
            SYSTEM_OPTIONS_FOOTER_Y + 9,
            notice_width + 4,
            22,
            [189, 204, 215, 255],
        );
        let notice = ui_foundation::ellipsize(font, notice, 9.0, notice_width as f32);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &notice,
            5,
            SYSTEM_OPTIONS_FOOTER_Y as i32 + 12,
            9.0,
            notice_width,
            18,
            [120, 24, 24],
        );
    }
    pixels
}

// Absent server data uses exactly the defaults shown by Keyboard Settings.
// An explicit empty server slot stays empty; it must not revive a hard-coded shortcut.
fn gameplay_key_binding(
    bindings: Option<&[openstory_protocol::KeyBinding]>,
    key: PhysicalKey,
) -> Option<openstory_protocol::KeyBinding> {
    let key = usize::from(maple_key_from_physical(key)?);
    Some(match bindings {
        Some(bindings) => bindings.get(key).copied().unwrap_or_default(),
        None => keyboard_settings::defaults()
            .get(key)
            .copied()
            .unwrap_or_default(),
    })
}

// Releases run before modal/text input routing. Retain the action from key-down
// so changing a keymap while a key is held cannot leave its old action active.
impl GameplayInput {
    fn release_key(&mut self, key: PhysicalKey) {
        if let Some(binding) = self.held_bindings.remove(&key)
            && !self.held_bindings.values().any(|other| *other == binding)
        {
            match binding.action {
                50 => self.pickup = false,
                51 => self.sit = false,
                200 => self.left = false,
                201 => self.right = false,
                202 => {
                    self.suppress_up_until_release = false;
                    self.up = false;
                }
                203 => self.down = false,
                _ => {}
            }
        }
        match key {
            PhysicalKey::Code(KeyCode::ArrowLeft) => {
                self.left = self
                    .held_bindings
                    .values()
                    .any(|binding| binding.action == 200)
            }
            PhysicalKey::Code(KeyCode::ArrowRight) => {
                self.right = self
                    .held_bindings
                    .values()
                    .any(|binding| binding.action == 201)
            }
            PhysicalKey::Code(KeyCode::ArrowUp) => {
                let held = self
                    .held_bindings
                    .values()
                    .any(|binding| binding.action == 202);
                if !held {
                    self.suppress_up_until_release = false;
                }
                self.up = held && !self.suppress_up_until_release;
            }
            PhysicalKey::Code(KeyCode::ArrowDown) => {
                self.down = self
                    .held_bindings
                    .values()
                    .any(|binding| binding.action == 203)
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod keyboard_input_tests {
    use super::*;

    #[test]
    fn releasing_one_of_two_bound_keys_keeps_the_action_until_both_release() {
        let mut input = GameplayInput::default();
        input.sit = true;
        let first = PhysicalKey::Code(KeyCode::KeyE);
        let second = PhysicalKey::Code(KeyCode::KeyR);
        let binding = openstory_protocol::KeyBinding {
            kind: 5,
            action: 51,
        };
        input.held_bindings.insert(first, binding);
        input.held_bindings.insert(second, binding);
        input.release_key(first);
        assert!(input.sit);
        input.release_key(second);
        assert!(!input.sit);
        assert!(input.held_bindings.is_empty());
    }

    #[test]
    fn modal_release_uses_key_down_action_and_input_reset_discards_held_keys() {
        let mut input = GameplayInput::default();
        let key = PhysicalKey::Code(KeyCode::KeyE);
        input.left = true;
        input.held_bindings.insert(
            key,
            openstory_protocol::KeyBinding {
                kind: 5,
                action: 200,
            },
        );
        // The release needs neither the current keymap nor the current UI focus.
        input.release_key(key);
        assert!(!input.left);
        input.held_bindings.insert(
            key,
            openstory_protocol::KeyBinding {
                kind: 5,
                action: 51,
            },
        );
        input = GameplayInput::default();
        assert!(input.held_bindings.is_empty());
        input.release_key(key);
        assert!(!input.sit);
    }

    #[test]
    fn fixed_arrows_release_behind_modal_windows_without_cancelling_other_keys() {
        let mut input = GameplayInput::default();
        input.left = true;
        let other = PhysicalKey::Code(KeyCode::KeyE);
        input.held_bindings.insert(
            other,
            openstory_protocol::KeyBinding {
                kind: 5,
                action: 200,
            },
        );
        input.release_key(PhysicalKey::Code(KeyCode::ArrowLeft));
        assert!(input.left);
        input.release_key(other);
        assert!(!input.left);
    }

    #[test]
    fn bottom_row_matches_v83_scan_codes_and_z_defaults_to_pickup() {
        for (index, (physical, label)) in [
            (KeyCode::KeyZ, "Z"),
            (KeyCode::KeyX, "X"),
            (KeyCode::KeyC, "C"),
            (KeyCode::KeyV, "V"),
            (KeyCode::KeyB, "B"),
            (KeyCode::KeyN, "N"),
            (KeyCode::KeyM, "M"),
            (KeyCode::Comma, ","),
            (KeyCode::Period, "."),
            (KeyCode::Slash, "/"),
        ]
        .into_iter()
        .enumerate()
        {
            let wire = 44 + index as u8;
            assert_eq!(
                maple_key_from_physical(PhysicalKey::Code(physical)),
                Some(wire)
            );
            assert_eq!(maple_key_label(wire), label);
            let rect = keyboard_settings::key_rect(wire).unwrap();
            assert_eq!(rect.x, 98.0 + index as f32 * 34.0);
            assert_eq!(
                keyboard_settings::key_at(rect.x + 16.0, rect.y + 16.0),
                Some(wire)
            );
        }
        let pickup = openstory_protocol::KeyBinding {
            kind: 5,
            action: 50,
        };
        assert_eq!(
            gameplay_key_binding(None, PhysicalKey::Code(KeyCode::KeyZ)),
            Some(pickup)
        );
        let mut server_map = vec![openstory_protocol::KeyBinding::default(); 90];
        server_map[44] = pickup;
        assert_eq!(
            gameplay_key_binding(Some(&server_map), PhysicalKey::Code(KeyCode::KeyZ)),
            Some(pickup)
        );
    }

    #[test]
    fn server_bindings_override_defaults_and_cleared_keys_stay_unbound() {
        let key = PhysicalKey::Code(KeyCode::KeyE);
        assert_eq!(
            gameplay_key_binding(None, key),
            Some(openstory_protocol::KeyBinding { kind: 4, action: 0 })
        );
        let mut bindings = keyboard_settings::defaults();
        bindings[18] = openstory_protocol::KeyBinding {
            kind: 1,
            action: 2101001,
        };
        assert_eq!(
            gameplay_key_binding(Some(&bindings), key),
            Some(bindings[18])
        );
        bindings[18] = openstory_protocol::KeyBinding::default();
        assert_eq!(
            gameplay_key_binding(Some(&bindings), key),
            Some(openstory_protocol::KeyBinding::default())
        );
        assert_eq!(
            gameplay_key_binding(Some(&[]), key),
            Some(openstory_protocol::KeyBinding::default())
        );
    }

    #[test]
    fn physical_keys_match_keyboard_targets_including_independent_modifiers() {
        for (physical, wire) in [
            (KeyCode::KeyE, 18),
            (KeyCode::ControlLeft, 29),
            (KeyCode::ControlRight, 85),
            (KeyCode::ShiftLeft, 42),
            (KeyCode::ShiftRight, 86),
            (KeyCode::AltLeft, 56),
            (KeyCode::AltRight, 87),
            (KeyCode::F12, 70),
            (KeyCode::Digit0, 2),
            (KeyCode::Digit1, 3),
        ] {
            assert_eq!(
                maple_key_from_physical(PhysicalKey::Code(physical)),
                Some(wire)
            );
            let rect = keyboard_settings::key_rect(wire).unwrap();
            assert_eq!(
                keyboard_settings::key_at(rect.x + 1.0, rect.y + 1.0),
                Some(wire)
            );
        }
    }
}
