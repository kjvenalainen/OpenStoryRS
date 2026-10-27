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

fn render_key_config_panel(
    font: &fontdue::Font,
    bindings: &[openstory_protocol::KeyBinding],
    quickslot_keys: &[u8; 8],
    capture: Option<usize>,
    quickslot_capture: Option<usize>,
    first_row: usize,
    actions: &[KeyConfigAction],
) -> Vec<u8> {
    let width = KEY_CONFIG_PANEL_WIDTH as usize;
    let height = KEY_CONFIG_PANEL_HEIGHT as usize;
    let mut pixels = vec![0_u8; width * height * 4];
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
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Keyboard Bindings",
        16,
        10,
        19.0,
        width - 32,
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
        10,
        18.0,
        18,
        24,
        [255, 224, 178],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Click an action or press 1-8, then press its key.",
        18,
        48,
        12.0,
        width - 36,
        20,
        [223, 216, 197],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Mouse wheel scrolls; Backspace clears a binding.",
        18,
        62,
        11.0,
        width - 36,
        16,
        [183, 177, 164],
    );
    let first_row = first_row.min(actions.len().saturating_sub(KEY_CONFIG_VISIBLE_ROWS));
    for (visible_row, index) in (first_row..actions.len())
        .take(KEY_CONFIG_VISIBLE_ROWS)
        .enumerate()
    {
        let action_entry = &actions[index];
        let y = KEY_CONFIG_ROW_TOP + visible_row as i32 * KEY_CONFIG_ROW_HEIGHT;
        let is_capturing = capture == Some(index);
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            14,
            y as usize,
            width - 28,
            28,
            if is_capturing {
                [91, 72, 43, 255]
            } else {
                [45, 41, 35, 255]
            },
        );
        let maple_key = bindings
            .iter()
            .take(90)
            .position(|binding| {
                binding.kind == action_entry.kind && binding.action == action_entry.action
            })
            .map(|key| key as u8);
        let assigned = if is_capturing {
            if action_entry.assignable {
                "Press a key...".to_owned()
            } else {
                "Backspace clears".to_owned()
            }
        } else {
            maple_key.map_or_else(|| "Unbound".to_owned(), maple_key_label)
        };
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("{}  {}", visible_row + 1, action_entry.label),
            24,
            y + 5,
            15.0,
            235,
            20,
            if action_entry.assignable {
                [238, 232, 216]
            } else {
                [175, 164, 148]
            },
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &assigned,
            267,
            y + 5,
            15.0,
            138,
            20,
            if is_capturing && action_entry.assignable {
                [255, 218, 142]
            } else if !action_entry.assignable {
                [207, 169, 133]
            } else {
                [199, 221, 195]
            },
        );
    }
    let last_row = (first_row + KEY_CONFIG_VISIBLE_ROWS).min(actions.len());
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        &format!(
            "Actions {}–{} of {}",
            first_row + 1,
            last_row,
            actions.len()
        ),
        18,
        333,
        11.0,
        width - 36,
        14,
        [183, 177, 164],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        14,
        350,
        width - 28,
        1,
        [101, 88, 66, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Quick-slot keys",
        18,
        356,
        14.0,
        width - 36,
        19,
        [240, 224, 191],
    );
    for row in 0..2 {
        for column in 0..4 {
            let slot = row * 4 + column;
            let x = 16 + column * 98;
            let y = 378 + row * 21;
            let capturing = quickslot_capture == Some(slot);
            fill_dialogue_rect(
                &mut pixels,
                width,
                height,
                x,
                y,
                94,
                19,
                if capturing {
                    [91, 72, 43, 255]
                } else {
                    [45, 41, 35, 255]
                },
            );
            let label = if capturing {
                format!("{}: Press key", slot + 1)
            } else {
                format!("{}: {}", slot + 1, maple_key_label(quickslot_keys[slot]))
            };
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &label,
                x as i32 + 4,
                y as i32 + 2,
                11.0,
                87,
                15,
                if capturing {
                    [255, 218, 142]
                } else {
                    [213, 213, 207]
                },
            );
        }
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Click a slot, then press a key. Esc/Backspace cancels; changes save locally.",
        18,
        423,
        10.0,
        width - 36,
        16,
        [183, 177, 164],
    );
    pixels
}

fn draw_option_checkbox(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    selected: bool,
    checked: bool,
) {
    fill_dialogue_rect(
        pixels,
        width,
        height,
        x,
        y,
        20,
        20,
        if selected {
            [236, 213, 159, 255]
        } else {
            [136, 125, 104, 255]
        },
    );
    fill_dialogue_rect(
        pixels,
        width,
        height,
        x + 3,
        y + 3,
        14,
        14,
        [33, 31, 28, 255],
    );
    if checked {
        fill_dialogue_rect(
            pixels,
            width,
            height,
            x + 6,
            y + 10,
            4,
            3,
            [232, 205, 140, 255],
        );
        fill_dialogue_rect(
            pixels,
            width,
            height,
            x + 9,
            y + 7,
            5,
            3,
            [232, 205, 140, 255],
        );
    }
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
        KeyCode::KeyZ => 45,
        KeyCode::KeyX => 46,
        KeyCode::KeyC => 47,
        KeyCode::KeyV => 48,
        KeyCode::KeyB => 49,
        KeyCode::KeyN => 50,
        KeyCode::KeyM => 51,
        KeyCode::Comma => 52,
        KeyCode::Period => 53,
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

fn quickslot_key_slot_at(local_x: f32, local_y: f32) -> Option<usize> {
    let row = if (378.0..397.0).contains(&local_y) {
        0
    } else if (399.0..418.0).contains(&local_y) {
        1
    } else {
        return None;
    };
    if !(16.0..408.0).contains(&local_x) {
        return None;
    }
    let column = ((local_x - 16.0) / 98.0).floor() as usize;
    (column < 4).then_some(row * 4 + column)
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
        45..=51 => ["Z", "X", "C", "V", "B", "N", "M"][usize::from(key - 45)].to_owned(),
        52 => ",".to_owned(),
        53 => ".".to_owned(),
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
    entries: &[SkillUiEntry],
    skill_points: u16,
    selected_tab: u8,
    scroll_row: usize,
) -> Vec<u8> {
    const WIDTH: usize = 430;
    const HEIGHT: usize = 500;
    const ROW_HEIGHT: usize = 34;
    const ROWS: usize = 10;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        HEIGHT,
        [25, 27, 31, 245],
    );
    for (x, y, width, height) in [
        (0, 0, WIDTH, 2),
        (0, HEIGHT - 2, WIDTH, 2),
        (0, 0, 2, HEIGHT),
        (WIDTH - 2, 0, 2, HEIGHT),
    ] {
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            y,
            width,
            height,
            [192, 164, 112, 255],
        );
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Skill Book",
        14,
        10,
        20.0,
        330,
        25,
        [246, 226, 185],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "X",
        402,
        12,
        15.0,
        18,
        20,
        [240, 220, 180],
    );
    let tabs = ["Beginner", "1st job", "2nd job", "3rd job", "4th job"];
    for (index, label) in tabs.into_iter().enumerate() {
        let x = 10 + index * 82;
        let active = index == usize::from(selected_tab);
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            42,
            76,
            26,
            if active {
                [92, 75, 48, 255]
            } else {
                [48, 48, 52, 255]
            },
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            x as i32 + 4,
            46,
            11.0,
            68,
            18,
            if active {
                [255, 231, 173]
            } else {
                [190, 190, 190]
            },
        );
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &format!("SP: {skill_points}"),
        15,
        74,
        12.0,
        120,
        17,
        [218, 218, 218],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        300,
        71,
        112,
        19,
        [69, 57, 40, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Macros…",
        328,
        73,
        11.0,
        74,
        16,
        [238, 220, 181],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        12,
        89,
        WIDTH - 24,
        1,
        [96, 88, 77, 255],
    );
    let end = entries.len().min(scroll_row.saturating_add(ROWS));
    for (row, skill) in entries
        .get(scroll_row..end)
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        let y = 94 + row * ROW_HEIGHT;
        let mastered = skill.level >= skill.max_level;
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            12,
            y,
            WIDTH - 24,
            ROW_HEIGHT - 2,
            if row % 2 == 0 {
                [36, 38, 42, 235]
            } else {
                [31, 33, 37, 235]
            },
        );
        let color = if skill.level > 0 {
            [238, 221, 181]
        } else {
            [168, 168, 168]
        };
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &skill.name,
            20,
            y as i32 + 2,
            13.0,
            300,
            17,
            color,
        );
        let level_cap = if skill.master_level > 0 {
            skill.max_level.min(skill.master_level)
        } else {
            skill.max_level
        };
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!("Lv. {} / {}   MP {}", skill.level, level_cap, skill.mp_cost),
            20,
            y as i32 + 17,
            10.0,
            260,
            13,
            [175, 178, 182],
        );
        if skill.level > 0 && !skill.passive {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                "Use",
                341,
                y as i32 + 8,
                12.0,
                38,
                18,
                [220, 220, 220],
            );
        }
        if skill.level < level_cap && !mastered {
            fill_dialogue_rect(
                &mut pixels,
                WIDTH,
                HEIGHT,
                391,
                y + 5,
                27,
                22,
                [78, 95, 61, 255],
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                "+",
                399,
                y as i32 + 5,
                16.0,
                14,
                19,
                [225, 242, 206],
            );
        }
    }
    if entries.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "No skills are available for this job tab.",
            20,
            112,
            12.0,
            WIDTH - 40,
            20,
            [185, 185, 185],
        );
    }
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        12,
        444,
        WIDTH - 24,
        1,
        [96, 88, 77, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Click + to spend SP; click Use for an active skill. K toggles this window.",
        16,
        451,
        10.0,
        WIDTH - 32,
        16,
        [195, 195, 195],
    );
    pixels
}

fn render_options_panel(
    font: &fontdue::Font,
    settings: AudioSettings,
    display_settings: DisplaySettings,
    input_settings: InputSettings,
    gameplay_settings: GameplaySettings,
    social_settings: SocialSettings,
    selected: OptionControl,
    notice: Option<&str>,
) -> Vec<u8> {
    const WIDTH: usize = SETTINGS_PANEL_WIDTH as usize;
    const HEIGHT: usize = SETTINGS_PANEL_HEIGHT as usize;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        0,
        0,
        WIDTH,
        HEIGHT,
        [177, 145, 92, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        4,
        4,
        WIDTH - 8,
        HEIGHT - 8,
        [31, 29, 27, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        4,
        4,
        WIDTH - 8,
        42,
        [83, 63, 40, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "OpenStoryRS Options",
        22,
        12,
        18.0,
        WIDTH - 44,
        22,
        [246, 226, 185],
    );

    for (slider, label, value) in [
        (
            OptionControl::GraphicsQuality,
            "Graphics quality",
            gameplay_settings.graphics_quality,
        ),
        (
            OptionControl::EffectsQuality,
            "Effects quality",
            gameplay_settings.effects_quality,
        ),
        (OptionControl::Music, "Music volume", settings.music_volume),
        (
            OptionControl::SoundEffects,
            "Sound effects",
            settings.effects_volume,
        ),
        (
            OptionControl::HpWarning,
            "HP warning",
            gameplay_settings.hp_warning,
        ),
        (
            OptionControl::MpWarning,
            "MP warning",
            gameplay_settings.mp_warning,
        ),
        (
            OptionControl::MouseSpeed,
            "Mouse speed",
            input_settings.mouse_speed,
        ),
    ] {
        let y = slider.track_y().expect("sliders have a track position");
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            26,
            y as i32 - 12,
            13.0,
            112,
            18,
            [231, 226, 214],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            148,
            y,
            280,
            4,
            [75, 71, 63, 255],
        );
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            148,
            y,
            usize::from(value) * 28 / 10,
            4,
            if slider == selected {
                [198, 155, 79, 255]
            } else {
                [146, 124, 84, 255]
            },
        );
        let thumb_x = 142 + usize::from(value) * 28 / 10;
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            thumb_x,
            y - 8,
            14,
            20,
            if slider == selected {
                [236, 213, 159, 255]
            } else {
                [203, 197, 180, 255]
            },
        );
        let value_label =
            if matches!(slider, OptionControl::HpWarning | OptionControl::MpWarning) && value == 0 {
                "Off".to_owned()
            } else {
                format!("{value}%")
            };
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &value_label,
            438,
            y as i32 - 12,
            13.0,
            36,
            18,
            [231, 226, 214],
        );
    }

    for (control, label, y, muted) in [
        (
            OptionControl::MuteMusic,
            "Mute",
            OptionControl::Music.track_y().expect("music has a slider"),
            settings.mute_music,
        ),
        (
            OptionControl::MuteEffects,
            "Mute",
            OptionControl::SoundEffects
                .track_y()
                .expect("sound effects have a slider"),
            settings.mute_effects,
        ),
    ] {
        draw_option_checkbox(
            &mut pixels,
            WIDTH,
            HEIGHT,
            486,
            y.saturating_sub(9),
            selected == control,
            muted,
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            512,
            y as i32 - 12,
            12.0,
            60,
            18,
            [231, 226, 214],
        );
    }

    let shake_selected = selected == OptionControl::ScreenShake;
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        24,
        270,
        22,
        22,
        if shake_selected {
            [236, 213, 159, 255]
        } else {
            [136, 125, 104, 255]
        },
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        27,
        273,
        16,
        16,
        [33, 31, 28, 255],
    );
    if gameplay_settings.screen_shake_enabled {
        fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, 30, 281, 5, 3, [232, 205, 140, 255]);
        fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, 34, 278, 3, 7, [232, 205, 140, 255]);
        fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, 37, 275, 3, 4, [232, 205, 140, 255]);
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Screen shake",
        56,
        272,
        13.0,
        180,
        18,
        [231, 226, 214],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        if gameplay_settings.screen_shake_enabled {
            "On"
        } else {
            "Off"
        },
        438,
        272,
        13.0,
        36,
        18,
        [231, 226, 214],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Windowed",
        56,
        305,
        13.0,
        90,
        18,
        [231, 226, 214],
    );
    draw_option_checkbox(
        &mut pixels,
        WIDTH,
        HEIGHT,
        24,
        302,
        selected == OptionControl::Windowed,
        !display_settings.fullscreen,
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Resolution",
        174,
        305,
        13.0,
        88,
        18,
        [231, 226, 214],
    );
    for (control, width, height, label, x) in [
        (
            OptionControl::Resolution800,
            800,
            600,
            "800 x 600",
            268,
        ),
        (
            OptionControl::Resolution1024,
            1024,
            768,
            "1024 x 768",
            398,
        ),
    ] {
        let active = display_settings.width == width && display_settings.height == height;
        draw_option_checkbox(
            &mut pixels,
            WIDTH,
            HEIGHT,
            x,
            302,
            selected == control,
            active,
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            (x + 25) as i32,
            305,
            12.0,
            105,
            18,
            [231, 226, 214],
        );
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Quality limits short-lived effects and map layers.",
        24,
        335,
        11.0,
        WIDTH - 48,
        16,
        [190, 185, 173],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        18,
        365,
        WIDTH - 36,
        1,
        [101, 88, 66, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Social permissions",
        24,
        372,
        14.0,
        WIDTH - 48,
        20,
        [240, 224, 191],
    );
    for (index, (control, label, allowed)) in [
        (OptionControl::AllowWhisper, "Whispers", social_settings.allow_whisper),
        (
            OptionControl::AllowChatInvite,
            "Messenger invitations",
            social_settings.allow_chat_invite,
        ),
        (
            OptionControl::AllowPartyInvite,
            "Party invitations",
            social_settings.allow_party_invite,
        ),
        (OptionControl::AllowGuildChat, "Guild chat", social_settings.allow_guild_chat),
        (
            OptionControl::AllowAllianceChat,
            "Alliance chat",
            social_settings.allow_alliance_chat,
        ),
        (
            OptionControl::AllowFamilyInvite,
            "Family invitations",
            social_settings.allow_family_invite,
        ),
        (
            OptionControl::AllowFriendChat,
            "Buddy chat",
            social_settings.allow_friend_chat,
        ),
        (
            OptionControl::AllowTradeRequest,
            "Trade requests",
            social_settings.allow_trade_request,
        ),
        (
            OptionControl::AllowExpeditionInvite,
            "Expedition invitations",
            social_settings.allow_expedition_invite,
        ),
        (
            OptionControl::AllowGuildInvite,
            "Guild invitations",
            social_settings.allow_guild_invite,
        ),
        (
            OptionControl::AllowAllianceInvite,
            "Alliance invitations",
            social_settings.allow_alliance_invite,
        ),
        (OptionControl::AllowFollow, "Follow requests", social_settings.allow_follow),
    ]
    .into_iter()
    .enumerate()
    {
        let column = index / 6;
        let row = index % 6;
        let x = 24 + column * 286;
        let y = 401 + row * 25;
        let is_selected = selected == control;
        draw_option_checkbox(&mut pixels, WIDTH, HEIGHT, x, y, is_selected, allowed);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            (x + 28) as i32,
            (y + 2) as i32,
            12.0,
            250,
            18,
            [231, 226, 214],
        );
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "Tab selects a setting; arrows change it and Space toggles permissions.",
        24,
        552,
        11.0,
        WIDTH - 48,
        16,
        [190, 185, 173],
    );
    if let Some(notice) = notice {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            notice,
            24,
            568,
            10.0,
            WIDTH - 48,
            18,
            [255, 180, 150],
        );
    }

    for (x, label, color) in [
        (204, "Save", [88, 68, 45, 255]),
        (312, "Cancel", [65, 63, 57, 255]),
    ] {
        fill_dialogue_rect(&mut pixels, WIDTH, HEIGHT, x, 584, 96, 36, color);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            (x + 22) as i32,
            592,
            13.0,
            70,
            20,
            [246, 231, 199],
        );
    }
    pixels
}
