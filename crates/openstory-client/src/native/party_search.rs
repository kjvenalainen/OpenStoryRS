//! Native Party Search window built from the legacy UI NX sprites.
//!
//! The v83 client never received a Party Search result packet in this repository;
//! the renderer can display rows supplied by a future server extension, while the
//! current Cosmic protocol only supports starting a leader search and pushing
//! matching party invites through the regular party invite packet.

use fontdue::Font;
use openstory_assets::{NxArchive, NxNode};

pub const PANEL_WIDTH: u32 = 305;
pub const PANEL_HEIGHT: u32 = 407;
pub const DEFAULT_X: f32 = 200.0;
pub const DEFAULT_Y: f32 = 100.0;
pub const MAX_RESULTS: usize = 100;
pub const VISIBLE_ROWS: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl HitRect {
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartyEntry {
    pub leader: String,
    pub member_count: i8,
    pub max_members: i8,
    pub min_level: i16,
    pub max_level: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Close,
    Start,
    Stop,
    Register,
    Cancel,
    Pause,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionHit {
    pub bounds: HitRect,
    pub action: Action,
}

pub struct Panel {
    pub width: u32,
    pub height: u32,
    /// BGRA pixels, matching `GpuRenderer::upload_bgra`.
    pub pixels: Vec<u8>,
    pub actions: Vec<ActionHit>,
}

#[derive(Clone, Debug)]
pub struct State {
    pub x: f32,
    pub y: f32,
    pub searching: bool,
    pub results: Vec<PartyEntry>,
    pub scroll_offset: usize,
    pub status: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            x: DEFAULT_X,
            y: DEFAULT_Y,
            searching: false,
            results: Vec::new(),
            scroll_offset: 0,
            status: "Click Start to search for parties".to_owned(),
        }
    }
}

impl State {
    /// Adds one result row, replacing an existing row from the same leader.
    /// The cap prevents a malformed or future server extension from growing UI
    /// state without bound.
    #[allow(dead_code)] // Kept for a future protocol result source.
    pub fn add_party(&mut self, entry: PartyEntry) {
        if let Some(existing) = self
            .results
            .iter_mut()
            .find(|existing| existing.leader == entry.leader)
        {
            *existing = entry;
            return;
        }
        if self.results.len() < MAX_RESULTS {
            self.results.push(entry);
        }
    }

    #[allow(dead_code)] // Kept for a future protocol result source.
    pub fn clear_results(&mut self) {
        self.results.clear();
        self.scroll_offset = 0;
    }

    pub fn set_searching(&mut self, searching: bool) {
        self.searching = searching;
        self.status = if searching {
            "Searching...".to_owned()
        } else {
            "Search stopped".to_owned()
        };
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    pub fn scroll_by(&mut self, rows: isize) {
        let maximum = self.results.len().saturating_sub(VISIBLE_ROWS);
        self.scroll_offset = self.scroll_offset.saturating_add_signed(rows).min(maximum);
    }
}

#[derive(Clone)]
struct Sprite {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
    origin_x: f32,
    origin_y: f32,
}

pub fn render(ui_archive: &NxArchive, font: &Font, state: &State) -> Result<Panel, String> {
    let root = ui_archive.root();
    let window = resolve(&root, "UIWindow.img/PartySearch")?;
    let background = sprite(&resolve(&window, "backgrnd")?)?;
    if background.width != PANEL_WIDTH as usize || background.height != PANEL_HEIGHT as usize {
        return Err(format!(
            "Party Search backdrop is {}x{}, expected {}x{}",
            background.width, background.height, PANEL_WIDTH, PANEL_HEIGHT
        ));
    }

    let width = PANEL_WIDTH as usize;
    let height = PANEL_HEIGHT as usize;
    let mut pixels = background.pixels.clone();
    let mut actions = Vec::with_capacity(6);

    let close = resolve(&root, "Basic.img/BtClose3/normal/0")?;
    let close_sprite = sprite(&close)?;
    blit(&mut pixels, width, height, &close_sprite, 286.0, 6.0);
    actions.push(ActionHit {
        bounds: hit_rect(state, 286.0, 6.0, &close_sprite),
        action: Action::Close,
    });

    let row_label_x = 40.0;
    let row_info_x = 130.0;
    let party_root = resolve(&window, "Party")?;
    let party_rows = (0..4)
        .map(|index| resolve(&party_root, &format!("party{index}")))
        .collect::<Result<Vec<_>, _>>()?;
    let party_rows = party_rows
        .iter()
        .map(sprite)
        .collect::<Result<Vec<_>, _>>()?;

    if state.results.is_empty() {
        draw_centered_text(
            &mut pixels,
            width,
            height,
            font,
            &state.status,
            width as f32 / 2.0,
            200.0,
            [255, 224, 128],
        );
    } else {
        let entries = state
            .results
            .iter()
            .skip(state.scroll_offset)
            .take(VISIBLE_ROWS);
        for (row, entry) in entries.enumerate() {
            let row_y = 40.0 + row as f32 * 30.0;
            let icon_index = entry.member_count.clamp(0, 3) as usize;
            blit(
                &mut pixels,
                width,
                height,
                &party_rows[icon_index],
                10.0,
                row_y,
            );

            let leader = truncate_to_width(font, &entry.leader, 85.0);
            draw_text(
                &mut pixels,
                width,
                height,
                font,
                &leader,
                row_label_x,
                row_y + 2.0,
                [245, 245, 245],
            );
            let member_count = entry.member_count.max(0);
            let max_members = entry.max_members.max(member_count);
            let min_level = entry.min_level.max(1);
            let max_level = entry.max_level.max(min_level);
            let info = format!("{member_count}/{max_members} (Lv.{min_level}-{max_level})");
            draw_text(
                &mut pixels,
                width,
                height,
                font,
                &info,
                row_info_x,
                row_y + 2.0,
                [192, 192, 192],
            );
        }
    }

    // The legacy constructor passed no per-button positions, which placed all
    // five buttons at the window origin. Put the controls in the authored footer.
    let controls = if state.searching {
        [
            ("BtStart", Action::Start, 8.0, 362.0, true),
            ("BtStop", Action::Stop, 76.0, 362.0, false),
            ("BtPause", Action::Pause, 144.0, 362.0, false),
            ("BtReg", Action::Register, 89.0, 384.0, false),
            ("BtCancel", Action::Cancel, 169.0, 384.0, false),
        ]
    } else {
        [
            ("BtStart", Action::Start, 8.0, 362.0, false),
            ("BtStop", Action::Stop, 76.0, 362.0, true),
            ("BtPause", Action::Pause, 144.0, 362.0, true),
            ("BtReg", Action::Register, 89.0, 384.0, false),
            ("BtCancel", Action::Cancel, 169.0, 384.0, false),
        ]
    };
    for (name, action, x, y, disabled) in controls {
        let state_path = if disabled { "disabled/0" } else { "normal/0" };
        let path = format!("{name}/{state_path}");
        let button = optional_resolve(&window, &path)?
            .or(optional_resolve(&window, &format!("{name}/normal/0"))?)
            .ok_or_else(|| format!("Party Search button {name} has no normal sprite"))?;
        let image = sprite(&button)?;
        blit(&mut pixels, width, height, &image, x, y);
        if !disabled {
            actions.push(ActionHit {
                bounds: hit_rect(state, x, y, &image),
                action,
            });
        }
    }

    Ok(Panel {
        width: PANEL_WIDTH,
        height: PANEL_HEIGHT,
        pixels,
        actions,
    })
}

fn hit_rect(state: &State, anchor_x: f32, anchor_y: f32, sprite: &Sprite) -> HitRect {
    HitRect {
        x: state.x + anchor_x - sprite.origin_x,
        y: state.y + anchor_y - sprite.origin_y,
        width: sprite.width as f32,
        height: sprite.height as f32,
    }
}

fn resolve(node: &NxNode, path: &str) -> Result<NxNode, String> {
    super::resolve_ui_path(node, path)?.ok_or_else(|| format!("NX node {path:?} was not found"))
}

fn optional_resolve(node: &NxNode, path: &str) -> Result<Option<NxNode>, String> {
    super::resolve_ui_path(node, path)
}

fn sprite(node: &NxNode) -> Result<Sprite, String> {
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!(
                "NX node {:?} is not a bitmap",
                node.name().unwrap_or_default()
            )
        })?;
    let (origin_x, origin_y) = node
        .child("origin")
        .map_err(|error| error.to_string())?
        .and_then(|origin| match origin.value().ok()? {
            openstory_assets::NodeValue::Vector { x, y } => Some((x as f32, y as f32)),
            _ => None,
        })
        .unwrap_or((0.0, 0.0));
    Ok(Sprite {
        pixels: bitmap
            .decode_bgra_shared()
            .map_err(|error| error.to_string())?
            .to_vec(),
        width: usize::from(bitmap.width()),
        height: usize::from(bitmap.height()),
        origin_x,
        origin_y,
    })
}

fn blit(canvas: &mut [u8], width: usize, height: usize, image: &Sprite, x: f32, y: f32) {
    let left = (x - image.origin_x).round() as i32;
    let top = (y - image.origin_y).round() as i32;
    for source_y in 0..image.height {
        let target_y = top + source_y as i32;
        if !(0..height as i32).contains(&target_y) {
            continue;
        }
        for source_x in 0..image.width {
            let target_x = left + source_x as i32;
            if !(0..width as i32).contains(&target_x) {
                continue;
            }
            let source = (source_y * image.width + source_x) * 4;
            let target = (target_y as usize * width + target_x as usize) * 4;
            blend_bgra(
                &mut canvas[target..target + 4],
                &image.pixels[source..source + 4],
            );
        }
    }
}

fn draw_centered_text(
    canvas: &mut [u8],
    width: usize,
    height: usize,
    font: &Font,
    text: &str,
    center_x: f32,
    baseline_y: f32,
    color: [u8; 3],
) {
    let text_width = text
        .chars()
        .map(|character| font.metrics(character, 14.0).advance_width)
        .sum::<f32>();
    draw_text(
        canvas,
        width,
        height,
        font,
        text,
        center_x - text_width / 2.0,
        baseline_y,
        color,
    );
}

fn draw_text(
    canvas: &mut [u8],
    width: usize,
    height: usize,
    font: &Font,
    text: &str,
    x: f32,
    baseline_y: f32,
    color: [u8; 3],
) {
    let mut cursor_x = x;
    for character in text.chars() {
        let (metrics, coverage) = font.rasterize(character, 14.0);
        let left = cursor_x.round() as i32 + metrics.xmin;
        let top = (baseline_y + 14.0).round() as i32 - metrics.height as i32 - metrics.ymin;
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                let target_x = left + column as i32;
                let target_y = top + row as i32;
                if !(0..width as i32).contains(&target_x) || !(0..height as i32).contains(&target_y)
                {
                    continue;
                }
                let alpha = coverage[row * metrics.width + column];
                if alpha == 0 {
                    continue;
                }
                let index = (target_y as usize * width + target_x as usize) * 4;
                blend_bgra(
                    &mut canvas[index..index + 4],
                    &[color[2], color[1], color[0], alpha],
                );
            }
        }
        cursor_x += metrics.advance_width;
    }
}

fn blend_bgra(destination: &mut [u8], source: &[u8]) {
    let alpha = u16::from(source[3]);
    if alpha == 0 {
        return;
    }
    let inverse = 255 - alpha;
    for channel in 0..3 {
        destination[channel] = ((u16::from(source[channel]) * alpha
            + u16::from(destination[channel]) * inverse)
            / 255) as u8;
    }
    destination[3] = (alpha + u16::from(destination[3]) * inverse / 255).min(255) as u8;
}

fn truncate_to_width(font: &Font, text: &str, maximum_width: f32) -> String {
    let text_width = |value: &str| {
        value
            .chars()
            .map(|character| font.metrics(character, 14.0).advance_width)
            .sum::<f32>()
    };
    if text_width(text) <= maximum_width {
        return text.to_owned();
    }
    let mut result = String::new();
    for character in text.chars() {
        if text_width(&format!("{result}{character}…")) > maximum_width {
            break;
        }
        result.push(character);
    }
    result.push('…');
    result
}
