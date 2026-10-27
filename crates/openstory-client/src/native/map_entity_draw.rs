fn append_death_art_draws<'a>(
    art: &LocalDeathArtRuntime,
    offset_x: f32,
    offset_y: f32,
    facing_left: bool,
    textures: &'a [GpuTexture],
    draws: &mut Vec<SpriteDraw<'a>>,
) {
    let tomb_sprites = if art.tomb_landed {
        &art.landed_sprites
    } else {
        art.fall_frames
            .get(art.tomb_frame_index)
            .map_or(&art.landed_sprites, |frame| &frame.sprites)
    };
    for (texture_index, rect) in tomb_sprites {
        let Some(texture) = textures.get(*texture_index) else {
            continue;
        };
        draws.push(SpriteDraw {
            texture,
            x: rect.x + offset_x,
            y: rect.y
                + offset_y
                + if art.tomb_landed {
                    0.0
                } else {
                    art.tomb_y_offset
                },
            width: rect.width,
            height: rect.height,
            uv: [0.0, 0.0, 1.0, 1.0],
            tint: [1.0, 1.0, 1.0],
            blend_mode: rect.blend_mode,
            flip_x: false,
            opacity: 1.0,
        });
    }
    let ghost_bob = (art.ghost_ticks as f32 / 45.0).sin() * 5.0;
    if let Some(frame) = art.ghost_frames.get(art.ghost_frame_index) {
        for (texture_index, rect) in &frame.sprites {
            let Some(texture) = textures.get(*texture_index) else {
                continue;
            };
            draws.push(SpriteDraw {
                texture,
                x: if facing_left {
                    1024.0 - (rect.x + rect.width) + offset_x
                } else {
                    rect.x + offset_x
                },
                y: rect.y + offset_y + ghost_bob,
                width: rect.width,
                height: rect.height,
                uv: [0.0, 0.0, 1.0, 1.0],
                tint: [1.0, 1.0, 1.0],
                blend_mode: rect.blend_mode,
                flip_x: facing_left,
                opacity: 1.0,
            });
        }
    }
}

fn append_chalkboard_draw<'a>(
    texture: &'a GpuTexture,
    offset_x: f32,
    offset_y: f32,
    draws: &mut Vec<SpriteDraw<'a>>,
) {
    draws.push(SpriteDraw {
        texture,
        x: 512.0 + offset_x - CHALKBOARD_WIDTH as f32 / 2.0,
        y: GAMEPLAY_CAMERA_CENTER_Y + offset_y - 136.0,
        width: CHALKBOARD_WIDTH as f32,
        height: CHALKBOARD_HEIGHT as f32,
        uv: [0.0, 0.0, 1.0, 1.0],
        tint: [1.0, 1.0, 1.0],
        blend_mode: BlendMode::Alpha,
        flip_x: false,
        opacity: 1.0,
    });
}

fn append_map_entity_draws<'a>(
    scene: &MapSceneRuntime,
    active_buffs: &HashMap<openstory_protocol::BuffStatKey, ActiveCharacterBuff>,
    textures: &'a [GpuTexture],
    solid_textures: &'a [GpuTexture],
    maple_tv_idle_texture: Option<&'a GpuTexture>,
    maple_tv_ad_texture: Option<&'a GpuTexture>,
    maple_tv_theme_frames: [Option<&'a MapleTvAnimationFrame>; 2],
    maple_tv_text_texture: Option<&'a GpuTexture>,
    maple_tv_broadcast: Option<&MapleTvBroadcastRuntime>,
    damage_texture: Option<&'a GpuTexture>,
    speech_textures: &'a HashMap<i32, GpuTexture>,
    chalkboard_textures: &'a HashMap<i32, GpuTexture>,
    pet_speech_textures: &'a HashMap<(i32, u8), GpuTexture>,
    local_speech_texture: Option<&'a GpuTexture>,
    local_chalkboard_texture: Option<&'a GpuTexture>,
    damage_glyphs: &[DamageGlyph],
    graphics_quality: u8,
    effect_budget: &mut usize,
    pass: MapEntityDrawPass,
    draws: &mut Vec<SpriteDraw<'a>>,
) {
    let actor_draw_start = draws.len();
    enum Actor<'a> {
        Drop(&'a MapDropRuntime),
        Door(&'a MapDoorRuntime),
        Reactor(&'a MapReactorRuntime),
        Summon(&'a MapSummonRuntime),
        Dragon(&'a MapDragonRuntime),
        Pet(&'a MapPetRuntime),
        Mob(&'a MapMobRuntime),
        Mist(&'a openstory_protocol::MapMistSpawn),
        Npc(&'a MapNpcRuntime),
        HiredMerchant(&'a MapHiredMerchantRuntime),
        CharacterSkillEffect(&'a MapCharacterSkillEffect),
        RemotePlayer(&'a RemotePlayerState),
        RemoteDeath(&'a RemotePlayerState, &'a LocalDeathArtRuntime),
        LocalPlayer(&'a AvatarFrameRuntime),
        LocalDeath(&'a LocalDeathArtRuntime),
        Projectile(&'a MapProjectileRuntime),
    }

    let mut actors = Vec::with_capacity(
        scene.drops.len()
            + scene.doors.len()
            + scene.reactors.len()
            + scene.summons.len()
            + scene.dragons.len()
            + scene.pets.len()
            + scene.mobs.len()
            + scene.npcs.len()
            + scene.hired_merchants.len()
            + scene.character_effects.len()
            + scene.remote_players.len()
            + scene.projectiles.len()
            + 1,
    );
    let local_player_layer = map_entity_foothold_layer(
        scene,
        scene.player.foothold_id,
        scene.player.x,
        scene.player.y,
    );
    for (&object_id, drop) in &scene.drops {
        let layer = map_entity_foothold_layer(scene, drop.body.foothold_id, drop.x, drop.y);
        if matches!(pass, MapEntityDrawPass::Layer(pass_layer) if pass_layer == layer)
            && drop.frames.get(drop.frame_index).is_some()
        {
            actors.push((drop.y, 0, object_id, Actor::Drop(drop)));
        }
    }
    for (&object_id, reactor) in &scene.reactors {
        if matches!(pass, MapEntityDrawPass::Layer(layer) if layer == reactor.foothold_layer) {
            actors.push((reactor.y, 1, object_id, Actor::Reactor(reactor)));
        }
    }
    if matches!(pass, MapEntityDrawPass::Layer(layer) if layer == 7) {
        for (&owner_id, door) in &scene.doors {
            if scene.door_frames.get(door.frame_index).is_some() {
                actors.push((door.y, 5, owner_id, Actor::Door(door)));
            }
        }
    }
    for (&object_id, summon) in &scene.summons {
        let layer = map_entity_foothold_layer(scene, summon.body.foothold_id, summon.x, summon.y);
        if matches!(pass, MapEntityDrawPass::Layer(pass_layer) if pass_layer == layer)
            && summon.frames.get(summon.frame_index).is_some()
        {
            actors.push((summon.y, 2, object_id, Actor::Summon(summon)));
        }
    }
    for (&owner_id, dragon) in &scene.dragons {
        let layer = map_entity_foothold_layer(scene, dragon.body.foothold_id, dragon.x, dragon.y);
        if matches!(pass, MapEntityDrawPass::Layer(pass_layer) if pass_layer == layer)
            && dragon.frames.get(dragon.frame_index).is_some()
        {
            actors.push((dragon.y, 2, owner_id, Actor::Dragon(dragon)));
        }
    }
    for (&(owner_id, slot), pet) in &scene.pets {
        let layer = map_entity_foothold_layer(scene, pet.body.foothold_id, pet.x, pet.y);
        if matches!(pass, MapEntityDrawPass::Layer(pass_layer) if pass_layer == layer)
            && pet.frames().get(pet.frame_index()).is_some()
        {
            let sort_id = owner_id.saturating_mul(3).saturating_add(i32::from(slot));
            actors.push((pet.y, 3, sort_id, Actor::Pet(pet)));
        }
    }
    for (&object_id, mob) in &scene.mobs {
        let layer = map_entity_foothold_layer(scene, mob.body.foothold_id, mob.x, mob.y);
        if matches!(pass, MapEntityDrawPass::Layer(pass_layer) if pass_layer == layer)
            && mob
                .animations
                .get(&mob.motion)
                .and_then(|frames| frames.get(mob.frame_index))
                .is_some()
        {
            actors.push((mob.y, 3, object_id, Actor::Mob(mob)));
        }
    }
    for (&object_id, npc) in &scene.npcs {
        let layer = map_entity_foothold_layer(scene, npc.foothold, npc.x, npc.y);
        if matches!(pass, MapEntityDrawPass::Layer(pass_layer) if pass_layer == layer)
            && (npc.imitated_look_sprites.is_some()
                || npc.frames().get(npc.frame_index).is_some())
        {
            actors.push((npc.y, 4, object_id, Actor::Npc(npc)));
        }
    }
    for (&object_id, mist) in &scene.mists {
        if graphics_quality <= 25 || mist.right <= mist.left || mist.bottom <= mist.top {
            continue;
        }
        let layer = map_entity_foothold_layer(
            scene,
            0,
            (f64::from(mist.left) + f64::from(mist.right)) / 2.0,
            f64::from(mist.bottom),
        );
        if matches!(pass, MapEntityDrawPass::Layer(pass_layer) if pass_layer == layer) {
            let y = (f64::from(mist.top) + f64::from(mist.bottom)) / 2.0;
            actors.push((y, 5, object_id, Actor::Mist(mist)));
        }
    }
    if matches!(pass, MapEntityDrawPass::Overlays) {
        for (&owner_id, merchant) in &scene.hired_merchants {
            actors.push((merchant.y, 4, owner_id, Actor::HiredMerchant(merchant)));
        }
    }
    for (index, effect) in scene.character_effects.iter().enumerate() {
        let (actor_y, foothold, actor_x) = if let Some((x, y)) = effect.point_position {
            (Some(y), 0, x)
        } else {
            effect.character_id.map_or(
                (
                    Some(scene.player.y),
                    scene.player.foothold_id,
                    scene.player.x,
                ),
                |character_id| {
                    scene
                        .remote_players
                        .get(&character_id)
                        .map_or((None, 0, 0.0), |player| {
                            (Some(player.y), player.foothold, player.x)
                        })
                },
            )
        };
        let layer = if effect.point_position.is_some() {
            7
        } else {
            map_entity_foothold_layer(scene, foothold, actor_x, actor_y.unwrap_or(0.0))
        };
        if let Some(actor_y) = actor_y
            && matches!(pass, MapEntityDrawPass::Layer(pass_layer) if pass_layer == layer)
            && effect.frames.get(effect.frame_index).is_some()
        {
            actors.push((
                actor_y,
                if effect.point_position.is_some() || effect.z < 0 {
                    5
                } else {
                    7
                },
                index as i32,
                Actor::CharacterSkillEffect(effect),
            ));
        }
    }
    for (&character_id, player) in &scene.remote_players {
        let layer = map_entity_foothold_layer(scene, player.foothold, player.x, player.y);
        if matches!(pass, MapEntityDrawPass::Layer(pass_layer) if pass_layer == layer) {
            if player.dead
                && let Some(art) = player.death_art.as_ref()
            {
                actors.push((player.y, 6, character_id, Actor::RemoteDeath(player, art)));
            } else if player
                .active_avatar_frame
                .and_then(|index| player.avatar_frames.get(index))
                .is_some()
            {
                actors.push((player.y, 6, character_id, Actor::RemotePlayer(player)));
            }
        }
    }
    if let Some(death_art) = scene.local_death_art.as_ref() {
        if matches!(pass, MapEntityDrawPass::Layer(layer) if layer == local_player_layer) {
            actors.push((scene.player.y, 6, 0, Actor::LocalDeath(death_art)));
        }
    } else if let Some(frame) = scene
        .active_avatar_frame
        .and_then(|index| scene.avatar_frames.get(index))
    {
        if matches!(pass, MapEntityDrawPass::Layer(layer) if layer == local_player_layer) {
            actors.push((scene.player.y, 6, 0, Actor::LocalPlayer(frame)));
        }
    }
    if matches!(pass, MapEntityDrawPass::Overlays) {
        for (index, projectile) in scene.projectiles.iter().enumerate() {
            if projectile.frames.get(projectile.frame_index).is_some() {
                actors.push((projectile.y, 8, index as i32, Actor::Projectile(projectile)));
            }
        }
    }
    actors.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.cmp(&right.2))
    });

    for (_, _, object_id, actor) in actors {
        match actor {
            Actor::Drop(drop) => {
                let Some(frame) = drop.frames.get(drop.frame_index) else {
                    continue;
                };
                let (texture_index, rect) = frame.sprite;
                let Some(texture) = textures.get(texture_index) else {
                    continue;
                };
                let opacity = match &drop.motion {
                    DropMotion::PickedUp { elapsed, .. } => {
                        (1.0 - elapsed.as_secs_f32() / 0.384).clamp(0.0, 1.0)
                    }
                    _ => 1.0,
                };
                draws.push(SpriteDraw {
                    texture,
                    x: rect.x + (drop.x - scene.player.x) as f32,
                    y: rect.y + (drop.y - scene.player.y) as f32,
                    width: rect.width,
                    height: rect.height,
                    uv: [0.0, 0.0, 1.0, 1.0],
                    tint: [1.0, 1.0, 1.0],
                    blend_mode: rect.blend_mode,
                    flip_x: false,
                    opacity,
                });
            }
            Actor::Door(door) => {
                let Some(frame) = scene.door_frames.get(door.frame_index) else {
                    continue;
                };
                let offset_x = (door.x - scene.player.x) as f32;
                let offset_y = (door.y - scene.player.y) as f32;
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
            Actor::Reactor(reactor) => {
                let frame = reactor
                    .active_frames
                    .as_ref()
                    .and_then(|frames| frames.get(reactor.frame_index))
                    .unwrap_or(&reactor.idle_frame);
                let offset_x = (reactor.x - scene.player.x) as f32;
                let offset_y = (reactor.y - scene.player.y) as f32;
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
            Actor::Summon(summon) => {
                let Some(frame) = summon.frames.get(summon.frame_index) else {
                    continue;
                };
                let offset_x = (summon.x - scene.player.x) as f32;
                let offset_y = (summon.y - scene.player.y) as f32;
                let flipped = summon.stance & 1 == 0;
                for (texture_index, rect) in &frame.sprites {
                    let Some(texture) = textures.get(*texture_index) else {
                        continue;
                    };
                    draws.push(SpriteDraw {
                        texture,
                        x: if flipped {
                            1024.0 - (rect.x + rect.width) + offset_x
                        } else {
                            rect.x + offset_x
                        },
                        y: rect.y + offset_y,
                        width: rect.width,
                        height: rect.height,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: rect.blend_mode,
                        flip_x: flipped,
                        opacity: 1.0,
                    });
                }
            }
            Actor::Dragon(dragon) => {
                let Some(frame) = dragon.frames.get(dragon.frame_index) else {
                    continue;
                };
                let offset_x = (dragon.x - scene.player.x) as f32;
                let offset_y = (dragon.y - scene.player.y) as f32;
                for (texture_index, rect) in &frame.sprites {
                    let Some(texture) = textures.get(*texture_index) else {
                        continue;
                    };
                    draws.push(SpriteDraw {
                        texture,
                        x: if dragon.flipped {
                            1024.0 - (rect.x + rect.width) + offset_x
                        } else {
                            rect.x + offset_x
                        },
                        y: rect.y + offset_y,
                        width: rect.width,
                        height: rect.height,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: rect.blend_mode,
                        flip_x: dragon.flipped,
                        opacity: 1.0,
                    });
                }
            }
            Actor::Pet(pet) => {
                let Some(frame) = pet.frames().get(pet.frame_index()) else {
                    continue;
                };
                let offset_x = (pet.x - scene.player.x) as f32;
                let offset_y = (pet.y - scene.player.y) as f32;
                let flipped = pet.stance & 1 == 1;
                for (texture_index, rect) in &frame.sprites {
                    let Some(texture) = textures.get(*texture_index) else {
                        continue;
                    };
                    draws.push(SpriteDraw {
                        texture,
                        x: if flipped {
                            1024.0 - (rect.x + rect.width) + offset_x
                        } else {
                            rect.x + offset_x
                        },
                        y: rect.y + offset_y,
                        width: rect.width,
                        height: rect.height,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: rect.blend_mode,
                        flip_x: flipped,
                        opacity: 1.0,
                    });
                }
                if let Some((texture_index, rect)) = pet.name_label
                    && let Some(texture) = textures.get(texture_index)
                {
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
                if pet.speech_bubble_remaining.is_some()
                    && let Some(texture) = pet_speech_textures.get(&(pet.owner_id, pet.slot))
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: 512.0 + offset_x - SPEECH_BUBBLE_ANCHOR.0 as f32,
                        y: GAMEPLAY_CAMERA_CENTER_Y + offset_y
                            - 48.0
                            - SPEECH_BUBBLE_ANCHOR.1 as f32,
                        width: SPEECH_BUBBLE_WIDTH as f32,
                        height: SPEECH_BUBBLE_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
            }
            Actor::Mob(mob) => {
                let Some(frame) = mob
                    .animations
                    .get(&mob.motion)
                    .and_then(|frames| frames.get(mob.frame_index))
                else {
                    continue;
                };
                let offset_x = (mob.x - scene.player.x) as f32;
                let offset_y = (mob.y - scene.player.y) as f32;
                let flipped = !mob.no_flip && (mob.server_flip != mob.faces_right);
                let tint = mob.status_tint();
                let opacity = if let Some(remaining) = mob.fade_remaining {
                    (remaining.as_secs_f32() / MOB_FADE_DURATION.as_secs_f32()).clamp(0.0, 1.0)
                } else if let Some(remaining) = mob.fade_in_remaining {
                    (1.0 - remaining.as_secs_f32() / MOB_FADE_DURATION.as_secs_f32())
                        .clamp(0.0, 1.0)
                } else {
                    1.0
                };
                append_mob_hit_effect_draws(mob, scene, textures, false, draws);
                for (texture_index, rect) in &frame.sprites {
                    let Some(texture) = textures.get(*texture_index) else {
                        continue;
                    };
                    let x = if flipped {
                        1024.0 - (rect.x + rect.width) + offset_x
                    } else {
                        rect.x + offset_x
                    };
                    if mob.is_frozen() {
                        draws.push(SpriteDraw {
                            texture,
                            x,
                            y: rect.y + offset_y,
                            width: rect.width,
                            height: rect.height,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: rect.blend_mode,
                            flip_x: flipped,
                            opacity: 1.0,
                        });
                    }
                    draws.push(SpriteDraw {
                        texture,
                        x,
                        y: rect.y + offset_y,
                        width: rect.width,
                        height: rect.height,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint,
                        blend_mode: rect.blend_mode,
                        flip_x: flipped,
                        opacity,
                    });
                }
                append_mob_hit_effect_draws(mob, scene, textures, true, draws);
                if !mob.dying && !mob.hp_bar_remaining.is_zero() {
                    if let Some(percent) = mob.hp_percent.filter(|percent| *percent > 0) {
                        let top = frame
                            .sprites
                            .iter()
                            .map(|(_, rect)| rect.y)
                            .fold(f32::INFINITY, f32::min)
                            + offset_y;
                        if top.is_finite() {
                            append_mob_hp_bar(
                                draws,
                                solid_textures,
                                512.0 + offset_x - 25.0,
                                top - 13.0,
                                percent,
                            );
                        }
                    }
                }
            }
            Actor::Npc(npc) => {
                let offset_x = (npc.x - scene.player.x) as f32;
                let offset_y = (npc.y - scene.player.y) as f32;
                let sprite_draws = if let Some(imitated) = npc.imitated_look_sprites.as_ref() {
                    imitated.as_slice()
                } else if let Some(frame) = npc.frames().get(npc.frame_index) {
                    frame.sprites.as_slice()
                } else {
                    continue;
                };
                for (texture_index, rect) in sprite_draws {
                    let Some(texture) = textures.get(*texture_index) else {
                        continue;
                    };
                    draws.push(SpriteDraw {
                        texture,
                        x: if npc.flipped {
                            1024.0 - (rect.x + rect.width) + offset_x
                        } else {
                            rect.x + offset_x
                        },
                        y: rect.y + offset_y,
                        width: rect.width,
                        height: rect.height,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: rect.blend_mode,
                        flip_x: npc.flipped,
                        opacity: 1.0,
                    });
                }
                if let Some(mark) = npc.quest_mark_frames.get(npc.quest_mark_frame_index) {
                    for (texture_index, rect) in &mark.sprites {
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
                if let Some(anchors) = npc.maple_tv_anchors {
                    let message_x = 512.0 + offset_x + f32::from(anchors.message_x);
                    let message_y =
                        GAMEPLAY_CAMERA_CENTER_Y + offset_y + f32::from(anchors.message_y);
                    if let Some(texture) = maple_tv_idle_texture {
                        draws.push(SpriteDraw {
                            texture,
                            x: message_x,
                            y: message_y,
                            width: texture.width() as f32,
                            height: texture.height() as f32,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                    if let Some(texture) = maple_tv_ad_texture {
                        draws.push(SpriteDraw {
                            texture,
                            x: 512.0 + offset_x + f32::from(anchors.ad_x),
                            y: GAMEPLAY_CAMERA_CENTER_Y + offset_y + f32::from(anchors.ad_y),
                            width: texture.width() as f32,
                            height: texture.height() as f32,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                    if let Some(broadcast) = maple_tv_broadcast {
                        let theme_index = match broadcast.type_id % 4 {
                            1 => Some(0),
                            2 => Some(1),
                            _ => None,
                        };
                        if let Some(index) = theme_index
                            && let Some(frame) = maple_tv_theme_frames[index]
                        {
                            let texture = &frame.texture;
                            let origin = frame.origin;
                            draws.push(SpriteDraw {
                                texture,
                                x: message_x + 120.0 - f32::from(origin.0),
                                y: message_y + 45.0 - f32::from(origin.1),
                                width: texture.width() as f32,
                                height: texture.height() as f32,
                                uv: [0.0, 0.0, 1.0, 1.0],
                                tint: [1.0, 1.0, 1.0],
                                blend_mode: BlendMode::Alpha,
                                flip_x: false,
                                opacity: 1.0,
                            });
                        }
                        for (texture_index, rect) in &broadcast.sender_avatar_draws {
                            if let Some(texture) = textures.get(*texture_index) {
                                draws.push(SpriteDraw {
                                    texture,
                                    x: message_x + rect.x,
                                    y: message_y + rect.y,
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
                        if broadcast.has_partner {
                            for (texture_index, rect) in &broadcast.partner_avatar_draws {
                                if let Some(texture) = textures.get(*texture_index) {
                                    draws.push(SpriteDraw {
                                        texture,
                                        x: message_x + rect.x,
                                        y: message_y + rect.y,
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
                        }
                        if let Some(texture) = maple_tv_text_texture {
                            draws.push(SpriteDraw {
                                texture,
                                x: message_x,
                                y: message_y,
                                width: 240.0,
                                height: 90.0,
                                uv: [0.0, 0.0, 1.0, 1.0],
                                tint: [1.0, 1.0, 1.0],
                                blend_mode: BlendMode::Alpha,
                                flip_x: false,
                                opacity: 1.0,
                            });
                        }
                    }
                }
            }
            Actor::Mist(mist) => {
                let Some(texture) = solid_textures.get(1) else {
                    continue;
                };
                let tint = match mist.mist_type {
                    1 => [0.60, 0.60, 0.54],
                    2 => [1.0, 1.0, 1.0],
                    _ => [0.50, 0.0, 0.50],
                };
                draws.push(SpriteDraw {
                    texture,
                    x: 512.0 + (f64::from(mist.left) - scene.player.x) as f32,
                    y: GAMEPLAY_CAMERA_CENTER_Y + (f64::from(mist.top) - scene.player.y) as f32,
                    width: (i64::from(mist.right) - i64::from(mist.left)) as f32,
                    height: (i64::from(mist.bottom) - i64::from(mist.top)) as f32,
                    uv: [0.0, 0.0, 1.0, 1.0],
                    tint,
                    blend_mode: BlendMode::Alpha,
                    flip_x: false,
                    opacity: 0.3,
                });
            }
            Actor::HiredMerchant(merchant) => {
                let offset_x = (merchant.x - scene.player.x) as f32;
                let offset_y = (merchant.y - scene.player.y) as f32;
                if let Some(frame) = merchant.frames.get(merchant.frame_index) {
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
                if let Some((texture_index, rect)) = merchant.sign {
                    if let Some(texture) = textures.get(texture_index) {
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
                if let Some((texture_index, rect)) = merchant.label {
                    if let Some(texture) = textures.get(texture_index) {
                        draws.push(SpriteDraw {
                            texture,
                            x: rect.x + offset_x,
                            y: rect.y + offset_y,
                            width: rect.width,
                            height: rect.height,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [0.0, 0.0, 0.0],
                            blend_mode: rect.blend_mode,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                }
            }
            Actor::CharacterSkillEffect(effect) => {
                if *effect_budget == 0 {
                    continue;
                }
                *effect_budget -= 1;
                let Some(frame) = effect.frames.get(effect.frame_index) else {
                    continue;
                };
                let (offset_x, offset_y, flipped) = if let Some((x, y)) = effect.point_position {
                    ((x - scene.player.x) as f32, (y - scene.player.y) as f32, false)
                } else if let Some(character_id) = effect.character_id {
                    let Some(player) = scene.remote_players.get(&character_id) else {
                        continue;
                    };
                    (
                        (player.x - scene.player.x + player.recoil_offset_x) as f32,
                        (player.y - scene.player.y) as f32,
                        !player.facing_left,
                    )
                } else {
                    (0.0, 0.0, scene.facing_right)
                };
                for (texture_index, rect) in &frame.sprites {
                    let Some(texture) = textures.get(*texture_index) else {
                        continue;
                    };
                    draws.push(SpriteDraw {
                        texture,
                        x: if flipped {
                            1024.0 - (rect.x + rect.width) + offset_x
                        } else {
                            rect.x + offset_x
                        },
                        y: rect.y + offset_y,
                        width: rect.width,
                        height: rect.height,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: rect.blend_mode,
                        flip_x: flipped,
                        opacity: 1.0,
                    });
                }
            }
            Actor::RemoteDeath(player, art) => {
                let offset_x = (player.x - scene.player.x + player.recoil_offset_x) as f32;
                let offset_y = (player.y - scene.player.y) as f32;
                append_item_aura_draws(
                    player.item_aura.as_ref(),
                    true,
                    offset_x,
                    offset_y,
                    textures,
                    draws,
                );
                append_death_art_draws(
                    art,
                    offset_x,
                    offset_y,
                    player.facing_left,
                    textures,
                    draws,
                );
                append_item_aura_draws(
                    player.item_aura.as_ref(),
                    false,
                    offset_x,
                    offset_y,
                    textures,
                    draws,
                );
                if let Some((texture_index, rect)) = player.name_label
                    && let Some(texture) = textures.get(texture_index)
                {
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
                        opacity: if player.hidden { 0.5 } else { 1.0 },
                    });
                }
                if let Some((texture_index, rect)) = player.guild_label
                    && let Some(texture) = textures.get(texture_index)
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: rect.x + offset_x,
                        y: rect.y + offset_y,
                        width: rect.width,
                        height: rect.height,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [0.0, 0.75, 1.0],
                        blend_mode: rect.blend_mode,
                        flip_x: false,
                        opacity: if player.hidden { 0.5 } else { 1.0 },
                    });
                }
                if player.chalkboard.is_some()
                    && let Some(texture) = chalkboard_textures.get(&object_id)
                {
                    append_chalkboard_draw(texture, offset_x, offset_y, draws);
                }
                if player.speech_bubble_remaining.is_some()
                    && let Some(texture) = speech_textures.get(&object_id)
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: 512.0 + offset_x - SPEECH_BUBBLE_ANCHOR.0 as f32,
                        y: GAMEPLAY_CAMERA_CENTER_Y + offset_y
                            - 85.0
                            - SPEECH_BUBBLE_ANCHOR.1 as f32,
                        width: SPEECH_BUBBLE_WIDTH as f32,
                        height: SPEECH_BUBBLE_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
            }
            Actor::RemotePlayer(player) => {
                let Some(frame) = player
                    .active_avatar_frame
                    .and_then(|index| player.avatar_frames.get(index))
                else {
                    continue;
                };
                let offset_x = (player.x - scene.player.x + player.recoil_offset_x) as f32;
                let offset_y = (player.y - scene.player.y) as f32;
                let facing_right = !player.facing_left;
                let mount_motion = player
                    .mount
                    .as_ref()
                    .map(|_| MountLookRuntime::motion_for_avatar(player.desired_avatar_motion));
                let rider_offset = player
                    .mount
                    .as_ref()
                    .zip(mount_motion)
                    .map_or((offset_x, offset_y), |(mount, motion)| {
                        let seat = mount.seat_offset(motion);
                        (offset_x + seat.0, offset_y + seat.1)
                    });
                if let Some(frame) = player
                    .mount
                    .as_ref()
                    .zip(mount_motion)
                    .and_then(|(mount, motion)| mount.current_frame(motion))
                {
                    for (texture_index, rect) in &frame.sprites {
                        let Some(texture) = textures.get(*texture_index) else {
                            continue;
                        };
                        draws.push(SpriteDraw {
                            texture,
                            x: if facing_right {
                                1024.0 - (rect.x + rect.width) + offset_x
                            } else {
                                rect.x + offset_x
                            },
                            y: rect.y + offset_y,
                            width: rect.width,
                            height: rect.height,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: rect.blend_mode,
                            flip_x: facing_right,
                            opacity: 1.0,
                        });
                    }
                }
                let (rider_offset_x, rider_offset_y) = rider_offset;
                let (tint, opacity) =
                    character_damage_tint(player.invincibility_remaining, player.hidden);
                append_item_aura_draws(
                    player.item_aura.as_ref(),
                    true,
                    rider_offset_x,
                    rider_offset_y,
                    textures,
                    draws,
                );
                for sprite in &frame.sprites {
                    if sprite.is_face
                        && let Some(expression) = player.face_expression.as_ref()
                    {
                        append_face_expression_draws(
                            expression,
                            frame.face_offset,
                            rider_offset_x,
                            rider_offset_y,
                            facing_right,
                            tint,
                            opacity,
                            textures,
                            draws,
                        );
                        continue;
                    }
                    let Some(texture) = textures.get(sprite.texture_index) else {
                        continue;
                    };
                    let original = &sprite.rect;
                    let x = if facing_right {
                        1024.0 - (original.x + original.width) + rider_offset_x
                    } else {
                        original.x + rider_offset_x
                    };
                    draws.push(SpriteDraw {
                        texture,
                        x,
                        y: original.y + rider_offset_y,
                        width: original.width,
                        height: original.height,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint,
                        blend_mode: original.blend_mode,
                        flip_x: facing_right,
                        opacity,
                    });
                }
                append_item_aura_draws(
                    player.item_aura.as_ref(),
                    false,
                    rider_offset_x,
                    rider_offset_y,
                    textures,
                    draws,
                );
                if let Some((texture_index, rect)) = player.name_label {
                    if let Some(texture) = textures.get(texture_index) {
                        draws.push(SpriteDraw {
                            texture,
                            x: rect.x + rider_offset_x,
                            y: rect.y + rider_offset_y,
                            width: rect.width,
                            height: rect.height,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: rect.blend_mode,
                            flip_x: false,
                            opacity: if player.hidden { 0.5 } else { 1.0 },
                        });
                    }
                }
                if let Some((texture_index, rect)) = player.guild_label {
                    if let Some(texture) = textures.get(texture_index) {
                        draws.push(SpriteDraw {
                            texture,
                            x: rect.x + rider_offset_x,
                            y: rect.y + rider_offset_y,
                            width: rect.width,
                            height: rect.height,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [0.0, 0.75, 1.0],
                            blend_mode: rect.blend_mode,
                            flip_x: false,
                            opacity: if player.hidden { 0.5 } else { 1.0 },
                        });
                    }
                }
                if player.chalkboard.is_some()
                    && let Some(texture) = chalkboard_textures.get(&object_id)
                {
                    append_chalkboard_draw(texture, rider_offset_x, rider_offset_y, draws);
                }
                if player.speech_bubble_remaining.is_some()
                    && let Some(texture) = speech_textures.get(&object_id)
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: 512.0 + rider_offset_x - SPEECH_BUBBLE_ANCHOR.0 as f32,
                        y: GAMEPLAY_CAMERA_CENTER_Y + rider_offset_y
                            - 85.0
                            - SPEECH_BUBBLE_ANCHOR.1 as f32,
                        width: SPEECH_BUBBLE_WIDTH as f32,
                        height: SPEECH_BUBBLE_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
            }
            Actor::LocalPlayer(frame) => {
                let facing_right = scene.facing_right;
                let hidden = character_has_buff(
                    active_buffs,
                    openstory_protocol::BuffMaskGroup::Second,
                    STATUS_DARKSIGHT,
                );
                let (tint, opacity) =
                    character_damage_tint(scene.local_invincibility_remaining, hidden);
                let sit_offset_y = scene
                    .local_chair
                    .as_ref()
                    .map_or(0.0, |chair| chair.sit_offset_y);
                let mount_motion = scene
                    .local_mount
                    .as_ref()
                    .map(|_| MountLookRuntime::motion_for_avatar(scene.desired_avatar_motion));
                let rider_offset = scene
                    .local_mount
                    .as_ref()
                    .zip(mount_motion)
                    .map_or((0.0, sit_offset_y), |(mount, motion)| {
                        let seat = mount.seat_offset(motion);
                        (seat.0, sit_offset_y + seat.1)
                    });
                if let Some(chair) = scene.local_chair.as_ref() {
                    append_chair_animation_draws(
                        &chair.behind,
                        scene.facing_right,
                        tint,
                        opacity,
                        textures,
                        draws,
                    );
                }
                if let Some(frame) = scene
                    .local_mount
                    .as_ref()
                    .zip(mount_motion)
                    .and_then(|(mount, motion)| mount.current_frame(motion))
                {
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
                            y: rect.y + sit_offset_y,
                            width: rect.width,
                            height: rect.height,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: rect.blend_mode,
                            flip_x: facing_right,
                            opacity: 1.0,
                        });
                    }
                }
                let (rider_offset_x, rider_offset_y) = rider_offset;
                append_item_aura_draws(
                    scene.local_item_aura.as_ref(),
                    true,
                    rider_offset_x,
                    rider_offset_y,
                    textures,
                    draws,
                );
                for sprite in &frame.sprites {
                    if sprite.is_face
                        && let Some(expression) = scene.local_face_expression.as_ref()
                    {
                        append_face_expression_draws(
                            expression,
                            frame.face_offset,
                            rider_offset_x,
                            rider_offset_y,
                            facing_right,
                            tint,
                            opacity,
                            textures,
                            draws,
                        );
                        continue;
                    }
                    let Some(texture) = textures.get(sprite.texture_index) else {
                        continue;
                    };
                    let rect = &sprite.rect;
                    let x = if scene.facing_right {
                        1024.0 - (rect.x + rect.width) + rider_offset_x
                    } else {
                        rect.x + rider_offset_x
                    };
                    draws.push(SpriteDraw {
                        texture,
                        x,
                        y: rect.y + rider_offset_y,
                        width: rect.width,
                        height: rect.height,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint,
                        blend_mode: rect.blend_mode,
                        flip_x: scene.facing_right,
                        opacity,
                    });
                }
                if let Some(chair) = scene.local_chair.as_ref() {
                    append_chair_animation_draws(
                        &chair.front,
                        scene.facing_right,
                        tint,
                        opacity,
                        textures,
                        draws,
                    );
                }
                append_item_aura_draws(
                    scene.local_item_aura.as_ref(),
                    false,
                    rider_offset_x,
                    rider_offset_y,
                    textures,
                    draws,
                );
                if scene.local_chalkboard.is_some()
                    && let Some(texture) = local_chalkboard_texture
                {
                    append_chalkboard_draw(texture, rider_offset_x, rider_offset_y, draws);
                }
                if scene.local_speech_bubble_remaining.is_some()
                    && let Some(texture) = local_speech_texture
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: 512.0 + rider_offset_x - SPEECH_BUBBLE_ANCHOR.0 as f32,
                        y: GAMEPLAY_CAMERA_CENTER_Y + rider_offset_y
                            - 85.0
                            - SPEECH_BUBBLE_ANCHOR.1 as f32,
                        width: SPEECH_BUBBLE_WIDTH as f32,
                        height: SPEECH_BUBBLE_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
            }
            Actor::LocalDeath(art) => {
                append_item_aura_draws(
                    scene.local_item_aura.as_ref(),
                    true,
                    0.0,
                    0.0,
                    textures,
                    draws,
                );
                append_death_art_draws(
                    art,
                    0.0,
                    0.0,
                    !scene.facing_right,
                    textures,
                    draws,
                );
                append_item_aura_draws(
                    scene.local_item_aura.as_ref(),
                    false,
                    0.0,
                    0.0,
                    textures,
                    draws,
                );
            }
            Actor::Projectile(projectile) => {
                if *effect_budget == 0 {
                    continue;
                }
                *effect_budget -= 1;
                let Some(frame) = projectile.frames.get(projectile.frame_index) else {
                    continue;
                };
                let anchor_x = 512.0 + (projectile.x - scene.player.x) as f32;
                let anchor_y = GAMEPLAY_CAMERA_CENTER_Y + (projectile.y - scene.player.y) as f32;
                for (texture_index, rect) in &frame.sprites {
                    let Some(texture) = textures.get(*texture_index) else {
                        continue;
                    };
                    let x = if projectile.facing_left {
                        anchor_x - (rect.x + rect.width)
                    } else {
                        anchor_x + rect.x
                    };
                    draws.push(SpriteDraw {
                        texture,
                        x,
                        y: anchor_y + rect.y,
                        width: rect.width,
                        height: rect.height,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: rect.blend_mode,
                        flip_x: projectile.facing_left,
                        opacity: 1.0,
                    });
                }
            }
        }
    }

    let camera_bias_x = scene.camera_offset.0 + scene.player.x as f32 - 512.0;
    let camera_bias_y = scene.camera_offset.1 + scene.player.y as f32 - GAMEPLAY_CAMERA_CENTER_Y;
    for draw in &mut draws[actor_draw_start..] {
        draw.x += camera_bias_x;
        draw.y += camera_bias_y;
    }

    if matches!(pass, MapEntityDrawPass::Overlays) {
        if let Some(frame) = scene
            .quest_clear_effect
            .as_ref()
            .and_then(|effect| effect.frames.get(effect.frame_index))
            && *effect_budget > 0
        {
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

        for portal in &scene.portals {
            let (frames, frame_index) = match portal.portal_type {
                2 => (
                    &scene.regular_portal_frames,
                    scene.regular_portal_frame_index,
                ),
                10 if portal.touched => {
                    (&scene.hidden_portal_frames, scene.hidden_portal_frame_index)
                }
                _ => continue,
            };
            let Some(frame) = frames.get(frame_index) else {
                continue;
            };
            let offset_x = (portal.x - scene.player.x) as f32 + camera_bias_x;
            let offset_y = (portal.y - scene.player.y) as f32 + camera_bias_y;
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

        if let Some(texture) = damage_texture {
            append_mob_damage_draws(scene, texture, damage_glyphs, effect_budget, draws);
        }
    }
}
