//! Shared logical layout and typography primitives for the client UI.
//!
//! Gameplay panels are authored in the same 1024x768 coordinate system as the
//! map. Window sizes and reference crops remain native pixel dimensions; this
//! module provides the common measurement and snapping rules used by renderers.

pub(super) const LOGICAL_WIDTH: f32 = 1024.0;
pub(super) const LOGICAL_HEIGHT: f32 = 768.0;
pub(super) const SKILL_MACRO_BUTTON: UiRect = UiRect::new(122.0, 265.0, 48.0, 18.0);
pub(super) const SKILL_CLOSE_BUTTON: UiRect = super::ui_dialog::inventory_close(175.0);
pub(super) const REVIVE_BUTTON: UiRect = UiRect::new(482.0, 425.0, 66.0, 27.0);

pub(super) const STATUS_CLOSE_BUTTON: UiRect =
    super::ui_dialog::inventory_close(super::STATUS_PANEL_WIDTH as f32);
pub(super) const STATUS_AUTO_BUTTON: UiRect = UiRect::new(98.0, 201.0, 73.0, 35.0);
pub(super) const STATUS_DETAIL_BUTTON: UiRect = UiRect::new(124.0, 323.0, 47.0, 18.0);

pub(super) fn status_ap_buttons() -> [(openstory_protocol::AbilityStat, UiRect); 6] {
    use openstory_protocol::AbilityStat;
    [
        (AbilityStat::Hp, 114.0),
        (AbilityStat::Mp, 132.0),
        (AbilityStat::Strength, 244.0),
        (AbilityStat::Dexterity, 262.0),
        (AbilityStat::Intelligence, 280.0),
        (AbilityStat::Luck, 298.0),
    ]
    .map(|(stat, y)| (stat, UiRect::new(155.0, y, 12.0, 12.0)))
}

pub(super) fn skill_tab(tab: usize) -> UiRect {
    UiRect::new(5.0 + tab as f32 * 32.0, 25.0, 32.0, 17.0)
}

pub(super) fn skill_row(row: usize) -> UiRect {
    UiRect::new(6.0, 100.0 + row as f32 * 40.0, 141.0, 35.0)
}

pub(super) fn skill_sp_button(row: usize) -> UiRect {
    UiRect::new(130.0, skill_row(row).y + 21.0, 12.0, 12.0)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct UiRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl UiRect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn centered_in(self, parent: Self) -> Self {
        Self::new(
            parent.x + (parent.width - self.width) * 0.5,
            parent.y + (parent.height - self.height) * 0.5,
            self.width,
            self.height,
        )
    }

    pub fn clamped_to_canvas(self) -> Self {
        let width = self.width.max(0.0).min(LOGICAL_WIDTH);
        let height = self.height.max(0.0).min(LOGICAL_HEIGHT);
        Self::new(
            self.x.clamp(0.0, LOGICAL_WIDTH - width),
            self.y.clamp(0.0, LOGICAL_HEIGHT - height),
            width,
            height,
        )
    }

    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }
}

/// A grid describes both painted cells and pointer targets. Gaps between cells
/// deliberately do not select an item.
#[derive(Clone, Copy)]
pub(super) struct UiGrid {
    pub x: f32,
    pub y: f32,
    pub columns: usize,
    pub rows: usize,
    pub step_x: f32,
    pub step_y: f32,
    pub cell_width: f32,
    pub cell_height: f32,
}

impl UiGrid {
    pub fn cell(self, index: usize) -> UiRect {
        UiRect::new(
            self.x + (index % self.columns) as f32 * self.step_x,
            self.y + (index / self.columns) as f32 * self.step_y,
            self.cell_width,
            self.cell_height,
        )
    }

    pub fn hit(self, x: f32, y: f32) -> Option<usize> {
        (0..self.columns * self.rows).find(|index| self.cell(*index).contains(x, y))
    }
}

pub(super) const INVENTORY_GRID: UiGrid = UiGrid {
    x: super::INVENTORY_SLOT_X as f32,
    y: super::INVENTORY_SLOT_Y as f32,
    columns: super::INVENTORY_SLOT_COLUMNS,
    rows: super::INVENTORY_SLOT_ROWS,
    step_x: super::INVENTORY_SLOT_STEP_X as f32,
    step_y: super::INVENTORY_SLOT_STEP_Y as f32,
    cell_width: super::INVENTORY_SLOT_CELL_WIDTH as f32,
    cell_height: super::INVENTORY_SLOT_CELL_HEIGHT as f32,
};

pub(super) const INVENTORY_CLOSE_BUTTON: UiRect = super::ui_dialog::inventory_close(175.0);
pub(super) const INVENTORY_GATHER_BUTTON: UiRect = UiRect::new(126.0, 6.0, 12.0, 12.0);
pub(super) const INVENTORY_SORT_BUTTON: UiRect = UiRect::new(141.0, 6.0, 12.0, 12.0);
pub(super) const INVENTORY_MESO_BUTTON: UiRect = UiRect::new(5.0, 253.0, 19.0, 23.0);
pub(super) const QUEST_LIST_CLOSE_BUTTON: UiRect =
    super::ui_dialog::close_button(super::QUEST_LOG_LIST_WIDTH as f32 - 18.0, 5.0);
pub(super) const QUEST_DETAIL_CLOSE_BUTTON: UiRect = super::ui_dialog::close_button(
    super::QUEST_LOG_DETAIL_X as f32 + super::QUEST_LOG_DETAIL_WIDTH as f32 - 18.0,
    5.0,
);

pub(super) fn quest_tab(index: usize) -> UiRect {
    UiRect::new(10.0 + index as f32 * 70.0, 23.0, 70.0, 19.0)
}

const QUEST_FOOTER_BUTTON_Y: f32 = super::QUEST_LOG_PANEL_HEIGHT as f32 - 24.0;
pub(super) const QUEST_HELPER_BUTTON: UiRect = UiRect::new(
    super::QUEST_LOG_PANEL_WIDTH as f32 - 154.0,
    QUEST_FOOTER_BUTTON_Y,
    86.0,
    17.0,
);
pub(super) const QUEST_TELEPORT_BUTTON: UiRect = UiRect::new(
    super::QUEST_LOG_PANEL_WIDTH as f32 - 61.0,
    QUEST_FOOTER_BUTTON_Y - 27.0,
    57.0,
    17.0,
);
pub(super) const QUEST_FORFEIT_BUTTON: UiRect = UiRect::new(
    super::QUEST_LOG_PANEL_WIDTH as f32 - 61.0,
    QUEST_FOOTER_BUTTON_Y,
    57.0,
    17.0,
);

pub(super) const QUEST_GRID: UiGrid = UiGrid {
    x: 8.0,
    y: 54.0,
    columns: 1,
    rows: super::QUEST_LOG_VISIBLE_ROWS,
    step_x: 230.0,
    step_y: 20.0,
    cell_width: super::QUEST_LOG_LIST_WIDTH as f32 - 26.0,
    cell_height: 20.0,
};

pub(super) const QUICKSLOT_GRID: UiGrid = UiGrid {
    x: 7.0,
    y: 8.0,
    columns: 4,
    rows: 2,
    step_x: 35.0,
    step_y: 34.0,
    cell_width: 32.0,
    cell_height: 30.0,
};

pub(super) fn inventory_tab(index: usize) -> UiRect {
    UiRect::new(3.0 + index as f32 * 34.0, 25.0, 33.0, 17.0)
}

pub(super) fn storage_tab(index: usize) -> UiRect {
    UiRect::new(240.0 + index as f32 * 42.0, 98.0, 41.0, 21.0)
}

pub(super) fn npc_shop_sell_tab(index: usize) -> UiRect {
    UiRect::new(237.0 + index as f32 * 42.0, 95.0, 41.0, 21.0)
}

/// Scroll offsets are clamped before either rendering or hit testing uses them.
pub(super) fn scroll_fraction(offset: usize, total: usize, visible: usize) -> f32 {
    let maximum = total.saturating_sub(visible);
    if maximum == 0 {
        0.0
    } else {
        offset.min(maximum) as f32 / maximum as f32
    }
}

pub(super) fn snap(value: f32) -> f32 {
    value.round()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UiTextWeight {
    Regular,
    Bold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UiTextRole {
    Title,
    Label,
    Body,
    Small,
    Value,
    Emphasis,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct UiTextStyle {
    pub size: f32,
    pub line_height: f32,
    pub weight: UiTextWeight,
    pub color: [u8; 3],
}

impl UiTextRole {
    /// Reference-matched defaults are in logical pixels. Panel renderers can
    /// override color while retaining shared metrics and weight.
    pub const fn style(self) -> UiTextStyle {
        match self {
            Self::Title => UiTextStyle {
                size: 13.0,
                line_height: 17.0,
                weight: UiTextWeight::Bold,
                color: [20, 20, 20],
            },
            Self::Label => UiTextStyle {
                size: 11.0,
                line_height: 14.0,
                weight: UiTextWeight::Bold,
                color: [34, 34, 34],
            },
            Self::Body => UiTextStyle {
                size: 13.0,
                line_height: 17.0,
                weight: UiTextWeight::Regular,
                color: [32, 32, 32],
            },
            Self::Small => UiTextStyle {
                size: 9.0,
                line_height: 12.0,
                weight: UiTextWeight::Regular,
                color: [48, 48, 48],
            },
            Self::Value => UiTextStyle {
                size: 11.0,
                line_height: 14.0,
                weight: UiTextWeight::Regular,
                color: [24, 24, 24],
            },
            Self::Emphasis => UiTextStyle {
                size: 13.0,
                line_height: 16.0,
                weight: UiTextWeight::Bold,
                color: [20, 20, 20],
            },
        }
    }
}

pub(super) fn measure_text(font: &fontdue::Font, text: &str, size: f32) -> f32 {
    let mut width = 0.0;
    let mut previous = None;
    for character in text.chars() {
        if (0xf0000..=0xf0007).contains(&(character as u32)) {
            continue;
        }
        if let Some(previous) = previous {
            width += font
                .horizontal_kern(previous, character, size)
                .unwrap_or(0.0);
        }
        width += font.metrics(character, size).advance_width;
        previous = Some(character);
    }
    width
}

pub(super) fn draw_tab_label(
    font: &fontdue::Font,
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    label: &str,
    bounds: UiRect,
) {
    draw_control_label(
        font,
        pixels,
        canvas_width,
        canvas_height,
        label,
        bounds,
        [255, 255, 255],
        [62, 54, 47],
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_control_label(
    font: &fontdue::Font,
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    label: &str,
    bounds: UiRect,
    color: [u8; 3],
    outline: [u8; 3],
) {
    const SIZE: f32 = 10.0;
    const LINE_HEIGHT: usize = 12;

    let label_font = bold_font().unwrap_or(font);
    let label = ellipsize(label_font, label, SIZE, (bounds.width - 4.0).max(0.0));
    let label_width = measure_text(label_font, &label, SIZE);
    let x = (bounds.x + ((bounds.width - label_width) / 2.0).max(0.0)).round() as i32;
    let y = (bounds.y + ((bounds.height - LINE_HEIGHT as f32) / 2.0)).round() as i32;
    let max_width = bounds.width.ceil().max(1.0) as usize;
    let max_height = LINE_HEIGHT;
    for (offset_x, offset_y) in [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ] {
        super::draw_dialogue_text(
            label_font,
            pixels,
            canvas_width,
            canvas_height,
            &label,
            x + offset_x,
            y + offset_y,
            SIZE,
            max_width,
            max_height,
            outline,
        );
    }
    super::draw_dialogue_text(
        label_font,
        pixels,
        canvas_width,
        canvas_height,
        &label,
        x,
        y,
        SIZE,
        max_width,
        max_height,
        color,
    );
}

pub(super) fn ellipsize(font: &fontdue::Font, text: &str, size: f32, max_width: f32) -> String {
    if measure_text(font, text, size) <= max_width {
        return text.to_owned();
    }
    let mut shortened = text.to_owned();
    while !shortened.is_empty() {
        shortened.pop();
        let candidate = format!("{shortened}...");
        if measure_text(font, &candidate, size) <= max_width {
            return candidate;
        }
    }
    String::new()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_text_role(
    font: &fontdue::Font,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    text: &str,
    x: i32,
    y: i32,
    role: UiTextRole,
    max_width: usize,
    max_height: usize,
    color: Option<[u8; 3]>,
) {
    let style = role.style();
    let color = color.unwrap_or(style.color);
    let max_height = max_height.min(style.line_height.ceil() as usize);
    let x = snap(x as f32) as i32;
    let y = snap(y as f32) as i32;
    let face = if style.weight == UiTextWeight::Bold {
        bold_font().unwrap_or(font)
    } else {
        font
    };
    let text = ellipsize(face, text, style.size, max_width as f32);
    super::draw_dialogue_text(
        face, pixels, width, height, &text, x, y, style.size, max_width, max_height, color,
    );
}

pub(super) fn bold_font() -> Option<&'static fontdue::Font> {
    static FONT: std::sync::OnceLock<Option<fontdue::Font>> = std::sync::OnceLock::new();
    FONT.get_or_init(|| {
        let configured = std::env::var_os("OPENSTORY_BOLD_FONT_PATH");
        // A custom regular face must not silently switch families for headings.
        if configured.is_none() && std::env::var_os("OPENSTORY_FONT_PATH").is_some() {
            return None;
        }
        // Arial Bold belongs with the Arial regular face. If the regular face
        // fell back to Roboto, keep headings in that family too instead of
        // mixing Arial Bold with Roboto Regular.
        if configured.is_none() && !system_arial_regular_available() {
            return None;
        }
        let paths = configured.map(std::path::PathBuf::from).into_iter().chain([
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf".into(),
            "C:/Windows/Fonts/arialbd.ttf".into(),
            "/usr/share/fonts/truetype/msttcorefonts/Arial_Bold.ttf".into(),
        ]);
        paths
            .filter_map(|path| std::fs::read(path).ok())
            .find_map(|bytes| {
                fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()
            })
    })
    .as_ref()
}

fn system_arial_regular_available() -> bool {
    [
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "C:/Windows/Fonts/arial.ttf",
        "/usr/share/fonts/truetype/msttcorefonts/Arial.ttf",
    ]
    .into_iter()
    .filter_map(|path| std::fs::read(path).ok())
    .any(|bytes| fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_gaps_and_edges_do_not_select_neighboring_slots() {
        let grid = INVENTORY_GRID;
        assert_eq!(grid.hit(grid.x, grid.y), Some(0));
        assert_eq!(grid.hit(grid.x + grid.cell_width, grid.y), None);
        assert_eq!(grid.hit(grid.x, grid.y + grid.cell_height), None);
        let last = grid.cell(23);
        assert_eq!(
            grid.hit(
                last.x + grid.cell_width - 1.0,
                last.y + grid.cell_height - 1.0
            ),
            Some(23)
        );
        assert_eq!(grid.hit(f32::NAN, 0.0), None);
    }

    #[test]
    fn glyphs_and_wrapped_text_stay_inside_the_requested_clip() {
        let font = fontdue::Font::from_bytes(
            include_bytes!("../../assets/fonts/Roboto-Regular.ttf").as_slice(),
            fontdue::FontSettings::default(),
        )
        .unwrap();
        let mut pixels = vec![0; 64 * 48 * 4];
        super::super::draw_dialogue_text(
            &font,
            &mut pixels,
            64,
            48,
            "WWWWWWWWWW WWWWWWWWW",
            12,
            9,
            14.0,
            20,
            17,
            [255, 255, 255],
        );
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
        for y in 0..48 {
            for x in 0..64 {
                if !(12..32).contains(&x) || !(9..26).contains(&y) {
                    assert_eq!(pixels[(y * 64 + x) * 4 + 3], 0, "pixel escaped at {x},{y}");
                }
            }
        }
        // Negative origins must clip safely without unsigned coordinate wraps.
        super::super::draw_dialogue_text(
            &font,
            &mut pixels,
            64,
            48,
            "negative origin",
            -12,
            -3,
            14.0,
            20,
            17,
            [255, 255, 255],
        );
    }

    #[test]
    fn scroll_thumb_is_finite_for_empty_short_and_overscrolled_lists() {
        assert_eq!(scroll_fraction(10, 0, 6), 0.0);
        assert_eq!(scroll_fraction(10, 4, 6), 0.0);
        assert_eq!(scroll_fraction(3, 12, 6), 0.5);
        assert_eq!(scroll_fraction(usize::MAX, 12, 6), 1.0);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_scrollbar(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    ui: &openstory_assets::NxArchive,
    x: i32,
    y: i32,
    track_height: usize,
    fraction: f32,
) {
    if !(51..=2048).contains(&track_height) {
        return;
    }
    let mut track = vec![0; 15 * track_height * 4];
    for row in (0..track_height).step_by(13) {
        let _ = super::composite_nx_ui_bitmap(
            &mut track,
            15,
            track_height,
            ui,
            "Basic.img/VScr5/enabled/base",
            0,
            row as i32,
        );
    }
    for (part, top) in [
        ("prev0", 0),
        ("next0", track_height - 13),
        (
            "thumb0",
            13 + ((track_height - 51) as f32 * fraction.clamp(0.0, 1.0)).round() as usize,
        ),
    ] {
        let _ = super::composite_nx_ui_bitmap(
            &mut track,
            15,
            track_height,
            ui,
            &format!("Basic.img/VScr5/enabled/{part}"),
            0,
            top as i32,
        );
    }
    super::composite_bgra_pixels(pixels, width, height, &track, 15, track_height, x, y);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_scrollbar_for_items(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    ui: &openstory_assets::NxArchive,
    x: i32,
    y: i32,
    track_height: usize,
    offset: usize,
    total_items: usize,
    visible_items: usize,
) {
    if !(51..=2048).contains(&track_height) || total_items == 0 {
        return;
    }
    let mut track = vec![0; 15 * track_height * 4];
    for row in (0..track_height).step_by(13) {
        let _ = super::composite_nx_ui_bitmap(
            &mut track,
            15,
            track_height,
            ui,
            "Basic.img/VScr5/enabled/base",
            0,
            row as i32,
        );
    }
    let inner_height = track_height - 26;
    let visible_fraction = visible_items.min(total_items) as f32 / total_items as f32;
    let thumb_height = ((inner_height as f32 * visible_fraction).round() as usize)
        .clamp(8.min(inner_height), inner_height);
    let thumb_travel = inner_height - thumb_height;
    let fraction = scroll_fraction(offset, total_items, visible_items);
    let thumb_top = 13 + (thumb_travel as f32 * fraction).round() as usize;
    for (part, top) in [("prev0", 0), ("next0", track_height - 13)] {
        let _ = super::composite_nx_ui_bitmap(
            &mut track,
            15,
            track_height,
            ui,
            &format!("Basic.img/VScr5/enabled/{part}"),
            0,
            top as i32,
        );
    }
    let _ = super::composite_nx_ui_bitmap_scaled_to(
        &mut track,
        15,
        track_height,
        ui,
        "Basic.img/VScr5/enabled/thumb0",
        0,
        thumb_top,
        15,
        thumb_height,
    );
    super::composite_bgra_pixels(pixels, width, height, &track, 15, track_height, x, y);
}
