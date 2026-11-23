//! In-game channel selector with native channel cards and shared window chrome.

use super::ui_dialog::{self, NxDialog};
use super::ui_foundation::UiRect;

use openstory_assets::{NodeValue, NxArchive, NxNode};

pub const PANEL_WIDTH: u32 = 436;
pub const PANEL_HEIGHT: u32 = 280;
const CLOSE: UiRect = ui_dialog::inventory_close(PANEL_WIDTH as f32);
pub const DEFAULT_X: f32 = (1024.0 - PANEL_WIDTH as f32) / 2.0;
pub const DEFAULT_Y: f32 = (768.0 - PANEL_HEIGHT as f32) / 2.0;
pub const MAX_VISIBLE_CHANNELS: usize = 20;

pub const CHANNEL_COLUMNS: usize = 4;
const CHANNEL_STRIDE_X: f32 = 106.0;
const CHANNEL_STRIDE_Y: f32 = 35.0;
const CHANNEL_GRID_X: f32 = 13.0;
const CHANNEL_GRID_Y: f32 = 68.0;

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

/// Renders the server's channel count, capped to the 4×5 grid in the legacy UI.
/// Channel IDs are zero-based, matching both v83 login selection and CHANGE_CHANNEL.
pub fn render(
    ui_archive: &NxArchive,
    login_ui: &NxArchive,
    font: &fontdue::Font,
    world_id: u8,
    channels: &[openstory_protocol::WorldChannel],
    current_channel: u8,
    selected_channel: u8,
) -> Result<Panel, String> {
    let width = PANEL_WIDTH as usize;
    let height = PANEL_HEIGHT as usize;
    let root = ui_archive.root();
    let window = resolve(&root, "UIWindow2.img/Channel")?;
    let mut pixels = vec![0_u8; width * height * 4];
    let mut actions = Vec::with_capacity(MAX_VISIBLE_CHANNELS + 3);
    if NxDialog::new(&mut pixels, width, height, ui_archive)
        .inventory_window(
            UiRect::new(0.0, 0.0, width as f32, height as f32),
            font,
            "Change Channel",
            CLOSE,
        )
        .is_ok()
    {
        actions.push(ActionHit {
            bounds: HitRect {
                x: CLOSE.x,
                y: CLOSE.y,
                width: CLOSE.width,
                height: CLOSE.height,
            },
            action: Action::Close,
        });
    } else {
        pixels.fill(0);
        draw_legacy_frame(&mut pixels, width, height, &window)?;
    }

    if let Some(world) = optional_resolve(&window, &format!("world/{world_id}"))? {
        let image = sprite(&world)?;
        blit(&mut pixels, width, height, &image, 16.0, 30.0);
    }

    let visible_count = channels.len().min(MAX_VISIBLE_CHANNELS);
    let current = usize::from(current_channel).min(visible_count.saturating_sub(1));
    let selected = usize::from(selected_channel).min(visible_count.saturating_sub(1));
    let login_root = login_ui.root();
    let gauge = sprite(&resolve(
        &login_root,
        "Login.img/WorldSelect/channel/chgauge",
    )?)?;
    let capacity = channels
        .iter()
        .map(|channel| channel.load & !(1 << 30))
        .max()
        .unwrap_or(0)
        .max(20);
    for (index, channel) in channels.iter().take(visible_count).enumerate() {
        let x = CHANNEL_GRID_X + (index % CHANNEL_COLUMNS) as f32 * CHANNEL_STRIDE_X;
        let y = CHANNEL_GRID_Y + (index / CHANNEL_COLUMNS) as f32 * CHANNEL_STRIDE_Y;
        let state = if index == current {
            "disabled"
        } else {
            "normal"
        };
        let mut image = sprite(&resolve(
            &login_root,
            &format!("Login.img/WorldSelect/channel/{index}/{state}"),
        )?)?;
        if index != selected {
            // Keep the native gold treatment on the selected card.
            for pixel in image.pixels.chunks_exact_mut(4) {
                let grey = ((u32::from(pixel[2]) * 299
                    + u32::from(pixel[1]) * 587
                    + u32::from(pixel[0]) * 114)
                    / 1000) as u8;
                pixel[..3].fill(grey);
            }
        }
        blit(&mut pixels, width, height, &image, x, y);
        let load = (channel.load & !(1 << 30)).max(0);
        let gauge_width = (73.0 * (load as f32 / capacity as f32).min(1.0)) as usize;
        if gauge_width > 0 {
            blit_scaled_width(
                &mut pixels,
                width,
                height,
                &gauge,
                x + 43.0,
                y + 21.0,
                gauge_width,
            );
        }
        actions.push(ActionHit {
            bounds: HitRect {
                x,
                y,
                width: image.width as f32,
                height: image.height as f32,
            },
            action: Action::Select(index as u8),
        });
    }

    let cancel = sprite(&resolve(&window, "BtCancel/normal/0")?)?;
    let change = sprite(&resolve(&window, "BtChange/normal/0")?)?;
    let button_y = height as f32 - 26.0;
    let change_x = (width as f32 - change.width as f32 - cancel.width as f32 - 8.0) / 2.0;
    let cancel_x = change_x + change.width as f32 + 8.0;
    let cancel_bounds = sprite_bounds(cancel_x, button_y, &cancel);
    blit(&mut pixels, width, height, &cancel, cancel_x, button_y);
    actions.push(ActionHit {
        bounds: cancel_bounds,
        action: Action::Close,
    });

    let change_bounds = sprite_bounds(change_x, button_y, &change);
    let change_disabled = selected == current;
    let button = if change_disabled {
        optional_resolve(&window, "BtChange/disabled/0")?
            .map(|node| sprite(&node))
            .transpose()?
            .unwrap_or(change)
    } else {
        change
    };
    blit(&mut pixels, width, height, &button, change_x, button_y);
    if !change_disabled {
        actions.push(ActionHit {
            bounds: change_bounds,
            action: Action::Change,
        });
    }

    Ok(Panel {
        width: PANEL_WIDTH,
        height: PANEL_HEIGHT,
        pixels,
        actions,
    })
}

fn draw_legacy_frame(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    window: &NxNode,
) -> Result<(), String> {
    if optional_resolve(window, "backgrnd")?.is_some() {
        for name in ["backgrnd", "backgrnd2", "backgrnd3"] {
            let image = sprite(&resolve(window, name)?)?;
            blit(pixels, width, height, &image, 0.0, 0.0);
        }
    } else {
        // The v83 UIWindow.img layout stores the header, repeating body, and
        // footer separately instead of exposing backgrnd* layers.
        let header = sprite(&resolve(window, "t")?)?;
        let body = sprite(&resolve(window, "c")?)?;
        let footer = sprite(&resolve(window, "s")?)?;
        blit_scaled_width(pixels, width, height, &header, 0.0, 0.0, width);
        let footer_y = height.saturating_sub(footer.height);
        for y in (header.height..footer_y).step_by(body.height.max(1)) {
            blit_scaled_width(pixels, width, height, &body, 0.0, y as f32, width);
        }
        blit_scaled_width(pixels, width, height, &footer, 0.0, footer_y as f32, width);
    }
    Ok(())
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

fn blit_scaled_width(
    canvas: &mut [u8],
    width: usize,
    height: usize,
    image: &Sprite,
    x: f32,
    y: f32,
    draw_width: usize,
) {
    if image.width == 0 || draw_width == 0 {
        return;
    }
    let left = (x - image.origin_x).round() as i32;
    let top = (y - image.origin_y).round() as i32;
    for source_y in 0..image.height {
        let target_y = top + source_y as i32;
        if !(0..height as i32).contains(&target_y) {
            continue;
        }
        for target_offset_x in 0..draw_width {
            let target_x = left + target_offset_x as i32;
            if !(0..width as i32).contains(&target_x) {
                continue;
            }
            let source_x = target_offset_x * image.width / draw_width;
            let source = (source_y * image.width + source_x) * 4;
            let target = (target_y as usize * width + target_x as usize) * 4;
            blend_bgra(
                &mut canvas[target..target + 4],
                &image.pixels[source..source + 4],
            );
        }
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
