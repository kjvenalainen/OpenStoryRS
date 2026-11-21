use super::*;

const MAX_MAP_PREFETCH_CANDIDATES: usize = 3;
const MAX_PREFETCHED_MAP_SCENES: usize = 3;
const MAX_PREFETCHED_MAP_SCENE_BYTES: usize = 128 * 1024 * 1024;
const MAP_TEXTURE_UPLOAD_BATCH_BYTES: usize = 4 * 1024 * 1024;
const MAP_TEXTURE_UPLOAD_BATCH_IMAGES: usize = 4;
const MAP_TEXTURE_UPLOAD_BATCH_MAX_DIMENSION: u32 = 512;

fn decoded_scene_bytes(scene: &LoginSceneAssets) -> usize {
    let mut bitmaps = HashSet::new();
    scene
        .sprites
        .iter()
        .filter(|sprite| bitmaps.insert(sprite.pixels.as_ptr() as usize))
        .fold(0_usize, |total, sprite| {
            total.saturating_add(sprite.pixels.len())
        })
}

fn map_scene_is_blank(scene: &LoginSceneAssets) -> bool {
    scene
        .map_scene
        .as_ref()
        .is_some_and(|map| map.world_draw_indices.is_empty() && map.map_backgrounds.is_empty())
}

impl ClientApp {
    pub(super) fn respond_to_server_notice(&mut self, accept: bool) {
        if self.server_notice_popup.is_none() {
            return;
        }
        if accept
            && self
                .server_notice_popup
                .as_ref()
                .is_some_and(|popup| popup.page + 1 < popup.page_count)
        {
            let popup = self.server_notice_popup.as_ref().unwrap();
            let next_page = popup.page + 1;
            let result = build_server_notice_art(
                &self.legacy_ui_archive,
                &self.font,
                &popup.message,
                next_page,
            )
            .and_then(|(_, _, pixels, _, _, _)| {
                let renderer = self
                    .renderer
                    .as_ref()
                    .ok_or_else(|| "renderer unavailable".to_owned())?;
                renderer
                    .update_bgra(&popup.texture, &pixels)
                    .map_err(|error| error.to_string())
            });
            match result {
                Ok(()) => {
                    if let Some(popup) = self.server_notice_popup.as_mut() {
                        popup.page = next_page;
                    }
                }
                Err(error) => eprintln!("OpenStoryRS: could not advance server notice: {error}"),
            }
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
            return;
        }
        self.server_notice_popup = None;
        if self.accept_tos_pending {
            self.accept_tos_pending = false;
            if accept {
                match self.session.as_ref().map(|session| {
                    session.send_packet(openstory_protocol::accept_terms_of_service())
                }) {
                    Some(Ok(())) => {}
                    Some(Err(error)) => {
                        eprintln!("OpenStory: could not accept server terms: {error}");
                    }
                    None => eprintln!("OpenStory: no login connection for server terms acceptance"),
                }
            }
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn show_map_scene(&mut self, map_id: i32, portal: u8) {
        self.pending_map_scene_upload = None;
        self.clear_boss_hp_gauge();
        self.weather_overlay.clear();
        self.weather_texture = None;
        self.gameplay_input = GameplayInput::default();
        if self.map_loading_texture.is_none()
            && let Some(renderer) = self.renderer.as_ref()
        {
            let pixels = rasterize_label(&self.font, "Loading map...", 18.0, 180, 28);
            match renderer.upload_bgra(180, 28, &pixels) {
                Ok(texture) => self.map_loading_texture = Some(texture),
                Err(error) => {
                    eprintln!("OpenStoryRS: could not prepare map loading label: {error}")
                }
            }
        }

        self.map_loading_started_at = Some(Instant::now());
        self.map_loading_frame_rendered = false;
        self.map_initial_npc_loading_until = None;
        let character_look = self
            .active_character
            .as_ref()
            .map(|character| character.look.clone());
        if let Some(look) = character_look
            && let Some(prefetched) = self.take_prefetched_map_scene(map_id, portal, &look)
        {
            self.cancel_map_prefetch();
            let (sender, receiver) = std::sync::mpsc::channel();
            let _ = sender.send((map_id, portal, prefetched.music_path, Ok(prefetched.scene)));
            self.map_load_receiver = Some(receiver);
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
            return;
        }
        self.cancel_map_prefetch();

        let map_archive = self.map_archive.clone();
        let character_archive = self.character_archive.clone();
        let character = self.active_character.clone();
        let font = self.font.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::Builder::new()
            .name(format!("map-load-{map_id}"))
            .spawn(move || {
                let music_path = map_music_path(&map_archive, map_id);
                let scene = load_map_scene(
                    &map_archive,
                    Some(&character_archive),
                    map_id,
                    portal,
                    character.as_ref(),
                    &font,
                );
                let _ = sender.send((map_id, portal, music_path, scene));
            });
        match worker {
            Ok(_) => {
                self.map_load_receiver = Some(receiver);
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            Err(error) => {
                self.map_load_receiver = None;
                self.map_loading_started_at = None;
                self.map_loading_frame_rendered = false;
                self.map_initial_npc_loading_until = None;
                self.map_entry_refresh_pending = false;
                self.pending_map_transition_ack = None;
                eprintln!("OpenStoryRS: could not start loading map {map_id}: {error}");
            }
        }
    }

    pub(super) fn process_map_load_completion(&mut self) {
        if self.map_loading_started_at.is_some() && !self.map_loading_frame_rendered {
            return;
        }
        let Some(receiver) = self.map_load_receiver.as_ref() else {
            return;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                eprintln!("OpenStoryRS: map loading worker stopped unexpectedly");
                self.map_load_receiver = None;
                self.map_loading_started_at = None;
                self.map_loading_frame_rendered = false;
                self.map_initial_npc_loading_until = None;
                self.map_entry_refresh_pending = false;
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
                return;
            }
        };
        let (map_id, portal, music_path, scene) = result;
        self.map_load_receiver = None;
        match scene {
            Ok(scene) => {
                if map_scene_is_blank(&scene)
                    && self.return_from_unusable_map(map_id, "it contains no background or map art")
                {
                    return;
                }
                self.begin_map_scene_upload(map_id, portal, music_path, scene);
            }
            Err(error) => self.finish_map_scene_load(map_id, portal, music_path, Err(error)),
        }
    }

    fn return_from_unusable_map(&mut self, map_id: i32, reason: &str) -> bool {
        let Some((old_map_id, old_portal)) = self.current_field else {
            return false;
        };
        if old_map_id == map_id {
            return false;
        }

        eprintln!(
            "OpenStoryRS: map {map_id} is unusable because {reason}; returning to previous map {old_map_id}"
        );
        if let Some(character) = self.active_character.as_mut() {
            character.stats.map_id = old_map_id;
            character.stats.portal = old_portal;
        }

        let return_packet = openstory_protocol::return_to_previous_map_packet(old_map_id);
        match (self.session.as_ref(), return_packet) {
            (Some(session), Ok(return_packet)) => {
                let result = session
                    .send_packet(openstory_protocol::player_map_transfer_packet())
                    .and_then(|()| session.send_packet(return_packet));
                if let Err(error) = result {
                    eprintln!(
                        "OpenStoryRS: could not return from unusable map {map_id} to map {old_map_id}: {error}"
                    );
                    self.channel_connected = false;
                }
            }
            (Some(session), Err(error)) => {
                if let Err(send_error) =
                    session.send_packet(openstory_protocol::player_map_transfer_packet())
                {
                    eprintln!(
                        "OpenStoryRS: could not finish unusable map {map_id} transition: {send_error}"
                    );
                    self.channel_connected = false;
                }
                eprintln!(
                    "OpenStoryRS: could not encode return from unusable map {map_id}: {error}"
                );
            }
            (None, _) => eprintln!(
                "OpenStoryRS: no active session to return from unusable map {map_id} to map {old_map_id}"
            ),
        }

        self.map_loading_started_at = None;
        self.map_loading_frame_rendered = false;
        self.map_initial_npc_loading_until = None;
        self.map_entry_refresh_pending = false;
        self.map_entry_refresh_step = 0;
        self.pending_map_transition_ack = None;
        self.pending_map_scene_upload = None;
        self.start_map_prefetch();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
        true
    }

    fn begin_map_scene_upload(
        &mut self,
        map_id: i32,
        portal: u8,
        music_path: Option<String>,
        mut scene: LoginSceneAssets,
    ) {
        let local_name_label_index = append_scene_local_name_tag(
            &mut scene.sprites,
            scene.map_scene.is_some(),
            self.active_character.as_ref(),
            &self.font,
            &self.ui_archive,
            &self.character_archive,
        );
        let mut seen = HashSet::new();
        let unique_sprite_indices = scene
            .sprites
            .iter()
            .enumerate()
            .filter_map(|(index, sprite)| seen.insert(sprite.texture_key.clone()).then_some(index))
            .collect();
        self.pending_map_scene_upload = Some(PendingMapSceneUpload {
            map_id,
            portal,
            music_path,
            scene,
            local_name_label_index,
            unique_sprite_indices,
            next_sprite: 0,
            textures: Vec::new(),
            texture_indices: HashMap::new(),
        });
    }

    pub(super) fn advance_pending_map_scene_upload(&mut self) {
        let Some(pending) = self.pending_map_scene_upload.as_ref() else {
            return;
        };
        let start = pending.next_sprite;
        let mut end = start;
        let mut batch_bytes = 0_usize;
        while end < pending.unique_sprite_indices.len()
            && end - start < MAP_TEXTURE_UPLOAD_BATCH_IMAGES
        {
            let sprite = &pending.scene.sprites[pending.unique_sprite_indices[end]];
            let is_large = sprite.texture_width > MAP_TEXTURE_UPLOAD_BATCH_MAX_DIMENSION
                || sprite.texture_height > MAP_TEXTURE_UPLOAD_BATCH_MAX_DIMENSION;
            if end > start && is_large {
                break;
            }
            let bytes = (sprite.texture_width as usize)
                .saturating_mul(sprite.texture_height as usize)
                .saturating_mul(4);
            if end > start && batch_bytes.saturating_add(bytes) > MAP_TEXTURE_UPLOAD_BATCH_BYTES {
                break;
            }
            batch_bytes = batch_bytes.saturating_add(bytes);
            end += 1;
            if is_large {
                break;
            }
        }

        if end > start {
            let atlas_images = {
                let pending = self.pending_map_scene_upload.as_ref().unwrap();
                pending.unique_sprite_indices[start..end]
                    .iter()
                    .map(|index| {
                        let sprite = &pending.scene.sprites[*index];
                        BgraImage {
                            width: sprite.texture_width,
                            height: sprite.texture_height,
                            pixels: sprite.pixels.as_ref(),
                        }
                    })
                    .collect::<Vec<_>>()
            };
            let result = self
                .renderer
                .as_ref()
                .ok_or_else(|| "graphics are not ready to upload a map".to_owned())
                .and_then(|renderer| {
                    renderer
                        .upload_bgra_atlas(&atlas_images)
                        .map_err(|error| error.to_string())
                });
            match result {
                Ok(textures) => {
                    let pending = self.pending_map_scene_upload.as_mut().unwrap();
                    for (offset, texture) in textures.into_iter().enumerate() {
                        let sprite_index = pending.unique_sprite_indices[start + offset];
                        let key = pending.scene.sprites[sprite_index].texture_key.clone();
                        let texture_index = pending.textures.len();
                        pending.texture_indices.insert(key, texture_index);
                        pending.textures.push(texture);
                    }
                    pending.next_sprite = end;
                }
                Err(error) => {
                    let pending = self.pending_map_scene_upload.take().unwrap();
                    self.finish_map_scene_load(
                        pending.map_id,
                        pending.portal,
                        pending.music_path,
                        Err(error),
                    );
                    return;
                }
            }
        }

        let finished = self
            .pending_map_scene_upload
            .as_ref()
            .is_some_and(|pending| pending.next_sprite == pending.unique_sprite_indices.len());
        if finished {
            let pending = self.pending_map_scene_upload.take().unwrap();
            let result = self.replace_scene_with_uploaded(
                pending.scene,
                pending.local_name_label_index,
                pending.textures,
                pending.texture_indices,
            );
            self.finish_map_scene_load(pending.map_id, pending.portal, pending.music_path, result);
        } else if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn finish_map_scene_load(
        &mut self,
        map_id: i32,
        portal: u8,
        music_path: Option<String>,
        result: Result<(), String>,
    ) {
        self.pending_map_transition_ack = None;
        match result {
            Ok(()) => {
                let up_is_held = self.gameplay_input.up
                    || self
                        .gameplay_input
                        .held_bindings
                        .values()
                        .any(|binding| binding.action == 202);
                if up_is_held {
                    self.gameplay_input.suppress_up_until_release = true;
                }
                self.gameplay_input.up = false;
                self.clear_clock();
                if let Some(path) = music_path {
                    self.audio.play_music(&self.sound_archive, &path);
                }
                self.current_field = Some((map_id, portal));
                self.show_game_menu = false;
                self.chat_open = false;
                self.chat_input_focused = false;
                self.chat_input.clear();
                self.npc_dialogue = None;
                self.pending_npc_talk = None;
                self.character_info = None;
                self.character_info_texture = None;
                self.cash_shop = None;
                self.cash_shop_texture = None;
                self.show_status = false;
                self.status_auto_confirmation = false;
                self.status_waiting_for_stats = false;
                self.show_login_fields = false;
                if self
                    .active_character
                    .as_ref()
                    .is_some_and(|character| character.stats.hp == 0)
                {
                    self.enter_death_state();
                }
                self.start_map_prefetch();
                self.map_entry_refresh_step = 0;
                self.pending_map_transition_ack = Some((map_id, portal));
            }
            Err(error) => {
                let reason = format!("it could not be loaded or rendered: {error}");
                if self.return_from_unusable_map(map_id, &reason) {
                    return;
                }
                eprintln!("OpenStory: could not load or render map {map_id}: {error}");
                self.map_entry_refresh_pending = false;
            }
        }
        if self.pending_map_transition_ack.is_none() {
            self.map_loading_started_at = None;
            self.map_loading_frame_rendered = false;
            self.map_initial_npc_loading_until = None;
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn advance_pending_map_scene_finalize(&mut self) {
        if self.pending_map_transition_ack.is_none() {
            return;
        }
        match self.map_entry_refresh_step {
            0 => {
                if let Some(look) = self
                    .active_character
                    .as_ref()
                    .map(|character| character.look.clone())
                {
                    self.refresh_local_mount(&look);
                }
            }
            1 => self.load_native_status_bar(),
            2 => self.refresh_quickslots_texture(),
            3 => self.refresh_chat_texture(),
            4 => {
                self.refresh_minimap_texture();
                self.next_minimap_refresh = Instant::now() + Duration::from_millis(100);
            }
            5 if self.map_entry_refresh_pending => self.sync_equipped_look(),
            6 if self.map_entry_refresh_pending => self.refresh_inventory_texture(),
            7 if self.map_entry_refresh_pending => self.refresh_pet_equipment_texture(),
            8 if self.map_entry_refresh_pending => self.refresh_skill_book_entries(),
            9 if self.map_entry_refresh_pending => self.refresh_skill_book_texture(),
            5..=9 => {
                self.map_entry_refresh_step = 10;
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
                return;
            }
            _ => {
                let Some((map_id, portal)) = self.pending_map_transition_ack.take() else {
                    return;
                };
                // The v83 server keeps map-changing actions locked until the
                // field and its native UI are ready.
                if let Some(session) = self.session.as_ref()
                    && let Err(error) =
                        session.send_packet(openstory_protocol::player_map_transfer_packet())
                {
                    eprintln!("OpenStoryRS: could not complete map {map_id} transition: {error}");
                    self.channel_connected = false;
                }
                eprintln!("OpenStory: loaded map {map_id} at portal {portal}");
                self.map_entry_refresh_pending = false;
                // The v83 field stream has no marker for the end of the initial
                // NPC batch, so use a short quiet window after the transfer ack.
                self.map_initial_npc_loading_until =
                    Some(Instant::now() + INITIAL_NPC_LOADING_QUIET_WINDOW);
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
                return;
            }
        }
        self.map_entry_refresh_step = self.map_entry_refresh_step.saturating_add(1);
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn finish_initial_npc_loading_if_idle(&mut self, now: Instant) {
        if self
            .map_initial_npc_loading_until
            .is_none_or(|deadline| now < deadline)
        {
            return;
        }
        self.map_initial_npc_loading_until = None;
        self.map_loading_started_at = None;
        self.map_loading_frame_rendered = false;
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn process_map_prefetch_results(&mut self) {
        let mut ready = Vec::new();
        let mut disconnected = false;
        if let Some(receiver) = self.map_prefetch_receiver.as_ref() {
            loop {
                match receiver.try_recv() {
                    Ok(scene) => ready.push(scene),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        for scene in ready {
            self.cache_prefetched_map_scene(scene);
        }
        if disconnected {
            self.map_prefetch_receiver = None;
            self.map_prefetch_cancel = None;
        }
    }

    fn start_map_prefetch(&mut self) {
        self.cancel_map_prefetch();
        let Some(character) = self.active_character.clone() else {
            return;
        };
        let Some(scene) = self.map_scene.as_ref() else {
            return;
        };
        let map_id = scene.map_id;
        let origin = (scene.player.x, scene.player.y);
        let look = character.look.clone();
        let mut seen = HashSet::new();
        let mut candidates = scene
            .portals
            .iter()
            .filter(|portal| {
                portal.target_map_id >= 0
                    && portal.target_map_id < 999_999_999
                    && portal.target_map_id != map_id
            })
            .filter_map(|portal| {
                let key = (portal.target_map_id, portal.target_name.clone());
                if !seen.insert(key.clone()) {
                    return None;
                }
                let dx = portal.x - origin.0;
                let dy = portal.y - origin.1;
                Some((dx * dx + dy * dy, key.0, key.1))
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| left.0.total_cmp(&right.0));
        let candidates = candidates
            .into_iter()
            .take(MAX_MAP_PREFETCH_CANDIDATES)
            .map(|(_, destination, target_name)| (destination, target_name))
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return;
        }

        let map_archive = self.map_archive.clone();
        let character_archive = self.character_archive.clone();
        let font = self.font.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::Builder::new()
            .name(format!("map-prefetch-{map_id}"))
            .spawn(move || {
                for (destination, target_name) in candidates {
                    if worker_cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let portal_id =
                        map_portal_id_for_target(&map_archive, destination, &target_name);
                    let music_path = map_music_path(&map_archive, destination);
                    let Ok(scene) = load_map_scene(
                        &map_archive,
                        Some(&character_archive),
                        destination,
                        portal_id,
                        Some(&character),
                        &font,
                    ) else {
                        continue;
                    };
                    if worker_cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let decoded_bytes = decoded_scene_bytes(&scene);
                    if decoded_bytes > MAX_PREFETCHED_MAP_SCENE_BYTES {
                        continue;
                    }
                    if sender
                        .send(PrefetchedMapScene {
                            map_id: destination,
                            portal_id,
                            character_look: look.clone(),
                            music_path,
                            scene,
                            decoded_bytes,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            });
        match worker {
            Ok(_) => {
                self.map_prefetch_receiver = Some(receiver);
                self.map_prefetch_cancel = Some(cancel);
            }
            Err(error) => {
                eprintln!("OpenStoryRS: could not start adjacent map prefetch: {error}");
            }
        }
    }

    fn cancel_map_prefetch(&mut self) {
        if let Some(cancel) = self.map_prefetch_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.map_prefetch_receiver = None;
    }

    fn cache_prefetched_map_scene(&mut self, scene: PrefetchedMapScene) {
        if scene.decoded_bytes > MAX_PREFETCHED_MAP_SCENE_BYTES {
            return;
        }
        if let Some(index) = self.prefetched_map_scenes.iter().position(|cached| {
            cached.map_id == scene.map_id
                && cached.portal_id == scene.portal_id
                && cached.character_look == scene.character_look
        }) {
            if let Some(replaced) = self.prefetched_map_scenes.remove(index) {
                self.prefetched_map_scene_bytes = self
                    .prefetched_map_scene_bytes
                    .saturating_sub(replaced.decoded_bytes);
            }
        }
        while self.prefetched_map_scenes.len() >= MAX_PREFETCHED_MAP_SCENES
            || self
                .prefetched_map_scene_bytes
                .saturating_add(scene.decoded_bytes)
                > MAX_PREFETCHED_MAP_SCENE_BYTES
        {
            let Some(evicted) = self.prefetched_map_scenes.pop_front() else {
                break;
            };
            self.prefetched_map_scene_bytes = self
                .prefetched_map_scene_bytes
                .saturating_sub(evicted.decoded_bytes);
        }
        self.prefetched_map_scene_bytes = self
            .prefetched_map_scene_bytes
            .saturating_add(scene.decoded_bytes);
        self.prefetched_map_scenes.push_back(scene);
    }

    fn take_prefetched_map_scene(
        &mut self,
        map_id: i32,
        portal_id: u8,
        character_look: &openstory_protocol::CharacterLook,
    ) -> Option<PrefetchedMapScene> {
        let index = self.prefetched_map_scenes.iter().position(|scene| {
            scene.map_id == map_id
                && scene.portal_id == portal_id
                && &scene.character_look == character_look
        })?;
        let scene = self.prefetched_map_scenes.remove(index)?;
        self.prefetched_map_scene_bytes = self
            .prefetched_map_scene_bytes
            .saturating_sub(scene.decoded_bytes);
        Some(scene)
    }

    pub(super) fn select_character(&mut self, index: usize) {
        let Some((character, pic_state)) = self.character_list.as_ref().and_then(|list| {
            list.characters
                .get(index)
                .cloned()
                .map(|character| (character, list.pic_state))
        }) else {
            return;
        };
        self.active_character = None;
        self.taming_mount_stats = None;
        match pic_state {
            0 => self.begin_pic_entry(character.id, PicEntryStep::Register),
            1 => self.begin_pic_entry(character.id, PicEntryStep::Verify),
            2 => self.send_character_selection(character),
            state => eprintln!("OpenStory: unsupported PIC state {state}"),
        }
    }

    pub(super) fn request_character_deletion(&mut self, index: usize) {
        let Some((character_id, pic_state)) = self.character_list.as_ref().and_then(|list| {
            list.characters
                .get(index)
                .map(|character| (character.id, list.pic_state))
        }) else {
            return;
        };
        if !self.ensure_character_delete_confirmation_assets() {
            return;
        }
        self.character_delete_confirmation = Some(CharacterDeleteConfirmation {
            character_id,
            pic_state,
            step: CharacterDeleteConfirmationStep::Initial,
        });
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn ensure_character_delete_confirmation_assets(&mut self) -> bool {
        let prompts = [
            "This will delete your character, do you want to continue?",
            "Are you sure you want to delete your character?",
        ];
        for (index, prompt) in prompts.iter().enumerate() {
            if !self.character_delete_confirmation_draws[index].is_empty() {
                continue;
            }
            let sprites = build_character_delete_confirmation_sprites(
                &self.legacy_ui_archive,
                &self.font,
                prompt,
            )
            .or_else(|_| {
                build_character_delete_confirmation_sprites(&self.ui_archive, &self.font, prompt)
            });
            let (sprites, buttons) = match sprites {
                Ok(assets) => assets,
                Err(error) => {
                    eprintln!("OpenStory: character deletion confirmation is unavailable: {error}");
                    return false;
                }
            };
            match self.upload_dynamic_sprites(sprites) {
                Ok(draws) => {
                    self.character_delete_confirmation_draws[index] = draws;
                    self.character_delete_confirmation_buttons[index] = buttons.map(Some);
                }
                Err(error) => {
                    eprintln!(
                        "OpenStory: could not upload character deletion confirmation: {error}"
                    );
                    return false;
                }
            }
        }
        true
    }

    pub(super) fn confirm_character_deletion(&mut self, accept: bool) {
        let Some(confirmation) = self.character_delete_confirmation else {
            return;
        };
        if !accept {
            self.character_delete_confirmation = None;
        } else if confirmation.step == CharacterDeleteConfirmationStep::Initial {
            if let Some(confirmation) = self.character_delete_confirmation.as_mut() {
                confirmation.step = CharacterDeleteConfirmationStep::Final;
            }
        } else {
            self.character_delete_confirmation = None;
            self.begin_character_deletion(confirmation.character_id, confirmation.pic_state);
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn begin_character_deletion(&mut self, character_id: i32, pic_state: u8) {
        match pic_state {
            0 => {
                self.begin_pic_entry_for(
                    character_id,
                    PicEntryStep::Notice,
                    PicPurpose::DeleteCharacter,
                );
                if let Some(entry) = self.pic_entry.as_mut() {
                    entry.notice = Some("A PIC must be set before deleting characters".to_owned());
                }
                self.refresh_pic_dialog_texture();
            }
            1 => {
                self.begin_pic_entry_for(
                    character_id,
                    PicEntryStep::Verify,
                    PicPurpose::DeleteCharacter,
                );
                if let Some(entry) = self.pic_entry.as_mut() {
                    entry.notice = Some("Enter the PIC to confirm deletion".to_owned());
                }
                self.refresh_pic_dialog_texture();
            }
            2 => self.send_delete_character(character_id, ""),
            state => eprintln!("OpenStory: unsupported PIC state {state}"),
        }
    }

    pub(super) fn show_delete_notice(&mut self, character_id: i32, message: String) {
        self.begin_pic_entry_for(
            character_id,
            PicEntryStep::Notice,
            PicPurpose::DeleteCharacter,
        );
        if let Some(entry) = self.pic_entry.as_mut() {
            entry.notice = Some(message);
        }
        self.refresh_pic_dialog_texture();
    }

    pub(super) fn send_character_selection(
        &mut self,
        character: openstory_protocol::CharacterInfo,
    ) {
        self.forced_equipment_stats = ForcedEquipmentStats::default();
        self.character_selection_status =
            Some(format!("Connecting to {}...", character.stats.name));
        let (mac_addresses, hardware_id) = character_selection_hardware_info();
        let packet = match openstory_protocol::select_character(
            character.id,
            &mac_addresses,
            &hardware_id,
        ) {
            Ok(packet) => packet,
            Err(error) => {
                eprintln!("OpenStory: could not create character selection packet: {error}");
                self.character_selection_status = Some(
                    "Could not create the character request. Click Start to retry.".to_owned(),
                );
                self.refresh_character_scene();
                return;
            }
        };
        if let Some(session) = self.session.as_ref() {
            if let Err(error) = session.send_packet(packet) {
                eprintln!("OpenStory: could not send character selection: {error}");
                self.character_selection_status =
                    Some("Could not contact the login server. Click Start to retry.".to_owned());
            } else {
                self.monster_book = monster_book::State::default();
                self.monster_book_animation_texture = None;
                self.active_character = Some(character);
                eprintln!("OpenStory: sent character selection request");
            }
        } else {
            self.character_selection_status =
                Some("No login server connection. Click Start to retry.".to_owned());
        }
        self.refresh_character_scene();
    }

    pub(super) fn show_character_selection_failure(&mut self, message: &str) {
        self.active_character = None;
        self.taming_mount_stats = None;
        self.character_selection_status = Some(message.to_owned());
        self.refresh_character_scene();
    }

    pub(super) fn begin_pic_entry(&mut self, character_id: i32, step: PicEntryStep) {
        self.begin_pic_entry_for(character_id, step, PicPurpose::SelectCharacter);
    }

    pub(super) fn begin_pic_entry_for(
        &mut self,
        character_id: i32,
        step: PicEntryStep,
        purpose: PicPurpose,
    ) {
        self.pic_entry = Some(PicEntry {
            character_id,
            step,
            purpose,
            input: String::new(),
            first_input: String::new(),
            notice: None,
        });
        self.refresh_pic_dialog_texture();
    }

    pub(super) fn refresh_pic_dialog_texture(&mut self) {
        let Some(entry) = self.pic_entry.as_ref() else {
            self.pic_dialog_texture = None;
            return;
        };
        let (heading, guidance) = match entry.step {
            PicEntryStep::Register => ("Create a PIC", "Enter 6 to 16 letters or numbers"),
            PicEntryStep::ConfirmRegistration => ("Confirm your PIC", "Enter the new PIC again"),
            PicEntryStep::Verify => ("Enter your PIC", "Enter 6 to 16 letters or numbers"),
            PicEntryStep::Notice => ("Character deletion failed", "Press Enter to close"),
        };
        let masked_input = "*".repeat(entry.input.chars().count());
        let notice = entry
            .notice
            .as_deref()
            .unwrap_or("Press Enter to continue, Escape to cancel");
        let pixels = rasterize_pic_dialog(&self.font, heading, guidance, &masked_input, notice);
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        if let Some(texture) = self.pic_dialog_texture.as_ref() {
            if let Err(error) = renderer.update_bgra(texture, &pixels) {
                eprintln!("OpenStory: could not update PIC dialog: {error}");
            }
        } else {
            match renderer.upload_bgra(PIC_DIALOG_WIDTH, PIC_DIALOG_HEIGHT, &pixels) {
                Ok(texture) => self.pic_dialog_texture = Some(texture),
                Err(error) => eprintln!("OpenStory: could not create PIC dialog: {error}"),
            }
        }
    }

    pub(super) fn edit_pic_entry(&mut self, input: &str) {
        let Some(entry) = self.pic_entry.as_mut() else {
            return;
        };
        if entry.step == PicEntryStep::Notice {
            return;
        }
        for character in input
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
        {
            if entry.input.len() >= 16 {
                break;
            }
            entry.input.push(character);
            entry.notice = None;
        }
        self.refresh_pic_dialog_texture();
    }

    pub(super) fn submit_pic_entry(&mut self) {
        let Some(entry) = self.pic_entry.as_mut() else {
            return;
        };
        if entry.step == PicEntryStep::Notice {
            self.pic_entry = None;
            self.pic_dialog_texture = None;
            return;
        }
        if entry.input.len() < 6 {
            entry.notice = Some("The PIC needs at least 6 characters".to_owned());
            self.refresh_pic_dialog_texture();
            return;
        }
        if entry
            .input
            .as_bytes()
            .windows(3)
            .any(|digits| digits[0] == digits[1] && digits[1] == digits[2])
        {
            entry.notice = Some("The PIC cannot repeat a character three times".to_owned());
            self.refresh_pic_dialog_texture();
            return;
        }
        match entry.step {
            PicEntryStep::Register => {
                entry.first_input.clone_from(&entry.input);
                entry.input.clear();
                entry.step = PicEntryStep::ConfirmRegistration;
                entry.notice = None;
                self.refresh_pic_dialog_texture();
            }
            PicEntryStep::ConfirmRegistration => {
                if entry.input != entry.first_input {
                    entry.input.clear();
                    entry.first_input.clear();
                    entry.step = PicEntryStep::Register;
                    entry.notice = Some("The PIC entries did not match; try again".to_owned());
                    self.refresh_pic_dialog_texture();
                    return;
                }
                let character_id = entry.character_id;
                let pic = entry.input.clone();
                self.send_pic_registration(character_id, &pic);
            }
            PicEntryStep::Verify => {
                let character_id = entry.character_id;
                let pic = entry.input.clone();
                match entry.purpose {
                    PicPurpose::SelectCharacter => self.send_character_pic(character_id, &pic),
                    PicPurpose::DeleteCharacter => self.send_delete_character(character_id, &pic),
                }
            }
            PicEntryStep::Notice => unreachable!("handled before PIC validation"),
        }
    }

    pub(super) fn send_delete_character(&mut self, character_id: i32, pic: &str) {
        let packet = match openstory_protocol::delete_character(character_id, pic) {
            Ok(packet) => packet,
            Err(error) => {
                eprintln!("OpenStory: could not create character deletion packet: {error}");
                if let Some(entry) = self.pic_entry.as_mut() {
                    entry.notice = Some("Could not create deletion request".to_owned());
                }
                self.refresh_pic_dialog_texture();
                return;
            }
        };
        let Some(session) = self.session.as_ref() else {
            eprintln!("OpenStory: connect to a server before deleting a character");
            return;
        };
        match session.send_packet(packet) {
            Ok(()) => {
                self.pic_entry = None;
                self.pic_dialog_texture = None;
            }
            Err(error) => {
                eprintln!("OpenStory: could not send character deletion request: {error}");
                if let Some(entry) = self.pic_entry.as_mut() {
                    entry.notice = Some("Could not send deletion request; try again".to_owned());
                }
                self.refresh_pic_dialog_texture();
            }
        }
    }

    pub(super) fn send_pic_registration(&mut self, character_id: i32, pic: &str) {
        let (mac_addresses, hardware_id) = character_selection_hardware_info();
        let packet = openstory_protocol::register_character_pic(
            character_id,
            pic,
            &mac_addresses,
            &hardware_id,
        );
        self.send_pic_packet(packet, character_id);
    }

    pub(super) fn send_character_pic(&mut self, character_id: i32, pic: &str) {
        let (mac_addresses, hardware_id) = character_selection_hardware_info();
        let packet = openstory_protocol::select_character_with_pic(
            pic,
            character_id,
            &mac_addresses,
            &hardware_id,
        );
        self.send_pic_packet(packet, character_id);
    }

    pub(super) fn send_pic_packet(
        &mut self,
        packet: Result<Vec<u8>, openstory_protocol::PacketError>,
        character_id: i32,
    ) {
        let packet = match packet {
            Ok(packet) => packet,
            Err(error) => {
                eprintln!("OpenStory: could not create PIC packet: {error}");
                if let Some(entry) = self.pic_entry.as_mut() {
                    entry.notice = Some("Could not create PIC packet".to_owned());
                }
                self.refresh_pic_dialog_texture();
                return;
            }
        };
        let Some(session) = self.session.as_ref() else {
            eprintln!("OpenStory: connect to a server before selecting a character");
            return;
        };
        match session.send_packet(packet) {
            Ok(()) => {
                self.active_character = self.character_list.as_ref().and_then(|list| {
                    list.characters
                        .iter()
                        .find(|character| character.id == character_id)
                        .cloned()
                });
                self.pic_entry = None;
                self.pic_dialog_texture = None;
            }
            Err(error) => {
                eprintln!("OpenStory: could not send PIC packet: {error}");
                if let Some(entry) = self.pic_entry.as_mut() {
                    entry.notice = Some("Could not send PIC; try again".to_owned());
                }
                self.refresh_pic_dialog_texture();
            }
        }
    }

    pub(super) fn activate_login(&mut self, action: LoginAction) {
        if matches!(action, LoginAction::Login) {
            self.persist_login_preferences();
        }
        let packet = match action {
            LoginAction::Login => match self.login_packet() {
                Ok(packet) => packet,
                Err(error) => {
                    eprintln!("OpenStory: {error}");
                    return;
                }
            },
            LoginAction::Guest => {
                match configured_machine_id().and_then(|machine_id| {
                    openstory_protocol::login_request("guest", "guest", machine_id)
                        .map_err(|error| error.to_string())
                }) {
                    Ok(packet) => packet,
                    Err(error) => {
                        eprintln!("OpenStory: could not create guest login packet: {error}");
                        return;
                    }
                }
            }
            LoginAction::Quit => return,
            LoginAction::ToggleSaveId => return,
            LoginAction::SelectWorld(_)
            | LoginAction::SelectChannel(_)
            | LoginAction::EnterWorld
            | LoginAction::ReturnToLogin
            | LoginAction::HighlightCharacter(_)
            | LoginAction::SelectCharacter(_)
            | LoginAction::DeleteCharacter(_)
            | LoginAction::ReturnToWorld
            | LoginAction::PreviousCharacterPage
            | LoginAction::NextCharacterPage
            | LoginAction::BeginCharacterCreation
            | LoginAction::CycleCreationOption(_, _)
            | LoginAction::ToggleCreationGender(_)
            | LoginAction::CheckCreationName
            | LoginAction::FocusCreationName
            | LoginAction::SubmitCharacterCreation
            | LoginAction::CancelCharacterCreation => return,
        };
        if !self.login_server_connected || self.session.is_none() {
            if self.session.is_none() {
                eprintln!(
                    "OpenStory: reconnecting to the login server at {}",
                    self.server_address
                );
                self.session = Some(session::connect(self.server_address.clone()));
            }
            self.pending_login_packet = Some(packet);
            eprintln!("OpenStory: login request queued until the server connection is ready");
            return;
        }
        match self
            .session
            .as_ref()
            .map(|session| session.send_packet(packet.clone()))
        {
            Some(Ok(())) => {}
            Some(Err(error)) => {
                eprintln!(
                    "OpenStory: could not send login request ({error}); reconnecting to {}",
                    self.server_address
                );
                self.login_server_connected = false;
                self.pending_login_packet = Some(packet);
                self.session = Some(session::connect(self.server_address.clone()));
            }
            None => {
                self.pending_login_packet = Some(packet);
                self.login_server_connected = false;
                self.session = Some(session::connect(self.server_address.clone()));
            }
        }
    }

    pub(super) fn login_packet(&self) -> Result<Vec<u8>, String> {
        if self.account_text.is_empty() {
            return Err("enter an account name before logging in".to_owned());
        }
        if self.account_text.chars().count() > 12 || self.password_text.chars().count() > 12 {
            return Err("account names and passwords are limited to 12 characters".to_owned());
        }
        if self.password_text.chars().count() <= 4 {
            return Err("the password must contain at least five characters".to_owned());
        }
        let machine_id = configured_machine_id()?;
        openstory_protocol::login_request(&self.account_text, &self.password_text, machine_id)
            .map_err(|error| error.to_string())
    }

    pub(super) fn refresh_save_id_indicator(&mut self) {
        let Some((unchecked_index, unchecked_rect, checked_index, checked_rect)) =
            self.save_id_indicator_draws
        else {
            return;
        };
        let hidden_unchecked = SpriteRect {
            width: 0.0,
            height: 0.0,
            ..unchecked_rect
        };
        let hidden_checked = SpriteRect {
            width: 0.0,
            height: 0.0,
            ..checked_rect
        };
        if let Some((_, rect)) = self.sprite_draws.get_mut(unchecked_index) {
            *rect = if self.save_account_id {
                hidden_unchecked
            } else {
                unchecked_rect
            };
        }
        if let Some((_, rect)) = self.sprite_draws.get_mut(checked_index) {
            *rect = if self.save_account_id {
                checked_rect
            } else {
                hidden_checked
            };
        }
    }

    pub(super) fn refresh_login_button_visuals(&mut self) -> bool {
        let hovered = self.cursor.and_then(|cursor| {
            self.login_buttons
                .iter()
                .position(|button| rect_contains(button.hit_rect, cursor))
        });
        self.hovered_login_button = hovered;
        let mut active_draw_indices = HashSet::with_capacity(self.login_buttons.len());
        for (index, button) in self.login_buttons.iter().enumerate() {
            let draw_index = if !self.login_button_enabled(index) {
                button.disabled_draw.unwrap_or(button.normal_draw)
            } else if self.pressed_login_button == Some(index) && hovered == Some(index) {
                button
                    .pressed_draw
                    .or(button.mouse_over_draw)
                    .unwrap_or(button.normal_draw)
            } else if hovered == Some(index) {
                button.mouse_over_draw.unwrap_or(button.normal_draw)
            } else if self.focused_login_button == Some(index) {
                button
                    .key_focused_draw
                    .or(button.mouse_over_draw)
                    .unwrap_or(button.normal_draw)
            } else if matches!(button.action, Some(LoginAction::Login | LoginAction::Guest)) {
                button.mouse_over_draw.unwrap_or(button.normal_draw)
            } else {
                button.normal_draw
            };
            active_draw_indices.insert(draw_index);
        }
        let changed = active_draw_indices != self.active_login_button_draw_indices;
        self.active_login_button_draw_indices = active_draw_indices;
        changed
    }

    pub(super) fn login_button_action(&self, index: usize) -> Option<LoginAction> {
        self.login_buttons.get(index)?.action
    }

    pub(super) fn login_action_enabled(&self, action: LoginAction) -> bool {
        match action {
            // Keep Login actionable even when the form needs correction. The
            // submit path performs the credential checks and reports them;
            // disabling the legacy button here leaves users with no feedback.
            LoginAction::Login
            | LoginAction::Guest
            | LoginAction::Quit
            | LoginAction::ToggleSaveId => true,
            _ => true,
        }
    }

    pub(super) fn login_button_enabled(&self, index: usize) -> bool {
        self.login_button_action(index)
            .is_some_and(|action| self.login_action_enabled(action))
    }

    pub(super) fn set_login_button_focus(&mut self, focused: Option<usize>) {
        if self.focused_login_button == focused {
            return;
        }
        self.focused_login_button = focused;
        if focused.is_none() {
            self.reset_login_caret();
        }
        self.refresh_field_texture(self.focused_field);
        self.refresh_login_button_visuals();
    }

    pub(super) fn move_login_button_focus(&mut self, direction: isize) {
        let candidates = (0..self.login_buttons.len())
            .filter(|index| self.login_button_enabled(*index))
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            self.set_login_button_focus(None);
            return;
        }
        let current = self
            .focused_login_button
            .and_then(|focused| candidates.iter().position(|index| *index == focused));
        let next = match current {
            Some(current) => current
                .saturating_add_signed(direction)
                .rem_euclid(candidates.len()),
            None if direction < 0 => candidates.len() - 1,
            None => 0,
        };
        self.set_login_button_focus(Some(candidates[next]));
    }

    pub(super) fn advance_login_focus(&mut self) {
        if self.focused_login_button.is_some() {
            let candidates = (0..self.login_buttons.len())
                .filter(|index| self.login_button_enabled(*index))
                .collect::<Vec<_>>();
            let current = self
                .focused_login_button
                .and_then(|focused| candidates.iter().position(|index| *index == focused));
            if current.is_some_and(|current| current + 1 == candidates.len()) {
                self.focused_field = LoginField::Account;
                self.set_login_button_focus(None);
                self.refresh_field_texture(LoginField::Password);
                return;
            }
            self.move_login_button_focus(1);
            return;
        }
        match self.focused_field {
            LoginField::Account => {
                self.focused_field = LoginField::Password;
                self.reset_login_caret();
                self.refresh_field_texture(LoginField::Account);
                self.refresh_field_texture(LoginField::Password);
            }
            LoginField::Password => self.move_login_button_focus(1),
        }
    }

    pub(super) fn activate_focused_login_button(&mut self, event_loop: &ActiveEventLoop) {
        let Some(index) = self.focused_login_button else {
            return;
        };
        let Some(action) = self.login_button_action(index) else {
            return;
        };
        if !self.login_action_enabled(action) {
            return;
        }
        self.audio
            .play_ui_sound(&self.sound_archive, "UI.img/BtMouseClick");
        match action {
            LoginAction::Quit => event_loop.exit(),
            LoginAction::ToggleSaveId => self.toggle_save_account_id(),
            action => self.activate_login(action),
        }
    }

    pub(super) fn persist_login_preferences(&self) {
        let preferences = LoginPreferences {
            save_account_id: self.save_account_id,
            account_id: self.account_text.clone(),
        };
        if let Err(error) = preferences.save() {
            eprintln!("OpenStoryRS: could not save login preference: {error}");
        }
    }

    pub(super) fn toggle_save_account_id(&mut self) {
        self.save_account_id = !self.save_account_id;
        self.persist_login_preferences();
        self.refresh_save_id_indicator();
    }

    pub(super) fn refresh_field_texture(&mut self, field: LoginField) {
        let (index, text) = match field {
            LoginField::Account => (0, self.account_text.as_str()),
            LoginField::Password => (1, self.password_text.as_str()),
        };
        let text = if matches!(field, LoginField::Password) {
            "*".repeat(text.chars().count())
        } else {
            text.to_owned()
        };
        let pixels = rasterize_field(
            &self.font,
            &text,
            self.login_caret_visible
                && self.show_login_fields
                && self.focused_login_button.is_none()
                && self.focused_field == field,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        match renderer.upload_bgra(FIELD_WIDTH, FIELD_HEIGHT, &pixels) {
            Ok(texture) => {
                if let Some(slot) = self.field_textures.get_mut(index) {
                    *slot = texture;
                }
            }
            Err(error) => eprintln!("OpenStory: could not update login text: {error}"),
        }
    }

    pub(super) fn reset_login_caret(&mut self) {
        self.login_caret_visible = true;
        self.next_login_caret_toggle = Instant::now() + LOGIN_CARET_BLINK_INTERVAL;
    }

    pub(super) fn insert_text(&mut self, input: &str) {
        if self.focused_login_button.take().is_some() {
            self.refresh_field_texture(self.focused_field);
            self.refresh_login_button_visuals();
        }
        let field = self.focused_field;
        let target = match field {
            LoginField::Account => &mut self.account_text,
            LoginField::Password => &mut self.password_text,
        };
        for character in input.chars().filter(|character| !character.is_control()) {
            if target.chars().count() >= 12 {
                break;
            }
            target.push(character);
        }
        self.reset_login_caret();
        self.refresh_field_texture(field);
        self.refresh_login_button_visuals();
    }

    pub(super) fn backspace(&mut self) {
        if self.focused_login_button.take().is_some() {
            self.refresh_field_texture(self.focused_field);
            self.refresh_login_button_visuals();
        }
        let field = self.focused_field;
        let target = match field {
            LoginField::Account => &mut self.account_text,
            LoginField::Password => &mut self.password_text,
        };
        target.pop();
        self.reset_login_caret();
        self.refresh_field_texture(field);
        self.refresh_login_button_visuals();
    }

    pub(super) fn focus_at(&mut self, point: Point) -> bool {
        let account = SpriteRect {
            x: FIELD_X,
            y: ACCOUNT_Y,
            width: FIELD_WIDTH as f32,
            height: 24.0,
            blend_mode: BlendMode::Alpha,
        };
        let password = SpriteRect {
            x: FIELD_X,
            y: PASSWORD_Y,
            width: FIELD_WIDTH as f32,
            height: 24.0,
            blend_mode: BlendMode::Alpha,
        };
        let previous = self.focused_field;
        let selected = if rect_contains(account, point) {
            Some(LoginField::Account)
        } else if rect_contains(password, point) {
            Some(LoginField::Password)
        } else {
            None
        };
        let Some(selected) = selected else {
            return false;
        };
        self.focused_field = selected;
        self.focused_login_button = None;
        self.reset_login_caret();
        self.refresh_field_texture(previous);
        self.refresh_field_texture(selected);
        self.refresh_login_button_visuals();
        true
    }

    pub(super) fn process_network_events(&mut self) {
        if self.map_load_receiver.is_some()
            || self.pending_map_scene_upload.is_some()
            || self.pending_map_transition_ack.is_some()
            || (self.map_loading_started_at.is_some()
                && self.map_initial_npc_loading_until.is_none())
        {
            return;
        }
        loop {
            let event = self.session.as_ref().and_then(|session| session.try_recv());
            let Some(event) = event else {
                break;
            };
            match event {
                openstory_protocol::SessionEvent::Connected { redirected } => {
                    if redirected {
                        self.channel_connected = true;
                        eprintln!("OpenStory: connected to the character's channel server");
                        continue;
                    }
                    self.login_server_connected = true;
                    self.channel_connected = false;
                    self.refresh_login_button_visuals();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    eprintln!("OpenStory: connected; starting the login session");
                    if let Some(session) = self.session.as_ref() {
                        if let Err(error) = session.send_packet(openstory_protocol::login_start()) {
                            eprintln!("OpenStory: could not start login session: {error}");
                        }
                    }
                    if let Some(packet) = self.pending_login_packet.take()
                        && let Some(session) = self.session.as_ref()
                        && let Err(error) = session.send_packet(packet)
                    {
                        eprintln!("OpenStory: could not send queued login request: {error}");
                    }
                    if std::env::var("OPENSTORY_AUTO_LOGIN").as_deref() == Ok("1") {
                        if let Some(session) = self.session.as_ref() {
                            if let Err(error) = configured_login_packet()
                                .and_then(|packet| session.send_packet(packet))
                            {
                                eprintln!("OpenStory: could not send account login: {error}");
                            }
                        }
                    }
                }
                openstory_protocol::SessionEvent::Packet(packet) => {
                    self.handle_server_packet(&packet);
                }
                openstory_protocol::SessionEvent::Disconnected(message) => {
                    self.pending_login_packet = None;
                    self.session = None;
                    self.login_server_connected = false;
                    self.channel_connected = false;
                    self.server_ui_disabled = false;
                    self.rps_game = None;
                    let fame_request_was_pending = self
                        .character_info
                        .as_mut()
                        .is_some_and(|ui| std::mem::take(&mut ui.fame_pending));
                    if fame_request_was_pending {
                        self.push_chat_line(
                            "[Fame] The connection closed before the server replied.".to_owned(),
                            [255, 105, 105],
                        );
                        self.refresh_character_info_texture();
                    }
                    if self.character_list.is_some()
                        && self.character_selection_status.is_some()
                        && self.current_field.is_none()
                    {
                        self.active_character = None;
                        self.taming_mount_stats = None;
                        self.character_selection_status =
                            Some("Connection closed. Click Start to retry.".to_owned());
                        self.refresh_character_scene();
                    }
                    self.refresh_login_button_visuals();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    self.pending_merchant_permit = None;
                    eprintln!("OpenStory: server connection closed: {message}");
                }
            }
            // During the initial NPC quiet window, return after one packet so
            // the loading spinner gets a frame between synchronous NPC loads.
            if self.map_load_receiver.is_some()
                || self.pending_map_scene_upload.is_some()
                || self.map_loading_started_at.is_some()
            {
                break;
            }
        }
    }
}
