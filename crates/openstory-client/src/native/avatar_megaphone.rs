//! Text layer for the v83 avatar-megaphone banner in Map.nx.

use fontdue::Font;
use openstory_assets::NxArchive;

pub const PANEL_WIDTH: u32 = 225;
pub const PANEL_HEIGHT: u32 = 180;

pub fn render_text(
    archive: &NxArchive,
    font: &Font,
    sender: &str,
    lines: &[String],
) -> Result<Vec<u8>, String> {
    let width = PANEL_WIDTH as usize;
    let height = PANEL_HEIGHT as usize;
    let mut pixels = vec![0; width * height * 4];
    let (name_width, name_height, name_pixels, origin) =
        super::read_ui_bitmap(archive, "MapHelper.img/AvatarMegaphone/name/1")?;
    blit(
        &mut pixels,
        width,
        height,
        name_width as usize,
        name_height as usize,
        &name_pixels,
        34 - origin.0 as i32,
        -6 - origin.1 as i32,
    );

    let size = 12.0;
    let sender = truncate_to_width(font, sender, size, 200.0);
    let sender_width = text_width(font, &sender, size);
    draw_text(
        &mut pixels,
        width,
        height,
        font,
        &sender,
        (PANEL_WIDTH as f32 - sender_width) / 2.0,
        -10.0,
        size,
        [255, 255, 255],
    );

    let mut rows = Vec::new();
    for line in lines {
        for row in wrap_line(font, line, 11.0, 130.0) {
            if rows.len() == 3 {
                break;
            }
            rows.push(row);
        }
        if rows.len() == 3 {
            break;
        }
    }
    for (index, line) in rows.iter().enumerate() {
        let size = 11.0;
        let line_width = text_width(font, line, size);
        draw_text(
            &mut pixels,
            width,
            height,
            font,
            line,
            156.0 - line_width / 2.0,
            20.0 + 32.0 * index as f32,
            size,
            [24, 24, 24],
        );
    }
    Ok(pixels)
}

fn wrap_line(font: &Font, text: &str, size: f32, maximum_width: f32) -> Vec<String> {
    let mut rows = Vec::new();
    let mut row = String::new();
    for word in text.split_whitespace() {
        let candidate = if row.is_empty() {
            word.to_owned()
        } else {
            format!("{row} {word}")
        };
        if !row.is_empty() && text_width(font, &candidate, size) > maximum_width {
            rows.push(std::mem::take(&mut row));
            row.push_str(word);
        } else {
            row = candidate;
        }
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows
}

fn truncate_to_width(font: &Font, text: &str, size: f32, maximum_width: f32) -> String {
    if text_width(font, text, size) <= maximum_width {
        return text.to_owned();
    }
    let mut result = String::new();
    for character in text.chars() {
        let candidate = format!("{result}{character}…");
        if text_width(font, &candidate, size) > maximum_width {
            break;
        }
        result.push(character);
    }
    result.push('…');
    result
}

fn text_width(font: &Font, text: &str, size: f32) -> f32 {
    text.chars()
        .map(|character| font.metrics(character, size).advance_width)
        .sum()
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
    let mut cursor_x = x;
    for character in text.chars() {
        let (metrics, coverage) = font.rasterize(character, size);
        let left = cursor_x.round() as i32 + metrics.xmin;
        let top = (baseline_y + size).round() as i32 - metrics.height as i32 - metrics.ymin;
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

fn blit(
    canvas: &mut [u8],
    width: usize,
    height: usize,
    source_width: usize,
    source_height: usize,
    source: &[u8],
    left: i32,
    top: i32,
) {
    for y in 0..source_height {
        let target_y = top + y as i32;
        if !(0..height as i32).contains(&target_y) {
            continue;
        }
        for x in 0..source_width {
            let target_x = left + x as i32;
            if !(0..width as i32).contains(&target_x) {
                continue;
            }
            let source_offset = (y * source_width + x) * 4;
            let target_offset = (target_y as usize * width + target_x as usize) * 4;
            blend_bgra(
                &mut canvas[target_offset..target_offset + 4],
                &source[source_offset..source_offset + 4],
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
