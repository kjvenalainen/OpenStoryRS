use super::*;

impl ClientApp {
    pub(super) fn update_boss_hp_gauge(&mut self, mob_id: i32, percentage: u8) {
        if percentage == 0 {
            self.clear_boss_hp_gauge();
            return;
        }
        let name_path = format!("Mob.img/{mob_id}/name");
        let name = self
            .string_archive
            .root()
            .resolve(&name_path)
            .ok()
            .flatten()
            .and_then(|node| match node.value().ok()? {
                NodeValue::String(value) if !value.trim().is_empty() => Some(value),
                _ => None,
            })
            .unwrap_or_else(|| format!("Monster {mob_id}"));
        let panel = match build_boss_hp_gauge_panel(
            &self.legacy_ui_archive,
            &self.font,
            mob_id,
            &name,
            percentage,
        ) {
            Ok(panel) => panel,
            Err(error) => {
                eprintln!("OpenStoryRS: could not render v83 boss HP gauge: {error}");
                self.clear_boss_hp_gauge();
                return;
            }
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        if let Some(texture) = self.boss_hp_gauge_texture.as_ref() {
            if let Err(error) = renderer.update_bgra(texture, &panel.pixels) {
                eprintln!("OpenStoryRS: could not update boss HP gauge: {error}");
                self.clear_boss_hp_gauge();
                return;
            }
        } else {
            match renderer.upload_bgra(panel.width, panel.height, &panel.pixels) {
                Ok(texture) => self.boss_hp_gauge_texture = Some(texture),
                Err(error) => {
                    eprintln!("OpenStoryRS: could not upload boss HP gauge: {error}");
                    self.clear_boss_hp_gauge();
                    return;
                }
            }
        }
        self.boss_hp_gauge_remaining = Duration::from_secs(4);
    }

    pub(super) fn clear_boss_hp_gauge(&mut self) {
        self.boss_hp_gauge_texture = None;
        self.boss_hp_gauge_remaining = Duration::ZERO;
    }

    pub(super) fn enter_death_state(&mut self) {
        if self.death_notice {
            return;
        }
        self.death_notice = true;
        self.death_respawn_pending = false;
        self.gameplay_input = GameplayInput::default();
        self.npc_dialogue = None;
        self.npc_shop = None;
        self.inventory_drag_source = None;
        self.inventory_drop_prompt = None;
        self.inventory_drop_prompt_texture = None;
        if let Some(look) = self
            .active_character
            .as_ref()
            .map(|character| character.look.clone())
        {
            let death_art = self.load_character_death_art(&look);
            if let Some(scene) = self.map_scene.as_mut() {
                scene.local_death_art = death_art;
            }
        }
        if let Some(scene) = self.map_scene.as_mut() {
            if !scene.player.on_ground
                && let Some((foothold_id, ground_y)) = scene
                    .footholds
                    .as_ref()
                    .and_then(|tree| tree.find_below(scene.player.x, scene.player.y))
            {
                scene.player.y = ground_y;
                scene.player.previous_x = scene.player.x;
                scene.player.previous_y = ground_y;
                scene.player.foothold_id = foothold_id;
                if let Some(foothold) = scene
                    .footholds
                    .as_ref()
                    .and_then(|tree| tree.get(foothold_id))
                {
                    scene.player.foothold_layer = foothold.layer;
                    scene.player.foothold_slope = foothold.slope();
                }
                scene.player.ground_below = ground_y;
                scene.player.on_ground = true;
            }
            scene.player.horizontal_speed = 0.0;
            scene.player.vertical_speed = 0.0;
            scene.attack_stance = None;
            scene.desired_avatar_motion = AvatarMotion::Stand;
            scene.active_avatar_frame =
                first_avatar_frame(&scene.avatar_frames, AvatarMotion::Stand);
            scene.avatar_elapsed = Duration::ZERO;
        }
        if let Some(scene) = self.map_scene.as_ref() {
            self.audio.play_world_sound(
                &self.sound_archive,
                "Game.img/Tombstone",
                (rounded_i16(scene.player.x), rounded_i16(scene.player.y)),
            );
        }
        self.refresh_death_notice_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn refresh_death_notice_texture(&mut self) {
        if !self.death_notice {
            self.death_notice_texture = None;
            return;
        }
        let pixels = render_death_notice(&self.font, self.death_respawn_pending);
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.death_notice_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer.upload_bgra(1024, 768, &pixels).map(Some)
        };
        match result {
            Ok(Some(texture)) => self.death_notice_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not render death notice: {error}"),
        }
    }

    pub(super) fn request_death_respawn(&mut self) {
        if !self.death_notice || self.death_respawn_pending || !self.channel_connected {
            return;
        }
        let Ok(packet) = openstory_protocol::change_map_packet(true, 0, "", false) else {
            eprintln!("OpenStoryRS: could not encode nearest-town respawn request");
            return;
        };
        let Some(session) = self.session.as_ref() else {
            return;
        };
        match session.send_packet(packet) {
            Ok(()) => {
                self.death_respawn_pending = true;
                self.refresh_death_notice_texture();
            }
            Err(error) => {
                eprintln!("OpenStoryRS: could not request nearest-town respawn: {error}");
                self.channel_connected = false;
            }
        }
    }

    pub(super) fn apply_character_buffs(
        &mut self,
        update: openstory_protocol::CharacterBuffUpdate,
    ) {
        let mount_changed = update.entries.iter().any(|entry| {
            entry.key.group == openstory_protocol::BuffMaskGroup::First
                && entry.key.flag == STATUS_MONSTER_RIDING
        });
        let disease_effects = update
            .entries
            .iter()
            .filter_map(|entry| mob_disease_skill_parts(entry.skill_id))
            .collect::<HashSet<_>>();
        let now = Instant::now();
        for entry in update.entries {
            let expires_at = (entry.duration_ms > 0)
                .then(|| now + Duration::from_millis(entry.duration_ms as u64));
            self.active_buffs.insert(
                entry.key,
                ActiveCharacterBuff {
                    value: entry.value,
                    skill_id: entry.skill_id,
                    duration_ms: entry.duration_ms,
                    applied_at: now,
                    expires_at,
                },
            );
        }
        for (skill_type, skill_level) in disease_effects {
            self.show_mob_disease_effect(None, skill_type, skill_level);
        }
        if mount_changed
            && let Some(look) = self
                .active_character
                .as_ref()
                .map(|character| character.look.clone())
        {
            self.refresh_local_mount(&look);
        }
        self.refresh_map_hud();
    }

    pub(super) fn show_mob_disease_effect(
        &mut self,
        character_id: Option<i32>,
        skill_type: u16,
        skill_level: u16,
    ) {
        if self.map_scene.is_none() {
            return;
        }
        let animations = match self.load_mob_disease_effect(skill_type, skill_level) {
            Ok(animations) => animations,
            Err(error) => {
                eprintln!(
                    "OpenStoryRS: could not load mob disease effect {skill_type}/{skill_level}: {error}"
                );
                Vec::new()
            }
        };
        if let Some(scene) = self.map_scene.as_mut() {
            scene.show_character_skill_effect(character_id, animations, 1.0);
        }
    }

    pub(super) fn cancel_character_buffs(
        &mut self,
        update: openstory_protocol::CharacterBuffCancel,
    ) {
        let mount_cancelled = update.first_mask & STATUS_MONSTER_RIDING != 0;
        self.active_buffs.retain(|key, _| {
            let mask = match key.group {
                openstory_protocol::BuffMaskGroup::First => update.first_mask,
                openstory_protocol::BuffMaskGroup::Second => update.second_mask,
            };
            mask & key.flag == 0
        });
        if mount_cancelled
            && let Some(look) = self
                .active_character
                .as_ref()
                .map(|character| character.look.clone())
        {
            self.refresh_local_mount(&look);
        }
        self.refresh_map_hud();
    }

    pub(super) fn active_buff_value(
        &self,
        group: openstory_protocol::BuffMaskGroup,
        flag: u64,
    ) -> i16 {
        self.active_buffs
            .get(&openstory_protocol::BuffStatKey { group, flag })
            .filter(|buff| {
                buff.expires_at
                    .is_none_or(|expires_at| expires_at > Instant::now())
            })
            .map_or(0, |buff| buff.value)
    }

    pub(super) fn expire_character_buffs(&mut self, now: Instant) {
        let initial_count = self.active_buffs.len();
        let was_riding = self
            .active_buffs
            .get(&openstory_protocol::BuffStatKey {
                group: openstory_protocol::BuffMaskGroup::First,
                flag: STATUS_MONSTER_RIDING,
            })
            .is_some_and(|buff| buff.value > 0);
        self.active_buffs
            .retain(|_, buff| buff.expires_at.is_none_or(|expires_at| expires_at > now));
        if self.active_buffs.len() != initial_count {
            self.refresh_map_hud();
        }
        if self
            .buff_list_refreshed_at
            .is_none_or(|last| now.saturating_duration_since(last) >= Duration::from_millis(125))
        {
            self.refresh_buff_list_texture(now);
        }
        if was_riding
            && !self
                .active_buffs
                .contains_key(&openstory_protocol::BuffStatKey {
                    group: openstory_protocol::BuffMaskGroup::First,
                    flag: STATUS_MONSTER_RIDING,
                })
            && let Some(look) = self
                .active_character
                .as_ref()
                .map(|character| character.look.clone())
        {
            self.refresh_local_mount(&look);
        }
    }

    pub(super) fn schedule_skill_hit_sounds(
        &mut self,
        skill_id: i32,
        events: &[((i16, i16), Duration)],
    ) {
        if skill_id <= 0 {
            return;
        }
        let start = Instant::now();
        for (position, delay) in events {
            if delay.is_zero() {
                self.audio
                    .play_skill_hit(&self.sound_archive, skill_id, *position);
                continue;
            }
            if self.pending_skill_hit_sounds.len() >= 512 {
                self.pending_skill_hit_sounds.remove(0);
            }
            if let Some(play_at) = start.checked_add(*delay) {
                self.pending_skill_hit_sounds.push(PendingSkillHitSound {
                    skill_id,
                    position: *position,
                    play_at,
                });
            }
        }
    }

    pub(super) fn play_due_skill_hit_sounds(&mut self, now: Instant) {
        let mut due = Vec::new();
        self.pending_skill_hit_sounds.retain(|sound| {
            if sound.play_at <= now {
                due.push(*sound);
                false
            } else {
                true
            }
        });
        for sound in due {
            self.audio
                .play_skill_hit(&self.sound_archive, sound.skill_id, sound.position);
        }
    }

    pub(super) fn refresh_map_hud(&mut self) {
        if self.map_scene.is_none() {
            self.map_hud_texture = None;
            self.buff_list_texture = None;
            return;
        }
        let Some(character) = self.active_character.as_ref() else {
            self.map_hud_texture = None;
            self.buff_list_texture = None;
            return;
        };
        let mut stats = character.stats.clone();
        let hyperbody_hp = self.active_buff_value(
            openstory_protocol::BuffMaskGroup::Second,
            0x0000_2000_0000_0000,
        );
        if hyperbody_hp > 0 {
            stats.max_hp = ((i32::from(stats.max_hp) * (100 + i32::from(hyperbody_hp))) / 100)
                .clamp(0, i32::from(u16::MAX)) as u16;
        }
        let hyperbody_mp = self.active_buff_value(
            openstory_protocol::BuffMaskGroup::Second,
            0x0000_4000_0000_0000,
        );
        if hyperbody_mp > 0 {
            stats.max_mp = ((i32::from(stats.max_mp) * (100 + i32::from(hyperbody_mp))) / 100)
                .clamp(0, i32::from(u16::MAX)) as u16;
        }
        let collapsed_chat_line = self
            .chat_lines
            .back()
            .map(|line| (line.text.as_str(), line.color));
        let pixels = render_map_hud(
            &self.font,
            self.status_bar_number_font.as_ref(),
            self.status_bar_layout,
            &stats,
            &self.active_buffs,
            collapsed_chat_line,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        const WIDTH: u32 = 1024;
        let height = self.status_bar_layout.height();
        let result = if let Some(texture) = self.map_hud_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer.upload_bgra(WIDTH, height, &pixels).map(Some)
        };
        match result {
            Ok(Some(texture)) => self.map_hud_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStory: could not update map HUD: {error}"),
        }
        self.refresh_buff_list_texture(Instant::now());
    }

    pub(super) fn refresh_buff_list_texture(&mut self, now: Instant) {
        self.buff_list_refreshed_at = Some(now);
        if self.map_scene.is_none() || self.active_character.is_none() {
            self.buff_list_texture = None;
            return;
        }

        let mut unique_buffs = std::collections::BTreeMap::new();
        for buff in self.active_buffs.values().copied().filter(|buff| {
            buff.expires_at.is_none_or(|expires_at| expires_at > now) && buff.skill_id != 0
        }) {
            unique_buffs.entry(buff.skill_id).or_insert(buff);
        }
        let mut icons = Vec::with_capacity(unique_buffs.len());
        for (buff_id, buff) in unique_buffs {
            let Some(action) = buff_id.checked_abs() else {
                continue;
            };
            let kind = if buff_id > 0 { 1 } else { 2 };
            let Some((width, height, pixels)) =
                self.quickslot_icon_pixels(openstory_protocol::KeyBinding { kind, action })
            else {
                continue;
            };
            let remaining = buff
                .expires_at
                .map(|expires_at| expires_at.saturating_duration_since(now));
            icons.push((
                width,
                height,
                pixels,
                remaining,
                buff.duration_ms,
                now.saturating_duration_since(buff.applied_at),
            ));
        }
        if icons.is_empty() {
            self.buff_list_texture = None;
            return;
        }

        let pixels = render_buff_list_pixels(&icons);
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.buff_list_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(1024, BUFF_LIST_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.buff_list_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update buff icons: {error}"),
        }
    }

    pub(super) fn load_native_status_bar(&mut self) {
        self.map_hud_texture = None;
        self.status_bar_draw_indices.clear();
        self.status_bar_gauges = [None; 3];
        self.status_bar_number_font = None;
        let (sprites, gauge_sprite_indices, layout) =
            match build_status_bar_sprites(&self.legacy_ui_archive) {
                Ok(assets) => assets,
                Err(error) => {
                    eprintln!("OpenStoryRS: native status bar is unavailable: {error}");
                    return;
                }
            };
        self.status_bar_layout = layout;
        let number_path = match layout {
            StatusBarLayout::Modern => "StatusBar2.img/mainBar/gauge/number",
            StatusBarLayout::V83 => "StatusBar.img/number",
        };
        let number_archive = match layout {
            StatusBarLayout::Modern => &self.ui_archive,
            StatusBarLayout::V83 => &self.legacy_ui_archive,
        };
        match load_status_bar_number_font(number_archive, number_path) {
            Ok(font) => self.status_bar_number_font = Some(font),
            Err(error) => {
                eprintln!("OpenStoryRS: native status-bar number font is unavailable: {error}");
                self.status_bar_number_font = None;
            }
        }
        let uploaded = match self.upload_dynamic_sprites(sprites) {
            Ok(draws) => draws,
            Err(error) => {
                eprintln!("OpenStoryRS: could not upload native status bar: {error}");
                return;
            }
        };
        let gauge_set: HashSet<usize> = gauge_sprite_indices
            .iter()
            .flat_map(|gauge| {
                [Some(gauge.fill_index), gauge.start_index, gauge.end_index]
                    .into_iter()
                    .flatten()
            })
            .collect();
        self.status_bar_draw_indices = uploaded
            .iter()
            .enumerate()
            .filter(|(index, _)| !gauge_set.contains(index))
            .map(|(_, draw)| *draw)
            .collect();
        self.status_bar_gauges = gauge_sprite_indices.map(|gauge| {
            let (texture_index, rect) = uploaded[gauge.fill_index];
            Some(StatusBarGauge {
                texture_index,
                start_texture_index: gauge.start_index.map(|index| uploaded[index].0),
                end_texture_index: gauge.end_index.map(|index| uploaded[index].0),
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            })
        });
    }

    pub(super) fn refresh_minimap_texture(&mut self) {
        if self.minimap_ui_assets.is_none() {
            let loaded = minimap::UiAssets::load(&self.ui_archive);
            if let Err(error) = &loaded {
                eprintln!("OpenStoryRS: minimap UI art unavailable: {error}");
            }
            self.minimap_ui_assets = Some(loaded);
        }
        let Some(Ok(ui_assets)) = self.minimap_ui_assets.as_ref() else {
            self.minimap_texture = None;
            self.minimap_bounds = None;
            self.minimap_hit_regions.clear();
            return;
        };
        let Some(scene) = self.map_scene.as_ref() else {
            self.minimap_texture = None;
            self.minimap_bounds = None;
            self.minimap_hit_regions.clear();
            return;
        };
        let Some(data) = scene.minimap.as_ref().filter(|data| !data.hidden) else {
            self.minimap_texture = None;
            self.minimap_bounds = None;
            self.minimap_hit_regions.clear();
            return;
        };

        let mut markers = Vec::with_capacity(
            1 + scene.npcs.len() + scene.mobs.len() + scene.remote_players.len(),
        );
        markers.push(minimap::Marker {
            kind: minimap::MarkerKind::Player,
            x: scene.player.x as f32,
            y: scene.player.y as f32,
            facing_left: !scene.facing_right,
            sprite: None,
        });
        markers.extend(scene.npcs.values().map(|npc| minimap::Marker {
            kind: minimap::MarkerKind::Npc,
            x: npc.x as f32,
            y: npc.y as f32,
            facing_left: npc.flipped,
            sprite: None,
        }));
        markers.extend(
            scene
                .mobs
                .values()
                .filter(|mob| !mob.dying)
                .map(|mob| minimap::Marker {
                    kind: minimap::MarkerKind::Mob,
                    x: mob.body.x as f32,
                    y: mob.body.y as f32,
                    facing_left: !mob.faces_right,
                    sprite: None,
                }),
        );
        markers.extend(scene.remote_players.values().map(|player| minimap::Marker {
            kind: minimap::MarkerKind::OtherPlayer,
            x: player.x as f32,
            y: player.y as f32,
            facing_left: player.facing_left,
            sprite: None,
        }));

        let map_id = scene.map_id;
        let presentation = scene.minimap_presentation;
        let map_name = map_string_name(&self.string_archive, map_id)
            .unwrap_or_else(|| format!("Map {map_id}"));
        let panel = match minimap::render(
            data,
            ui_assets,
            LogicalSize::LEGACY_1024X768,
            &map_name,
            presentation,
            &markers,
            &self.font,
        ) {
            Ok(panel) => panel,
            Err(error) => {
                eprintln!("OpenStoryRS: could not render minimap for map {map_id}: {error}");
                self.minimap_texture = None;
                self.minimap_bounds = None;
                self.minimap_hit_regions.clear();
                return;
            }
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let same_size = self.minimap_bounds.is_some_and(|bounds| {
            bounds.width == panel.width as f32 && bounds.height == panel.height as f32
        });
        let result = if same_size {
            self.minimap_texture.as_ref().map_or_else(
                || {
                    renderer
                        .upload_bgra(panel.width, panel.height, &panel.pixels)
                        .map(Some)
                },
                |texture| renderer.update_bgra(texture, &panel.pixels).map(|()| None),
            )
        } else {
            renderer
                .upload_bgra(panel.width, panel.height, &panel.pixels)
                .map(Some)
        };
        match result {
            Ok(texture) => {
                if let Some(texture) = texture {
                    self.minimap_texture = Some(texture);
                }
                self.minimap_bounds = Some(minimap::HitRect {
                    x: panel.x,
                    y: panel.y,
                    width: panel.width as f32,
                    height: panel.height as f32,
                });
                self.minimap_hit_regions = panel.hit_regions;
            }
            Err(error) => {
                eprintln!("OpenStoryRS: could not upload minimap: {error}");
                self.minimap_texture = None;
                self.minimap_bounds = None;
                self.minimap_hit_regions.clear();
            }
        }
    }

    pub(super) fn toggle_minimap(&mut self) {
        if let Some(scene) = self.map_scene.as_mut()
            && scene.minimap.as_ref().is_some_and(|data| !data.hidden)
        {
            scene.minimap_presentation = match scene.minimap_presentation {
                minimap::Presentation::Collapsed => minimap::Presentation::Normal,
                minimap::Presentation::Normal => minimap::Presentation::Expanded,
                minimap::Presentation::Expanded => minimap::Presentation::Collapsed,
            };
            self.refresh_minimap_texture();
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }
    }

    pub(super) fn adjust_minimap_presentation(&mut self, expand: bool) {
        if let Some(scene) = self.map_scene.as_mut()
            && scene.minimap.as_ref().is_some_and(|data| !data.hidden)
        {
            scene.minimap_presentation = match (scene.minimap_presentation, expand) {
                (minimap::Presentation::Collapsed, true) => minimap::Presentation::Normal,
                (minimap::Presentation::Normal, true) => minimap::Presentation::Expanded,
                (minimap::Presentation::Expanded, false) => minimap::Presentation::Normal,
                (minimap::Presentation::Normal, false) => minimap::Presentation::Collapsed,
                (presentation, _) => presentation,
            };
            self.refresh_minimap_texture();
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }
    }

    pub(super) fn click_minimap(&mut self, cursor: Point) -> bool {
        let Some(bounds) = self.minimap_bounds else {
            return false;
        };
        if !bounds.contains(cursor.x, cursor.y) {
            return false;
        }
        let action = self
            .minimap_hit_regions
            .iter()
            .rev()
            .find(|hit| hit.bounds.contains(cursor.x, cursor.y))
            .map(|hit| hit.action);
        match action {
            Some(minimap::HitAction::Expand) => self.adjust_minimap_presentation(true),
            Some(minimap::HitAction::Collapse) => self.adjust_minimap_presentation(false),
            Some(minimap::HitAction::OpenWorldMap) => self.open_world_map(),
            None => {}
        }
        true
    }

    pub(super) fn open_messenger(&mut self) {
        self.messenger
            .get_or_insert_with(messenger::State::default)
            .open();
        self.messenger_chat_focused = false;
        self.refresh_messenger_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}
