//! v83 Monster Carnival panel assembled from the legacy UI archive.

use fontdue::Font;
use openstory_assets::{NodeValue, NxArchive, NxNode};

pub const PANEL_WIDTH: u32 = 162;
pub const PANEL_HEIGHT: u32 = 107;

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

fn render_base(archive: &NxArchive, side_view: bool) -> Result<Panel, String> {
    let root = archive.root();
    let carnival = resolve(&root, "UIWindow.img/MonsterCarnival")?;
    let width = PANEL_WIDTH as usize;
    let height = PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];

    let mut dialog = super::NxDialog::new(&mut pixels, width, height, archive);
    dialog.frame("UIWindow.img/MonsterCarnival/backgrnd", 0, 0)?;
    if side_view {
        dialog.frame("UIWindow.img/MonsterCarnival/backgrnd2", 0, 1)?;
        for (index, y) in [23.0, 42.0, 61.0, 80.0].into_iter().enumerate() {
            let icon = sprite(&resolve(&carnival, &format!("icon{index}"))?)?;
            blit(&mut pixels, width, height, &icon, 10.0, y);
        }
    }

    let close = HitRect {
        x: 143.0,
        y: 6.0,
        width: 12.0,
        height: 12.0,
    };
    let side_button = sprite(&resolve(&carnival, "BtSide/normal/0")?)?;
    let side = HitRect {
        x: 150.0,
        y: 39.0,
        width: side_button.width as f32,
        height: side_button.height as f32,
    };
    blit(&mut pixels, width, height, &side_button, side.x, side.y);
    super::NxDialog::new(&mut pixels, width, height, archive).anchored_button(
        super::ui_dialog::CLOSE_BUTTON_ART,
        close.x as i32,
        close.y as i32,
    )?;

    Ok(Panel {
        width: PANEL_WIDTH,
        height: PANEL_HEIGHT,
        pixels,
        close,
        side,
    })
}

pub fn render_state(archive: &NxArchive, font: &Font, state: &State) -> Result<Panel, String> {
    let mut panel = render_base(archive, state.side_view)?;
    let width = panel.width as usize;
    let height = panel.height as usize;
    let pixels = &mut panel.pixels;

    if state.side_view {
        for (index, y) in [24.0, 43.0, 62.0, 81.0].into_iter().enumerate() {
            draw_text(
                pixels,
                width,
                height,
                font,
                &format!("x {}", state.summon_counts[index]),
                33.0,
                y,
                10.0,
                [45, 45, 45],
            );
        }
    } else {
        for (value, y) in [
            (format!("{}", state.my_cp), 31.0),
            (format!("{} / {}", state.my_cp, state.my_total_cp), 53.0),
            (
                format!("{} / {}", state.enemy_cp, state.enemy_total_cp),
                75.0,
            ),
        ] {
            draw_centered_text(pixels, width, height, font, &value, 124.0, y, [40, 40, 40]);
        }
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
    let mut size = 10.0;
    let mut text_width = super::ui_foundation::measure_text(font, text, size);
    while text_width > 52.0 && size > 6.0 {
        size -= 1.0;
        text_width = super::ui_foundation::measure_text(font, text, size);
    }
    draw_text(
        canvas,
        width,
        height,
        font,
        text,
        center_x - text_width / 2.0,
        baseline_y,
        size,
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
    size: f32,
    color: [u8; 3],
) {
    super::draw_dialogue_text(
        font,
        canvas,
        width,
        height,
        text,
        x.round() as i32,
        baseline_y.round() as i32,
        size,
        width,
        height,
        color,
    );
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
