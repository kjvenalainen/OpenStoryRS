fn append_mob_hit_effect_draws<'a>(
    mob: &MapMobRuntime,
    scene: &MapSceneRuntime,
    textures: &'a [GpuTexture],
    above_mob: bool,
    draws: &mut Vec<SpriteDraw<'a>>,
) {
    let offset_x = (mob.x - scene.player.x) as f32;
    let offset_y = (mob.y - scene.player.y) as f32;
    for effect in mob
        .hit_effects
        .iter()
        .filter(|effect| (effect.z >= 0) == above_mob)
    {
        let Some(frame) = effect.frames.get(effect.frame_index) else {
            continue;
        };
        for (texture_index, rect) in &frame.sprites {
            let Some(texture) = textures.get(*texture_index) else {
                continue;
            };
            draws.push(SpriteDraw {
                texture,
                x: if effect.flip_x {
                    1024.0 - (rect.x + rect.width) + offset_x - effect.anchor_x
                } else {
                    rect.x + offset_x + effect.anchor_x
                },
                y: rect.y + offset_y + effect.anchor_y,
                width: rect.width,
                height: rect.height,
                uv: [0.0, 0.0, 1.0, 1.0],
                tint: [1.0, 1.0, 1.0],
                blend_mode: rect.blend_mode,
                flip_x: effect.flip_x,
                opacity: 1.0,
            });
        }
    }
}

fn append_face_expression_draws<'a>(
    expression: &FaceExpressionRuntime,
    face_offset: (f32, f32),
    offset_x: f32,
    offset_y: f32,
    facing_right: bool,
    tint: [f32; 3],
    opacity: f32,
    textures: &'a [GpuTexture],
    draws: &mut Vec<SpriteDraw<'a>>,
) {
    let Some(frame) = expression.frames.get(expression.frame_index) else {
        return;
    };
    for (texture_index, rect) in &frame.sprites {
        let Some(texture) = textures.get(*texture_index) else {
            continue;
        };
        draws.push(SpriteDraw {
            texture,
            x: if facing_right {
                1024.0 - (rect.x + rect.width) + offset_x - face_offset.0
            } else {
                rect.x + offset_x + face_offset.0
            },
            y: rect.y + offset_y + face_offset.1,
            width: rect.width,
            height: rect.height,
            uv: [0.0, 0.0, 1.0, 1.0],
            tint,
            blend_mode: rect.blend_mode,
            flip_x: facing_right,
            opacity,
        });
    }
}

fn character_damage_tint(remaining: Duration, hidden: bool) -> ([f32; 3], f32) {
    if !remaining.is_zero() {
        let elapsed_fraction = (1.0 - remaining.as_secs_f32() / 2.0).clamp(0.0, 1.0);
        let rgb = 0.9 - 0.5 * (elapsed_fraction * 30.0).sin().abs();
        ([rgb, rgb, rgb], 1.0)
    } else if hidden {
        ([1.0, 1.0, 1.0], 0.5)
    } else {
        ([1.0, 1.0, 1.0], 1.0)
    }
}

fn append_chair_animation_draws<'a>(
    animation: &ChairAnimationRuntime,
    facing_right: bool,
    tint: [f32; 3],
    opacity: f32,
    textures: &'a [GpuTexture],
    draws: &mut Vec<SpriteDraw<'a>>,
) {
    let Some(frame) = animation.frames.get(animation.frame_index) else {
        return;
    };
    for (texture_index, rect) in &frame.sprites {
        let Some(texture) = textures.get(*texture_index) else {
            continue;
        };
        draws.push(SpriteDraw {
            texture,
            x: if facing_right {
                1024.0 - (rect.x + rect.width)
            } else {
                rect.x
            },
            y: rect.y,
            width: rect.width,
            height: rect.height,
            uv: [0.0, 0.0, 1.0, 1.0],
            tint,
            blend_mode: rect.blend_mode,
            flip_x: facing_right,
            opacity,
        });
    }
}

#[derive(Clone, Copy)]
enum MapEntityDrawPass {
    Layer(u8),
    Overlays,
}

fn map_entity_foothold_layer(scene: &MapSceneRuntime, foothold_id: u16, x: f64, y: f64) -> u8 {
    scene
        .footholds
        .as_ref()
        .and_then(|footholds| {
            footholds
                .get(foothold_id)
                .or_else(|| {
                    footholds
                        .find_below(x, y)
                        .and_then(|(id, _)| footholds.get(id))
                })
                .map(|foothold| foothold.layer)
        })
        .unwrap_or(0)
}

fn append_item_aura_draws<'a>(
    aura: Option<&PersistentItemAuraRuntime>,
    below: bool,
    offset_x: f32,
    offset_y: f32,
    textures: &'a [GpuTexture],
    draws: &mut Vec<SpriteDraw<'a>>,
) {
    let frame = aura.and_then(|aura| if below { aura.below() } else { aura.above() });
    let Some(frame) = frame else {
        return;
    };
    for (texture_index, rect) in &frame.sprites {
        let Some(texture) = textures.get(*texture_index) else {
            continue;
        };
        draws.push(SpriteDraw {
            texture,
            x: rect.x + offset_x,
            y: rect.y + offset_y,
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
