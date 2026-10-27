//! v83 Monster Carnival panel assembled from the legacy UI archive.

use fontdue::Font;
use openstory_assets::{NodeValue, NxArchive, NxNode};

pub const PANEL_WIDTH: u32 = 162;
pub const PANEL_HEIGHT: u32 = 190;

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

pub struct Panel {
    pub width: u32,
    pub height: u32,
    /// BGRA pixels, matching `GpuRenderer::upload_bgra`.
    pub pixels: Vec<u8>,
    pub close: HitRect,
    pub side: HitRect,
}

#[derive(Clone, Debug)]
pub struct State {
    pub x: f32,
    pub y: f32,
    pub team: Option<i8>,
    pub my_cp: i16,
    pub my_total_cp: i16,
    pub enemy_cp: i16,
    pub enemy_total_cp: i16,
    pub summon_counts: [i8; 4],
    pub side_view: bool,
}

impl Default for State {
    fn default() -> Self {
        // The C++ panel uses CENTER_OFFSET at (400, 50), which adds the
        // 800x600 content offset in a 1024x768 client viewport.
        Self {
            x: 512.0,
            y: 134.0,
            team: None,
            my_cp: 0,
            my_total_cp: 0,
            enemy_cp: 0,
            enemy_total_cp: 0,
            summon_counts: [0; 4],
            side_view: false,
        }
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

fn render_base(archive: &NxArchive) -> Result<Panel, String> {
    let root = archive.root();
    let carnival = resolve(&root, "UIWindow.img/MonsterCarnival")?;
    let background = sprite(&resolve(&carnival, "backgrnd")?)?;
    let background2 = sprite(&resolve(&carnival, "backgrnd2")?)?;
    let width = PANEL_WIDTH as usize;
    let height = PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];

    // Preserve the original sprite order: backgrnd, then backgrnd2.
    blit(&mut pixels, width, height, &background, 0.0, 0.0);
    blit(&mut pixels, width, height, &background2, 0.0, 0.0);

    for (index, y) in [40.0, 70.0, 100.0, 130.0].into_iter().enumerate() {
        let icon = sprite(&resolve(&carnival, &format!("icon{index}"))?)?;
        blit(&mut pixels, width, height, &icon, 20.0, y);
    }

    let close = HitRect {
        x: 143.0,
        y: 6.0,
        width: 15.0,
        height: 15.0,
    };
    let side_button = sprite(&resolve(&carnival, "BtSide/normal/0")?)?;
    let side = HitRect {
        x: 150.0,
        y: 39.0,
        width: side_button.width as f32,
        height: side_button.height as f32,
    };
    blit(&mut pixels, width, height, &side_button, side.x, side.y);
    draw_close_button(&mut pixels, width, height, close);

    Ok(Panel {
        width: PANEL_WIDTH,
        height: PANEL_HEIGHT,
        pixels,
        close,
        side,
    })
}

pub fn render_state(archive: &NxArchive, font: &Font, state: &State) -> Result<Panel, String> {
    let mut panel = render_base(archive)?;
    let width = panel.width as usize;
    let height = panel.height as usize;
    let pixels = &mut panel.pixels;

    let team_label = match state.team {
        Some(0) => "Team: Maple Red",
        Some(_) => "Team: Maple Blue",
        None => "Team: ?",
    };
    let team_header = HitRect {
        x: 4.0,
        y: 6.0,
        width: 116.0,
        height: 18.0,
    };
    fill_rect(pixels, width, height, team_header, [238, 245, 251, 255]);
    draw_text(
        pixels,
        width,
        height,
        font,
        team_label,
        8.0,
        19.0,
        [55, 50, 64],
    );
    draw_centered_text(
        pixels,
        width,
        height,
        font,
        &format!("CP: {} / {}", state.my_cp, state.my_total_cp),
        width as f32 / 4.0,
        174.0,
        [75, 135, 255],
    );
    draw_centered_text(
        pixels,
        width,
        height,
        font,
        &format!("CP: {} / {}", state.enemy_cp, state.enemy_total_cp),
        3.0 * width as f32 / 4.0,
        174.0,
        [255, 85, 85],
    );
    for (index, y) in [49.0, 79.0, 109.0, 139.0].into_iter().enumerate() {
        draw_text(
            pixels,
            width,
            height,
            font,
            &format!("x {}", state.summon_counts[index]),
            47.0,
            y,
            [255, 255, 255],
        );
    }
    if state.side_view {
        draw_centered_text(
            pixels,
            width,
            height,
            font,
            "Side",
            136.0,
            115.0,
            [255, 255, 255],
        );
    }
    Ok(panel)
}

fn resolve(node: &NxNode, path: &str) -> Result<NxNode, String> {
    super::resolve_ui_path(node, path)?.ok_or_else(|| format!("NX node {path:?} was not found"))
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

fn draw_close_button(canvas: &mut [u8], width: usize, height: usize, rect: HitRect) {
    fill_rect(canvas, width, height, rect, [49, 32, 36, 255]);
    for offset in 0..7_i32 {
        let x = rect.x.round() as i32 + 4 + offset;
        let y = rect.y.round() as i32 + 4 + offset;
        set_pixel(canvas, width, height, x, y, [225, 225, 225, 255]);
        set_pixel(
            canvas,
            width,
            height,
            rect.x.round() as i32 + 10 - offset,
            y,
            [225, 225, 225, 255],
        );
    }
}

fn fill_rect(canvas: &mut [u8], width: usize, height: usize, rect: HitRect, color: [u8; 4]) {
    for y in rect.y as i32..(rect.y + rect.height) as i32 {
        for x in rect.x as i32..(rect.x + rect.width) as i32 {
            set_pixel(canvas, width, height, x, y, color);
        }
    }
}

fn set_pixel(canvas: &mut [u8], width: usize, height: usize, x: i32, y: i32, color: [u8; 4]) {
    if !(0..width as i32).contains(&x) || !(0..height as i32).contains(&y) {
        return;
    }
    let offset = (y as usize * width + x as usize) * 4;
    canvas[offset..offset + 4].copy_from_slice(&[color[2], color[1], color[0], color[3]]);
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
                let offset = (target_y as usize * width + target_x as usize) * 4;
                blend_bgra(
                    &mut canvas[offset..offset + 4],
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
