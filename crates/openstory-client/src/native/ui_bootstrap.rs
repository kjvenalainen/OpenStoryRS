fn configured_login_packet() -> Result<Vec<u8>, String> {
    let account = std::env::var("OPENSTORY_ACCOUNT").ok();
    let password = std::env::var("OPENSTORY_PASSWORD").ok();
    let (account, password) = match (account, password) {
        (None, None) => {
            return Err(
                "set OPENSTORY_ACCOUNT and OPENSTORY_PASSWORD before logging in".to_owned(),
            );
        }
        (Some(account), Some(password)) => (account, password),
        _ => {
            return Err("set both OPENSTORY_ACCOUNT and OPENSTORY_PASSWORD to log in".to_owned());
        }
    };
    if account.is_empty() || password.is_empty() {
        return Err("OPENSTORY_ACCOUNT and OPENSTORY_PASSWORD must not be empty".to_owned());
    }

    let machine_id = configured_machine_id()?;
    let packet = openstory_protocol::login_request(&account, &password, machine_id)
        .map_err(|error| error.to_string())?;
    Ok(packet)
}

fn configured_machine_id() -> Result<u32, String> {
    std::env::var("OPENSTORY_MACHINE_ID")
        .ok()
        .map(|value| {
            u32::from_str_radix(value.trim_start_matches("0x"), 16)
                .map_err(|error| format!("invalid OPENSTORY_MACHINE_ID: {error}"))
        })
        .transpose()
        .map(|value| value.unwrap_or(0))
}

fn load_system_font() -> Result<fontdue::Font, String> {
    if let Some(path) = std::env::var_os("OPENSTORY_FONT_PATH") {
        let path = PathBuf::from(path);
        let bytes = fs::read(&path)
            .map_err(|error| format!("could not read font {}: {error}", path.display()))?;
        return fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|error| format!("could not load font {}: {error}", path.display()));
    }
    fontdue::Font::from_bytes(
        include_bytes!("../../assets/fonts/Roboto-Regular.ttf").as_slice(),
        fontdue::FontSettings::default(),
    )
    .map_err(|error| format!("could not load the bundled MapleStory Roboto font: {error}"))
}

fn rasterize_field(font: &fontdue::Font, text: &str, draw_caret: bool) -> Vec<u8> {
    const FONT_SIZE: f32 = 13.0;
    const BASELINE: i32 = 21;
    let mut pixels = vec![0; FIELD_WIDTH as usize * FIELD_HEIGHT as usize * 4];
    let mut pen_x = 2.0_f32;
    let mut previous = None;

    for character in text.chars() {
        if let Some(previous) = previous {
            pen_x += font
                .horizontal_kern(previous, character, FONT_SIZE)
                .unwrap_or(0.0);
        }
        let (metrics, coverage) = rasterize_ui_glyph(font, character, FONT_SIZE);
        let left = pen_x.round() as i32 + metrics.xmin;
        let top = BASELINE - metrics.ymin - metrics.height as i32;
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                let x = left + column as i32;
                let y = top + row as i32;
                if x < 0 || y < 0 || x >= FIELD_WIDTH as i32 || y >= FIELD_HEIGHT as i32 {
                    continue;
                }
                let source = row * metrics.width + column;
                let destination = (y as usize * FIELD_WIDTH as usize + x as usize) * 4;
                pixels[destination..destination + 4].copy_from_slice(&[
                    255,
                    255,
                    255,
                    coverage[source],
                ]);
            }
        }
        pen_x += metrics.advance_width;
        previous = Some(character);
    }

    if draw_caret {
        let x = pen_x.round().clamp(1.0, FIELD_WIDTH as f32 - 2.0) as usize;
        for y in 10..24 {
            let offset = (y * FIELD_WIDTH as usize + x) * 4;
            pixels[offset..offset + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
    }
    pixels
}

fn rasterize_pic_dialog(
    font: &fontdue::Font,
    heading: &str,
    guidance: &str,
    masked_input: &str,
    notice: &str,
) -> Vec<u8> {
    let mut pixels = vec![0; PIC_DIALOG_WIDTH as usize * PIC_DIALOG_HEIGHT as usize * 4];
    let width = PIC_DIALOG_WIDTH as usize;
    for y in 82..116 {
        for x in 16..366 {
            let edge = x == 16 || x == 365 || y == 82 || y == 115;
            let color = if edge {
                [144, 128, 96, 255]
            } else {
                [28, 25, 20, 255]
            };
            let offset = (y * width + x) * 4;
            pixels[offset..offset + 4].copy_from_slice(&color);
        }
    }

    fn draw_text(
        font: &fontdue::Font,
        pixels: &mut [u8],
        text: &str,
        x: f32,
        baseline: i32,
        font_size: f32,
    ) {
        let mut pen_x = x;
        let mut previous = None;
        for character in text.chars() {
            if let Some(previous) = previous {
                pen_x += font
                    .horizontal_kern(previous, character, font_size)
                    .unwrap_or(0.0);
            }
            let (metrics, coverage) = rasterize_ui_glyph(font, character, font_size);
            let left = pen_x.round() as i32 + metrics.xmin;
            let top = baseline - metrics.ymin - metrics.height as i32;
            for row in 0..metrics.height {
                for column in 0..metrics.width {
                    let px = left + column as i32;
                    let py = top + row as i32;
                    if px < 0
                        || py < 0
                        || px >= PIC_DIALOG_WIDTH as i32
                        || py >= PIC_DIALOG_HEIGHT as i32
                    {
                        continue;
                    }
                    let source = row * metrics.width + column;
                    let destination = (py as usize * PIC_DIALOG_WIDTH as usize + px as usize) * 4;
                    pixels[destination..destination + 4].copy_from_slice(&[
                        255,
                        255,
                        255,
                        coverage[source],
                    ]);
                }
            }
            pen_x += metrics.advance_width;
            previous = Some(character);
        }
    }

    draw_text(font, &mut pixels, heading, 18.0, 34, 21.0);
    draw_text(font, &mut pixels, guidance, 18.0, 61, 14.0);
    draw_text(font, &mut pixels, masked_input, 27.0, 106, 17.0);
    draw_text(font, &mut pixels, notice, 18.0, 151, 13.0);
    pixels
}

fn render_map_hud(
    font: &fontdue::Font,
    number_font: Option<&StatusBarNumberFont>,
    layout: StatusBarLayout,
    stats: &openstory_protocol::CharacterStats,
    active_buffs: &HashMap<openstory_protocol::BuffStatKey, ActiveCharacterBuff>,
    chat_line: Option<(&str, [u8; 3])>,
) -> Vec<u8> {
    const WIDTH: usize = 1024;
    let height = layout.height() as usize;
    let modern = layout == StatusBarLayout::Modern;
    let mut pixels = vec![0_u8; WIDTH * height * 4];
    // NX artwork supplies the labels and frame; this overlay supplies character
    // data and live stat values while gauge fills are drawn from their own sprites.
    if let Some((text, color)) = chat_line {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            height,
            text,
            if modern { 4 } else { 4 },
            if modern { 22 } else { 10 },
            if modern { 12.0 } else { 10.0 },
            if modern { 480 } else { 500 },
            17,
            color,
        );
    }
    if modern {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            height,
            &stats.name,
            77,
            48,
            13.0,
            160,
            17,
            [255, 255, 255],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            height,
            &character_job_name(stats.job),
            77,
            63,
            11.0,
            160,
            15,
            [255, 220, 44],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            height,
            &stats.level.to_string(),
            31,
            60,
            12.0,
            34,
            15,
            [255, 255, 255],
        );
    } else {
        if let Some(number_font) = number_font {
            draw_status_bar_number_scaled(
                number_font,
                &mut pixels,
                WIDTH,
                height,
                &stats.level.to_string(),
                78,
                39,
                2.2,
                Some([255, 145, 28]),
            );
        } else {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                height,
                &stats.level.to_string(),
                40,
                37,
                14.0,
                55,
                29,
                [255, 178, 42],
            );
        }
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            height,
            &status_bar_job_name(stats.job),
            88,
            32,
            12.0,
            122,
            15,
            [244, 246, 250],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            height,
            &stats.name,
            88,
            49,
            11.0,
            122,
            15,
            [244, 246, 250],
        );
        for (value, value_x, max_width) in [
            (
                format!("[{}/{}]", stats.hp, stats.max_hp),
                244,
                67,
            ),
            (
                format!("[{}/{}]", stats.mp, stats.max_mp),
                352,
                67,
            ),
            (
                format!(
                    "{}[{:.2}%]",
                    stats.experience,
                    v83_exp_fraction(stats.level, stats.experience) * 100.0
                ),
                473,
                84,
            ),
        ] {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                height,
                &value,
                value_x,
                32,
                10.0,
                max_width,
                13,
                [255, 255, 255],
            );
        }
    }
    if modern && let Some(number_font) = number_font {
        draw_status_bar_number(
            number_font,
            &mut pixels,
            WIDTH,
            height,
            &format!("[{}/{}]", stats.hp, stats.max_hp),
            if modern { 388 } else { 260 },
            if modern { 54 } else { 57 },
        );
        draw_status_bar_number(
            number_font,
            &mut pixels,
            WIDTH,
            height,
            &format!("[{}/{}]", stats.mp, stats.max_mp),
            if modern { 559 } else { 368 },
            if modern { 54 } else { 57 },
        );
        draw_status_bar_number(
            number_font,
            &mut pixels,
            WIDTH,
            height,
            &format!(
                "{}[{}%]",
                stats.experience,
                format!(
                    "{:.2}",
                    v83_exp_fraction(stats.level, stats.experience) * 100.0
                )
            ),
            if modern { 559 } else { 491 },
            if modern { 70 } else { 57 },
        );
    }

    let mut status_x = if modern { 576 } else { 790 };
    for (flag, label, background, foreground) in [
        (STATUS_STUN, "STUN", [107, 76, 42, 255], [255, 231, 160]),
        (STATUS_POISON, "POISON", [45, 89, 54, 255], [205, 255, 199]),
        (STATUS_SEAL, "SEAL", [69, 58, 104, 255], [222, 206, 255]),
        (STATUS_SLOW, "SLOW", [70, 70, 76, 255], [220, 220, 229]),
        (
            STATUS_CONFUSE,
            "CONFUSE",
            [93, 52, 54, 255],
            [255, 206, 203],
        ),
        (STATUS_SEDUCE, "SEDUCE", [103, 54, 81, 255], [255, 206, 228]),
        (STATUS_DARKNESS, "DARK", [41, 43, 54, 255], [216, 219, 241]),
        (STATUS_WEAKEN, "WEAKEN", [99, 65, 43, 255], [255, 220, 183]),
        (STATUS_CURSE, "CURSE", [77, 49, 91, 255], [239, 211, 255]),
        (STATUS_ZOMBIFY, "UNDEAD", [69, 81, 48, 255], [228, 239, 185]),
    ] {
        let active = if flag == STATUS_ZOMBIFY {
            character_has_mob_skill_buff(
                active_buffs,
                openstory_protocol::BuffMaskGroup::Second,
                flag,
                MOB_SKILL_TYPE_UNDEAD,
            )
        } else {
            character_has_buff(
                active_buffs,
                openstory_protocol::BuffMaskGroup::Second,
                flag,
            )
        };
        if !active {
            continue;
        }
        if status_x + 62 > if modern { 744 } else { 744 } {
            break;
        }
        fill_dialogue_rect(
            &mut pixels,
            WIDTH,
            height,
            status_x,
            if modern { 3 } else { 5 },
            62,
            19,
            background,
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            height,
            label,
            (status_x + 4) as i32,
            if modern { 6 } else { 8 },
            8.0,
            54,
            12,
            foreground,
        );
        status_x += 66;
    }
    pixels
}

fn status_bar_job_name(job: u16) -> String {
    match job {
        112 => "Hero".to_owned(),
        122 => "Paladin".to_owned(),
        132 => "Dark Knight".to_owned(),
        _ => character_job_name(job),
    }
}

const BUFF_LIST_HEIGHT: u32 = 32;

fn render_buff_list_pixels(
    icons: &[(u32, u32, Arc<[u8]>, Option<Duration>, i32, Duration)],
) -> Vec<u8> {
    const WIDTH: usize = 1024;
    const HEIGHT: usize = BUFF_LIST_HEIGHT as usize;
    const ICON_SIZE: usize = 32;
    const FLASH_WINDOW: Duration = Duration::from_secs(3);
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];

    for (index, (icon_width, icon_height, icon, remaining, duration_ms, elapsed)) in
        icons.iter().take(31).enumerate()
    {
        let x = WIDTH - 35 - ICON_SIZE * index;
        let opacity = if remaining.is_some_and(|remaining| remaining <= FLASH_WINDOW) {
            let flash_start = Duration::from_millis(
                duration_ms.saturating_sub(FLASH_WINDOW.as_millis() as i32).max(0) as u64,
            );
            let flash_elapsed = elapsed.saturating_sub(flash_start);
            let phase_ms = flash_elapsed.as_millis() % 320;
            if phase_ms <= 160 {
                1.0 - phase_ms as f32 / 160.0
            } else {
                (phase_ms - 160) as f32 / 160.0
            }
        } else {
            1.0
        };
        for y in 0..ICON_SIZE {
            let source_y = y * *icon_height as usize / ICON_SIZE;
            for icon_x in 0..ICON_SIZE {
                let source_x = icon_x * *icon_width as usize / ICON_SIZE;
                let source = (source_y * *icon_width as usize + source_x) * 4;
                let destination = (y * WIDTH + x + icon_x) * 4;
                pixels[destination..destination + 3].copy_from_slice(&icon[source..source + 3]);
                pixels[destination + 3] =
                    (f32::from(icon[source + 3]) * opacity).round() as u8;
            }
        }

        let cover = if let Some(remaining) = remaining {
            if *duration_ms <= FLASH_WINDOW.as_millis() as i32
                || *remaining <= FLASH_WINDOW
            {
                1.0
            } else {
                let cover_duration_ms = (*duration_ms - FLASH_WINDOW.as_millis() as i32).max(1);
                (elapsed.as_millis() as f32 / cover_duration_ms as f32).clamp(0.0, 1.0)
            }
        } else {
            0.0
        };
        let cover_height = (30.0 * cover).round() as usize;
        let cover_alpha = (153.0 * cover).round() as u16;
        for y in (30 - cover_height)..30 {
            for local_x in 1..31 {
                let offset = (y * WIDTH + x + local_x) * 4;
                let inverse = 255 - cover_alpha;
                let source_alpha = u32::from(pixels[offset + 3]);
                let output_alpha = cover_alpha as u32
                    + (source_alpha * u32::from(inverse) + 127) / 255;
                for channel in 0..3 {
                    let numerator = u32::from(pixels[offset + channel])
                        * source_alpha
                        * u32::from(inverse);
                    pixels[offset + channel] = if output_alpha == 0 {
                        0
                    } else {
                        ((numerator + output_alpha * 127) / (output_alpha * 255)) as u8
                    };
                }
                pixels[offset + 3] = output_alpha.min(255) as u8;
            }
        }
    }
    pixels
}

fn render_status_menu(font: &fontdue::Font) -> Vec<u8> {
    let width = STATUS_MENU_WIDTH as usize;
    let height = STATUS_MENU_HEIGHT as usize;
    let mut pixels = vec![0_u8; width * height * 4];
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        height,
        [18, 17, 15, 224],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        1,
        [181, 137, 56, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        height - 1,
        width,
        1,
        [181, 137, 56, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        1,
        height,
        [181, 137, 56, 255],
    );
    fill_dialogue_rect(
        &mut pixels,
        width,
        height,
        width - 1,
        0,
        1,
        height,
        [181, 137, 56, 255],
    );
    for (index, label) in STATUS_MENU_ROWS.iter().enumerate() {
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            label,
            7,
            5 + index as i32 * STATUS_MENU_ROW_HEIGHT as i32,
            11.0,
            width - 14,
            STATUS_MENU_ROW_HEIGHT,
            [245, 237, 213],
        );
    }
    pixels
}

fn is_known_attack_skill(skill_id: i32) -> bool {
    matches!(
        skill_id,
        1_000
            | 1_001_004
            | 1_001_005
            | 1_111_003
            | 1_111_004
            | 1_111_005
            | 1_111_008
            | 1_121_006
            | 1_121_008
            | 1_201_006
            | 1_211_002
            | 1_221_007
            | 1_221_009
            | 1_221_011
            | 1_311_001
            | 1_311_002
            | 1_311_003
            | 1_311_004
            | 1_311_005
            | 1_311_006
            | 1_321_003
            | 2_001_004
            | 2_001_005
            | 2_101_003
            | 2_101_004
            | 2_101_005
            | 2_111_002
            | 2_111_003
            | 2_111_004
            | 2_111_006
            | 2_121_003
            | 2_121_006
            | 2_121_007
            | 4_001_334
            | 4_001_344
            | 4_201_005
            | 4_211_002
            | 4_211_004
            | 4_111_005
            | 4_121_007
            | 4_121_008
            | 4_211_006
            | 4_221_001
            | 4_221_007
    )
}

fn skill_subjob_for_tab(job: u16, tab: u8) -> Option<u16> {
    let job_level = if matches!(job, 0 | 1000 | 2000 | 900) {
        0
    } else if job == 910 || job % 100 == 0 {
        1
    } else if job % 10 == 0 {
        2
    } else if job % 10 == 1 {
        3
    } else {
        4
    };
    if tab > job_level {
        return None;
    }
    let beginner = if (2000..=2112).contains(&job) {
        2000
    } else if (1000..=1512).contains(&job) {
        1000
    } else if matches!(job, 900 | 910) {
        900
    } else {
        0
    };
    Some(match tab {
        0 => beginner,
        1 if job == 910 => 910,
        1 => (job / 100) * 100,
        2 => (job / 10) * 10,
        3 if job_level == 4 => job - 1,
        3 => job,
        4 => job,
        _ => return None,
    })
}
