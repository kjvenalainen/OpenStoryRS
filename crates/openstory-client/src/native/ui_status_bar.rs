fn rect_contains(rect: SpriteRect, point: Point) -> bool {
    point.x >= rect.x
        && point.y >= rect.y
        && point.x < rect.x + rect.width
        && point.y < rect.y + rect.height
}

fn render_status_message_line(font: &fontdue::Font, text: &str, color: [u8; 3]) -> Vec<u8> {
    const TEXT_SIZE: f32 = 12.0;
    let width = STATUS_MESSAGE_WIDTH as usize;
    let height = STATUS_MESSAGE_HEIGHT as usize;
    let mut pixels = vec![0_u8; width * height * 4];
    let text_width = dialogue_text_width(font, text, TEXT_SIZE).ceil() as usize;
    let x = width.saturating_sub(text_width.saturating_add(2));
    let max_width = width.saturating_sub(x + 1);
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        text,
        x as i32 + 1,
        2,
        TEXT_SIZE,
        max_width,
        height.saturating_sub(2),
        [0, 0, 0],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        text,
        x as i32,
        1,
        TEXT_SIZE,
        max_width,
        height.saturating_sub(2),
        color,
    );
    pixels
}

#[derive(Clone, Copy)]
struct SpriteRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    blend_mode: BlendMode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StatusBarLayout {
    Modern,
    V83,
}

impl StatusBarLayout {
    fn top(self) -> f32 {
        match self {
            Self::Modern => 683.0,
            Self::V83 => 697.0,
        }
    }

    fn height(self) -> u32 {
        match self {
            Self::Modern => 85,
            Self::V83 => 71,
        }
    }
}

#[derive(Clone, Copy)]
struct StatusBarGauge {
    texture_index: usize,
    start_texture_index: Option<usize>,
    end_texture_index: Option<usize>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy)]
struct StatusBarGaugeAsset {
    fill_index: usize,
    start_index: Option<usize>,
    end_index: Option<usize>,
}

struct BossHpGaugePanel {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

fn build_boss_hp_gauge_panel(
    archive: &NxArchive,
    font: &fontdue::Font,
    mob_id: i32,
    name: &str,
    percentage: u8,
) -> Result<BossHpGaugePanel, String> {
    const WIDTH: usize = 420;
    const HEIGHT: usize = 56;
    const BAR_X: i32 = 60;
    const BAR_Y: i32 = 29;
    const BAR_WIDTH: i32 = 348;

    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    composite_ui_bitmap(&mut pixels, WIDTH, HEIGHT, archive, "UIWindow.img/MobGage/backgrnd", 12, 9)?;
    let icon_path = format!("UIWindow.img/MobGage/Mob/{mob_id}");
    if archive
        .root()
        .resolve(&icon_path)
        .map_err(|error| error.to_string())?
        .is_some()
    {
        composite_ui_bitmap(&mut pixels, WIDTH, HEIGHT, archive, &icon_path, 17, 15)?;
    }

    composite_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        archive,
        "UIWindow.img/MobGage/backgrnd2",
        BAR_X,
        BAR_Y,
    )?;
    composite_tiled_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        archive,
        "UIWindow.img/MobGage/backgrnd3",
        BAR_X + 4,
        BAR_X + BAR_WIDTH - 5,
        BAR_Y,
    )?;
    composite_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        archive,
        "UIWindow.img/MobGage/backgrnd4",
        BAR_X + BAR_WIDTH - 5,
        BAR_Y,
    )?;

    let fill_start = BAR_X + 4;
    let fill_width = (BAR_WIDTH - 9) * i32::from(percentage) / 100;
    let gauge_base = "UIWindow.img/MobGage/Gage/1/0";
    let gauge_highlight = "UIWindow.img/MobGage/Gage/1/1";
    composite_tiled_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        archive,
        gauge_base,
        fill_start,
        fill_start + fill_width,
        BAR_Y + 5,
    )?;
    composite_tiled_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        archive,
        gauge_highlight,
        fill_start,
        fill_start + fill_width,
        BAR_Y + 4,
    )?;

    let label = rasterize_label_color(font, name, 14.0, 345, 20, [255, 255, 255]);
    composite_bgra_pixels(&mut pixels, WIDTH, HEIGHT, &label, 345, 20, BAR_X, 4);

    Ok(BossHpGaugePanel {
        width: WIDTH as u32,
        height: HEIGHT as u32,
        pixels,
    })
}

fn composite_ui_bitmap(
    destination: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    archive: &NxArchive,
    path: &str,
    anchor_x: i32,
    anchor_y: i32,
) -> Result<(), String> {
    let node = archive
        .root()
        .resolve(path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("missing v83 UI asset {path:?}"))?;
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("v83 UI asset {path:?} is not a bitmap"))?;
    let origin = node
        .child("origin")
        .map_err(|error| error.to_string())?
        .map(|origin| origin.value())
        .transpose()
        .map_err(|error| error.to_string())?;
    let (origin_x, origin_y) = match origin {
        Some(NodeValue::Vector { x, y }) => (x, y),
        Some(_) => return Err(format!("v83 UI asset {path:?} has a malformed origin")),
        None => (0, 0),
    };
    composite_bgra_pixels(
        destination,
        canvas_width,
        canvas_height,
        &bitmap
            .decode_bgra_shared()
            .map_err(|error| error.to_string())?,
        usize::from(bitmap.width()),
        usize::from(bitmap.height()),
        anchor_x - origin_x,
        anchor_y - origin_y,
    );
    Ok(())
}

fn composite_tiled_ui_bitmap(
    destination: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    archive: &NxArchive,
    path: &str,
    start_x: i32,
    end_x: i32,
    anchor_y: i32,
) -> Result<(), String> {
    let node = archive
        .root()
        .resolve(path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("missing v83 UI asset {path:?}"))?;
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("v83 UI asset {path:?} is not a bitmap"))?;
    let origin = node
        .child("origin")
        .map_err(|error| error.to_string())?
        .map(|origin| origin.value())
        .transpose()
        .map_err(|error| error.to_string())?;
    let (origin_x, origin_y) = match origin {
        Some(NodeValue::Vector { x, y }) => (x, y),
        Some(_) => return Err(format!("v83 UI asset {path:?} has a malformed origin")),
        None => (0, 0),
    };
    let pixels = bitmap
        .decode_bgra_shared()
        .map_err(|error| error.to_string())?;
    for anchor_x in start_x..end_x {
        composite_bgra_pixels(
            destination,
            canvas_width,
            canvas_height,
            &pixels,
            usize::from(bitmap.width()),
            usize::from(bitmap.height()),
            anchor_x - origin_x,
            anchor_y - origin_y,
        );
    }
    Ok(())
}

fn composite_bgra_pixels(
    destination: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    source: &[u8],
    source_width: usize,
    source_height: usize,
    left: i32,
    top: i32,
) {
    for source_y in 0..source_height {
        let destination_y = top + source_y as i32;
        if !(0..canvas_height as i32).contains(&destination_y) {
            continue;
        }
        for source_x in 0..source_width {
            let destination_x = left + source_x as i32;
            if !(0..canvas_width as i32).contains(&destination_x) {
                continue;
            }
            let source_index = (source_y * source_width + source_x) * 4;
            let destination_index = (destination_y as usize * canvas_width
                + destination_x as usize)
                * 4;
            let source_alpha = u32::from(source[source_index + 3]);
            if source_alpha == 0 {
                continue;
            }
            let destination_alpha = u32::from(destination[destination_index + 3]);
            let inverse = 255 - source_alpha;
            let output_alpha = source_alpha + destination_alpha * inverse / 255;
            for channel in 0..3 {
                let foreground = u32::from(source[source_index + channel]);
                let background = u32::from(destination[destination_index + channel]);
                let premultiplied = foreground * source_alpha
                    + background * destination_alpha * inverse / 255;
                destination[destination_index + channel] =
                    (premultiplied / output_alpha.max(1)) as u8;
            }
            destination[destination_index + 3] = output_alpha as u8;
        }
    }
}

struct StatusBarNumberGlyph {
    width: u32,
    height: u32,
    origin_x: i32,
    origin_y: i32,
    pixels: Arc<[u8]>,
}

struct StatusBarNumberFont {
    glyphs: HashMap<char, StatusBarNumberGlyph>,
}

fn load_status_bar_number_font(
    archive: &NxArchive,
    path: &str,
) -> Result<StatusBarNumberFont, String> {
    let node = archive
        .root()
        .resolve(path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("missing {path}"))?;
    let mut glyphs = HashMap::new();
    for child in node.children().map_err(|error| error.to_string())? {
        let name = child.name().map_err(|error| error.to_string())?;
        let Some(bitmap) = child.bitmap().map_err(|error| error.to_string())? else {
            continue;
        };
        let character = match name.as_str() {
            "Lbracket" => 'L',
            "Rbracket" => 'R',
            "slash" => 's',
            "percent" => 'p',
            _ => match name.chars().next() {
                Some(character) => character,
                None => continue,
            },
        };
        let origin = child
            .child("origin")
            .map_err(|error| error.to_string())?
            .map(|origin| origin.value())
            .transpose()
            .map_err(|error| error.to_string())?;
        let (origin_x, origin_y) = match origin {
            Some(NodeValue::Vector { x, y }) => (x, y),
            Some(_) => return Err(format!("malformed origin for status-bar glyph {name:?}")),
            None => (0, 0),
        };
        glyphs.insert(
            character,
            StatusBarNumberGlyph {
                width: u32::from(bitmap.width()),
                height: u32::from(bitmap.height()),
                origin_x,
                origin_y,
                pixels: bitmap
                    .decode_bgra_shared()
                    .map_err(|error| error.to_string())?,
            },
        );
    }
    if !('0'..='9').all(|digit| glyphs.contains_key(&digit)) {
        return Err("StatusBar.img/number is missing one or more digit glyphs".to_owned());
    }
    Ok(StatusBarNumberFont { glyphs })
}

fn draw_status_bar_number(
    font: &StatusBarNumberFont,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    text: &str,
    right: i32,
    y: i32,
) {
    draw_status_bar_number_scaled(font, pixels, width, height, text, right, y, 1.0, None);
}

fn draw_status_bar_number_scaled(
    font: &StatusBarNumberFont,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    text: &str,
    right: i32,
    y: i32,
    scale: f32,
    tint: Option<[u8; 3]>,
) {
    let mut shift = 0_i32;
    for character in text.chars().rev() {
        let character = match character {
            '[' => 'L',
            ']' => 'R',
            '/' => 's',
            '%' => 'p',
            character => character,
        };
        let Some(glyph) = font.glyphs.get(&character) else {
            continue;
        };
        let glyph_width = (glyph.width as f32 * scale).round() as i32;
        let glyph_height = (glyph.height as f32 * scale).round() as i32;
        let left = right - shift - (glyph.origin_x as f32 * scale).round() as i32;
        let top = y - (glyph.origin_y as f32 * scale).round() as i32;
        shift += glyph_width;
        for destination_offset_y in 0..glyph_height {
            let destination_y = top + destination_offset_y;
            if !(0..height as i32).contains(&destination_y) {
                continue;
            }
            let source_y = ((destination_offset_y as f32 / scale).floor() as i32)
                .clamp(0, glyph.height as i32 - 1);
            for destination_offset_x in 0..glyph_width {
                let destination_x = left + destination_offset_x;
                if !(0..width as i32).contains(&destination_x) {
                    continue;
                }
                let source_x = ((destination_offset_x as f32 / scale).floor() as i32)
                    .clamp(0, glyph.width as i32 - 1);
                let source = (source_y as usize * glyph.width as usize + source_x as usize) * 4;
                let destination = (destination_y as usize * width + destination_x as usize) * 4;
                let alpha = u16::from(glyph.pixels[source + 3]);
                if alpha == 0 {
                    continue;
                }
                let mut source_pixel = [
                    glyph.pixels[source],
                    glyph.pixels[source + 1],
                    glyph.pixels[source + 2],
                ];
                if let Some([red, green, blue]) = tint {
                    source_pixel[0] = ((u16::from(source_pixel[0]) * u16::from(blue)) / 255) as u8;
                    source_pixel[1] = ((u16::from(source_pixel[1]) * u16::from(green)) / 255) as u8;
                    source_pixel[2] = ((u16::from(source_pixel[2]) * u16::from(red)) / 255) as u8;
                }
                let destination_alpha = u16::from(pixels[destination + 3]);
                if destination_alpha == 0 {
                    pixels[destination..destination + 3].copy_from_slice(&source_pixel);
                    pixels[destination + 3] = alpha as u8;
                    continue;
                }
                let inverse = 255 - alpha;
                let output_alpha = alpha + (destination_alpha * inverse + 127) / 255;
                for channel in 0..3 {
                    let source_color = u32::from(source_pixel[channel]);
                    let destination_color = u32::from(pixels[destination + channel]);
                    let numerator = source_color * u32::from(alpha)
                        + (destination_color * u32::from(destination_alpha) * u32::from(inverse)
                            + 127)
                            / 255;
                    pixels[destination + channel] =
                        (numerator / u32::from(output_alpha.max(1))) as u8;
                }
                pixels[destination + 3] = output_alpha as u8;
            }
        }
    }
}

const V83_EXP_TO_NEXT_LEVEL: [i64; 201] = [
    1, 15, 34, 57, 92, 135, 372, 560, 840, 1242, 1144, 1573, 2144, 2800, 3640, 4700, 5893, 7360,
    9144, 11120, 13477, 16268, 19320, 22880, 27008, 31477, 36600, 42444, 48720, 55813, 63800,
    86784, 98208, 110932, 124432, 139372, 155865, 173280, 192400, 213345, 235372, 259392, 285532,
    312928, 342624, 374760, 408336, 445544, 483532, 524160, 567772, 598886, 631704, 666321, 702836,
    741351, 781976, 824828, 870028, 917625, 967995, 1021041, 1076994, 1136013, 1198266, 1263930,
    1333194, 1406252, 1483314, 1564600, 1650340, 1740778, 1836173, 1936794, 2042930, 2154882,
    2272970, 2397528, 2528912, 2667496, 2813674, 2967863, 3130502, 3302053, 3483005, 3673873,
    3875201, 4087562, 4311559, 4547832, 4797053, 5059931, 5337215, 5629694, 5938202, 6263614,
    6606860, 6968915, 7350811, 7753635, 8178534, 8626718, 9099462, 9598112, 10124088, 10678888,
    11264090, 11881362, 12532461, 13219239, 13943653, 14707765, 15513750, 16363902, 17260644,
    18206527, 19204245, 20256637, 21366700, 22537594, 23772654, 25075395, 26449526, 27898960,
    29427822, 31040466, 32741483, 34535716, 36428273, 38424542, 40530206, 42751262, 45094030,
    47565183, 50171755, 52921167, 55821246, 58880250, 62106888, 65510344, 69100311, 72887008,
    76881216, 81094306, 85594273, 90225770, 95170142, 100385466, 105886589, 111689174, 117809740,
    124265714, 131075474, 138258410, 145834970, 153826726, 162256430, 171148082, 180526997,
    190419876, 200854885, 211861732, 223471711, 223471711, 248635353, 262260570, 276632449,
    291791906, 307782102, 324648562, 342439302, 361204976, 380999008, 401877754, 423900654,
    447130410, 471633156, 497478653, 524740482, 553496261, 583827855, 615821622, 649568646,
    685165008, 722712050, 762316670, 804091623, 848155844, 894634784, 943660770, 995373379,
    1049919840, 1107455447, 1168144006, 1232158297, 1299680571, 1370903066, 1446028554, 1525246918,
    1608855764, 1697021059,
];

fn v83_exp_fraction(level: u8, experience: i32) -> f32 {
    let Some(required) = V83_EXP_TO_NEXT_LEVEL.get(usize::from(level)) else {
        return 0.0;
    };
    if *required <= 0 {
        0.0
    } else {
        (experience.max(0) as f64 / *required as f64).clamp(0.0, 1.0) as f32
    }
}

struct DecodedSprite {
    texture_key: String,
    rect: SpriteRect,
    depth_order: i16,
    texture_width: u32,
    texture_height: u32,
    pixels: Arc<[u8]>,
}

fn append_status_bar_bitmap(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    path: &str,
    x: f32,
    y: f32,
    draw_width: f32,
    draw_height: f32,
    crop: Option<(u32, u32, u32, u32)>,
) -> Result<usize, String> {
    let node = archive
        .root()
        .resolve(path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("missing native status-bar asset {path:?}"))?;
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("native status-bar asset {path:?} is not a bitmap"))?;
    let source_width = u32::from(bitmap.width());
    let source_height = u32::from(bitmap.height());
    let source = bitmap
        .decode_bgra_shared()
        .map_err(|error| error.to_string())?;
    let (texture_width, texture_height, pixels, crop_key) = if let Some((
        crop_x,
        crop_y,
        crop_width,
        crop_height,
    )) = crop
    {
        if crop_width == 0
            || crop_height == 0
            || crop_x.saturating_add(crop_width) > source_width
            || crop_y.saturating_add(crop_height) > source_height
        {
            return Err(format!(
                "invalid crop {crop_x},{crop_y} {crop_width}x{crop_height} for {path:?} ({source_width}x{source_height})"
            ));
        }
        let mut pixels = vec![0_u8; crop_width as usize * crop_height as usize * 4];
        for row in 0..crop_height as usize {
            let source_start =
                ((crop_y as usize + row) * source_width as usize + crop_x as usize) * 4;
            let destination_start = row * crop_width as usize * 4;
            let row_bytes = crop_width as usize * 4;
            pixels[destination_start..destination_start + row_bytes]
                .copy_from_slice(&source[source_start..source_start + row_bytes]);
        }
        (
            crop_width,
            crop_height,
            Arc::<[u8]>::from(pixels),
            format!("#{crop_x},{crop_y},{crop_width},{crop_height}"),
        )
    } else {
        (source_width, source_height, source, String::new())
    };
    let origin = node
        .child("origin")
        .map_err(|error| error.to_string())?
        .map(|origin| origin.value())
        .transpose()
        .map_err(|error| error.to_string())?;
    let (origin_x, origin_y) = match origin {
        Some(NodeValue::Vector { x, y }) => (x as f32, y as f32),
        Some(_) => return Err(format!("malformed native status-bar origin in {path:?}")),
        None => (0.0, 0.0),
    };
    let index = sprites.len();
    sprites.push(DecodedSprite {
        texture_key: format!("native-status-bar:{path}{crop_key}"),
        rect: SpriteRect {
            x: x - origin_x,
            y: y - origin_y,
            width: draw_width,
            height: draw_height,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width,
        texture_height,
        pixels,
    });
    Ok(index)
}

fn append_status_bar_nx_sprite(
    sprites: &mut Vec<DecodedSprite>,
    archive: &NxArchive,
    path: &str,
    anchor_x: f32,
    anchor_y: f32,
) -> Result<usize, String> {
    let node = archive
        .root()
        .resolve(path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("missing native status-bar asset {path:?}"))?;
    let index = sprites.len();
    append_node_sprite(sprites, &node, path, anchor_x, anchor_y, 1.0)?;
    Ok(index)
}

fn build_status_bar_sprites(
    archive: &NxArchive,
) -> Result<
    (
        Vec<DecodedSprite>,
        [StatusBarGaugeAsset; 3],
        StatusBarLayout,
    ),
    String,
> {
    if archive
        .root()
        .resolve("StatusBar2.img/mainBar")
        .map_err(|error| error.to_string())?
        .is_some()
    {
        return build_modern_status_bar_sprites(archive);
    }
    build_v83_status_bar_sprites(archive)
}

fn build_modern_status_bar_sprites(
    archive: &NxArchive,
) -> Result<
    (
        Vec<DecodedSprite>,
        [StatusBarGaugeAsset; 3],
        StatusBarLayout,
    ),
    String,
> {
    let mut sprites = Vec::with_capacity(24);
    append_status_bar_bitmap(
        &mut sprites,
        archive,
        "StatusBar2.img/mainBar/backgrnd",
        512.0,
        768.0,
        2048.0,
        85.0,
        None,
    )?;
    append_status_bar_nx_sprite(
        &mut sprites,
        archive,
        "StatusBar2.img/mainBar/gaugeBackgrd",
        512.0,
        768.0,
    )?;
    append_status_bar_nx_sprite(
        &mut sprites,
        archive,
        "StatusBar2.img/mainBar/lvBacktrnd",
        512.0,
        768.0,
    )?;

    let mut gauges = [StatusBarGaugeAsset {
        fill_index: 0,
        start_index: None,
        end_index: None,
    }; 3];
    for (index, (kind, x, y, width)) in [
        ("hp", 251.0, 737.0, 137.0),
        ("mp", 422.0, 737.0, 137.0),
        ("exp", 251.0, 753.0, 308.0),
    ]
    .into_iter()
    .enumerate()
    {
        let base = format!("StatusBar2.img/mainBar/gauge/{kind}");
        gauges[index] = StatusBarGaugeAsset {
            fill_index: append_status_bar_nx_sprite(
                &mut sprites,
                archive,
                &format!("{base}/0"),
                x,
                y,
            )?,
            start_index: Some(append_status_bar_nx_sprite(
                &mut sprites,
                archive,
                &format!("{base}/1"),
                x,
                y,
            )?),
            end_index: Some(append_status_bar_nx_sprite(
                &mut sprites,
                archive,
                &format!("{base}/2"),
                x + width,
                y,
            )?),
        };
        sprites[gauges[index].fill_index].rect.width = width;
    }

    append_status_bar_nx_sprite(
        &mut sprites,
        archive,
        "StatusBar2.img/mainBar/lvCover",
        512.0,
        768.0,
    )?;
    append_status_bar_nx_sprite(
        &mut sprites,
        archive,
        "StatusBar2.img/mainBar/gaugeCover",
        512.0,
        768.0,
    )?;

    for (button, offset_x, offset_y) in [
        ("BtChat", 0.0, 0.0),
        ("BtClaim", 0.0, 0.0),
        ("BtCashShop", 0.0, 0.0),
        ("BtMTS", 17.0, 0.0),
        ("BtMenu", 53.0, 0.0),
        ("BtSystem", 53.0, 0.0),
        ("BtStat", 0.0, 0.0),
        ("BtEquip", 0.0, 0.0),
        ("BtInven", 0.0, 0.0),
        ("BtSkill", 0.0, 0.0),
        ("BtKeysetting", 0.0, 0.0),
        ("BtNotice", 0.0, 0.0),
        ("BtChannel", 53.0, 0.0),
    ] {
        let path = format!("StatusBar2.img/mainBar/{button}/normal/0");
        let node = archive
            .root()
            .resolve(&path)
            .map_err(|error| error.to_string())?;
        if let Some(node) = node {
            append_node_sprite(
                &mut sprites,
                &node,
                &path,
                512.0 + offset_x,
                768.0 + offset_y,
                1.0,
            )?;
        }
    }
    Ok((sprites, gauges, StatusBarLayout::Modern))
}

fn build_v83_status_bar_sprites(
    archive: &NxArchive,
) -> Result<
    (
        Vec<DecodedSprite>,
        [StatusBarGaugeAsset; 3],
        StatusBarLayout,
    ),
    String,
> {
    const BAR_TOP: f32 = 697.0;
    let mut sprites = Vec::with_capacity(24);
    // The full v83 strip carries the right-side controls; backgrnd2 is the
    // compact status panel shown in the reference and supplies its NX frame.
    append_status_bar_bitmap(
        &mut sprites,
        archive,
        "StatusBar.img/base/backgrnd",
        0.0,
        BAR_TOP,
        1024.0,
        71.0,
        None,
    )?;
    append_status_bar_bitmap(
        &mut sprites,
        archive,
        "StatusBar.img/base/backgrnd2",
        0.0,
        BAR_TOP - 12.0,
        570.0,
        71.0,
        None,
    )?;
    // The compact panel includes LV/HP/MP/EXP legends, so omit the caption row
    // from gauge/bar to keep those labels from being drawn a second time.
    // Keep the first three non-gauge entries as the base, compact panel, and
    // gauge track before live gauge fills.
    let gauge_x = 219.0;
    append_status_bar_bitmap(
        &mut sprites,
        archive,
        "StatusBar.img/gauge/graduation",
        gauge_x,
        BAR_TOP + 32.0,
        340.0,
        31.0,
        None,
    )?;
    let hp_gauge = append_status_bar_bitmap(
        &mut sprites,
        archive,
        "StatusBar.img/gauge/bar",
        gauge_x + 2.0,
        BAR_TOP + 46.0,
        105.0,
        16.0,
        Some((2, 15, 105, 16)),
    )?;
    let mp_gauge = append_status_bar_bitmap(
        &mut sprites,
        archive,
        "StatusBar.img/gauge/bar",
        gauge_x + 110.0,
        BAR_TOP + 46.0,
        105.0,
        16.0,
        Some((110, 15, 105, 16)),
    )?;
    let exp_gauge = append_status_bar_bitmap(
        &mut sprites,
        archive,
        "StatusBar.img/gauge/bar",
        gauge_x + 223.0,
        BAR_TOP + 46.0,
        115.0,
        16.0,
        Some((223, 15, 115, 16)),
    )?;
    let mut add = |path: &str,
                   x: f32,
                   y: f32,
                   width: f32,
                   height: f32,
                   crop: Option<(u32, u32, u32, u32)>| {
        append_status_bar_bitmap(&mut sprites, archive, path, x, y, width, height, crop)
    };

    let small_x = 1024.0 - 8.0 - (6.0 * 28.0 + 5.0 * 2.0);
    let small_y = BAR_TOP + 5.0;
    let small_buttons = [
        "StatKey",
        "InvenKey",
        "EquipKey",
        "SkillKey",
        "KeySet",
        "QuickSlot",
    ];
    for (index, button) in small_buttons.into_iter().enumerate() {
        add(
            &format!("StatusBar.img/{button}/normal/0"),
            small_x + index as f32 * 30.0,
            small_y,
            28.0,
            20.0,
            None,
        )?;
    }

    let claim_x = small_x - 28.0;
    let whisper_x = claim_x - 15.0;
    let notice_x = whisper_x - 48.0;
    add(
        "StatusBar.img/BtClaim/normal/0",
        claim_x,
        small_y,
        20.0,
        19.0,
        None,
    )?;
    add(
        "StatusBar.img/BtWhisper/normal/0",
        whisper_x,
        small_y,
        12.0,
        19.0,
        None,
    )?;
    add(
        "StatusBar.img/base/box",
        notice_x,
        small_y,
        42.0,
        19.0,
        None,
    )?;
    add(
        "StatusBar.img/base/iconMemo",
        notice_x + 4.0,
        small_y + 4.0,
        14.0,
        11.0,
        None,
    )?;

    let big_x = 1024.0 - 8.0 - (4.0 * 54.0 + 3.0 * 2.0);
    let big_y = BAR_TOP + 35.0;
    for (index, button) in ["BtMenu", "BtShop", "BtShort", "BtNPT"]
        .into_iter()
        .enumerate()
    {
        add(
            &format!("StatusBar.img/{button}/normal/0"),
            big_x + index as f32 * 56.0,
            big_y,
            54.0,
            34.0,
            None,
        )?;
    }

    Ok((
        sprites,
        [
            StatusBarGaugeAsset {
                fill_index: hp_gauge,
                start_index: None,
                end_index: None,
            },
            StatusBarGaugeAsset {
                fill_index: mp_gauge,
                start_index: None,
                end_index: None,
            },
            StatusBarGaugeAsset {
                fill_index: exp_gauge,
                start_index: None,
                end_index: None,
            },
        ],
        StatusBarLayout::V83,
    ))
}

struct LoginSceneAssets {
    sprites: Vec<DecodedSprite>,
    actions: Vec<LoginActionRegion>,
    map_scene: Option<MapSceneData>,
    animated_art: Vec<MapArtAnimationAsset>,
    login_buttons: Vec<LoginButtonAsset>,
    save_id_indicator_draws: Option<(usize, SpriteRect, usize, SpriteRect)>,
}

fn upload_scene_sprites(
    renderer: &GpuRenderer,
    sprites: Vec<DecodedSprite>,
) -> Result<
    (
        Vec<GpuTexture>,
        Vec<(usize, SpriteRect)>,
        HashMap<String, usize>,
    ),
    String,
> {
    let mut unique_sprites = Vec::new();
    let mut texture_indices = HashMap::new();
    for sprite in &sprites {
        if !texture_indices.contains_key(&sprite.texture_key) {
            let index = unique_sprites.len();
            unique_sprites.push(sprite);
            texture_indices.insert(sprite.texture_key.clone(), index);
        }
    }
    let atlas_images = unique_sprites
        .iter()
        .map(|sprite| BgraImage {
            width: sprite.texture_width,
            height: sprite.texture_height,
            pixels: sprite.pixels.as_ref(),
        })
        .collect::<Vec<_>>();
    let textures = renderer
        .upload_bgra_atlas(&atlas_images)
        .map_err(|error| error.to_string())?;
    let draws = sprites
        .into_iter()
        .map(|sprite| {
            let texture_index = texture_indices
                .get(&sprite.texture_key)
                .copied()
                .expect("scene texture was indexed before upload");
            (texture_index, sprite.rect)
        })
        .collect();
    Ok((textures, draws, texture_indices))
}
