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
            cursor: None,
            cursor_physical: None,
            dialog_window_offsets: HashMap::new(),
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
            settings_texture: None,
            status_texture: None,
            status_detail_texture: None,
            key_config_texture: None,
            skill_macro_texture: None,
            inventory_texture: None,
            equipment_inventory_texture: None,
            inventory_tooltip_texture: None,
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
            quest_log_texture: None,
            monster_book: monster_book::State::default(),
            monster_book_animation_texture: None,
            world_map: world_map::State::default(),
            world_map_key_text_for_ime_dedup: None,
            channel_switch_open: false,
            channel_switch_texture: None,
            channel_switch_actions: Vec::new(),
            channel_switch_selected: 0,
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
            chat_open: false,
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
            quickslot_key_capture: None,
            key_config_scroll_row: 0,
            key_config_actions: Vec::new(),
            key_config_actions_dirty: true,
            server_keymap: None,
            quickslot_keys: saved_quickslot_keys.unwrap_or(DEFAULT_QUICKSLOT_KEYS),
            quickslot_keys_customized,
            show_quickslots: false,
            quickslot_texture: None,
            quickslot_icon_cache: HashMap::new(),
            show_status_menu: false,
            status_menu_texture: None,
            combo_count: 0,
            combo_displayed: 0,
            combo_fade_remaining: Duration::ZERO,
            combo_texture: None,
            skill_macro_editor: skill_macros::State::default(),
            skill_macros: vec![
                openstory_protocol::SkillMacro::default();
                openstory_protocol::SKILL_MACRO_COUNT
            ],
            pending_macro_skills: VecDeque::new(),
            next_macro_skill_at: Instant::now(),
            status_auto_confirmation: false,
            status_waiting_for_stats: false,
            settings_dragging: None,
            settings_selected_slider: OptionControl::Music,
            settings,
            settings_original: settings,
            display_settings,
            display_settings_original: display_settings,
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
            active_buffs: HashMap::new(),
            pending_skill_hit_sounds: Vec::new(),
            skill_cooldowns: HashMap::new(),
            forced_equipment_stats: ForcedEquipmentStats::default(),
            inventory_tab: openstory_protocol::InventoryType::Equip,
            inventory_scroll_row: 0,
            inventory_hover_slot: None,
            inventory_drag_source: None,
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
            npc_shop: None,
            hired_merchant_room: None,
            trade_room: None,
            character_info: None,
            storage: None,
            pending_storage_restores: Vec::new(),
            cash_shop: None,
            cash_products,
            map_hud_texture: None,
            buff_list_texture: None,
            buff_list_refreshed_at: None,
            status_bar_draw_indices: Vec::new(),
            status_bar_gauges: [None; 3],
            status_bar_number_font: None,
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
            quest_log_selected: 0,
            quest_log_scroll: 0,
            quest_reward_selected: 0,
            quest_forfeit_confirmation: None,
            current_field: None,
            show_login_fields: true,
            map_scene: None,
            last_npc_talk_at: None,
            last_local_skill_cast: None,
            pending_zombify_heal_feedback_until: None,
            gameplay_input: GameplayInput::default(),
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
                    eprintln!("OpenStoryRS: could not request pet loot: {error}");
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
        let facing = match (left && !is_stunned, right && !is_stunned) {
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
                if let Some(ladder) = scene.local_ladder {
                    scene.player.mode = PhysicsMode::Fixated;
                    scene.player.x = f64::from(ladder.x);
                    scene.player.horizontal_speed = 0.0;
                    if self.gameplay_input.jump_buffer > 0 && (left || right) && !is_stunned {
                        self.audio.play_world_sound(
                            &self.sound_archive,
                            "Game.img/Jump",
                            (rounded_i16(scene.player.x), rounded_i16(scene.player.y)),
                        );
                        let jump_right = right;
                        if scene.facing_right != jump_right {
                            scene.facing_right = jump_right;
                            scene.refresh_avatar_draws(&mut self.sprite_draws);
                        }
                        scene.player.horizontal_speed = if left {
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
                        } else if self.gameplay_input.up && !self.gameplay_input.down {
                            -climb_force
                        } else if self.gameplay_input.down && !self.gameplay_input.up {
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
                        } else if left && !right {
                            -SWIM_FORCE
                        } else if right && !left {
                            SWIM_FORCE
                        } else {
                            0.0
                        };
                        let vertical = if is_stunned {
                            0.0
                        } else if self.gameplay_input.up && !self.gameplay_input.down {
                            -SWIM_FORCE
                        } else if self.gameplay_input.down && !self.gameplay_input.up {
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
                            let direction = if is_stunned {
                                0.0
                            } else {
                                match (left, right) {
                                    (true, false) => -1.0,
                                    (false, true) => 1.0,
                                    _ => 0.0,
                                }
                            };
                            if direction != 0.0 {
                                scene.player.apply_force(direction * walk_force, 0.0);
                            }
                            if self.gameplay_input.jump_buffer > 0 && !is_stunned {
                                if self.gameplay_input.down
                                    && self.gameplay_input.jump_down_ticks < 5
                                {
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
                            } else if !self.gameplay_input.down {
                                self.gameplay_input.jump_down_ticks = 0;
                            }
                        } else {
                            self.gameplay_input.jump_down_ticks = 0;
                            if is_stunned {
                                scene.player.horizontal_speed = 0.0;
                            } else if left && !right && scene.player.horizontal_speed > 0.0 {
                                scene.player.horizontal_speed =
                                    (scene.player.horizontal_speed - 0.025).max(0.0);
                            } else if right && !left && scene.player.horizontal_speed < 0.0 {
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
                        self.gameplay_input.down,
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
                scene.player.vertical_speed = -3.5;
                scene.player.on_ground = false;
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
        self.active_buffs
            .retain(|_, buff| buff.expires_at.is_none_or(|expires_at| expires_at > now));
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
            attack_stats.element = skill.element;
            attack_stats.fixed_damage = skill.fixed_damage;
            if skill.magic_damage && skill.fixed_damage.is_none() {
                // Match Cosmic's v83 magic upper-bound formula. `mad` in
                // Skill.nx is already the multiplier (for example 33),
                // while total magic includes INT and equipment MATK.
                let magic = attack_stats.magic_total.max(0);
                let magic_per_thousand = magic.saturating_add(999) / 1000;
                let numerator = magic
                    .saturating_mul(magic_per_thousand)
                    .saturating_add(magic);
                let mut base_max = numerator.saturating_add(29) / 30;
                let total_intelligence = attack_stats.intelligence.max(0);
                base_max = base_max.saturating_add(total_intelligence.saturating_add(199) / 200);
                if let Some(amplification_skill) = magic_amplification_skill(character.stats.job)
                    && let Some(level) =
                        learned_skill_level(&self.skill_archive, &self.skills, amplification_skill)
                {
                    let amplification = integer_property(&level, "y").unwrap_or(100).max(0);
                    base_max = base_max.saturating_mul(amplification) / 100;
                }
                let maximum_damage = base_max as f64 * skill.magic_damage_multiplier;
                let unmastered_minimum =
                    4.0 * total_intelligence as f64 + attack_stats.luck.max(0) as f64;
                let mastery_fraction = if unmastered_minimum > 0.0 {
                    (4.0 * total_intelligence as f64 * skill.magic_mastery
                        + attack_stats.luck.max(0) as f64)
                        / unmastered_minimum
                } else {
                    skill.magic_mastery
                }
                .clamp(0.0, 1.0);
                attack_stats.minimum_damage = maximum_damage * mastery_fraction;
                attack_stats.maximum_damage = maximum_damage;
            } else if !skill.magic_damage && skill.fixed_damage.is_none() {
                attack_stats.minimum_damage *= skill.damage_multiplier;
                attack_stats.maximum_damage *= skill.damage_multiplier;
            }
        }
        let hit_count = skill.map_or(1, |skill| {
            if attack_stats.attack_type == openstory_protocol::AttackType::Ranged {
                skill.bullet_count
            } else {
                skill.attack_count
            }
        });
        let target_count = skill.map_or(1, |skill| usize::from(skill.mob_count));
        let attack_range = skill
            .and_then(|skill| skill.attack_range)
            .unwrap_or_else(|| match (skill.is_some(), attack_stats.attack_type) {
                (false, openstory_protocol::AttackType::Close) => [-300, 0, -150, 150],
                _ => [-400, -5, -50, 50],
            });
        let horizontal_range_multiplier =
            skill.map_or(1.0, |skill| skill.horizontal_range_multiplier);
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
        let Some(stance) =
            skill_stance.or_else(|| regular_attack_stance(attack_stats, attack_index))
        else {
            return;
        };
        let skill_hit_delays = if skill_id > 0 {
            attack_hit_delays(
                &self.character_archive,
                &self.skill_archive,
                &character.look,
                skill_id,
                skill_level,
                stance,
                attack_stats.speed,
                usize::from(hit_count.clamp(1, 15)),
            )
        } else {
            Vec::new()
        };
        let target_positions =
            scene.nearest_attack_targets(target_count, attack_range, horizontal_range_multiplier);
        if skill_id > 0 {
            let sound_start = Instant::now();
            for (object_id, _, (target_x, target_y)) in &target_positions {
                for hit_index in 0..hit_count.clamp(1, 15) {
                    let position = (rounded_i16(*target_x), rounded_i16(*target_y));
                    let delay = skill_hit_delays
                        .get(usize::from(hit_index))
                        .copied()
                        .unwrap_or_default();
                    scene.schedule_mob_attack_impact(
                        *object_id,
                        Arc::clone(&skill_hit_effects),
                        !scene.facing_right,
                        None,
                        delay,
                    );
                    if delay.is_zero() {
                        self.audio
                            .play_skill_hit(&self.sound_archive, skill_id, position);
                    } else {
                        if self.pending_skill_hit_sounds.len() >= 512 {
                            self.pending_skill_hit_sounds.remove(0);
                        }
                        if let Some(play_at) = sound_start.checked_add(delay) {
                            self.pending_skill_hit_sounds.push(PendingSkillHitSound {
                                skill_id,
                                position,
                                play_at,
                            });
                        }
                    }
                }
            }
        }
        let projectile_targets = target_positions
            .iter()
            .map(|(_, _, (x, y))| (*x, *y - 24.0))
            .take(15)
            .collect::<Vec<_>>();
        let reactor_target =
            scene.nearest_reactor_target(attack_range, horizontal_range_multiplier);
        let reactor_target_position = reactor_target.and_then(|object_id| {
            scene
                .reactors
                .get(&object_id)
                .map(|reactor| (reactor.x, reactor.y))
        });
        let targets = target_positions
            .into_iter()
            .map(
                |(object_id, mob, _)| openstory_protocol::AttackDamageTarget {
                    object_id,
                    damage_lines: (0..hit_count.clamp(1, 15))
                        .map(|_| basic_attack_damage(scene, attack_stats, mob))
                        .collect(),
                },
            )
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
        if let Some(frames) = projectile_frames {
            if attack_stats.attack_type == openstory_protocol::AttackType::Ranged || skill_id > 0 {
                if projectile_targets.is_empty() {
                    let (target_x, target_y) = reactor_target_position.unwrap_or_else(|| {
                        (
                            scene.player.x + if scene.facing_right { 400.0 } else { -400.0 },
                            scene.player.y - 26.0,
                        )
                    });
                    scene.launch_projectile(frames, target_x, target_y);
                } else {
                    for (target_x, target_y) in projectile_targets {
                        scene.launch_projectile(frames.clone(), target_x, target_y);
                    }
                }
            }
        }
        scene.start_attack(stance, &mut self.sprite_draws);
        if skill_id > 0 {
            scene.show_character_skill_effect(None, skill_effects, skill_effect_speed_factor);
            self.audio.play_skill_use(
                &self.sound_archive,
                skill_id,
                (rounded_i16(scene.player.x), rounded_i16(scene.player.y)),
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
    }

    pub(super) fn toggle_key_config(&mut self) {
        self.show_key_config = !self.show_key_config;
        self.key_config_capture = None;
        self.quickslot_key_capture = None;
        if self.show_key_config {
            self.key_config_scroll_row = 0;
            self.key_config_actions_dirty = true;
            self.show_inventory = false;
            self.show_skill_book = false;
            self.show_quest_log = false;
            self.show_pet_equipment = false;
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
    }

    pub(super) fn toggle_status_menu(&mut self) {
        self.show_status_menu = !self.show_status_menu;
        if self.show_status_menu {
            self.refresh_status_menu_texture();
        }
    }

    pub(super) fn refresh_status_menu_texture(&mut self) {
        if !self.show_status_menu {
            return;
        }
        let pixels = render_status_menu(&self.font);
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.status_menu_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(STATUS_MENU_WIDTH, STATUS_MENU_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.status_menu_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update the status menu: {error}"),
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
        self.show_equipment_inventory = false;
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
        self.show_skill_book = false;
        self.show_pet_equipment = false;
        self.update_inventory_hover();
        self.refresh_inventory_texture();
    }

    pub(super) fn toggle_equipment_inventory(&mut self) {
        self.show_equipment_inventory = !self.show_equipment_inventory;
        if self.show_equipment_inventory {
            self.show_inventory = false;
            self.inventory_drag_source = None;
            self.inventory_hover_slot = None;
            self.pending_scroll_slot = None;
            self.show_skill_book = false;
            self.show_pet_equipment = false;
            self.refresh_equipment_inventory_texture();
        }
    }

    pub(super) fn activate_status_menu_item(&mut self, item: usize) {
        self.show_status_menu = false;
        match item {
            0 => self.open_status_window(),
            1 => {
                self.show_skill_book = !self.show_skill_book;
                if self.show_skill_book {
                    self.show_inventory = false;
                    self.show_pet_equipment = false;
                    self.skill_book_tab = 0;
                    self.skill_book_scroll_row = 0;
                    self.refresh_skill_book_entries();
                    self.refresh_skill_book_texture();
                }
            }
            2 => {
                self.show_quest_log = true;
                self.show_inventory = false;
                self.show_pet_equipment = false;
                self.show_skill_book = false;
                self.inventory_drag_source = None;
                self.pending_scroll_slot = None;
                self.quest_log_tab = QuestLogTab::Active;
                self.quest_log_scroll = 0;
                self.quest_log_selected = 0;
                self.quest_reward_selected = 0;
                self.quest_forfeit_confirmation = None;
                self.refresh_quest_log_texture();
            }
            3 => self.toggle_event_list(),
            4 => self.open_inventory_tab(openstory_protocol::InventoryType::Use),
            5 => self.open_inventory_tab(openstory_protocol::InventoryType::Equip),
            6 => {
                self.show_party_panel = !self.show_party_panel;
                if self.show_party_panel {
                    self.show_buddy_panel = false;
                    self.show_guild_panel = false;
                    self.show_alliance_panel = false;
                }
                self.refresh_party_texture();
            }
            7 => {
                self.show_buddy_panel = !self.show_buddy_panel;
                if self.show_buddy_panel {
                    self.show_party_panel = false;
                    self.show_guild_panel = false;
                    self.show_alliance_panel = false;
                }
                self.refresh_buddy_texture();
            }
            8 => {
                self.show_guild_panel = !self.show_guild_panel;
                if self.show_guild_panel {
                    self.show_party_panel = false;
                    self.show_buddy_panel = false;
                    self.show_alliance_panel = false;
                }
                self.refresh_guild_texture();
            }
            9 => {
                self.monster_book.open = !self.monster_book.open;
                if self.monster_book.open {
                    self.refresh_monster_book_texture();
                }
            }
            10 => {
                if self.world_map.open {
                    self.world_map.open = false;
                    self.refresh_world_map_texture();
                } else {
                    self.open_world_map();
                }
            }
            11 => self.toggle_key_config(),
            12 => self.open_settings(),
            13 => self.open_quit_confirmation(),
            _ => {}
        }
    }

    pub(super) fn click_native_status_bar(&mut self, cursor: Point) -> bool {
        if self.current_field.is_none() {
            return false;
        }

        if self.show_status_menu {
            let menu_bounds = SpriteRect {
                x: STATUS_MENU_X,
                y: STATUS_MENU_Y,
                width: STATUS_MENU_WIDTH as f32,
                height: STATUS_MENU_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            };
            if rect_contains(menu_bounds, cursor) {
                let row_y = cursor.y - (STATUS_MENU_Y + 4.0);
                if row_y >= 0.0 {
                    let row = (row_y / STATUS_MENU_ROW_HEIGHT as f32) as usize;
                    if row < STATUS_MENU_ROWS.len() {
                        self.audio
                            .play_ui_sound(&self.sound_archive, "UI.img/BtMouseClick");
                        self.activate_status_menu_item(row);
                    }
                }
                return true;
            }
            self.show_status_menu = false;
        }

        let small_x = 838.0;
        let small_y = STATUS_BAR_TOP + 5.0;
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
                                self.show_inventory = false;
                                self.show_pet_equipment = false;
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

        let big_x = 794.0;
        let big_y = STATUS_BAR_TOP + 35.0;
        if (big_y..big_y + 34.0).contains(&cursor.y) {
            let button = (0..4).find(|index| {
                let left = big_x + *index as f32 * 56.0;
                (left..left + 54.0).contains(&cursor.x)
            });
            match button {
                Some(0..=3) => {
                    self.audio
                        .play_ui_sound(&self.sound_archive, "UI.img/BtMouseClick");
                    match button {
                        Some(0) => self.toggle_status_menu(),
                        Some(1) => self.open_cash_shop(),
                        Some(2) => self.open_settings(),
                        Some(3) => {
                            self.chat_target = ChatTarget::Public;
                            self.chat_input = "/trade ".to_owned();
                            self.chat_open = true;
                            self.refresh_chat_texture();
                        }
                        _ => {}
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
        const PANEL_X: f32 = 865.0;
        const PANEL_Y: f32 = 615.0;
        const CELL_X: f32 = 7.0;
        const CELL_Y: f32 = 8.0;
        const CELL_WIDTH: f32 = 32.0;
        const CELL_HEIGHT: f32 = 30.0;
        const COLUMN_STEP: f32 = 35.0;
        const ROW_STEP: f32 = 34.0;
        (0..8).find(|slot| {
            let column = (*slot % 4) as f32;
            let row = (*slot / 4) as f32;
            rect_contains(
                SpriteRect {
                    x: PANEL_X + CELL_X + column * COLUMN_STEP,
                    y: PANEL_Y + CELL_Y + row * ROW_STEP,
                    width: CELL_WIDTH,
                    height: CELL_HEIGHT,
                    blend_mode: BlendMode::Alpha,
                },
                cursor,
            )
        })
    }

    pub(super) fn click_quickslot(&mut self, cursor: Point) -> bool {
        let Some(slot) = self.quickslot_at(cursor) else {
            return false;
        };
        let key = self.quickslot_keys[slot];
        if let Some(binding) = self
            .server_keymap
            .as_ref()
            .and_then(|bindings| bindings.get(usize::from(key)))
            .copied()
            && binding.kind != 0
        {
            self.apply_keymap_binding(ElementState::Pressed, false, binding);
        }
        true
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
                    "{:03}.img/skill/{}/icon",
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
        let bindings = self.server_keymap.as_deref().unwrap_or(&fallback);
        let slot_bindings = self
            .quickslot_keys
            .iter()
            .map(|key| bindings.get(usize::from(*key)).copied().unwrap_or_default())
            .collect::<Vec<_>>();
        let icons: [Option<(u32, u32, Arc<[u8]>)>; 8] = std::array::from_fn(|slot| {
            let binding = slot_bindings.get(slot).copied().unwrap_or_default();
            (binding.kind != 0)
                .then(|| self.quickslot_icon_pixels(binding))
                .flatten()
        });
        let pixels = render_quickslot_panel_pixels(
            quickslot_background_image(&self.legacy_ui_archive),
            &self.ui_archive,
            &self.quickslot_keys,
            &icons,
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
            Ok(Some(texture)) => self.quickslot_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not refresh quick slots: {error}"),
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
                self.server_keymap.as_deref(),
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
        self.key_config_scroll_row = self.key_config_scroll_row.min(
            self.key_config_actions
                .len()
                .saturating_sub(KEY_CONFIG_VISIBLE_ROWS),
        );
        let fallback;
        let bindings = if let Some(bindings) = self.server_keymap.as_ref() {
            bindings.as_slice()
        } else {
            fallback = default_keymap_bindings();
            fallback.as_slice()
        };
        let pixels = render_key_config_panel(
            &self.font,
            &self.ui_archive,
            bindings,
            &self.quickslot_keys,
            self.key_config_capture,
            self.quickslot_key_capture,
            self.key_config_scroll_row,
            &self.key_config_actions,
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

    pub(super) fn click_skill_macro_editor(&mut self, cursor: Point) {
        let bounds = SpriteRect {
            x: skill_macros::PANEL_X,
            y: skill_macros::PANEL_Y,
            width: skill_macros::PANEL_WIDTH as f32,
            height: skill_macros::PANEL_HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        };
        if !rect_contains(bounds, cursor) {
            self.skill_macro_editor.close();
            return;
        }
        let skills = self.skill_macro_choices();
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
            Some(skill_macros::Action::SelectSlot(slot)) => {
                self.skill_macro_editor.select_slot(slot);
            }
            Some(skill_macros::Action::ClearSkill(slot)) => {
                self.skill_macro_editor.clear_skill(slot);
            }
            Some(skill_macros::Action::AssignSkill(skill_id)) => {
                self.skill_macro_editor.assign_skill(skill_id);
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

    pub(super) fn click_key_config(&mut self, cursor: Point) {
        let panel = SpriteRect {
            x: KEY_CONFIG_PANEL_X,
            y: KEY_CONFIG_PANEL_Y,
            width: KEY_CONFIG_PANEL_WIDTH as f32,
            height: KEY_CONFIG_PANEL_HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        };
        if !rect_contains(panel, cursor) {
            return;
        }
        let local_x = cursor.x - panel.x;
        let local_y = cursor.y - panel.y;
        if local_x >= panel.width - 42.0 && local_y < 42.0 {
            self.show_key_config = false;
            self.key_config_capture = None;
            self.quickslot_key_capture = None;
        } else if let Some(slot) = quickslot_key_slot_at(local_x, local_y) {
            self.begin_quickslot_key_capture(slot);
        } else {
            let fallback;
            let bindings = if let Some(bindings) = self.server_keymap.as_deref() {
                bindings
            } else {
                fallback = default_keymap_bindings();
                fallback.as_slice()
            };

            // Clicking a bound key selects its action. The next physical key
            // press assigns that action to the new key.
            let bound_action = bindings
                .iter()
                .take(90)
                .enumerate()
                .find(|(key, binding)| {
                    binding.kind != 0
                        && keycap_position(*key as u8).is_some_and(|(x, y)| {
                            (x as f32..(x + 30) as f32).contains(&local_x)
                                && (y as f32..(y + 29) as f32).contains(&local_y)
                        })
                })
                .and_then(|(_, binding)| {
                    self.key_config_actions.iter().position(|action| {
                        action.kind == binding.kind && action.action == binding.action
                    })
                });

            if let Some(row) = bound_action {
                self.begin_key_config_capture(row);
            } else if let Some(row) = self.key_config_actions.iter().position(|action| {
                unbound_action_cell(action.kind, action.action).is_some_and(|(column, grid_row)| {
                    let x = 5 + column * 34;
                    let y = 267 + grid_row * 34;
                    (x as f32..(x + 34) as f32).contains(&local_x)
                        && (y as f32..(y + 34) as f32).contains(&local_y)
                })
            }) {
                self.begin_key_config_capture(row);
            }
        }
    }

    pub(super) fn assign_quickslot_key_capture(&mut self, key: u8) {
        let Some(slot) = self.quickslot_key_capture.take() else {
            return;
        };
        if key == 0 || key >= openstory_protocol::KEYMAP_ENTRY_COUNT as u8 {
            self.refresh_key_config_texture();
            return;
        }
        let previous = self.quickslot_keys[slot];
        for (other_slot, other_key) in self.quickslot_keys.iter_mut().enumerate() {
            if other_slot != slot && *other_key == key {
                *other_key = previous;
            }
        }
        self.quickslot_keys[slot] = key;
        self.quickslot_keys_customized = true;
        match save_quickslot_keys(self.quickslot_keys) {
            Ok(path) => eprintln!("OpenStoryRS: saved quick-slot keys to {}", path.display()),
            Err(error) => eprintln!("OpenStoryRS: could not save quick-slot keys: {error}"),
        }
        self.refresh_key_config_texture();
        self.refresh_quickslots_texture();
    }

    pub(super) fn scroll_key_config(&mut self, direction: isize) {
        self.key_config_scroll_row = self
            .key_config_scroll_row
            .saturating_add_signed(direction)
            .min(
                self.key_config_actions
                    .len()
                    .saturating_sub(KEY_CONFIG_VISIBLE_ROWS),
            );
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
            .server_keymap
            .clone()
            .unwrap_or_else(default_keymap_bindings);
        let mut updates = Vec::with_capacity(2);
        for (index, binding) in bindings.iter_mut().enumerate() {
            if binding.kind == kind && binding.action == action && index != usize::from(key) {
                *binding = openstory_protocol::KeyBinding::default();
                updates.push((index as u8, *binding));
            }
        }
        let binding = openstory_protocol::KeyBinding { kind, action };
        bindings[usize::from(key)] = binding;
        updates.push((key, binding));
        self.server_keymap = Some(bindings);
        self.send_key_config_updates(&updates);
        self.refresh_key_config_texture();
        self.refresh_quickslots_texture();
    }

    pub(super) fn clear_key_config_binding(&mut self, row: usize) {
        let Some(action_entry) = self.key_config_actions.get(row) else {
            return;
        };
        let (kind, action) = (action_entry.kind, action_entry.action);
        let mut bindings = self
            .server_keymap
            .clone()
            .unwrap_or_else(default_keymap_bindings);
        let mut updates = Vec::new();
        for (index, binding) in bindings.iter_mut().enumerate() {
            if binding.kind == kind && binding.action == action {
                *binding = openstory_protocol::KeyBinding::default();
                updates.push((index as u8, *binding));
            }
        }
        self.server_keymap = Some(bindings);
        self.key_config_actions_dirty = true;
        self.send_key_config_updates(&updates);
        self.refresh_key_config_texture();
        self.refresh_quickslots_texture();
    }

    pub(super) fn use_keybound_skill(&mut self, skill_id: i32) {
        let skill = self
            .skill_ui_entry_by_id(skill_id)
            .filter(|skill| skill.level > 0 && !skill.passive);
        if let Some(skill) = skill {
            self.use_skill(&skill);
        }
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
            return;
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
                return;
            }
        };
        if !self.channel_connected {
            self.push_chat_line(
                "[Quick slots] Connect to a field before changing bindings.".to_owned(),
                [255, 215, 128],
            );
            self.refresh_chat_texture();
            return;
        }
        let Some(key) = self.quickslot_keys.get(quickslot).copied() else {
            return;
        };
        let binding = openstory_protocol::KeyBinding {
            kind,
            action: item_id,
        };
        let mut bindings = self
            .server_keymap
            .clone()
            .unwrap_or_else(default_keymap_bindings);
        let Some(target) = bindings.get_mut(usize::from(key)) else {
            return;
        };
        *target = binding;
        self.server_keymap = Some(bindings);
        self.send_key_config_updates(&[(key, binding)]);
        self.key_config_actions_dirty = true;
        self.refresh_key_config_texture();
        self.refresh_quickslots_texture();
        self.refresh_inventory_texture();
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
                0 | 1 => {
                    let tab = if binding.action == 0 {
                        openstory_protocol::InventoryType::Equip
                    } else {
                        openstory_protocol::InventoryType::Use
                    };
                    self.open_inventory_tab(tab);
                }
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
                6 | 10 => {
                    self.chat_target = ChatTarget::Public;
                    self.chat_open = true;
                    self.refresh_chat_texture();
                }
                7 => self.toggle_minimap(),
                8 => {
                    self.show_quest_log = !self.show_quest_log;
                    self.refresh_quest_log_texture();
                }
                9 => self.toggle_key_config(),
                11 => self.open_new_whisper_window(),
                14 => self.toggle_status_menu(),
                12 => {
                    self.chat_target = ChatTarget::Party;
                    self.chat_open = true;
                    self.refresh_chat_texture();
                }
                13 => {
                    self.chat_target = ChatTarget::Buddy;
                    self.chat_open = true;
                    self.refresh_chat_texture();
                }
                15 => {
                    self.toggle_quickslots();
                }
                16 => {
                    self.chat_open = !self.chat_open;
                    self.refresh_chat_texture();
                }
                17 => {
                    self.show_guild_panel = !self.show_guild_panel;
                    self.refresh_guild_texture();
                }
                18 => {
                    self.chat_target = ChatTarget::Guild;
                    self.chat_open = true;
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
                _ => {}
            },
            5 => match binding.action {
                50 if pressed && !repeat => {
                    if let Some((object_id, x, y)) = self.map_scene.as_ref().and_then(|scene| {
                        scene.nearest_drop().map(|object_id| {
                            (
                                object_id,
                                rounded_i16(scene.player.x),
                                rounded_i16(scene.player.y),
                            )
                        })
                    }) && let Some(session) = self.session.as_ref()
                        && let Err(error) =
                            session.send_packet(openstory_protocol::pickup_item(object_id, x, y))
                    {
                        eprintln!("OpenStory: could not request item pickup: {error}");
                        self.channel_connected = false;
                    }
                }
                51 => self.gameplay_input.sit = pressed,
                52 if pressed && !repeat => self.try_local_basic_attack(),
                53 if pressed && !repeat => self.gameplay_input.jump_buffer = 12,
                54 if pressed && !repeat => self.request_nearest_npc_talk(),
                200 => self.gameplay_input.left = pressed,
                201 => self.gameplay_input.right = pressed,
                202 => self.gameplay_input.up = pressed,
                203 => self.gameplay_input.down = pressed,
                _ => {}
            },
            6 if pressed && !repeat => {
                self.send_face_expression(binding.action.saturating_sub(99));
            }
            _ => {}
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
                PhysicalKey::Code(KeyCode::Escape) => self.show_key_config = false,
                PhysicalKey::Code(KeyCode::Digit1) => {
                    self.begin_key_config_capture(self.key_config_scroll_row)
                }
                PhysicalKey::Code(KeyCode::Digit2) => {
                    self.begin_key_config_capture(self.key_config_scroll_row + 1)
                }
                PhysicalKey::Code(KeyCode::Digit3) => {
                    self.begin_key_config_capture(self.key_config_scroll_row + 2)
                }
                PhysicalKey::Code(KeyCode::Digit4) => {
                    self.begin_key_config_capture(self.key_config_scroll_row + 3)
                }
                PhysicalKey::Code(KeyCode::Digit5) => {
                    self.begin_key_config_capture(self.key_config_scroll_row + 4)
                }
                PhysicalKey::Code(KeyCode::Digit6) => {
                    self.begin_key_config_capture(self.key_config_scroll_row + 5)
                }
                PhysicalKey::Code(KeyCode::Digit7) => {
                    self.begin_key_config_capture(self.key_config_scroll_row + 6)
                }
                PhysicalKey::Code(KeyCode::Digit8) => {
                    self.begin_key_config_capture(self.key_config_scroll_row + 7)
                }
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
        if self.show_status {
            if state == ElementState::Pressed && !repeat {
                match key {
                    PhysicalKey::Code(KeyCode::Escape) => {
                        if self.status_auto_confirmation {
                            self.resolve_status_auto_allocate(false);
                        } else {
                            self.show_status = false;
                        }
                    }
                    PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter)
                        if self.status_auto_confirmation =>
                    {
                        self.resolve_status_auto_allocate(true);
                    }
                    _ => {}
                }
            }
            return true;
        }
        let pressed = state == ElementState::Pressed;
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
        if self.npc_shop.is_some() {
            return false;
        }
        if self.monster_book.open {
            if pressed && !repeat && matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
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
        if matches!(key, PhysicalKey::Code(KeyCode::KeyE)) {
            if pressed && !repeat {
                self.toggle_equipment_inventory();
            }
            return true;
        }
        if let Some(bindings) = self.server_keymap.as_ref() {
            if !matches!(key, PhysicalKey::Code(KeyCode::Escape))
                && let Some(maple_key) = maple_key_from_physical(key)
                && let Some(binding) = bindings.get(usize::from(maple_key)).copied()
                && binding.kind != 0
                && self.apply_keymap_binding(state, repeat, binding)
            {
                return true;
            }
            // Escape belongs to the shared UI/modal handler below. Returning from
            // this server-keymap branch used to bypass that handler and leave the
            // quit dialog waiting on an unrelated redraw.
            if !matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
                match key {
                    PhysicalKey::Code(KeyCode::ArrowLeft | KeyCode::KeyA) => {
                        self.gameplay_input.left = pressed;
                    }
                    PhysicalKey::Code(KeyCode::ArrowRight | KeyCode::KeyD) => {
                        self.gameplay_input.right = pressed;
                    }
                    PhysicalKey::Code(KeyCode::ArrowUp | KeyCode::KeyW) => {
                        self.gameplay_input.up = pressed;
                    }
                    PhysicalKey::Code(KeyCode::ArrowDown | KeyCode::KeyS) => {
                        self.gameplay_input.down = pressed;
                    }
                    _ => {}
                }
                return true;
            }
        }
        if pressed
            && !repeat
            && self
                .map_scene
                .as_ref()
                .is_some_and(MapSceneRuntime::is_locally_seated)
            && matches!(
                key,
                PhysicalKey::Code(
                    KeyCode::ArrowLeft
                        | KeyCode::ArrowRight
                        | KeyCode::KeyA
                        | KeyCode::KeyD
                        | KeyCode::AltLeft
                        | KeyCode::AltRight
                        | KeyCode::ControlLeft
                        | KeyCode::ControlRight
                )
            )
        {
            self.stand_from_local_seat();
        }
        if self.show_quest_log {
            if pressed && !repeat {
                match key {
                    PhysicalKey::Code(KeyCode::Escape | KeyCode::KeyJ) => {
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
        if self.show_pet_equipment {
            if matches!(key, PhysicalKey::Code(KeyCode::KeyP | KeyCode::Escape))
                && pressed
                && !repeat
            {
                self.show_pet_equipment = false;
            } else if pressed && !repeat {
                let scroll = match key {
                    PhysicalKey::Code(KeyCode::ArrowUp) => -1,
                    PhysicalKey::Code(KeyCode::ArrowDown) => 1,
                    PhysicalKey::Code(KeyCode::PageUp) => -5,
                    PhysicalKey::Code(KeyCode::PageDown) => 5,
                    _ => 0,
                };
                self.scroll_pet_equipment_by(scroll);
            }
            return true;
        }
        if pressed && !repeat {
            let expression = match key {
                PhysicalKey::Code(KeyCode::F1) => Some(1),
                PhysicalKey::Code(KeyCode::F2) => Some(2),
                PhysicalKey::Code(KeyCode::F3) => Some(3),
                PhysicalKey::Code(KeyCode::F4) => Some(4),
                PhysicalKey::Code(KeyCode::F5) => Some(5),
                PhysicalKey::Code(KeyCode::F6) => Some(6),
                PhysicalKey::Code(KeyCode::F7) => Some(7),
                _ => None,
            };
            if let Some(expression) = expression {
                self.send_face_expression(expression);
                return true;
            }
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
        match key {
            PhysicalKey::Code(KeyCode::KeyI) => {
                if pressed && !repeat {
                    self.show_equipment_inventory = false;
                    self.show_inventory = !self.show_inventory;
                    if self.show_inventory {
                        self.show_skill_book = false;
                        self.inventory_scroll_row = 0;
                        self.inventory_hover_slot = None;
                        self.update_inventory_hover();
                        self.refresh_inventory_texture();
                    } else {
                        self.inventory_drag_source = None;
                        self.inventory_hover_slot = None;
                        self.pending_scroll_slot = None;
                    }
                }
            }
            PhysicalKey::Code(KeyCode::KeyP) => {
                if pressed && !repeat {
                    self.show_pet_equipment = true;
                    self.pet_equipment_scroll_row = 0;
                    self.show_inventory = false;
                    self.inventory_hover_slot = None;
                    self.show_skill_book = false;
                    self.inventory_drag_source = None;
                    self.pending_scroll_slot = None;
                    self.refresh_pet_equipment_texture();
                }
            }
            PhysicalKey::Code(KeyCode::KeyX) => {
                if pressed
                    && !repeat
                    && self
                        .map_scene
                        .as_ref()
                        .is_some_and(|scene| scene.local_chair.is_some())
                {
                    self.gameplay_input.sit = false;
                    self.stand_from_local_seat();
                } else {
                    self.gameplay_input.sit = pressed;
                }
            }
            PhysicalKey::Code(KeyCode::KeyC) => {
                if pressed && !repeat {
                    self.open_status_window();
                }
            }
            PhysicalKey::Code(KeyCode::Backquote) => {
                if pressed && !repeat {
                    self.open_cash_shop();
                }
            }
            PhysicalKey::Code(KeyCode::KeyK) => {
                if pressed && !repeat {
                    self.show_skill_book = !self.show_skill_book;
                    if self.show_skill_book {
                        self.show_inventory = false;
                        self.inventory_hover_slot = None;
                        self.pending_scroll_slot = None;
                        self.skill_book_tab = 0;
                        self.skill_book_scroll_row = 0;
                        self.refresh_skill_book_entries();
                        self.refresh_skill_book_texture();
                    }
                }
            }
            PhysicalKey::Code(KeyCode::KeyJ) => {
                if pressed && !repeat {
                    self.show_quest_log = true;
                    self.show_inventory = false;
                    self.inventory_hover_slot = None;
                    self.show_pet_equipment = false;
                    self.show_skill_book = false;
                    self.inventory_drag_source = None;
                    self.pending_scroll_slot = None;
                    self.quest_log_tab = QuestLogTab::Active;
                    self.quest_log_scroll = 0;
                    self.quest_log_selected = 0;
                    self.quest_reward_selected = 0;
                    self.quest_forfeit_confirmation = None;
                    self.refresh_quest_log_texture();
                }
            }
            PhysicalKey::Code(KeyCode::KeyO) => {
                if pressed && !repeat {
                    self.show_party_panel = !self.show_party_panel;
                    if self.show_party_panel {
                        self.show_buddy_panel = false;
                        self.show_guild_panel = false;
                        self.show_alliance_panel = false;
                    }
                    self.refresh_party_texture();
                }
            }
            PhysicalKey::Code(KeyCode::KeyB) => {
                if pressed && !repeat {
                    self.show_buddy_panel = !self.show_buddy_panel;
                    if self.show_buddy_panel {
                        self.show_party_panel = false;
                        self.show_guild_panel = false;
                        self.show_alliance_panel = false;
                    }
                    self.refresh_buddy_texture();
                }
            }
            PhysicalKey::Code(KeyCode::KeyG) => {
                if pressed && !repeat {
                    self.show_guild_panel = !self.show_guild_panel;
                    if !self.show_guild_panel {
                        self.guild_panel_mode = GuildPanelMode::Overview;
                    }
                    if self.show_guild_panel {
                        self.show_party_panel = false;
                        self.show_buddy_panel = false;
                        self.show_alliance_panel = false;
                    }
                    self.refresh_guild_texture();
                }
            }
            PhysicalKey::Code(KeyCode::KeyU) => {
                if pressed && !repeat {
                    if self.show_alliance_panel {
                        self.show_alliance_panel = false;
                    } else {
                        self.open_alliance_panel();
                    }
                }
            }
            PhysicalKey::Code(KeyCode::KeyV) => {
                if pressed && !repeat {
                    self.open_guild_bbs();
                }
            }
            PhysicalKey::Code(KeyCode::Backslash) => {
                if pressed && !repeat {
                    self.toggle_key_config();
                }
            }
            PhysicalKey::Code(KeyCode::ArrowLeft | KeyCode::KeyA) => {
                self.gameplay_input.left = pressed;
            }
            PhysicalKey::Code(KeyCode::ArrowRight | KeyCode::KeyD) => {
                self.gameplay_input.right = pressed;
            }
            PhysicalKey::Code(KeyCode::ArrowUp | KeyCode::KeyW) => {
                self.gameplay_input.up = pressed;
            }
            PhysicalKey::Code(KeyCode::ArrowDown | KeyCode::KeyS) => {
                self.gameplay_input.down = pressed;
            }
            PhysicalKey::Code(KeyCode::Space) => {
                if pressed && !repeat {
                    self.request_nearest_npc_talk();
                }
            }
            PhysicalKey::Code(KeyCode::AltLeft | KeyCode::AltRight) => {
                if pressed && !repeat {
                    self.gameplay_input.jump_buffer = 12;
                }
            }
            PhysicalKey::Code(KeyCode::ControlLeft | KeyCode::ControlRight) => {
                if pressed && !repeat {
                    self.try_local_basic_attack();
                }
            }
            PhysicalKey::Code(KeyCode::KeyZ) => {
                if pressed && !repeat {
                    let pickup = self.map_scene.as_ref().and_then(|scene| {
                        scene.nearest_drop().map(|object_id| {
                            (
                                object_id,
                                rounded_i16(scene.player.x),
                                rounded_i16(scene.player.y),
                            )
                        })
                    });
                    if let Some((object_id, x, y)) = pickup {
                        if let Some(session) = self.session.as_ref() {
                            if let Err(error) = session
                                .send_packet(openstory_protocol::pickup_item(object_id, x, y))
                            {
                                eprintln!("OpenStory: could not request item pickup: {error}");
                                self.channel_connected = false;
                            }
                        }
                    }
                }
            }
            _ => return false,
        }
        true
    }

    pub(super) fn replace_scene(&mut self, scene: LoginSceneAssets) -> Result<(), String> {
        let LoginSceneAssets {
            sprites,
            actions,
            map_scene,
            animated_art,
            login_buttons,
            save_id_indicator_draws,
        } = scene;
        let renderer = self
            .renderer
            .as_ref()
            .ok_or_else(|| "graphics are not ready to change screens".to_owned())?;
        let (textures, mut draws, texture_indices) = upload_scene_sprites(renderer, sprites)?;
        let mut map_scene = map_scene.map(|scene| MapSceneRuntime::new(scene, &mut draws));
        if let Some(scene) = map_scene.as_mut() {
            scene
                .mark_npcs_scriptable(&self.scriptable_npc_ids.iter().copied().collect::<Vec<_>>());
        }
        self.sprite_textures = textures;
        self.texture_indices = texture_indices;
        self.remote_art_cache.clear();
        self.item_use_effect_cache.clear();
        self.item_aura_frames_cache.clear();
        self.face_expression_cache.clear();
        self.npc_dialogue = None;
        self.rps_game = None;
        self.inventory_drag_source = None;
        self.inventory_hover_slot = None;
        self.inventory_drop_prompt = None;
        self.inventory_drop_prompt_texture = None;
        self.death_notice = false;
        self.death_respawn_pending = false;
        self.death_notice_texture = None;
        self.pending_scroll_slot = None;
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
        if let Some(look) = self
            .active_character
            .as_ref()
            .map(|character| character.look.clone())
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
            self.refresh_map_hud();
        } else {
            self.map_hud_texture = None;
        }
        self.refresh_minimap_texture();
        if self.map_scene.is_none() {
            self.current_field = None;
            self.clear_clock();
            self.gameplay_input = GameplayInput::default();
            self.show_status_menu = false;
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
        Ok(())
    }
}
