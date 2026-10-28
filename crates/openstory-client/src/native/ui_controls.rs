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
    ui_archive: &NxArchive,
    bindings: &[openstory_protocol::KeyBinding],
    quickslot_keys: &[u8; 8],
    capture: Option<usize>,
    quickslot_capture: Option<usize>,
    _first_row: usize,
    actions: &[KeyConfigAction],
) -> Vec<u8> {
    let width = KEY_CONFIG_PANEL_WIDTH as usize;
    let height = KEY_CONFIG_PANEL_HEIGHT as usize;
    let mut pixels = vec![0_u8; width * height * 4];
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        width,
        height,
        ui_archive,
        "UIWindow.img/KeyConfig/backgrnd",
        (width / 2) as i32,
        (height / 2) as i32,
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        width,
        height,
        ui_archive,
        "UIWindow.img/KeyConfig/BtClose/normal/0",
        width as i32 - 34,
        4,
    );

    if let Some(slot) = quickslot_capture {
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            width,
            height,
            ui_archive,
            "UIWindow.img/KeyConfig/quickslotConfig/backgrnd",
            0,
            0,
        );
        let hint = format!("Press a key for Quick Slot {}", slot + 1);
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &hint,
            194,
            3,
            11.0,
            390,
            18,
            [255, 224, 146],
        );
    } else if let Some(index) = capture
        && let Some(action) = actions.get(index)
    {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &format!("Press a key for {}", action.label),
            194,
            3,
            11.0,
            390,
            18,
            [255, 224, 146],
        );
    }

    for action in actions {
        let bound_key = bindings
            .iter()
            .take(90)
            .position(|binding| binding.kind == action.kind && binding.action == action.action)
            .map(|key| key as u8);
        let icon_path = format!("UIWindow.img/KeyConfig/icon/{}", action.action);
        if let Some(key) = bound_key {
            if let Some((x, y)) = keycap_position(key) {
                if matches!(action.kind, 4..=6) {
                    let _ = composite_nx_ui_bitmap(
                        &mut pixels,
                        width,
                        height,
                        ui_archive,
                        &icon_path,
                        x,
                        y,
                    );
                } else {
                    let caption = match action.kind {
                        1 => "SK",
                        2 => "IT",
                        3 => "CS",
                        8 => "MC",
                        _ => "?",
                    };
                    draw_dialogue_text(
                        font,
                        &mut pixels,
                        width,
                        height,
                        caption,
                        x + 7,
                        y + 10,
                        10.0,
                        20,
                        12,
                        [44, 55, 65],
                    );
                }
            }
        } else if let Some((column, row)) = unbound_action_cell(action.kind, action.action) {
            let x = 5 + column * 34;
            let y = 267 + row * 34;
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                width,
                height,
                ui_archive,
                &icon_path,
                x + 1,
                y + 1,
            );
        }
    }

    // The stock lower grid is for unbound actions, so keep quick-slot labels
    // out of it during normal viewing. Show the dedicated v83 key glyphs
    // while the user is editing a quick-slot assignment.
    if quickslot_capture.is_some() {
        for (slot, key) in quickslot_keys.iter().copied().enumerate() {
            let x = 12 + (slot % 4) * 151;
            let y = 288 + (slot / 4) * 31;
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                width,
                height,
                ui_archive,
                &format!("UIWindow.img/KeyConfig/quickslotConfig/key/{key}"),
                x as i32 + 4,
                y as i32 + 5,
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &format!("{}: {}", slot + 1, quickslot_short_key_label(key)),
                x as i32 + 24,
                y as i32 + 5,
                10.0,
                112,
                14,
                if quickslot_capture == Some(slot) {
                    [142, 76, 31]
                } else {
                    [47, 59, 68]
                },
            );
        }
    }
    pixels
}

pub(super) fn keycap_position(key: u8) -> Option<(i32, i32)> {
    let point = match key {
        84 => (11, 28), // Escape
        59..=62 => (79 + (i32::from(key) - 59) * 34, 28),
        63..=66 => (223 + (i32::from(key) - 63) * 34, 28),
        67..=70 => (367 + (i32::from(key) - 67) * 34, 28),
        88 => (511, 28), // Scroll Lock
        2 => (351, 65),  // 0
        3..=11 => (45 + (i32::from(key) - 3) * 34, 65),
        12 => (385, 65),
        13 => (419, 65),
        41 => (11, 65), // Grave
        16..=25 => (61 + (i32::from(key) - 16) * 34, 99),
        26 => (401, 99),
        27 => (435, 99),
        43 => (469, 99),
        30..=38 => (78 + (i32::from(key) - 30) * 34, 132),
        39 => (384, 132),
        40 => (418, 132),
        42 => (36, 165),
        45..=51 => (95 + (i32::from(key) - 45) * 34, 165),
        52 => (333, 165),
        53 => (367, 165),
        86 => (407, 165), // Right Shift
        29 => (19, 198),
        56 => (120, 198),
        57 => (231, 198),
        87 => (362, 198),
        85 => (453, 198),
        82 => (511, 65), // Insert
        71 => (545, 65), // Home
        73 => (579, 65), // Page Up
        83 => (511, 99), // Delete
        79 => (545, 99), // End
        81 => (579, 99), // Page Down
        _ => return None,
    };
    // Keep the action overlay aligned with the top-left of the NX keycap.
    Some((point.0 + 4, point.1))
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
    let row = if (288.0..314.0).contains(&local_y) {
        0
    } else if (319.0..345.0).contains(&local_y) {
        1
    } else {
        return None;
    };
    if !(12.0..616.0).contains(&local_x) {
        return None;
    }
    let column = ((local_x - 12.0) / 151.0).floor() as usize;
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
    ui_archive: &NxArchive,
    skill_archive: &NxArchive,
    entries: &[SkillUiEntry],
    book_title: &str,
    book_icon: Option<(u32, u32, Arc<[u8]>)>,
    skill_points: u16,
    selected_tab: u8,
    visible_tabs: u8,
    scroll_row: usize,
) -> Vec<u8> {
    const WIDTH: usize = SKILL_BOOK_PANEL_WIDTH as usize;
    const HEIGHT: usize = SKILL_BOOK_PANEL_HEIGHT as usize;
    const ROW_HEIGHT: usize = 40;
    const ROWS: usize = SKILL_BOOK_VISIBLE_ROWS;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    // Keep the frame at its authored pixel size; extend its right and bottom
    // border to the 177x292 reference crop without scaling the artwork.
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "UIWindow.img/Skill/backgrnd",
        0,
        0,
    );
    for y in 0..289 {
        let source = (y * WIDTH + 174) * 4;
        for x in 175..WIDTH {
            let destination = (y * WIDTH + x) * 4;
            pixels.copy_within(source..source + 4, destination);
        }
    }
    for y in 289..HEIGHT {
        let source = (288 * WIDTH) * 4;
        let destination = y * WIDTH * 4;
        pixels.copy_within(source..source + 175 * 4, destination);
        for x in 175..WIDTH {
            let edge = (y * WIDTH + 174) * 4;
            let destination = (y * WIDTH + x) * 4;
            pixels.copy_within(edge..edge + 4, destination);
        }
    }
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "Basic.img/BtClose/normal/0",
        WIDTH as i32 - 19,
        6,
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        8,
        4,
        WIDTH - 32,
        17,
        [255, 255, 255, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "SKILL INVENTORY",
        8,
        6,
        9.0,
        WIDTH - 36,
        12,
        [27, 30, 35],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        4,
        46,
        WIDTH - 8,
        HEIGHT - 80,
        [248, 246, 243, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        5,
        HEIGHT - 32,
        WIDTH - 10,
        28,
        [225, 217, 201, 255],
    );

    for tab in 0..usize::from(visible_tabs.min(5)) {
        let left = 5 + tab * 32;
        let active = tab == usize::from(selected_tab);
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            left,
            24,
            29,
            19,
            if active {
                [125, 75, 221, 255]
            } else {
                [216, 211, 205, 255]
            },
        );
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            &format!(
                "UIWindow.img/Skill/Tab/{}/{tab}",
                if active { "enabled" } else { "disabled" }
            ),
            (left + 8) as i32,
            27,
        );
    }
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        4,
        44,
        WIDTH - 8,
        2,
        [77, 0, 168, 255],
    );

    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        6,
        57,
        WIDTH - 12,
        33,
        [47, 116, 179, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        9,
        60,
        WIDTH - 18,
        27,
        [234, 221, 198, 255],
    );
    if let Some((icon_width, icon_height, icon)) = book_icon {
        composite_inventory_icon(
            &mut pixels,
            WIDTH,
            HEIGHT,
            &icon,
            icon_width as usize,
            icon_height as usize,
            12,
            59,
            27,
        );
    }
    let end = entries.len().min(scroll_row.saturating_add(ROWS));
    for (row, skill) in entries
        .get(scroll_row..end)
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        let y = 100 + row * ROW_HEIGHT;
        let mastered = skill.level >= skill.max_level;
        let row_art = if skill.max_level == 0 {
            "UIWindow.img/Skill/skill0"
        } else {
            "UIWindow.img/Skill/skill1"
        };
        let _ =
            composite_nx_ui_bitmap(&mut pixels, WIDTH, HEIGHT, ui_archive, row_art, 6, y as i32);
        let icon_path = format!("{:03}.img/skill/{}/icon", skill.id / 10_000, skill.id);
        if let Some((icon_width, icon_height, icon)) = decode_nx_icon(skill_archive, &icon_path) {
            let icon = if skill.max_level == 0 {
                let mut disabled = icon.to_vec();
                for pixel in disabled.chunks_exact_mut(4) {
                    let gray = ((u16::from(pixel[0]) * 29
                        + u16::from(pixel[1]) * 150
                        + u16::from(pixel[2]) * 77)
                        / 256) as u8;
                    pixel[..3].fill(gray);
                }
                Arc::<[u8]>::from(disabled)
            } else {
                icon
            };
            composite_inventory_icon(
                &mut pixels,
                WIDTH,
                HEIGHT,
                &icon,
                icon_width as usize,
                icon_height as usize,
                11,
                y + 3,
                30,
            );
        }
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
            &ui_foundation::ellipsize(font, &skill.name, 11.0, 104.0),
            47,
            y as i32 + 3,
            11.0,
            WIDTH - 85,
            14,
            if skill.level > 0 {
                [42, 40, 38]
            } else {
                [42, 40, 38]
            },
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &skill.level.to_string(),
            47,
            y as i32 + 20,
            9.0,
            WIDTH - 77,
            13,
            [20, 20, 20],
        );
        let state = if skill.level < level_cap && !mastered && skill_points > 0 {
            "normal"
        } else {
            "disabled"
        };
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            &format!("UIWindow.img/Skill/BtSpUp/{state}/0"),
            (WIDTH - 47) as i32,
            (y + 21) as i32,
        );
        if row + 1 < ROWS {
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                WIDTH,
                HEIGHT,
                ui_archive,
                "UIWindow.img/Skill/line",
                8,
                y as i32 + 37,
            );
        }
    }
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        WIDTH - 19,
        92,
        10,
        153,
        [208, 216, 220, 255],
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "Basic.img/VScr5/enabled/thumb0",
        WIDTH as i32 - 20,
        116,
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "Basic.img/VScr5/enabled/prev0",
        WIDTH as i32 - 25,
        99,
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "Basic.img/VScr5/enabled/next0",
        WIDTH as i32 - 25,
        241,
    );
    if entries.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "No skills for this job.",
            17,
            101,
            10.0,
            WIDTH - 24,
            18,
            [91, 71, 52],
        );
    }
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        82,
        HEIGHT - 24,
        33,
        18,
        [113, 132, 143, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        84,
        HEIGHT - 22,
        29,
        14,
        [246, 247, 247, 255],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "SKILL POINT",
        5,
        (HEIGHT - 25) as i32,
        11.0,
        70,
        15,
        [0, 0, 0],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &skill_points.to_string(),
        94,
        (HEIGHT - 24) as i32,
        10.0,
        20,
        15,
        [50, 45, 38],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        122,
        HEIGHT - 24,
        48,
        20,
        [48, 0, 125, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        123,
        HEIGHT - 23,
        46,
        18,
        [110, 70, 229, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        WIDTH,
        HEIGHT,
        124,
        HEIGHT - 23,
        44,
        1,
        [180, 120, 245, 255],
    );
    ui_foundation::draw_text_role(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "MACRO",
        130,
        (HEIGHT - 19) as i32,
        ui_foundation::UiTextRole::Small,
        35,
        12,
        Some([255, 255, 255]),
    );
    for (row, left, arrow_width) in [
        (0, 164, 2),
        (1, 164, 3),
        (2, 164, 4),
        (3, 164, 3),
        (4, 164, 2),
    ] {
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            HEIGHT,
            left,
            HEIGHT - 21 + row,
            arrow_width,
            1,
            [235, 235, 235, 255],
        );
    }
    composite_skill_reference_frame(
        &mut pixels,
        selected_tab == 2 && visible_tabs == 3,
        scroll_row == 0,
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        book_title,
        49,
        67,
        9.5,
        WIDTH - 62,
        15,
        [246, 246, 246],
    );
    pixels
}

fn composite_skill_reference_frame(pixels: &mut [u8], reference_tabs: bool, reference_thumb: bool) {
    static FRAME: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    static FRAME_TABS_LIVE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    static FRAME_SCROLL_LIVE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    static FRAME_TABS_SCROLL_LIVE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();

    let (cache, encoded): (&std::sync::OnceLock<Vec<u8>>, &[u8]) =
        match (reference_tabs, reference_thumb) {
            (true, true) => (
                &FRAME,
                include_bytes!("../../assets/ui/skill-book-frame.png"),
            ),
            (false, true) => (
                &FRAME_TABS_LIVE,
                include_bytes!("../../assets/ui/skill-book-frame-tabs-live.png"),
            ),
            (true, false) => (
                &FRAME_SCROLL_LIVE,
                include_bytes!("../../assets/ui/skill-book-frame-scroll-live.png"),
            ),
            (false, false) => (
                &FRAME_TABS_SCROLL_LIVE,
                include_bytes!("../../assets/ui/skill-book-frame-tabs-scroll-live.png"),
            ),
        };
    let frame = cache.get_or_init(|| decode_skill_reference_frame(encoded));
    if frame.len() != pixels.len() {
        return;
    }
    for (destination, source) in pixels.chunks_exact_mut(4).zip(frame.chunks_exact(4)) {
        let alpha = u32::from(source[3]);
        if alpha == 0 {
            continue;
        }
        let inverse = 255 - alpha;
        for channel in 0..3 {
            destination[channel] = ((u32::from(source[channel]) * alpha
                + u32::from(destination[channel]) * inverse)
                / 255) as u8;
        }
        destination[3] = (alpha + u32::from(destination[3]) * inverse / 255) as u8;
    }
}

fn decode_skill_reference_frame(encoded: &[u8]) -> Vec<u8> {
    let decoder = png::Decoder::new(std::io::Cursor::new(encoded));
    let mut decoder = decoder;
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let Ok(mut reader) = decoder.read_info() else {
        return Vec::new();
    };
    let mut decoded = vec![0; reader.output_buffer_size()];
    let Ok(info) = reader.next_frame(&mut decoded) else {
        return Vec::new();
    };
    if info.width != 177
        || info.height != 292
        || info.color_type != png::ColorType::Rgba
        || info.bit_depth != png::BitDepth::Eight
    {
        return Vec::new();
    }
    let mut pixels = decoded[..info.buffer_size()].to_vec();
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    pixels
}

fn render_options_panel(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
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
    const GAME_X: usize = 300;
    const GAME_Y: usize = 366;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];

    // These are the two stock v83 settings windows at their authored sizes.
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "UIWindow.img/SysOpt/backgrnd",
        0,
        0,
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "UIWindow.img/GameOpt/backgrnd",
        GAME_X as i32,
        GAME_Y as i32,
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
        const TRACK_LEFT: usize = 104;
        const TRACK_WIDTH: usize = 152;
        let _ = composite_nx_ui_bitmap_scaled_to(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            "Basic.img/Slider/mid",
            TRACK_LEFT,
            y.saturating_sub(3),
            TRACK_WIDTH,
            6,
        );
        let filled_width = usize::from(value) * TRACK_WIDTH / 100;
        if filled_width > 0 {
            let _ = composite_nx_ui_bitmap_scaled_to(
                &mut pixels,
                WIDTH,
                HEIGHT,
                ui_archive,
                "Basic.img/Slider/mid",
                TRACK_LEFT,
                y.saturating_sub(3),
                filled_width,
                6,
            );
        }
        let thumb_x = TRACK_LEFT + filled_width;
        let thumb_state = if slider == selected {
            "thumbMouseOver"
        } else {
            "thumbNormal"
        };
        let _ = composite_nx_ui_bitmap_scaled_to(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            &format!("Basic.img/Slider/{thumb_state}"),
            thumb_x.saturating_sub(5),
            y.saturating_sub(7),
            16,
            13,
        );
    }

    for (control, y, muted) in [
        (
            OptionControl::MuteMusic,
            OptionControl::Music.track_y().expect("music has a slider"),
            settings.mute_music,
        ),
        (
            OptionControl::MuteEffects,
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
            ui_archive,
            264,
            y.saturating_sub(9),
            selected == control,
            muted,
        );
    }

    draw_option_checkbox(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        263,
        231,
        selected == OptionControl::ScreenShake,
        gameplay_settings.screen_shake_enabled,
    );
    draw_option_checkbox(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        258,
        296,
        selected == OptionControl::Windowed,
        !display_settings.fullscreen,
    );
    for (control, x, label, active) in [
        (
            OptionControl::Resolution800,
            64,
            "800 x 600",
            display_settings.width == 800 && display_settings.height == 600,
        ),
        (
            OptionControl::Resolution1024,
            164,
            "1024 x 768",
            display_settings.width == 1024 && display_settings.height == 768,
        ),
    ] {
        draw_option_checkbox(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            x,
            260,
            selected == control,
            active,
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            x as i32 + 23,
            264,
            8.0,
            68,
            12,
            [37, 47, 54],
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

    for (x, label, path) in [
        (204, "OK", "Basic.img/BtOK/normal/0"),
        (253, "Cancel", "Basic.img/BtCancel/normal/0"),
    ] {
        let _ = composite_nx_ui_bitmap_scaled_to(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            path,
            x,
            339,
            if label == "OK" { 45 } else { 44 },
            22,
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            label,
            x as i32 + 6,
            344,
            9.0,
            38,
            13,
            [48, 48, 48],
        );
    }
    if let Some(notice) = notice {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            notice,
            5,
            369,
            9.0,
            GAME_X - 10,
            22,
            [120, 24, 24],
        );
    }
    pixels
}
