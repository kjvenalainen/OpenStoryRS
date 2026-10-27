use super::*;

const CLOCK_BACKGROUND: &str = "UIWindow.img/muruengRaid/clock/0";
const CLOCK_NUMBER_ROOT: &str = "UIWindow.img/muruengRaid/number";

#[derive(Clone, Copy)]
pub(super) enum ClockState {
    Time {
        hour: u8,
        minute: u8,
        second: u8,
    },
    Countdown {
        remaining_seconds: i32,
        elapsed_ms: u64,
    },
}

#[derive(Clone, Copy)]
struct ClockTick {
    changed: bool,
    cleared: bool,
}

impl ClockState {
    fn from_update(update: openstory_protocol::ClockUpdate) -> Option<Self> {
        match update {
            openstory_protocol::ClockUpdate::Time {
                hour,
                minute,
                second,
            } => Some(Self::Time {
                hour,
                minute,
                second,
            }),
            openstory_protocol::ClockUpdate::Countdown { seconds } => Some(Self::Countdown {
                remaining_seconds: seconds,
                elapsed_ms: 0,
            }),
            openstory_protocol::ClockUpdate::Clear => None,
        }
    }

    fn advance(&mut self, updates: u8) -> ClockTick {
        let Self::Countdown {
            remaining_seconds,
            elapsed_ms,
        } = self
        else {
            return ClockTick {
                changed: false,
                cleared: false,
            };
        };

        *elapsed_ms = elapsed_ms.saturating_add(u64::from(updates).saturating_mul(8));
        let mut changed = false;
        while *elapsed_ms >= 1_000 {
            *elapsed_ms -= 1_000;
            if *remaining_seconds > 0 {
                *remaining_seconds -= 1;
                changed = true;
            } else {
                return ClockTick {
                    changed: false,
                    cleared: true,
                };
            }
        }
        ClockTick {
            changed,
            cleared: false,
        }
    }
}

pub(super) struct ClockPanel {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) pixels: Vec<u8>,
}

fn bitmap_properties(archive: &NxArchive, path: &str) -> Result<(u16, u16, (i32, i32)), String> {
    let node = resolve_ui_path(&archive.root(), path)?
        .ok_or_else(|| format!("missing v83 clock asset {path:?}"))?;
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("v83 clock asset {path:?} is not a bitmap"))?;
    let origin = node
        .child("origin")
        .map_err(|error| error.to_string())?
        .map(|origin| origin.value())
        .transpose()
        .map_err(|error| error.to_string())?;
    let origin = match origin {
        Some(NodeValue::Vector { x, y }) => (x, y),
        Some(_) => return Err(format!("v83 clock asset {path:?} has a malformed origin")),
        None => (0, 0),
    };
    Ok((bitmap.width(), bitmap.height(), origin))
}

fn draw_number(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    archive: &NxArchive,
    value: i32,
    left: i32,
    center_y: i32,
    digit_width: i32,
) -> Result<(), String> {
    let value = value.rem_euclid(100);
    for (offset, digit) in [value / 10, value % 10].into_iter().enumerate() {
        let path = format!("{CLOCK_NUMBER_ROOT}/{digit}");
        let center_x = left + offset as i32 * digit_width + digit_width / 2;
        composite_nx_ui_bitmap(
            pixels,
            canvas_width,
            canvas_height,
            archive,
            &path,
            center_x,
            center_y,
        )?;
    }
    Ok(())
}

pub(super) fn render(archive: &NxArchive, state: ClockState) -> Result<ClockPanel, String> {
    let (background_width, background_height, background_origin) =
        bitmap_properties(archive, CLOCK_BACKGROUND)?;
    let (digit_width, _, _) = bitmap_properties(archive, &format!("{CLOCK_NUMBER_ROOT}/0"))?;
    let (colon_width, _, _) = bitmap_properties(archive, &format!("{CLOCK_NUMBER_ROOT}/bar"))?;
    let width = usize::from(background_width);
    let height = usize::from(background_height);
    let mut pixels = vec![0; width * height * 4];

    composite_nx_ui_bitmap(
        &mut pixels,
        width,
        height,
        archive,
        CLOCK_BACKGROUND,
        background_origin.0,
        background_origin.1,
    )?;

    let digit_width = i32::from(digit_width);
    let colon_width = i32::from(colon_width);
    let center_y = background_origin.1;
    let (values, separators) = match state {
        ClockState::Time {
            hour,
            minute,
            second,
        } => (
            [
                Some(i32::from(hour)),
                Some(i32::from(minute)),
                Some(i32::from(second)),
            ],
            2,
        ),
        ClockState::Countdown {
            remaining_seconds, ..
        } => {
            let seconds = remaining_seconds.max(0);
            let minutes = seconds / 60;
            let seconds = seconds % 60;
            if minutes >= 100 {
                ([Some(minutes / 60), Some(minutes % 60), Some(seconds)], 2)
            } else {
                ([None, Some(minutes), Some(seconds)], 1)
            }
        }
    };

    let field_count = if separators == 2 { 3 } else { 2 };
    let total_width = field_count as i32 * 2 * digit_width + separators as i32 * colon_width;
    let mut x = background_origin.0 - total_width / 2;
    let first_field = usize::from(field_count == 2);
    for value in values.into_iter().skip(first_field).flatten() {
        draw_number(
            &mut pixels,
            width,
            height,
            archive,
            value,
            x,
            center_y,
            digit_width,
        )?;
        x += digit_width * 2;
        if x < background_origin.0 + total_width / 2 {
            let colon_path = format!("{CLOCK_NUMBER_ROOT}/bar");
            composite_nx_ui_bitmap(
                &mut pixels,
                width,
                height,
                archive,
                &colon_path,
                x + colon_width / 2,
                center_y,
            )?;
            x += colon_width;
        }
    }

    Ok(ClockPanel {
        width: u32::from(background_width),
        height: u32::from(background_height),
        x: 512.0 - background_origin.0 as f32,
        y: 32.0 - background_origin.1 as f32,
        pixels,
    })
}

impl ClientApp {
    pub(super) fn apply_clock_update(&mut self, update: openstory_protocol::ClockUpdate) {
        self.clock_state = ClockState::from_update(update);
        if self.clock_state.is_some() {
            self.refresh_clock_texture();
        } else {
            self.clock_texture = None;
            self.clock_position = None;
        }
    }

    pub(super) fn advance_clock(&mut self, updates: u8) {
        let Some(state) = self.clock_state.as_mut() else {
            return;
        };
        let tick = state.advance(updates);
        if tick.cleared {
            self.clock_state = None;
            self.clock_texture = None;
            self.clock_position = None;
        } else if tick.changed {
            self.refresh_clock_texture();
        }
    }

    pub(super) fn clear_clock(&mut self) {
        self.clock_state = None;
        self.clock_texture = None;
        self.clock_position = None;
    }

    pub(super) fn refresh_clock_texture(&mut self) {
        self.clock_texture = None;
        self.clock_position = None;
        let Some(state) = self.clock_state else {
            return;
        };
        let panel = match render(&self.ui_archive, state) {
            Ok(panel) => panel,
            Err(error) => {
                eprintln!("OpenStoryRS: clock assets unavailable: {error}");
                self.clock_texture = None;
                return;
            }
        };
        let Some(renderer) = self.renderer.as_ref() else {
            self.clock_position = Some((panel.x, panel.y));
            return;
        };
        self.clock_position = Some((panel.x, panel.y));
        match renderer.upload_bgra(panel.width, panel.height, &panel.pixels) {
            Ok(texture) => self.clock_texture = Some(texture),
            Err(error) => {
                eprintln!("OpenStoryRS: could not upload clock: {error}");
                self.clock_texture = None;
            }
        }
    }
}
