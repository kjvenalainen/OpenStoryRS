//! Shared logical layout and typography primitives for the client UI.
//!
//! Gameplay panels are authored in the same 1024x768 coordinate system as the
//! map. Window sizes and reference crops remain native pixel dimensions; this
//! module provides the common measurement and snapping rules used by renderers.

pub(super) const LOGICAL_WIDTH: f32 = 1024.0;
pub(super) const LOGICAL_HEIGHT: f32 = 768.0;

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
                size: 14.0,
                line_height: 18.0,
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
    super::draw_dialogue_text(
        font, pixels, width, height, text, x, y, style.size, max_width, max_height, color,
    );
    if style.weight == UiTextWeight::Bold {
        super::draw_dialogue_text(
            font,
            pixels,
            width,
            height,
            text,
            x + 1,
            y,
            style.size,
            max_width.saturating_sub(1),
            max_height,
            color,
        );
    }
}
