fn append_mob_damage_draws<'a>(
    scene: &MapSceneRuntime,
    texture: &'a GpuTexture,
    glyphs: &[DamageGlyph],
    effect_budget: &mut usize,
    draws: &mut Vec<SpriteDraw<'a>>,
) {
    let draw_count = scene.damage_numbers.len().min(*effect_budget);
    *effect_budget -= draw_count;
    for number in scene.damage_numbers.iter().take(draw_count) {
        let age = number.elapsed.as_secs_f32();
        let lifetime = number.lifetime.as_secs_f32().max(f32::EPSILON);
        let opacity = (1.0 - age / lifetime).clamp(0.0, 1.0);
        let rise = age * number.rise_speed;
        let anchor_x = scene.camera_offset.0 + number.world_x as f32;
        let anchor_y = scene.camera_offset.1 + number.world_y as f32 - rise;

        if number.miss {
            if number.critical {
                continue;
            }
            let Some(glyph) = glyphs.get(80) else {
                continue;
            };
            draws.push(SpriteDraw {
                texture,
                x: anchor_x - glyph.origin_x,
                y: anchor_y - glyph.width / 2.0 - glyph.origin_y,
                width: glyph.width,
                height: glyph.height,
                uv: glyph.uv,
                tint: [1.0, 1.0, 1.0],
                blend_mode: BlendMode::Alpha,
                flip_x: false,
                opacity,
            });
            continue;
        }
        if number.digits.is_empty() {
            continue;
        }
        let leading_digit = usize::from(number.digits[0]);
        let leading_glyph = damage_glyph_index(
            leading_digit,
            number.critical,
            true,
            number.heal,
            number.player_damage,
        );
        if glyphs.get(leading_glyph).is_none() {
            continue;
        }
        let total_advance = (0..number.digits.len())
            .map(|index| damage_digit_advance(&number.digits, index, number.critical))
            .sum::<i16>();
        let anchor_y = anchor_y - f32::from(total_advance / 2);
        let mut cursor_x = 0.0;
        for (index, digit) in number.digits.iter().copied().enumerate() {
            let glyph_index = damage_glyph_index(
                usize::from(digit),
                number.critical,
                index == 0,
                number.heal,
                number.player_damage,
            );
            let Some(glyph) = glyphs.get(glyph_index) else {
                cursor_x += f32::from(damage_digit_advance(&number.digits, index, number.critical));
                continue;
            };
            let vertical_shift = if index > 0 && index % 2 == 1 {
                2.0
            } else if index > 0 {
                -2.0
            } else {
                0.0
            };
            draws.push(SpriteDraw {
                texture,
                x: anchor_x + cursor_x - glyph.origin_x,
                y: anchor_y + vertical_shift - glyph.origin_y,
                width: glyph.width,
                height: glyph.height,
                uv: glyph.uv,
                tint: [1.0, 1.0, 1.0],
                blend_mode: BlendMode::Alpha,
                flip_x: false,
                opacity,
            });
            cursor_x += f32::from(damage_digit_advance(&number.digits, index, number.critical));
        }
    }
}

fn damage_glyph_index(
    digit: usize,
    critical: bool,
    first: bool,
    heal: bool,
    player_damage: bool,
) -> usize {
    if heal {
        return 40 + usize::from(!first) * 10 + digit;
    }
    if player_damage {
        return 60 + usize::from(!first) * 10 + digit;
    }
    match (critical, first) {
        (false, true) => digit,
        (false, false) => 10 + digit,
        (true, true) => 20 + digit,
        (true, false) => 30 + digit,
    }
}

fn damage_digit_advance(digits: &[u8], index: usize, critical: bool) -> i16 {
    const DIGIT_ADVANCE: [i16; 10] = [24, 20, 22, 22, 24, 23, 24, 22, 24, 24];
    if index == 0 {
        return DIGIT_ADVANCE[usize::from(digits[0])] + if critical { 8 } else { 2 };
    }
    let current = DIGIT_ADVANCE[usize::from(digits[index])] + if critical { 4 } else { 0 };
    if let Some(next) = digits.get(index + 1) {
        let next = DIGIT_ADVANCE[usize::from(*next)] + if critical { 4 } else { 0 };
        (current + next) / 2
    } else {
        current
    }
}

fn append_map_screen_effect_draws<'a>(
    scene: &MapSceneRuntime,
    textures: &'a [GpuTexture],
    effect_budget: &mut usize,
    draws: &mut Vec<SpriteDraw<'a>>,
) {
    if *effect_budget == 0 {
        return;
    }
    let Some(effect) = scene.screen_effect.as_ref() else {
        return;
    };
    let Some(frame) = effect.frames.get(effect.frame_index) else {
        return;
    };
    *effect_budget -= 1;
    for (texture_index, rect) in &frame.sprites {
        let Some(texture) = textures.get(*texture_index) else {
            continue;
        };
        draws.push(SpriteDraw {
            texture,
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
            uv: [0.0, 0.0, 1.0, 1.0],
            tint: [1.0, 1.0, 1.0],
            blend_mode: rect.blend_mode,
            flip_x: false,
            opacity: 1.0,
        });
    }
}

fn append_mob_hp_bar<'a>(
    draws: &mut Vec<SpriteDraw<'a>>,
    textures: &'a [GpuTexture],
    x: f32,
    y: f32,
    percent: u8,
) {
    let mut push = |texture_index: usize, x, y, width, height| {
        if let Some(texture) = textures.get(texture_index) {
            draws.push(SpriteDraw {
                texture,
                x,
                y,
                width,
                height,
                uv: [0.0, 0.0, 1.0, 1.0],
                tint: [1.0, 1.0, 1.0],
                blend_mode: BlendMode::Alpha,
                flip_x: false,
                opacity: 1.0,
            });
        }
    };
    push(0, x, y, 50.0, 10.0);
    push(1, x + 1.0, y + 1.0, 48.0, 1.0);
    push(1, x + 1.0, y + 8.0, 48.0, 1.0);
    push(1, x + 1.0, y + 2.0, 1.0, 6.0);
    push(1, x + 48.0, y + 2.0, 1.0, 6.0);
    let fill_width = 44.0 * f32::from(percent.min(100)) / 100.0;
    if fill_width > 0.0 {
        push(2, x + 3.0, y + 3.0, fill_width, 3.0);
        push(3, x + 3.0, y + 6.0, fill_width, 1.0);
    }
}

#[derive(Clone, Copy)]
struct LoginActionRegion {
    rect: SpriteRect,
    action: LoginAction,
}

#[derive(Clone, Copy)]
struct LoginButtonAsset {
    hit_rect: SpriteRect,
    action: Option<LoginAction>,
    normal_draw: usize,
    mouse_over_draw: Option<usize>,
    pressed_draw: Option<usize>,
    disabled_draw: Option<usize>,
    key_focused_draw: Option<usize>,
}
