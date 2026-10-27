fn load_map_scene(
    archive: &NxArchive,
    character_archive: Option<&NxArchive>,
    map_id: i32,
    portal_id: u8,
    character: Option<&openstory_protocol::CharacterInfo>,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    if map_id < 0 {
        return Err(format!("unsupported special map id {map_id}"));
    }
    let map_path = format!("Map/Map{}/{map_id:09}.img", map_id / 100_000_000);
    let map = archive
        .root()
        .resolve(&map_path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("map node {map_path:?} was not found"))?;
    let underwater = map
        .child("info")
        .map_err(|error| error.to_string())?
        .and_then(|info| integer_property(&info, "swim"))
        .unwrap_or(0)
        != 0;
    let minimap = match minimap::MiniMapData::load(archive, map_id) {
        Ok(minimap) => minimap,
        Err(error) => {
            eprintln!("OpenStoryRS: could not load minimap for map {map_id}: {error}");
            None
        }
    };
    let spawn = map_spawn_point(&map, portal_id)?;
    let footholds = load_map_footholds(&map)?;
    let spawn_y = footholds
        .as_ref()
        .map(|tree| tree.y_below(spawn.0 as i16, spawn.1 as i16))
        .map(f32::from)
        .unwrap_or(spawn.1);
    let camera_bounds = map_camera_bounds(&map, footholds.as_ref());
    let camera_offset = camera_bounds.map_or(
        (512.0 - spawn.0, GAMEPLAY_CAMERA_CENTER_Y - spawn_y),
        |bounds| bounds.offset_for(f64::from(spawn.0), f64::from(spawn_y)),
    );
    let camera_x = camera_offset.0;
    let camera_y = camera_offset.1;
    let seats = load_map_seats(&map)?;
    let ladders = load_map_ladders(&map)?;
    let mut sprites = Vec::with_capacity(64);
    let mut cache = MapArtCache::default();
    let mut map_art_animations = Vec::new();
    let mut map_backgrounds = Vec::new();
    let portals = load_map_portals(&map)?;

    if let Some(back) = map.child("back").map_err(|error| error.to_string())? {
        let entries = numeric_children(&back)?;
        for entry in entries
            .iter()
            .filter(|entry| integer_property(entry, "front").unwrap_or(0) == 0)
        {
            let first_sprite = sprites.len();
            append_map_background(
                &mut sprites,
                archive,
                entry,
                camera_x,
                camera_y,
                &mut cache,
                &mut map_art_animations,
            )?;
            if let Some(background) =
                map_background_asset(entry, first_sprite..sprites.len(), &sprites, None)
            {
                map_backgrounds.push(background);
            }
        }
    }
    let layer_insert_indices = append_map_layers(
        &mut sprites,
        archive,
        &map,
        camera_x,
        camera_y,
        &mut cache,
        &mut map_art_animations,
    )?;
    let regular_portal_frames = if portals.iter().any(|portal| portal.portal_type == 2) {
        append_portal_animation(
            &mut sprites,
            archive,
            "MapHelper.img/portal/game/pv",
            &mut cache,
        )?
    } else {
        Vec::new()
    };
    let hidden_portal_frames = if portals.iter().any(|portal| portal.portal_type == 10) {
        append_portal_animation(
            &mut sprites,
            archive,
            "MapHelper.img/portal/game/ph/default/portalContinue",
            &mut cache,
        )?
    } else {
        Vec::new()
    };
    let mut door_frames = append_portal_animation(
        &mut sprites,
        archive,
        "MapHelper.img/portal/game/portal",
        &mut cache,
    )?;
    // The supplied v83 Map.nx has no dedicated `portal` node under `game`;
    // reuse the authored visible-portal animation so server doors remain visible.
    if door_frames.is_empty() {
        door_frames = append_portal_animation(
            &mut sprites,
            archive,
            "MapHelper.img/portal/game/pv",
            &mut cache,
        )?;
    }
    let avatar_frames =
        if let (Some(character_archive), Some(character)) = (character_archive, character) {
            append_character_animations(
                &mut sprites,
                character_archive,
                &character.look,
                512.0,
                GAMEPLAY_CAMERA_CENTER_Y,
                &mut cache,
            )?
        } else {
            Vec::new()
        };
    let foreground_start_index = sprites.len();
    if let Some(back) = map.child("back").map_err(|error| error.to_string())? {
        for entry in numeric_children(&back)?
            .iter()
            .filter(|entry| integer_property(entry, "front").unwrap_or(0) != 0)
        {
            let first_sprite = sprites.len();
            append_map_background(
                &mut sprites,
                archive,
                entry,
                camera_x,
                camera_y,
                &mut cache,
                &mut map_art_animations,
            )?;
            if let Some(background) =
                map_background_asset(entry, first_sprite..sprites.len(), &sprites, None)
            {
                map_backgrounds.push(background);
            }
        }
    }

    if let Some(environments) = map.child("env").map_err(|error| error.to_string())? {
        for entry in numeric_children(&environments)? {
            let name = entry.name().map_err(|error| error.to_string())?;
            let first_sprite = sprites.len();
            let first_animation = map_art_animations.len();
            append_map_background(
                &mut sprites,
                archive,
                &entry,
                camera_x,
                camera_y,
                &mut cache,
                &mut map_art_animations,
            )?;
            for animation in &mut map_art_animations[first_animation..] {
                animation.environment_name = Some(name.clone());
            }
            if let Some(environment) =
                map_background_asset(&entry, first_sprite..sprites.len(), &sprites, Some(name))
            {
                map_backgrounds.push(environment);
            }
        }
    }

    let background_draw_indices = map_backgrounds
        .iter()
        .flat_map(|background| background.sprites.clone())
        .collect::<HashSet<_>>();
    let world_draw_indices = sprites
        .iter()
        .enumerate()
        .filter(|(index, sprite)| {
            !background_draw_indices.contains(index)
                && !sprite.texture_key.starts_with("character:")
                && !sprite.texture_key.starts_with("label:")
                && !sprite.texture_key.starts_with("portal:")
        })
        .map(|(index, _)| index)
        .collect();
    append_text_sprite(
        &mut sprites,
        font,
        &format!("Map {map_id}"),
        16.0,
        12.0,
        15.0,
    );
    Ok(LoginSceneAssets {
        sprites,
        actions: Vec::new(),
        map_scene: Some(MapSceneData {
            map_id,
            underwater,
            footholds,
            spawn: (f64::from(spawn.0), f64::from(spawn_y)),
            camera_offset,
            camera_bounds,
            seats,
            ladders,
            world_draw_indices,
            map_backgrounds,
            layer_insert_indices,
            foreground_start_index,
            map_art_animations,
            avatar_frames,
            minimap,
            portals,
            regular_portal_frames,
            hidden_portal_frames,
            door_frames,
        }),
        animated_art: Vec::new(),
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn map_music_path(archive: &NxArchive, map_id: i32) -> Option<String> {
    if map_id < 0 {
        return None;
    }
    let map_path = format!("Map/Map{}/{map_id:09}.img", map_id / 100_000_000);
    let map = archive.root().resolve(&map_path).ok()??;
    let info = map.child("info").ok()??;
    let bgm = string_property(&info, "bgm")?;
    let (category, track) = bgm.split_once('/')?;
    Some(format!("{category}.img/{track}"))
}

const MAP_ART_CACHE_MAX_ENTRIES: usize = 16_384;
const MAP_ART_CACHE_MAX_BYTES: usize = 128 * 1024 * 1024;

type MapArtEntry = (u32, u32, (f32, f32), Arc<[u8]>);

fn map_background_asset(
    entry: &openstory_assets::NxNode,
    sprites: Range<usize>,
    draws: &[DecodedSprite],
    environment_name: Option<String>,
) -> Option<MapBackgroundAsset> {
    let sample = draws.get(sprites.start)?;
    let kind = integer_property(entry, "type").unwrap_or(0);
    let horizontal_tiled = matches!(kind, 1 | 3 | 4 | 6);
    let vertical_tiled = matches!(kind, 2 | 3 | 5 | 7);
    let rx = number_property(entry, "rx").unwrap_or(0.0);
    let ry = number_property(entry, "ry").unwrap_or(0.0);
    let horizontal_mobile = matches!(kind, 4 | 6) && rx != 0.0;
    let vertical_mobile = matches!(kind, 5 | 7) && ry != 0.0;
    let tile_width = integer_property(entry, "cx")
        .filter(|value| *value > 0)
        .map_or(sample.rect.width, |value| value as f32)
        .max(1.0);
    let tile_height = integer_property(entry, "cy")
        .filter(|value| *value > 0)
        .map_or(sample.rect.height, |value| value as f32)
        .max(1.0);
    let (tile_count_x, tile_count_y) = map_background_tile_counts(kind, tile_width, tile_height);
    Some(MapBackgroundAsset {
        sprites,
        environment_name,
        horizontal_tiled,
        vertical_tiled,
        tile_width,
        tile_height,
        tile_count_x,
        tile_count_y,
        horizontal_mobile,
        vertical_mobile,
        horizontal_speed: if horizontal_mobile { rx / 16.0 } else { 0.0 },
        vertical_speed: if vertical_mobile { ry / 16.0 } else { 0.0 },
        horizontal_parallax: if horizontal_mobile { 0.0 } else { rx },
        vertical_parallax: if vertical_mobile { 0.0 } else { ry },
    })
}

fn map_background_tile_counts(kind: i64, tile_width: f32, tile_height: f32) -> (usize, usize) {
    let horizontal = matches!(kind, 1 | 3 | 4 | 6);
    let vertical = matches!(kind, 2 | 3 | 5 | 7);
    let count_x = if horizontal {
        ((1024.0 / tile_width.max(1.0)).ceil() as usize + 3).clamp(1, 32)
    } else {
        1
    };
    let count_y = if vertical {
        ((768.0 / tile_height.max(1.0)).ceil() as usize + 3).clamp(1, 32)
    } else {
        1
    };
    (count_x, count_y)
}

#[derive(Default)]
struct MapArtCache {
    entries: HashMap<String, MapArtEntry>,
    last_access: HashMap<String, u64>,
    access_clock: u64,
    cached_bytes: usize,
}

impl MapArtCache {
    fn get(&mut self, path: &str) -> Option<MapArtEntry> {
        if !self.entries.contains_key(path) {
            return None;
        }
        let access = self.next_access_stamp();
        self.last_access.insert(path.to_owned(), access);
        self.entries.get(path).cloned()
    }

    fn insert(&mut self, path: String, entry: MapArtEntry) {
        if self.entries.contains_key(&path) {
            let access = self.next_access_stamp();
            self.last_access.insert(path, access);
            return;
        }
        let entry_bytes = entry.3.len();
        if entry_bytes > MAP_ART_CACHE_MAX_BYTES {
            return;
        }
        while self.entries.len() >= MAP_ART_CACHE_MAX_ENTRIES
            || self.cached_bytes.saturating_add(entry_bytes) > MAP_ART_CACHE_MAX_BYTES
        {
            let Some(evicted_path) = self
                .last_access
                .iter()
                .min_by_key(|(_, access)| *access)
                .map(|(path, _)| path.clone())
            else {
                break;
            };
            self.last_access.remove(&evicted_path);
            if let Some(evicted) = self.entries.remove(&evicted_path) {
                self.cached_bytes = self.cached_bytes.saturating_sub(evicted.3.len());
            }
        }
        let access = self.next_access_stamp();
        self.last_access.insert(path.clone(), access);
        self.cached_bytes += entry_bytes;
        self.entries.insert(path, entry);
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.last_access.clear();
        self.access_clock = 0;
        self.cached_bytes = 0;
    }

    fn next_access_stamp(&mut self) -> u64 {
        if self.access_clock == u64::MAX {
            let mut entries = self
                .last_access
                .iter()
                .map(|(path, access)| (path.clone(), *access))
                .collect::<Vec<_>>();
            entries.sort_by_key(|(_, access)| *access);
            for (rank, (path, _)) in entries.into_iter().enumerate() {
                self.last_access.insert(path, rank as u64);
            }
            self.access_clock = self.last_access.len() as u64;
        }
        self.access_clock += 1;
        self.access_clock
    }
}
