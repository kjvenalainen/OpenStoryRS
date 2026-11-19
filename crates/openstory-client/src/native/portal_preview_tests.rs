use super::*;

#[test]
fn scripted_portals_reach_the_server_and_exits_take_priority() {
    let portal = |portal_type, name: &str, target_map_id| MapPortalRuntime {
        portal_type,
        name: name.into(),
        target_name: String::new(),
        target_map_id,
        x: 0.0,
        y: 0.0,
        touched: false,
    };
    let portals = [portal(7, "script", 999_999_999), portal(2, "exit", 100)];
    assert_eq!(
        select_portal(&portals, 0.0, 0.0, true, &[]).unwrap().name,
        "exit"
    );
    assert_eq!(
        select_portal(&portals[..1], 0.0, 0.0, true, &[])
            .unwrap()
            .name,
        "script"
    );
    assert!(select_portal(&portals[..1], 0.0, 0.0, false, &[]).is_none());
    let touch = [portal(9, "touch", 999_999_999)];
    assert_eq!(
        select_portal(&touch, 0.0, 0.0, false, &[]).unwrap().name,
        "touch"
    );
    assert!(select_portal(&touch, 100.0, 0.0, true, &[]).is_none());
}

#[test]
#[ignore = "requires OPENSTORY_DATA_DIR; writes target/ui-previews portal screenshots"]
fn mushroom_town_portal_coordinates_and_warp() {
    check_portal(10_000, (1077.0, 480.0), 485.0, 20_000);
}

#[test]
#[ignore = "requires OPENSTORY_DATA_DIR; writes target/ui-previews portal screenshots"]
fn second_map_portal_coordinates_and_warp() {
    check_portal(20_000, (630.0, 210.0), 215.0, 30_000);
}

fn check_portal(map_id: i32, anchor: (f64, f64), floor_y: f64, destination: i32) {
    let data_dir = std::env::var("OPENSTORY_DATA_DIR").expect("set OPENSTORY_DATA_DIR");
    let archive = |name: &str| NxArchive::open(Path::new(&data_dir).join(name)).unwrap();
    let maps = archive("Map.nx");
    let characters = archive("Character.nx");
    let ui = archive("UI.nx");
    let font = load_system_font().unwrap();
    let mut assets = load_map_scene(&maps, None, map_id, 0, None, &font).unwrap();
    let data = assets.map_scene.take().unwrap();
    let mut draws = assets
        .sprites
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s.rect))
        .collect::<Vec<_>>();
    let mut scene = MapSceneRuntime::new(data, &mut draws);
    let exit = scene.portals.iter().find(|p| p.name == "out00").unwrap();
    assert_eq!(
        (exit.x, exit.y, exit.target_map_id),
        (anchor.0, anchor.1, destination)
    );
    scene.player = PhysicsBody::new(exit.x, exit.y);
    for _ in 0..60 {
        PhysicsEngine.move_body(scene.footholds.as_ref().unwrap(), &mut scene.player);
    }
    assert_eq!(scene.player.y, floor_y);
    // Preview the outer glow where the old horizontal hitbox rejected Up.
    scene.player.x = anchor.0 + 35.0;
    let delta = scene.update_camera();
    for i in &scene.world_draw_indices {
        assets.sprites[*i].rect.x += delta.0;
        assets.sprites[*i].rect.y += delta.1;
    }
    let mut visible = scene
        .world_draw_indices
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    for (i, rect) in &scene.regular_portal_frames[0].sprites {
        let sprite = &mut assets.sprites[*i];
        sprite.rect = *rect;
        sprite.rect.x += anchor.0 as f32 + scene.camera_offset.0 - 512.0;
        sprite.rect.y += anchor.1 as f32 + scene.camera_offset.1 - GAMEPLAY_CAMERA_CENTER_Y;
        visible.insert(*i);
    }
    let mut sprites = assets
        .sprites
        .into_iter()
        .enumerate()
        .filter_map(|(i, s)| visible.contains(&i).then_some(s))
        .collect::<Vec<_>>();
    append_character_preview(
        &mut sprites,
        &characters,
        &preview_character().look,
        scene.player.x as f32 + scene.camera_offset.0,
        scene.player.y as f32 + scene.camera_offset.1,
        1.0,
        true,
        &mut MapArtCache::default(),
    )
    .unwrap();
    if let Some(minimap) = scene.minimap.as_ref() {
        let map_name = format!("Map {map_id}");
        let panel = minimap::render(
            minimap,
            &minimap::UiAssets::load(&ui).unwrap(),
            LogicalSize::new(1024.0, 768.0),
            &map_name,
            &map_name,
            minimap::Presentation::Normal,
            None,
            &[minimap::Marker {
                kind: minimap::MarkerKind::Player,
                x: scene.player.x as f32,
                y: scene.player.y as f32,
                facing_left: false,
                sprite: None,
            }],
            &font,
        )
        .unwrap();
        sprites.push(DecodedSprite {
            texture_key: "portal-test:minimap".into(),
            rect: SpriteRect {
                x: panel.x,
                y: panel.y,
                width: panel.width as f32,
                height: panel.height as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: panel.width,
            texture_height: panel.height,
            pixels: panel.pixels.into(),
        });
    }
    let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/ui-previews");
    fs::create_dir_all(&output).unwrap();
    write_png(
        &output.join(format!("map-{map_id}-portal.png")),
        1024,
        768,
        &compose_scene(&sprites, 1024, 768),
    )
    .unwrap();
    // The animation extends across the platform above. Its visible glow must
    // not make that platform (or the floor below) an entry point.
    for y in [floor_y - 120.0, floor_y + 120.0] {
        scene.player.y = y;
        assert!(scene.advance_portals(true, true, 1).is_none());
    }
    scene.player.y = floor_y;
    assert!(scene.advance_portals(false, true, 1).is_none());
    assert!(scene.advance_portals(true, false, 1).is_none());
    let Some(PortalWarp::ChangeMap {
        map_id,
        portal_name,
    }) = scene.advance_portals(true, true, 1)
    else {
        panic!("Up at Mushroom Town's exit should request a map change");
    };
    assert_eq!(map_id, destination);
    assert_eq!(portal_name, "out00");
    assert_eq!(
        openstory_protocol::change_map_packet(false, -1, &portal_name, false).unwrap(),
        [
            38, 0, 0, 255, 255, 255, 255, 5, 0, b'o', b'u', b't', b'0', b'0', 0, 0, 0
        ]
    );
    assert!(scene.advance_portals(true, true, 1).is_none());
    // Inside either edge of the authored 87 px glow, outside the old 50 px box.
    for dx in [-35.0, 35.0] {
        scene.portal_cooldown = 0;
        scene.player.x = anchor.0 + dx;
        assert!(
            matches!(
                scene.advance_portals(true, true, 1),
                Some(PortalWarp::ChangeMap { .. })
            ),
            "visible portal edge dx={dx} should be usable"
        );
    }
    scene.portal_cooldown = 0;
    scene.player.x = anchor.0 + 60.0;
    assert!(scene.advance_portals(true, true, 1).is_none());
}

#[test]
fn visible_portal_glow_is_usable_without_crossing_platforms() {
    let mut portal = MapPortalRuntime {
        portal_type: 2,
        name: "out00".into(),
        target_name: "in00".into(),
        target_map_id: 30_000,
        x: 630.0,
        y: 210.0,
        touched: false,
    };
    let frames = [PortalFrameRuntime {
        delay: Duration::from_millis(100),
        sprites: vec![(
            0,
            SpriteRect {
                x: 512.0 - 43.0,
                y: GAMEPLAY_CAMERA_CENTER_Y - 173.0,
                width: 87.0,
                height: 182.0,
                blend_mode: BlendMode::Alpha,
            },
        )],
    }];
    assert!(!portal_bounds_contains(&portal, 665.0, 215.0));
    for dx in [-35.0, 0.0, 35.0] {
        assert!(portal_activation_contains(
            &portal,
            portal.x + dx,
            215.0,
            &frames
        ));
    }
    for (dx, y) in [(60.0, 215.0), (0.0, 95.0), (0.0, 335.0)] {
        assert!(!portal_activation_contains(
            &portal,
            portal.x + dx,
            y,
            &frames
        ));
    }
    // Missing animation and hidden portals retain the legacy bounds.
    assert!(portal_activation_contains(&portal, 630.0, 215.0, &[]));
    assert!(!portal_activation_contains(&portal, 665.0, 215.0, &[]));
    portal.portal_type = 10;
    assert!(!portal_activation_contains(&portal, 665.0, 215.0, &frames));
}
