pub fn run() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let mut explicit_data_dir: Option<PathBuf> = None;
    let mut print_assets = false;
    let mut inspect_node: Option<(PathBuf, String)> = None;
    let mut inspect_map: Option<i32> = None;
    let mut inspect_avatar: Option<(u8, i32, i32)> = None;
    let mut ui_preview_screen: Option<String> = None;
    let mut ui_preview_output = PathBuf::from("target/ui-previews");
    let mut ui_preview_hitboxes = false;
    let mut connect_to: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--help" | "-h") => {
                println!(
                    "OpenStoryRS\n\nOptions:\n  --data-dir PATH        Use an existing folder of NX files\n  --list-assets          Validate and list required NX files\n  --inspect-nx FILE PATH  Inspect an NX node and decode it if it is a bitmap\n  --inspect-map ID       Decode and summarize a map scene without opening a window\n  --inspect-avatar SKIN FACE HAIR  Resolve and decode a base character appearance\n  --ui-preview SCREEN    Render an offline UI preview (or 'all')\n  --out DIR              Write UI previews under DIR (default: target/ui-previews)\n  --ui-hitboxes          Also write screen hitbox overlays\n  Preview screens: login, login-active, world-select, character-select-empty, character-select-populated, character-create-class, character-create-explorer, native-keybindings, native-quickslots, native-skill-macros, native-items, native-equipment, native-skills, native-pet-equipment, native-minimap-collapsed, native-minimap-normal, native-minimap-expanded, native-npc-store, native-cash-shop, native-cash-shop-locker, native-cash-shop-coupon, native-cash-shop-confirmation, native-monster-book, native-monster-book-detail, native-monster-book-detail-move, native-monster-book-detail-die, native-status, native-status-bar, native-clock, native-wedding, native-rps, native-event-list, native-dragon, native-monster-carnival, native-avatar-megaphone, native-quit-confirmation, native-server-notice, native-quest-log, native-quest-log-detail, native-chat, native-party, native-buddy, native-guild, native-alliance, native-character-info, native-world-map, native-world-map-link, native-world-map-search, native-world-map-marker, native-world-map-result, native-options, native-audio-settings\n  --connect HOST:PORT    Connect to a v83 server (default: OPENSTORY_SERVER or 127.0.0.1:8484)\n  --help                 Show this help"
                );
                return Ok(());
            }
            Some("--list-assets") => print_assets = true,
            Some("--inspect-map") => {
                let Some(value) = args.next() else {
                    return Err("--inspect-map requires a numeric map id".to_owned());
                };
                inspect_map = Some(
                    value
                        .to_string_lossy()
                        .parse::<i32>()
                        .map_err(|error| format!("invalid map id: {error}"))?,
                );
            }
            Some("--inspect-avatar") => {
                let Some(skin) = args.next() else {
                    return Err("--inspect-avatar requires SKIN FACE HAIR".to_owned());
                };
                let Some(face) = args.next() else {
                    return Err("--inspect-avatar requires SKIN FACE HAIR".to_owned());
                };
                let Some(hair) = args.next() else {
                    return Err("--inspect-avatar requires SKIN FACE HAIR".to_owned());
                };
                inspect_avatar = Some((
                    skin.to_string_lossy()
                        .parse::<u8>()
                        .map_err(|error| format!("invalid skin id: {error}"))?,
                    face.to_string_lossy()
                        .parse::<i32>()
                        .map_err(|error| format!("invalid face id: {error}"))?,
                    hair.to_string_lossy()
                        .parse::<i32>()
                        .map_err(|error| format!("invalid hair id: {error}"))?,
                ));
            }
            Some("--ui-preview") => {
                let Some(screen) = args.next() else {
                    return Err("--ui-preview requires a screen name or 'all'".to_owned());
                };
                ui_preview_screen = Some(screen.to_string_lossy().into_owned());
            }
            Some("--out") => {
                let Some(path) = args.next() else {
                    return Err("--out requires an output directory".to_owned());
                };
                ui_preview_output = PathBuf::from(path);
            }
            Some("--ui-hitboxes") => ui_preview_hitboxes = true,
            Some("--connect") => {
                let Some(address) = args.next() else {
                    return Err("--connect requires HOST:PORT".to_owned());
                };
                connect_to = Some(address.to_string_lossy().into_owned());
            }
            Some("--data-dir") => {
                let Some(path) = args.next() else {
                    return Err("--data-dir requires a path".to_owned());
                };
                explicit_data_dir = Some(PathBuf::from(path));
            }
            Some("--inspect-nx") => {
                let Some(file) = args.next() else {
                    return Err("--inspect-nx requires an NX file and node path".to_owned());
                };
                let Some(path) = args.next() else {
                    return Err("--inspect-nx requires an NX file and node path".to_owned());
                };
                inspect_node = Some((PathBuf::from(file), path.to_string_lossy().into_owned()));
            }
            Some(other) => return Err(format!("unknown option {other:?}; use --help")),
            None => return Err("arguments must be valid UTF-8".to_owned()),
        }
    }

    if connect_to.is_none() {
        connect_to = std::env::var("OPENSTORY_SERVER")
            .ok()
            .filter(|address| !address.trim().is_empty())
            .or_else(|| Some("127.0.0.1:8484".to_owned()));
    }

    let directory = AssetDirectory::resolve_and_validate(explicit_data_dir.as_deref())
        .map_err(|error| error.to_string())?;
    println!("NX data directory: {}", directory.root().display());

    if let Some((file, path)) = inspect_node {
        let archive_path = if file == PathBuf::from("UI.nx") || file == PathBuf::from("UI-v83.nx") {
            directory
                .ui_archive_path()
                .ok_or_else(|| "missing UI-v83.nx or UI.nx".to_owned())?
        } else {
            directory
                .file_size(&file)
                .map_err(|error| error.to_string())?;
            directory.root().join(file)
        };
        let archive = NxArchive::open(&archive_path).map_err(|error| error.to_string())?;
        let node = archive
            .root()
            .resolve(&path)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("NX node {path:?} was not found"))?;
        println!(
            "{path}: {:?}",
            node.value().map_err(|error| error.to_string())?
        );
        if let Some(bitmap) = node.bitmap().map_err(|error| error.to_string())? {
            let pixels = bitmap.decode_bgra().map_err(|error| error.to_string())?;
            println!(
                "decoded bitmap: {}x{} ({} bytes)",
                bitmap.width(),
                bitmap.height(),
                pixels.len()
            );
        }
        for child in node.children().map_err(|error| error.to_string())? {
            println!(
                "  {}: {:?}",
                child.name().map_err(|error| error.to_string())?,
                child.value().map_err(|error| error.to_string())?
            );
        }
        return Ok(());
    }

    if print_assets {
        for name in REQUIRED_NX_FILES {
            let size = directory
                .file_size(name)
                .map_err(|error| error.to_string())?;
            let archive = NxArchive::open(directory.root().join(name))
                .map_err(|error| format!("{name}: {error}"))?;
            let header = archive.header();
            let top_level_nodes = archive
                .root()
                .children()
                .map_err(|error| format!("{name}: {error}"))?
                .len();
            println!(
                "{name:16} {size:>12} bytes  {:>7} nodes  {:>4} top-level  {:>7} bitmaps  {:>6} audio",
                header.node_count, top_level_nodes, header.bitmap_count, header.audio_count
            );
        }
        let ui_path = directory
            .ui_archive_path()
            .ok_or_else(|| "missing UI-v83.nx or UI.nx".to_owned())?;
        let ui_name = ui_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("UI archive");
        let size = directory
            .file_size(ui_name)
            .map_err(|error| error.to_string())?;
        let archive = NxArchive::open(&ui_path).map_err(|error| format!("{ui_name}: {error}"))?;
        let header = archive.header();
        let top_level_nodes = archive
            .root()
            .children()
            .map_err(|error| format!("{ui_name}: {error}"))?
            .len();
        println!(
            "{ui_name:16} {size:>12} bytes  {:>7} nodes  {:>4} top-level  {:>7} bitmaps  {:>6} audio",
            header.node_count, top_level_nodes, header.bitmap_count, header.audio_count
        );
        return Ok(());
    }

    let map_archive =
        NxArchive::open(directory.root().join("Map.nx")).map_err(|error| error.to_string())?;
    if let Some((skin, face_id, hair_id)) = inspect_avatar {
        let character_archive = NxArchive::open(directory.root().join("Character.nx"))
            .map_err(|error| error.to_string())?;
        let character = openstory_protocol::CharacterInfo {
            look: openstory_protocol::CharacterLook {
                skin,
                face_id,
                hair_id,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut sprites = Vec::new();
        let mut cache = MapArtCache::default();
        let frames = append_character_animations(
            &mut sprites,
            &character_archive,
            &character.look,
            512.0,
            384.0,
            &mut cache,
        )?;
        let texture_count = sprites
            .iter()
            .map(|sprite| sprite.texture_key.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len();
        println!(
            "avatar skin={skin} face={face_id} hair={hair_id}: {} stand/walk frames, {} frame layers across {texture_count} decoded textures",
            frames.len(),
            sprites.len()
        );
        return Ok(());
    }
    if let Some(map_id) = inspect_map {
        let font = load_system_font()?;
        let scene = load_map_scene(&map_archive, None, map_id, 0, None, &font)?;
        let (animation_count, animation_frame_count) = scene
            .map_scene
            .as_ref()
            .map(|scene| {
                (
                    scene.map_art_animations.len(),
                    scene
                        .map_art_animations
                        .iter()
                        .map(|animation| animation.frames.len())
                        .sum::<usize>(),
                )
            })
            .unwrap_or_default();
        let seat_count = scene
            .map_scene
            .as_ref()
            .map_or(0, |scene| scene.seats.len());
        let texture_count = scene
            .sprites
            .iter()
            .map(|sprite| sprite.texture_key.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len();
        println!(
            "map {map_id}: {} visible sprite instances across {texture_count} decoded textures; {animation_count} animated map-art entries with {animation_frame_count} frames; {seat_count} authored seats",
            scene.sprites.len(),
        );
        return Ok(());
    }
    let ui_archive_path = directory
        .ui_archive_path()
        .ok_or_else(|| "missing UI-v83.nx or UI.nx".to_owned())?;
    let ui_archive = NxArchive::open(&ui_archive_path).map_err(|error| {
        format!(
            "could not open UI archive {}: {error}",
            ui_archive_path.display()
        )
    })?;
    // Use the selected v83 archive for legacy and modern client UI paths alike.
    let legacy_ui_archive = ui_archive.clone();
    let character_archive = NxArchive::open(directory.root().join("Character.nx"))
        .map_err(|error| error.to_string())?;
    let npc_archive =
        NxArchive::open(directory.root().join("Npc.nx")).map_err(|error| error.to_string())?;
    let mob_archive =
        NxArchive::open(directory.root().join("Mob.nx")).map_err(|error| error.to_string())?;
    let effect_archive =
        NxArchive::open(directory.root().join("Effect.nx")).map_err(|error| error.to_string())?;
    let item_archive =
        NxArchive::open(directory.root().join("Item.nx")).map_err(|error| error.to_string())?;
    let quest_archive =
        NxArchive::open(directory.root().join("Quest.nx")).map_err(|error| error.to_string())?;
    let skill_archive =
        NxArchive::open(directory.root().join("Skill.nx")).map_err(|error| error.to_string())?;
    let string_archive =
        NxArchive::open(directory.root().join("String.nx")).map_err(|error| error.to_string())?;
    let reactor_archive =
        NxArchive::open(directory.root().join("Reactor.nx")).map_err(|error| error.to_string())?;
    let etc_archive =
        NxArchive::open(directory.root().join("Etc.nx")).map_err(|error| error.to_string())?;
    let sound_archive =
        NxArchive::open(directory.root().join("Sound.nx")).map_err(|error| error.to_string())?;
    let font = load_system_font()?;
    if let Some(screen) = ui_preview_screen {
        return ui_preview::run(
            &screen,
            &ui_preview_output,
            ui_preview_hitboxes,
            &map_archive,
            &legacy_ui_archive,
            &ui_archive,
            &character_archive,
            &skill_archive,
            &etc_archive,
            &item_archive,
            &mob_archive,
            &quest_archive,
            &string_archive,
            &font,
        );
    }
    let login_scene = load_login_scene(&map_archive, &legacy_ui_archive)?;
    debug_ui_stage("assets loaded; creating Winit event loop");
    let event_loop = EventLoop::new().map_err(|error| error.to_string())?;
    debug_ui_stage("event loop created; initializing client state");
    let mut app = ClientApp::new(
        login_scene,
        font,
        connect_to,
        map_archive,
        legacy_ui_archive,
        ui_archive,
        character_archive,
        npc_archive,
        mob_archive,
        effect_archive,
        item_archive,
        quest_archive,
        skill_archive,
        string_archive,
        reactor_archive,
        etc_archive,
        sound_archive,
    );
    debug_ui_stage("client state ready; entering Winit event loop");
    let result = event_loop.run_app(&mut app);
    debug_ui_stage("Winit event loop returned");
    result.map_err(|error| error.to_string())
}
