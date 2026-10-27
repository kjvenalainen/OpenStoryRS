use std::sync::Arc;

use fontdue::Font;
use openstory_assets::{NodeValue, NxArchive, NxNode};

use super::{BlendMode, Point, SpriteRect, draw_dialogue_text, fill_dialogue_rect, rect_contains};

pub(super) const WIDTH: u32 = 236;
pub(super) const HEIGHT: u32 = 182;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Button {
    Send,
    Cancel,
}

pub(super) struct Panel {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

struct Sprite {
    width: usize,
    height: usize,
    origin_x: f32,
    origin_y: f32,
    pixels: Arc<[u8]>,
}

pub(super) fn render(
    archive: &NxArchive,
    font: &Font,
    input: &str,
    saved_message: Option<&str>,
    whisper: bool,
    attached_name: Option<&str>,
    attached_icon: Option<(u32, u32, Arc<[u8]>)>,
    caret_visible: bool,
) -> Result<Panel, String> {
    let root = "UIWindow2.img/ItemMegaphone";
    let background = load_sprite(archive, &format!("{root}/backgrnd"))?;
    let (width, height) = background
        .as_ref()
        .map(|sprite| (sprite.width, sprite.height))
        .unwrap_or((WIDTH as usize, HEIGHT as usize));
    let mut pixels = vec![0; width * height * 4];
    if let Some(background) = background.as_ref() {
        blit_sprite(&mut pixels, width, height, background, 0.0, 0.0);
    } else {
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            0,
            0,
            width,
            height,
            [36, 27, 20, 248],
        );
        fill_dialogue_rect(
            &mut pixels,
            width,
            height,
            1,
            1,
            width - 2,
            1,
            [122, 82, 40, 255],
        );
    }

    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Item Megaphone",
        16,
        14,
        14.0,
        width.saturating_sub(32),
        18,
        [35, 24, 15],
    );

    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        16,
        60,
        width.saturating_sub(32),
        18,
        [244, 244, 244, 225],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        16,
        60,
        width.saturating_sub(32),
        1,
        [94, 79, 61, 255],
    );
    let displayed_message = saved_message.unwrap_or(input);
    if displayed_message.is_empty() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            "Enter a message",
            20,
            62,
            11.0,
            width.saturating_sub(40),
            14,
            [118, 108, 96],
        );
    } else {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            displayed_message,
            20,
            62,
            11.0,
            width.saturating_sub(40),
            14,
            [20, 18, 15],
        );
        if saved_message.is_none() && caret_visible {
            let caret_x =
                20 + super::dialogue_text_width(font, displayed_message, 11.0).ceil() as i32;
            draw_dialogue_text(
                font,
                &mut pixels,
                width,
                height,
                "|",
                caret_x,
                62,
                11.0,
                width.saturating_sub(caret_x.max(0) as usize + 8),
                14,
                [20, 18, 15],
            );
        }
    }

    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        16,
        90,
        32,
        32,
        [242, 239, 228, 165],
    );
    fill_dialogue_rect(&mut pixels, width, height, 16, 90, 32, 1, [93, 76, 57, 255]);
    if let Some((icon_width, icon_height, icon_pixels)) = attached_icon {
        composite_icon(
            &mut pixels,
            width,
            height,
            &icon_pixels,
            icon_width as usize,
            icon_height as usize,
            16,
            90,
            32,
        );
    }
    let attachment_label = attached_name.unwrap_or("Drag an item here");
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        attachment_label,
        56,
        98,
        10.0,
        width.saturating_sub(66),
        28,
        [56, 43, 31],
    );

    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        10,
        154,
        12,
        12,
        [234, 229, 211, 255],
    );
    if whisper && let Some(check) = load_sprite(archive, &format!("{root}/check"))? {
        blit_sprite(&mut pixels, width, height, &check, 0.0, 0.0);
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        "Whisper icon",
        28,
        153,
        10.0,
        width.saturating_sub(40),
        16,
        [49, 38, 27],
    );

    for name in ["BtOK", "BtCancle"] {
        if let Some(button) = load_sprite(archive, &format!("{root}/{name}/normal/0"))? {
            blit_sprite(&mut pixels, width, height, &button, 0.0, 0.0);
        }
    }

    Ok(Panel {
        width: width as u32,
        height: height as u32,
        pixels,
    })
}

pub(super) fn button_at(archive: &NxArchive, x: f32, y: f32) -> Option<Button> {
    let local = Point { x, y };
    for (button, name) in [(Button::Cancel, "BtCancle"), (Button::Send, "BtOK")] {
        let path = format!("UIWindow2.img/ItemMegaphone/{name}/normal/0");
        if let Ok(Some(rect)) = sprite_rect(archive, &path)
            && rect_contains(rect, local)
        {
            return Some(button);
        }
    }
    // Keep the buttons usable if a server's UI archive omits their state nodes.
    if (144.0..184.0).contains(&x) && (150.0..176.0).contains(&y) {
        Some(Button::Send)
    } else if (188.0..228.0).contains(&x) && (150.0..176.0).contains(&y) {
        Some(Button::Cancel)
    } else {
        None
    }
}

fn sprite_rect(archive: &NxArchive, path: &str) -> Result<Option<SpriteRect>, String> {
    let Some(sprite) = load_sprite(archive, path)? else {
        return Ok(None);
    };
    Ok(Some(SpriteRect {
        x: -sprite.origin_x,
        y: -sprite.origin_y,
        width: sprite.width as f32,
        height: sprite.height as f32,
        blend_mode: BlendMode::Alpha,
    }))
}

fn load_sprite(archive: &NxArchive, path: &str) -> Result<Option<Sprite>, String> {
    let node = super::resolve_ui_path(&archive.root(), path)?;
    let Some(node) = node else {
        return Ok(None);
    };
    sprite_from_node(&node).map(Some)
}

fn sprite_from_node(node: &NxNode) -> Result<Sprite, String> {
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Item Megaphone art node is not a bitmap".to_owned())?;
    let (origin_x, origin_y) = match node
        .child("origin")
        .map_err(|error| error.to_string())?
        .map(|origin| origin.value())
        .transpose()
        .map_err(|error| error.to_string())?
    {
        Some(NodeValue::Vector { x, y }) => (x as f32, y as f32),
        Some(_) => return Err("Item Megaphone sprite has a malformed origin".to_owned()),
        None => (0.0, 0.0),
    };
    Ok(Sprite {
        width: usize::from(bitmap.width()),
        height: usize::from(bitmap.height()),
        origin_x,
        origin_y,
        pixels: bitmap
            .decode_bgra_shared()
            .map_err(|error| error.to_string())?,
    })
}

fn blit_sprite(
    canvas: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    sprite: &Sprite,
    anchor_x: f32,
    anchor_y: f32,
) {
    let x = (anchor_x - sprite.origin_x).round() as i32;
    let y = (anchor_y - sprite.origin_y).round() as i32;
    for source_y in 0..sprite.height {
        let destination_y = y + source_y as i32;
        if destination_y < 0 || destination_y >= canvas_height as i32 {
            continue;
        }
        for source_x in 0..sprite.width {
            let destination_x = x + source_x as i32;
            if destination_x < 0 || destination_x >= canvas_width as i32 {
                continue;
            }
            let source = (source_y * sprite.width + source_x) * 4;
            let destination = (destination_y as usize * canvas_width + destination_x as usize) * 4;
            blend_pixel(canvas, destination, &sprite.pixels[source..source + 4]);
        }
    }
}

fn composite_icon(
    canvas: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    source: &[u8],
    source_width: usize,
    source_height: usize,
    x: usize,
    y: usize,
    target_size: usize,
) {
    if source_width == 0 || source_height == 0 || target_size == 0 {
        return;
    }
    for target_y in 0..target_size {
        let destination_y = y + target_y;
        if destination_y >= canvas_height {
            break;
        }
        let source_y = target_y * source_height / target_size;
        for target_x in 0..target_size {
            let destination_x = x + target_x;
            if destination_x >= canvas_width {
                break;
            }
            let source_x = target_x * source_width / target_size;
            let source_index = (source_y * source_width + source_x) * 4;
            let destination_index = (destination_y * canvas_width + destination_x) * 4;
            blend_pixel(
                canvas,
                destination_index,
                &source[source_index..source_index + 4],
            );
        }
    }
}

fn blend_pixel(canvas: &mut [u8], destination: usize, source: &[u8]) {
    let alpha = u32::from(source[3]);
    if alpha == 0 {
        return;
    }
    let inverse = 255 - alpha;
    let destination_alpha = u32::from(canvas[destination + 3]);
    let output_alpha = alpha + destination_alpha * inverse / 255;
    if output_alpha == 0 {
        return;
    }
    for channel in 0..3 {
        let premultiplied = u32::from(source[channel]) * alpha
            + u32::from(canvas[destination + channel]) * destination_alpha * inverse / 255;
        canvas[destination + channel] = (premultiplied / output_alpha) as u8;
    }
    canvas[destination + 3] = output_alpha as u8;
}
