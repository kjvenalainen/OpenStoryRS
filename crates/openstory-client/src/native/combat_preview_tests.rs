use super::*;

fn scene_with_mob() -> MapSceneRuntime {
    let mut scene = MapSceneRuntime::new(
        MapSceneData {
            map_id: 100_000_000,
            underwater: false,
            footholds: None,
            spawn: (0.0, 0.0),
            camera_offset: (512.0, GAMEPLAY_CAMERA_CENTER_Y),
            camera_bounds: None,
            seats: Vec::new(),
            ladders: Vec::new(),
            world_draw_indices: Vec::new(),
            map_backgrounds: Vec::new(),
            layer_insert_indices: [0; 8],
            foreground_start_index: 0,
            map_art_animations: Vec::new(),
            avatar_frames: Vec::new(),
            minimap: None,
            portals: Vec::new(),
            regular_portal_frames: Vec::new(),
            hidden_portal_frames: Vec::new(),
            door_frames: Vec::new(),
        },
        &mut [],
    );
    scene.mobs.insert(
        1,
        MapMobRuntime {
            mob_id: 100_100,
            x: 100.0,
            y: 0.0,
            body: PhysicsBody::new(100.0, 0.0),
            foothold: 0,
            control_mode: 0,
            server_flip: false,
            no_flip: false,
            faces_right: false,
            traits: MobTraits {
                can_move: false,
                can_jump: false,
                can_fly: false,
                body_attack: false,
                boss: false,
                knockback: 0,
                speed: 0.0,
                fly_speed: 0.0,
                physical_attack: 1,
                level: 1,
                physical_defense: 0.0,
                magic_defense: 0.0,
                elemental_attributes: [1; 6],
                avoid: 0.0,
            },
            motion: MobMotion::Stand,
            server_stance: 0,
            frame_index: 0,
            elapsed: Duration::ZERO,
            animations: HashMap::from([(
                MobMotion::Stand,
                vec![MobFrameRuntime {
                    delay: Duration::from_millis(100),
                    sprites: vec![(
                        0,
                        SpriteRect {
                            x: 492.0,
                            y: GAMEPLAY_CAMERA_CENTER_Y - 40.0,
                            width: 40.0,
                            height: 40.0,
                            blend_mode: BlendMode::Alpha,
                        },
                    )],
                }],
            )]),
            skill_actions: HashMap::new(),
            hit_effects: Vec::new(),
            dying: false,
            fade_remaining: None,
            fade_in_remaining: None,
            hp_percent: None,
            hp_bar_remaining: Duration::ZERO,
            statuses: HashSet::new(),
            control_acked: false,
            movement_dirty: false,
            ai_counter: 0,
            random_state: 1,
            fly_direction: 0,
            aggro_target: None,
        },
    );
    scene
}

#[test]
fn mob_hit_reaction_uses_push_threshold_and_attack_direction() {
    let mut scene = scene_with_mob();
    let mob = scene.mobs.get_mut(&1).unwrap();
    mob.control_mode = 1;
    mob.traits.can_move = true;
    mob.traits.knockback = 50;
    mob.animations.insert(
        MobMotion::Hit,
        vec![MobFrameRuntime {
            delay: Duration::from_millis(100),
            sprites: Vec::new(),
        }],
    );

    scene.apply_mob_damage_reaction(1, 49, true);
    assert_eq!(scene.mobs[&1].motion, MobMotion::Stand);
    assert!(!scene.mobs[&1].movement_dirty);

    scene.apply_mob_damage_reaction(1, 50, true);
    let mob = &scene.mobs[&1];
    assert_eq!(mob.motion, MobMotion::Hit);
    assert!(mob.server_flip);
    assert_eq!(mob.server_stance, 8);
    assert_eq!(mob.ai_counter, 170);
    assert!(mob.movement_dirty);

    scene.apply_mob_damage_reaction(1, 50, false);
    assert!(!scene.mobs[&1].server_flip);
    assert_eq!(scene.mobs[&1].server_stance, 9);
}

#[test]
fn delayed_damage_survives_mob_removal_and_expires() {
    let mut scene = scene_with_mob();
    for (damage, critical, stack_offset) in [(123, false, 0.0), (456, true, 30.0), (0, false, 66.0)]
    {
        scene.schedule_mob_attack_impact(
            1,
            Arc::new(Vec::new()),
            false,
            Some((
                openstory_protocol::MobDamage {
                    object_id: 1,
                    damage,
                },
                critical,
                stack_offset,
            )),
            Duration::from_millis(80),
        );
    }
    assert!(scene.damage_numbers.is_empty());
    scene.mobs.remove(&1);
    scene.advance_mobs(9);
    assert!(scene.damage_numbers.is_empty());
    scene.advance_mobs(1);
    assert_eq!(scene.damage_numbers.len(), 3);
    assert_eq!(scene.damage_numbers[0].digits, [1, 2, 3]);
    assert_eq!(scene.damage_numbers[1].digits, [4, 5, 6]);
    assert!(scene.damage_numbers[1].critical);
    assert!(scene.damage_numbers[2].miss);
    assert_eq!(
        scene.damage_numbers[0].world_y - scene.damage_numbers[1].world_y,
        30.0
    );
    assert!(scene.pending_mob_attack_impacts.is_empty());
    scene.advance_mobs(188);
    assert!(scene.damage_numbers.is_empty());
}

#[test]
fn damage_roll_preserves_critical_for_local_presentation() {
    let mut scene = scene_with_mob();
    let mob = scene.mobs[&1].traits;
    let mut attack = BasicAttackStats {
        weapon_type: 130,
        attack_type: openstory_protocol::AttackType::Close,
        projectile_id: None,
        attack_class: 0,
        speed: 6,
        level: 1,
        minimum_damage: 100.0,
        maximum_damage: 100.0,
        intelligence: 0,
        luck: 0,
        magic_total: 0,
        magic_damage: false,
        element: None,
        fixed_damage: None,
        accuracy: 100,
        critical: 0.0,
    };
    assert_eq!(basic_attack_damage(&mut scene, attack, mob), (100, false));
    attack.critical = 1.0;
    assert_eq!(basic_attack_damage(&mut scene, attack, mob), (150, true));
    attack.fixed_damage = Some(10);
    assert_eq!(basic_attack_damage(&mut scene, attack, mob), (10, false));
}

#[test]
#[ignore = "requires local NX assets; writes target/ui-previews combat screenshots"]
fn render_combat_damage_preview() {
    let data_dir = std::env::var("OPENSTORY_DATA_DIR").expect("set OPENSTORY_DATA_DIR");
    let archive = |name: &str| NxArchive::open(Path::new(&data_dir).join(name)).unwrap();
    let effects = archive("Effect.nx");
    let mobs = archive("Mob.nx");
    let maps = archive("Map.nx");
    let characters = archive("Character.nx");
    let font = load_system_font().unwrap();
    let mut scene = scene_with_mob();
    let mut sprites = gameplay_backdrop(&maps, &characters, &font).unwrap();
    let mut mob_sprites = Vec::new();
    append_sprite(
        &mut mob_sprites,
        &mobs,
        "0100100.img/stand/0",
        512.0,
        GAMEPLAY_CAMERA_CENTER_Y,
        1.0,
    )
    .unwrap();
    scene
        .mobs
        .get_mut(&1)
        .unwrap()
        .animations
        .get_mut(&MobMotion::Stand)
        .unwrap()[0]
        .sprites[0]
        .1 = mob_sprites[0].rect;
    mob_sprites[0].rect.x += 100.0;
    sprites.extend(mob_sprites);
    for (damage, critical, stack_offset) in
        [(123, false, 0.0), (4567, true, 30.0), (0, false, 66.0)]
    {
        scene.schedule_mob_attack_impact(
            1,
            Arc::new(Vec::new()),
            false,
            Some((
                openstory_protocol::MobDamage {
                    object_id: 1,
                    damage,
                },
                critical,
                stack_offset,
            )),
            Duration::ZERO,
        );
    }
    let mut art = Vec::new();
    for state in [
        "NoRed1",
        "NoRed0",
        "NoCri1",
        "NoCri0",
        "NoBlue1",
        "NoBlue0",
        "NoViolet1",
        "NoViolet0",
    ] {
        for digit in 0..10 {
            append_sprite(
                &mut art,
                &effects,
                &format!("BasicEff.img/{state}/{digit}"),
                0.0,
                0.0,
                1.0,
            )
            .unwrap();
        }
    }
    append_sprite(
        &mut art,
        &effects,
        "BasicEff.img/NoRed0/Miss",
        0.0,
        0.0,
        1.0,
    )
    .unwrap();
    let glyphs = art
        .iter()
        .enumerate()
        .map(|(index, sprite)| DamageGlyph {
            uv: [index as f32, 0.0, 0.0, 0.0],
            width: sprite.rect.width,
            height: sprite.rect.height,
            origin_x: -sprite.rect.x,
            origin_y: -sprite.rect.y,
        })
        .collect::<Vec<_>>();
    let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/ui-previews");
    fs::create_dir_all(&output).unwrap();
    for (name, elapsed) in [
        ("combat-damage", Duration::ZERO),
        ("combat-damage-fading", Duration::from_millis(750)),
    ] {
        let mut pixels = compose_scene(&sprites, CANVAS_WIDTH, CANVAS_HEIGHT);
        for number in &mut scene.damage_numbers {
            number.elapsed = elapsed;
            for_each_damage_glyph(
                number,
                scene.camera_offset,
                &glyphs,
                |glyph, x, y, opacity| {
                    let source = &art[glyph.uv[0] as usize];
                    let mut sprite = DecodedSprite {
                        texture_key: source.texture_key.clone(),
                        rect: SpriteRect {
                            x,
                            y,
                            ..source.rect
                        },
                        depth_order: source.depth_order,
                        texture_width: source.texture_width,
                        texture_height: source.texture_height,
                        pixels: Arc::clone(&source.pixels),
                    };
                    let mut faded = sprite.pixels.to_vec();
                    for pixel in faded.chunks_exact_mut(4) {
                        pixel[3] = (f32::from(pixel[3]) * opacity) as u8;
                    }
                    sprite.pixels = faded.into();
                    composite_sprite(&mut pixels, CANVAS_WIDTH, CANVAS_HEIGHT, &sprite);
                },
            );
        }
        for (width, height) in PREVIEW_SIZES {
            let resized = scale_to_viewport(&pixels, width, height);
            write_png(
                &output.join(format!("{name}-{width}x{height}.png")),
                width,
                height,
                &resized,
            )
            .unwrap();
        }
    }
}
