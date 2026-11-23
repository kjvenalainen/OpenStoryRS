use super::*;
use openstory_assets::NxNode;
use openstory_protocol::{CharacterStats, InventoryItem, InventorySnapshot};

const EQUIPMENT_WIDTH: usize = 248;
const STACK_WIDTH: usize = 292;
const PADDING: usize = 12;
const PORTRAIT_SIZE: usize = 68;
const HEADER_HEIGHT: usize = 27;
const DETAIL_STEP: usize = 13;
const DESCRIPTION_SIZE: f32 = 12.0;
const DESCRIPTION_STEP: usize = 16;
const MAX_HEIGHT: usize = ui_foundation::LOGICAL_HEIGHT as usize - 20;
const CORNER_RADIUS: usize = 4;

const FRAME_EDGE: [u8; 4] = [232, 222, 217, 232];
const FRAME_BODY: [u8; 4] = [74, 31, 28, 195];
const WHITE: [u8; 3] = [248, 247, 255];
const LABEL: [u8; 3] = [226, 225, 241];
const MUTED: [u8; 3] = [181, 180, 205];
const ACCENT: [u8; 3] = DIALOGUE_HIGHLIGHT_COLOR;
const ENHANCED_TITLE: [u8; 3] = [255, 228, 123];
const REQUIREMENT_MISSING: [u8; 3] = [255, 145, 147];
const RULE: [u8; 4] = [214, 197, 189, 158];

pub(super) struct Tooltip {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Tooltip {
    fn new(width: usize, height: usize) -> Self {
        let mut tooltip = Self {
            width: width as u32,
            height: height as u32,
            pixels: vec![0; width * height * 4],
        };
        tooltip.fill(0, 0, width, height, FRAME_EDGE);
        tooltip.fill(1, 1, width - 2, height - 2, FRAME_BODY);
        tooltip.round_corners();
        tooltip.fill(6, 10, 2, 6, [255, 210, 116, 255]);
        tooltip
    }

    fn round_corners(&mut self) {
        let width = self.width as usize;
        let height = self.height as usize;
        let radius = CORNER_RADIUS as f32;
        for y in 0..CORNER_RADIUS {
            for x in 0..CORNER_RADIUS {
                let distance = (radius - x as f32 - 0.5).hypot(radius - y as f32 - 0.5);
                let outer = (radius + 0.5 - distance).clamp(0.0, 1.0);
                let inner = (radius - 0.5 - distance).clamp(0.0, 1.0);
                let edge_alpha = (outer - inner) * f32::from(FRAME_EDGE[3]);
                let body_alpha = inner * f32::from(FRAME_BODY[3]);
                let alpha = edge_alpha + body_alpha;
                let mut pixel = [0; 4];
                if alpha > 0.0 {
                    for channel in 0..3 {
                        pixel[channel] = ((edge_alpha * f32::from(FRAME_EDGE[channel])
                            + body_alpha * f32::from(FRAME_BODY[channel]))
                            / alpha)
                            .round() as u8;
                    }
                    pixel[3] = alpha.round() as u8;
                }
                for (column, row) in [
                    (x, y),
                    (width - x - 1, y),
                    (x, height - y - 1),
                    (width - x - 1, height - y - 1),
                ] {
                    let index = (row * width + column) * 4;
                    self.pixels[index..index + 4].copy_from_slice(&pixel);
                }
            }
        }
    }

    fn fill(&mut self, x: usize, y: usize, w: usize, h: usize, bgra: [u8; 4]) {
        fill_dialogue_rect(
            &mut self.pixels,
            self.width as usize,
            self.height as usize,
            x,
            y,
            w,
            h,
            bgra,
        );
    }

    fn rule(&mut self, y: usize) {
        self.fill(PADDING, y, self.width as usize - 2 * PADDING, 1, RULE);
    }

    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        font: &fontdue::Font,
        text: &str,
        x: usize,
        y: usize,
        size: f32,
        width: usize,
        height: usize,
        color: [u8; 3],
    ) {
        draw_dialogue_text(
            font,
            &mut self.pixels,
            self.width as usize,
            self.height as usize,
            text,
            x as i32,
            y as i32,
            size,
            width,
            height,
            color,
        );
    }

    fn portrait(&mut self, y: usize, icon: Option<(u32, u32, Arc<[u8]>)>) {
        self.fill(
            PADDING,
            y,
            PORTRAIT_SIZE,
            PORTRAIT_SIZE,
            [226, 214, 211, 219],
        );
        self.fill(
            PADDING + 1,
            y + 1,
            PORTRAIT_SIZE - 2,
            PORTRAIT_SIZE - 2,
            [207, 194, 190, 213],
        );
        self.fill(
            PADDING + 1,
            y + 1,
            PORTRAIT_SIZE - 2,
            1,
            [245, 235, 231, 222],
        );
        let Some((width, height, pixels)) = icon else {
            return;
        };
        let (width, height) = (width as usize, height as usize);
        let longest = width.max(height);
        let target = (PORTRAIT_SIZE - 4).min(longest * 2);
        let scaled_width = (width * target / longest).max(1);
        let scaled_height = (height * target / longest).max(1);
        let mut scaled = Vec::with_capacity(scaled_width * scaled_height * 4);
        for row in 0..scaled_height {
            let source_y = row * height / scaled_height;
            for column in 0..scaled_width {
                let source_x = column * width / scaled_width;
                let start = (source_y * width + source_x) * 4;
                scaled.extend_from_slice(&pixels[start..start + 4]);
            }
        }
        composite_bgra_pixels(
            &mut self.pixels,
            self.width as usize,
            self.height as usize,
            &scaled,
            scaled_width,
            scaled_height,
            (PADDING + (PORTRAIT_SIZE - scaled_width) / 2) as i32,
            (y + (PORTRAIT_SIZE - scaled_height) / 2) as i32,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    font: &fontdue::Font,
    item: &InventoryItem,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    player_stats: Option<&CharacterStats>,
    player_inventory: Option<&InventorySnapshot>,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    item_name_cache: &mut HashMap<i32, Option<String>>,
    equipped_label: bool,
) -> Tooltip {
    let equipment = item.item_id / 1_000_000 == 1;
    let info = inventory_item_info_node(item_archive, character_archive, item.item_id);
    let untradable = item.cash
        || info.as_ref().is_some_and(|node| {
            inventory_item_property_truthy(node, "tradeBlock")
                || inventory_item_property_truthy(node, "cash")
        });
    let name = item_name(item.item_id, string_archive, item_name_cache);
    let description = item_string_property(item.item_id, string_archive, "desc")
        .map(|description| formatted_description(&description))
        .unwrap_or_default();
    let icon = inventory_item_icon(item.item_id, item_archive, character_archive, icon_cache);
    let gender = if equipped_label {
        Some("EQUIPPED")
    } else {
        match (equipment, item.item_id / 1_000 % 10) {
            (true, 0) => Some("Male only"),
            (true, 1) => Some("Female only"),
            _ => None,
        }
    };
    let restriction = match (untradable, gender) {
        (true, Some(gender)) => Some(format!("Untradable  ·  {gender}")),
        (true, None) => Some("Untradable".to_owned()),
        (false, gender) => gender.map(str::to_owned),
    };
    let body_y = HEADER_HEIGHT + if restriction.is_some() { 25 } else { 9 };
    let mut tooltip = if equipment {
        render_equipment(
            font,
            item,
            info.as_ref(),
            player_stats,
            player_inventory,
            &description,
            body_y,
        )
    } else {
        render_stack(font, &description, item.count, body_y)
    };
    let enhancements = item.equip.as_ref().map_or(0, |equip| equip.level);
    draw_title(&mut tooltip, font, &name, enhancements);
    if let Some(restriction) = restriction {
        tooltip.text(
            font,
            &restriction,
            PADDING,
            HEADER_HEIGHT + 3,
            10.5,
            tooltip.width as usize - 2 * PADDING,
            15,
            ACCENT,
        );
    }
    tooltip.portrait(body_y, icon);
    tooltip
}

fn draw_title(tooltip: &mut Tooltip, font: &fontdue::Font, name: &str, enhancements: u8) {
    const SIZE: f32 = 13.0;
    let face = ui_foundation::bold_font().unwrap_or(font);
    let width = tooltip.width as usize - 2 * PADDING;
    let suffix = (enhancements > 0).then(|| format!("(+{enhancements})"));
    let suffix_width = suffix.as_ref().map_or(0, |suffix| {
        ui_foundation::measure_text(face, suffix, SIZE).ceil() as usize + 5
    });
    let name = ui_foundation::ellipsize(face, name, SIZE, (width - suffix_width) as f32);
    let name_width = ui_foundation::measure_text(face, &name, SIZE).ceil() as usize;
    let color = if enhancements > 0 {
        ENHANCED_TITLE
    } else {
        WHITE
    };
    tooltip.text(
        face,
        &name,
        PADDING,
        4,
        SIZE,
        width - suffix_width,
        18,
        color,
    );
    if let Some(suffix) = suffix {
        tooltip.text(
            face,
            &suffix,
            PADDING + name_width + 5,
            4,
            SIZE,
            suffix_width,
            18,
            color,
        );
    }
}

fn description_height(font: &fontdue::Font, text: &str, width: usize, y: usize) -> usize {
    let available = (MAX_HEIGHT - y - 10) / DESCRIPTION_STEP * DESCRIPTION_STEP;
    dialogue_text_content_height(font, text, DESCRIPTION_SIZE, width, &EMPTY_NPC_INLINE_ICONS)
        .min(available)
}

fn formatted_description(description: &str) -> String {
    let normalized = description
        .replace("\\r\\n", "\n")
        .replace("\\n", "\n")
        .replace("\\r", "\n")
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut chars = normalized.trim().chars().peekable();
    let mut text = String::new();
    let mut highlighted = false;
    while let Some(character) = chars.next() {
        match (character, chars.peek()) {
            ('#', Some('c')) => {
                chars.next();
                text.push('\u{f0007}');
                highlighted = true;
            }
            ('#', Some('b' | 'k' | 'r' | 'd' | 'e' | 'n' | 'g')) => {
                chars.next();
                text.push('\u{f0001}');
                highlighted = false;
            }
            ('#', _) if highlighted => {
                text.push('\u{f0001}');
                highlighted = false;
            }
            _ => text.push(character),
        }
    }
    text
}

fn render_stack(font: &fontdue::Font, description: &str, count: u16, body_y: usize) -> Tooltip {
    let text_x = PADDING + PORTRAIT_SIZE + 12;
    let text_width = STACK_WIDTH - text_x - PADDING;
    let count_height = if count > 1 { 17 } else { 0 };
    let description_height =
        description_height(font, description, text_width, body_y + count_height);
    let body_height = PORTRAIT_SIZE.max(description_height + count_height);
    let mut tooltip = Tooltip::new(STACK_WIDTH, body_y + body_height + 10);
    tooltip.text(
        font,
        description,
        text_x,
        body_y,
        DESCRIPTION_SIZE,
        text_width,
        description_height,
        WHITE,
    );
    if count > 1 {
        tooltip.text(
            font,
            &format!("Quantity: {count}"),
            text_x,
            body_y + description_height,
            10.5,
            text_width,
            count_height,
            ACCENT,
        );
    }
    tooltip
}

fn render_equipment(
    font: &fontdue::Font,
    item: &InventoryItem,
    info: Option<&NxNode>,
    player_stats: Option<&CharacterStats>,
    player_inventory: Option<&InventorySnapshot>,
    description: &str,
    body_y: usize,
) -> Tooltip {
    let details = equipment_details(item, info);
    let classes_y = body_y + 96;
    let detail_y = classes_y + 26;
    let after_details = detail_y + details.len() * DETAIL_STEP;
    let description_y = after_details + 6;
    let text_width = EQUIPMENT_WIDTH - 2 * PADDING;
    let sell_price = info
        .and_then(|node| integer_property(node, "price"))
        .unwrap_or(0)
        / 2;
    let price_height = if sell_price > 0 { 19 } else { 0 };
    let description_height =
        description_height(font, description, text_width, description_y + price_height);
    let after_description = if description_height > 0 {
        description_y + description_height
    } else {
        after_details
    };
    let height = (after_description + price_height + 8).min(MAX_HEIGHT);
    let mut tooltip = Tooltip::new(EQUIPMENT_WIDTH, height);

    draw_requirements(
        &mut tooltip,
        font,
        info,
        item,
        player_stats,
        player_inventory,
        body_y,
    );
    let required_job = info
        .and_then(|node| integer_property(node, "reqJob"))
        .unwrap_or(0);
    draw_classes(&mut tooltip, font, required_job, classes_y);
    tooltip.rule(detail_y - 5);

    let face = ui_foundation::bold_font().unwrap_or(font);
    for (index, (label, value)) in details.iter().enumerate() {
        let y = detail_y + index * DETAIL_STEP;
        tooltip.fill(PADDING, y + 6, 2, 2, [72, 170, 255, 255]);
        tooltip.text(font, label, PADDING + 7, y, 10.5, 114, DETAIL_STEP, LABEL);
        tooltip.text(
            face,
            value,
            138,
            y,
            10.5,
            EQUIPMENT_WIDTH - 138 - PADDING,
            DETAIL_STEP,
            WHITE,
        );
    }
    if description_height > 0 {
        tooltip.rule(after_details + 1);
        tooltip.text(
            font,
            description,
            PADDING,
            description_y,
            DESCRIPTION_SIZE,
            text_width,
            description_height,
            WHITE,
        );
    }
    if sell_price > 0 {
        tooltip.rule(after_description + 3);
        tooltip.text(
            font,
            &format!("Shop sell price: {}", format_meso_amount(sell_price)),
            PADDING,
            after_description + 7,
            9.5,
            text_width,
            14,
            MUTED,
        );
    }
    tooltip
}

fn draw_requirements(
    tooltip: &mut Tooltip,
    font: &fontdue::Font,
    info: Option<&NxNode>,
    item: &InventoryItem,
    player_stats: Option<&CharacterStats>,
    player_inventory: Option<&InventorySnapshot>,
    y: usize,
) {
    let player_values = player_stats.map(|stats| {
        let mut bonuses = [0_i64; 4];
        if let Some(inventory) = player_inventory {
            for equip in inventory.equipped().filter_map(|item| item.equip.as_ref()) {
                for (bonus, stat) in bonuses.iter_mut().zip(equip.stats) {
                    *bonus += i64::from(stat);
                }
            }
        }
        [
            i64::from(stats.level),
            i64::from(stats.strength) + bonuses[0],
            i64::from(stats.dexterity) + bonuses[1],
            i64::from(stats.intelligence) + bonuses[2],
            i64::from(stats.luck) + bonuses[3],
            i64::from(stats.fame),
        ]
    });
    let required_fame = info.and_then(|node| {
        integer_property(node, "reqFAME").or_else(|| integer_property(node, "reqPOP"))
    });
    let requirements = [
        ("Req. level", "reqLevel"),
        ("Req. STR", "reqSTR"),
        ("Req. DEX", "reqDEX"),
        ("Req. INT", "reqINT"),
        ("Req. LUK", "reqLUK"),
        ("Req. fame", "reqFAME"),
    ];
    for (row, (label, key)) in requirements.into_iter().enumerate() {
        let required = if key == "reqFAME" {
            required_fame
        } else {
            info.and_then(|node| integer_property(node, key))
                .or(Some(0))
        };
        let color = if required
            .is_some_and(|value| player_values.is_some_and(|stats| stats[row] < value))
        {
            REQUIREMENT_MISSING
        } else {
            WHITE
        };
        draw_requirement(tooltip, font, label, required, y + row * 11, color);
    }

    let has_progression = info.is_some_and(|node| node.child("level").ok().flatten().is_some())
        || item
            .equip
            .as_ref()
            .is_some_and(|equip| equip.item_level > 1 || equip.item_exp > 0);
    let progression = item.equip.as_ref().filter(|_| has_progression);
    draw_requirement(
        tooltip,
        font,
        "Item level",
        progression.map(|equip| i64::from(equip.item_level)),
        y + 68,
        MUTED,
    );
    draw_requirement(
        tooltip,
        font,
        "Item EXP",
        progression.map(|equip| i64::from(equip.item_exp)),
        y + 79,
        MUTED,
    );
}

fn draw_requirement(
    tooltip: &mut Tooltip,
    font: &fontdue::Font,
    label: &str,
    value: Option<i64>,
    y: usize,
    color: [u8; 3],
) {
    let text = value.map_or_else(|| "–".to_owned(), |value| value.to_string());
    tooltip.text(font, label, 94, y, 10.0, 67, 13, LABEL);
    tooltip.text(
        font,
        &text,
        164,
        y,
        10.0,
        EQUIPMENT_WIDTH - 164 - PADDING,
        13,
        color,
    );
}

fn draw_classes(tooltip: &mut Tooltip, font: &fontdue::Font, required_job: i64, y: usize) {
    const CLASSES: [(&str, i64); 6] = [
        ("Beginner", 32),
        ("Warrior", 1),
        ("Magician", 2),
        ("Bowman", 4),
        ("Thief", 8),
        ("Pirate", 16),
    ];
    const SIZE: f32 = 9.5;
    const INSET: usize = 8;
    const MIN_PADDING: usize = 6;
    const ACTIVE: [u8; 3] = [255, 181, 75];
    const INACTIVE: [u8; 3] = [182, 181, 205];
    let face = ui_foundation::bold_font().unwrap_or(font);
    let rail_width = EQUIPMENT_WIDTH - 2 * INSET;
    let natural_width: f32 = CLASSES
        .iter()
        .map(|(label, _)| ui_foundation::measure_text(face, label, SIZE))
        .sum();
    // Leave a pixel per label for rounding measured widths up to whole pixels.
    let text_space = rail_width - (MIN_PADDING + 1) * CLASSES.len();
    let size = SIZE * (text_space as f32 / natural_width).min(1.0);
    let label_widths =
        CLASSES.map(|(label, _)| ui_foundation::measure_text(face, label, size).ceil() as usize);
    let spare = rail_width - label_widths.iter().sum::<usize>();
    let mut left = INSET;
    for (index, ((label, bit), label_width)) in CLASSES.into_iter().zip(label_widths).enumerate() {
        let padding = (index + 1) * spare / CLASSES.len() - index * spare / CLASSES.len();
        let width = label_width + padding;
        let available = required_job == 0 || required_job > 0 && required_job & bit != 0;
        let background = if available {
            [56, 34, 31, 241]
        } else {
            [41, 25, 23, 234]
        };
        tooltip.fill(left + 1, y, width - 2, 17, background);
        if available {
            tooltip.fill(left + 3, y + 15, width - 6, 1, [75, 181, 255, 255]);
        }
        tooltip.text(
            face,
            label,
            left + padding / 2,
            y + 1,
            size,
            label_width + 1,
            13,
            if available { ACTIVE } else { INACTIVE },
        );
        left += width;
    }
}

fn equipment_details(item: &InventoryItem, info: Option<&NxNode>) -> Vec<(&'static str, String)> {
    let item_prefix = item.item_id / 10_000;
    let category = match item_prefix {
        100 => "Hat",
        101 => "Face accessory",
        102 => "Eye accessory",
        103 => "Earrings",
        104 => "Top",
        105 => "Overall",
        106 => "Bottom",
        107 => "Shoes",
        108 => "Gloves",
        109 => "Shield",
        110 => "Cape",
        111 => "Ring",
        112 => "Pendant",
        113 => "Belt",
        114 => "Medal",
        130 => "One-handed sword",
        131 => "One-handed axe",
        132 => "One-handed mace",
        133 => "Dagger",
        137 => "Wand",
        138 => "Staff",
        140 => "Two-handed sword",
        141 => "Two-handed axe",
        142 => "Two-handed mace",
        143 => "Spear",
        144 => "Polearm",
        145 => "Bow",
        146 => "Crossbow",
        147 => "Claw",
        148 => "Knuckle",
        149 => "Gun",
        170 => "Weapon cover",
        180..=183 => "Pet equipment",
        190 => "Mount",
        191 => "Saddle",
        _ => "Equipment",
    };
    let mut rows = vec![("Category", category.to_owned())];
    if matches!(item_prefix, 130..=149 | 170)
        && let Some(speed) = info.and_then(|node| integer_property(node, "attackSpeed"))
    {
        let label = match speed {
            2 => "Fastest",
            3 => "Faster",
            4 | 5 => "Fast",
            6 => "Normal",
            7 => "Slow",
            8 | 9 => "Very slow",
            _ => "Normal",
        };
        rows.push(("Attack speed", format!("{label} ({speed})")));
    }

    let equip = item.equip.as_ref();
    for (index, (property, label)) in [
        ("incSTR", "STR"),
        ("incDEX", "DEX"),
        ("incINT", "INT"),
        ("incLUK", "LUK"),
        ("incMHP", "HP"),
        ("incMMP", "MP"),
        ("incPAD", "Weapon attack"),
        ("incMAD", "Magic attack"),
        ("incPDD", "Weapon defense"),
        ("incMDD", "Magic defense"),
        ("incACC", "Accuracy"),
        ("incEVA", "Avoidability"),
        ("incHands", "Hands"),
        ("incSpeed", "Speed"),
        ("incJump", "Jump"),
    ]
    .into_iter()
    .enumerate()
    {
        let value = match equip {
            Some(equip) => i64::from(equip.stats[index]),
            None => info
                .and_then(|node| integer_property(node, property))
                .unwrap_or(0),
        };
        if value != 0 {
            let display = if (6..=9).contains(&index) {
                value.to_string()
            } else {
                format!("{value:+}")
            };
            rows.push((label, display));
        }
    }
    let critical_rate = info.and_then(|node| {
        ["incCr", "incCR", "incCriticalRate", "incCrit"]
            .into_iter()
            .find_map(|property| integer_property(node, property))
    });
    if let Some(value) = critical_rate.filter(|value| *value != 0) {
        rows.push(("Critical rate", format!("{value:+}%")));
    }
    let slots = equip.map_or_else(
        || {
            info.and_then(|node| integer_property(node, "tuc"))
                .unwrap_or(0)
        },
        |equip| i64::from(equip.upgrade_slots),
    );
    rows.push(("Upgrades remaining", slots.to_string()));
    rows
}

pub(super) fn position(cursor: Point, width: u32, height: u32) -> Point {
    const OFFSET: f32 = 18.0;
    let fit_axis = |pointer: f32, size: u32, limit: f32| {
        let size = size as f32;
        let after = pointer + OFFSET;
        let position = if after + size <= limit {
            after
        } else {
            pointer - OFFSET - size
        };
        // A scaled viewport can produce fractional pointer coordinates.
        position.clamp(0.0, limit - size).round()
    };
    Point {
        x: fit_axis(cursor.x, width, ui_foundation::LOGICAL_WIDTH),
        y: fit_axis(cursor.y, height, ui_foundation::LOGICAL_HEIGHT),
    }
}

pub(super) fn draw(texture: &GpuTexture, cursor: Point) -> SpriteDraw<'_> {
    let point = position(cursor, texture.width(), texture.height());
    draw_at(texture, point)
}

pub(super) fn comparison_positions(
    cursor: Point,
    primary_width: u32,
    primary_height: u32,
    comparison_width: u32,
    comparison_height: u32,
) -> (Point, Point) {
    const GAP: f32 = 10.0;
    let width = ui_foundation::LOGICAL_WIDTH;
    let height = ui_foundation::LOGICAL_HEIGHT;
    let tallest = primary_height.max(comparison_height);
    let primary = position(cursor, primary_width, tallest);
    let fits_with_overflow = |point: Point| {
        let left_overflow = (-point.x).max(0.0);
        let top_overflow = (-point.y).max(0.0);
        let right_overflow = (point.x + comparison_width as f32 - width).max(0.0);
        let bottom_overflow = (point.y + comparison_height as f32 - height).max(0.0);
        left_overflow <= comparison_width as f32 * 0.5
            && right_overflow <= comparison_width as f32 * 0.5
            && top_overflow <= comparison_height as f32 * 0.5
            && bottom_overflow <= comparison_height as f32 * 0.5
    };
    let horizontal_center = (primary.x + (primary_width as f32 - comparison_width as f32) / 2.0)
        .clamp(0.0, width - comparison_width as f32);
    let candidates = [
        Point {
            x: primary.x + primary_width as f32 + GAP,
            y: primary.y,
        },
        Point {
            x: horizontal_center,
            y: primary.y + primary_height as f32 + GAP,
        },
        Point {
            x: primary.x - GAP - comparison_width as f32,
            y: primary.y,
        },
        Point {
            x: horizontal_center,
            y: primary.y - GAP - comparison_height as f32,
        },
    ];
    let comparison = candidates
        .into_iter()
        .find(|point| fits_with_overflow(*point))
        .unwrap_or(Point {
            x: candidates[2].x.clamp(0.0, width - comparison_width as f32),
            y: primary.y,
        });
    (primary, comparison)
}

pub(super) fn draw_comparison(
    primary: &GpuTexture,
    comparison: &GpuTexture,
    cursor: Point,
) -> [SpriteDraw<'_>; 2] {
    let (primary_point, comparison_point) = comparison_positions(
        cursor,
        primary.width(),
        primary.height(),
        comparison.width(),
        comparison.height(),
    );
    [
        draw_at(primary, primary_point),
        draw_at(comparison, comparison_point),
    ]
}

fn draw_at(texture: &GpuTexture, point: Point) -> SpriteDraw<'_> {
    SpriteDraw {
        texture,
        x: point.x,
        y: point.y,
        width: texture.width() as f32,
        height: texture.height() as f32,
        uv: [0.0, 0.0, 1.0, 1.0],
        tint: [1.0; 3],
        blend_mode: BlendMode::Alpha,
        flip_x: false,
        opacity: 1.0,
    }
}

fn update_tooltip_texture(
    renderer: &GpuRenderer,
    current: Option<&GpuTexture>,
    tooltip: &Tooltip,
) -> Result<Option<GpuTexture>, String> {
    match current {
        Some(texture) if texture.width() == tooltip.width && texture.height() == tooltip.height => {
            renderer
                .update_bgra(texture, &tooltip.pixels)
                .map(|()| None)
        }
        _ => renderer
            .upload_bgra(tooltip.width, tooltip.height, &tooltip.pixels)
            .map(Some)
    }
}

impl ClientApp {
    pub(super) fn update_item_tooltip(&mut self, item: Option<InventoryItem>) {
        self.update_item_tooltip_with_comparison(item, None);
    }

    pub(super) fn update_item_tooltip_with_comparison(
        &mut self,
        item: Option<InventoryItem>,
        comparison: Option<InventoryItem>,
    ) {
        let Some(item) = item else {
            self.inventory_tooltip_texture = None;
            self.inventory_comparison_tooltip_texture = None;
            return;
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let tooltip = render(
            &self.font,
            &item,
            &self.item_archive,
            &self.character_archive,
            &self.string_archive,
            self.active_character
                .as_ref()
                .map(|character| &character.stats),
            self.inventory.as_ref(),
            &mut self.inventory_icon_cache,
            &mut self.item_name_cache,
            false,
        );
        match update_tooltip_texture(renderer, self.inventory_tooltip_texture.as_ref(), &tooltip) {
            Ok(Some(texture)) => self.inventory_tooltip_texture = Some(texture),
            Ok(None) => {}
            Err(error) => {
                self.inventory_tooltip_texture = None;
                eprintln!("OpenStoryRS: could not update item tooltip: {error}");
            }
        }

        let Some(comparison) = comparison else {
            self.inventory_comparison_tooltip_texture = None;
            return;
        };
        let comparison_tooltip = render(
            &self.font,
            &comparison,
            &self.item_archive,
            &self.character_archive,
            &self.string_archive,
            self.active_character
                .as_ref()
                .map(|character| &character.stats),
            self.inventory.as_ref(),
            &mut self.inventory_icon_cache,
            &mut self.item_name_cache,
            true,
        );
        match update_tooltip_texture(
            renderer,
            self.inventory_comparison_tooltip_texture.as_ref(),
            &comparison_tooltip,
        ) {
            Ok(Some(texture)) => self.inventory_comparison_tooltip_texture = Some(texture),
            Ok(None) => {}
            Err(error) => {
                self.inventory_comparison_tooltip_texture = None;
                eprintln!("OpenStoryRS: could not update equipped item tooltip: {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openstory_protocol::EquipInventoryData;

    #[test]
    fn flavor_text_preserves_line_breaks_and_highlighted_phrases() {
        assert_eq!(
            formatted_description("First line\\r\\n#cSecond line#\\nThird line\r\nLast line"),
            "First line\n\u{f0007}Second line\u{f0001}\nThird line\nLast line"
        );
    }

    #[test]
    fn equipment_details_keep_all_live_stats_and_upgrade_slots() {
        let item = InventoryItem {
            item_id: 1_372_038,
            equip: Some(EquipInventoryData {
                stats: std::array::from_fn(|index| index as u16 + 1),
                ..Default::default()
            }),
            ..Default::default()
        };

        let details = equipment_details(&item, None);
        assert_eq!(details.len(), 17);
        assert_eq!(details[7], ("Weapon attack", "7".to_owned()));
        assert_eq!(details[15], ("Jump", "+15".to_owned()));
        assert_eq!(details[16], ("Upgrades remaining", "0".to_owned()));
    }

    #[test]
    fn position_flips_at_canvas_edges_using_the_rendered_dimensions() {
        let cursor = Point { x: 990.4, y: 738.2 };
        let equipment = position(cursor, EQUIPMENT_WIDTH as u32, 278);
        assert_eq!((equipment.x, equipment.y), (724.0, 442.0));

        let stack = position(cursor, STACK_WIDTH as u32, 111);
        assert_eq!((stack.x, stack.y), (680.0, 609.0));
    }
}
