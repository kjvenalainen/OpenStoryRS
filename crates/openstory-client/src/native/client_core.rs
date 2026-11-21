use super::*;

impl ClientApp {
    pub(super) fn new(
        scene: LoginSceneAssets,
        font: fontdue::Font,
        connect_to: Option<String>,
        map_archive: NxArchive,
        legacy_ui_archive: NxArchive,
        ui_archive: NxArchive,
        character_archive: NxArchive,
        npc_archive: NxArchive,
        mob_archive: NxArchive,
        effect_archive: NxArchive,
        item_archive: NxArchive,
        quest_archive: NxArchive,
        skill_archive: NxArchive,
        string_archive: NxArchive,
        reactor_archive: NxArchive,
        etc_archive: NxArchive,
        sound_archive: NxArchive,
    ) -> Self {
        let LoginSceneAssets {
            sprites,
            actions,
            animated_art,
            login_buttons,
            save_id_indicator_draws,
            ..
        } = scene;
        let (login_animations, login_animation_draw_indices, active_login_animation_draw_indices) =
            prepare_map_art_animations(animated_art);
        let (login_buttons, login_button_draw_indices, active_login_button_draw_indices) =
            prepare_login_buttons(login_buttons);
        let cash_products = load_cash_products(
            &etc_archive,
            &string_archive,
            &character_archive,
            &item_archive,
        );
        let login_preferences = LoginPreferences::load();
        let account_text = std::env::var("OPENSTORY_ACCOUNT").unwrap_or_else(|_| {
            if login_preferences.save_account_id {
                login_preferences.account_id.clone()
            } else {
                String::new()
            }
        });
        let password_text = std::env::var("OPENSTORY_PASSWORD").unwrap_or_default();
        let focused_field = if account_text.is_empty() {
            LoginField::Account
        } else {
            LoginField::Password
        };
        let settings = AudioSettings::load();
        let display_settings = DisplaySettings::load();
        let input_settings = InputSettings::load();
        let gameplay_settings = GameplaySettings::load();
        let social_settings = SocialSettings::load();
        let saved_quickslot_keys = load_quickslot_keys();
        let quickslot_keys_customized = saved_quickslot_keys.is_some();
        let mut dialog_window_offsets = DialogWindow::PERSISTED
            .iter()
            .filter_map(|window| {
                let name = format!("{window:?}");
                settings::load_dialog_window_offset(&name).map(|offset| (*window, offset))
            })
            .collect::<HashMap<_, _>>();
        if !dialog_window_offsets.contains_key(&DialogWindow::SkillBook)
            && let Some((x, y)) = settings::load_skill_book_position()
        {
            let position = ui_foundation::UiRect::new(
                x,
                y,
                SKILL_BOOK_PANEL_WIDTH as f32,
                SKILL_BOOK_PANEL_HEIGHT as f32,
            )
            .clamped_to_canvas();
            dialog_window_offsets.insert(
                DialogWindow::SkillBook,
                (
                    position.x - SKILL_BOOK_PANEL_X,
                    position.y - SKILL_BOOK_PANEL_Y,
                ),
            );
        }
        let mut audio = audio::AudioEngine::new();
        audio.set_music_volume(settings.effective_music_volume());
        audio.set_sound_volume(settings.effective_effects_volume());
        apply_mouse_speed(input_settings.mouse_speed);
        audio.play_music(&sound_archive, "BgmUI.img/Title");
        let quest_start_npc_index = build_quest_start_npc_index(&quest_archive);
        let chat_balloon_skin = match load_chat_balloon_skin(&ui_archive) {
            Ok(skin) => Some(skin),
            Err(error) => {
                eprintln!("OpenStoryRS: chat balloons unavailable: {error}");
                None
            }
        };
        Self {
            window: None,
            renderer: None,
            next_frame: Instant::now(),
            pending_sprites: Some(sprites),
            sprite_draws: Vec::new(),
            sprite_textures: Vec::new(),
            solid_textures: Vec::new(),
            texture_indices: HashMap::new(),
            login_animations,
            login_animation_draw_indices,
            active_login_animation_draw_indices,
            login_buttons,
            login_button_draw_indices,
            active_login_button_draw_indices,
            hovered_login_button: None,
            pressed_login_button: None,
            remote_art_cache: MapArtCache::default(),
            projectile_frames_cache: HashMap::new(),
            item_use_effect_cache: HashMap::new(),
            item_aura_frames_cache: HashMap::new(),
            summon_frames_cache: HashMap::new(),
            dragon_frames_cache: HashMap::new(),
            pet_frames_cache: HashMap::new(),
            pet_interaction_frames_cache: HashMap::new(),
            skill_projectile_cache: HashMap::new(),
            skill_effect_cache: HashMap::new(),
            skill_point_effect_cache: HashMap::new(),
            mob_disease_effect_cache: HashMap::new(),
            character_effect_cache: HashMap::new(),
            field_effect_cache: HashMap::new(),
            weather_overlay: weather::WeatherOverlay::default(),
            weather_texture: None,
            scrolling_notice: None,
            server_notice_popup: None,
            avatar_megaphone: None,
            monster_carnival: None,
            clock_state: None,
            clock_texture: None,
            clock_position: None,
            skill_hit_effect_cache: HashMap::new(),
            face_expression_cache: HashMap::new(),
            last_face_expression_at: None,
            teleport_effect_frames: None,
            login_actions: actions,
            last_login_double_click: None,
            cursor: None,
            cursor_physical: None,
            dialog_window_offsets,
            raised_dialog_windows: Vec::new(),
            dialog_window_drag: None,
            field_textures: Vec::with_capacity(2),
            account_text,
            password_text,
            focused_field,
            login_caret_visible: true,
            next_login_caret_toggle: Instant::now() + LOGIN_CARET_BLINK_INTERVAL,
            focused_login_button: None,
            character_action_focus: None,
            suppress_login_ime_space: false,
            login_key_text_for_ime_dedup: None,
            character_creation_key_text_for_ime_dedup: None,
            save_account_id: login_preferences.save_account_id,
            save_id_indicator_draws,
            pic_entry: None,
            pic_dialog_texture: None,
            character_delete_confirmation: None,
            character_delete_confirmation_draws: std::array::from_fn(|_| Vec::new()),
            character_delete_confirmation_buttons: [[None; 2]; 2],
            settings_texture: None,
            status_texture: None,
            status_detail_texture: None,
            status_auto_assign_texture: None,
            key_config_texture: None,
            skill_macro_texture: None,
            skill_macro_drag_texture: None,
            inventory_texture: None,
            inventory_drag_texture: None,
            equipment_inventory_texture: None,
            inventory_tooltip_texture: None,
            npc_shop_texture: None,
            npc_shop: None,
            storage_texture: None,
            hired_merchant_texture: None,
            trade_texture: None,
            character_info_texture: None,
            inventory_drop_prompt_texture: None,
            death_notice_texture: None,
            pet_equipment_texture: None,
            cash_shop_texture: None,
            cash_shop_hit_regions: Vec::new(),
            owl_texture: None,
            skill_book_texture: None,
            skill_tooltip_texture: None,
            skill_tooltip_key: None,
            skill_tooltip_height: 0,
            quickslot_tooltip_skill_id: None,
            quest_log_texture: None,
            quest_helper_texture: None,
            quest_helper_texture_size: None,
            quest_helper_dismissed: false,
            monster_book: monster_book::State::default(),
            monster_book_animation_texture: None,
            world_map: world_map::State::default(),
            world_map_key_text_for_ime_dedup: None,
            channel_switch_open: false,
            channel_switch_texture: None,
            channel_switch_actions: Vec::new(),
            channel_switch_selected: 0,
            channel_switch_last_click: None,
            messenger: None,
            messenger_texture: None,
            messenger_actions: Vec::new(),
            messenger_bounds: None,
            messenger_chat_focused: false,
            messenger_drag_offset: None,
            minimap_ui_assets: None,
            minimap_texture: None,
            minimap_bounds: None,
            minimap_hit_regions: Vec::new(),
            next_minimap_refresh: Instant::now(),
            party_search: None,
            party_search_texture: None,
            party_search_actions: Vec::new(),
            chat_texture: None,
            item_megaphone_texture: None,
            whisper_texture: None,
            party_texture: None,
            buddy_texture: None,
            guild_texture: None,
            family_texture: None,
            family_tree_texture: None,
            alliance_texture: None,
            guild_bbs_texture: None,
            status_messages: VecDeque::new(),
            chat_lines: VecDeque::from([ChatLine {
                text: "[Welcome] Welcome to MapleStory!".to_owned(),
                color: [255, 255, 0],
            }]),
            chat_input: String::new(),
            chat_scroll: 0,
            chat_open: true,
            chat_input_focused: false,
            chat_target: ChatTarget::Public,
            cash_item_compose: None,
            owl_search: None,
            pending_merchant_permit: None,
            pending_trade_invite: None,
            trade_request_pending: false,
            whisper_conversations: Vec::with_capacity(8),
            active_whisper: None,
            whisper_input_focused: false,
            whisper_target_focused: false,
            party_state: None,
            pending_party_invite: None,
            last_party_invite_target: String::new(),
            show_party_panel: false,
            guild_info: None,
            show_guild_panel: false,
            family_ui: FamilyUiState::default(),
            show_family_panel: false,
            show_family_tree: false,
            pending_family_join: None,
            pending_family_summon: None,
            guild_panel_mode: GuildPanelMode::Overview,
            guild_scroll: 0,
            pending_guild_invite: None,
            pending_guild_creation_invite: None,
            alliance_info: None,
            alliance_scroll: 0,
            show_alliance_panel: false,
            pending_alliance_invite: None,
            guild_bbs: None,
            buddy_entries: Vec::new(),
            buddy_capacity: 20,
            buddy_filter: None,
            pending_buddy_invite: None,
            show_buddy_panel: false,
            buddy_scroll: 0,
            chat_balloon_skin,
            local_speech_texture: None,
            remote_speech_textures: HashMap::new(),
            local_chalkboard_texture: None,
            remote_chalkboard_textures: HashMap::new(),
            pet_speech_textures: HashMap::new(),
            inventory_icon_cache: HashMap::new(),
            show_inventory: false,
            show_equipment_inventory: false,
            show_settings: false,
            show_status: false,
            show_status_detail: false,
            show_key_config: false,
            key_config_capture: None,
            key_config_draft: None,
            key_config_quickslot_draft: None,
            quickslot_key_capture: None,
            key_config_scroll_row: 0,
            key_config_actions: Vec::new(),
            key_config_actions_dirty: true,
            server_keymap: None,
            quickslot_keys: saved_quickslot_keys.unwrap_or(DEFAULT_QUICKSLOT_KEYS),
            quickslot_hover_slot: None,
            quickslot_pointer_binding: None,
            quickslot_keys_customized,
            show_quickslots: true,
            quickslot_texture: None,
            quickslot_rendered_counts: None,
            quickslot_icon_cache: HashMap::new(),
            show_game_menu: false,
            game_menu_texture: None,
            combo_count: 0,
            combo_displayed: 0,
            combo_fade_remaining: Duration::ZERO,
            combo_texture: None,
            skill_macro_editor: skill_macros::State::default(),
            skill_macros: vec![
                openstory_protocol::SkillMacro::default();
                openstory_protocol::SKILL_MACRO_COUNT
            ],
            skill_book_drag: None,
            skill_book_last_click: None,
            pending_macro_skills: VecDeque::new(),
            next_macro_skill_at: Instant::now(),
            status_auto_confirmation: false,
            status_waiting_for_stats: false,
            settings_dragging: None,
            settings_selected_slider: OptionControl::Music,
            settings,
            settings_original: settings,
            display_settings,
            input_settings,
            input_settings_original: input_settings,
            gameplay_settings,
            gameplay_settings_original: gameplay_settings,
            social_settings,
            social_settings_original: social_settings,
            settings_notice: None,
            show_pet_equipment: false,
            pet_equipment_scroll_row: 0,
            show_skill_book: false,
            show_quest_log: false,
            skill_book_tab: 0,
            skill_book_scroll_row: 0,
            skills: HashMap::new(),
            skill_book_entries: Vec::new(),
            active_buffs: CharacterBuffs::default(),
            pending_skill_hit_sounds: Vec::new(),
            pending_mob_hit_sounds: Vec::new(),
            skill_cooldowns: HashMap::new(),
            forced_equipment_stats: ForcedEquipmentStats::default(),
            inventory_tab: openstory_protocol::InventoryType::Equip,
            inventory_scroll_row: 0,
            inventory_hover_slot: None,
            inventory_drag_source: None,
            inventory_pending_move: None,
            pending_scroll_slot: None,
            inventory_drop_prompt: None,
            death_notice: false,
            death_respawn_pending: false,
            font,
            server_address: connect_to
                .clone()
                .unwrap_or_else(|| "127.0.0.1:8484".to_owned()),
            session: connect_to.map(session::connect),
            pending_login_packet: None,
            login_server_connected: false,
            channel_connected: false,
            server_ui_disabled: false,
            debug_event_loop_logged: false,
            audio,
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
            item_name_cache: HashMap::new(),
            owl_name_index: None,
            damage_glyph_texture: None,
            damage_glyphs: Vec::with_capacity(20),
            maple_tv_assets_loaded: false,
            maple_tv_base_pixels: None,
            maple_tv_idle_texture: None,
            maple_tv_ad_animations: Vec::with_capacity(3),
            maple_tv_ad_index: 0,
            maple_tv_theme_animations: [MapleTvAnimation::empty(), MapleTvAnimation::empty()],
            maple_tv_text_texture: None,
            maple_tv_broadcast: None,
            npc_dialogue: None,
            npc_dialogue_scroll_dragging: false,
            hired_merchant_room: None,
            trade_room: None,
            character_info: None,
            storage: None,
            pending_storage_restores: Vec::new(),
            cash_shop: None,
            cash_products,
            mts_open: false,
            mts_open_snapshot: None,
            mts_balances: None,
            mts_last_event: None,
            bot_inventory_snapshot: None,
            bot_inventory_last_event: None,
            session_energy: None,
            gm_account: false,
            gm_hidden: false,
            map_hud_texture: None,
            buff_list_texture: None,
            buff_list_refreshed_at: None,
            status_bar_draw_indices: Vec::new(),
            status_bar_gauges: [None; 3],
            status_bar_number_font: None,
            status_bar_level_font: None,
            status_bar_layout: StatusBarLayout::V83,
            boss_hp_gauge_texture: None,
            boss_hp_gauge_remaining: Duration::ZERO,
            event_list_open: false,
            event_list_texture: None,
            event_list: Vec::new(),
            event_list_received: false,
            event_list_offset: 0,
            event_list_selected: None,
            event_list_peak_durations: HashMap::new(),
            next_event_info_request: Instant::now(),
            next_event_second: Instant::now(),
            wedding_panel_open: false,
            wedding_panel_texture: None,
            wedding_countdown_seconds: 0,
            rps_game: None,
            quit_confirmation: false,
            quit_confirmation_draws: Vec::new(),
            quit_confirmation_buttons: [None; 2],
            quit_requested: false,
            scriptable_npc_ids: HashSet::new(),
            worlds: Vec::new(),
            preferred_world: None,
            selected_world: None,
            selected_channel: 0,
            character_list: None,
            character_page: 0,
            selected_character_index: 0,
            character_selection_status: None,
            character_creation: None,
            accept_tos_pending: false,
            active_character: None,
            taming_mount_stats: None,
            inventory: None,
            extra_pendant_slot: false,
            auto_hp_potion_id: 0,
            auto_mp_potion_id: 0,
            auto_hp_potion_cooldown: Duration::ZERO,
            auto_mp_potion_cooldown: Duration::ZERO,
            teleport_rock_locations: openstory_protocol::TeleportRockLocations::default(),
            quest_log: openstory_protocol::QuestLog::default(),
            quest_start_npc_index,
            available_quests: Vec::new(),
            quest_log_tab: QuestLogTab::Active,
            quest_log_detail_open: true,
            quest_log_selected: 0,
            quest_log_scroll: 0,
            quest_reward_selected: 0,
            quest_forfeit_confirmation: None,
            current_field: None,
            map_load_receiver: None,
            pending_map_scene_upload: None,
            map_loading_started_at: None,
            map_loading_frame_rendered: false,
            map_loading_texture: None,
            map_initial_npc_loading_until: None,
            map_entry_refresh_pending: false,
            map_entry_refresh_step: 0,
            pending_map_transition_ack: None,
            map_prefetch_receiver: None,
            map_prefetch_cancel: None,
            prefetched_map_scenes: VecDeque::new(),
            prefetched_map_scene_bytes: 0,
            show_login_fields: true,
            map_scene: None,
            last_npc_talk_at: None,
            pending_npc_talk: None,
            last_local_skill_cast: None,
            pending_zombify_heal_feedback_until: None,
            gameplay_input: GameplayInput::default(),
            key_config_pointer_start: None,
            simulation_clock: FixedStepClock::default(),
            client_started_at: Instant::now(),
            last_simulation_update: Instant::now(),
        }
    }

    pub(super) fn advance_idle_regeneration(&mut self, elapsed: Duration, is_dead: bool) {
        const REGEN_INTERVAL: Duration = Duration::from_secs(10);
        let character = self.active_character.as_ref().map(|character| {
            (
                i32::from(character.stats.level),
                i32::from(character.stats.intelligence),
                i32::from(character.stats.hp),
                i32::from(character.stats.max_hp),
                i32::from(character.stats.mp),
                i32::from(character.stats.max_mp),
            )
        });
        let hp_recovery_skill_level = self
            .skills
            .get(&2_000_000)
            .map_or(0, |skill| skill.level.clamp(0, 255));
        let darksight = character_has_buff(
            &self.active_buffs,
            openstory_protocol::BuffMaskGroup::Second,
            STATUS_DARKSIGHT,
        );
        let packet = {
            let Some(scene) = self.map_scene.as_mut() else {
                return;
            };
            scene.regen_hit_lockout_remaining =
                scene.regen_hit_lockout_remaining.saturating_sub(elapsed);
            if is_dead
                || scene.attack_stance.is_some()
                || !scene.local_invincibility_remaining.is_zero()
                || darksight
                || !scene.regen_hit_lockout_remaining.is_zero()
            {
                scene.idle_regen_elapsed = Duration::ZERO;
                return;
            }
            scene.idle_regen_elapsed = scene.idle_regen_elapsed.saturating_add(elapsed);
            if scene.idle_regen_elapsed < REGEN_INTERVAL {
                return;
            }
            scene.idle_regen_elapsed = Duration::ZERO;
            let Some((level, intelligence, hp, max_hp, mp, max_mp)) = character else {
                return;
            };
            let chair_multiplier = if scene.local_chair.is_some() { 3 } else { 1 };
            let missing_hp = (max_hp - hp).max(0);
            let missing_mp = (max_mp - mp).max(0);
            let heal_hp = ((level / 5 + 2).max(1) * chair_multiplier)
                .min(missing_hp)
                .min(100);
            let heal_mp =
                ((level / 5 + intelligence / 20 + 3 + level * hp_recovery_skill_level / 10).max(1)
                    * chair_multiplier)
                    .min(missing_mp)
                    .min(999);
            if heal_hp == 0 && heal_mp == 0 {
                return;
            }
            Some(openstory_protocol::heal_over_time_packet(
                heal_hp as i16,
                heal_mp as i16,
            ))
        };
        if let Some(packet) = packet
            && self.channel_connected
            && let Some(session) = self.session.as_ref()
            && let Err(error) = session.send_packet(packet)
        {
            eprintln!("OpenStory: could not send passive HP/MP recovery: {error}");
            self.channel_connected = false;
        }
    }

    pub(super) fn advance_auto_potions(&mut self, elapsed: Duration, is_dead: bool) {
        const AUTO_POTION_COOLDOWN: Duration = Duration::from_secs(4);
        self.auto_hp_potion_cooldown = self.auto_hp_potion_cooldown.saturating_sub(elapsed);
        self.auto_mp_potion_cooldown = self.auto_mp_potion_cooldown.saturating_sub(elapsed);

        if is_dead || !self.channel_connected || self.session.is_none() || self.map_scene.is_none()
        {
            return;
        }
        let Some((character_id, hp, max_hp, mp, max_mp)) =
            self.active_character.as_ref().map(|character| {
                (
                    character.id,
                    i32::from(character.stats.hp),
                    i32::from(character.stats.max_hp),
                    i32::from(character.stats.mp),
                    i32::from(character.stats.max_mp),
                )
            })
        else {
            return;
        };
        let has_pet = self.map_scene.as_ref().is_some_and(|scene| {
            scene
                .pets
                .keys()
                .any(|(owner_id, _)| *owner_id == character_id)
        });
        if !has_pet {
            return;
        }

        let find_use_slot = |item_id| {
            self.inventory
                .as_ref()?
                .items
                .iter()
                .find(|item| {
                    item.inventory_type == openstory_protocol::InventoryType::Use
                        && item.item_id == item_id
                        && item.count > 0
                })
                .and_then(|item| i16::try_from(item.slot).ok())
        };
        let hp_slot = (self.auto_hp_potion_id > 0
            && hp.saturating_mul(2) < max_hp
            && self.auto_hp_potion_cooldown.is_zero())
        .then(|| find_use_slot(self.auto_hp_potion_id))
        .flatten();
        let mp_slot = (self.auto_mp_potion_id > 0
            && mp.saturating_mul(2) < max_mp
            && self.auto_mp_potion_cooldown.is_zero())
        .then(|| find_use_slot(self.auto_mp_potion_id))
        .flatten();
        if hp_slot.is_none() && mp_slot.is_none() {
            return;
        }

        let client_tick = self.client_started_at.elapsed().as_millis() as i32;
        let mut packets = Vec::with_capacity(2);
        if let Some(slot) = hp_slot {
            self.audio
                .play_item_sound(&self.sound_archive, self.auto_hp_potion_id);
            packets.push(openstory_protocol::use_item_packet(
                slot,
                self.auto_hp_potion_id,
                client_tick,
            ));
            self.auto_hp_potion_cooldown = AUTO_POTION_COOLDOWN;
        }
        if let Some(slot) = mp_slot {
            self.audio
                .play_item_sound(&self.sound_archive, self.auto_mp_potion_id);
            packets.push(openstory_protocol::use_item_packet(
                slot,
                self.auto_mp_potion_id,
                client_tick,
            ));
            self.auto_mp_potion_cooldown = AUTO_POTION_COOLDOWN;
        }
        for packet in packets {
            let Some(session) = self.session.as_ref() else {
                break;
            };
            if let Err(error) = session.send_packet(packet) {
                eprintln!("OpenStoryRS: could not send automatic potion request: {error}");
                self.channel_connected = false;
                break;
            }
        }
    }

    pub(super) fn advance_login_animations(&mut self, elapsed: Duration) {
        if elapsed.is_zero() || self.login_animations.is_empty() {
            return;
        }
        for animation in &mut self.login_animations {
            animation.elapsed = animation.elapsed.saturating_add(elapsed);
            let mut changed = false;
            loop {
                if animation.hold_last && animation.frame_index + 1 == animation.frames.len() {
                    animation.elapsed = Duration::ZERO;
                    break;
                }
                let Some(frame) = animation.frames.get(animation.frame_index) else {
                    break;
                };
                let delay = Duration::from_millis(u64::from(frame.delay_ms.max(1)));
                if animation.elapsed < delay {
                    break;
                }
                animation.elapsed -= delay;
                animation.frame_index = if animation.frame_index + 1 == animation.frames.len() {
                    0
                } else {
                    animation.frame_index + 1
                };
                changed = true;
            }
            if changed {
                for frame in &animation.frames {
                    for draw_index in frame.sprites.clone() {
                        self.active_login_animation_draw_indices.remove(&draw_index);
                    }
                }
                if let Some(frame) = animation.frames.get(animation.frame_index) {
                    self.active_login_animation_draw_indices
                        .extend(frame.sprites.clone());
                }
            }
        }
    }

    pub(super) fn update_map_simulation(&mut self, elapsed: Duration) {
        let frame = self.simulation_clock.advance(elapsed);
        self.advance_clock(frame.updates);
        if frame.updates > 0 && !self.boss_hp_gauge_remaining.is_zero() {
            self.boss_hp_gauge_remaining = self
                .boss_hp_gauge_remaining
                .saturating_sub(Duration::from_millis(8 * u64::from(frame.updates)));
            if self.boss_hp_gauge_remaining.is_zero() {
                self.boss_hp_gauge_texture = None;
            }
        }
        if frame.updates > 0 {
            self.advance_combo_counter(Duration::from_millis(8 * u64::from(frame.updates)));
        }
        if frame.updates > 0 {
            let texture_width = self.weather_texture.as_ref().map_or(0, GpuTexture::width);
            self.weather_overlay.advance(frame.updates, texture_width);
            self.advance_scrolling_notice(frame.updates);
        }
        if self.cash_shop.is_some() {
            self.gameplay_input = GameplayInput::default();
            return;
        }
        if frame.updates == 0 {
            return;
        }
        let is_stunned =
            self.active_buff_value(openstory_protocol::BuffMaskGroup::Second, STATUS_STUN) != 0;
        let is_confused =
            self.active_buff_value(openstory_protocol::BuffMaskGroup::Second, STATUS_CONFUSE) != 0;
        let is_slowed =
            self.active_buff_value(openstory_protocol::BuffMaskGroup::Second, STATUS_SLOW) != 0;
        if is_stunned {
            self.gameplay_input.jump_buffer = 0;
        }
        let is_dead = self.death_notice
            || self
                .active_character
                .as_ref()
                .is_some_and(|character| character.stats.hp == 0);
        self.advance_idle_regeneration(
            Duration::from_millis(8 * u64::from(frame.updates)),
            is_dead,
        );
        self.advance_auto_potions(Duration::from_millis(8 * u64::from(frame.updates)), is_dead);
        let (walk_force, jump_force, climb_force) =
            self.active_character
                .as_ref()
                .map_or((0.16, 4.5, 1.0), |character| {
                    player_movement_forces(
                        character,
                        &self.character_archive,
                        &self.active_buffs,
                        &self.forced_equipment_stats,
                        is_slowed,
                    )
                });
        let Some(scene) = self.map_scene.as_mut() else {
            self.gameplay_input.jump_buffer = 0;
            return;
        };
        let local_character_id = self.active_character.as_ref().map(|character| character.id);
        scene.advance_map_art_animations(frame.updates);
        scene.advance_climb_cooldown(frame.updates);
        scene.advance_local_chair_animations(frame.updates);
        scene.advance_remote_players(frame.updates);
        scene.local_speech_bubble_remaining =
            scene.local_speech_bubble_remaining.and_then(|remaining| {
                let remaining =
                    remaining.saturating_sub(Duration::from_millis(8 * u64::from(frame.updates)));
                (!remaining.is_zero()).then_some(remaining)
            });
        scene.advance_npcs(frame.updates);
        scene.advance_hired_merchants(frame.updates);
        scene.advance_quest_clear_effect(frame.updates);
        scene.advance_drops(frame.updates, local_character_id);
        scene.advance_reactors(frame.updates);
        scene.advance_projectiles(frame.updates);
        scene.advance_summons(frame.updates);
        scene.advance_dragons(frame.updates);
        let pet_packets = scene.advance_pets(frame.updates, local_character_id);
        scene.advance_character_effects(frame.updates);
        scene.advance_screen_effect(frame.updates);
        let mob_packets = scene.advance_mobs(frame.updates);
        if self.channel_connected {
            for packet in pet_packets {
                let Some(session) = self.session.as_ref() else {
                    break;
                };
                if let Err(error) = session.send_packet(packet) {
                    eprintln!("OpenStoryRS: could not send pet update: {error}");
                    self.channel_connected = false;
                    break;
                }
            }
        }
        if self.channel_connected {
            for packet in mob_packets {
                let Some(session) = self.session.as_ref() else {
                    break;
                };
                if let Err(error) = session.send_packet(packet) {
                    eprintln!("OpenStory: could not send controlled mob movement: {error}");
                    self.channel_connected = false;
                    break;
                }
            }
        }
        if is_dead {
            self.gameplay_input = GameplayInput::default();
            scene.player.horizontal_speed = 0.0;
            scene.player.vertical_speed = 0.0;
            scene.player.on_ground = true;
            scene.attack_stance = None;
            scene.advance_local_death_art(frame.updates);
            if scene.local_death_art.is_none() {
                scene.advance_avatar_animation(
                    Duration::from_millis(8 * u64::from(frame.updates)),
                    AvatarMotion::Stand,
                    AvatarMotion::Stand,
                    &mut self.sprite_draws,
                );
            }
            return;
        }
        let (left, right) = if scene.is_locally_seated() {
            (false, false)
        } else if is_confused {
            (self.gameplay_input.right, self.gameplay_input.left)
        } else {
            (self.gameplay_input.left, self.gameplay_input.right)
        };
        // Upstream ignores player movement input during attacks but still lets
        // the physics step resolve existing momentum.
        let attack_active = scene.attack_stance.is_some();
        let movement_left = left && !attack_active;
        let movement_right = right && !attack_active;
        let movement_up = self.gameplay_input.up && !attack_active;
        let movement_down = self.gameplay_input.down && !attack_active;
        if attack_active {
            self.gameplay_input.jump_buffer = 0;
        }
        let facing = match (movement_left && !is_stunned, movement_right && !is_stunned) {
            (true, false) => Some(false),
            (false, true) => Some(true),
            _ => None,
        };
        if let Some(facing_right) = facing {
            scene.set_facing(facing_right, &mut self.sprite_draws);
        }
        self.audio
            .set_listener((rounded_i16(scene.player.x), rounded_i16(scene.player.y)));
        if self.gameplay_input.sit {
            scene.sit_on_nearby_map_seat(&mut self.sprite_draws);
        }
        if scene.local_chair.is_some() {
            scene.player.horizontal_speed = 0.0;
            scene.player.vertical_speed = 0.0;
            scene.player.on_ground = true;
            self.gameplay_input.jump_buffer = 0;
        } else if scene.local_map_seat.is_some() {
            scene.player.horizontal_speed = 0.0;
            scene.player.vertical_speed = 0.0;
            scene.player.on_ground = true;
            self.gameplay_input.jump_buffer = 0;
            let movement = player_movement(
                &scene.player,
                scene.facing_right,
                false,
                false,
                true,
                false,
                None,
            );
            if scene.last_movement != Some(movement) && self.channel_connected {
                if let Some(session) = self.session.as_ref() {
                    match session.send_packet(openstory_protocol::move_player(movement)) {
                        Ok(()) => scene.last_movement = Some(movement),
                        Err(error) => {
                            eprintln!("OpenStory: could not send seated player movement: {error}");
                            self.channel_connected = false;
                        }
                    }
                }
            }
        } else if let Some(footholds) = scene.footholds.as_ref() {
            for _ in 0..frame.updates {
                self.gameplay_input.pickup_cooldown_ticks =
                    self.gameplay_input.pickup_cooldown_ticks.saturating_sub(1);
                if let Some(ladder) = scene.local_ladder {
                    scene.player.mode = PhysicsMode::Fixated;
                    scene.player.x = f64::from(ladder.x);
                    scene.player.horizontal_speed = 0.0;
                    if self.gameplay_input.jump_buffer > 0
                        && (movement_left || movement_right)
                        && !is_stunned
                    {
                        self.audio.play_world_sound(
                            &self.sound_archive,
                            "Game.img/Jump",
                            (rounded_i16(scene.player.x), rounded_i16(scene.player.y)),
                        );
                        let jump_right = movement_right;
                        if scene.facing_right != jump_right {
                            scene.facing_right = jump_right;
                            scene.refresh_avatar_draws(&mut self.sprite_draws);
                        }
                        scene.player.horizontal_speed = if movement_left {
                            -walk_force * 8.0
                        } else {
                            walk_force * 8.0
                        };
                        scene.player.vertical_speed = -jump_force / 1.5;
                        scene.local_ladder = None;
                        scene.player.mode = PhysicsMode::Normal;
                        scene.player.on_ground = false;
                        scene.player.foothold_layer = scene
                            .footholds
                            .as_ref()
                            .and_then(|tree| tree.get(scene.player.foothold_id))
                            .map_or(0, |foothold| foothold.layer);
                        scene.climb_cooldown = 32;
                        self.gameplay_input.jump_buffer = 0;
                    } else {
                        scene.player.vertical_speed = if is_stunned {
                            0.0
                        } else if movement_up && !movement_down {
                            -climb_force
                        } else if movement_down && !movement_up {
                            climb_force
                        } else {
                            0.0
                        };
                    }
                } else {
                    let swimming = scene.underwater && !scene.player.on_ground;
                    scene.player.mode = if swimming {
                        PhysicsMode::Swimming
                    } else {
                        PhysicsMode::Normal
                    };
                    if swimming {
                        const SWIM_FORCE: f64 = 0.25;
                        let horizontal = if is_stunned {
                            0.0
                        } else if movement_left && !movement_right {
                            -SWIM_FORCE
                        } else if movement_right && !movement_left {
                            SWIM_FORCE
                        } else {
                            0.0
                        };
                        let vertical = if is_stunned {
                            0.0
                        } else if movement_up && !movement_down {
                            -SWIM_FORCE
                        } else if movement_down && !movement_up {
                            SWIM_FORCE
                        } else {
                            0.0
                        };
                        scene.player.apply_force(horizontal, vertical);
                        self.gameplay_input.jump_buffer = 0;
                        self.gameplay_input.jump_down_ticks = 0;
                    } else {
                        if is_stunned {
                            scene.player.horizontal_speed = 0.0;
                        }
                        if scene.player.on_ground {
                            let crouching = movement_down;
                            if crouching {
                                scene.player.horizontal_speed = 0.0;
                            }
                            let direction = if is_stunned || crouching {
                                0.0
                            } else {
                                match (movement_left, movement_right) {
                                    (true, false) => -1.0,
                                    (false, true) => 1.0,
                                    _ => 0.0,
                                }
                            };
                            if direction != 0.0 {
                                scene.player.apply_force(direction * walk_force, 0.0);
                            }
                            if self.gameplay_input.jump_buffer > 0 && !is_stunned && !attack_active
                            {
                                if movement_down && self.gameplay_input.jump_down_ticks < 5 {
                                    self.gameplay_input.jump_down_ticks += 1;
                                } else {
                                    self.audio.play_world_sound(
                                        &self.sound_archive,
                                        "Game.img/Jump",
                                        (rounded_i16(scene.player.x), rounded_i16(scene.player.y)),
                                    );
                                    let lower_foothold = self
                                        .gameplay_input
                                        .down
                                        .then(|| {
                                            scene.footholds.as_ref().and_then(|tree| {
                                                let ground = tree
                                                    .get(scene.player.foothold_id)
                                                    .and_then(|foothold| {
                                                        foothold.ground_y(scene.player.x)
                                                    })
                                                    .unwrap_or(scene.player.y);
                                                tree.find_below(scene.player.x, ground + 1.0)
                                            })
                                        })
                                        .flatten();
                                    if let Some((foothold_id, _)) = lower_foothold {
                                        scene.player.y += 1.0;
                                        scene.player.foothold_id = foothold_id;
                                        scene.player.foothold_layer = scene
                                            .footholds
                                            .as_ref()
                                            .and_then(|tree| tree.get(foothold_id))
                                            .map_or(0, |foothold| foothold.layer);
                                        scene.player.vertical_speed = 0.0;
                                        scene.player.on_ground = false;
                                        scene.player.jump_down_enabled = false;
                                    } else {
                                        scene.player.apply_force(0.0, -jump_force);
                                    }
                                    self.gameplay_input.jump_buffer = 0;
                                    self.gameplay_input.jump_down_ticks = 0;
                                }
                            } else if !movement_down {
                                self.gameplay_input.jump_down_ticks = 0;
                            }
                        } else {
                            self.gameplay_input.jump_down_ticks = 0;
                            if is_stunned {
                                scene.player.horizontal_speed = 0.0;
                            } else if movement_left
                                && !movement_right
                                && scene.player.horizontal_speed > 0.0
                            {
                                scene.player.horizontal_speed =
                                    (scene.player.horizontal_speed - 0.025).max(0.0);
                            } else if movement_right
                                && !movement_left
                                && scene.player.horizontal_speed < 0.0
                            {
                                scene.player.horizontal_speed =
                                    (scene.player.horizontal_speed + 0.025).min(0.0);
                            }
                            self.gameplay_input.jump_buffer =
                                self.gameplay_input.jump_buffer.saturating_sub(1);
                        }
                    }
                }
                PhysicsEngine.move_body(footholds, &mut scene.player);
                if let Some(ladder) = scene.local_ladder {
                    if ladder.fell_off(rounded_i16(scene.player.y), self.gameplay_input.down) {
                        scene.local_ladder = None;
                        scene.player.mode = PhysicsMode::Normal;
                        scene.player.on_ground = false;
                        scene.player.foothold_layer = scene
                            .footholds
                            .as_ref()
                            .and_then(|tree| tree.get(scene.player.foothold_id))
                            .map_or(0, |foothold| foothold.layer);
                        scene.climb_cooldown = 32;
                    }
                } else if !is_stunned && scene.attack_stance.is_none() {
                    let upwards = self.gameplay_input.up && !self.gameplay_input.down;
                    if (upwards || self.gameplay_input.down) && scene.climb_cooldown == 0 {
                        let x = rounded_i16(scene.player.x);
                        let y = rounded_i16(scene.player.y);
                        if let Some(ladder) = scene
                            .map_ladders
                            .iter()
                            .copied()
                            .find(|ladder| ladder.in_range(x, y, upwards))
                        {
                            scene.local_ladder = Some(ladder);
                            scene.player.x = f64::from(ladder.x);
                            scene.player.horizontal_speed = 0.0;
                            scene.player.vertical_speed = 0.0;
                            scene.player.foothold_layer = 7;
                            scene.player.mode = PhysicsMode::Fixated;
                        }
                    }
                }
                let seated_on_map = if self.gameplay_input.sit
                    && scene.local_chair.is_none()
                    && scene.local_map_seat.is_none()
                    && scene.local_ladder.is_none()
                    && scene.attack_stance.is_none()
                {
                    nearby_map_seat(&scene.map_seats, &scene.player).is_some_and(|(x, y)| {
                        scene.local_map_seat = Some((x, y));
                        place_player_on_map_seat(
                            (x, y),
                            &mut scene.player,
                            scene.footholds.as_ref(),
                        );
                        scene.last_movement = None;
                        scene.desired_avatar_motion = AvatarMotion::Sit;
                        scene.active_avatar_frame =
                            first_avatar_frame(&scene.avatar_frames, AvatarMotion::Sit);
                        scene.avatar_elapsed = Duration::ZERO;
                        true
                    })
                } else {
                    false
                };
                if self.channel_connected {
                    let movement = player_movement(
                        &scene.player,
                        scene.facing_right,
                        left && !is_stunned,
                        right && !is_stunned,
                        scene.local_map_seat.is_some(),
                        movement_down,
                        scene.local_ladder.map(|ladder| ladder.is_ladder),
                    );
                    if scene.last_movement != Some(movement) {
                        if let Some(session) = self.session.as_ref() {
                            match session.send_packet(openstory_protocol::move_player(movement)) {
                                Ok(()) => scene.last_movement = Some(movement),
                                Err(error) => {
                                    eprintln!("OpenStory: could not send player movement: {error}");
                                    self.channel_connected = false;
                                }
                            }
                        }
                    }
                }
                if seated_on_map {
                    self.gameplay_input.jump_buffer = 0;
                    break;
                }
            }
        } else {
            self.gameplay_input.jump_buffer = 0;
        }

        if scene.footholds.is_some()
            && scene.local_chair.is_none()
            && scene.local_map_seat.is_none()
            && self.channel_connected
            && self.gameplay_input.pickup
            && self.gameplay_input.pickup_cooldown_ticks == 0
            && let Some(object_id) = scene.nearest_drop()
        {
            // The legacy client checks once per ~16 ms game frame.
            self.gameplay_input.pickup_cooldown_ticks = 2;
            let (x, y) = (rounded_i16(scene.player.x), rounded_i16(scene.player.y));
            if let Some(session) = self.session.as_ref() {
                match session.send_packet(openstory_protocol::pickup_item(object_id, x, y)) {
                    Ok(()) => scene.mark_drop_pickup_attempt(object_id),
                    Err(error) => {
                        eprintln!("OpenStory: could not request item pickup: {error}");
                        self.channel_connected = false;
                    }
                }
            }
        }

        let active_character = self.active_character.as_ref();
        let touch_protected = !self.channel_connected
            || active_character.is_none_or(|character| character.stats.hp == 0)
            || character_has_buff(
                &self.active_buffs,
                openstory_protocol::BuffMaskGroup::Second,
                STATUS_DARKSIGHT,
            );
        let touch_elapsed = Duration::from_millis(8 * u64::from(frame.updates));
        if let Some(touch) = scene.check_mob_touch(touch_elapsed, touch_protected)
            && let Some(character) = active_character
        {
            let damage = calculate_player_touch_damage(
                character,
                self.inventory.as_ref(),
                touch.damage,
                &self.character_archive,
                &self.skill_archive,
                &self.skills,
                &self.active_buffs,
                &self.forced_equipment_stats,
            );
            let direction = u8::from(touch.origin_x <= scene.player.x);
            let stance = f64::from(character_buff_value(
                &self.active_buffs,
                openstory_protocol::BuffMaskGroup::Second,
                0x10,
            )) / 100.0;
            let knockback =
                damage > 0 && scene.local_ladder.is_none() && scene.next_attack_random() > stance;
            self.pending_zombify_heal_feedback_until = None;
            scene.show_local_player_damage(damage);
            if damage > 0 {
                self.audio.play_world_sound(
                    &self.sound_archive,
                    "Game.img/Damage",
                    (rounded_i16(scene.player.x), rounded_i16(scene.player.y)),
                );
            }
            if knockback {
                scene.player.horizontal_speed = if direction == 0 { -1.5 } else { 1.5 };
                scene.player.vertical_force -= 3.5;
            }
            let timestamp = self.client_started_at.elapsed().as_millis() as u32;
            let packet = openstory_protocol::take_damage_packet(
                timestamp,
                damage,
                touch.mob_id,
                touch.object_id,
                direction,
            );
            if let Some(session) = self.session.as_ref()
                && let Err(error) = session.send_packet(packet)
            {
                eprintln!("OpenStory: could not report touch damage: {error}");
                self.channel_connected = false;
            }
        }

        match scene.advance_portals(
            self.gameplay_input.up,
            scene.attack_stance.is_none()
                && !scene.is_locally_seated()
                && scene.local_ladder.is_none(),
            frame.updates,
        ) {
            Some(PortalWarp::SameMap { x, y }) => {
                if self.gameplay_input.up {
                    self.gameplay_input.suppress_up_until_release = true;
                    self.gameplay_input.up = false;
                }
                self.audio.play_world_sound(
                    &self.sound_archive,
                    "Game.img/Portal",
                    (rounded_i16(scene.player.x), rounded_i16(scene.player.y)),
                );
                scene.respawn_at_portal(x, y);
            }
            Some(PortalWarp::ChangeMap {
                map_id,
                portal_name,
            }) => {
                if self.gameplay_input.up {
                    self.gameplay_input.suppress_up_until_release = true;
                    self.gameplay_input.up = false;
                }
                self.audio.play_world_sound(
                    &self.sound_archive,
                    "Game.img/Portal",
                    (rounded_i16(scene.player.x), rounded_i16(scene.player.y)),
                );
                match openstory_protocol::change_map_packet(false, -1, &portal_name, false) {
                    Ok(packet) => {
                        if let Some(session) = self.session.as_ref() {
                            if let Err(error) = session.send_packet(packet) {
                                eprintln!("OpenStory: could not request portal warp: {error}");
                                self.channel_connected = false;
                            }
                        } else {
                            eprintln!(
                                "OpenStory: could not request portal warp to map {map_id}: no active session"
                            );
                        }
                    }
                    Err(error) => {
                        eprintln!("OpenStory: could not encode portal warp request: {error}")
                    }
                }
            }
            None => {}
        }

        let fallback_motion = if let Some(ladder) = scene.local_ladder {
            if ladder.is_ladder {
                AvatarMotion::Ladder
            } else {
                AvatarMotion::Rope
            }
        } else if scene.player.mode == PhysicsMode::Swimming && !scene.player.on_ground {
            AvatarMotion::Swim
        } else if scene.player.on_ground && self.gameplay_input.down {
            AvatarMotion::Crouch
        } else if !scene.player.on_ground {
            AvatarMotion::Jump
        } else if scene.is_locally_seated() {
            AvatarMotion::Sit
        } else if self.gameplay_input.left != self.gameplay_input.right {
            AvatarMotion::Walk
        } else {
            AvatarMotion::Stand
        };
        let motion = scene
            .attack_stance
            .map(AvatarMotion::Attack)
            .unwrap_or(fallback_motion);
        let animation_speed = if scene.local_ladder.is_some() {
            scene.player.vertical_speed.abs()
        } else {
            1.0
        };
        scene.advance_avatar_animation(
            Duration::from_secs_f64(0.008 * f64::from(frame.updates) * animation_speed),
            motion,
            fallback_motion,
            &mut self.sprite_draws,
        );
        self.audio
            .set_listener((rounded_i16(scene.player.x), rounded_i16(scene.player.y)));
        let camera_delta = scene.update_camera();
        scene.advance_background_positions(
            camera_delta.0,
            camera_delta.1,
            frame.updates,
            &mut self.sprite_draws,
        );
        if camera_delta.0 == 0.0 && camera_delta.1 == 0.0 {
            return;
        }
        for index in &scene.world_draw_indices {
            if let Some((_, rect)) = self.sprite_draws.get_mut(*index) {
                rect.x += camera_delta.0;
                rect.y += camera_delta.1;
            }
        }
    }

    pub(super) fn stand_from_local_seat(&mut self) -> bool {
        let Some(scene) = self.map_scene.as_mut() else {
            return false;
        };
        let was_sitting_in_chair = scene.local_chair.take().is_some();
        let was_sitting_on_map_seat = scene.local_map_seat.take().is_some();
        if !was_sitting_in_chair && !was_sitting_on_map_seat {
            return false;
        }
        scene.player.horizontal_speed = 0.0;
        scene.player.vertical_speed = 0.0;
        scene.player.on_ground = true;
        scene.last_movement = None;
        scene.desired_avatar_motion = AvatarMotion::Stand;
        scene.active_avatar_frame = first_avatar_frame(&scene.avatar_frames, AvatarMotion::Stand);
        scene.avatar_elapsed = Duration::ZERO;
        scene.refresh_avatar_draws(&mut self.sprite_draws);
        if was_sitting_in_chair
            && self.channel_connected
            && let Some(session) = self.session.as_ref()
            && let Err(error) = session.send_packet(openstory_protocol::cancel_chair_packet())
        {
            eprintln!("OpenStory: could not cancel chair use: {error}");
            self.channel_connected = false;
        }
        true
    }

    pub(super) fn try_local_basic_attack(&mut self) {
        self.try_local_attack(None);
    }

    pub(super) fn try_local_skill_attack(&mut self, skill: &SkillUiEntry) {
        self.try_local_attack(Some(skill));
    }

    pub(super) fn try_local_attack(&mut self, skill: Option<&SkillUiEntry>) {
        let skill_id = skill.map_or(0, |skill| skill.id);
        if self
            .map_scene
            .as_ref()
            .is_some_and(|scene| scene.local_ladder.is_some())
        {
            return;
        }
        let now = Instant::now();
        self.active_buffs.expire(now);
        if character_has_buff(
            &self.active_buffs,
            openstory_protocol::BuffMaskGroup::Second,
            STATUS_STUN,
        ) {
            return;
        }
        if self
            .map_scene
            .as_ref()
            .is_some_and(MapSceneRuntime::is_locally_seated)
        {
            self.stand_from_local_seat();
        }
        let Some(character) = self.active_character.clone() else {
            return;
        };
        let mut attack_stats = match basic_attack_stats(
            &character,
            self.inventory.as_ref(),
            &self.character_archive,
            &self.item_archive,
            &self.skill_archive,
            &self.skills,
            &self.active_buffs,
            &self.forced_equipment_stats,
        ) {
            Ok(Some(stats)) => stats,
            Ok(None) => return,
            Err(error) => {
                eprintln!("OpenStory: could not prepare a basic attack: {error}");
                return;
            }
        };
        if skill.is_some() && matches!(attack_stats.weapon_type, 137 | 138) {
            attack_stats.attack_type = openstory_protocol::AttackType::Magic;
        }
        if let Some(skill) = skill {
            attack_stats.magic_damage = skill.magic_damage;
            attack_stats.fixed_damage = skill.fixed_damage;
            // Upstream selects DMG_MAGIC when the level has `mad`, but its
            // attack range still comes from CharStats' weapon formula. `mad`
            // itself is not folded into damage there.
            if !skill.magic_damage && skill.fixed_damage.is_none() {
                attack_stats.minimum_damage *= skill.damage_multiplier;
                attack_stats.maximum_damage *= skill.damage_multiplier;
            }
        }
        // Upstream prepare_attack reduces degenerate close-range attacks to
        // one tenth. Empty-projectile bow/claw/gun attacks always degenerate;
        // wand/staff regular swings do, while their skills remain magic.
        let degenerate_ranged = attack_stats.attack_type == openstory_protocol::AttackType::Close
            && matches!(attack_stats.weapon_type, 145 | 146 | 147 | 149);
        let degenerate_magic = skill.is_none() && matches!(attack_stats.weapon_type, 137 | 138);
        if degenerate_ranged || degenerate_magic {
            attack_stats.minimum_damage /= 10.0;
            attack_stats.maximum_damage /= 10.0;
        }
        let hit_count = skill.map_or(1, |skill| {
            if attack_stats.attack_type == openstory_protocol::AttackType::Ranged {
                skill.bullet_count
            } else {
                skill.attack_count
            }
        });
        let target_count = skill.map_or(1, |skill| usize::from(skill.mob_count));
        let horizontal_range_multiplier =
            skill.map_or(1.0, |skill| skill.horizontal_range_multiplier);
        let horizontal_range_bonus = if attack_stats.attack_type
            == openstory_protocol::AttackType::Ranged
        {
            projectile_range_extension(&self.skill_archive, &self.skills, attack_stats.weapon_type)
        } else {
            0
        };
        let skill_level = skill.map_or(0, |skill| skill.level.clamp(0, 255) as u8);
        let skill_projectile_frames = if skill_id > 0 {
            match self.load_skill_projectile_frames(skill_id, skill_level) {
                Ok(frames) => frames,
                Err(error) => {
                    eprintln!("OpenStory: could not load skill {skill_id} projectile: {error}");
                    None
                }
            }
        } else {
            None
        };
        let projectile_frames = match skill_projectile_frames {
            Some(frames) => Some(frames),
            None => attack_stats.projectile_id.and_then(|item_id| {
                match self.load_projectile_frames(item_id) {
                    Ok(frames) => Some(frames),
                    Err(error) => {
                        eprintln!("OpenStory: could not load projectile {item_id}: {error}");
                        None
                    }
                }
            }),
        };
        if self
            .map_scene
            .as_ref()
            .is_none_or(|scene| scene.attack_stance.is_some())
        {
            return;
        }
        let skill_stance = (skill_id > 0)
            .then(|| {
                character_skill_action_stance(
                    &self.skill_archive,
                    &character.look,
                    skill_id,
                    skill_level,
                )
                .ok()
                .flatten()
            })
            .flatten();
        let two_handed = character_weapon_id(&character.look).is_some_and(is_two_handed_weapon);
        let skill_effects = if skill_id > 0 {
            match self.load_character_skill_effects(
                skill_id,
                skill_level,
                character.stats.level,
                two_handed,
            ) {
                Ok(animations) => animations,
                Err(error) => {
                    eprintln!("OpenStory: could not load skill {skill_id} effect: {error}");
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        let skill_hit_effects = if skill_id > 0 {
            match self.load_skill_hit_effects(
                skill_id,
                skill_level,
                character.stats.level,
                two_handed,
            ) {
                Ok(animations) => animations,
                Err(error) => {
                    eprintln!("OpenStory: could not load skill {skill_id} hit effect: {error}");
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        let skill_hit_effects = Arc::new(skill_hit_effects);
        let skill_effect_speed_factor =
            (1.7 - f32::from(attack_stats.speed) / 10.0).clamp(0.2, 3.0);
        let Some(scene) = self.map_scene.as_mut() else {
            return;
        };
        let attack_index = scene.attack_count;
        let Some(stance) = skill_stance.or_else(|| {
            regular_attack_stance(
                attack_stats.attack_class,
                attack_stats.weapon_type,
                attack_stats.attack_type == openstory_protocol::AttackType::Ranged,
                attack_index,
            )
        }) else {
            return;
        };
        let attack_range = skill
            .and_then(|skill| skill.attack_range)
            .or_else(|| {
                let regular_action = skill_id == 0
                    || character_skill_action_name(
                        &self.skill_archive,
                        &character.look,
                        skill_id,
                        skill_level,
                    )
                    .ok()
                    .flatten()
                    .is_none();
                (regular_action
                    && attack_stats.attack_type == openstory_protocol::AttackType::Close)
                    .then(|| {
                        normal_weapon_attack_range(&self.character_archive, &character.look, stance)
                    })
                    .flatten()
            })
            .unwrap_or([-400, -5, -50, 50]);
        let skill_hit_delays = attack_hit_delays(
            &self.character_archive,
            &self.skill_archive,
            &character.look,
            skill_id,
            skill_level,
            stance,
            attack_stats.speed,
            usize::from(hit_count.clamp(1, 15)),
        );
        let target_positions = scene.nearest_attack_targets(
            target_count,
            attack_range,
            horizontal_range_multiplier,
            horizontal_range_bonus,
            attack_stats.attack_type == openstory_protocol::AttackType::Ranged,
        );
        let shows_projectile = projectile_frames
            .as_ref()
            .is_some_and(|frames| !frames.is_empty())
            && (attack_stats.attack_type == openstory_protocol::AttackType::Ranged || skill_id > 0);
        let mut mob_hit_sound_events = Vec::new();
        let mut skill_hit_sound_events = Vec::new();
        for (object_id, _, (target_x, target_y)) in &target_positions {
            let Some(mob_id) = scene.mobs.get(object_id).map(|mob| mob.mob_id) else {
                continue;
            };
            let position = (rounded_i16(*target_x), rounded_i16(*target_y));
            let projectile_travel = if shows_projectile {
                projectile_flight_duration(scene.player.x, *target_x)
            } else {
                Duration::ZERO
            };
            for hit_index in 0..hit_count.clamp(1, 15) {
                let delay =
                    skill_hit_delays[usize::from(hit_index)].saturating_add(projectile_travel);
                mob_hit_sound_events.push((*object_id, mob_id, position, delay));
                if skill_id > 0 {
                    skill_hit_sound_events.push((*object_id, position, delay));
                }
            }
        }
        let projectile_targets = target_positions
            .iter()
            .map(|(_, _, (x, y))| (*x, *y - 24.0))
            .take(15)
            .collect::<Vec<_>>();
        let reactor_target = scene.nearest_reactor_target(
            attack_range,
            horizontal_range_multiplier,
            horizontal_range_bonus,
        );
        let reactor_target_position = reactor_target.and_then(|object_id| {
            scene
                .reactors
                .get(&object_id)
                .map(|reactor| (reactor.x, reactor.y))
        });
        // The server broadcasts attacks to other players, so local damage must
        // be presented from the same rolls used in the outgoing packet.
        let mut impacts = Vec::new();
        let targets = target_positions
            .into_iter()
            .map(|(object_id, mob, (target_x, _))| {
                let projectile_travel = if shows_projectile {
                    projectile_flight_duration(scene.player.x, target_x)
                } else {
                    Duration::ZERO
                };
                let mut stack_offset = 0.0;
                let damage_lines: Vec<i32> = (0..hit_count.clamp(1, 15))
                    .map(|hit_index| {
                        let (damage, critical) = basic_attack_damage(scene, attack_stats, mob);
                        impacts.push((
                            openstory_protocol::MobDamage { object_id, damage },
                            critical,
                            stack_offset,
                            skill_hit_delays[usize::from(hit_index)]
                                .saturating_add(projectile_travel),
                        ));
                        stack_offset += if critical { 36.0 } else { 30.0 };
                        damage
                    })
                    .collect();
                if damage_lines.iter().any(|damage| *damage > 0) {
                    scene.mark_local_mob_aggro(object_id);
                }
                openstory_protocol::AttackDamageTarget {
                    object_id,
                    damage_lines,
                }
            })
            .collect();
        let packet = match openstory_protocol::build_player_attack_packet(
            &openstory_protocol::PlayerAttackPacket {
                attack_type: attack_stats.attack_type,
                hit_count: hit_count.clamp(1, 15),
                skill_id,
                charge: 0,
                display: 0,
                to_left: !scene.facing_right,
                stance,
                speed: attack_stats.speed,
                targets,
            },
        ) {
            Ok(packet) => packet,
            Err(error) => {
                eprintln!("OpenStory: could not encode a basic attack: {error}");
                return;
            }
        };
        if skill_id > 0 {
            scene.show_character_skill_effect(None, skill_effects, skill_effect_speed_factor);
            self.audio.play_skill_use(
                &self.sound_archive,
                skill_id,
                (rounded_i16(scene.player.x), rounded_i16(scene.player.y)),
            );
        }
        scene.start_attack(stance, &mut self.sprite_draws);
        let regular_action = skill_id == 0
            || character_skill_action_name(
                &self.skill_archive,
                &character.look,
                skill_id,
                skill_level,
            )
            .ok()
            .flatten()
            .is_none();
        if regular_action && let Some(weapon_id) = character_weapon_id(&character.look) {
            let degenerate = attack_stats.attack_type == openstory_protocol::AttackType::Close
                && matches!(attack_stats.weapon_type, 145 | 146 | 147 | 149);
            self.audio.play_weapon_attack(
                &self.character_archive,
                &self.sound_archive,
                weapon_id,
                degenerate,
                (rounded_i16(scene.player.x), rounded_i16(scene.player.y)),
            );
        }
        if let Some(frames) = projectile_frames {
            if attack_stats.attack_type == openstory_protocol::AttackType::Ranged || skill_id > 0 {
                if projectile_targets.is_empty() {
                    let (target_x, target_y) = reactor_target_position.unwrap_or_else(|| {
                        (
                            scene.player.x + if scene.facing_right { 400.0 } else { -400.0 },
                            scene.player.y - 26.0,
                        )
                    });
                    for hit_index in 0..hit_count.clamp(1, 15) {
                        scene.launch_projectile(
                            frames.clone(),
                            target_x,
                            target_y,
                            skill_hit_delays[usize::from(hit_index)],
                        );
                    }
                } else {
                    for (target_x, target_y) in projectile_targets {
                        for hit_index in 0..hit_count.clamp(1, 15) {
                            scene.launch_projectile(
                                frames.clone(),
                                target_x,
                                target_y,
                                skill_hit_delays[usize::from(hit_index)],
                            );
                        }
                    }
                }
            }
        }
        for (damage, critical, stack_offset, delay) in impacts {
            scene.schedule_mob_attack_impact(
                damage.object_id,
                Arc::clone(&skill_hit_effects),
                !scene.facing_right,
                Some((damage, critical, stack_offset)),
                shows_projectile,
                delay,
            );
        }
        if self.channel_connected {
            if let Some(session) = self.session.as_ref() {
                match session.send_packet(packet) {
                    Err(error) => {
                        eprintln!("OpenStory: could not send a basic attack: {error}");
                        self.channel_connected = false;
                    }
                    Ok(()) => {
                        if skill_id > 0 {
                            self.last_local_skill_cast = Some((skill_id, Instant::now()));
                        }
                        if let Some(object_id) = reactor_target {
                            let stance = if matches!(stance, 0 | 2) {
                                if scene.facing_right { 3 } else { 1 }
                            } else {
                                stance
                            };
                            let packet = openstory_protocol::damage_reactor_packet(
                                object_id,
                                rounded_i16(scene.player.x),
                                rounded_i16(scene.player.y),
                                i16::from(stance),
                                0,
                            );
                            if let Err(error) = session.send_packet(packet) {
                                eprintln!("OpenStory: could not request reactor hit: {error}");
                                self.channel_connected = false;
                            }
                        }
                    }
                }
            }
        }
        self.schedule_mob_hit_sounds(&mob_hit_sound_events);
        if skill_id > 0 {
            self.schedule_skill_hit_sounds(skill_id, &skill_hit_sound_events);
        }
    }

    pub(super) fn toggle_key_config(&mut self) {
        if self.show_key_config {
            self.close_key_config(false);
            return;
        }
        self.show_key_config = true;
        self.gameplay_input = GameplayInput::default();
        self.quickslot_pointer_binding = None;
        self.key_config_pointer_start = None;
        self.chat_open = false;
        self.chat_input_focused = false;
        self.whisper_input_focused = false;
        self.key_config_capture = None;
        self.quickslot_key_capture = None;
        if self.show_key_config {
            self.key_config_draft = Some(
                self.server_keymap
                    .clone()
                    .unwrap_or_else(default_keymap_bindings),
            );
            self.key_config_quickslot_draft = Some(self.quickslot_keys);
            self.key_config_scroll_row = 0;
            self.key_config_actions_dirty = true;
            self.refresh_key_config_texture();
        }
    }

    pub(super) fn toggle_quickslots(&mut self) {
        if self.current_field.is_none() {
            return;
        }
        self.show_quickslots = !self.show_quickslots;
        if self.show_quickslots {
            self.refresh_quickslots_texture();
        }
        self.update_inventory_hover();
        self.refresh_inventory_tooltip_texture();
    }

    pub(super) fn toggle_game_menu(&mut self) {
        self.show_game_menu = !self.show_game_menu;
        if self.show_game_menu {
            self.refresh_game_menu_texture();
        }
    }

    pub(super) fn refresh_game_menu_texture(&mut self) {
        if !self.show_game_menu {
            return;
        }
        let pixels = match render_game_menu(&self.ui_archive, &self.font) {
            Ok(pixels) => pixels,
            Err(error) => {
                eprintln!("OpenStoryRS: could not render the game menu: {error}");
                return;
            }
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.game_menu_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(GAME_MENU_WIDTH, GAME_MENU_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.game_menu_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update the game menu: {error}"),
        }
    }

    pub(super) fn set_combo_count(&mut self, count: i32) {
        self.combo_count = count.max(0);
    }

    pub(super) fn apply_skill_cooldown(&mut self, skill_id: i32, duration_seconds: i16) {
        if duration_seconds <= 0 {
            self.skill_cooldowns.remove(&skill_id);
        } else if let Some(expires_at) =
            Instant::now().checked_add(Duration::from_secs(duration_seconds as u64))
        {
            self.skill_cooldowns.insert(skill_id, expires_at);
        }
    }

    fn advance_combo_counter(&mut self, elapsed: Duration) {
        if self.combo_count != self.combo_displayed {
            self.combo_displayed = self.combo_count;
            self.combo_fade_remaining = Duration::from_secs(3);
            self.refresh_combo_counter_texture();
        } else {
            self.combo_fade_remaining = self.combo_fade_remaining.saturating_sub(elapsed);
        }
    }

    fn refresh_combo_counter_texture(&mut self) {
        let pixels = render_combo_counter(&self.font, self.combo_displayed);
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.combo_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(COMBO_COUNTER_WIDTH, COMBO_COUNTER_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.combo_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update the combo counter: {error}"),
        }
    }

    pub(super) fn open_inventory_tab(&mut self, tab: openstory_protocol::InventoryType) {
        if self.show_inventory && self.inventory_tab == tab {
            self.show_inventory = false;
            self.inventory_drag_source = None;
            self.inventory_hover_slot = None;
            self.pending_scroll_slot = None;
            return;
        }
        self.show_inventory = true;
        self.inventory_tab = tab;
        self.inventory_scroll_row = 0;
        self.inventory_drag_source = None;
        self.inventory_hover_slot = None;
        self.pending_scroll_slot = None;
        self.update_inventory_hover();
        self.refresh_inventory_texture();
    }

    pub(super) fn toggle_equipment_inventory(&mut self) {
        self.show_equipment_inventory = !self.show_equipment_inventory;
        if self.show_equipment_inventory {
            self.refresh_equipment_inventory_texture();
        }
    }

    pub(super) fn activate_game_menu_item(&mut self, item: usize) {
        self.show_game_menu = false;
        match item {
            0 => self.open_channel_switch(),
            1 => self.open_settings(),
            2 => self.open_quit_confirmation(),
            _ => {}
        }
    }

    pub(super) fn game_menu_position(&self) -> (f32, f32) {
        game_menu_position_for_layout(self.status_bar_layout)
    }

    pub(super) fn open_quest_log(&mut self) {
        self.show_quest_log = true;
        self.quest_log_tab = QuestLogTab::Active;
        self.quest_log_detail_open = true;
        self.quest_log_scroll = 0;
        self.quest_log_selected = 0;
        self.quest_reward_selected = 0;
        self.quest_forfeit_confirmation = None;
        self.refresh_quest_log_texture();
    }

    pub(super) fn click_native_status_bar(&mut self, cursor: Point) -> bool {
        if self.current_field.is_none() {
            return false;
        }

        if self.show_game_menu {
            let (menu_x, menu_y) = self.game_menu_position();
            let menu_bounds = SpriteRect {
                x: menu_x,
                y: menu_y,
                width: GAME_MENU_WIDTH as f32,
                height: GAME_MENU_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            };
            if rect_contains(menu_bounds, cursor) {
                let local_x = cursor.x - menu_x;
                let local_y = cursor.y - menu_y;
                if GAME_MENU_CLOSE.contains(local_x, local_y) {
                    self.audio
                        .play_ui_sound(&self.sound_archive, "UI.img/BtMouseClick");
                    self.show_game_menu = false;
                } else if let Some(row) = (0..GAME_MENU_ROWS.len())
                    .find(|index| game_menu_row(*index).contains(local_x, local_y))
                {
                    self.audio
                        .play_ui_sound(&self.sound_archive, "UI.img/BtMouseClick");
                    self.activate_game_menu_item(row);
                }
                return true;
            }
            self.show_game_menu = false;
            self.game_menu_texture = None;
            let (button_x, button_width) = match self.status_bar_layout {
                StatusBarLayout::Modern => (794.0, 54.0),
                StatusBarLayout::V83 => (
                    V83_STATUS_BAR_BIG_BUTTON_X + V83_STATUS_BAR_BIG_BUTTON_STEP,
                    V83_STATUS_BAR_BIG_BUTTON_WIDTH,
                ),
            };
            let button_y = STATUS_BAR_TOP + 35.0;
            if rect_contains(
                SpriteRect {
                    x: button_x,
                    y: button_y,
                    width: button_width,
                    height: 34.0,
                    blend_mode: BlendMode::Alpha,
                },
                cursor,
            ) {
                return true;
            }
        }

        let small_x = match self.status_bar_layout {
            StatusBarLayout::Modern => 838.0,
            StatusBarLayout::V83 => {
                V83_STATUS_BAR_WIDTH - V83_STATUS_BAR_EDGE_PAD - (6.0 * 28.0 + 5.0 * 2.0)
            }
        };
        let small_y = STATUS_BAR_TOP + 6.0;
        if (small_y..small_y + 20.0).contains(&cursor.y) {
            let icon = (0..6).find(|index| {
                let left = small_x + *index as f32 * 30.0;
                (left..left + 28.0).contains(&cursor.x)
            });
            match icon {
                Some(0..=5) => {
                    self.audio
                        .play_ui_sound(&self.sound_archive, "UI.img/BtMouseClick");
                    match icon {
                        Some(0) => self.open_status_window(),
                        Some(1) => self.open_inventory_tab(openstory_protocol::InventoryType::Use),
                        Some(2) => {
                            self.open_inventory_tab(openstory_protocol::InventoryType::Equip)
                        }
                        Some(3) => {
                            self.show_skill_book = !self.show_skill_book;
                            if self.show_skill_book {
                                self.skill_book_tab = 0;
                                self.skill_book_scroll_row = 0;
                                self.refresh_skill_book_entries();
                                self.refresh_skill_book_texture();
                            }
                        }
                        Some(4) => self.toggle_key_config(),
                        Some(5) => self.toggle_quickslots(),
                        _ => {}
                    }
                }
                _ => return false,
            }
            return true;
        }

        let v83 = self.status_bar_layout == StatusBarLayout::V83;
        let big_x = if v83 {
            V83_STATUS_BAR_BIG_BUTTON_X
        } else {
            794.0
        };
        let big_y = STATUS_BAR_TOP + 35.0;
        let big_button_width = if v83 {
            V83_STATUS_BAR_BIG_BUTTON_WIDTH
        } else {
            54.0
        };
        let big_button_step = if v83 {
            V83_STATUS_BAR_BIG_BUTTON_STEP
        } else {
            56.0
        };
        let big_button_count = if v83 { 3 } else { 4 };
        if (big_y..big_y + 34.0).contains(&cursor.y) {
            let button = (0..big_button_count).find(|index| {
                let left = big_x + *index as f32 * big_button_step;
                (left..left + big_button_width).contains(&cursor.x)
            });
            match button {
                Some(_) => {
                    self.audio
                        .play_ui_sound(&self.sound_archive, "UI.img/BtMouseClick");
                    if v83 {
                        match button {
                            Some(0) => self.open_cash_shop(),
                            Some(1) => self.toggle_game_menu(),
                            Some(2) => self.toggle_key_config(),
                            _ => {}
                        }
                    } else {
                        match button {
                            Some(0) => self.toggle_game_menu(),
                            Some(1) => self.open_cash_shop(),
                            Some(2) => self.open_settings(),
                            Some(3) => {
                                self.chat_target = ChatTarget::Public;
                                self.chat_input = "/trade ".to_owned();
                                self.chat_open = true;
                                self.chat_input_focused = true;
                                self.refresh_chat_texture();
                            }
                            _ => {}
                        }
                    }
                }
                _ => return false,
            }
            return true;
        }

        false
    }

    pub(super) fn quickslot_at(&self, cursor: Point) -> Option<usize> {
        if !self.show_quickslots {
            return None;
        }
        ui_foundation::QUICKSLOT_GRID.hit(cursor.x - 865.0, cursor.y - 615.0)
    }

    pub(super) fn current_quickslot_binding(
        &self,
        slot: usize,
    ) -> Option<openstory_protocol::KeyBinding> {
        let key = *self.current_quickslot_keys().get(slot)?;
        if let Some(bindings) = self
            .key_config_draft
            .as_deref()
            .or(self.server_keymap.as_deref())
        {
            return bindings.get(usize::from(key)).copied();
        }
        default_keymap_bindings().get(usize::from(key)).copied()
    }

    pub(super) fn click_quickslot(&mut self, cursor: Point) -> bool {
        let Some(slot) = self.quickslot_at(cursor) else {
            return false;
        };
        let key = self.current_quickslot_keys()[slot];
        let fallback = default_keymap_bindings();
        if let Some(binding) = self
            .key_config_draft
            .as_deref()
            .or(self.server_keymap.as_deref())
            .unwrap_or(&fallback)
            .get(usize::from(key))
            .copied()
            && binding.kind != 0
        {
            self.apply_keymap_binding(ElementState::Pressed, false, binding);
            if binding.kind == 5 && matches!(binding.action, 51 | 200..=203) {
                self.quickslot_pointer_binding = Some(binding);
            }
        }
        true
    }

    pub(super) fn finish_quickslot_pointer(&mut self) {
        if let Some(binding) = self.quickslot_pointer_binding.take()
            && !self
                .gameplay_input
                .held_bindings
                .values()
                .any(|held| *held == binding)
        {
            self.apply_keymap_binding(ElementState::Released, false, binding);
        }
    }

    pub(super) fn quickslot_icon_pixels(
        &mut self,
        binding: openstory_protocol::KeyBinding,
    ) -> Option<(u32, u32, Arc<[u8]>)> {
        if binding.action < 0 || (binding.action == 0 && binding.kind != 8) {
            return None;
        }
        let cache_key = (binding.kind, binding.action);
        if let Some(cached) = self.quickslot_icon_cache.get(&cache_key) {
            return cached.clone();
        }
        let icon = match binding.kind {
            1 => decode_nx_icon(
                &self.skill_archive,
                &format!(
                    "{:03}.img/skill/{:07}/icon",
                    binding.action / 10_000,
                    binding.action
                ),
            ),
            2 | 3 => inventory_item_icon(
                binding.action,
                &self.item_archive,
                &self.character_archive,
                &mut self.inventory_icon_cache,
            ),
            4 | 5 => {
                let path = format!("UIWindow.img/KeyConfig/icon/{}", binding.action);
                decode_nx_icon(&self.ui_archive, &path).or_else(|| {
                    decode_nx_icon(
                        &self.ui_archive,
                        &format!("UIWindow2.img/KeyConfig/icon/{}", binding.action),
                    )
                })
            }
            8 => decode_nx_icon(
                &self.ui_archive,
                &format!("UIWindow.img/SkillMacro/Macroicon/{}/icon", binding.action),
            ),
            _ => None,
        };
        if self.quickslot_icon_cache.len() >= 256 {
            self.quickslot_icon_cache.clear();
        }
        self.quickslot_icon_cache.insert(cache_key, icon.clone());
        icon
    }

    pub(super) fn refresh_quickslots_texture(&mut self) {
        if !self.show_quickslots {
            return;
        }
        const WIDTH: usize = 151;
        const HEIGHT: usize = 80;
        let fallback = default_keymap_bindings();
        let bindings = self
            .key_config_draft
            .as_deref()
            .or(self.server_keymap.as_deref())
            .unwrap_or(&fallback);
        let quickslot_keys = self.current_quickslot_keys();
        let slot_bindings = quickslot_keys
            .iter()
            .map(|key| bindings.get(usize::from(*key)).copied().unwrap_or_default())
            .collect::<Vec<_>>();
        let item_counts = self.current_quickslot_item_counts();
        let icons: [Option<(u32, u32, Arc<[u8]>)>; 8] = std::array::from_fn(|slot| {
            let binding = slot_bindings.get(slot).copied().unwrap_or_default();
            (binding.kind != 0)
                .then(|| self.quickslot_icon_pixels(binding))
                .flatten()
        });
        let pixels = render_quickslot_panel_pixels(
            quickslot_background_image(&self.legacy_ui_archive),
            &self.ui_archive,
            &quickslot_keys,
            &icons,
            &item_counts,
        );

        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.quickslot_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(WIDTH as u32, HEIGHT as u32, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => {
                self.quickslot_texture = Some(texture);
                self.quickslot_rendered_counts = Some(item_counts);
            }
            Ok(None) => self.quickslot_rendered_counts = Some(item_counts),
            Err(error) => eprintln!("OpenStoryRS: could not refresh quick slots: {error}"),
        }
        self.refresh_inventory_tooltip_texture();
    }

    fn current_quickslot_item_counts(&self) -> [u32; 8] {
        let fallback = default_keymap_bindings();
        let bindings = self
            .key_config_draft
            .as_deref()
            .or(self.server_keymap.as_deref())
            .unwrap_or(&fallback);
        let inventory = self.inventory.as_ref();
        let quickslot_keys = self.current_quickslot_keys();
        std::array::from_fn(|slot| {
            let binding = bindings
                .get(usize::from(quickslot_keys[slot]))
                .copied()
                .unwrap_or_default();
            if matches!(binding.kind, 2 | 3) {
                inventory.map_or(0, |inventory| inventory.item_count(binding.action))
            } else {
                0
            }
        })
    }

    pub(super) fn refresh_quickslot_counts_if_changed(&mut self) {
        if !self.show_quickslots {
            return;
        }
        let counts = self.current_quickslot_item_counts();
        if self.quickslot_texture.is_none() || self.quickslot_rendered_counts != Some(counts) {
            self.refresh_quickslots_texture();
        }
    }

    pub(super) fn refresh_key_config_texture(&mut self) {
        if !self.show_key_config {
            return;
        }
        if self.key_config_actions_dirty {
            let captured_action = self.key_config_capture.and_then(|index| {
                self.key_config_actions
                    .get(index)
                    .map(|entry| (entry.kind, entry.action))
            });
            self.key_config_actions = build_key_config_actions(
                self.inventory.as_ref(),
                &self.skills,
                self.key_config_draft
                    .as_deref()
                    .or(self.server_keymap.as_deref()),
                &self.skill_macros,
                &self.string_archive,
                &mut self.item_name_cache,
            );
            self.key_config_capture = captured_action.and_then(|(kind, action)| {
                self.key_config_actions
                    .iter()
                    .position(|entry| entry.kind == kind && entry.action == action)
            });
            self.key_config_actions_dirty = false;
        }
        let fallback = default_keymap_bindings();
        let bindings = self
            .key_config_draft
            .as_deref()
            .or(self.server_keymap.as_deref())
            .unwrap_or(&fallback);
        self.key_config_scroll_row = self
            .key_config_scroll_row
            .min(keyboard_settings::scroll_max(
                &self.key_config_actions,
                bindings,
            ));
        let mut counts = HashMap::new();
        if let Some(inventory) = self.inventory.as_ref() {
            for item in &inventory.items {
                *counts.entry(item.item_id).or_insert(0_u32) += u32::from(item.count);
            }
        }
        let pixels = render_key_config_panel(
            &self.font,
            &self.ui_archive,
            &self.skill_archive,
            &self.item_archive,
            &self.character_archive,
            bindings,
            self.key_config_quickslot_draft
                .as_ref()
                .unwrap_or(&self.quickslot_keys),
            self.key_config_capture,
            self.quickslot_key_capture,
            self.key_config_scroll_row,
            &self.key_config_actions,
            &mut self.inventory_icon_cache,
            &counts,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.key_config_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(KEY_CONFIG_PANEL_WIDTH, KEY_CONFIG_PANEL_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.key_config_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStory: could not update key bindings panel: {error}"),
        }
    }

    pub(super) fn skill_macro_choices(&self) -> Vec<(i32, String)> {
        self.skills
            .values()
            .filter(|skill| skill.id > 0 && skill.level > 0 && (skill.id % 10_000) / 1_000 != 0)
            .map(|skill| {
                (
                    skill.id,
                    key_config_skill_name(skill.id, &self.string_archive),
                )
            })
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .collect()
    }

    pub(super) fn refresh_skill_macro_texture(&mut self) {
        if !self.skill_macro_editor.open {
            return;
        }
        let skills = self.skill_macro_choices();
        self.skill_macro_editor.scroll_skills(0, skills.len());
        let pixels = skill_macros::render(
            &self.font,
            &self.ui_archive,
            &self.skill_archive,
            &self.skill_macro_editor,
            &skills,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.skill_macro_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(
                    skill_macros::PANEL_WIDTH,
                    skill_macros::PANEL_HEIGHT,
                    &pixels,
                )
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.skill_macro_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update skill macros: {error}"),
        }
    }

    pub(super) fn open_skill_macro_editor(&mut self) {
        self.skill_macro_editor.open(&self.skill_macros);
        self.refresh_skill_macro_texture();
    }

    fn begin_macro_skill_drag(&mut self, skill_id: i32, cursor: Point, assign_on_click: bool) {
        self.skill_macro_editor.begin_skill_drag(
            skill_id,
            cursor.x - skill_macros::PANEL_X,
            cursor.y - skill_macros::PANEL_Y,
            assign_on_click,
        );
        self.skill_macro_drag_texture = decode_nx_icon(
            &self.skill_archive,
            &format!("{:03}.img/skill/{skill_id:07}/icon", skill_id / 10_000),
        )
        .and_then(|(width, height, pixels)| {
            self.renderer
                .as_ref()
                .and_then(|renderer| renderer.upload_bgra(width, height, &pixels).ok())
        });
    }

    pub(super) fn click_skill_macro_editor(&mut self, cursor: Point) {
        let bounds = SpriteRect {
            x: skill_macros::PANEL_X,
            y: skill_macros::PANEL_Y,
            width: skill_macros::PANEL_WIDTH as f32,
            height: skill_macros::PANEL_HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        };
        let skills = self.skill_macro_choices();
        if !rect_contains(bounds, cursor) {
            let book_x = cursor.x - SKILL_BOOK_PANEL_X;
            let book_y = cursor.y - SKILL_BOOK_PANEL_Y;
            let in_book = self.show_skill_book
                && (0.0..SKILL_BOOK_PANEL_WIDTH as f32).contains(&book_x)
                && (0.0..SKILL_BOOK_PANEL_HEIGHT as f32).contains(&book_y);
            if in_book
                && (SKILL_BOOK_PANEL_WIDTH as f32 - 24.0..SKILL_BOOK_PANEL_WIDTH as f32)
                    .contains(&book_x)
                && (4.0..25.0).contains(&book_y)
            {
                self.begin_skill_book_pointer(cursor);
                if !self.show_skill_book {
                    self.skill_macro_editor.close();
                    self.skill_macro_drag_texture = None;
                }
                self.refresh_skill_macro_texture();
                return;
            }
            if in_book && (24.0..44.0).contains(&book_y) {
                self.begin_skill_book_pointer(cursor);
                return;
            }
            let skill =
                if in_book && (10.0..42.0).contains(&book_x) && (100.0..260.0).contains(&book_y) {
                    let row = ((book_y - 100.0) / 40.0).floor() as usize;
                    self.skill_book_entries
                        .get(self.skill_book_scroll_row.saturating_add(row))
                        .filter(|entry| entry.level > 0 && !entry.passive)
                        .filter(|entry| skills.iter().any(|(id, _)| *id == entry.id))
                        .map(|entry| entry.id)
                } else {
                    None
                };
            if let Some(skill_id) = skill {
                self.begin_macro_skill_drag(skill_id, cursor, false);
                self.refresh_skill_macro_texture();
            } else if !in_book {
                self.skill_macro_editor.close();
            }
            return;
        }
        let action = skill_macros::hit_test(
            cursor.x - bounds.x,
            cursor.y - bounds.y,
            &self.skill_macro_editor,
            &skills,
        );
        match action {
            Some(skill_macros::Action::Close) => self.skill_macro_editor.close(),
            Some(skill_macros::Action::Save) => self.save_skill_macro_editor(),
            Some(skill_macros::Action::SelectMacro(index)) => {
                self.skill_macro_editor.select_macro(index);
            }
            Some(skill_macros::Action::FocusName) => {
                self.skill_macro_editor.name_focused = true;
            }
            Some(skill_macros::Action::ToggleShout) => {
                self.skill_macro_editor.toggle_shout();
            }
            Some(skill_macros::Action::SelectSlot(row, slot)) => {
                self.skill_macro_editor.select_macro(row);
                self.skill_macro_editor.select_slot(slot);
            }
            Some(skill_macros::Action::ClearSkill(slot)) => {
                self.skill_macro_editor.clear_skill(slot);
            }
            Some(skill_macros::Action::AssignSkill(skill_id)) => {
                self.begin_macro_skill_drag(skill_id, cursor, true);
            }
            None => {}
        }
        self.refresh_skill_macro_texture();
    }

    pub(super) fn save_skill_macro_editor(&mut self) {
        let macros = self.skill_macro_editor.drafts().to_vec();
        let packet = match openstory_protocol::save_skill_macros_packet(&macros) {
            Ok(packet) => packet,
            Err(error) => {
                eprintln!("OpenStoryRS: could not encode skill macros: {error}");
                return;
            }
        };
        let Some(session) = self.session.as_ref() else {
            eprintln!("OpenStoryRS: cannot save skill macros while disconnected");
            return;
        };
        match session.send_packet(packet) {
            Ok(()) => {
                self.skill_macros = macros;
                self.skill_macro_editor.close();
                self.key_config_actions_dirty = true;
                self.refresh_key_config_texture();
            }
            Err(error) => {
                eprintln!("OpenStoryRS: could not save skill macros: {error}");
                self.channel_connected = false;
            }
        }
    }

    pub(super) fn handle_skill_macro_editor_key(
        &mut self,
        key: PhysicalKey,
        state: ElementState,
        repeat: bool,
        text: Option<&str>,
    ) -> bool {
        if !self.skill_macro_editor.open {
            return false;
        }
        if state == ElementState::Pressed {
            match key {
                PhysicalKey::Code(KeyCode::Escape) if !repeat => {
                    self.skill_macro_editor.close();
                }
                PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) if !repeat => {
                    self.save_skill_macro_editor();
                }
                PhysicalKey::Code(KeyCode::Backspace) => {
                    self.skill_macro_editor.backspace();
                }
                PhysicalKey::Code(KeyCode::Tab) if !repeat => {
                    self.skill_macro_editor.name_focused = !self.skill_macro_editor.name_focused;
                }
                PhysicalKey::Code(KeyCode::ArrowUp) if !self.skill_macro_editor.name_focused => {
                    self.skill_macro_editor
                        .scroll_skills(-1, self.skill_macro_choices().len());
                }
                PhysicalKey::Code(KeyCode::ArrowDown) if !self.skill_macro_editor.name_focused => {
                    self.skill_macro_editor
                        .scroll_skills(1, self.skill_macro_choices().len());
                }
                PhysicalKey::Code(KeyCode::PageUp) => {
                    self.skill_macro_editor.scroll_skills(
                        -(skill_macros::VISIBLE_SKILLS as isize),
                        self.skill_macro_choices().len(),
                    );
                }
                PhysicalKey::Code(KeyCode::PageDown) => {
                    self.skill_macro_editor.scroll_skills(
                        skill_macros::VISIBLE_SKILLS as isize,
                        self.skill_macro_choices().len(),
                    );
                }
                _ => {}
            }
            if self.skill_macro_editor.open
                && let Some(text) =
                    text.filter(|text| text.chars().any(|character| !character.is_control()))
            {
                self.skill_macro_editor.insert_text(text);
                self.skill_macro_editor.key_text_for_ime_dedup = Some(text.to_owned());
            }
        }
        self.refresh_skill_macro_texture();
        true
    }

    pub(super) fn capture_key_config_tray_action(&mut self, offset: usize) {
        let fallback = default_keymap_bindings();
        let bindings = self
            .key_config_draft
            .as_deref()
            .or(self.server_keymap.as_deref())
            .unwrap_or(&fallback);
        let mut entries = keyboard_settings::tray_entries(&self.key_config_actions, bindings);
        entries.sort_by_key(|(_, cell)| *cell);
        let index = entries
            .into_iter()
            .filter(|(_, cell)| {
                *cell >= self.key_config_scroll_row * 18
                    && *cell < (self.key_config_scroll_row + 2) * 18
            })
            .nth(offset)
            .map(|(index, _)| index);
        if let Some(index) = index {
            self.begin_key_config_capture(index);
        }
    }

    pub(super) fn begin_key_config_capture(&mut self, row: usize) {
        if row < self.key_config_actions.len() {
            self.quickslot_key_capture = None;
            self.key_config_capture = Some(row);
            self.refresh_key_config_texture();
        }
    }

    pub(super) fn begin_quickslot_key_capture(&mut self, slot: usize) {
        if slot < self.quickslot_keys.len() {
            self.key_config_capture = None;
            self.quickslot_key_capture = Some(slot);
            self.refresh_key_config_texture();
        }
    }

    pub(super) fn close_key_config(&mut self, apply: bool) {
        if apply {
            if let Some(draft) = self.key_config_draft.take() {
                let previous = self
                    .server_keymap
                    .clone()
                    .unwrap_or_else(default_keymap_bindings);
                let updates = keyboard_settings::changes(&previous, &draft);
                self.server_keymap = Some(draft);
                self.send_key_config_updates(&updates);
            }
            if let Some(keys) = self.key_config_quickslot_draft.take()
                && keys != self.quickslot_keys
            {
                self.quickslot_keys = keys;
                self.quickslot_keys_customized = true;
                if let Err(error) = save_quickslot_keys(keys) {
                    eprintln!("OpenStoryRS: could not save quick-slot keys: {error}");
                }
            }
        }
        self.key_config_draft = None;
        self.key_config_quickslot_draft = None;
        self.key_config_capture = None;
        self.quickslot_key_capture = None;
        self.show_key_config = false;
        self.key_config_pointer_start = None;
        self.gameplay_input = GameplayInput::default();
        self.refresh_quickslots_texture();
    }

    pub(super) fn current_quickslot_keys(&self) -> [u8; 8] {
        self.key_config_quickslot_draft
            .unwrap_or(self.quickslot_keys)
    }

    pub(super) fn key_config_drop_target(&self, x: f32, y: f32) -> Option<u8> {
        if self.quickslot_key_capture.is_some() && keyboard_settings::QUICKSLOT_POPUP.contains(x, y)
        {
            return keyboard_settings::quickslot_at(x, y)
                .map(|slot| self.current_quickslot_keys()[slot]);
        }
        keyboard_settings::key_at(x, y)
    }

    pub(super) fn click_key_config(&mut self, cursor: Point) {
        let (x, y) = (cursor.x - KEY_CONFIG_PANEL_X, cursor.y - KEY_CONFIG_PANEL_Y);
        if self.quickslot_key_capture.is_some() && keyboard_settings::QUICKSLOT_POPUP.contains(x, y)
        {
            if let Some(slot) = keyboard_settings::quickslot_at(x, y) {
                self.begin_quickslot_key_capture(slot);
            }
            self.refresh_key_config_texture();
            return;
        }
        if let Some(button) = keyboard_settings::button_at(x, y) {
            match button {
                keyboard_settings::Button::Ok => self.close_key_config(true),
                keyboard_settings::Button::Cancel | keyboard_settings::Button::Close => {
                    self.close_key_config(false)
                }
                keyboard_settings::Button::Quickslots => {
                    if self.quickslot_key_capture.is_some() {
                        self.quickslot_key_capture = None;
                    } else {
                        self.begin_quickslot_key_capture(0);
                    }
                }
                keyboard_settings::Button::Default => {
                    self.key_config_quickslot_draft = Some(DEFAULT_QUICKSLOT_KEYS);
                    self.key_config_draft = Some(keyboard_settings::defaults());
                    self.key_config_actions_dirty = true;
                    self.key_config_scroll_row = 0;
                    self.key_config_capture = None;
                    self.quickslot_key_capture = None;
                    self.refresh_quickslots_texture();
                }
                keyboard_settings::Button::Clear => {
                    self.key_config_draft = Some(vec![
                        openstory_protocol::KeyBinding::default();
                        openstory_protocol::KEYMAP_ENTRY_COUNT
                    ]);
                    self.key_config_actions_dirty = true;
                    self.key_config_scroll_row = 0;
                    self.key_config_capture = None;
                    self.quickslot_key_capture = None;
                    self.refresh_quickslots_texture();
                }
            }
        } else if self.quickslot_key_capture.is_some() {
            if let Some(slot) = keyboard_settings::quickslot_at(x, y) {
                self.begin_quickslot_key_capture(slot);
            } else if let Some(key) = keyboard_settings::key_at(x, y) {
                self.assign_quickslot_key_capture(key);
            }
        } else if let Some(key) = keyboard_settings::key_at(x, y)
            && self.key_config_capture.is_some()
        {
            self.assign_key_config_capture(key);
        } else {
            let fallback = default_keymap_bindings();
            let bindings = self
                .key_config_draft
                .as_deref()
                .or(self.server_keymap.as_deref())
                .unwrap_or(&fallback);
            let action = keyboard_settings::key_at(x, y)
                .and_then(|key| bindings.get(key as usize))
                .filter(|binding| binding.kind != 0)
                .and_then(|binding| {
                    self.key_config_actions.iter().position(|action| {
                        action.kind == binding.kind && action.action == binding.action
                    })
                })
                .or_else(|| {
                    keyboard_settings::tray_action_at(
                        x,
                        y,
                        self.key_config_scroll_row,
                        &self.key_config_actions,
                        bindings,
                    )
                });
            if let Some(index) = action {
                self.begin_key_config_capture(index);
            }
        }
        self.key_config_pointer_start = self.key_config_capture.map(|_| cursor);
        self.refresh_key_config_texture();
    }

    pub(super) fn finish_key_config_pointer(&mut self, cursor: Option<Point>) {
        let Some(start) = self.key_config_pointer_start.take() else {
            return;
        };
        let Some(cursor) = cursor else {
            return;
        };
        if (cursor.x - start.x).abs() < 4.0 && (cursor.y - start.y).abs() < 4.0 {
            return;
        }
        let (x, y) = (cursor.x - KEY_CONFIG_PANEL_X, cursor.y - KEY_CONFIG_PANEL_Y);
        if let Some(key) = keyboard_settings::key_at(x, y) {
            self.assign_key_config_capture(key);
        } else if keyboard_settings::TRAY.hit(x, y).is_some()
            && let Some(row) = self.key_config_capture.take()
        {
            self.clear_key_config_binding(row);
        }
        self.refresh_key_config_texture();
    }

    pub(super) fn assign_quickslot_key_capture(&mut self, key: u8) {
        let Some(slot) = self.quickslot_key_capture else {
            return;
        };
        let keys = self
            .key_config_quickslot_draft
            .get_or_insert(self.quickslot_keys);
        keyboard_settings::assign_quickslot(keys, slot, key);
        self.refresh_key_config_texture();
        self.refresh_quickslots_texture();
    }

    pub(super) fn scroll_key_config(&mut self, direction: isize) {
        let fallback = default_keymap_bindings();
        let bindings = self
            .key_config_draft
            .as_deref()
            .or(self.server_keymap.as_deref())
            .unwrap_or(&fallback);
        let maximum = keyboard_settings::scroll_max(&self.key_config_actions, bindings);
        self.key_config_scroll_row = self
            .key_config_scroll_row
            .saturating_add_signed(direction)
            .min(maximum);
        self.refresh_key_config_texture();
    }

    pub(super) fn send_key_config_updates(
        &mut self,
        updates: &[(u8, openstory_protocol::KeyBinding)],
    ) {
        if updates.is_empty() {
            return;
        }
        match openstory_protocol::change_keymap_packet(updates) {
            Ok(packet) => {
                if let Some(session) = self.session.as_ref()
                    && let Err(error) = session.send_packet(packet)
                {
                    eprintln!("OpenStory: could not save key binding: {error}");
                    self.channel_connected = false;
                }
            }
            Err(error) => eprintln!("OpenStory: could not encode key binding: {error}"),
        }
    }

    pub(super) fn assign_key_config_capture(&mut self, key: u8) {
        let Some(row) = self.key_config_capture.take() else {
            return;
        };
        if key == 0 || key >= openstory_protocol::KEYMAP_ENTRY_COUNT as u8 {
            self.refresh_key_config_texture();
            return;
        }
        let Some(action_entry) = self.key_config_actions.get(row) else {
            self.refresh_key_config_texture();
            return;
        };
        let (kind, action, assignable) = (
            action_entry.kind,
            action_entry.action,
            action_entry.assignable,
        );
        if !assignable {
            self.refresh_key_config_texture();
            return;
        }

        let mut bindings = self
            .key_config_draft
            .clone()
            .or_else(|| self.server_keymap.clone())
            .unwrap_or_else(default_keymap_bindings);
        keyboard_settings::assign(
            &mut bindings,
            key,
            openstory_protocol::KeyBinding { kind, action },
        );
        self.key_config_draft = Some(bindings);
        self.refresh_key_config_texture();
        self.refresh_quickslots_texture();
    }

    pub(super) fn update_skill_book_drag(&mut self, cursor: Point) -> bool {
        let Some(drag) = self.skill_book_drag.as_mut() else {
            return false;
        };
        if !drag.moved
            && ((cursor.x - drag.start.x).abs() >= 4.0 || (cursor.y - drag.start.y).abs() >= 4.0)
        {
            drag.moved = true;
        }
        drag.moved
    }

    pub(super) fn finish_skill_book_drag(&mut self, cursor: Option<Point>) {
        let Some(drag) = self.skill_book_drag.take() else {
            return;
        };
        self.skill_macro_drag_texture = None;
        let Some(cursor) = cursor else {
            self.skill_book_last_click = None;
            return;
        };
        if !drag.moved {
            let now = Instant::now();
            let double_clicked =
                self.skill_book_last_click
                    .take()
                    .is_some_and(|(previous_skill, previous_at)| {
                        previous_skill == drag.skill_id
                            && now.duration_since(previous_at) <= Duration::from_millis(500)
                    });
            if double_clicked {
                self.use_keybound_skill(drag.skill_id);
            } else {
                self.skill_book_last_click = Some((drag.skill_id, now));
            }
            return;
        }
        self.skill_book_last_click = None;

        if self.show_key_config
            && self.dialog_window_hit_test(DialogWindow::KeyConfig, cursor)
            && self.topmost_dialog_window_at(cursor) == Some(DialogWindow::KeyConfig)
        {
            let (offset_x, offset_y) = self.dialog_window_offset(DialogWindow::KeyConfig);
            let (x, y) = (
                cursor.x - KEY_CONFIG_PANEL_X - offset_x,
                cursor.y - KEY_CONFIG_PANEL_Y - offset_y,
            );
            if let Some(key) = self.key_config_drop_target(x, y) {
                self.assign_skill_to_key(drag.skill_id, key);
            }
        } else if self.current_field.is_some()
            && let Some(slot) = self.quickslot_at(cursor)
        {
            let topmost_window = self.topmost_dialog_window_at(cursor);
            if topmost_window.is_none() || topmost_window == Some(DialogWindow::SkillBook) {
                let key = self.current_quickslot_keys()[slot];
                self.assign_skill_to_key(drag.skill_id, key);
            }
        }
    }

    fn assign_skill_to_key(&mut self, skill_id: i32, key: u8) -> bool {
        let Some(skill) = self
            .skills
            .get(&skill_id)
            .filter(|skill| skill.level > 0 && (skill.id % 10_000) / 1_000 != 0)
        else {
            return false;
        };
        let value = openstory_protocol::KeyBinding {
            kind: 1,
            action: skill.id,
        };
        let previous = self
            .key_config_draft
            .clone()
            .or_else(|| self.server_keymap.clone())
            .unwrap_or_else(default_keymap_bindings);
        let mut updated = previous.clone();
        if !keyboard_settings::assign(&mut updated, key, value) {
            return false;
        }
        if self.show_key_config {
            self.key_config_draft = Some(updated);
        } else {
            let updates = keyboard_settings::changes(&previous, &updated);
            self.server_keymap = Some(updated);
            self.send_key_config_updates(&updates);
        }
        self.key_config_actions_dirty = true;
        self.refresh_key_config_texture();
        self.refresh_quickslots_texture();
        true
    }

    pub(super) fn clear_key_config_binding(&mut self, row: usize) {
        let Some(action_entry) = self.key_config_actions.get(row) else {
            return;
        };
        let (kind, action) = (action_entry.kind, action_entry.action);
        let mut bindings = self
            .key_config_draft
            .clone()
            .or_else(|| self.server_keymap.clone())
            .unwrap_or_else(default_keymap_bindings);
        for binding in &mut bindings {
            if binding.kind == kind && binding.action == action {
                *binding = openstory_protocol::KeyBinding::default();
            }
        }
        self.key_config_draft = Some(bindings);
        self.key_config_actions_dirty = true;
        self.refresh_key_config_texture();
        self.refresh_quickslots_texture();
    }

    pub(super) fn use_keybound_skill(&mut self, skill_id: i32) {
        let Some(skill) = self
            .skill_ui_entry_by_id(skill_id)
            .filter(|skill| skill.level > 0 && !skill.passive)
        else {
            self.push_chat_line(
                "[Notice] That skill is not learned or available to this character.".to_owned(),
                [255, 215, 128],
            );
            return;
        };
        self.use_skill(&skill);
    }

    pub(super) fn skill_ui_entry_by_id(&mut self, skill_id: i32) -> Option<SkillUiEntry> {
        if let Some(skill) = self
            .skill_book_entries
            .iter()
            .find(|skill| skill.id == skill_id)
            .cloned()
        {
            return Some(skill);
        }
        let job = self.active_character.as_ref()?.stats.job;
        let original_tab = self.skill_book_tab;
        let mut found = None;
        for tab in 0..5 {
            if skill_subjob_for_tab(job, tab).is_none() {
                continue;
            }
            self.skill_book_tab = tab;
            self.refresh_skill_book_entries();
            if let Some(skill) = self
                .skill_book_entries
                .iter()
                .find(|skill| skill.id == skill_id)
                .cloned()
            {
                found = Some(skill);
                break;
            }
        }
        self.skill_book_tab = original_tab;
        self.refresh_skill_book_entries();
        if self.show_skill_book {
            self.refresh_skill_book_texture();
        }
        found
    }

    pub(super) fn trigger_skill_macro(&mut self, macro_index: i32) {
        let Some(macro_entry) = usize::try_from(macro_index)
            .ok()
            .and_then(|index| self.skill_macros.get(index))
            .cloned()
        else {
            return;
        };
        if macro_entry.shout && !macro_entry.name.is_empty() {
            match openstory_protocol::general_chat_packet(&macro_entry.name, true) {
                Ok(packet) => {
                    if let Some(session) = self.session.as_ref()
                        && let Err(error) = session.send_packet(packet)
                    {
                        eprintln!("OpenStoryRS: could not shout skill macro: {error}");
                        self.channel_connected = false;
                    }
                }
                Err(error) => {
                    eprintln!("OpenStoryRS: invalid skill macro shout: {error}")
                }
            }
        }
        self.pending_macro_skills.clear();
        self.pending_macro_skills.extend(
            macro_entry
                .skills
                .into_iter()
                .filter(|skill_id| *skill_id > 0),
        );
        self.next_macro_skill_at = Instant::now();
        self.advance_skill_macro();
    }

    pub(super) fn advance_skill_macro(&mut self) {
        if self.pending_macro_skills.is_empty() {
            return;
        }
        if self.map_loading_started_at.is_some() {
            return;
        }
        if self.current_field.is_none() || !self.channel_connected {
            self.pending_macro_skills.clear();
            return;
        }
        let now = Instant::now();
        if now < self.next_macro_skill_at
            || self
                .map_scene
                .as_ref()
                .is_some_and(|scene| scene.attack_stance.is_some())
        {
            return;
        }
        if let Some(skill_id) = self.pending_macro_skills.pop_front() {
            self.use_keybound_skill(skill_id);
            self.next_macro_skill_at = now + Duration::from_millis(250);
        }
    }

    pub(super) fn use_keybound_item(&mut self, item_id: i32) {
        let item = self.inventory.as_ref().and_then(|inventory| {
            inventory
                .items
                .iter()
                .find(|item| {
                    item.item_id == item_id
                        && item.count > 0
                        && matches!(
                            item.inventory_type,
                            openstory_protocol::InventoryType::Use
                                | openstory_protocol::InventoryType::Setup
                                | openstory_protocol::InventoryType::Cash
                        )
                })
                .map(|item| (item.inventory_type, item.slot))
        });
        let Some((inventory_type, slot)) = item else {
            return;
        };
        self.activate_inventory_item(inventory_type, slot);
    }

    pub(super) fn assign_inventory_item_to_quickslot(
        &mut self,
        quickslot: usize,
        inventory_type: openstory_protocol::InventoryType,
        inventory_slot: u16,
    ) {
        let Some(key) = self.current_quickslot_keys().get(quickslot).copied() else {
            return;
        };
        self.assign_inventory_item_to_key(inventory_type, inventory_slot, key);
    }

    pub(super) fn assign_inventory_item_to_key(
        &mut self,
        inventory_type: openstory_protocol::InventoryType,
        inventory_slot: u16,
        key: u8,
    ) -> bool {
        let Some(item_id) = self.inventory.as_ref().and_then(|inventory| {
            inventory
                .items
                .iter()
                .find(|item| {
                    item.inventory_type == inventory_type
                        && item.slot == inventory_slot
                        && item.count > 0
                })
                .map(|item| item.item_id)
        }) else {
            return false;
        };
        let kind = match inventory_type {
            openstory_protocol::InventoryType::Use | openstory_protocol::InventoryType::Setup => 2,
            openstory_protocol::InventoryType::Cash => 3,
            _ => {
                self.push_chat_line(
                    "[Quick slots] Only Use, Setup, and Cash items can be assigned.".to_owned(),
                    [255, 215, 128],
                );
                self.refresh_chat_texture();
                return false;
            }
        };
        if !self.show_key_config && !self.channel_connected {
            self.push_chat_line(
                "[Quick slots] Connect to a field before changing bindings.".to_owned(),
                [255, 215, 128],
            );
            self.refresh_chat_texture();
            return false;
        }
        let binding = openstory_protocol::KeyBinding {
            kind,
            action: item_id,
        };
        let previous = self
            .server_keymap
            .clone()
            .unwrap_or_else(default_keymap_bindings);
        let mut updated = self
            .key_config_draft
            .clone()
            .unwrap_or_else(|| previous.clone());
        if !keyboard_settings::assign(&mut updated, key, binding) {
            return false;
        }
        if self.show_key_config {
            self.key_config_draft = Some(updated);
        } else {
            let updates = keyboard_settings::changes(&previous, &updated);
            self.server_keymap = Some(updated);
            self.send_key_config_updates(&updates);
        }
        self.key_config_actions_dirty = true;
        self.refresh_key_config_texture();
        self.refresh_quickslots_texture();
        self.refresh_inventory_texture();
        true
    }

    pub(super) fn apply_keymap_binding(
        &mut self,
        state: ElementState,
        repeat: bool,
        binding: openstory_protocol::KeyBinding,
    ) -> bool {
        let pressed = state == ElementState::Pressed;
        if binding.kind == 0 {
            return false;
        }
        match binding.kind {
            1 => {
                if pressed && !repeat {
                    self.use_keybound_skill(binding.action);
                }
            }
            2 | 3 => {
                if pressed && !repeat {
                    self.use_keybound_item(binding.action);
                }
            }
            8 => {
                if pressed && !repeat {
                    self.trigger_skill_macro(binding.action);
                }
            }
            4 if pressed && !repeat => match binding.action {
                0 => self.toggle_equipment_inventory(),
                1 => self.open_inventory_tab(self.inventory_tab),
                2 => self.open_status_window(),
                3 => {
                    self.show_skill_book = !self.show_skill_book;
                    if self.show_skill_book {
                        self.refresh_skill_book_entries();
                        self.refresh_skill_book_texture();
                    }
                }
                4 => {
                    self.show_buddy_panel = !self.show_buddy_panel;
                    self.refresh_buddy_texture();
                }
                5 => {
                    if self.world_map.open {
                        self.world_map.open = false;
                        self.refresh_world_map_texture();
                    } else {
                        self.open_world_map();
                    }
                }
                6 => self.open_messenger(),
                10 => {
                    self.chat_target = ChatTarget::Public;
                    self.chat_open = true;
                    self.chat_input_focused = false;
                    self.refresh_chat_texture();
                }
                7 => self.toggle_minimap(),
                8 => {
                    if !self.show_quest_log {
                        self.quest_log_detail_open = true;
                    }
                    self.show_quest_log = !self.show_quest_log;
                    self.refresh_quest_log_texture();
                }
                9 => self.toggle_key_config(),
                11 => self.open_new_whisper_window(),
                14 => self.toggle_game_menu(),
                12 => {
                    self.chat_target = ChatTarget::Party;
                    self.chat_open = true;
                    self.chat_input_focused = false;
                    self.refresh_chat_texture();
                }
                13 => {
                    self.chat_target = ChatTarget::Buddy;
                    self.chat_open = true;
                    self.chat_input_focused = false;
                    self.refresh_chat_texture();
                }
                15 => {
                    self.toggle_quickslots();
                }
                16 => {
                    self.chat_open = !self.chat_open;
                    self.chat_input_focused = false;
                    self.refresh_chat_texture();
                }
                17 => {
                    self.show_guild_panel = !self.show_guild_panel;
                    self.refresh_guild_texture();
                }
                18 => {
                    self.chat_target = ChatTarget::Guild;
                    self.chat_open = true;
                    self.chat_input_focused = false;
                    self.refresh_chat_texture();
                }
                19 => {
                    self.show_party_panel = !self.show_party_panel;
                    self.refresh_party_texture();
                }
                21 => {
                    if let Some(character_id) =
                        self.active_character.as_ref().map(|character| character.id)
                    {
                        self.request_character_info(character_id);
                    }
                }
                22 => {
                    if self.monster_book.open {
                        self.monster_book.open = false;
                        self.monster_book.texture = None;
                        self.monster_book.clear_selected_card();
                        self.monster_book_animation_texture = None;
                    } else {
                        self.monster_book.open = true;
                        self.refresh_monster_book_texture();
                    }
                }
                23 => self.open_cash_shop(),
                24 => {
                    self.chat_target = ChatTarget::Alliance;
                    self.chat_open = true;
                    self.chat_input_focused = false;
                    self.refresh_chat_texture();
                }
                25 => self.open_channel_switch(),
                // v83 assigns menu action 26 to Family (the old UI has no
                // Family icon, so the C++ client aliases this to MEDALS).
                26 | 42 => self.toggle_family_panel(),
                // 27 is Boss Party in the v83 keymap. That panel is not
                // implemented here, so keep the saved action a no-op instead
                // of opening the unrelated party-search window.
                27 => {}
                28 => self.open_settings(),
                32 => self.toggle_event_list(),
                43 => self.open_guild_bbs(),
                44 => {
                    if self.show_alliance_panel {
                        self.show_alliance_panel = false;
                    } else {
                        self.open_alliance_panel();
                    }
                }
                45 => {
                    self.show_pet_equipment = !self.show_pet_equipment;
                    if self.show_pet_equipment {
                        self.pet_equipment_scroll_row = 0;
                        self.refresh_pet_equipment_texture();
                    }
                }
                _ => {}
            },
            5 => match binding.action {
                50 => self.gameplay_input.pickup = pressed,
                51 if !pressed || !repeat => {
                    if pressed
                        && !repeat
                        && self
                            .map_scene
                            .as_ref()
                            .is_some_and(MapSceneRuntime::is_locally_seated)
                    {
                        self.gameplay_input.sit = false;
                        self.stand_from_local_seat();
                    } else {
                        self.gameplay_input.sit = pressed;
                    }
                }
                52 if pressed && !repeat => self.try_local_basic_attack(),
                53 if pressed && !repeat => {
                    self.stand_from_local_seat();
                    self.gameplay_input.jump_buffer = 12;
                }
                54 if pressed && !repeat => self.request_nearest_npc_talk(),
                200 | 201 => {
                    if pressed {
                        self.stand_from_local_seat();
                    }
                    if binding.action == 200 {
                        self.gameplay_input.left = pressed;
                    } else {
                        self.gameplay_input.right = pressed;
                    }
                }
                202 => {
                    self.gameplay_input.up =
                        pressed && !self.gameplay_input.suppress_up_until_release;
                }
                203 => self.gameplay_input.down = pressed,
                _ => {}
            },
            6 if pressed && !repeat => {
                self.send_face_expression(binding.action.saturating_sub(99));
            }
            _ => {}
        }
        if pressed && !repeat && binding.kind == 4 && self.chat_input_focused {
            self.gameplay_input = GameplayInput::default();
        }
        true
    }

    pub(super) fn handle_key_config_key(&mut self, key: PhysicalKey, repeat: bool) -> bool {
        if repeat {
            return true;
        }
        if self.quickslot_key_capture.is_some() {
            match key {
                PhysicalKey::Code(KeyCode::Escape | KeyCode::Backspace) => {
                    self.quickslot_key_capture = None;
                }
                _ => {
                    if let Some(maple_key) = maple_key_from_physical(key) {
                        self.assign_quickslot_key_capture(maple_key);
                    }
                }
            }
        } else if let Some(row) = self.key_config_capture {
            match key {
                PhysicalKey::Code(KeyCode::Escape) => self.key_config_capture = None,
                PhysicalKey::Code(KeyCode::Backspace) => {
                    self.key_config_capture = None;
                    self.clear_key_config_binding(row);
                }
                _ => {
                    if let Some(maple_key) = maple_key_from_physical(key) {
                        self.assign_key_config_capture(maple_key);
                    }
                }
            }
        } else {
            match key {
                PhysicalKey::Code(KeyCode::Escape) => self.close_key_config(false),
                PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                    self.close_key_config(true)
                }
                PhysicalKey::Code(KeyCode::F12) => self.begin_quickslot_key_capture(0),
                PhysicalKey::Code(KeyCode::Digit1) => self.capture_key_config_tray_action(0),
                PhysicalKey::Code(KeyCode::Digit2) => self.capture_key_config_tray_action(1),
                PhysicalKey::Code(KeyCode::Digit3) => self.capture_key_config_tray_action(2),
                PhysicalKey::Code(KeyCode::Digit4) => self.capture_key_config_tray_action(3),
                PhysicalKey::Code(KeyCode::Digit5) => self.capture_key_config_tray_action(4),
                PhysicalKey::Code(KeyCode::Digit6) => self.capture_key_config_tray_action(5),
                PhysicalKey::Code(KeyCode::Digit7) => self.capture_key_config_tray_action(6),
                PhysicalKey::Code(KeyCode::Digit8) => self.capture_key_config_tray_action(7),
                PhysicalKey::Code(KeyCode::ArrowUp) => self.scroll_key_config(-1),
                PhysicalKey::Code(KeyCode::ArrowDown) => self.scroll_key_config(1),
                PhysicalKey::Code(KeyCode::PageUp) => {
                    self.scroll_key_config(-(KEY_CONFIG_VISIBLE_ROWS as isize))
                }
                PhysicalKey::Code(KeyCode::PageDown) => {
                    self.scroll_key_config(KEY_CONFIG_VISIBLE_ROWS as isize)
                }
                _ => {}
            }
        }
        self.refresh_key_config_texture();
        true
    }

    pub(super) fn is_menu_binding(&self, key: PhysicalKey, action: i32) -> bool {
        gameplay_key_binding(self.server_keymap.as_deref(), key)
            .is_some_and(|binding| binding.kind == 4 && binding.action == action)
    }

    pub(super) fn release_gameplay_key(&mut self, key: PhysicalKey) {
        self.gameplay_input.release_key(key);
    }

    pub(super) fn set_gameplay_key(
        &mut self,
        key: PhysicalKey,
        state: ElementState,
        repeat: bool,
    ) -> bool {
        if self.quit_confirmation {
            if state == ElementState::Pressed && !repeat {
                match key {
                    PhysicalKey::Code(KeyCode::Escape) => {
                        self.quit_confirmation = false;
                    }
                    PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                        self.quit_requested = true;
                    }
                    _ => {}
                }
            }
            return true;
        }
        if self.status_auto_confirmation {
            if state == ElementState::Pressed && !repeat {
                match key {
                    PhysicalKey::Code(KeyCode::Escape) => {
                        self.resolve_status_auto_allocate(false);
                    }
                    PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                        self.resolve_status_auto_allocate(true);
                    }
                    _ => {}
                }
            }
            return true;
        }
        if self.current_field.is_none() || self.map_scene.is_none() {
            return false;
        }
        if self.show_key_config {
            if state == ElementState::Pressed {
                return self.handle_key_config_key(key, repeat);
            }
            return true;
        }
        if self.show_settings {
            return true;
        }
        if self.show_status && state == ElementState::Pressed && !repeat {
            if self.is_menu_binding(key, 2) {
                self.show_status = false;
                return true;
            }
            match key {
                PhysicalKey::Code(KeyCode::Escape) => {
                    self.show_status = false;
                    return true;
                }
                _ => {}
            }
        }
        let pressed = state == ElementState::Pressed;
        if !pressed {
            return true;
        }
        if self.handle_messenger_key(key, state, repeat) {
            return true;
        }
        if self.channel_switch_open {
            if pressed && !repeat {
                match key {
                    PhysicalKey::Code(KeyCode::Escape) => self.close_channel_switch(),
                    PhysicalKey::Code(KeyCode::ArrowLeft) => {
                        self.move_channel_switch_selection(-1, 0)
                    }
                    PhysicalKey::Code(KeyCode::ArrowRight) => {
                        self.move_channel_switch_selection(1, 0)
                    }
                    PhysicalKey::Code(KeyCode::ArrowUp) => {
                        self.move_channel_switch_selection(0, -1)
                    }
                    PhysicalKey::Code(KeyCode::ArrowDown) => {
                        self.move_channel_switch_selection(0, 1)
                    }
                    PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                        self.request_channel_switch()
                    }
                    _ => {}
                }
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            return true;
        }
        if self.npc_dialogue.is_some() {
            return false;
        }
        if self.show_skill_book
            && pressed
            && !repeat
            && matches!(key, PhysicalKey::Code(KeyCode::Escape))
        {
            self.show_skill_book = false;
            self.skill_book_texture = None;
            return true;
        }
        if self.monster_book.open {
            if pressed
                && !repeat
                && (matches!(key, PhysicalKey::Code(KeyCode::Escape))
                    || self.is_menu_binding(key, 22))
            {
                self.monster_book.open = false;
                self.monster_book.texture = None;
                self.monster_book.clear_selected_card();
                self.monster_book_animation_texture = None;
            }
            return true;
        }
        if self.party_search.is_some() {
            if pressed && !repeat && matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
                self.party_search = None;
                self.party_search_texture = None;
                self.party_search_actions.clear();
            }
            return true;
        }
        if self.world_map.open
            && pressed
            && !repeat
            && matches!(key, PhysicalKey::Code(KeyCode::Escape))
        {
            self.escape_world_map();
            return true;
        }
        if self.show_quest_log
            && !self.is_menu_binding(key, 8)
            && matches!(
                key,
                PhysicalKey::Code(KeyCode::Escape | KeyCode::ArrowLeft | KeyCode::ArrowRight)
            )
        {
            if pressed && !repeat {
                match key {
                    PhysicalKey::Code(KeyCode::Escape) => {
                        self.show_quest_log = false;
                        self.quest_forfeit_confirmation = None;
                    }
                    PhysicalKey::Code(KeyCode::ArrowLeft) => {
                        self.change_selected_quest_reward(-1);
                    }
                    PhysicalKey::Code(KeyCode::ArrowRight) => {
                        self.change_selected_quest_reward(1);
                    }
                    _ => {}
                }
            }
            return true;
        }
        if self.cash_shop.is_some() {
            if pressed && !repeat {
                match key {
                    PhysicalKey::Code(KeyCode::Escape) => {
                        let modal = self.cash_shop.as_ref().map_or(0, |shop| {
                            if shop.confirmation {
                                1
                            } else if shop.coupon_open {
                                2
                            } else {
                                0
                            }
                        });
                        if let Some(shop) = self.cash_shop.as_mut() {
                            match modal {
                                1 => shop.confirmation = false,
                                2 => {
                                    shop.coupon_open = false;
                                    shop.coupon_focused = false;
                                }
                                _ => {}
                            }
                        }
                        if modal == 0 {
                            self.exit_cash_shop();
                        }
                        self.refresh_cash_shop_texture();
                    }
                    PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                        let (confirming, coupon_focused) =
                            self.cash_shop.as_ref().map_or((false, false), |shop| {
                                (shop.confirmation, shop.coupon_focused)
                            });
                        if confirming {
                            self.buy_selected_cash_item();
                        } else if coupon_focused {
                            self.redeem_cash_shop_coupon();
                        } else if let Some(shop) = self.cash_shop.as_mut() {
                            shop.search_focused = false;
                            self.refresh_cash_shop_texture();
                        }
                    }
                    PhysicalKey::Code(KeyCode::Backspace) => {
                        if let Some(shop) = self.cash_shop.as_mut() {
                            if shop.search_focused {
                                shop.query.pop();
                                shop.page = 0;
                            } else if shop.coupon_focused {
                                shop.coupon_code.pop();
                            }
                        }
                        self.refresh_cash_shop_texture();
                    }
                    _ => {}
                }
            }
            return true;
        }
        if self.show_pet_equipment && pressed && !repeat {
            if matches!(key, PhysicalKey::Code(KeyCode::Escape)) || self.is_menu_binding(key, 45) {
                self.show_pet_equipment = false;
                return true;
            }
            let scroll = match key {
                PhysicalKey::Code(KeyCode::ArrowUp) => -1,
                PhysicalKey::Code(KeyCode::ArrowDown) => 1,
                PhysicalKey::Code(KeyCode::PageUp) => -5,
                PhysicalKey::Code(KeyCode::PageDown) => 5,
                _ => 0,
            };
            if scroll != 0 {
                self.scroll_pet_equipment_by(scroll);
                return true;
            }
        }
        if !matches!(key, PhysicalKey::Code(KeyCode::Escape))
            && let Some(binding) = gameplay_key_binding(self.server_keymap.as_deref(), key)
            && binding.kind != 0
        {
            if pressed && binding.kind == 5 && matches!(binding.action, 50 | 51 | 200..=203) {
                self.gameplay_input.held_bindings.insert(key, binding);
            }
            return self.apply_keymap_binding(state, repeat, binding);
        }
        if !matches!(
            key,
            PhysicalKey::Code(KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter)
        ) {
            let action = match key {
                PhysicalKey::Code(KeyCode::ArrowLeft) => Some(200),
                PhysicalKey::Code(KeyCode::ArrowRight) => Some(201),
                PhysicalKey::Code(KeyCode::ArrowUp) => Some(202),
                PhysicalKey::Code(KeyCode::ArrowDown) => Some(203),
                _ => None,
            };
            if let Some(action) = action {
                let binding = openstory_protocol::KeyBinding { kind: 5, action };
                if pressed {
                    self.gameplay_input.held_bindings.insert(key, binding);
                }
                self.apply_keymap_binding(state, repeat, binding);
            }
            return true;
        }
        if pressed && !repeat && matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
            if self.show_skill_book {
                self.show_skill_book = false;
            } else if self.show_family_tree {
                self.show_family_tree = false;
                self.refresh_family_tree_texture();
            } else if self.show_family_panel {
                self.show_family_panel = false;
            } else if self.show_alliance_panel {
                self.show_alliance_panel = false;
            } else if self.show_guild_panel {
                self.show_guild_panel = false;
                self.guild_panel_mode = GuildPanelMode::Overview;
            } else if self.show_party_panel {
                self.show_party_panel = false;
            } else if self.show_buddy_panel {
                self.show_buddy_panel = false;
            } else if self.show_equipment_inventory {
                self.show_equipment_inventory = false;
            } else if self.show_inventory {
                if self.pending_scroll_slot.take().is_some() {
                    self.refresh_inventory_texture();
                } else {
                    self.show_inventory = false;
                    self.inventory_drag_source = None;
                    self.inventory_hover_slot = None;
                }
            } else {
                self.open_quit_confirmation();
            }
            return true;
        }
        false
    }

    pub(super) fn replace_scene(&mut self, mut scene: LoginSceneAssets) -> Result<(), String> {
        let local_name_label_index = append_scene_local_name_tag(
            &mut scene.sprites,
            scene.map_scene.is_some(),
            self.active_character.as_ref(),
            &self.font,
            &self.ui_archive,
            &self.character_archive,
        );
        let renderer = self
            .renderer
            .as_ref()
            .ok_or_else(|| "graphics are not ready to change screens".to_owned())?;
        let (textures, draws, texture_indices) = upload_scene_sprites(renderer, &scene.sprites)?;
        self.finish_scene_replacement(
            scene,
            local_name_label_index,
            textures,
            draws,
            texture_indices,
        )
    }

    pub(super) fn replace_scene_with_uploaded(
        &mut self,
        scene: LoginSceneAssets,
        local_name_label_index: Option<usize>,
        textures: Vec<GpuTexture>,
        texture_indices: HashMap<String, usize>,
    ) -> Result<(), String> {
        let draws = scene
            .sprites
            .iter()
            .map(|sprite| {
                let texture_index = texture_indices
                    .get(&sprite.texture_key)
                    .copied()
                    .expect("scene texture was indexed before upload");
                (texture_index, sprite.rect)
            })
            .collect();
        self.finish_scene_replacement(
            scene,
            local_name_label_index,
            textures,
            draws,
            texture_indices,
        )
    }

    fn finish_scene_replacement(
        &mut self,
        scene: LoginSceneAssets,
        local_name_label_index: Option<usize>,
        textures: Vec<GpuTexture>,
        mut draws: Vec<(usize, SpriteRect)>,
        texture_indices: HashMap<String, usize>,
    ) -> Result<(), String> {
        let LoginSceneAssets {
            sprites: _,
            actions,
            map_scene,
            animated_art,
            login_buttons,
            save_id_indicator_draws,
        } = scene;
        let mut map_scene = map_scene.map(|scene| MapSceneRuntime::new(scene, &mut draws));
        if let Some(scene) = map_scene.as_mut() {
            scene.local_name_label_draw_index = local_name_label_index;
            scene.local_name_label =
                local_name_label_index.and_then(|index| draws.get(index).copied());
            scene.energy = self.session_energy;
            scene.gm_account = self.gm_account;
            scene.gm_hidden = self.gm_hidden;
            scene
                .mark_npcs_scriptable(&self.scriptable_npc_ids.iter().copied().collect::<Vec<_>>());
        }
        self.sprite_textures = textures;
        self.texture_indices = texture_indices;
        self.remote_art_cache.clear();
        // Runtime frame caches contain indices into the sprite table above.
        // Replacing the scene rebuilds that table, so cached frames must be
        // discarded before they can resolve to unrelated textures.
        self.projectile_frames_cache.clear();
        self.item_use_effect_cache.clear();
        self.item_aura_frames_cache.clear();
        self.summon_frames_cache.clear();
        self.dragon_frames_cache.clear();
        self.pet_frames_cache.clear();
        self.pet_interaction_frames_cache.clear();
        self.skill_projectile_cache.clear();
        self.skill_effect_cache.clear();
        self.skill_point_effect_cache.clear();
        self.mob_disease_effect_cache.clear();
        self.character_effect_cache.clear();
        self.field_effect_cache.clear();
        self.skill_hit_effect_cache.clear();
        self.face_expression_cache.clear();
        self.teleport_effect_frames = None;
        self.npc_dialogue = None;
        self.pending_npc_talk = None;
        self.rps_game = None;
        self.inventory_drag_source = None;
        self.inventory_hover_slot = None;
        self.inventory_drop_prompt = None;
        self.inventory_drop_prompt_texture = None;
        self.death_notice = false;
        self.death_respawn_pending = false;
        self.death_notice_texture = None;
        self.pending_scroll_slot = None;
        self.pending_skill_hit_sounds.clear();
        self.pending_mob_hit_sounds.clear();
        self.sprite_draws = draws;
        self.login_actions = actions;
        self.map_scene = map_scene;
        self.event_list_open = false;
        self.event_list_texture = None;
        self.event_list.clear();
        self.event_list_received = false;
        self.event_list_offset = 0;
        self.event_list_selected = None;
        self.event_list_peak_durations.clear();
        self.local_chalkboard_texture = None;
        self.remote_chalkboard_textures.clear();
        if self.map_scene.is_none() {
            self.clear_boss_hp_gauge();
        }
        let defer_map_ui_refresh =
            self.map_loading_started_at.is_some() && self.map_scene.is_some();
        if let Some(look) = self
            .active_character
            .as_ref()
            .map(|character| character.look.clone())
            && !defer_map_ui_refresh
        {
            self.refresh_local_mount(&look);
        }
        self.save_id_indicator_draws = save_id_indicator_draws;
        let (login_animations, login_animation_draw_indices, active_login_animation_draw_indices) =
            prepare_map_art_animations(animated_art);
        self.login_animations = login_animations;
        self.login_animation_draw_indices = login_animation_draw_indices;
        self.active_login_animation_draw_indices = active_login_animation_draw_indices;
        let (login_buttons, login_button_draw_indices, active_login_button_draw_indices) =
            prepare_login_buttons(login_buttons);
        self.login_buttons = login_buttons;
        self.login_button_draw_indices = login_button_draw_indices;
        self.active_login_button_draw_indices = active_login_button_draw_indices;
        self.focused_login_button = None;
        self.refresh_field_texture(self.focused_field);
        self.hovered_login_button = None;
        self.pressed_login_button = None;
        self.refresh_login_button_visuals();
        self.refresh_save_id_indicator();
        self.local_speech_texture = None;
        self.remote_speech_textures.clear();
        self.local_chalkboard_texture = None;
        self.remote_chalkboard_textures.clear();
        self.pet_speech_textures.clear();
        if self.map_scene.is_some() {
            if !defer_map_ui_refresh {
                self.refresh_map_hud();
            }
        } else {
            self.map_hud_texture = None;
        }
        if !defer_map_ui_refresh {
            self.refresh_minimap_texture();
        }
        if self.map_scene.is_none() {
            self.current_field = None;
            self.clear_clock();
            self.gameplay_input = GameplayInput::default();
            self.show_game_menu = false;
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
        Ok(())
    }
}

fn projectile_range_extension(
    skill_archive: &NxArchive,
    skills: &HashMap<i32, openstory_protocol::SkillEntry>,
    weapon_type: i32,
) -> i16 {
    let skill_ids: &[i32] = match weapon_type {
        145 | 146 => &[3_000_002, 13_000_001], // Eye of Amazon
        147 => &[4_000_001, 14_000_001],       // Keen Eyes
        _ => return 0,
    };
    skill_ids
        .iter()
        .filter_map(|skill_id| {
            let level = skills.get(skill_id)?.level;
            if level <= 0 {
                return None;
            }
            let skill_path = format!("{:03}.img/skill/{skill_id:07}", skill_id / 10_000);
            let skill = skill_archive.root().resolve(&skill_path).ok()??;
            let level = skill
                .child("level")
                .ok()??
                .child(&level.to_string())
                .ok()??;
            integer_property(&level, "range")
                .filter(|range| *range > 0)
                .map(|range| range.clamp(0, i64::from(i16::MAX)) as i16)
        })
        .max()
        .unwrap_or(0)
}
