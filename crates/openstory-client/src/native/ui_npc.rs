fn map_string_category(map_id: i32) -> &'static str {
    if map_id < 100_000_000 {
        "maple"
    } else if map_id < 200_000_000 {
        "victoria"
    } else if map_id < 300_000_000 {
        "ossyria"
    } else if map_id < 540_000_000 {
        "elin"
    } else if map_id < 600_000_000 {
        "singapore"
    } else if map_id < 670_000_000 {
        "MasteriaGL"
    } else if map_id < 682_000_000 {
        let prefix3 = (map_id / 1_000_000) * 1_000_000;
        let prefix4 = (map_id / 100_000) * 100_000;
        if prefix3 == 674_000_000 || prefix4 == 680_100_000 || prefix4 == 889_100_000 {
            "etc"
        } else if prefix3 == 677_000_000 {
            "Episode1GL"
        } else {
            "weddingGL"
        }
    } else if map_id < 683_000_000 {
        "HalloweenGL"
    } else if map_id < 800_000_000 {
        "event"
    } else if map_id < 900_000_000 {
        "jp"
    } else {
        "etc"
    }
}

fn map_string_name(archive: &NxArchive, map_id: i32) -> Option<String> {
    let category = map_string_category(map_id);
    nx_string_value(archive, &format!("Map.img/{category}/{map_id}/mapName"))
}

fn map_string_street_name(archive: &NxArchive, map_id: i32) -> Option<String> {
    let category = map_string_category(map_id);
    nx_string_value(archive, &format!("Map.img/{category}/{map_id}/streetName"))
}

fn format_npc_dialogue_text(
    text: &str,
    player_name: &str,
    string_archive: &NxArchive,
    item_name_cache: &mut HashMap<i32, Option<String>>,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    skill_archive: &NxArchive,
    ui_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
) -> (String, NpcInlineIcons) {
    let normalized = text.replace("\\r\\n", "\n").replace("\\r", "\n");
    let characters = normalized.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(text.len());
    let mut item_macro_ids = Vec::new();
    let mut inline_icons = NpcInlineIcons::new();
    let mut icon_markers = HashMap::new();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if character == '\\' && characters.get(index + 1) == Some(&'n') {
            output.push('\n');
            index += 2;
            continue;
        }
        if character.is_control() && !matches!(character, '\n' | '\t') {
            index += 1;
            continue;
        }
        if character == 'M'
            && characters.get(index + 1) == Some(&'a')
            && characters.get(index + 2) == Some(&'p')
            && characters.get(index + 3) == Some(&' ')
        {
            let map_id_start = index + 4;
            let map_id_end = map_id_start
                + characters[map_id_start..]
                    .iter()
                    .take_while(|value| value.is_ascii_digit())
                    .count();
            let has_map_id = map_id_end > map_id_start;
            let has_token_boundary = match characters.get(map_id_end) {
                Some(next) => !next.is_ascii_alphanumeric() && *next != '_',
                None => true,
            };
            if has_map_id
                && has_token_boundary
                && let Ok(map_id) = characters[map_id_start..map_id_end]
                    .iter()
                    .collect::<String>()
                    .parse::<i32>()
                && let Some(map_name) = map_string_name(string_archive, map_id)
            {
                output.push_str(&map_name);
                index = map_id_end;
                continue;
            }
        }
        if character != '#' {
            output.push(character);
            index += 1;
            continue;
        }
        let Some(code) = characters.get(index + 1).copied() else {
            output.push('#');
            break;
        };
        if let Some(style) = match code {
            'b' => Some('\u{f0000}'),
            'k' => Some('\u{f0001}'),
            'r' => Some('\u{f0002}'),
            'd' => Some('\u{f0003}'),
            'e' => Some('\u{f0004}'),
            'n' => Some('\u{f0005}'),
            'g' => Some('\u{f0006}'),
            _ => None,
        } {
            output.push(style);
            index += 2;
            continue;
        }
        if code == 'c' {
            index += 2;
            continue;
        }
        if code == 'h' && characters.get(index + 2) == Some(&'#') {
            output.push_str(player_name);
            index += 3;
            continue;
        }
        if matches!(code, 'L' | 'l') {
            output.push('#');
            output.push(code);
            index += 2;
            continue;
        }
        if !code.is_ascii_alphabetic() {
            output.push('#');
            index += 1;
            continue;
        }
        let Some(end) = characters[index + 2..]
            .iter()
            .position(|value| *value == '#')
        else {
            output.push('#');
            index += 1;
            continue;
        };
        let end = index + 2 + end;
        let id = characters[index + 2..end].iter().collect::<String>();
        if !id.is_empty() && id.chars().all(|value| value.is_ascii_digit()) {
            if let Ok(id_number) = id.parse::<i32>() {
                match code {
                    'p' => output.push_str(
                        &nx_string_value(string_archive, &format!("Npc.img/{id}/name"))
                            .unwrap_or_else(|| format!("NPC {id}")),
                    ),
                    'm' => output.push_str(
                        &map_string_name(string_archive, id_number)
                            .unwrap_or_else(|| format!("Map {id}")),
                    ),
                    'o' => output.push_str(
                        &nx_string_value(string_archive, &format!("Mob.img/{id}/name"))
                            .unwrap_or_else(|| format!("Mob {id}")),
                    ),
                    't' | 'z' => {
                        item_macro_ids.push(id);
                        output.push_str(&item_name(id_number, string_archive, item_name_cache));
                    }
                    'v' | 'i' | 'q' | 's' | 'f' => {
                        let icon_kind = if code == 'i' { 'v' } else { code };
                        if matches!(code, 'v' | 'i') {
                            item_macro_ids.push(id);
                        }
                        if let Some(marker) = icon_markers.get(&(icon_kind, id_number)).copied() {
                            output.push(marker);
                        } else if inline_icons.len() < 0x18ff
                            && let Some(icon) = npc_dialogue_icon(
                                icon_kind,
                                id_number,
                                item_archive,
                                character_archive,
                                skill_archive,
                                ui_archive,
                                icon_cache,
                            )
                            && let Some(marker) = char::from_u32(0xe000 + inline_icons.len() as u32)
                        {
                            inline_icons.insert(marker, icon);
                            icon_markers.insert((icon_kind, id_number), marker);
                            output.push(marker);
                        }
                    }
                    _ => {}
                }
                index = end + 1;
                continue;
            }
        }
        output.push('#');
        index += 1;
    }

    for id in item_macro_ids.into_iter().filter(|id| id.len() >= 4) {
        let mut search_from = 0;
        while let Some(relative) = output.get(search_from..).and_then(|tail| tail.find(&id)) {
            let start = search_from + relative;
            let end = start + id.len();
            let left_is_digit = start > 0
                && output[..start]
                    .chars()
                    .next_back()
                    .is_some_and(|value| value.is_ascii_digit());
            let right_is_digit = output[end..]
                .chars()
                .next()
                .is_some_and(|value| value.is_ascii_digit());
            if left_is_digit || right_is_digit {
                search_from = end;
                continue;
            }
            let mut remove_start = start;
            let mut remove_end = end;
            if output[..start].ends_with(' ') {
                remove_start -= 1;
            } else if output[end..].starts_with(' ') {
                remove_end += 1;
            }
            output.replace_range(remove_start..remove_end, "");
            search_from = remove_start;
        }
    }
    (output, inline_icons)
}

fn npc_dialogue_icon(
    kind: char,
    id: i32,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    skill_archive: &NxArchive,
    ui_archive: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
) -> Option<(u32, u32, Arc<[u8]>)> {
    if kind == 'v' {
        return inventory_item_icon(id, item_archive, character_archive, icon_cache);
    }
    let (archive, path) = match kind {
        'q' => (ui_archive, format!("UIWindow.img/QuestIcon/{id}")),
        's' => (
            skill_archive,
            format!("{:03}.img/skill/{id:07}/icon", id / 10_000),
        ),
        'f' => (
            character_archive,
            format!("Face/000{id:05}.img/default/face"),
        ),
        _ => return None,
    };
    let node = resolve_ui_path(&archive.root(), &path).ok().flatten()?;
    let art = first_bitmap_node(&node).ok().flatten()?;
    let bitmap = art.bitmap().ok().flatten()?;
    let (width, height) = (u32::from(bitmap.width()), u32::from(bitmap.height()));
    if width == 0 || height == 0 || width > 512 || height > 512 {
        return None;
    }
    Some((width, height, bitmap.decode_bgra_shared().ok()?))
}

fn parse_npc_selections(text: &str) -> (String, Vec<(i32, String)>) {
    let mut plain = String::new();
    let mut selections = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = text[cursor..].find("#L") {
        let start = cursor + relative_start;
        plain.push_str(&text[cursor..start]);
        let id_start = start + 2;
        let Some(id_end_relative) = text[id_start..].find('#') else {
            plain.push_str(&text[start..]);
            return (clean_npc_markup(&plain), selections);
        };
        let id_end = id_start + id_end_relative;
        let Ok(selection_id) = text[id_start..id_end].parse::<i32>() else {
            plain.push_str(&text[start..id_end + 1]);
            cursor = id_end + 1;
            continue;
        };
        let label_start = id_end + 1;
        // Several shipped island quiz pages omit #l, including Robin's last question.
        let rest = &text[label_start..];
        let terminator = rest.find("#l");
        let next_option = rest.find("#L");
        let label_length = [terminator, next_option]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(rest.len());
        let label_end = label_start + label_length;
        selections.push((
            selection_id,
            clean_npc_markup(&text[label_start..label_end])
                .trim()
                .to_owned(),
        ));
        cursor = label_end
            + if terminator == Some(label_length) {
                2
            } else {
                0
            };
    }
    plain.push_str(&text[cursor..]);
    (clean_npc_markup(&plain).trim().to_owned(), selections)
}

fn clean_npc_markup(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\\' && chars.peek() == Some(&'n') {
            chars.next();
            output.push('\n');
        } else if character == '#' {
            if let Some(next) = chars.next() {
                if next.is_ascii_alphabetic() {
                    if !matches!(next, 'b' | 'k' | 'r' | 'g' | 'e' | 'n' | 'h') {
                        while chars.next().is_some_and(|value| value != '#') {}
                    }
                } else {
                    output.push(character);
                    output.push(next);
                }
            }
        } else {
            output.push(character);
        }
    }
    output
}

fn merchant_item_info(
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    item_id: i32,
) -> Option<openstory_assets::NxNode> {
    let item_prefix = item_id / 10_000;
    let item_file = format!("0{item_id}.img");
    let item_node = format!("0{item_id}");
    let (archive, info_path) = match item_id / 1_000_000 {
        1 => {
            let category = match item_prefix {
                180..=183 => "PetEquip",
                190 | 191 => "TamingMob",
                _ => match item_prefix - 100 {
                    0 => "Cap",
                    1..=3 | 12..=14 => "Accessory",
                    4 => "Coat",
                    5 => "Longcoat",
                    6 => "Pants",
                    7 => "Shoes",
                    8 => "Glove",
                    9 => "Shield",
                    10 => "Cape",
                    11 => "Ring",
                    30..=70 => "Weapon",
                    _ => return None,
                },
            };
            (character_archive, format!("{category}/{item_file}/info"))
        }
        2 => (
            item_archive,
            format!("Consume/0{item_prefix}.img/{item_node}/info"),
        ),
        3 => (
            item_archive,
            format!("Install/0{item_prefix}.img/{item_node}/info"),
        ),
        4 => (
            item_archive,
            format!("Etc/0{item_prefix}.img/{item_node}/info"),
        ),
        _ => return None,
    };
    resolve_ui_path(&archive.root(), &info_path).ok().flatten()
}

fn merchant_stock_block_reason(
    item: &openstory_protocol::InventoryItem,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
) -> Option<&'static str> {
    if item.inventory_type == openstory_protocol::InventoryType::Cash || item.cash {
        return Some("Cash items cannot be listed in a player shop.");
    }
    let info = merchant_item_info(item_archive, character_archive, item.item_id)?;
    if item_flag_is_set(&info, "cash") {
        Some("Cash items cannot be listed in a player shop.")
    } else if item_flag_is_set(&info, "tradeBlock") {
        Some("This item cannot be traded or listed in a player shop.")
    } else {
        None
    }
}

fn item_flag_is_set(info: &openstory_assets::NxNode, name: &str) -> bool {
    integer_property(info, name).is_some_and(|value| value != 0)
}

fn format_meso_amount(amount: i64) -> String {
    let digits = amount.max(0).to_string();
    let mut reversed = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            reversed.push(',');
        }
        reversed.push(digit);
    }
    format!("{} Mesos", reversed.chars().rev().collect::<String>())
}

// All NPC dialogue coordinates are NX-local: bitmap anchors, text and hitboxes
// share one 529x260 panel coordinate space. The panel itself is centered in the
// 1024x768 logical canvas (render scaling is applied later by the UI renderer).
struct NpcDialogueLayout;

impl NpcDialogueLayout {
    const WIDTH: usize = 529;
    const HEIGHT: usize = 260;
    const BOTTOM_Y: usize = 202;
    const PANEL_X: f32 = (1024.0 - Self::WIDTH as f32) / 2.0;
    const PANEL_Y: f32 = (768.0 - Self::HEIGHT as f32) / 2.0;

    const NPC_BAR_X: i32 = 22;
    const NPC_BAR_Y: i32 = 141;
    const NPC_BAR_WIDTH: i32 = 121;
    const NPC_CENTER_X: i32 = Self::NPC_BAR_X + Self::NPC_BAR_WIDTH / 2;
    const TEXT_X: i32 = 166;
    const TEXT_Y: i32 = 48;
    const TEXT_WIDTH: usize = 320;
    const SCROLL_X: usize = 490;
    const INPUT_X: i32 = Self::TEXT_X + 8;
    const INPUT_Y: i32 = 177;
    const INPUT_HEIGHT: usize = 20;
    const SELECTION_ROW_STEP: i32 = 25;
    const SELECTION_HEIGHT: usize = 23;
    const ADJUST_MINUS_X: i32 = 328;
    const ADJUST_PLUS_X: i32 = 365;
    const CLOSE_X: i32 = 9;
    const YES_X: i32 = 400;
    const NO_X: i32 = 465;
    const PREV_X: i32 = 389;
    const NEXT_X: i32 = 471;
    const OK_X: i32 = 471;
    const SELECTION_BOTTOM_Y: i32 = 227;
    const BUTTON_Y: i32 = 236;
}

fn append_npc_dialogue_input(input: &mut String, message_type: i8, text: &str) {
    let limit = match message_type {
        2 => 50,
        3 => 16,
        _ => return,
    };
    for character in text.chars() {
        if input.chars().count() >= limit {
            break;
        }
        let accepted = if message_type == 2 {
            !character.is_control()
        } else {
            character.is_ascii_digit() || (character == '-' && input.is_empty())
        };
        if accepted {
            input.push(character);
        }
    }
}

fn npc_dialogue_number(input: &str, bounds: Option<(i32, i32, i32)>) -> i32 {
    let value = input
        .parse::<i64>()
        .unwrap_or_else(|_| i64::from(bounds.map_or(0, |bounds| bounds.0)))
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    match bounds {
        Some((_, minimum, maximum)) if minimum <= maximum => value.clamp(minimum, maximum),
        _ => value,
    }
}

fn npc_dialogue_cancel_response(message_type: i8) -> i8 {
    match message_type {
        0 | 1 | 5 | 6 => -1,
        _ => 0,
    }
}

fn npc_dialogue_text_view_height(message_type: i8) -> usize {
    match message_type {
        2 | 3 => 112,
        4 => 106,
        _ => NpcDialogueLayout::BOTTOM_Y - NpcDialogueLayout::TEXT_Y as usize,
    }
}

fn npc_dialogue_selection_visible_rows(text_view_height: usize) -> usize {
    let first_row_y = NpcDialogueLayout::TEXT_Y
        + if text_view_height == 0 {
            0
        } else {
            text_view_height as i32 + 8
        };
    let available_height = NpcDialogueLayout::SELECTION_BOTTOM_Y - first_row_y;
    let row_height = NpcDialogueLayout::SELECTION_HEIGHT as i32;
    let row_step = NpcDialogueLayout::SELECTION_ROW_STEP;
    if available_height < row_height {
        return 0;
    }
    (1 + (available_height - row_height) / row_step).clamp(0, 5) as usize
}

#[cfg(test)]
mod npc_dialogue_viewport_tests {
    use super::*;

    #[test]
    fn long_dialogue_and_scrolled_text_stay_above_the_footer() {
        let font = fontdue::Font::from_bytes(include_bytes!("../../assets/fonts/Roboto-Regular.ttf").as_slice(), fontdue::FontSettings::default()).unwrap();
        let text = "A long line of dialogue that must remain inside the text viewport. ".repeat(30);
        let icons = NpcInlineIcons::new();
        let view_height = npc_dialogue_text_view_height(0);
        let size = ui_foundation::UiTextRole::Body.style().size;
        let content_height = dialogue_text_content_height(&font, &text, size, NpcDialogueLayout::TEXT_WIDTH, &icons);
        assert!(content_height > view_height);
        for offset in [0, content_height - view_height] {
            let mut pixels = vec![0; NpcDialogueLayout::WIDTH * NpcDialogueLayout::HEIGHT * 4];
            draw_dialogue_text_with_icons_scrolled(&font, &mut pixels, NpcDialogueLayout::WIDTH, NpcDialogueLayout::HEIGHT, &text, NpcDialogueLayout::TEXT_X, NpcDialogueLayout::TEXT_Y, size, NpcDialogueLayout::TEXT_WIDTH, view_height, [0, 0, 0], &icons, offset);
            assert!(pixels.iter().any(|&pixel| pixel != 0));
            assert!(pixels[NpcDialogueLayout::BOTTOM_Y * NpcDialogueLayout::WIDTH * 4..].iter().all(|&pixel| pixel == 0));
        }
    }
}

fn dialogue_text_content_height(
    font: &fontdue::Font,
    text: &str,
    size: f32,
    max_width: usize,
    inline_icons: &NpcInlineIcons,
) -> usize {
    let line_height = size.ceil() as usize + 4;
    let mut line_y = 0_usize;
    let mut pen_x = 0.0_f32;
    let mut previous = None;
    let mut bold = false;
    let mut has_content = false;
    let wrapped = wrap_dialogue_text(font, text, size, max_width, inline_icons);
    for character in wrapped.chars() {
        if character == '\u{f0004}' {
            bold = true;
            previous = None;
            continue;
        }
        if character == '\u{f0005}' {
            bold = false;
            previous = None;
            continue;
        }
        if (0xf0000..=0xf0006).contains(&(character as u32)) {
            continue;
        }
        let font = if bold {
            ui_foundation::bold_font().unwrap_or(font)
        } else {
            font
        };
        has_content = true;
        if character == '\n' {
            line_y = line_y.saturating_add(line_height);
            pen_x = 0.0;
            previous = None;
            continue;
        }
        if inline_icons.contains_key(&character) {
            let icon_size = (size.ceil() as usize + 4).clamp(16, 32);
            if pen_x + icon_size as f32 > max_width as f32 {
                line_y = line_y.saturating_add(line_height);
                pen_x = 0.0;
            }
            pen_x += icon_size as f32;
            previous = None;
            continue;
        }
        if let Some(previous) = previous {
            pen_x += font
                .horizontal_kern(previous, character, size)
                .unwrap_or(0.0);
        }
        let advance_width = font.metrics(character, size).advance_width;
        if pen_x + advance_width > max_width as f32 {
            line_y = line_y.saturating_add(line_height);
            pen_x = 0.0;
        }
        pen_x += advance_width;
        previous = Some(character);
    }
    if has_content {
        line_y.saturating_add(line_height)
    } else {
        0
    }
}

fn render_npc_dialogue(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    npc_archive: &NxArchive,
    string_archive: &NxArchive,
    npc_id: i32,
    speaker: i8,
    message_type: i8,
    navigation: Option<(bool, bool)>,
    text: &str,
    text_scroll: usize,
    inline_icons: &NpcInlineIcons,
    selections: &[(i32, String)],
    selected_selection: usize,
    first_visible_selection: usize,
    hovered_selection: Option<usize>,
    input: &str,
) -> (Vec<u8>, SpriteRect, Vec<NpcDialogueButton>, usize) {
    let (width, height) = (NpcDialogueLayout::WIDTH, NpcDialogueLayout::HEIGHT);
    let (panel_x, panel_y) = (NpcDialogueLayout::PANEL_X, NpcDialogueLayout::PANEL_Y);
    let mut pixels = vec![0_u8; width * height * 4];
    let _ = composite_npc_dialogue_frame(&mut pixels, width, height, ui_archive);
    if speaker == 0 {
        let mut npc_path = format!("{:07}.img", npc_id.max(0));
        for _ in 0..8 {
            let link = npc_archive
                .root()
                .resolve(&format!("{npc_path}/info/link"))
                .ok()
                .flatten()
                .and_then(|node| match node.value().ok()? {
                    NodeValue::String(link) => Some(link),
                    _ => None,
                });
            let Some(link) = link else {
                break;
            };
            npc_path = format!("{link}.img");
        }
        let npc_path = format!("{npc_path}/stand/0");
        let name = string_archive
            .root()
            .resolve(&format!("Npc.img/{npc_id}/name"))
            .ok()
            .flatten()
            .and_then(|node| match node.value().ok()? {
                NodeValue::String(name) if !name.trim().is_empty() => Some(name),
                _ => None,
            })
            .unwrap_or_else(|| format!("NPC {npc_id}"));
        if let Some(npc) = npc_archive.root().resolve(&npc_path).ok().flatten() {
            let _ = composite_nx_node_bitmap(
                &mut pixels,
                width,
                height,
                &npc,
                NpcDialogueLayout::NPC_CENTER_X,
                NpcDialogueLayout::NPC_BAR_Y,
            );
        }
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            width,
            height,
            ui_archive,
            "UIWindow.img/UtilDlgEx/bar",
            NpcDialogueLayout::NPC_BAR_X,
            NpcDialogueLayout::NPC_BAR_Y,
        );
        let name_style = ui_foundation::UiTextRole::Label.style();
        let name = ui_foundation::ellipsize(
            font, &name, name_style.size, (NpcDialogueLayout::NPC_BAR_WIDTH - 8) as f32,
        );
        let name_width = (dialogue_text_width(font, &name, name_style.size).ceil() as i32) + 1;
        ui_foundation::draw_text_role(
            font,
            &mut pixels,
            width,
            height,
            &name,
            NpcDialogueLayout::NPC_BAR_X + (NpcDialogueLayout::NPC_BAR_WIDTH - name_width) / 2,
            NpcDialogueLayout::NPC_BAR_Y + 2,
            ui_foundation::UiTextRole::Label,
            NpcDialogueLayout::NPC_BAR_WIDTH as usize,
            17,
            Some([255, 255, 255]),
        );
    }
    let body_style = ui_foundation::UiTextRole::Body.style();
    let text_content_height = if text.trim().is_empty() {
        0
    } else {
        dialogue_text_content_height(
            font,
            text,
            body_style.size,
            NpcDialogueLayout::TEXT_WIDTH,
            inline_icons,
        )
    };
    let text_view_height = if message_type == 4 {
        if text.trim().is_empty() {
            0
        } else {
            text_content_height.min(npc_dialogue_text_view_height(4))
        }
    } else {
        npc_dialogue_text_view_height(message_type)
    };
    let text_max_scroll = text_content_height.saturating_sub(text_view_height);
    let text_scroll = text_scroll.min(text_max_scroll);
    draw_dialogue_text_with_icons_scrolled(
        font,
        &mut pixels,
        width,
        height,
        text,
        NpcDialogueLayout::TEXT_X,
        NpcDialogueLayout::TEXT_Y,
        body_style.size,
        NpcDialogueLayout::TEXT_WIDTH,
        text_view_height,
        [46, 46, 46],
        inline_icons,
        text_scroll,
    );
    ui_foundation::draw_scrollbar(
        &mut pixels,
        width,
        height,
        ui_archive,
        NpcDialogueLayout::SCROLL_X as i32,
        NpcDialogueLayout::TEXT_Y,
        text_view_height,
        ui_foundation::scroll_fraction(text_scroll, text_content_height, text_view_height),
    );

    let mut buttons = Vec::new();
    if message_type == 4 {
        for (row, (index, (selection, label))) in selections
            .iter()
            .enumerate()
            .skip(first_visible_selection)
            .take(npc_dialogue_selection_visible_rows(text_view_height))
            .enumerate()
        {
            let local_y = NpcDialogueLayout::TEXT_Y
                + if text_view_height == 0 {
                    0
                } else {
                    text_view_height as i32 + 8
                }
                + row as i32 * NpcDialogueLayout::SELECTION_ROW_STEP;
            let body_size = body_style.size;
            let label_width = dialogue_text_width(font, label, body_size)
                .ceil()
                .clamp(12.0, (NpcDialogueLayout::TEXT_WIDTH - 24) as f32);
            let label_x = NpcDialogueLayout::TEXT_X + 24;
            let rect = SpriteRect {
                x: panel_x + (label_x - 14) as f32,
                y: panel_y + local_y as f32,
                width: (label_width + 20.0).min(NpcDialogueLayout::TEXT_WIDTH as f32 - 8.0),
                height: NpcDialogueLayout::SELECTION_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            };
            let hovered = hovered_selection == Some(index);
            let selected = index == selected_selection;
            let marker = if selected || hovered { "dot1" } else { "dot0" };
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                width,
                height,
                ui_archive,
                &format!("UIWindow.img/UtilDlgEx/{marker}"),
                NpcDialogueLayout::TEXT_X + 10,
                local_y + 7,
            );
            let link_color = if hovered {
                [132, 71, 168]
            } else {
                [108, 61, 145]
            };
            draw_dialogue_text_with_icons(
                font,
                &mut pixels,
                width,
                height,
                label,
                label_x,
                local_y + 2,
                body_size,
                NpcDialogueLayout::TEXT_WIDTH - 26,
                NpcDialogueLayout::SELECTION_HEIGHT,
                link_color,
                inline_icons,
            );
            let underline_width = label_width.ceil() as usize;
            let _ = composite_nx_ui_bitmap_scaled_to(
                &mut pixels,
                width,
                height,
                ui_archive,
                "UIWindow.img/UtilDlgEx/line",
                label_x.max(0) as usize,
                (local_y + 20).max(0) as usize,
                underline_width,
                if hovered { 2 } else { 1 },
            );
            buttons.push(NpcDialogueButton {
                rect,
                action: NpcDialogueAction::Select(*selection),
            });
        }
        push_nx_dialogue_button(
            &mut pixels,
            &mut buttons,
            ui_archive,
            width,
            height,
            "UIWindow.img/UtilDlgEx/BtClose/normal/0",
            NpcDialogueLayout::CLOSE_X,
            NpcDialogueLayout::BUTTON_Y,
            NpcDialogueAction::More(0),
        );
    } else {
        if message_type == 2 || message_type == 3 {
            let mut visible = input;
            while dialogue_text_width(font, visible, 16.0) > 287.0 {
                let Some((index, _)) = visible.char_indices().nth(1) else {
                    break;
                };
                visible = &visible[index..];
            }
            let visible_with_caret = format!("{visible}|");
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                &visible_with_caret,
                NpcDialogueLayout::INPUT_X,
                NpcDialogueLayout::INPUT_Y,
                16.0,
                NpcDialogueLayout::TEXT_WIDTH - 16,
                NpcDialogueLayout::INPUT_HEIGHT,
                [46, 46, 46],
            );
        }
        push_nx_dialogue_button(
            &mut pixels,
            &mut buttons,
            ui_archive,
            width,
            height,
            "UIWindow.img/UtilDlgEx/BtClose/normal/0",
            NpcDialogueLayout::CLOSE_X,
            NpcDialogueLayout::BUTTON_Y,
            NpcDialogueAction::More(npc_dialogue_cancel_response(message_type)),
        );
        match message_type {
            1 => {
                push_nx_yes_no_buttons(&mut pixels, &mut buttons, ui_archive, width, height);
            }
            2 => push_nx_dialogue_button(
                &mut pixels,
                &mut buttons,
                ui_archive,
                width,
                height,
                "UIWindow.img/UtilDlgEx/BtOK/normal/0",
                NpcDialogueLayout::OK_X,
                NpcDialogueLayout::BUTTON_Y,
                NpcDialogueAction::SubmitText,
            ),
            3 => {
                for (x, path, delta) in [
                    (
                        NpcDialogueLayout::ADJUST_MINUS_X,
                        "Basic.img/BtMin/normal/0",
                        -1,
                    ),
                    (
                        NpcDialogueLayout::ADJUST_PLUS_X,
                        "Basic.img/BtMax/normal/0",
                        1,
                    ),
                ] {
                    push_nx_dialogue_button(
                        &mut pixels,
                        &mut buttons,
                        ui_archive,
                        width,
                        height,
                        path,
                        x + 10,
                        NpcDialogueLayout::BUTTON_Y + 3,
                        NpcDialogueAction::AdjustNumber(delta),
                    );
                }
                push_nx_dialogue_button(
                    &mut pixels,
                    &mut buttons,
                    ui_archive,
                    width,
                    height,
                    "UIWindow.img/UtilDlgEx/BtOK/normal/0",
                    NpcDialogueLayout::OK_X,
                    NpcDialogueLayout::BUTTON_Y,
                    NpcDialogueAction::SubmitNumber,
                );
            }
            5 => push_nx_dialogue_button(
                &mut pixels,
                &mut buttons,
                ui_archive,
                width,
                height,
                "UIWindow.img/UtilDlgEx/BtNext/normal/0",
                NpcDialogueLayout::NEXT_X,
                NpcDialogueLayout::BUTTON_Y,
                NpcDialogueAction::More(1),
            ),
            6 => {
                push_nx_dialogue_button(
                    &mut pixels,
                    &mut buttons,
                    ui_archive,
                    width,
                    height,
                    "UIWindow.img/UtilDlgEx/BtPrev/normal/0",
                    NpcDialogueLayout::PREV_X,
                    NpcDialogueLayout::BUTTON_Y,
                    NpcDialogueAction::More(0),
                );
                push_nx_dialogue_button(
                    &mut pixels,
                    &mut buttons,
                    ui_archive,
                    width,
                    height,
                    "UIWindow.img/UtilDlgEx/BtNext/normal/0",
                    NpcDialogueLayout::NEXT_X,
                    NpcDialogueLayout::BUTTON_Y,
                    NpcDialogueAction::More(1),
                );
            }
            12 => {
                for (path, x, response) in [
                    (
                        "UIWindow.img/UtilDlgEx/BtQYes/normal/0",
                        NpcDialogueLayout::YES_X,
                        1,
                    ),
                    (
                        "UIWindow.img/UtilDlgEx/BtQNo/normal/0",
                        NpcDialogueLayout::NO_X,
                        0,
                    ),
                ] {
                    push_nx_dialogue_button(
                        &mut pixels,
                        &mut buttons,
                        ui_archive,
                        width,
                        height,
                        path,
                        x,
                        NpcDialogueLayout::BUTTON_Y,
                        NpcDialogueAction::More(response),
                    );
                }
            }
            0 if navigation.is_some_and(|(previous, next)| previous || next) => {
                let (previous, next) = navigation.unwrap_or_default();
                for (visible, path, x, response) in [
                    (
                        previous,
                        "UIWindow.img/UtilDlgEx/BtPrev/normal/0",
                        NpcDialogueLayout::PREV_X,
                        0,
                    ),
                    (
                        next,
                        "UIWindow.img/UtilDlgEx/BtNext/normal/0",
                        NpcDialogueLayout::NEXT_X,
                        1,
                    ),
                ] {
                    if visible {
                        push_nx_dialogue_button(
                            &mut pixels,
                            &mut buttons,
                            ui_archive,
                            width,
                            height,
                            path,
                            x,
                            NpcDialogueLayout::BUTTON_Y,
                            NpcDialogueAction::More(response),
                        );
                    }
                }
            }
            _ => push_nx_dialogue_button(
                &mut pixels,
                &mut buttons,
                ui_archive,
                width,
                height,
                "UIWindow.img/UtilDlgEx/BtOK/normal/0",
                NpcDialogueLayout::OK_X,
                NpcDialogueLayout::BUTTON_Y,
                NpcDialogueAction::More(1),
            ),
        }
    }
    (
        pixels,
        SpriteRect {
            x: panel_x,
            y: panel_y,
            width: width as f32,
            height: height as f32,
            blend_mode: BlendMode::Alpha,
        },
        buttons,
        text_max_scroll,
    )
}

fn push_nx_yes_no_buttons(
    pixels: &mut [u8],
    buttons: &mut Vec<NpcDialogueButton>,
    archive: &NxArchive,
    width: usize,
    height: usize,
) {
    push_nx_dialogue_button(
        pixels,
        buttons,
        archive,
        width,
        height,
        "UIWindow.img/UtilDlgEx/BtYes/normal/0",
        NpcDialogueLayout::YES_X,
        NpcDialogueLayout::BUTTON_Y,
        NpcDialogueAction::More(1),
    );
    push_nx_dialogue_button(
        pixels,
        buttons,
        archive,
        width,
        height,
        "UIWindow.img/UtilDlgEx/BtNo/normal/0",
        NpcDialogueLayout::NO_X,
        NpcDialogueLayout::BUTTON_Y,
        NpcDialogueAction::More(0),
    );
}

fn push_nx_dialogue_button(
    pixels: &mut [u8],
    buttons: &mut Vec<NpcDialogueButton>,
    archive: &NxArchive,
    width: usize,
    height: usize,
    path: &str,
    x: i32,
    y: i32,
    action: NpcDialogueAction,
) {
    let bounds = NxDialog::new(pixels, width, height, archive)
        .anchored_button(path, x, y)
        .unwrap_or(ui_foundation::UiRect::new(x as f32, y as f32, 46.0, 18.0));
    let rect = SpriteRect {
        x: NpcDialogueLayout::PANEL_X + bounds.x,
        y: NpcDialogueLayout::PANEL_Y + bounds.y,
        width: bounds.width,
        height: bounds.height,
        blend_mode: BlendMode::Alpha,
    };
    buttons.push(NpcDialogueButton { rect, action });
}

fn composite_npc_dialogue_frame(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    ui_archive: &NxArchive,
) -> bool {
    NxDialog::new(pixels, width, height, ui_archive)
        .vertical_frame(
            [
                "UIWindow.img/UtilDlgEx/t",
                "UIWindow.img/UtilDlgEx/c",
                "UIWindow.img/UtilDlgEx/s",
            ],
            NpcDialogueLayout::BOTTOM_Y as i32,
        )
        .is_ok()
}

pub(super) fn render_npc_dialogue_preview(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    npc_archive: &NxArchive,
    string_archive: &NxArchive,
) -> (Vec<u8>, (f32, f32, f32, f32)) {
    let (pixels, panel, _, _) = render_npc_dialogue(
        font,
        ui_archive,
        npc_archive,
        string_archive,
        ui_preview::fixture_named_id(string_archive, "Npc.img", "Robin", 2007),
        0,
        0,
        None,
        "\u{f0004}Ascension Powers\u{f0005}\n\n\u{f0000}Upon reaching level 200, you have unlocked new command:\n\u{f0002}@ascension <option>\u{f0001}\n\n\u{f0004}Available Options:\u{f0005}\n0 - View your active ascensions\n1 - Hoarder Ascension: Gain 5x EXP and drop rates for all characters\n2 - Resilient Ascension: Characters become immune to debuffs\n\n\u{f0004}Please Note:\u{f0005} \u{f0002}Ascending will send your current character to level 1.",
        0,
        &NpcInlineIcons::new(),
        &[],
        0,
        0,
        None,
        "",
    );
    (pixels, (panel.x, panel.y, panel.width, panel.height))
}

fn dialogue_text_width(font: &fontdue::Font, text: &str, size: f32) -> f32 {
    ui_foundation::measure_text(font, text, size)
}

fn rasterize_maple_tv_text(
    font: &fontdue::Font,
    lines: &[String],
    start_x: i32,
    color: [u8; 3],
) -> Vec<u8> {
    const WIDTH: usize = 240;
    const HEIGHT: usize = 90;
    const FONT_SIZE: f32 = 11.0;
    const LINE_HEIGHT: i32 = 16;
    let mut pixels = vec![0; WIDTH * HEIGHT * 4];
    let right = WIDTH as f32 - 5.0;
    let mut y = 8_i32;

    for line in lines.iter().take(5) {
        let mut x = start_x as f32;
        let mut previous = None;
        for character in line.chars() {
            if y + LINE_HEIGHT > HEIGHT as i32 {
                break;
            }
            if character == '\r' {
                continue;
            }
            if character == '\n' {
                x = start_x as f32;
                y += LINE_HEIGHT;
                previous = None;
                continue;
            }
            let kern = previous.map_or(0.0, |previous| {
                font.horizontal_kern(previous, character, FONT_SIZE)
                    .unwrap_or(0.0)
            });
            let (metrics, coverage) = rasterize_ui_glyph(font, character, FONT_SIZE);
            let mut glyph_kern = kern;
            if x + glyph_kern + metrics.advance_width > right {
                x = start_x as f32;
                y += LINE_HEIGHT;
                glyph_kern = 0.0;
            }
            if y + LINE_HEIGHT > HEIGHT as i32 {
                break;
            }
            x += glyph_kern;
            let left = x.round() as i32 + metrics.xmin;
            let baseline = y + FONT_SIZE.ceil() as i32 + 1;
            let top = baseline - metrics.ymin - metrics.height as i32;
            for row in 0..metrics.height {
                for column in 0..metrics.width {
                    let px = left + column as i32;
                    let py = top + row as i32;
                    if px < start_x || px >= WIDTH as i32 || py < 0 || py >= HEIGHT as i32 {
                        continue;
                    }
                    let alpha = coverage[row * metrics.width + column];
                    if alpha == 0 {
                        continue;
                    }
                    let index = (py as usize * WIDTH + px as usize) * 4;
                    pixels[index..index + 4]
                        .copy_from_slice(&[color[2], color[1], color[0], alpha]);
                }
            }
            x += metrics.advance_width;
            previous = Some(character);
        }
        y += LINE_HEIGHT;
    }
    pixels
}

fn draw_dialogue_text(
    font: &fontdue::Font,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    text: &str,
    x: i32,
    y: i32,
    size: f32,
    max_width: usize,
    max_height: usize,
    color: [u8; 3],
) {
    let x = ui_foundation::snap(x as f32) as i32;
    let y = ui_foundation::snap(y as f32) as i32;
    draw_dialogue_text_with_icons(
        font,
        pixels,
        width,
        height,
        text,
        x,
        y,
        size,
        max_width,
        max_height,
        color,
        &EMPTY_NPC_INLINE_ICONS,
    );
}

fn draw_dialogue_text_with_icons(
    font: &fontdue::Font,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    text: &str,
    x: i32,
    y: i32,
    size: f32,
    max_width: usize,
    max_height: usize,
    color: [u8; 3],
    inline_icons: &NpcInlineIcons,
) {
    draw_dialogue_text_with_icons_scrolled(
        font,
        pixels,
        width,
        height,
        text,
        x,
        y,
        size,
        max_width,
        max_height,
        color,
        inline_icons,
        0,
    );
}

fn draw_dialogue_text_with_icons_scrolled(
    font: &fontdue::Font,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    text: &str,
    x: i32,
    y: i32,
    size: f32,
    max_width: usize,
    max_height: usize,
    color: [u8; 3],
    inline_icons: &NpcInlineIcons,
    vertical_offset: usize,
) {
    let line_height = size.ceil() as i32 + 4;
    let mut pen_x = x as f32;
    let mut line_y = y;
    let mut previous = None;
    let mut text_color = color;
    let mut bold = false;
    let wrapped_text = wrap_dialogue_text(font, text, size, max_width, inline_icons);
    for character in wrapped_text.chars() {
        match character {
            '\u{f0000}' => {
                text_color = [108, 61, 145];
                previous = None;
                continue;
            }
            '\u{f0001}' => {
                text_color = color;
                previous = None;
                continue;
            }
            '\u{f0002}' => {
                text_color = [225, 45, 36];
                previous = None;
                continue;
            }
            '\u{f0003}' => {
                text_color = [80, 80, 80];
                previous = None;
                continue;
            }
            '\u{f0004}' => {
                bold = true;
                previous = None;
                continue;
            }
            '\u{f0005}' => {
                bold = false;
                previous = None;
                continue;
            }
            '\u{f0006}' => {
                text_color = [35, 130, 45];
                previous = None;
                continue;
            }
            _ => {}
        }
        let font = if bold {
            ui_foundation::bold_font().unwrap_or(font)
        } else {
            font
        };
        if character == '\n' {
            pen_x = x as f32;
            line_y += line_height;
            previous = None;
            continue;
        }
        if let Some((icon_width, icon_height, icon_pixels)) = inline_icons.get(&character) {
            let icon_size = (size.ceil() as usize + 4).clamp(16, 32);
            if pen_x + icon_size as f32 > x as f32 + max_width as f32 {
                pen_x = x as f32;
                line_y += line_height;
            }
            let icon_y = line_y - vertical_offset as i32;
            if icon_y >= y && icon_y + icon_size as i32 <= y + max_height as i32 {
                composite_inventory_icon(
                    pixels,
                    width,
                    height,
                    icon_pixels,
                    *icon_width as usize,
                    *icon_height as usize,
                    pen_x.round().max(0.0) as usize,
                    icon_y.max(0) as usize,
                    icon_size,
                );
            }
            pen_x += icon_size as f32;
            previous = None;
            continue;
        }
        if let Some(previous) = previous {
            pen_x += font
                .horizontal_kern(previous, character, size)
                .unwrap_or(0.0);
        }
        let (metrics, coverage) = rasterize_ui_glyph(font, character, size);
        if pen_x + metrics.advance_width > x as f32 + max_width as f32 {
            pen_x = x as f32;
            line_y += line_height;
        }
        let display_line_y = line_y - vertical_offset as i32;
        let left = pen_x.round() as i32 + metrics.xmin;
        let baseline = display_line_y + size.ceil() as i32 + 1;
        let top = baseline - metrics.ymin - metrics.height as i32;
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                let px = left + column as i32;
                let py = top + row as i32;
                if px < x.max(0)
                    || px >= x.saturating_add(max_width.min(i32::MAX as usize) as i32)
                    || py < y.max(0)
                    || px >= width as i32
                    || py >= (y + max_height as i32).min(height as i32)
                {
                    continue;
                }
                let alpha = coverage[row * metrics.width + column];
                if alpha == 0 {
                    continue;
                }
                let index = (py as usize * width + px as usize) * 4;
                let source_alpha = u32::from(alpha);
                let destination_alpha = u32::from(pixels[index + 3]);
                let inverse = 255 - source_alpha;
                let output_alpha = source_alpha + destination_alpha * inverse / 255;
                if output_alpha == 0 {
                    continue;
                }
                for (channel, foreground) in [text_color[2], text_color[1], text_color[0]]
                    .into_iter()
                    .enumerate()
                {
                    let destination = u32::from(pixels[index + channel]);
                    let premultiplied = u32::from(foreground) * source_alpha
                        + destination * destination_alpha * inverse / 255;
                    pixels[index + channel] = (premultiplied / output_alpha) as u8;
                }
                pixels[index + 3] = output_alpha as u8;
            }
        }
        pen_x += metrics.advance_width;
        previous = Some(character);
    }
}

fn wrap_dialogue_text(
    font: &fontdue::Font,
    text: &str,
    size: f32,
    max_width: usize,
    inline_icons: &NpcInlineIcons,
) -> String {
    let mut output = String::new();
    for (paragraph_index, paragraph) in text.split('\n').enumerate() {
        if paragraph_index > 0 {
            output.push('\n');
        }
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            let mut width = 0.0_f32;
            let mut previous = None;
            for character in candidate.chars() {
                if (0xf0000..=0xf0006).contains(&(character as u32)) {
                    continue;
                }
                if inline_icons.contains_key(&character) {
                    width += (size.ceil() as usize + 4).clamp(16, 32) as f32;
                    previous = None;
                } else {
                    if let Some(previous) = previous {
                        width += font
                            .horizontal_kern(previous, character, size)
                            .unwrap_or(0.0);
                    }
                    width += font.metrics(character, size).advance_width;
                    previous = Some(character);
                }
            }
            if !line.is_empty() && width > max_width as f32 {
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str(&line);
                line.clear();
                line.push_str(word);
            } else {
                line = candidate;
            }
        }
        if !line.is_empty() {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(&line);
        }
    }
    output
}

fn fill_dialogue_rect(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: [u8; 4],
) {
    let right = x.saturating_add(width).min(canvas_width);
    let bottom = y.saturating_add(height).min(canvas_height);
    for row in y.min(canvas_height)..bottom {
        for column in x.min(canvas_width)..right {
            let index = (row * canvas_width + column) * 4;
            pixels[index..index + 4].copy_from_slice(&color);
        }
    }
}

fn composite_nx_ui_bitmap(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    archive: &NxArchive,
    path: &str,
    anchor_x: i32,
    anchor_y: i32,
) -> Result<(), String> {
    let node = resolve_ui_path(&archive.root(), path)?
        .ok_or_else(|| format!("missing v83 UI asset {path:?}"))?;
    composite_nx_node_bitmap(
        pixels,
        canvas_width,
        canvas_height,
        &node,
        anchor_x,
        anchor_y,
    )
}

fn composite_nx_node_bitmap(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    node: &openstory_assets::NxNode,
    anchor_x: i32,
    anchor_y: i32,
) -> Result<(), String> {
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "NX sprite node is not a bitmap".to_owned())?;
    let source = bitmap
        .decode_bgra_shared()
        .map_err(|error| error.to_string())?;
    let origin = node
        .child("origin")
        .map_err(|error| error.to_string())?
        .map(|origin| origin.value())
        .transpose()
        .map_err(|error| error.to_string())?;
    let (origin_x, origin_y) = match origin {
        Some(NodeValue::Vector { x, y }) => (x, y),
        Some(_) => return Err("NX sprite has a malformed origin".to_owned()),
        None => (0, 0),
    };
    let left = anchor_x - origin_x;
    let top = anchor_y - origin_y;
    let source_width = usize::from(bitmap.width());
    let source_height = usize::from(bitmap.height());
    for source_y in 0..source_height {
        let destination_y = top + source_y as i32;
        if !(0..canvas_height as i32).contains(&destination_y) {
            continue;
        }
        for source_x in 0..source_width {
            let destination_x = left + source_x as i32;
            if !(0..canvas_width as i32).contains(&destination_x) {
                continue;
            }
            let source_index = (source_y * source_width + source_x) * 4;
            let destination_index =
                (destination_y as usize * canvas_width + destination_x as usize) * 4;
            let source_alpha = u32::from(source[source_index + 3]);
            if source_alpha == 0 {
                continue;
            }
            let destination_alpha = u32::from(pixels[destination_index + 3]);
            let inverse = 255 - source_alpha;
            let output_alpha = source_alpha + destination_alpha * inverse / 255;
            for channel in 0..3 {
                let foreground = u32::from(source[source_index + channel]);
                let background = u32::from(pixels[destination_index + channel]);
                let premultiplied =
                    foreground * source_alpha + background * destination_alpha * inverse / 255;
                pixels[destination_index + channel] = (premultiplied / output_alpha.max(1)) as u8;
            }
            pixels[destination_index + 3] = output_alpha as u8;
        }
    }
    Ok(())
}
