//! Native-size keyboard settings. Rendering and input share key, tray and button geometry.
use super::ui_foundation::{UiGrid, UiRect};
use super::*;

pub(super) const TRAY: UiGrid = UiGrid {
    x: 9.0,
    y: 270.0,
    columns: 18,
    rows: 2,
    step_x: 34.0,
    step_y: 34.0,
    cell_width: 32.0,
    cell_height: 32.0,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Button {
    Ok,
    Cancel,
    Default,
    Clear,
    Close,
    Quickslots,
}

pub(super) fn buttons() -> [(Button, UiRect, &'static str); 6] {
    [
        (Button::Ok, UiRect::new(10.0, 240.0, 47.0, 18.0), "BtOK"),
        (
            Button::Quickslots,
            UiRect::new(505.0, 240.0, 96.0, 18.0),
            "BtQuickSlot",
        ),
        (
            Button::Cancel,
            UiRect::new(61.0, 240.0, 47.0, 18.0),
            "BtCancel",
        ),
        (
            Button::Default,
            UiRect::new(113.0, 240.0, 60.0, 18.0),
            "BtDefault",
        ),
        (
            Button::Clear,
            UiRect::new(178.0, 240.0, 77.0, 18.0),
            "BtDelete",
        ),
        (
            Button::Close,
            ui_dialog::close_button(607.0, 10.0),
            "BtClose",
        ),
    ]
}

pub(super) fn button_at(x: f32, y: f32) -> Option<Button> {
    buttons()
        .into_iter()
        .find(|(_, rect, _)| rect.contains(x, y))
        .map(|(button, _, _)| button)
}

/// Wire key numbers follow the existing input translator, including its number-row ordering.
pub(super) fn key_rect(key: u8) -> Option<UiRect> {
    let (x, y, width) = match key {
        84 => (11, 29, 32),
        59..=62 => (79 + (i32::from(key) - 59) * 34, 29, 32),
        63..=66 => (223 + (i32::from(key) - 63) * 34, 29, 32),
        67..=70 => (367 + (i32::from(key) - 67) * 34, 29, 32),
        88 => (545, 29, 32),
        2 => (351, 65, 32),
        3..=11 => (45 + (i32::from(key) - 3) * 34, 65, 32),
        12 => (385, 65, 32),
        13 => (419, 65, 32),
        41 => (11, 65, 32),
        16..=25 => (61 + (i32::from(key) - 16) * 34, 99, 32),
        26 => (401, 99, 32),
        27 => (435, 99, 32),
        43 => (469, 99, 32),
        30..=38 => (78 + (i32::from(key) - 30) * 34, 132, 32),
        39 => (384, 132, 32),
        40 => (418, 132, 32),
        42 => (11, 165, 81),
        44..=53 => (95 + (i32::from(key) - 44) * 34, 165, 32),
        86 => (435, 165, 67),
        29 => (11, 198, 48),
        56 => (112, 198, 50),
        57 => (166, 198, 167),
        87 => (336, 198, 50),
        85 => (447, 198, 55),
        82 => (511, 65, 32),
        71 => (545, 65, 32),
        73 => (579, 65, 32),
        83 => (511, 99, 32),
        79 => (545, 99, 32),
        81 => (579, 99, 32),
        _ => return None,
    };
    Some(UiRect::new(
        (x + 3) as f32,
        (y + 3) as f32,
        width as f32,
        32.0,
    ))
}

fn key_content_rect(key: u8, rect: UiRect) -> UiRect {
    let top_row = matches!(key, 59..=70 | 84 | 88);
    UiRect::new(
        rect.x + 1.0,
        rect.y - 2.0 * f32::from(top_row),
        rect.width,
        rect.height,
    )
}

pub(super) fn key_at(x: f32, y: f32) -> Option<u8> {
    (0..90).find(|key| key_rect(*key).is_some_and(|rect| rect.contains(x, y)))
}

fn preferred_cell(kind: u8, action: i32) -> Option<usize> {
    // Stable slots are retained while bound actions leave an empty tray cell.
    if kind == 4 && action == 6 {
        return Some(6);
    }
    if kind == 5 && action == 51 {
        return Some(18 + 4);
    }
    if kind == 5 && action == 52 {
        return Some(18 + 5);
    }
    super::unbound_action_cell(kind, action)
        .map(|(column, row)| row as usize * 18 + column as usize)
}

pub(super) fn tray_entries(
    actions: &[KeyConfigAction],
    bindings: &[openstory_protocol::KeyBinding],
) -> Vec<(usize, usize)> {
    let unbound = actions
        .iter()
        .enumerate()
        .filter(|(_, action)| {
            !bindings
                .iter()
                .any(|binding| binding.kind == action.kind && binding.action == action.action)
        })
        .collect::<Vec<_>>();
    let mut occupied = HashSet::new();
    let mut entries = Vec::new();
    let mut overflow = Vec::new();
    for (index, action) in unbound {
        if let Some(cell) =
            preferred_cell(action.kind, action.action).filter(|cell| occupied.insert(*cell))
        {
            entries.push((index, cell));
        } else {
            overflow.push(index);
        }
    }
    let mut next = 0;
    for index in overflow {
        while occupied.contains(&next) {
            next += 1;
        }
        occupied.insert(next);
        entries.push((index, next));
    }
    entries
}

pub(super) fn scroll_max(
    actions: &[KeyConfigAction],
    bindings: &[openstory_protocol::KeyBinding],
) -> usize {
    tray_entries(actions, bindings)
        .iter()
        .map(|(_, cell)| cell / 18 + 1)
        .max()
        .unwrap_or(0)
        .saturating_sub(2)
}

pub(super) fn tray_action_at(
    x: f32,
    y: f32,
    first_row: usize,
    actions: &[KeyConfigAction],
    bindings: &[openstory_protocol::KeyBinding],
) -> Option<usize> {
    let cell = TRAY.hit(x, y)? + first_row * 18;
    tray_entries(actions, bindings)
        .into_iter()
        .find(|(_, slot)| *slot == cell)
        .map(|(index, _)| index)
}

pub(super) const QUICKSLOT_POPUP: UiRect = UiRect::new(184.0, 30.0, 266.0, 238.0);

pub(super) fn quickslot_rect(slot: usize) -> UiRect {
    UiRect::new(
        QUICKSLOT_POPUP.x + 64.0 + (slot % 4) as f32 * 35.0,
        QUICKSLOT_POPUP.y + 113.0 + (slot / 4) as f32 * 34.0,
        32.0,
        30.0,
    )
}

pub(super) fn quickslot_at(x: f32, y: f32) -> Option<usize> {
    (0..8).find(|slot| quickslot_rect(*slot).contains(x, y))
}

fn blit_crop(
    dst: &mut [u8],
    source: &[u8],
    source_width: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    dx: i32,
    dy: i32,
) {
    let mut crop = Vec::with_capacity(width * height * 4);
    for row in y..y + height {
        crop.extend_from_slice(
            &source[(row * source_width + x) * 4..(row * source_width + x + width) * 4],
        );
    }
    composite_bgra_pixels(
        dst,
        KEY_CONFIG_PANEL_WIDTH as usize,
        KEY_CONFIG_PANEL_HEIGHT as usize,
        &crop,
        width,
        height,
        dx,
        dy,
    );
}

fn blank_key(pixels: &mut [u8], source: &[u8], rect: UiRect, scroll_lock: bool) {
    // Use the bare "2" key, or the neighboring Break cap in the top-right cluster.
    let (source_x, source_y, left_edge, blank_start, blank_width) = if scroll_lock {
        (580, 27, 3, 27, 2)
    } else {
        (80, 66, 5, 16, 12)
    };
    const CAP_WIDTH: usize = 34;
    const EDGE: usize = 5;
    let width = rect.width as usize;
    for column in 0..width {
        let source_column = if column < left_edge {
            column
        } else if column >= width - EDGE {
            CAP_WIDTH - (width - column)
        } else {
            blank_start + (column - left_edge) % blank_width
        };
        blit_crop(
            pixels,
            source,
            629,
            source_x + source_column,
            source_y,
            1,
            32,
            rect.x as i32 + column as i32,
            rect.y as i32,
        );
    }
}

fn legend(
    font: &fontdue::Font,
    pixels: &mut [u8],
    ui: &NxArchive,
    key: u8,
    rect: UiRect,
    center: bool,
) {
    let asset_key = match key {
        2 => 11,
        3..=11 => key - 1,
        86 => 42,
        85 => 29,
        87 => 56,
        _ => key,
    };
    let path = format!("UIWindow.img/KeyConfig/key/{asset_key}");
    if let Some((width, height, image)) = (!matches!(key, 84 | 88))
        .then(|| decode_nx_icon(ui, &path))
        .flatten()
    {
        let x = if center {
            rect.x + (rect.width - width as f32) * 0.5
        } else {
            rect.x + 3.0
        };
        composite_bgra_pixels(
            pixels,
            KEY_CONFIG_PANEL_WIDTH as usize,
            KEY_CONFIG_PANEL_HEIGHT as usize,
            &image,
            width as usize,
            height as usize,
            x.round() as i32,
            rect.y as i32 + 2,
        );
    } else {
        // Use short, readable labels for Escape and Scroll Lock.
        let label = match key {
            84 => "Esc".to_owned(),
            88 => "Slk".to_owned(),
            _ => super::maple_key_label(key),
        };
        let label_x = rect.x as i32 + if key == 88 { 2 } else { 3 };
        draw_dialogue_text(
            font,
            pixels,
            KEY_CONFIG_PANEL_WIDTH as usize,
            KEY_CONFIG_PANEL_HEIGHT as usize,
            &label,
            label_x,
            rect.y as i32 + 2,
            10.0,
            rect.width as usize - 6,
            13,
            [150, 155, 160],
        );
    }
}

// Extension actions have no NX icon. Reuse a native action tile's frame and
// palette, with bottom-aligned pixel lettering matching the shipped key icons.
fn label_lines(label: &str) -> Vec<String> {
    let label = label
        .split(" (")
        .next()
        .unwrap_or(label)
        .chars()
        .map(|ch| {
            if ch.is_ascii() {
                ch.to_ascii_uppercase()
            } else {
                '?'
            }
        })
        .collect::<String>();
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in label.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > 7 {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        for ch in word.chars() {
            if line.len() == 7 {
                lines.push(std::mem::take(&mut line));
            }
            line.push(ch);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines.truncate(3);
    lines
}

fn pixel_letter(ch: char) -> [u8; 5] {
    match ch {
        'A' => [2, 5, 7, 5, 5],
        'B' => [6, 5, 6, 5, 6],
        'C' => [3, 4, 4, 4, 3],
        'D' => [6, 5, 5, 5, 6],
        'E' => [7, 4, 6, 4, 7],
        'F' => [7, 4, 6, 4, 4],
        'G' => [3, 4, 5, 5, 3],
        'H' => [5, 5, 7, 5, 5],
        'I' => [7, 2, 2, 2, 7],
        'J' => [1, 1, 1, 5, 2],
        'K' => [5, 5, 6, 5, 5],
        'L' => [4, 4, 4, 4, 7],
        'M' => [5, 7, 7, 5, 5],
        'N' => [5, 7, 7, 7, 5],
        'O' => [2, 5, 5, 5, 2],
        'P' => [6, 5, 6, 4, 4],
        'Q' => [2, 5, 5, 3, 1],
        'R' => [6, 5, 6, 5, 5],
        'S' => [3, 4, 2, 1, 6],
        'T' => [7, 2, 2, 2, 2],
        'U' => [5, 5, 5, 5, 7],
        'V' => [5, 5, 5, 5, 2],
        'W' => [5, 5, 7, 7, 5],
        'X' => [5, 5, 2, 5, 5],
        'Y' => [5, 5, 2, 2, 2],
        'Z' => [7, 1, 2, 4, 7],
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [6, 1, 2, 4, 7],
        '3' => [6, 1, 2, 1, 6],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 6, 1, 6],
        '6' => [3, 4, 6, 5, 2],
        '7' => [7, 1, 2, 2, 2],
        '8' => [2, 5, 2, 5, 2],
        '9' => [2, 5, 3, 1, 6],
        '-' => [0, 0, 7, 0, 0],
        ' ' => [0; 5],
        _ => [7, 1, 2, 0, 2],
    }
}

fn native_label_tile(template: &[u8], label: &str) -> Vec<u8> {
    let mut pixels = template.to_vec();
    let background: [u8; 4] = template[(5 * 32 + 16) * 4..(5 * 32 + 17) * 4]
        .try_into()
        .unwrap();
    // Preserve the source's rounded corners, highlight, outline and bottom bevel.
    for y in 3..29 {
        for x in 3..31 {
            pixels[(y * 32 + x) * 4..(y * 32 + x + 1) * 4].copy_from_slice(&background);
        }
    }
    let lines = label_lines(label);
    let top = 28 - lines.len() * 6;
    for (row, line) in lines.iter().enumerate() {
        let x = 3 + (28 - (line.len() * 4 - 1)) / 2;
        let y = top + row * 6;
        // Native labels use crisp white glyphs with a dark one-pixel shadow.
        for (dy, color) in [(1, [34, 17, 51, 255]), (0, [255; 4])] {
            for (letter, ch) in line.chars().enumerate() {
                for (gy, bits) in pixel_letter(ch).into_iter().enumerate() {
                    for gx in 0..3 {
                        if bits & (1 << (2 - gx)) != 0 {
                            let index = ((y + gy + dy) * 32 + x + letter * 4 + gx) * 4;
                            pixels[index..index + 4].copy_from_slice(&color);
                        }
                    }
                }
            }
        }
    }
    pixels
}

fn extension_action_icon(
    ui: &NxArchive,
    kind: u8,
    action: i32,
    label: &str,
) -> Option<(u32, u32, Arc<[u8]>)> {
    if kind == 4 && action == 42 {
        return decode_nx_icon(ui, "UIWindow.img/KeyConfig/icon/26");
    }
    let (width, height, template) = decode_nx_icon(ui, "UIWindow.img/KeyConfig/icon/10")?;
    if width != 32 || height != 32 || template.len() != 32 * 32 * 4 {
        return None;
    }
    let label = match (kind, action) {
        (4, 44) => "Alli ance",
        (4, 45) => "Pet equip",
        _ => label,
    };
    Some((32, 32, Arc::from(native_label_tile(&template, label))))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    font: &fontdue::Font,
    ui: &NxArchive,
    skills: &NxArchive,
    items: &NxArchive,
    characters: &NxArchive,
    bindings: &[openstory_protocol::KeyBinding],
    quickslots: &[u8; 8],
    capture: Option<usize>,
    quickslot_capture: Option<usize>,
    first_row: usize,
    actions: &[KeyConfigAction],
    active_skill_ids: &HashSet<i32>,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    item_counts: &HashMap<i32, u32>,
) -> Vec<u8> {
    let (width, height) = (
        KEY_CONFIG_PANEL_WIDTH as usize,
        KEY_CONFIG_PANEL_HEIGHT as usize,
    );
    let mut pixels = vec![0; width * height * 4];
    let mut dialog = NxDialog::new(&mut pixels, width, height, ui);
    let background = dialog
        .frame_bitmap("UIWindow.img/KeyConfig/backgrnd")
        .filter(|(width, height, _)| (*width, *height) == (629, 373))
        .map(|(_, _, pixels)| pixels);
    if let Some(source) = background.as_ref() {
        // Lower the top key row one pixel, omit a tray row and retain the authored bottom border.
        let _ = dialog.frame_rows(
            source,
            629,
            3,
            &[
                (0..28, 3),
                (60..335, 63),
                (27..28, 31),
                (28..60, 32),
                (369..373, 338),
            ],
        );
        dialog.replace_title(
            font,
            "Keyboard Settings",
            UiRect::new(13.0, 9.0, 177.0, 12.0),
        );
        // Replace the baked shortcut tiles and Shift legends at their actual
        // bitmap bounds so the old bevel cannot peek through around the edges.
        for (rect, scroll_lock) in [
            (UiRect::new(15.0, 31.0, 34.0, 32.0), false),   // Esc
            (UiRect::new(15.0, 102.0, 49.0, 32.0), false),  // Tab
            (UiRect::new(549.0, 31.0, 34.0, 32.0), true),   // Slk
            (UiRect::new(15.0, 168.0, 83.0, 32.0), false),  // Left Shift
            (UiRect::new(439.0, 168.0, 68.0, 32.0), false), // Right Shift
        ] {
            blank_key(&mut pixels, source, rect, scroll_lock);
        }
        for key in [84, 88, 42, 86] {
            let rect = key_content_rect(key, key_rect(key).unwrap());
            legend(font, &mut pixels, ui, key, rect, matches!(key, 42 | 86));
        }
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Tab",
            19,
            103,
            10.0,
            38,
            13,
            [160, 160, 160],
        );
    }
    let mut draw_icon = |pixels: &mut [u8], kind: u8, action: i32, x: i32, y: i32| {
        let icon = match kind {
            1 => decode_nx_icon(
                skills,
                &format!("{:03}.img/skill/{action}/icon", action / 10_000),
            ),
            2 | 3 => inventory_item_icon(action, items, characters, icon_cache),
            8 => decode_nx_icon(
                ui,
                &format!("UIWindow.img/SkillMacro/Macroicon/{action}/icon"),
            ),
            _ => decode_nx_icon(ui, &format!("UIWindow.img/KeyConfig/icon/{action}")),
        }
        .or_else(|| {
            actions
                .iter()
                .find(|entry| entry.kind == kind && entry.action == action)
                .and_then(|entry| extension_action_icon(ui, kind, action, &entry.label))
        });
        if let Some((iw, ih, image)) = icon {
            let mut cell = vec![0; 32 * 32 * 4];
            composite_inventory_icon(
                &mut cell,
                32,
                32,
                &image,
                iw as usize,
                ih as usize,
                0,
                0,
                32,
            );
            composite_bgra_pixels(pixels, width, height, &cell, 32, 32, x, y);
        }
    };
    // Iterate physical slots, including duplicate bindings, rather than only the first key for each action.
    for (key, binding) in bindings
        .iter()
        .take(90)
        .enumerate()
        .filter(|(_, binding)| binding.kind != 0)
    {
        let Some(rect) = key_rect(key as u8) else {
            continue;
        };
        if let Some(source) = background.as_ref() {
            blank_key(&mut pixels, source, rect, key == 88);
        }
        let content_rect = key_content_rect(key as u8, rect);
        let x = (content_rect.x + (content_rect.width - 32.0) * 0.5).round() as i32;
        draw_icon(
            &mut pixels,
            binding.kind,
            binding.action,
            x,
            content_rect.y as i32,
        );
        if binding.kind == 1 && active_skill_ids.contains(&binding.action) {
            draw_auto_buff_sparkle(
                &mut pixels,
                width,
                height,
                x as usize + 32,
                content_rect.y as usize + 32,
            );
        }
        legend(
            font,
            &mut pixels,
            ui,
            key as u8,
            content_rect,
            rect.width > 32.0,
        );
        if matches!(binding.kind, 2 | 3)
            && let Some(count) = item_counts.get(&binding.action)
        {
            // Use the same NX glyph widths and fitting as inventory stacks.
            // Fixed 8px advances truncated five-digit counts and spread short
            // counts beyond the icon's left edge.
            let _ = draw_item_stack_count(
                &mut pixels,
                width,
                height,
                ui,
                *count,
                x,
                content_rect.y as i32 + 20,
                32,
            );
        }
    }
    let mut dialog = NxDialog::new(&mut pixels, width, height, ui);
    for (button, rect, name) in buttons() {
        if button == Button::Close {
            let _ = dialog.close(rect);
        } else {
            let art = format!("UIWindow.img/KeyConfig/{name}");
            let _ = dialog.button(
                rect,
                ui_dialog::NxButton::new(&art, ui_dialog::ButtonState::Normal),
            );
        }
    }
    if let Some(slot) = quickslot_capture {
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            width,
            height,
            ui,
            "UIWindow.img/KeyConfig/quickslotConfig/backgrnd",
            QUICKSLOT_POPUP.x as i32,
            QUICKSLOT_POPUP.y as i32,
        );
        for (index, key) in quickslots.iter().enumerate() {
            let rect = quickslot_rect(index);
            let path = format!("UIWindow.img/KeyConfig/quickslotConfig/key/{key}");
            if let Some((glyph_width, glyph_height, _)) = decode_nx_icon(ui, &path) {
                let _ = composite_nx_ui_bitmap(
                    &mut pixels,
                    width,
                    height,
                    ui,
                    &path,
                    rect.x as i32 + (rect.width as i32 - glyph_width as i32) / 2,
                    rect.y as i32 + (rect.height as i32 - glyph_height as i32) / 2,
                );
            } else {
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    width,
                    height,
                    &quickslot_short_key_label(*key),
                    rect.x as i32 + 2,
                    rect.y as i32 + 5,
                    9.0,
                    28,
                    16,
                    [35, 35, 35],
                );
            }
            if slot == index {
                for (x, y, w, h) in [
                    (rect.x as usize, rect.y as usize, 32, 1),
                    (rect.x as usize, rect.y as usize + 29, 32, 1),
                    (rect.x as usize, rect.y as usize, 1, 30),
                    (rect.x as usize + 31, rect.y as usize, 1, 30),
                ] {
                    fill_dialogue_rect(&mut pixels, width, height, x, y, w, h, [35, 180, 245, 255]);
                }
            }
        }
    } else {
        for (index, cell) in tray_entries(actions, bindings) {
            if cell < first_row * 18 || cell >= (first_row + 2) * 18 {
                continue;
            }
            let rect = TRAY.cell(cell - first_row * 18);
            let action = &actions[index];
            draw_icon(
                &mut pixels,
                action.kind,
                action.action,
                rect.x as i32,
                rect.y as i32,
            );
        }
    }
    if let Some(selected) = capture.and_then(|index| actions.get(index)) {
        for (key, _binding) in bindings.iter().enumerate().filter(|(_, binding)| {
            binding.kind == selected.kind && binding.action == selected.action
        }) {
            if let Some(rect) = key_rect(key as u8) {
                for row in 0..rect.height as usize {
                    for column in 0..rect.width as usize {
                        if row == 0
                            || column == 0
                            || row + 1 == rect.height as usize
                            || column + 1 == rect.width as usize
                        {
                            let index =
                                ((rect.y as usize + row) * width + rect.x as usize + column) * 4;
                            pixels[index..index + 4].copy_from_slice(&[35, 180, 245, 255]);
                        }
                    }
                }
            }
        }
    }
    if let Some(action) = capture.and_then(|index| actions.get(index)) {
        let hint = ui_foundation::ellipsize(
            font,
            &format!("Click / press key: {}", action.label),
            10.0,
            230.0,
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &hint,
            267,
            243,
            10.0,
            230,
            14,
            [45, 65, 80],
        );
    }
    pixels
}

pub(super) fn reference_bindings() -> Vec<openstory_protocol::KeyBinding> {
    let mut bindings = vec![openstory_protocol::KeyBinding::default(); 90];
    for (key, kind, action) in [
        (3, 4, 10),
        (4, 4, 18),
        (5, 4, 12),
        (6, 4, 13),
        (16, 4, 8),
        (17, 4, 5),
        (25, 4, 19),
        (26, 4, 14),
        (27, 4, 15),
        (43, 4, 9),
        (35, 4, 11),
        (39, 4, 13),
        (40, 4, 16),
        (44, 5, 50),
        (50, 4, 7),
        (56, 5, 53),
        (87, 5, 53),
        (82, 4, 3),
        (71, 4, 0),
        (73, 4, 4),
        (83, 4, 1),
        (79, 4, 2),
        (81, 4, 17),
        (18, 1, 2111006),
        (19, 1, 2001002),
        (30, 2, 2000002),
        (31, 1, 2101004),
        (32, 1, 2101001),
        (33, 1, 2111005),
        (45, 1, 2111003),
        (46, 1, 2101002),
        (47, 1, 2111002),
        (48, 1, 2111004),
        (29, 1, 2101003),
        (85, 1, 2101003),
        (57, 2, 2000006),
    ] {
        bindings[key] = openstory_protocol::KeyBinding { kind, action };
    }
    for key in 59..=65 {
        bindings[key] = openstory_protocol::KeyBinding {
            kind: 6,
            action: 100 + key as i32 - 59,
        };
    }
    bindings
}

pub(super) fn assign(
    draft: &mut Vec<openstory_protocol::KeyBinding>,
    key: u8,
    value: openstory_protocol::KeyBinding,
) -> bool {
    if key == 0 || key == 84 || usize::from(key) >= openstory_protocol::KEYMAP_ENTRY_COUNT {
        return false;
    }
    draft.resize(
        openstory_protocol::KEYMAP_ENTRY_COUNT,
        openstory_protocol::KeyBinding::default(),
    );
    for (index, binding) in draft.iter_mut().enumerate() {
        if index != usize::from(key) && *binding == value {
            *binding = openstory_protocol::KeyBinding::default();
        }
    }
    draft[usize::from(key)] = value;
    true
}

/// Keep the eight display keys unique by swapping an existing key with the old slot.
pub(super) fn assign_quickslot(keys: &mut [u8; 8], slot: usize, key: u8) -> bool {
    if slot >= keys.len()
        || key == 0
        || key == 84
        || usize::from(key) >= openstory_protocol::KEYMAP_ENTRY_COUNT
    {
        return false;
    }
    let previous = keys[slot];
    for (index, other) in keys.iter_mut().enumerate() {
        if index != slot && *other == key {
            *other = previous;
        }
    }
    keys[slot] = key;
    true
}

pub(super) fn changes(
    previous: &[openstory_protocol::KeyBinding],
    draft: &[openstory_protocol::KeyBinding],
) -> Vec<(u8, openstory_protocol::KeyBinding)> {
    draft
        .iter()
        .take(openstory_protocol::KEYMAP_ENTRY_COUNT)
        .enumerate()
        .filter(|(key, binding)| previous.get(*key).copied().unwrap_or_default() != **binding)
        .map(|(key, binding)| (key as u8, *binding))
        .collect()
}

pub(super) fn defaults() -> Vec<openstory_protocol::KeyBinding> {
    // Classic v83 action IDs, expressed in this client's existing wire-key numbering.
    let mut result =
        vec![openstory_protocol::KeyBinding::default(); openstory_protocol::KEYMAP_ENTRY_COUNT];
    for (key, kind, action) in [
        (3, 4, 10),
        (4, 4, 12),
        (5, 4, 13),
        (6, 4, 18),
        (7, 4, 24),
        (16, 4, 8),
        (17, 4, 5),
        (18, 4, 0),
        (19, 4, 4),
        (22, 4, 44),
        (23, 4, 1),
        (25, 4, 19),
        (26, 4, 14),
        (27, 4, 15),
        (43, 4, 9),
        (31, 4, 2),
        (34, 4, 17),
        (35, 4, 11),
        (37, 4, 3),
        (40, 4, 16),
        (44, 5, 50),
        (45, 5, 51),
        (46, 4, 6),
        (47, 4, 43),
        (50, 4, 7),
        (29, 5, 52),
        (56, 5, 53),
        (57, 5, 54),
        (85, 5, 52),
        (87, 5, 53),
    ] {
        result[key] = openstory_protocol::KeyBinding { kind, action };
    }
    for key in 59..=65 {
        result[key] = openstory_protocol::KeyBinding {
            kind: 6,
            action: 100 + key as i32 - 59,
        };
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use openstory_protocol::KeyBinding;

    #[test]
    fn extension_labels_preserve_the_native_frame_and_fit_inside_the_tile() {
        let mut template = vec![19; 32 * 32 * 4];
        template[(5 * 32 + 16) * 4..(5 * 32 + 17) * 4].copy_from_slice(&[153, 136, 187, 255]);
        for label in [
            "Events",
            "Guild BBS",
            "Alliance",
            "Pet equipment",
            "Family (extended)",
        ] {
            let lines = label_lines(label);
            assert!(lines.len() <= 3);
            assert!(
                lines
                    .iter()
                    .all(|line| line.len() <= 7 && line.chars().all(|ch| !ch.is_lowercase()))
            );
            let tile = native_label_tile(&template, label);
            for y in 0..32 {
                for x in 0..32 {
                    if x < 3 || x >= 31 || y < 3 || y >= 29 {
                        let index = (y * 32 + x) * 4;
                        assert_eq!(&tile[index..index + 4], &template[index..index + 4]);
                    }
                }
            }
            assert!(tile.chunks_exact(4).any(|pixel| pixel == [255; 4]));
        }
    }

    #[test]
    fn reassignment_is_staged_and_commit_contains_only_changed_wire_keys() {
        let mut original = vec![KeyBinding::default(); 90];
        original[16] = KeyBinding {
            kind: 1,
            action: 2101001,
        };
        original[17] = KeyBinding { kind: 4, action: 5 };
        let mut draft = original.clone();
        assert!(assign(&mut draft, 18, original[16]));
        assert_eq!(original[16].action, 2101001);
        assert_eq!(original[18], KeyBinding::default());
        assert_eq!(
            changes(&original, &draft),
            vec![(16, KeyBinding::default()), (18, original[16])]
        );
        drop(draft); // Cancel discards the draft; the original is still intact.
        assert_eq!(original[17].action, 5);
        assert_eq!(changes(&original, &original), Vec::new());
    }

    #[test]
    fn assignment_bounds_and_incomplete_snapshots_are_safe() {
        let mut draft = Vec::new();
        let binding = KeyBinding {
            kind: 5,
            action: 50,
        };
        assert!(!assign(&mut draft, 0, binding));
        assert!(!assign(&mut draft, 90, binding));
        assert!(!assign(&mut draft, 84, binding));
        assert!(draft.is_empty());
        assert!(assign(&mut draft, 89, binding));
        assert_eq!(draft.len(), 90);
        assert_eq!(draft[89], binding);
    }

    #[test]
    fn default_bindings_are_visible_and_executable_catalog_actions() {
        let actions = super::super::static_key_config_actions();
        for (key, binding) in defaults()
            .into_iter()
            .enumerate()
            .filter(|(_, binding)| binding.kind != 0)
        {
            assert!(
                key_rect(key as u8).is_some(),
                "default key {key} is missing from the keyboard"
            );
            assert!(
                actions.iter().any(|action| action.assignable
                    && action.kind == binding.kind
                    && action.action == binding.action),
                "default key {key} has no supported action"
            );
        }
    }

    #[test]
    fn quickslot_editor_cells_match_the_native_popup() {
        for slot in 0..8 {
            let rect = quickslot_rect(slot);
            assert!(QUICKSLOT_POPUP.contains(rect.x, rect.y));
            assert!(
                QUICKSLOT_POPUP.contains(rect.x + rect.width - 1.0, rect.y + rect.height - 1.0)
            );
            assert_eq!(quickslot_at(rect.x + 16.0, rect.y + 15.0), Some(slot));
        }
        assert_eq!(
            quickslot_at(QUICKSLOT_POPUP.x + 4.0, QUICKSLOT_POPUP.y + 4.0),
            None
        );
    }

    #[test]
    fn quickslot_reassignment_swaps_duplicates_and_rejects_reserved_keys() {
        let mut keys = DEFAULT_QUICKSLOT_KEYS;
        let old = keys;
        assert!(assign_quickslot(&mut keys, 0, old[5]));
        assert_eq!(keys[0], old[5]);
        assert_eq!(keys[5], old[0]);
        assert_eq!(keys.iter().copied().collect::<HashSet<_>>().len(), 8);
        let swapped = keys;
        for (slot, key) in [(8, 16), (0, 0), (0, 84), (0, 90)] {
            assert!(!assign_quickslot(&mut keys, slot, key));
            assert_eq!(keys, swapped);
        }
    }

    #[test]
    fn tray_pages_preserve_unbound_actions_and_match_click_geometry() {
        let actions = (0..100)
            .map(|id| KeyConfigAction {
                label: format!("Skill {id}"),
                kind: 1,
                action: 1000 + id,
                assignable: true,
            })
            .collect::<Vec<_>>();
        let bindings = vec![KeyBinding {
            kind: 1,
            action: 1000,
        }];
        let entries = tray_entries(&actions, &bindings);
        assert_eq!(entries.len(), 99);
        assert_eq!(
            entries
                .iter()
                .map(|(_, cell)| *cell)
                .collect::<HashSet<_>>()
                .len(),
            99
        );
        let page = scroll_max(&actions, &bindings);
        let (index, cell) = *entries.iter().max_by_key(|(_, cell)| *cell).unwrap();
        let rect = TRAY.cell(cell - page * 18);
        assert_eq!(
            tray_action_at(rect.x + 1.0, rect.y + 1.0, page, &actions, &bindings),
            Some(index)
        );
        assert_eq!(
            tray_action_at(TRAY.x + 33.0, TRAY.y, 0, &actions, &bindings),
            None
        );
    }

    #[test]
    fn reference_keys_buttons_and_wide_key_edges_are_separate_targets() {
        for key in [2, 3, 11, 16, 29, 42, 57, 59, 85, 86, 87, 88] {
            let rect = key_rect(key).unwrap();
            assert_eq!(key_at(rect.x + rect.width * 0.5, rect.y + 16.0), Some(key));
        }
        for (button, rect, _) in buttons() {
            assert_eq!(button_at(rect.x + 1.0, rect.y + 1.0), Some(button));
        }
        assert_eq!(button_at(60.0, 245.0), None);
        assert_eq!(key_at(0.0, 0.0), None);
    }
}
