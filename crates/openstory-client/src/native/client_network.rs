use super::*;

impl ClientApp {
    pub(super) fn respond_to_server_ping(&mut self) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        if let Err(error) = session.send_packet(openstory_protocol::pong()) {
            eprintln!("OpenStoryRS: could not respond to server ping: {error}");
        }
    }

    pub(super) fn send_rps_selection(&mut self, selection: u8) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        if !self.channel_connected {
            return;
        }
        if let Err(error) = session.send_packet(openstory_protocol::rps_selection_packet(selection))
        {
            eprintln!("OpenStoryRS: could not send RPS selection: {error}");
            self.channel_connected = false;
        }
    }

    pub(super) fn add_remote_character(
        &mut self,
        character: openstory_protocol::RemoteCharacterSpawn,
    ) {
        let pet_spawns = character.pets;
        let item_effect_id = character.item_effect_id;
        let mount_item_id = character.riding_mount;
        let guild_name = character.guild_name;
        let guild_mark = character.guild_mark;
        if self
            .active_character
            .as_ref()
            .is_some_and(|active| active.id == character.id)
        {
            self.set_character_item_effect(character.id, item_effect_id);
            for pet in pet_spawns {
                self.spawn_pet(pet);
            }
            return;
        }
        let (avatar_frames, name_label) = self
            .load_remote_avatar(&character.look, &character.name)
            .unwrap_or_else(|error| {
                eprintln!("OpenStory: could not load remote character appearance: {error}");
                (Vec::new(), None)
            });
        let guild_label = self
            .load_remote_guild_label(&guild_name)
            .unwrap_or_else(|error| {
                eprintln!("OpenStoryRS: could not load remote guild label: {error}");
                None
            });
        let item_aura = match self.load_persistent_item_aura(item_effect_id) {
            Ok(aura) => aura,
            Err(error) => {
                eprintln!(
                    "OpenStory: could not load character item aura {item_effect_id}: {error}"
                );
                None
            }
        };
        let mount = match self.load_remote_mount(mount_item_id) {
            Ok(mount) => mount,
            Err(error) => {
                eprintln!("OpenStory: could not load remote mount {mount_item_id}: {error}");
                None
            }
        };
        let death_art = self.load_character_death_art(&character.look);
        let desired_avatar_motion = avatar_motion_for_stance(character.stance);
        let active_avatar_frame = first_avatar_frame(&avatar_frames, desired_avatar_motion);
        let Some(scene) = self.map_scene.as_mut() else {
            return;
        };
        let id = character.id;
        let x = f64::from(character.x);
        let y = f64::from(character.y);
        let dead = character.stance & !1 == 18;
        let mut remote_player = RemotePlayerState {
            name: character.name,
            guild_name,
            _guild_mark: guild_mark,
            level: character.level,
            job: character.job,
            look: character.look,
            x,
            y,
            recoil_offset_x: 0.0,
            recoil_velocity_x: 0.0,
            target_x: x,
            target_y: y,
            previous_x: x,
            previous_y: y,
            foothold: 0,
            stance: character.stance,
            dead,
            death_art,
            facing_left: character.stance & 1 != 0,
            hidden: false,
            mount,
            attack_speed: 6,
            attack_stance: None,
            speech_bubble_remaining: None,
            chalkboard: None,
            invincibility_remaining: Duration::ZERO,
            mini_room: character.mini_room,
            face_expression: None,
            item_aura,
            avatar_frames,
            active_avatar_frame,
            desired_avatar_motion,
            avatar_elapsed: Duration::ZERO,
            name_label,
            guild_label,
        };
        if dead {
            remote_player.snap_to_ground_if_needed(scene.footholds.as_ref());
        }
        scene.remote_players.insert(id, remote_player);
        if std::env::var("OPENSTORY_DEBUG_REMOTE").as_deref() == Ok("1") {
            if let Some(summary) = scene.remote_summary(id) {
                eprintln!("OpenStory: spawned remote character {summary}");
            }
        }
        for pet in pet_spawns {
            self.spawn_pet(pet);
        }
    }

    pub(super) fn load_character_death_art(
        &mut self,
        look: &openstory_protocol::CharacterLook,
    ) -> Option<LocalDeathArtRuntime> {
        let mut sprites = Vec::new();
        let assets = match append_local_death_art(
            &mut sprites,
            &self.effect_archive,
            &self.character_archive,
            look,
            &mut self.remote_art_cache,
        ) {
            Ok(Some(assets)) => assets,
            Ok(None) => return None,
            Err(error) => {
                eprintln!("OpenStory: could not load death art: {error}");
                return None;
            }
        };
        match self.upload_dynamic_sprites(sprites) {
            Ok(draws) => local_death_art_runtime(assets, &draws),
            Err(error) => {
                eprintln!("OpenStory: could not upload death art: {error}");
                None
            }
        }
    }

    pub(super) fn load_remote_guild_label(
        &mut self,
        guild_name: &str,
    ) -> Result<Option<(usize, SpriteRect)>, String> {
        if guild_name.is_empty() {
            return Ok(None);
        }
        let mut sprites = Vec::with_capacity(1);
        append_text_sprite(
            &mut sprites,
            &self.font,
            guild_name,
            centered_text_x(&self.font, guild_name, 11.0),
            GAMEPLAY_CAMERA_CENTER_Y - 76.0,
            11.0,
        );
        Ok(self.upload_dynamic_sprites(sprites)?.first().copied())
    }

    pub(super) fn set_character_item_effect(&mut self, character_id: i32, item_id: i32) {
        let item_aura = match self.load_persistent_item_aura(item_id) {
            Ok(aura) => aura,
            Err(error) => {
                eprintln!("OpenStory: could not load character item aura {item_id}: {error}");
                None
            }
        };
        let Some(scene) = self.map_scene.as_mut() else {
            return;
        };
        if self
            .active_character
            .as_ref()
            .is_some_and(|character| character.id == character_id)
        {
            scene.local_item_aura = item_aura;
        } else if let Some(player) = scene.remote_players.get_mut(&character_id) {
            player.item_aura = item_aura;
        }
    }

    pub(super) fn load_remote_avatar(
        &mut self,
        look: &openstory_protocol::CharacterLook,
        name: &str,
    ) -> Result<(Vec<AvatarFrameRuntime>, Option<(usize, SpriteRect)>), String> {
        let mut sprites = Vec::new();
        let assets = append_character_animations(
            &mut sprites,
            &self.character_archive,
            look,
            512.0,
            GAMEPLAY_CAMERA_CENTER_Y,
            &mut self.remote_art_cache,
        )?;
        let name_label_index = sprites.len();
        append_text_sprite(
            &mut sprites,
            &self.font,
            name,
            centered_text_x(&self.font, name, 15.0),
            GAMEPLAY_CAMERA_CENTER_Y - 92.0,
            15.0,
        );
        let draws = self.upload_dynamic_sprites(sprites)?;
        Ok((
            uploaded_avatar_frames_runtime(assets, &draws),
            draws.get(name_label_index).copied(),
        ))
    }

    pub(super) fn load_remote_mount(
        &mut self,
        item_id: i32,
    ) -> Result<Option<MountLookRuntime>, String> {
        if item_id <= 0 {
            return Ok(None);
        }
        let item_name = format!("{item_id:08}.img");
        let root_path = format!("TamingMob/{item_name}");
        let Some(mount) = self
            .character_archive
            .root()
            .resolve(&root_path)
            .map_err(|error| error.to_string())?
        else {
            return Ok(Some(MountLookRuntime {
                animations: HashMap::new(),
            }));
        };

        let mut sprites = Vec::new();
        let mut animation_assets: HashMap<MountMotion, Vec<MountFrameAsset>> = HashMap::new();
        for (motion, primary, fallback) in [
            (MountMotion::Stand, "stand1", Some("stand")),
            (MountMotion::Walk, "walk1", Some("walk")),
            (MountMotion::Jump, "jump", None),
            (MountMotion::Rope, "rope", None),
            (MountMotion::Ladder, "ladder", None),
            (MountMotion::Prone, "prone", None),
            (MountMotion::Fly, "fly", None),
        ] {
            let source =
                mount
                    .child(primary)
                    .map_err(|error| error.to_string())?
                    .or(match fallback {
                        Some(fallback) => {
                            mount.child(fallback).map_err(|error| error.to_string())?
                        }
                        None => None,
                    });
            let Some(source) = source else {
                continue;
            };
            let mut frames = Vec::new();
            for frame in numeric_children(&source)? {
                let frame_name = frame.name().map_err(|error| error.to_string())?;
                let first_sprite = sprites.len();
                append_map_art_sprite(
                    &mut sprites,
                    &frame,
                    &format!("mount:{item_id}/{primary}/{frame_name}"),
                    512.0,
                    GAMEPLAY_CAMERA_CENTER_Y,
                    255,
                    false,
                    &mut self.remote_art_cache,
                )?;
                if sprites.len() == first_sprite {
                    continue;
                }
                let navel = vector_path(&frame, "0/map/navel")
                    .or_else(|| vector_path(&frame, "map/navel"))
                    .unwrap_or((0.0, -30.0));
                frames.push(MountFrameAsset {
                    delay_ms: integer_property(&frame, "delay")
                        .and_then(|delay| u16::try_from(delay).ok())
                        .filter(|delay| *delay > 0)
                        .unwrap_or(100),
                    sprites: first_sprite..sprites.len(),
                    navel,
                });
            }
            if !frames.is_empty() {
                animation_assets.insert(motion, frames);
            }
        }
        if animation_assets.is_empty() {
            return Ok(Some(MountLookRuntime {
                animations: HashMap::new(),
            }));
        }
        let draws = self.upload_dynamic_sprites(sprites)?;
        let animations = animation_assets
            .into_iter()
            .map(|(motion, frames)| {
                let frames = frames
                    .into_iter()
                    .map(|frame| MountFrameRuntime {
                        delay: Duration::from_millis(u64::from(frame.delay_ms.max(1))),
                        sprites: frame
                            .sprites
                            .filter_map(|draw_index| draws.get(draw_index).copied())
                            .collect(),
                        navel: frame.navel,
                    })
                    .collect::<Vec<_>>();
                (
                    motion,
                    MountAnimationRuntime {
                        frames,
                        frame_index: 0,
                        elapsed: Duration::ZERO,
                    },
                )
            })
            .collect();
        Ok(Some(MountLookRuntime { animations }))
    }

    pub(super) fn refresh_local_mount(&mut self, look: &openstory_protocol::CharacterLook) {
        if self.map_scene.is_none() {
            return;
        }
        let riding = self
            .active_buffs
            .get(&openstory_protocol::BuffStatKey {
                group: openstory_protocol::BuffMaskGroup::First,
                flag: STATUS_MONSTER_RIDING,
            })
            .is_some_and(|buff| buff.value > 0);
        let item_id = if riding { equipped_mount_id(look) } else { 0 };
        let mount = match self.load_remote_mount(item_id) {
            Ok(mount) => mount,
            Err(error) => {
                eprintln!("OpenStory: could not load local mount {item_id}: {error}");
                None
            }
        };
        if let Some(scene) = self.map_scene.as_mut() {
            scene.local_mount = mount;
        }
    }

    pub(super) fn load_face_expression(
        &mut self,
        look: &openstory_protocol::CharacterLook,
        action: i32,
    ) -> Result<Option<FaceExpressionRuntime>, String> {
        let Some(expression_index) = face_expression_index(action) else {
            return Ok(None);
        };
        let cache_key = (look.face_id, expression_index);
        if let Some(frames) = self.face_expression_cache.get(&cache_key) {
            return Ok(frames.clone().map(|frames| FaceExpressionRuntime {
                frames,
                frame_index: 0,
                elapsed: Duration::ZERO,
            }));
        }

        let expression_name = FACE_EXPRESSION_NAMES[usize::from(expression_index)];
        let expression_path = format!("Face/000{:05}.img/{expression_name}", look.face_id);
        let expression_node = self
            .character_archive
            .root()
            .resolve(&expression_path)
            .map_err(|error| error.to_string())?;
        let mut sprites = Vec::new();
        let mut assets = Vec::new();
        if let Some(expression_node) = expression_node {
            for frame_node in numeric_children(&expression_node)? {
                let Some(face_node) = frame_node
                    .child("face")
                    .map_err(|error| error.to_string())?
                else {
                    continue;
                };
                let frame_name = frame_node.name().map_err(|error| error.to_string())?;
                let first_sprite = sprites.len();
                append_map_art_sprite(
                    &mut sprites,
                    &face_node,
                    &format!("character:{expression_path}/{frame_name}/face"),
                    512.0,
                    GAMEPLAY_CAMERA_CENTER_Y,
                    255,
                    false,
                    &mut self.remote_art_cache,
                )?;
                if sprites.len() == first_sprite {
                    continue;
                }
                set_sprite_depth(&mut sprites, first_sprite, 19);
                let delay = integer_property(&frame_node, "delay")
                    .and_then(|delay| u16::try_from(delay).ok())
                    .filter(|delay| *delay > 0)
                    .unwrap_or(100);
                assets.push((delay, first_sprite..sprites.len()));
            }
        }
        let draws = self.upload_dynamic_sprites(sprites)?;
        let frames = assets
            .into_iter()
            .filter_map(|(delay_ms, sprites)| {
                let sprites = sprites
                    .filter_map(|index| draws.get(index).copied())
                    .collect::<Vec<_>>();
                (!sprites.is_empty()).then_some(FaceExpressionFrameRuntime {
                    delay: Duration::from_millis(u64::from(delay_ms.max(1))),
                    sprites,
                })
            })
            .collect::<Vec<_>>();
        let frames = (!frames.is_empty()).then_some(frames);
        self.face_expression_cache.insert(cache_key, frames.clone());
        Ok(frames.map(|frames| FaceExpressionRuntime {
            frames,
            frame_index: 0,
            elapsed: Duration::ZERO,
        }))
    }

    pub(super) fn show_remote_face_expression(
        &mut self,
        update: openstory_protocol::RemoteFacialExpression,
    ) {
        let Some(look) = self
            .map_scene
            .as_ref()
            .and_then(|scene| scene.remote_players.get(&update.character_id))
            .map(|player| player.look.clone())
        else {
            return;
        };
        let expression = match self.load_face_expression(&look, update.expression) {
            Ok(expression) => expression,
            Err(error) => {
                eprintln!("OpenStory: could not load facial expression: {error}");
                return;
            }
        };
        if let Some(player) = self
            .map_scene
            .as_mut()
            .and_then(|scene| scene.remote_players.get_mut(&update.character_id))
        {
            player.face_expression = expression;
        }
    }

    pub(super) fn show_local_face_expression(&mut self, action: i32) {
        let Some(look) = self
            .active_character
            .as_ref()
            .map(|character| character.look.clone())
        else {
            return;
        };
        match self.load_face_expression(&look, action) {
            Ok(expression) => {
                if let Some(scene) = self.map_scene.as_mut() {
                    scene.local_face_expression = expression;
                }
            }
            Err(error) => {
                eprintln!("OpenStory: could not load local facial expression: {error}")
            }
        }
    }

    pub(super) fn send_face_expression(&mut self, action: i32) {
        let now = Instant::now();
        if self
            .last_face_expression_at
            .is_some_and(|last| now.saturating_duration_since(last) < Duration::from_millis(1500))
        {
            return;
        }
        let Some(session) = self.session.as_ref() else {
            return;
        };
        if !self.channel_connected {
            return;
        }
        if let Err(error) = session.send_packet(openstory_protocol::face_expression(action)) {
            eprintln!("OpenStory: could not send facial expression: {error}");
            self.channel_connected = false;
            return;
        }
        self.last_face_expression_at = Some(now);
        self.show_local_face_expression(action);
    }

    pub(super) fn upload_dynamic_sprites(
        &mut self,
        sprites: Vec<DecodedSprite>,
    ) -> Result<Vec<(usize, SpriteRect)>, String> {
        let mut draws = Vec::with_capacity(sprites.len());
        for sprite in sprites {
            let texture_index = if let Some(index) = self.texture_indices.get(&sprite.texture_key) {
                *index
            } else {
                let renderer = self
                    .renderer
                    .as_ref()
                    .ok_or_else(|| "graphics are not ready for remote art".to_owned())?;
                let texture = renderer
                    .upload_bgra(sprite.texture_width, sprite.texture_height, &sprite.pixels)
                    .map_err(|error| error.to_string())?;
                let index = self.sprite_textures.len();
                self.sprite_textures.push(texture);
                self.texture_indices
                    .insert(sprite.texture_key.clone(), index);
                index
            };
            draws.push((texture_index, sprite.rect));
        }
        Ok(draws)
    }

    pub(super) fn ensure_maple_tv_assets(&mut self) {
        if self.maple_tv_assets_loaded || self.renderer.is_none() {
            return;
        }
        self.maple_tv_assets_loaded = true;

        let idle = self.load_maple_tv_bitmap("MapleTV.img/TVbasic/0");
        match idle {
            Ok((texture, pixels, _)) => {
                self.maple_tv_idle_texture = Some(texture);
                self.maple_tv_base_pixels = Some(pixels);
            }
            Err(error) => eprintln!("OpenStory: Maple TV screen art unavailable: {error}"),
        }
        let mut ad_animations = Vec::with_capacity(3);
        for index in 0..3 {
            let path = format!("MapleTV.img/TVmedia/{index}");
            match self.load_maple_tv_animation(&path) {
                Ok(animation) => ad_animations.push(animation),
                Err(error) => {
                    eprintln!("OpenStory: Maple TV ad art unavailable: {error}");
                    ad_animations.push(MapleTvAnimation::empty());
                }
            }
        }
        self.maple_tv_ad_animations = ad_animations;
        self.maple_tv_ad_index = 0;

        for (index, path) in ["MapleTV.img/TVchat1", "MapleTV.img/TVchat2"]
            .into_iter()
            .enumerate()
        {
            match self.load_maple_tv_animation(path) {
                Ok(animation) => self.maple_tv_theme_animations[index] = animation,
                Err(error) => eprintln!("OpenStory: Maple TV theme art unavailable: {error}"),
            }
        }
    }

    pub(super) fn load_maple_tv_bitmap(
        &self,
        path: &str,
    ) -> Result<(GpuTexture, Arc<[u8]>, (i16, i16)), String> {
        let (width, height, pixels, origin) = read_ui_bitmap(&self.ui_archive, path)?;
        let renderer = self
            .renderer
            .as_ref()
            .ok_or_else(|| "graphics are not ready for Maple TV art".to_owned())?;
        let texture = renderer
            .upload_bgra(width, height, &pixels)
            .map_err(|error| error.to_string())?;
        Ok((texture, pixels, origin))
    }

    pub(super) fn load_maple_tv_animation(&self, path: &str) -> Result<MapleTvAnimation, String> {
        const MAX_FRAMES: usize = 128;
        let source = self
            .ui_archive
            .root()
            .resolve(path)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Maple TV animation {path:?} was not found"))?;
        let frame_nodes = numeric_children(&source)?;
        if frame_nodes.len() > MAX_FRAMES {
            return Err(format!(
                "Maple TV animation {path:?} has {} frames (limit {MAX_FRAMES})",
                frame_nodes.len()
            ));
        }
        let mut frames = Vec::with_capacity(frame_nodes.len().max(1));
        if frame_nodes.is_empty() {
            let (texture, _, origin) = self.load_maple_tv_bitmap(path)?;
            frames.push(MapleTvAnimationFrame {
                texture,
                delay: Duration::from_millis(100),
                origin,
            });
        } else {
            for frame in frame_nodes {
                let name = frame.name().map_err(|error| error.to_string())?;
                let frame_path = format!("{path}/{name}");
                let (texture, _, origin) = self.load_maple_tv_bitmap(&frame_path)?;
                let delay = integer_property(&frame, "delay")
                    .and_then(|delay| u64::try_from(delay).ok())
                    .filter(|delay| *delay > 0)
                    .unwrap_or(100);
                frames.push(MapleTvAnimationFrame {
                    texture,
                    delay: Duration::from_millis(delay),
                    origin,
                });
            }
        }
        if frames.is_empty() {
            return Err(format!("Maple TV animation {path:?} has no bitmap frames"));
        }
        Ok(MapleTvAnimation {
            frames,
            frame_index: 0,
            elapsed: Duration::ZERO,
        })
    }

    pub(super) fn advance_maple_tv_animations(&mut self, elapsed: Duration) {
        if let Some(animation) = self.maple_tv_ad_animations.get_mut(self.maple_tv_ad_index)
            && animation.advance_once(elapsed)
        {
            self.maple_tv_ad_index =
                (self.maple_tv_ad_index + 1) % self.maple_tv_ad_animations.len();
        }
        let theme_index =
            self.maple_tv_broadcast
                .as_ref()
                .and_then(|broadcast| match broadcast.type_id % 4 {
                    1 => Some(0),
                    2 => Some(1),
                    _ => None,
                });
        if let Some(index) = theme_index {
            self.maple_tv_theme_animations[index].advance_loop(elapsed);
        }
    }

    pub(super) fn start_maple_tv_broadcast(
        &mut self,
        broadcast: openstory_protocol::MapleTvBroadcast,
    ) {
        self.ensure_maple_tv_assets();
        let mut sprites = Vec::new();
        let mut cache = MapArtCache::default();
        let sender_start = sprites.len();
        if let Err(error) = append_character_preview(
            &mut sprites,
            &self.character_archive,
            &broadcast.sender_look,
            30.0,
            82.0,
            0.65,
            false,
            &mut cache,
        ) {
            eprintln!("OpenStory: could not render Maple TV sender: {error}");
            sprites.truncate(sender_start);
        }
        let sender_end = sprites.len();
        let partner_start = sprites.len();
        if let Some(partner) = broadcast.partner_look.as_ref()
            && let Err(error) = append_character_preview(
                &mut sprites,
                &self.character_archive,
                partner,
                72.0,
                82.0,
                0.65,
                false,
                &mut cache,
            )
        {
            eprintln!("OpenStory: could not render Maple TV partner: {error}");
            sprites.truncate(partner_start);
        }
        let partner_end = sprites.len();
        let avatar_draws = match self.upload_dynamic_sprites(sprites) {
            Ok(draws) => draws,
            Err(error) => {
                eprintln!("OpenStory: could not upload Maple TV avatars: {error}");
                Vec::new()
            }
        };
        let sender_avatar_draws = avatar_draws
            .get(sender_start..sender_end)
            .unwrap_or_default()
            .to_vec();
        let partner_avatar_draws = avatar_draws
            .get(partner_start..partner_end)
            .unwrap_or_default()
            .to_vec();

        let themed = matches!(broadcast.type_id % 4, 1 | 2);
        let color = if themed { [255, 255, 255] } else { [0, 0, 0] };
        let text_left = if broadcast.partner_look.is_some() {
            106
        } else {
            66
        };
        let pixels = rasterize_maple_tv_text(&self.font, &broadcast.lines, text_left, color);
        self.maple_tv_text_texture = self.renderer.as_ref().and_then(|renderer| {
            match renderer.upload_bgra(240, 90, &pixels) {
                Ok(texture) => Some(texture),
                Err(error) => {
                    eprintln!("OpenStory: could not upload Maple TV message: {error}");
                    None
                }
            }
        });
        self.maple_tv_broadcast = Some(MapleTvBroadcastRuntime {
            type_id: broadcast.type_id,
            has_partner: broadcast.partner_look.is_some(),
            expires_at: Instant::now()
                + Duration::from_millis(u64::from(broadcast.remaining_ms.max(1))),
            sender_avatar_draws,
            partner_avatar_draws,
        });
        self.push_chat_line(
            "[Maple TV] A world broadcast is on the air.".to_owned(),
            [255, 222, 112],
        );
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn stop_maple_tv_broadcast(&mut self) {
        if self.maple_tv_broadcast.take().is_none() {
            return;
        }
        self.maple_tv_text_texture = None;
        self.push_chat_line("[Maple TV] Broadcast ended.".to_owned(), [255, 222, 112]);
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn expire_maple_tv_broadcast(&mut self, now: Instant) {
        if self
            .maple_tv_broadcast
            .as_ref()
            .is_some_and(|broadcast| now >= broadcast.expires_at)
        {
            self.stop_maple_tv_broadcast();
        }
    }

    pub(super) fn load_projectile_frames(
        &mut self,
        item_id: i32,
    ) -> Result<Vec<ProjectileFrameRuntime>, String> {
        if let Some(frames) = self.projectile_frames_cache.get(&item_id) {
            return Ok(frames.clone());
        }
        let mut sprites = Vec::new();
        let assets = append_projectile_animation(
            &mut sprites,
            &self.item_archive,
            item_id,
            &mut self.remote_art_cache,
        )?;
        let draws = self.upload_dynamic_sprites(sprites)?;
        let frames = projectile_frames_runtime(assets, &draws);
        if self.projectile_frames_cache.len() >= 64 {
            self.projectile_frames_cache.clear();
        }
        self.projectile_frames_cache.insert(item_id, frames.clone());
        Ok(frames)
    }

    pub(super) fn load_local_chair(&mut self, item_id: i32) -> Result<LocalChairRuntime, String> {
        let mut sprites = Vec::new();
        let assets = append_local_chair(
            &mut sprites,
            &self.item_archive,
            item_id,
            &mut self.remote_art_cache,
        )?;
        let draws = self.upload_dynamic_sprites(sprites)?;
        Ok(LocalChairRuntime {
            behind: ChairAnimationRuntime {
                frames: projectile_frames_runtime(assets.behind, &draws),
                frame_index: 0,
                elapsed: Duration::ZERO,
            },
            front: ChairAnimationRuntime {
                frames: projectile_frames_runtime(assets.front, &draws),
                frame_index: 0,
                elapsed: Duration::ZERO,
            },
            sit_offset_y: assets.sit_offset_y,
        })
    }

    pub(super) fn load_persistent_item_aura(
        &mut self,
        item_id: i32,
    ) -> Result<Option<PersistentItemAuraRuntime>, String> {
        if item_id <= 0 {
            return Ok(None);
        }
        let frames = if let Some(frames) = self.item_aura_frames_cache.get(&item_id) {
            frames.clone()
        } else {
            let mut sprites = Vec::new();
            let assets = append_item_aura_animations(
                &mut sprites,
                &self.effect_archive,
                item_id,
                &mut self.remote_art_cache,
            )?;
            let draws = self.upload_dynamic_sprites(sprites)?;
            let frames = PersistentItemAuraFrames {
                back: projectile_frames_runtime(assets.back, &draws),
                front: projectile_frames_runtime(assets.front, &draws),
                split: assets.split,
            };
            if self.item_aura_frames_cache.len() >= 128 {
                self.item_aura_frames_cache.clear();
            }
            self.item_aura_frames_cache.insert(item_id, frames.clone());
            frames
        };
        Ok(PersistentItemAuraRuntime::new(frames))
    }

    pub(super) fn load_item_use_effect_frames(
        &mut self,
        item_id: i32,
    ) -> Result<Option<Vec<ProjectileFrameRuntime>>, String> {
        if let Some(frames) = self.item_use_effect_cache.get(&item_id) {
            return Ok(frames.clone());
        }
        let path = format!("ItemEff.img/{item_id}");
        let Some(effect) = self
            .effect_archive
            .root()
            .resolve(&path)
            .map_err(|error| error.to_string())?
        else {
            self.item_use_effect_cache.insert(item_id, None);
            return Ok(None);
        };
        let Some(first) = effect.child("0").map_err(|error| error.to_string())? else {
            self.item_use_effect_cache.insert(item_id, None);
            return Ok(None);
        };
        let first_is_bitmap = first.bitmap().map_err(|error| error.to_string())?.is_some();
        let frames_node = if first_is_bitmap { effect } else { first };
        let mut sprites = Vec::new();
        let assets = append_summon_frames_from_node(
            &mut sprites,
            &frames_node,
            &path,
            &mut self.remote_art_cache,
        )?;
        let frames = if assets.is_empty() {
            None
        } else {
            let draws = self.upload_dynamic_sprites(sprites)?;
            let frames = projectile_frames_runtime(assets, &draws);
            (!frames.is_empty()).then_some(frames)
        };
        if self.item_use_effect_cache.len() >= 128 {
            self.item_use_effect_cache.clear();
        }
        self.item_use_effect_cache.insert(item_id, frames.clone());
        Ok(frames)
    }

    pub(super) fn show_local_item_use_effect(&mut self, item_id: i32) {
        let frames = match self.load_item_use_effect_frames(item_id) {
            Ok(Some(frames)) => frames,
            Ok(None) => return,
            Err(error) => {
                eprintln!("OpenStory: could not load item {item_id} use effect: {error}");
                return;
            }
        };
        if let Some(scene) = self.map_scene.as_mut() {
            scene.show_character_skill_effect(
                None,
                vec![CharacterSkillEffectAnimation { z: 0, frames }],
                1.0,
            );
        }
    }

    pub(super) fn load_summon_frames(
        &mut self,
        skill_id: i32,
    ) -> Result<Vec<ProjectileFrameRuntime>, String> {
        if let Some(frames) = self.summon_frames_cache.get(&skill_id) {
            return Ok(frames.clone());
        }
        let mut sprites = Vec::new();
        let assets = append_summon_animation(
            &mut sprites,
            &self.skill_archive,
            skill_id,
            &mut self.remote_art_cache,
        )?;
        let draws = self.upload_dynamic_sprites(sprites)?;
        let frames = projectile_frames_runtime(assets, &draws);
        if self.summon_frames_cache.len() >= 96 {
            self.summon_frames_cache.clear();
        }
        self.summon_frames_cache.insert(skill_id, frames.clone());
        Ok(frames)
    }

    pub(super) fn load_dragon_frames(
        &mut self,
        job: i16,
    ) -> Result<Vec<ProjectileFrameRuntime>, String> {
        if let Some(frames) = self.dragon_frames_cache.get(&job) {
            return Ok(frames.clone());
        }
        let mut sprites = Vec::new();
        let assets = append_dragon_animation(
            &mut sprites,
            &self.mob_archive,
            job,
            &mut self.remote_art_cache,
        )?;
        let draws = self.upload_dynamic_sprites(sprites)?;
        let frames = projectile_frames_runtime(assets, &draws);
        if self.dragon_frames_cache.len() >= 5 {
            self.dragon_frames_cache.clear();
        }
        self.dragon_frames_cache.insert(job, frames.clone());
        Ok(frames)
    }

    pub(super) fn load_pet_animations(
        &mut self,
        item_id: i32,
    ) -> Result<HashMap<u8, Vec<ProjectileFrameRuntime>>, String> {
        if let Some(animations) = self.pet_frames_cache.get(&item_id) {
            return Ok(animations.clone());
        }
        let mut sprites = Vec::new();
        let assets = append_pet_animations(
            &mut sprites,
            &self.item_archive,
            item_id,
            &mut self.remote_art_cache,
        )?;
        let draws = self.upload_dynamic_sprites(sprites)?;
        let animations = assets
            .into_iter()
            .map(|(state, frames)| (state, projectile_frames_runtime(frames, &draws)))
            .filter(|(_, frames)| !frames.is_empty())
            .collect::<HashMap<_, _>>();
        if self.pet_frames_cache.len() >= 64 {
            self.pet_frames_cache.clear();
        }
        self.pet_frames_cache.insert(item_id, animations.clone());
        Ok(animations)
    }

    pub(super) fn load_pet_interaction_frames(
        &mut self,
        item_id: i32,
        feed: bool,
        index: u8,
        success: bool,
    ) -> Result<Option<Vec<ProjectileFrameRuntime>>, String> {
        let key = (item_id, feed, index, success);
        if let Some(frames) = self.pet_interaction_frames_cache.get(&key) {
            return Ok(frames.clone());
        }
        let mut sprites = Vec::new();
        let assets = append_pet_interaction_animation(
            &mut sprites,
            &self.item_archive,
            item_id,
            feed,
            index,
            success,
            &mut self.remote_art_cache,
        )?;
        let frames = if assets.is_empty() {
            None
        } else {
            let draws = self.upload_dynamic_sprites(sprites)?;
            let frames = projectile_frames_runtime(assets, &draws);
            (!frames.is_empty()).then_some(frames)
        };
        if self.pet_interaction_frames_cache.len() >= 128 {
            self.pet_interaction_frames_cache.clear();
        }
        self.pet_interaction_frames_cache
            .insert(key, frames.clone());
        Ok(frames)
    }

    pub(super) fn load_skill_projectile_frames(
        &mut self,
        skill_id: i32,
        skill_level: u8,
    ) -> Result<Option<Vec<ProjectileFrameRuntime>>, String> {
        let key = (skill_id, skill_level);
        if let Some(frames) = self.skill_projectile_cache.get(&key) {
            return Ok(frames.clone());
        }
        let mut sprites = Vec::new();
        let asset = append_skill_projectile_animation(
            &mut sprites,
            &self.skill_archive,
            skill_id,
            skill_level,
            &mut self.remote_art_cache,
        )?;
        let frames = if let Some(assets) = asset {
            let draws = self.upload_dynamic_sprites(sprites)?;
            Some(projectile_frames_runtime(assets, &draws))
        } else {
            None
        };
        if self.skill_projectile_cache.len() >= 128 {
            self.skill_projectile_cache.clear();
        }
        self.skill_projectile_cache.insert(key, frames.clone());
        Ok(frames)
    }

    pub(super) fn load_character_skill_effects(
        &mut self,
        skill_id: i32,
        skill_level: u8,
        character_level: u8,
        two_handed: bool,
    ) -> Result<Vec<CharacterSkillEffectAnimation>, String> {
        let key = (skill_id, skill_level, character_level, two_handed);
        if let Some(animations) = self.skill_effect_cache.get(&key) {
            return Ok(animations.clone());
        }
        let mut sprites = Vec::new();
        let assets = append_skill_use_effects(
            &mut sprites,
            &self.skill_archive,
            skill_id,
            skill_level,
            character_level,
            two_handed,
            &mut self.remote_art_cache,
        )?;
        let draws = self.upload_dynamic_sprites(sprites)?;
        let animations = assets
            .into_iter()
            .map(|asset| CharacterSkillEffectAnimation {
                z: asset.z,
                frames: projectile_frames_runtime(asset.frames, &draws),
            })
            .filter(|animation| !animation.frames.is_empty())
            .collect::<Vec<_>>();
        if self.skill_effect_cache.len() >= 128 {
            self.skill_effect_cache.clear();
        }
        self.skill_effect_cache.insert(key, animations.clone());
        Ok(animations)
    }

    pub(super) fn load_skill_point_effect(
        &mut self,
        skill_id: i32,
    ) -> Result<Option<Vec<ProjectileFrameRuntime>>, String> {
        if let Some(frames) = self.skill_point_effect_cache.get(&skill_id) {
            return Ok(frames.clone());
        }
        let path = format!("{:03}.img/skill/{skill_id}/effect", skill_id / 10_000);
        let source = self
            .skill_archive
            .root()
            .resolve(&path)
            .map_err(|error| error.to_string())?;
        let frames = if let Some(source) = source {
            let mut sprites = Vec::new();
            let assets = append_animation_frames_from_node_at(
                &mut sprites,
                &source,
                &format!("skill-point:{path}"),
                512.0,
                GAMEPLAY_CAMERA_CENTER_Y,
                &mut self.remote_art_cache,
            )?;
            if assets.is_empty() {
                None
            } else {
                let draws = self.upload_dynamic_sprites(sprites)?;
                Some(projectile_frames_runtime(assets, &draws))
            }
        } else {
            None
        };
        if self.skill_point_effect_cache.len() >= 128 {
            self.skill_point_effect_cache.clear();
        }
        self.skill_point_effect_cache
            .insert(skill_id, frames.clone());
        Ok(frames)
    }

    pub(super) fn load_mob_disease_effect(
        &mut self,
        skill_type: u16,
        skill_level: u16,
    ) -> Result<Vec<CharacterSkillEffectAnimation>, String> {
        let key = (skill_type, skill_level);
        if let Some(animations) = self.mob_disease_effect_cache.get(&key) {
            return Ok(animations.clone());
        }
        let path = format!("MobSkill.img/{skill_type}/level/{skill_level}/effect");
        let source = self
            .skill_archive
            .root()
            .resolve(&path)
            .map_err(|error| error.to_string())?;
        let animations = if let Some(source) = source {
            let mut sprites = Vec::new();
            let frames = append_summon_frames_from_node(
                &mut sprites,
                &source,
                &format!("mob-disease:{path}"),
                &mut self.remote_art_cache,
            )?;
            if frames.is_empty() {
                Vec::new()
            } else {
                let draws = self.upload_dynamic_sprites(sprites)?;
                vec![CharacterSkillEffectAnimation {
                    z: 0,
                    frames: projectile_frames_runtime(frames, &draws),
                }]
            }
        } else {
            Vec::new()
        };
        if self.mob_disease_effect_cache.len() >= 64 {
            self.mob_disease_effect_cache.clear();
        }
        self.mob_disease_effect_cache
            .insert(key, animations.clone());
        Ok(animations)
    }

    pub(super) fn load_skill_hit_effects(
        &mut self,
        skill_id: i32,
        skill_level: u8,
        character_level: u8,
        two_handed: bool,
    ) -> Result<Vec<SkillHitEffectAnimation>, String> {
        let key = (skill_id, skill_level, character_level, two_handed);
        if let Some(animations) = self.skill_hit_effect_cache.get(&key) {
            return Ok(animations.clone());
        }
        let mut sprites = Vec::new();
        let assets = append_skill_hit_effects(
            &mut sprites,
            &self.skill_archive,
            skill_id,
            skill_level,
            character_level,
            two_handed,
            &mut self.remote_art_cache,
        )?;
        let draws = self.upload_dynamic_sprites(sprites)?;
        let animations = assets
            .into_iter()
            .map(|asset| SkillHitEffectAnimation {
                pos: asset.pos,
                z: asset.z,
                frames: projectile_frames_runtime(asset.frames, &draws),
            })
            .filter(|animation| !animation.frames.is_empty())
            .collect::<Vec<_>>();
        if self.skill_hit_effect_cache.len() >= 128 {
            self.skill_hit_effect_cache.clear();
        }
        self.skill_hit_effect_cache.insert(key, animations.clone());
        Ok(animations)
    }

    pub(super) fn load_teleport_effect_frames(
        &mut self,
    ) -> Result<Vec<ProjectileFrameRuntime>, String> {
        if let Some(frames) = self.teleport_effect_frames.as_ref() {
            return Ok(frames.clone());
        }
        let mut sprites = Vec::new();
        let assets = self
            .effect_archive
            .root()
            .resolve("BasicEff.img/Teleport")
            .map_err(|error| error.to_string())?
            .map(|effect| {
                append_summon_frames_from_node(
                    &mut sprites,
                    &effect,
                    "effect:basic-teleport",
                    &mut self.remote_art_cache,
                )
            })
            .transpose()?
            .unwrap_or_default();
        let draws = self.upload_dynamic_sprites(sprites)?;
        let frames = projectile_frames_runtime(assets, &draws);
        self.teleport_effect_frames = Some(frames.clone());
        Ok(frames)
    }
}
