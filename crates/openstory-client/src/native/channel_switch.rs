//! In-game channel selector using the legacy `UIWindow2.img/Channel` artwork.

use fontdue::Font;
use openstory_assets::{NodeValue, NxArchive, NxNode};

pub const PANEL_WIDTH: u32 = 370;
pub const PANEL_HEIGHT: u32 = 168;
pub const DEFAULT_X: f32 = (1024.0 - PANEL_WIDTH as f32) / 2.0;
pub const DEFAULT_Y: f32 = (768.0 - PANEL_HEIGHT as f32) / 2.0;
pub const MAX_VISIBLE_CHANNELS: usize = 20;

const CHANNEL_COLUMNS: usize = 5;
const CHANNEL_ROWS: usize = 4;
const CHANNEL_STRIDE_X: f32 = 70.0;
const CHANNEL_STRIDE_Y: f32 = 20.0;
const CHANNEL_GRID_X: f32 = 12.0;
const CHANNEL_GRID_Y: f32 = 56.0;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Close,
    Change,
    Select(u8),
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

#[derive(Clone)]
struct Sprite {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
    origin_x: f32,
    origin_y: f32,
}

/// Renders the server's channel count, capped to the 5×4 grid in the legacy UI.
/// Channel IDs are zero-based, matching both v83 login selection and CHANGE_CHANNEL.
pub fn render(
    ui_archive: &NxArchive,
    font: &Font,
    world_id: u8,
    channel_count: usize,
    current_channel: u8,
    selected_channel: u8,
) -> Result<Panel, String> {
    let width = PANEL_WIDTH as usize;
    let height = PANEL_HEIGHT as usize;
    let root = ui_archive.root();
    let window = resolve(&root, "UIWindow2.img/Channel")?;
    let mut pixels = vec![0_u8; width * height * 4];
    let mut actions = Vec::with_capacity(MAX_VISIBLE_CHANNELS + 2);

    for name in ["backgrnd", "backgrnd2", "backgrnd3"] {
        let image = sprite(&resolve(&window, name)?)?;
        blit(&mut pixels, width, height, &image, 0.0, 0.0);
    }

    if let Some(world) = optional_resolve(&window, &format!("world/{world_id}"))? {
        let image = sprite(&world)?;
        blit(&mut pixels, width, height, &image, 16.0, 30.0);
    }

    let visible_count = channel_count.min(MAX_VISIBLE_CHANNELS);
    let current = usize::from(current_channel).min(visible_count.saturating_sub(1));
    let selected = usize::from(selected_channel).min(visible_count.saturating_sub(1));
    let cover_current = sprite(&resolve(&window, "channel0")?)?;
    let cover_selected = sprite(&resolve(&window, "channel1")?)?;
    let channel_root = resolve(&window, "ch")?;

    for index in 0..visible_count.min(CHANNEL_COLUMNS * CHANNEL_ROWS) {
        let row = index / CHANNEL_COLUMNS;
        let column = index % CHANNEL_COLUMNS;
        let x = (column as f32 * CHANNEL_STRIDE_X) as f32;
        let y = (row as f32 * CHANNEL_STRIDE_Y) as f32;
        let anchor_x = CHANNEL_GRID_X + x;
        let anchor_y = CHANNEL_GRID_Y + y;

        if index == current {
            blit(
                &mut pixels,
                width,
                height,
                &cover_current,
                anchor_x,
                anchor_y,
            );
        } else if index == selected {
            blit(
                &mut pixels,
                width,
                height,
                &cover_selected,
                anchor_x,
                anchor_y,
            );
        }

        let channel_image = sprite(&resolve(&channel_root, &index.to_string())?)?;
        blit(
            &mut pixels,
            width,
            height,
            &channel_image,
            21.0 + x,
            61.0 + y,
        );
        actions.push(ActionHit {
            bounds: HitRect {
                x: anchor_x,
                y: anchor_y,
                width: CHANNEL_STRIDE_X,
                height: CHANNEL_STRIDE_Y,
            },
            action: Action::Select(index as u8),
        });
    }

    let cancel = sprite(&resolve(&window, "BtCancel/normal/0")?)?;
    let cancel_bounds = sprite_bounds(0.0, 0.0, &cancel);
    blit(&mut pixels, width, height, &cancel, 0.0, 0.0);
    actions.push(ActionHit {
        bounds: cancel_bounds,
        action: Action::Close,
    });

    let change = sprite(&resolve(&window, "BtChange/normal/0")?)?;
    let change_bounds = sprite_bounds(0.0, 0.0, &change);
    let change_disabled = selected == current;
    let button = if change_disabled {
        optional_resolve(&window, "BtChange/disabled/0")?
            .map(|node| sprite(&node))
            .transpose()?
            .unwrap_or(change)
    } else {
        change
    };
    blit(&mut pixels, width, height, &button, 0.0, 0.0);
    if !change_disabled {
        actions.push(ActionHit {
            bounds: change_bounds,
            action: Action::Change,
        });
    }

    draw_text(
        &mut pixels,
        width,
        height,
        font,
        &format!(
            "Channel {} of {}",
            selected.saturating_add(1),
            visible_count
        ),
        132.0,
        143.0,
        [244, 232, 201],
    );

    Ok(Panel {
        width: PANEL_WIDTH,
        height: PANEL_HEIGHT,
        pixels,
        actions,
    })
}

fn sprite_bounds(anchor_x: f32, anchor_y: f32, sprite: &Sprite) -> HitRect {
    HitRect {
        x: anchor_x - sprite.origin_x,
        y: anchor_y - sprite.origin_y,
        width: sprite.width as f32,
        height: sprite.height as f32,
    }
}

fn resolve(node: &NxNode, path: &str) -> Result<NxNode, String> {
    node.resolve(path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("NX node {path:?} was not found"))
}

fn optional_resolve(node: &NxNode, path: &str) -> Result<Option<NxNode>, String> {
    node.resolve(path).map_err(|error| error.to_string())
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
            NodeValue::Vector { x, y } => Some((x as f32, y as f32)),
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
        let (metrics, coverage) = font.rasterize(character, 12.0);
        let left = cursor_x.round() as i32 + metrics.xmin;
        let top = (baseline_y + 12.0).round() as i32 - metrics.height as i32 - metrics.ymin;
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
