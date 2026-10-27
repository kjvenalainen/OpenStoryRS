use super::*;

impl ApplicationHandler for ClientApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        debug_ui_stage("resumed; creating window");

        let attributes = Window::default_attributes()
            .with_title("OpenStoryRS")
            .with_resizable(true)
            .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("OpenStory: could not create a window: {error}");
                event_loop.exit();
                return;
            }
        };
        debug_ui_stage("window created; initializing renderer");
        window.set_ime_allowed(true);

        match pollster::block_on(GpuRenderer::new(
            window.clone(),
            LogicalSize::LEGACY_1024X768,
        )) {
            Ok(renderer) => {
                debug_ui_stage("renderer initialized; loading HUD assets");
                let (damage_texture, damage_glyphs) =
                    match load_damage_glyph_atlas(&self.effect_archive, &renderer) {
                        Ok(assets) => assets,
                        Err(error) => {
                            eprintln!("OpenStory: could not load NX damage digits: {error}");
                            event_loop.exit();
                            return;
                        }
                    };
                self.damage_glyph_texture = Some(damage_texture);
                self.damage_glyphs = damage_glyphs;
                debug_ui_stage("HUD assets loaded; uploading login artwork");
                let Some(sprites) = self.pending_sprites.take() else {
                    eprintln!("OpenStory: login artwork was already consumed");
                    event_loop.exit();
                    return;
                };
                let (textures, draws, texture_indices) =
                    match upload_scene_sprites(&renderer, sprites) {
                        Ok(uploaded) => uploaded,
                        Err(error) => {
                            eprintln!("OpenStory: could not upload login artwork: {error}");
                            event_loop.exit();
                            return;
                        }
                    };
                self.sprite_textures = textures;
                self.sprite_draws = draws;
                self.texture_indices = texture_indices;
                self.refresh_save_id_indicator();
                debug_ui_stage("login artwork uploaded; creating login controls");
                for (field, text) in [
                    (LoginField::Account, self.account_text.as_str()),
                    (LoginField::Password, self.password_text.as_str()),
                ] {
                    let text = if field == LoginField::Password {
                        "*".repeat(text.chars().count())
                    } else {
                        text.to_owned()
                    };
                    let pixels = rasterize_field(
                        &self.font,
                        &text,
                        self.focused_login_button.is_none() && self.focused_field == field,
                    );
                    match renderer.upload_bgra(FIELD_WIDTH, FIELD_HEIGHT, &pixels) {
                        Ok(texture) => self.field_textures.push(texture),
                        Err(error) => {
                            eprintln!("OpenStory: could not create login text field: {error}");
                            event_loop.exit();
                            return;
                        }
                    }
                }
                self.solid_textures = [
                    [0, 0, 0, 255],
                    [255, 255, 255, 255],
                    [144, 238, 144, 255],
                    [79, 168, 0, 255],
                ]
                .into_iter()
                .filter_map(|pixels| match renderer.upload_bgra(1, 1, &pixels) {
                    Ok(texture) => Some(texture),
                    Err(error) => {
                        eprintln!("OpenStory: could not create map UI texture: {error}");
                        event_loop.exit();
                        None
                    }
                })
                .collect();
                if self.solid_textures.len() != 4 {
                    return;
                }
                self.window = Some(window.clone());
                self.renderer = Some(renderer);
                self.load_native_status_bar();
                self.next_frame = Instant::now() + Duration::from_millis(16);
                self.refresh_login_button_visuals();
                window.request_redraw();
                debug_ui_stage("window ready");
            }
            Err(error) => {
                eprintln!("OpenStory: graphics initialization failed: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self
            .window
            .as_ref()
            .is_none_or(|window| window.id() != window_id)
        {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_physical = Some((position.x, position.y));
                let size = self
                    .window
                    .as_ref()
                    .expect("window is present for its own event")
                    .inner_size();
                self.cursor = Viewport::fit(
                    LogicalSize::LEGACY_1024X768,
                    PhysicalSize {
                        width: size.width,
                        height: size.height,
                    },
                )
                .and_then(|viewport| {
                    viewport.to_logical(Point {
                        x: position.x as f32,
                        y: position.y as f32,
                    })
                });
                if let Some(cursor) = self.cursor {
                    self.update_messenger_drag(cursor);
                    if self.update_world_map_hover(cursor)
                        && let Some(window) = self.window.as_ref()
                    {
                        window.request_redraw();
                    }
                }
                if self.npc_dialogue_scroll_dragging
                    && let Some(cursor) = self.cursor
                    && let Some(panel) = self.npc_dialogue.as_ref().map(|dialogue| dialogue.panel)
                {
                    let scroll_cursor = Point {
                        x: panel.x + 584.0,
                        y: cursor.y,
                    };
                    if self.set_npc_dialogue_scroll_from_cursor(scroll_cursor)
                        && let Some(window) = self.window.as_ref()
                    {
                        window.request_redraw();
                    }
                }
                if let (Some(slider), Some(cursor)) = (self.settings_dragging, self.cursor) {
                    self.set_option_setting_from_cursor(slider, cursor.x);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
                let hover_changed = self.update_inventory_hover();
                if self.inventory_drag_source.is_some() || hover_changed {
                    self.refresh_inventory_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
                if self.refresh_login_button_visuals()
                    && let Some(window) = self.window.as_ref()
                {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.messenger_drag_offset = None;
                self.npc_dialogue_scroll_dragging = false;
                if self.pressed_login_button.take().is_some()
                    && self.refresh_login_button_visuals()
                    && let Some(window) = self.window.as_ref()
                {
                    window.request_redraw();
                }
                if self.settings_dragging.take().is_some() {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.inventory_drag_source.is_some() {
                    self.finish_inventory_pointer(self.cursor);
                    if self.update_inventory_hover() {
                        self.refresh_inventory_texture();
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let Some(cursor) = self.cursor else {
                    return;
                };
                if self.quit_confirmation {
                    let accept = self.quit_confirmation_buttons[0]
                        .is_some_and(|bounds| rect_contains(bounds, cursor));
                    let cancel = self.quit_confirmation_buttons[1]
                        .is_some_and(|bounds| rect_contains(bounds, cursor));
                    self.quit_confirmation = false;
                    if accept {
                        self.quit_requested = true;
                        event_loop.exit();
                    } else if !cancel {
                        // Clicking outside the compact modal dismisses it, matching Escape.
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.click_character_selection(cursor) {
                    return;
                }
                if self.click_item_megaphone(cursor) {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.show_key_config {
                    self.click_key_config(cursor);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.skill_macro_editor.open {
                    self.click_skill_macro_editor(cursor);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.monster_book.open && self.monster_book.search_open {
                    return;
                }
                if self.click_channel_switch(cursor) {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.click_messenger(cursor) {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                self.messenger_chat_focused = false;
                if self.click_minimap(cursor) {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.death_notice {
                    if (464.0..560.0).contains(&cursor.x) && (412.0..448.0).contains(&cursor.y) {
                        self.request_death_respawn();
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.character_info.is_some() {
                    self.click_character_info(cursor);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.click_monster_book(cursor) {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.click_party_search(cursor) {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.click_world_map(cursor) {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.hired_merchant_room.is_some() {
                    if self.show_inventory
                        && self
                            .hired_merchant_room
                            .as_ref()
                            .is_none_or(|room| room.stock_source.is_none())
                        && rect_contains(
                            SpriteRect {
                                x: INVENTORY_PANEL_X,
                                y: INVENTORY_PANEL_Y,
                                width: INVENTORY_PANEL_WIDTH as f32,
                                height: INVENTORY_PANEL_HEIGHT as f32,
                                blend_mode: BlendMode::Alpha,
                            },
                            cursor,
                        )
                    {
                        self.begin_inventory_pointer(cursor);
                        self.refresh_inventory_texture();
                    } else {
                        self.click_hired_merchant_room(cursor);
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.trade_room.is_some() {
                    if self.show_inventory
                        && self
                            .trade_room
                            .as_ref()
                            .is_none_or(|room| room.pending_item.is_none())
                        && rect_contains(
                            SpriteRect {
                                x: INVENTORY_PANEL_X,
                                y: INVENTORY_PANEL_Y,
                                width: INVENTORY_PANEL_WIDTH as f32,
                                height: INVENTORY_PANEL_HEIGHT as f32,
                                blend_mode: BlendMode::Alpha,
                            },
                            cursor,
                        )
                    {
                        self.begin_inventory_pointer(cursor);
                        self.refresh_inventory_texture();
                    } else {
                        self.click_trade_room(cursor);
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.owl_search.is_some() {
                    self.click_owl_panel(cursor);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.show_settings {
                    self.click_settings(cursor);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.inventory_drop_prompt.is_some() {
                    self.click_inventory_drop_prompt(cursor);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.pic_entry.is_some() {
                    return;
                }
                if self.click_whisper_window(cursor) {
                    return;
                }
                if self.show_family_tree {
                    if cursor.x >= 223.0
                        && cursor.x < 223.0 + FAMILY_TREE_PANEL_WIDTH as f32
                        && cursor.y >= 191.0
                        && cursor.y < 191.0 + FAMILY_TREE_PANEL_HEIGHT as f32
                    {
                        self.click_family_tree(cursor);
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.show_family_panel {
                    if cursor.x >= 375.0
                        && cursor.x < 375.0 + FAMILY_PANEL_WIDTH as f32
                        && cursor.y >= 165.0
                        && cursor.y < 165.0 + FAMILY_PANEL_HEIGHT as f32
                    {
                        self.click_family_panel(cursor);
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.guild_bbs.is_some() {
                    self.click_guild_bbs(cursor);
                    self.refresh_guild_bbs_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.show_guild_panel {
                    if cursor.x >= 680.0
                        && cursor.x < 680.0 + GUILD_PANEL_WIDTH as f32
                        && cursor.y >= 174.0
                        && cursor.y < 174.0 + GUILD_PANEL_HEIGHT as f32
                    {
                        self.click_guild_panel(cursor);
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.show_alliance_panel
                    && (cursor.x >= 300.0 + ALLIANCE_PANEL_WIDTH as f32 - 34.0)
                    && (cursor.x < 300.0 + ALLIANCE_PANEL_WIDTH as f32)
                    && (cursor.y >= 190.0)
                    && (cursor.y < 222.0)
                {
                    self.show_alliance_panel = false;
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.show_status {
                    self.click_status_panel(cursor);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.storage.is_some() {
                    self.click_storage(cursor);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.show_quest_log && self.click_quest_log(cursor) {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.npc_shop.is_some() {
                    self.click_npc_shop(cursor);
                    return;
                }
                if self.npc_dialogue.is_some() {
                    self.click_npc_dialogue(cursor);
                    return;
                }
                if self.cash_shop.is_some() {
                    self.click_cash_shop(cursor);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.show_pet_equipment && self.begin_pet_equipment_pointer(cursor) {
                    return;
                }
                if self.show_skill_book && self.begin_skill_book_pointer(cursor) {
                    return;
                }
                if self.show_inventory && self.begin_inventory_pointer(cursor) {
                    return;
                }
                if self.click_native_status_bar(cursor) {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.click_quickslot(cursor) {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                // The account field overlaps the left edge of the Login
                // button in the legacy layout. Let an explicit button hit
                // win so clicks there submit instead of only moving field
                // focus.
                let login_button_at_cursor = self
                    .login_buttons
                    .iter()
                    .any(|button| rect_contains(button.hit_rect, cursor));
                if self.show_login_fields && !login_button_at_cursor && self.focus_at(cursor) {
                    return;
                }
                if self.focused_login_button.take().is_some() {
                    self.refresh_field_texture(self.focused_field);
                }
                self.pressed_login_button = self
                    .login_buttons
                    .iter()
                    .position(|button| rect_contains(button.hit_rect, cursor));
                if self.refresh_login_button_visuals()
                    && let Some(window) = self.window.as_ref()
                {
                    window.request_redraw();
                }
                let action = self
                    .login_buttons
                    .iter()
                    .position(|button| rect_contains(button.hit_rect, cursor))
                    .and_then(|index| self.login_button_action(index))
                    .or_else(|| {
                        self.login_actions
                            .iter()
                            .rev()
                            .find(|hit| rect_contains(hit.rect, cursor))
                            .map(|hit| hit.action)
                    });
                if let Some(action) = action {
                    if !self.login_action_enabled(action) {
                        return;
                    }
                    self.audio
                        .play_ui_sound(&self.sound_archive, "UI.img/BtMouseClick");
                    match action {
                        LoginAction::Quit => event_loop.exit(),
                        LoginAction::ToggleSaveId => self.toggle_save_account_id(),
                        LoginAction::SelectWorld(world_id) => self.select_world(world_id),
                        LoginAction::SelectChannel(channel_id) => self.select_channel(channel_id),
                        LoginAction::EnterWorld => self.request_characters(),
                        LoginAction::HighlightCharacter(index) => self.click_character_slot(index),
                        LoginAction::SelectCharacter(index) => self.select_character(index),
                        LoginAction::DeleteCharacter(index) => self.delete_character(index),
                        LoginAction::PreviousCharacterPage => self.change_character_page(-1),
                        LoginAction::NextCharacterPage => self.change_character_page(1),
                        LoginAction::BeginCharacterCreation => self.begin_character_creation(),
                        LoginAction::ChooseCreationClass(job) => self.choose_creation_class(job),
                        LoginAction::CycleCreationOption(option, direction) => {
                            self.cycle_creation_option(option, direction)
                        }
                        LoginAction::ToggleCreationGender(female) => {
                            self.toggle_creation_gender(female)
                        }
                        LoginAction::FocusCreationName => {
                            self.focus_creation_name();
                        }
                        LoginAction::SubmitCharacterCreation => {
                            self.submit_character_creation();
                        }
                        LoginAction::CancelCharacterCreation => {
                            self.cancel_character_creation();
                        }
                        LoginAction::ReturnToWorld => {
                            self.character_list = None;
                            self.refresh_world_scene();
                            self.show_login_fields = false;
                        }
                        action => self.activate_login(action),
                    }
                } else if let Some(owner_id) = self
                    .map_scene
                    .as_ref()
                    .and_then(|scene| scene.hired_merchant_at(cursor))
                {
                    self.request_hired_merchant_visit(owner_id);
                } else if let Some(object_id) = self
                    .map_scene
                    .as_ref()
                    .and_then(|scene| scene.personal_shop_at(cursor))
                {
                    self.request_personal_shop_visit(object_id);
                } else if let Some(object_id) = self
                    .map_scene
                    .as_ref()
                    .and_then(|scene| scene.scripted_npc_at(cursor))
                {
                    self.request_npc_talk(object_id);
                } else if let Some(character_id) = self
                    .map_scene
                    .as_ref()
                    .and_then(|scene| scene.remote_character_at(cursor))
                {
                    self.request_character_info(character_id);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(position) => position.y as f32,
                };
                if self.show_key_config {
                    if self.cursor.is_some_and(|cursor| {
                        rect_contains(
                            SpriteRect {
                                x: KEY_CONFIG_PANEL_X + 14.0,
                                y: KEY_CONFIG_PANEL_Y + KEY_CONFIG_ROW_TOP as f32,
                                width: KEY_CONFIG_PANEL_WIDTH as f32 - 28.0,
                                height: KEY_CONFIG_VISIBLE_ROWS as f32
                                    * KEY_CONFIG_ROW_HEIGHT as f32,
                                blend_mode: BlendMode::Alpha,
                            },
                            cursor,
                        )
                    }) {
                        self.scroll_key_config(if amount > 0.0 { -1 } else { 1 });
                        if let Some(window) = self.window.as_ref() {
                            window.request_redraw();
                        }
                    }
                    return;
                }
                if self.skill_macro_editor.open {
                    if let Some(cursor) = self.cursor {
                        let local_x = cursor.x - skill_macros::PANEL_X;
                        let local_y = cursor.y - skill_macros::PANEL_Y;
                        if skill_macros::skill_list_contains(local_x, local_y) {
                            self.skill_macro_editor.scroll_skills(
                                if amount > 0.0 { -1 } else { 1 },
                                self.skill_macro_choices().len(),
                            );
                            self.refresh_skill_macro_texture();
                            if let Some(window) = self.window.as_ref() {
                                window.request_redraw();
                            }
                        }
                    }
                    return;
                }
                if self
                    .cursor
                    .is_some_and(|cursor| self.scroll_world_map_results(cursor, amount))
                {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self
                    .cursor
                    .is_some_and(|cursor| self.scroll_monster_book_drops(cursor, amount))
                {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.monster_book.search_open {
                    return;
                }
                if self.monster_book.open
                    && self.monster_book.set_effect_open
                    && self.cursor.is_some_and(|cursor| {
                        (self.monster_book.x..self.monster_book.x + self.monster_book.width as f32)
                            .contains(&cursor.x)
                            && (self.monster_book.y
                                ..self.monster_book.y + self.monster_book.height as f32)
                                .contains(&cursor.y)
                    })
                {
                    self.monster_book.set_scroll = if amount > 0.0 {
                        self.monster_book.set_scroll.saturating_sub(1)
                    } else if amount < 0.0 {
                        self.monster_book
                            .set_scroll
                            .saturating_add(1)
                            .min(monster_book::set_scroll_max())
                    } else {
                        self.monster_book.set_scroll
                    };
                    self.refresh_monster_book_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if let Some(cursor) = self.cursor
                    && self.party_search.as_ref().is_some_and(|state| {
                        (state.x..state.x + party_search::PANEL_WIDTH as f32).contains(&cursor.x)
                            && (state.y..state.y + party_search::PANEL_HEIGHT as f32)
                                .contains(&cursor.y)
                    })
                {
                    let rows = if amount > 0.0 { -1 } else { 1 };
                    if let Some(state) = self.party_search.as_mut() {
                        state.scroll_by(rows);
                    }
                    self.refresh_party_search_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.hired_merchant_room.is_some() {
                    if let Some(cursor) = self.cursor {
                        if self.show_inventory
                            && (INVENTORY_PANEL_X..INVENTORY_PANEL_X + INVENTORY_PANEL_WIDTH as f32)
                                .contains(&cursor.x)
                            && (INVENTORY_PANEL_Y
                                ..INVENTORY_PANEL_Y + INVENTORY_PANEL_HEIGHT as f32)
                                .contains(&cursor.y)
                        {
                            if let Some(inventory) = self.inventory.as_ref() {
                                let maximum = inventory_slot_max(inventory, self.inventory_tab)
                                    .div_ceil(INVENTORY_SLOT_COLUMNS as u16)
                                    .saturating_sub(INVENTORY_SLOT_ROWS as u16);
                                self.inventory_scroll_row = if amount > 0.0 {
                                    self.inventory_scroll_row.saturating_sub(1)
                                } else {
                                    self.inventory_scroll_row.saturating_add(1).min(maximum)
                                };
                            }
                            self.refresh_inventory_texture();
                        } else {
                            self.scroll_hired_merchant_room(cursor, amount);
                        }
                        if let Some(window) = self.window.as_ref() {
                            window.request_redraw();
                        }
                    }
                } else if self.trade_room.is_some() {
                    if let Some(cursor) = self.cursor
                        && self.show_inventory
                        && (INVENTORY_PANEL_X..INVENTORY_PANEL_X + INVENTORY_PANEL_WIDTH as f32)
                            .contains(&cursor.x)
                        && (INVENTORY_PANEL_Y..INVENTORY_PANEL_Y + INVENTORY_PANEL_HEIGHT as f32)
                            .contains(&cursor.y)
                    {
                        if let Some(inventory) = self.inventory.as_ref() {
                            let maximum = inventory_slot_max(inventory, self.inventory_tab)
                                .div_ceil(INVENTORY_SLOT_COLUMNS as u16)
                                .saturating_sub(INVENTORY_SLOT_ROWS as u16);
                            self.inventory_scroll_row = if amount > 0.0 {
                                self.inventory_scroll_row.saturating_sub(1)
                            } else {
                                self.inventory_scroll_row.saturating_add(1).min(maximum)
                            };
                        }
                        self.refresh_inventory_texture();
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                } else if self.owl_search.is_some() {
                    if let Some(owl) = self.owl_search.as_mut()
                        && owl.result_item_id.is_some()
                        && !owl.results.is_empty()
                    {
                        let maximum = owl.results.len().saturating_sub(8);
                        owl.result_offset = if amount > 0.0 {
                            owl.result_offset.saturating_sub(1)
                        } else {
                            owl.result_offset.saturating_add(1).min(maximum)
                        };
                        owl.selected_result = Some(owl.result_offset);
                        self.refresh_owl_texture();
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.show_pet_equipment
                    && self.cursor.is_some_and(|cursor| {
                        (634.0..1002.0).contains(&cursor.x) && (348.0..588.0).contains(&cursor.y)
                    })
                {
                    let scroll = if amount > 0.0 {
                        -1
                    } else if amount < 0.0 {
                        1
                    } else {
                        0
                    };
                    self.scroll_pet_equipment_by(scroll);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                let scroll_npc_choices = self.npc_dialogue.as_ref().is_some_and(|dialogue| {
                    dialogue.state.message_type == 4
                        && dialogue.selections.len() > 5
                        && self
                            .cursor
                            .is_some_and(|cursor| rect_contains(dialogue.panel, cursor))
                });
                if scroll_npc_choices {
                    self.move_npc_dialogue_selection(if amount > 0.0 { -1 } else { 1 });
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                } else if self.npc_dialogue.as_ref().is_some_and(|dialogue| {
                    dialogue.state.message_type != 4
                        && dialogue.text_max_scroll > 0
                        && self
                            .cursor
                            .is_some_and(|cursor| rect_contains(dialogue.panel, cursor))
                }) {
                    self.scroll_npc_dialogue_text(if amount > 0.0 { -1 } else { 1 });
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                } else if self.storage.is_some()
                    && self.cursor.is_some_and(|cursor| {
                        (182.0..842.0).contains(&cursor.x) && (230.0..534.0).contains(&cursor.y)
                    })
                {
                    if let Some(cursor) = self.cursor {
                        self.scroll_storage(cursor, amount);
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                } else if self.show_quest_log
                    && self.cursor.is_some_and(|cursor| {
                        (QUEST_LOG_PANEL_X + 8.0..QUEST_LOG_PANEL_X + 288.0).contains(&cursor.x)
                            && (QUEST_LOG_PANEL_Y + 50.0..QUEST_LOG_PANEL_Y + 254.0)
                                .contains(&cursor.y)
                    })
                {
                    let count = quest_ids_for_tab(
                        &self.quest_log,
                        &self.available_quests,
                        self.quest_log_tab,
                    )
                    .len();
                    let maximum = count.saturating_sub(QUEST_LOG_VISIBLE_ROWS);
                    self.quest_log_scroll = if amount > 0.0 {
                        self.quest_log_scroll.saturating_sub(1)
                    } else {
                        self.quest_log_scroll.saturating_add(1).min(maximum)
                    };
                    self.quest_log_selected = self.quest_log_scroll;
                    self.quest_reward_selected = 0;
                    self.refresh_quest_log_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                } else if self.show_buddy_panel
                    && self.cursor.is_some_and(|cursor| {
                        (728.0..1012.0).contains(&cursor.x) && (190.0..482.0).contains(&cursor.y)
                    })
                {
                    let visible_rows = if self.pending_buddy_invite.is_some() {
                        5
                    } else {
                        7
                    };
                    let maximum = self.buddy_display_count().saturating_sub(visible_rows);
                    self.buddy_scroll = if amount > 0.0 {
                        self.buddy_scroll.saturating_sub(1)
                    } else {
                        self.buddy_scroll.saturating_add(1).min(maximum)
                    };
                    self.refresh_buddy_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                } else if self.show_guild_panel
                    && self.cursor.is_some_and(|cursor| {
                        (680.0..1016.0).contains(&cursor.x) && (174.0..594.0).contains(&cursor.y)
                    })
                {
                    let maximum = self
                        .guild_info
                        .as_ref()
                        .map_or(0, |guild| guild.members.len().saturating_sub(12));
                    self.guild_scroll = if amount > 0.0 {
                        self.guild_scroll.saturating_sub(1)
                    } else {
                        self.guild_scroll.saturating_add(1).min(maximum)
                    };
                    self.refresh_guild_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                } else if self.cash_shop.is_some()
                    && self.cursor.is_some_and(|cursor| {
                        (198.0..1004.0).contains(&cursor.x) && (78.0..610.0).contains(&cursor.y)
                    })
                {
                    let max_page = self.cash_shop.as_ref().map_or(0, |shop| {
                        cash_filtered_indices(
                            &self.cash_products,
                            shop,
                            &self.string_archive,
                            &mut self.item_name_cache,
                        )
                        .len()
                        .saturating_sub(1)
                            / CASH_SHOP_PAGE_SIZE
                    });
                    if let Some(shop) = self.cash_shop.as_mut() {
                        shop.page = if amount > 0.0 {
                            shop.page.saturating_sub(1)
                        } else {
                            (shop.page + 1).min(max_page)
                        };
                    }
                    self.refresh_cash_shop_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                } else if self.npc_shop.is_some()
                    && self.cursor.is_some_and(|cursor| {
                        (202.0..822.0).contains(&cursor.x) && (124.0..644.0).contains(&cursor.y)
                    })
                {
                    self.scroll_npc_shop(amount);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                } else if self.show_inventory
                    && self.cursor.is_some_and(|cursor| {
                        (INVENTORY_PANEL_X..INVENTORY_PANEL_X + INVENTORY_PANEL_WIDTH as f32)
                            .contains(&cursor.x)
                            && (INVENTORY_PANEL_Y
                                ..INVENTORY_PANEL_Y + INVENTORY_PANEL_HEIGHT as f32)
                                .contains(&cursor.y)
                    })
                {
                    self.scroll_inventory(amount);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                } else if self.show_skill_book
                    && self.cursor.is_some_and(|cursor| {
                        (586.0..1016.0).contains(&cursor.x) && (128.0..628.0).contains(&cursor.y)
                    })
                {
                    let direction = if amount > 0.0 { -1_isize } else { 1 };
                    let maximum = self.skill_book_entries.len().saturating_sub(10);
                    self.skill_book_scroll_row = self
                        .skill_book_scroll_row
                        .saturating_add_signed(direction)
                        .min(maximum);
                    self.refresh_skill_book_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event: mut event, ..
            } => {
                // Some macOS input paths report the logical Escape key while
                // leaving physical_key unidentified. Normalize it once so every
                // modal and the gameplay quit handler see the same key.
                if matches!(&event.logical_key, Key::Named(NamedKey::Escape)) {
                    event.physical_key = PhysicalKey::Code(KeyCode::Escape);
                }
                if event.state == ElementState::Pressed && !event.repeat {
                    self.show_status_menu = false;
                }
                // A text event is followed by Ime::Commit on some backends. Keep the
                // candidate only until the next keyboard event so a matching commit
                // cannot insert ordinary key text twice.
                self.login_key_text_for_ime_dedup = None;
                self.character_creation_key_text_for_ime_dedup = None;
                self.skill_macro_editor.key_text_for_ime_dedup = None;
                if let Some(input) = self.family_ui.input.as_mut() {
                    input.key_text_for_ime_dedup = None;
                }
                if self.family_ui.input.is_some() || self.family_ui.confirm_leave {
                    if event.state == ElementState::Pressed {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::Escape) if !event.repeat => {
                                self.family_ui.input = None;
                                self.family_ui.confirm_leave = false;
                            }
                            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter)
                                if !event.repeat =>
                            {
                                if self.family_ui.input.is_some() {
                                    self.submit_family_input();
                                } else {
                                    self.resolve_family_leave(true);
                                }
                            }
                            PhysicalKey::Code(KeyCode::Backspace)
                                if self.family_ui.input.is_some() =>
                            {
                                self.backspace_family_input();
                            }
                            _ => {
                                if self.family_ui.input.is_some()
                                    && let Some(text) = event.text.as_deref().filter(|text| {
                                        text.chars().any(|character| !character.is_control())
                                    })
                                {
                                    self.edit_family_input(text);
                                    if let Some(input) = self.family_ui.input.as_mut() {
                                        input.key_text_for_ime_dedup = Some(text.to_owned());
                                    }
                                }
                            }
                        }
                    }
                    self.refresh_family_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.skill_macro_editor.open {
                    self.handle_skill_macro_editor_key(
                        event.physical_key,
                        event.state,
                        event.repeat,
                        event.text.as_deref(),
                    );
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.monster_book.open && self.monster_book.search_open {
                    if event.state == ElementState::Pressed {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::Escape) if !event.repeat => {
                                self.monster_book.search_open = false;
                                self.monster_book.search_notice = None;
                                self.refresh_monster_book_texture();
                            }
                            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter)
                                if !event.repeat =>
                            {
                                self.submit_monster_book_search();
                            }
                            PhysicalKey::Code(KeyCode::Backspace) => {
                                self.monster_book.search_query.pop();
                                self.monster_book.search_notice = None;
                                self.refresh_monster_book_texture();
                            }
                            _ => {}
                        }
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.death_notice {
                    if event.state == ElementState::Pressed && !event.repeat {
                        match event.physical_key {
                            PhysicalKey::Code(
                                KeyCode::Enter
                                | KeyCode::NumpadEnter
                                | KeyCode::Escape
                                | KeyCode::Space,
                            ) => self.request_death_respawn(),
                            _ => {}
                        }
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.character_info.is_some() {
                    if event.state == ElementState::Pressed
                        && !event.repeat
                        && event.physical_key == PhysicalKey::Code(KeyCode::Escape)
                    {
                        self.character_info = None;
                        self.character_info_texture = None;
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.owl_search.is_some() {
                    if event.state == ElementState::Pressed && !event.repeat {
                        self.handle_owl_key(event.physical_key);
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.show_guild_panel
                    && !matches!(&self.guild_panel_mode, GuildPanelMode::Overview)
                {
                    if event.state == ElementState::Pressed {
                        self.handle_guild_panel_key(event.physical_key, event.repeat);
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                // NPC dialogue is a modal interaction. Handle its keys before
                // inventory/storage/settings shortcuts so Space cannot be
                // swallowed by another gameplay UI or interpreted as jump.
                if let Some(message_type) = self
                    .npc_dialogue
                    .as_ref()
                    .map(|dialogue| dialogue.state.message_type)
                {
                    if event.state == ElementState::Pressed {
                        let is_space =
                            matches!(event.physical_key, PhysicalKey::Code(KeyCode::Space))
                                || matches!(&event.logical_key, Key::Named(NamedKey::Space))
                                || event.text.as_deref() == Some(" ");
                        if is_space && !event.repeat {
                            // Let Space skip the prompt's typewriter reveal even
                            // for text-entry dialogs. Once the prompt is visible,
                            // Space remains available as an ordinary input character.
                            let revealed = self.fast_forward_npc_dialogue_text();
                            if !revealed && message_type != 2 {
                                self.advance_npc_dialogue();
                            }
                        } else {
                            match event.physical_key {
                                PhysicalKey::Code(KeyCode::Escape) if !event.repeat => {
                                    self.cancel_npc_dialogue();
                                }
                                PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter)
                                    if !event.repeat =>
                                {
                                    self.advance_npc_dialogue();
                                }
                                PhysicalKey::Code(KeyCode::Backspace)
                                    if matches!(message_type, 2 | 3) =>
                                {
                                    if let Some(dialogue) = self.npc_dialogue.as_mut() {
                                        dialogue.input.pop();
                                    }
                                    self.rebuild_npc_dialogue();
                                }
                                PhysicalKey::Code(KeyCode::ArrowUp | KeyCode::ArrowDown)
                                    if message_type == 4 && !event.repeat =>
                                {
                                    let direction = match event.physical_key {
                                        PhysicalKey::Code(KeyCode::ArrowUp) => -1,
                                        _ => 1,
                                    };
                                    self.move_npc_dialogue_selection(direction);
                                }
                                PhysicalKey::Code(KeyCode::PageUp | KeyCode::PageDown)
                                    if message_type != 4 && !event.repeat =>
                                {
                                    let direction = match event.physical_key {
                                        PhysicalKey::Code(KeyCode::PageUp) => -1,
                                        _ => 1,
                                    };
                                    self.scroll_npc_dialogue_text(direction * 5);
                                }
                                _ => {}
                            }
                        }
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    // Text input still arrives through WindowEvent::Ime below.
                    return;
                }
                if self.hired_merchant_room.is_some() {
                    if event.state == ElementState::Pressed {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::KeyI) if !event.repeat => {
                                self.show_inventory = !self.show_inventory;
                                if self.show_inventory {
                                    self.inventory_scroll_row = 0;
                                    self.update_inventory_hover();
                                    self.refresh_inventory_texture();
                                } else {
                                    self.inventory_drag_source = None;
                                    self.inventory_hover_slot = None;
                                }
                            }
                            PhysicalKey::Code(KeyCode::Escape) if !event.repeat => {
                                if self
                                    .hired_merchant_room
                                    .as_ref()
                                    .is_some_and(|room| room.stock_source.is_some())
                                {
                                    if let Some(room) = self.hired_merchant_room.as_mut() {
                                        room.stock_source = None;
                                    }
                                    self.refresh_hired_merchant_room_texture();
                                } else {
                                    self.leave_hired_merchant_room(false);
                                }
                            }
                            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter)
                                if !event.repeat =>
                            {
                                if self
                                    .hired_merchant_room
                                    .as_ref()
                                    .is_some_and(|room| room.stock_source.is_some())
                                {
                                    self.submit_hired_merchant_stock();
                                } else if self
                                    .hired_merchant_room
                                    .as_ref()
                                    .is_some_and(|room| room.chat_focused)
                                {
                                    self.send_hired_merchant_chat();
                                } else {
                                    self.buy_selected_hired_merchant_item();
                                }
                            }
                            PhysicalKey::Code(KeyCode::Backspace) => {
                                if let Some(room) = self.hired_merchant_room.as_mut()
                                    && room.stock_source.is_some()
                                {
                                    room.stock_price.pop();
                                    self.refresh_hired_merchant_room_texture();
                                } else if let Some(room) = self.hired_merchant_room.as_mut()
                                    && room.chat_focused
                                {
                                    room.chat_input.pop();
                                    self.refresh_hired_merchant_room_texture();
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.trade_room.is_some() {
                    if event.state == ElementState::Pressed {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::KeyI) if !event.repeat => {
                                self.show_inventory = !self.show_inventory;
                                if self.show_inventory {
                                    self.inventory_scroll_row = 0;
                                    self.update_inventory_hover();
                                    self.refresh_inventory_texture();
                                } else {
                                    self.inventory_drag_source = None;
                                    self.inventory_hover_slot = None;
                                }
                            }
                            PhysicalKey::Code(KeyCode::Escape) if !event.repeat => {
                                if self
                                    .trade_room
                                    .as_ref()
                                    .is_some_and(|room| room.pending_item.is_some())
                                {
                                    if let Some(room) = self.trade_room.as_mut() {
                                        room.pending_item = None;
                                        room.quantity_input.clear();
                                        room.notice = None;
                                    }
                                    self.refresh_trade_texture();
                                } else if self.send_trade_action(
                                    openstory_protocol::trade_exit_packet(),
                                    "cancel the trade",
                                ) {
                                    self.trade_room = None;
                                    self.trade_texture = None;
                                    self.show_inventory = false;
                                }
                            }
                            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter)
                                if !event.repeat =>
                            {
                                let (pending, meso_focused, chat_focused) = self
                                    .trade_room
                                    .as_ref()
                                    .map(|room| {
                                        (
                                            room.pending_item.is_some(),
                                            room.meso_focused,
                                            room.chat_focused,
                                        )
                                    })
                                    .unwrap_or_default();
                                if pending {
                                    self.submit_trade_quantity();
                                } else if meso_focused {
                                    self.submit_trade_meso();
                                } else if chat_focused {
                                    self.send_trade_chat();
                                } else {
                                    self.click_trade_room(Point {
                                        x: TRADE_PANEL_X + 20.0,
                                        y: TRADE_PANEL_Y + 568.0,
                                    });
                                }
                            }
                            PhysicalKey::Code(KeyCode::Backspace) => {
                                if let Some(room) = self.trade_room.as_mut() {
                                    if room.pending_item.is_some() {
                                        room.quantity_input.pop();
                                    } else if room.meso_focused {
                                        room.meso_input.pop();
                                    } else if room.chat_focused {
                                        room.chat_input.pop();
                                    }
                                }
                                self.refresh_trade_texture();
                            }
                            _ => {}
                        }
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.whisper_input_focused && self.active_whisper.is_some() {
                    if event.state == ElementState::Pressed {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::Escape) if !event.repeat => {
                                self.whisper_input_focused = false;
                                self.whisper_target_focused = false;
                                self.refresh_whisper_texture();
                            }
                            PhysicalKey::Code(KeyCode::Tab) if !event.repeat => {
                                if let Some(index) = self.active_whisper
                                    && let Some(conversation) =
                                        self.whisper_conversations.get(index)
                                    && conversation.target.is_empty()
                                {
                                    self.whisper_target_focused = !self.whisper_target_focused;
                                } else {
                                    self.whisper_target_focused = false;
                                }
                                self.refresh_whisper_texture();
                            }
                            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter)
                                if !event.repeat =>
                            {
                                if self.whisper_target_focused {
                                    self.whisper_target_focused = false;
                                    self.refresh_whisper_texture();
                                } else {
                                    self.submit_whisper_input();
                                }
                            }
                            PhysicalKey::Code(KeyCode::Backspace) => {
                                if let Some(index) = self.active_whisper
                                    && let Some(conversation) =
                                        self.whisper_conversations.get_mut(index)
                                {
                                    if self.whisper_target_focused {
                                        conversation.target.pop();
                                    } else {
                                        conversation.input.pop();
                                    }
                                }
                                self.refresh_whisper_texture();
                            }
                            _ => {}
                        }
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.guild_bbs.is_some() {
                    if event.state == ElementState::Pressed {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::Escape) if !event.repeat => {
                                let mut close_board = false;
                                if let Some(bbs) = self.guild_bbs.as_mut() {
                                    if bbs.draft.take().is_some() {
                                        bbs.feedback = None;
                                    } else if bbs.confirm_delete {
                                        bbs.confirm_delete = false;
                                    } else {
                                        close_board = true;
                                    }
                                }
                                if close_board {
                                    self.guild_bbs = None;
                                }
                            }
                            PhysicalKey::Code(KeyCode::ArrowLeft) if !event.repeat => {
                                if self.guild_bbs.as_ref().is_some_and(|bbs| {
                                    bbs.draft.is_none() && !bbs.reply_focused && !bbs.confirm_delete
                                }) {
                                    let page = self.guild_bbs.as_ref().map_or(0, |bbs| bbs.page);
                                    self.request_guild_bbs_page(page.saturating_sub(1));
                                }
                            }
                            PhysicalKey::Code(KeyCode::ArrowRight) if !event.repeat => {
                                if self.guild_bbs.as_ref().is_some_and(|bbs| {
                                    bbs.draft.is_none() && !bbs.reply_focused && !bbs.confirm_delete
                                }) {
                                    let page = self.guild_bbs.as_ref().map_or(0, |bbs| bbs.page);
                                    self.request_guild_bbs_page(page.saturating_add(1));
                                }
                            }
                            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter)
                                if !event.repeat =>
                            {
                                let focus = self
                                    .guild_bbs
                                    .as_ref()
                                    .and_then(|bbs| bbs.draft.as_ref())
                                    .map(|draft| draft.focus);
                                match focus {
                                    Some(GuildBbsDraftFocus::Title) => {
                                        if let Some(draft) = self
                                            .guild_bbs
                                            .as_mut()
                                            .and_then(|bbs| bbs.draft.as_mut())
                                        {
                                            draft.focus = GuildBbsDraftFocus::Content;
                                        }
                                    }
                                    Some(GuildBbsDraftFocus::Content) => {
                                        // The legacy content field wraps automatically and
                                        // treats Enter as a no-op; Publish submits the post.
                                    }
                                    None => {
                                        if self
                                            .guild_bbs
                                            .as_ref()
                                            .is_some_and(|bbs| bbs.reply_focused)
                                        {
                                            self.send_guild_bbs_reply();
                                        }
                                    }
                                }
                            }
                            PhysicalKey::Code(KeyCode::Backspace) => {
                                if let Some(bbs) = self.guild_bbs.as_mut() {
                                    if let Some(draft) = bbs.draft.as_mut() {
                                        match draft.focus {
                                            GuildBbsDraftFocus::Title => {
                                                draft.title.pop();
                                            }
                                            GuildBbsDraftFocus::Content => {
                                                draft.content.pop();
                                            }
                                        }
                                    } else if bbs.reply_focused {
                                        bbs.reply_input.pop();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    self.refresh_guild_bbs_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.inventory_drop_prompt.is_some() {
                    if event.state == ElementState::Pressed && !event.repeat {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::Escape) => {
                                self.inventory_drop_prompt = None;
                                self.inventory_drop_prompt_texture = None;
                            }
                            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                                self.resolve_inventory_drop_prompt(true);
                            }
                            PhysicalKey::Code(KeyCode::Backspace) => {
                                if let Some(prompt) = self.inventory_drop_prompt.as_mut() {
                                    if prompt.step == InventoryDropStep::Quantity {
                                        prompt.quantity.pop();
                                        prompt.notice = None;
                                    }
                                }
                                self.refresh_inventory_drop_prompt_texture();
                            }
                            _ => {}
                        }
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.storage.is_some() {
                    if event.state == ElementState::Pressed && !event.repeat {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::Escape) => self.close_storage(),
                            PhysicalKey::Code(KeyCode::Backspace) => {
                                if let Some(storage) = self.storage.as_mut()
                                    && storage.meso_focused
                                {
                                    storage.meso_input.pop();
                                    storage.notice = None;
                                }
                                self.refresh_storage_texture();
                            }
                            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                                if let Some(storage) = self.storage.as_mut() {
                                    storage.meso_focused = false;
                                }
                                self.refresh_storage_texture();
                            }
                            _ => {
                                let focused = self
                                    .storage
                                    .as_ref()
                                    .is_some_and(|storage| storage.meso_focused);
                                if focused
                                    && let Some(text) = event.text.as_deref()
                                    && let Some(storage) = self.storage.as_mut()
                                {
                                    for character in text.chars().filter(char::is_ascii_digit) {
                                        if storage.meso_input.len() >= 10 {
                                            break;
                                        }
                                        let mut candidate = storage.meso_input.clone();
                                        candidate.push(character);
                                        if candidate.parse::<i32>().is_ok() {
                                            storage.meso_input = candidate;
                                        }
                                    }
                                    storage.notice = None;
                                    self.refresh_storage_texture();
                                }
                            }
                        }
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.show_settings {
                    if event.state == ElementState::Pressed && !event.repeat {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::Escape) => self.cancel_settings(),
                            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                                self.save_settings()
                            }
                            PhysicalKey::Code(KeyCode::Tab) => {
                                self.settings_selected_slider = match self.settings_selected_slider
                                {
                                    OptionSlider::Music => OptionSlider::Effects,
                                    OptionSlider::Effects => OptionSlider::HpWarning,
                                    OptionSlider::HpWarning => OptionSlider::MpWarning,
                                    OptionSlider::MpWarning => OptionSlider::Music,
                                };
                                self.refresh_settings_texture();
                            }
                            PhysicalKey::Code(KeyCode::ArrowLeft | KeyCode::ArrowDown) => {
                                self.adjust_selected_option_setting(-5)
                            }
                            PhysicalKey::Code(KeyCode::ArrowRight | KeyCode::ArrowUp) => {
                                self.adjust_selected_option_setting(5)
                            }
                            _ => {}
                        }
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.pic_entry.is_some() {
                    if event.state != ElementState::Pressed || event.repeat {
                        return;
                    }
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Escape) => {
                            self.pic_entry = None;
                            self.pic_dialog_texture = None;
                        }
                        PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                            self.submit_pic_entry();
                        }
                        PhysicalKey::Code(KeyCode::Backspace) => {
                            if let Some(entry) = self.pic_entry.as_mut() {
                                entry.input.pop();
                                entry.notice = None;
                            }
                            self.refresh_pic_dialog_texture();
                        }
                        _ => {}
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.character_creation.is_some() {
                    let mut creation_name_changed = false;
                    if event.state == ElementState::Pressed
                        && !event.repeat
                        && let Some(text) = event.text.as_deref().filter(|text| {
                            text.chars().any(|character| character.is_ascii_graphic())
                        })
                        && let Some(creation) = self.character_creation.as_mut()
                        && creation.name_focused
                        && !creation.awaiting_name
                        && !creation.awaiting_create
                    {
                        self.character_creation_key_text_for_ime_dedup = Some(text.to_owned());
                        for character in text.chars() {
                            if character.is_ascii_graphic() && creation.name.chars().count() < 12 {
                                creation.name.push(character);
                            }
                        }
                        creation.notice = None;
                        creation_name_changed = true;
                    }
                    if event.state != ElementState::Pressed || event.repeat {
                        return;
                    }
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Escape) => self.cancel_character_creation(),
                        PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                            self.submit_character_creation()
                        }
                        PhysicalKey::Code(KeyCode::Backspace) => {
                            if let Some(creation) = self.character_creation.as_mut() {
                                if creation.name_focused
                                    && !creation.awaiting_name
                                    && !creation.awaiting_create
                                {
                                    creation.name.pop();
                                    creation.notice = None;
                                    self.reset_login_caret();
                                }
                            }
                            self.refresh_character_creation_scene();
                        }
                        _ => {}
                    }
                    if creation_name_changed {
                        self.reset_login_caret();
                        self.refresh_character_creation_scene();
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.chat_open
                    && self.current_field.is_some()
                    && self.npc_dialogue.is_none()
                    && self.npc_shop.is_none()
                    && self.cash_shop.is_none()
                {
                    if event.state == ElementState::Pressed && !event.repeat {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::Escape) => {
                                self.chat_open = false;
                                self.cash_item_compose = None;
                                self.refresh_chat_texture();
                            }
                            PhysicalKey::Code(KeyCode::Tab) => {
                                let whisper = self.cash_item_compose.as_mut().and_then(|compose| {
                                    if compose.kind.has_whisper_toggle(compose.item_id) {
                                        compose.whisper = !compose.whisper;
                                        Some(compose.whisper)
                                    } else {
                                        None
                                    }
                                });
                                if let Some(enabled) = whisper {
                                    self.push_chat_line(
                                        format!(
                                            "[Megaphone] Whisper icon {}.",
                                            if enabled { "enabled" } else { "disabled" }
                                        ),
                                        [255, 222, 112],
                                    );
                                } else if self.cash_item_compose.is_none() {
                                    self.chat_target = self.chat_target.next();
                                }
                                self.refresh_chat_texture();
                            }
                            PhysicalKey::Code(KeyCode::KeyI)
                                if self.cash_item_compose.as_ref().is_some_and(|compose| {
                                    compose.awaiting_attachment
                                        && compose.kind.uses_item_attachment(compose.item_id)
                                }) =>
                            {
                                self.show_inventory = !self.show_inventory;
                                if self.show_inventory {
                                    self.inventory_scroll_row = 0;
                                    self.update_inventory_hover();
                                    self.refresh_inventory_texture();
                                } else {
                                    self.inventory_drag_source = None;
                                    self.inventory_hover_slot = None;
                                }
                            }
                            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                                self.submit_chat_message();
                            }
                            PhysicalKey::Code(KeyCode::Backspace) => {
                                self.chat_input.pop();
                                self.refresh_chat_texture();
                            }
                            _ => {}
                        }
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if (self.pending_guild_invite.is_some()
                    || self.pending_guild_creation_invite.is_some())
                    && event.state == ElementState::Pressed
                    && !event.repeat
                {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::KeyY) => {
                            if self.pending_guild_invite.is_some() {
                                self.resolve_guild_invite(true);
                                self.refresh_guild_texture();
                            } else {
                                self.resolve_guild_creation_invite(true);
                            }
                            if let Some(window) = self.window.as_ref() {
                                window.request_redraw();
                            }
                            return;
                        }
                        PhysicalKey::Code(KeyCode::KeyN) => {
                            if self.pending_guild_invite.is_some() {
                                self.resolve_guild_invite(false);
                                self.refresh_guild_texture();
                            } else {
                                self.resolve_guild_creation_invite(false);
                            }
                            if let Some(window) = self.window.as_ref() {
                                window.request_redraw();
                            }
                            return;
                        }
                        _ => {}
                    }
                }
                if (self.pending_family_join.is_some() || self.pending_family_summon.is_some())
                    && event.state == ElementState::Pressed
                    && !event.repeat
                {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::KeyY) => {
                            self.answer_family_request(true);
                            if let Some(window) = self.window.as_ref() {
                                window.request_redraw();
                            }
                            return;
                        }
                        PhysicalKey::Code(KeyCode::KeyN) => {
                            self.answer_family_request(false);
                            if let Some(window) = self.window.as_ref() {
                                window.request_redraw();
                            }
                            return;
                        }
                        _ => {}
                    }
                }
                if self.pending_alliance_invite.is_some()
                    && event.state == ElementState::Pressed
                    && !event.repeat
                {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::KeyY) => {
                            self.resolve_alliance_invite(true);
                            if let Some(window) = self.window.as_ref() {
                                window.request_redraw();
                            }
                            return;
                        }
                        PhysicalKey::Code(KeyCode::KeyN) => {
                            self.resolve_alliance_invite(false);
                            if let Some(window) = self.window.as_ref() {
                                window.request_redraw();
                            }
                            return;
                        }
                        _ => {}
                    }
                }
                if self.world_map.open
                    && self.world_map.search_focused
                    && event.state == ElementState::Pressed
                {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Backspace) => {
                            self.world_map_key_text_for_ime_dedup = None;
                            self.backspace_world_map_search();
                        }
                        PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                            self.world_map.search_focused = false;
                            self.world_map_key_text_for_ime_dedup = None;
                            self.refresh_world_map_texture();
                        }
                        PhysicalKey::Code(KeyCode::Escape) => {
                            self.world_map_key_text_for_ime_dedup = None;
                            self.escape_world_map();
                        }
                        _ => {
                            if let Some(text) = event.text.as_deref().filter(|text| {
                                text.chars().any(|character| !character.is_control())
                            }) {
                                self.world_map_key_text_for_ime_dedup = Some(text.to_owned());
                                self.edit_world_map_search(text);
                            }
                        }
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.show_login_fields
                    && event.state == ElementState::Pressed
                    && event.physical_key != PhysicalKey::Code(KeyCode::Space)
                {
                    self.suppress_login_ime_space = false;
                }
                if self.show_login_fields
                    && !self.login_buttons.is_empty()
                    && event.state == ElementState::Pressed
                    && !event.repeat
                {
                    let handled = match event.physical_key {
                        PhysicalKey::Code(KeyCode::Tab) => {
                            self.advance_login_focus();
                            true
                        }
                        PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                            if self.focused_login_button.is_some() {
                                self.activate_focused_login_button(event_loop);
                            } else if self.login_action_enabled(LoginAction::Login) {
                                self.activate_login(LoginAction::Login);
                            }
                            true
                        }
                        PhysicalKey::Code(KeyCode::Space)
                            if self.focused_login_button.is_some() =>
                        {
                            self.suppress_login_ime_space = true;
                            self.activate_focused_login_button(event_loop);
                            true
                        }
                        PhysicalKey::Code(
                            KeyCode::ArrowLeft
                            | KeyCode::ArrowUp
                            | KeyCode::ArrowRight
                            | KeyCode::ArrowDown,
                        ) => {
                            let direction = match event.physical_key {
                                PhysicalKey::Code(KeyCode::ArrowLeft | KeyCode::ArrowUp) => -1,
                                _ => 1,
                            };
                            self.move_login_button_focus(direction);
                            true
                        }
                        _ => false,
                    };
                    if handled {
                        if self.refresh_login_button_visuals()
                            && let Some(window) = self.window.as_ref()
                        {
                            window.request_redraw();
                        }
                        return;
                    }
                }
                if self.show_login_fields && event.state == ElementState::Pressed {
                    if event.physical_key == PhysicalKey::Code(KeyCode::Space)
                        && self.focused_login_button.is_some()
                    {
                        // Space activates a focused login button. Ignore repeats instead
                        // of letting them turn into a field character.
                        return;
                    }
                    if let Some(text) = event
                        .text
                        .as_deref()
                        .filter(|text| text.chars().any(|character| !character.is_control()))
                    {
                        self.login_key_text_for_ime_dedup = Some(text.to_owned());
                        self.insert_text(text);
                        if let Some(window) = self.window.as_ref() {
                            window.request_redraw();
                        }
                        return;
                    }
                }
                if self.character_list.is_some()
                    && self.character_creation.is_none()
                    && !self.show_login_fields
                    && self.current_field.is_none()
                {
                    if event.state == ElementState::Pressed && !event.repeat {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::Tab) => {
                                self.advance_character_action_focus();
                            }
                            PhysicalKey::Code(KeyCode::ArrowLeft | KeyCode::ArrowUp) => {
                                self.move_character_selection(-1);
                            }
                            PhysicalKey::Code(KeyCode::ArrowRight | KeyCode::ArrowDown) => {
                                self.move_character_selection(1);
                            }
                            PhysicalKey::Code(KeyCode::PageUp) => {
                                self.change_character_page(-1);
                            }
                            PhysicalKey::Code(KeyCode::PageDown) => {
                                self.change_character_page(1);
                            }
                            PhysicalKey::Code(
                                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space,
                            ) => {
                                self.activate_focused_character_action();
                            }
                            PhysicalKey::Code(KeyCode::Escape) => {
                                self.character_list = None;
                                self.character_selection_status = None;
                                self.refresh_world_scene();
                            }
                            _ => {}
                        }
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.set_gameplay_key(event.physical_key, event.state, event.repeat) {
                    if self.quit_requested {
                        event_loop.exit();
                    }
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.npc_shop.is_some() && event.repeat {
                    return;
                }
                if event.state != ElementState::Pressed {
                    return;
                }
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyH) if self.current_field.is_some() => {
                        self.open_new_whisper_window();
                    }
                    PhysicalKey::Code(KeyCode::Escape) => {
                        if self.show_skill_book {
                            self.show_skill_book = false;
                        } else if self.show_pet_equipment {
                            self.show_pet_equipment = false;
                        } else if self.show_inventory {
                            if self.pending_scroll_slot.take().is_some() {
                                self.refresh_inventory_texture();
                            } else {
                                self.show_inventory = false;
                                self.inventory_drag_source = None;
                                self.inventory_hover_slot = None;
                            }
                        } else if self.npc_shop.is_some() {
                            if self
                                .npc_shop
                                .as_ref()
                                .is_some_and(|shop| shop.confirmation.is_some())
                            {
                                self.resolve_npc_shop_confirmation(false);
                            } else {
                                self.close_npc_shop();
                            }
                        } else {
                            self.open_quit_confirmation();
                        }
                    }
                    PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                        if self.npc_shop.is_some() {
                            if self
                                .npc_shop
                                .as_ref()
                                .is_some_and(|shop| shop.confirmation.is_some())
                            {
                                self.resolve_npc_shop_confirmation(true);
                            } else {
                                self.buy_selected_npc_shop_item();
                            }
                        } else if self.current_field.is_some() {
                            if self.active_whisper.is_some() {
                                self.whisper_input_focused = true;
                                self.gameplay_input = GameplayInput::default();
                                self.refresh_whisper_texture();
                            } else {
                                self.chat_open = true;
                                self.gameplay_input = GameplayInput::default();
                                self.refresh_chat_texture();
                            }
                        } else if self.show_login_fields {
                            self.activate_login(LoginAction::Login)
                        } else if self.character_list.is_some() {
                            self.select_character(self.character_page * CHARACTER_PAGE_SIZE);
                        } else {
                            self.request_characters();
                        }
                    }
                    PhysicalKey::Code(KeyCode::Tab) => {
                        let previous = self.focused_field;
                        self.focused_field = match previous {
                            LoginField::Account => LoginField::Password,
                            LoginField::Password => LoginField::Account,
                        };
                        self.refresh_field_texture(previous);
                        self.refresh_field_texture(self.focused_field);
                    }
                    PhysicalKey::Code(KeyCode::Backspace) => {
                        self.backspace();
                    }
                    _ => {}
                }
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                if self.family_ui.input.is_some() {
                    let duplicate = self
                        .family_ui
                        .input
                        .as_mut()
                        .and_then(|input| input.key_text_for_ime_dedup.take());
                    if duplicate.as_deref() == Some(text.as_str()) {
                        return;
                    }
                    self.edit_family_input(&text);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.skill_macro_editor.open {
                    if self
                        .skill_macro_editor
                        .key_text_for_ime_dedup
                        .take()
                        .as_deref()
                        == Some(text.as_str())
                    {
                        return;
                    }
                    self.skill_macro_editor.key_text_for_ime_dedup = None;
                    self.skill_macro_editor.insert_text(&text);
                    self.refresh_skill_macro_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.monster_book.open && self.monster_book.search_open {
                    for character in text.chars() {
                        if !character.is_control()
                            && self.monster_book.search_query.chars().count() < 48
                        {
                            self.monster_book.search_query.push(character);
                        }
                    }
                    self.monster_book.search_notice = None;
                    self.refresh_monster_book_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.world_map.open {
                    if !self.world_map.search_focused {
                        return;
                    }
                    if self.world_map_key_text_for_ime_dedup.take().as_deref()
                        == Some(text.as_str())
                    {
                        return;
                    }
                    self.world_map_key_text_for_ime_dedup = None;
                    self.edit_world_map_search(&text);
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }
                if self.show_login_fields
                    && self.login_key_text_for_ime_dedup.take().as_deref() == Some(text.as_str())
                {
                    return;
                }
                self.login_key_text_for_ime_dedup = None;
                if self.suppress_login_ime_space {
                    self.suppress_login_ime_space = false;
                    if text == " " {
                        return;
                    }
                }
                if self.handle_messenger_text(&text) {
                    return;
                }
                if self.show_guild_panel
                    && !matches!(&self.guild_panel_mode, GuildPanelMode::Overview)
                {
                    self.edit_guild_panel_text(&text);
                } else if self.hired_merchant_room.is_some() {
                    if let Some(room) = self.hired_merchant_room.as_mut()
                        && room.stock_source.is_some()
                    {
                        for character in text.chars() {
                            if character.is_ascii_digit() && room.stock_price.len() < 9 {
                                room.stock_price.push(character);
                            }
                        }
                        room.notice = None;
                    } else if let Some(room) = self.hired_merchant_room.as_mut()
                        && room.chat_focused
                    {
                        for character in text.chars() {
                            if !character.is_control()
                                && room.chat_input.encode_utf16().count() + character.len_utf16()
                                    <= 60
                            {
                                room.chat_input.push(character);
                            }
                        }
                        room.notice = None;
                    }
                    self.refresh_hired_merchant_room_texture();
                } else if self.trade_room.is_some() {
                    if let Some(room) = self.trade_room.as_mut() {
                        for character in text.chars() {
                            if character.is_ascii_digit() {
                                if room.pending_item.is_some() && room.quantity_input.len() < 5 {
                                    room.quantity_input.push(character);
                                } else if room.meso_focused && room.meso_input.len() < 10 {
                                    room.meso_input.push(character);
                                } else if room.chat_focused
                                    && !character.is_control()
                                    && room.chat_input.encode_utf16().count()
                                        + character.len_utf16()
                                        <= 60
                                {
                                    room.chat_input.push(character);
                                }
                            } else if room.chat_focused
                                && !character.is_control()
                                && room.chat_input.encode_utf16().count() + character.len_utf16()
                                    <= 60
                            {
                                room.chat_input.push(character);
                            }
                        }
                        room.notice = None;
                    }
                    self.refresh_trade_texture();
                } else if let Some(owl) = self.owl_search.as_mut() {
                    if owl.result_item_id.is_none() {
                        for character in text.chars() {
                            if !character.is_control() && owl.query.chars().count() < 40 {
                                owl.query.push(character);
                            }
                        }
                        owl.notice = None;
                    }
                    self.refresh_owl_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                } else if self.guild_bbs.is_some() {
                    if let Some(bbs) = self.guild_bbs.as_mut() {
                        if let Some(draft) = bbs.draft.as_mut() {
                            match draft.focus {
                                GuildBbsDraftFocus::Title => {
                                    for character in text.chars() {
                                        if !character.is_control()
                                            && draft.title.encode_utf16().count()
                                                + character.len_utf16()
                                                <= GUILD_BBS_TITLE_LIMIT
                                        {
                                            draft.title.push(character);
                                        }
                                    }
                                }
                                GuildBbsDraftFocus::Content => {
                                    for character in text.chars() {
                                        if !character.is_control()
                                            && draft.content.encode_utf16().count()
                                                + character.len_utf16()
                                                <= GUILD_BBS_CONTENT_LIMIT
                                        {
                                            draft.content.push(character);
                                        }
                                    }
                                }
                            }
                        } else if bbs.reply_focused {
                            for character in text.chars() {
                                if !character.is_control()
                                    && bbs.reply_input.encode_utf16().count()
                                        + character.len_utf16()
                                        <= GUILD_BBS_REPLY_LIMIT
                                {
                                    bbs.reply_input.push(character);
                                }
                            }
                        }
                    }
                    self.refresh_guild_bbs_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                } else if self.inventory_drop_prompt.is_some() {
                    if let Some(prompt) = self.inventory_drop_prompt.as_mut() {
                        if prompt.step == InventoryDropStep::Quantity {
                            let max_digits = prompt.max_quantity.to_string().len().max(1);
                            for character in text.chars() {
                                if character.is_ascii_digit() && prompt.quantity.len() < max_digits
                                {
                                    prompt.quantity.push(character);
                                }
                            }
                            prompt.notice = None;
                        }
                    }
                    self.refresh_inventory_drop_prompt_texture();
                } else if self.pic_entry.is_some() {
                    self.edit_pic_entry(&text);
                } else if self.character_creation.is_some() {
                    if self
                        .character_creation_key_text_for_ime_dedup
                        .take()
                        .as_deref()
                        == Some(text.as_str())
                    {
                        return;
                    }
                    self.character_creation_key_text_for_ime_dedup = None;
                    let mut name_changed = false;
                    if let Some(creation) = self.character_creation.as_mut() {
                        if creation.name_focused
                            && !creation.awaiting_name
                            && !creation.awaiting_create
                        {
                            for character in text.chars() {
                                if character.is_ascii_graphic()
                                    && creation.name.chars().count() < 12
                                {
                                    creation.name.push(character);
                                    name_changed = true;
                                }
                            }
                            creation.notice = None;
                        }
                    }
                    if name_changed {
                        self.reset_login_caret();
                    }
                    self.refresh_character_creation_scene();
                } else if let Some(dialogue) = self.npc_dialogue.as_mut() {
                    if matches!(dialogue.state.message_type, 2 | 3) {
                        for character in text.chars() {
                            if dialogue.state.message_type == 2
                                || character.is_ascii_digit()
                                || (character == '-' && dialogue.input.is_empty())
                            {
                                dialogue.input.push(character);
                            }
                        }
                        self.rebuild_npc_dialogue();
                    }
                } else if self.whisper_input_focused {
                    if let Some(index) = self.active_whisper
                        && let Some(conversation) = self.whisper_conversations.get_mut(index)
                    {
                        if self.whisper_target_focused {
                            for character in text.chars() {
                                if (character.is_alphanumeric() || character == '_')
                                    && conversation.target.chars().count() < 13
                                {
                                    conversation.target.push(character);
                                }
                            }
                        } else {
                            for character in text.chars() {
                                if !character.is_control()
                                    && conversation.input.chars().count() < MAX_CHAT_MESSAGE_CHARS
                                {
                                    conversation.input.push(character);
                                }
                            }
                        }
                    }
                    self.refresh_whisper_texture();
                } else if self.chat_open
                    && self.current_field.is_some()
                    && self.npc_shop.is_none()
                    && self.cash_shop.is_none()
                {
                    let max_chars = self
                        .cash_item_compose
                        .as_ref()
                        .map_or(MAX_CHAT_MESSAGE_CHARS, CashItemCompose::input_max_chars);
                    for character in text.chars() {
                        if !character.is_control() && self.chat_input.chars().count() < max_chars {
                            self.chat_input.push(character);
                        }
                    }
                    self.refresh_chat_texture();
                } else if self.cash_shop.is_some() {
                    if let Some(shop) = self.cash_shop.as_mut() {
                        if shop.coupon_focused {
                            for character in text.chars() {
                                if (character.is_ascii_alphanumeric() || character == '-')
                                    && shop.coupon_code.chars().count() < 32
                                {
                                    shop.coupon_code.push(character.to_ascii_uppercase());
                                }
                            }
                            shop.notice = None;
                        } else if shop.search_focused {
                            for character in text.chars() {
                                if !character.is_control() && shop.query.chars().count() < 40 {
                                    shop.query.push(character);
                                }
                            }
                            shop.page = 0;
                        }
                    }
                    self.refresh_cash_shop_texture();
                } else {
                    self.insert_text(&text);
                }
            }
            WindowEvent::Focused(false) => {
                self.gameplay_input = GameplayInput::default();
                if self.inventory_drag_source.take().is_some() {
                    self.refresh_inventory_texture();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size);
                }
                self.cursor = self.cursor_physical.and_then(|(x, y)| {
                    Viewport::fit(
                        LogicalSize::LEGACY_1024X768,
                        PhysicalSize {
                            width: size.width,
                            height: size.height,
                        },
                    )
                    .and_then(|viewport| {
                        viewport.to_logical(Point {
                            x: x as f32,
                            y: y as f32,
                        })
                    })
                });
                self.update_inventory_hover();
            }
            WindowEvent::RedrawRequested => {
                let mut draws = Vec::with_capacity(self.sprite_draws.len() + 128);
                let maple_tv_ad_texture = self
                    .maple_tv_ad_animations
                    .get(self.maple_tv_ad_index)
                    .and_then(MapleTvAnimation::current)
                    .map(|frame| &frame.texture);
                let maple_tv_theme_frames = [
                    self.maple_tv_theme_animations[0].current(),
                    self.maple_tv_theme_animations[1].current(),
                ];
                let mut next_map_layer = 0;
                let mut map_overlays_inserted = false;
                for (draw_index, (texture_index, rect)) in self.sprite_draws.iter().enumerate() {
                    if let Some(scene) = self.map_scene.as_ref() {
                        while next_map_layer < scene.layer_insert_indices.len()
                            && scene.layer_insert_indices[next_map_layer] == draw_index
                        {
                            append_map_entity_draws(
                                scene,
                                &self.active_buffs,
                                &self.sprite_textures,
                                &self.solid_textures,
                                self.maple_tv_idle_texture.as_ref(),
                                maple_tv_ad_texture,
                                maple_tv_theme_frames,
                                self.maple_tv_text_texture.as_ref(),
                                self.maple_tv_broadcast.as_ref(),
                                self.damage_glyph_texture.as_ref(),
                                &self.remote_speech_textures,
                                &self.pet_speech_textures,
                                self.local_speech_texture.as_ref(),
                                &self.damage_glyphs,
                                MapEntityDrawPass::Layer(next_map_layer as u8),
                                &mut draws,
                            );
                            next_map_layer += 1;
                        }
                        if !map_overlays_inserted && scene.foreground_start_index == draw_index {
                            append_map_entity_draws(
                                scene,
                                &self.active_buffs,
                                &self.sprite_textures,
                                &self.solid_textures,
                                self.maple_tv_idle_texture.as_ref(),
                                maple_tv_ad_texture,
                                maple_tv_theme_frames,
                                self.maple_tv_text_texture.as_ref(),
                                self.maple_tv_broadcast.as_ref(),
                                self.damage_glyph_texture.as_ref(),
                                &self.remote_speech_textures,
                                &self.pet_speech_textures,
                                self.local_speech_texture.as_ref(),
                                &self.damage_glyphs,
                                MapEntityDrawPass::Overlays,
                                &mut draws,
                            );
                            map_overlays_inserted = true;
                        }
                    }
                    if (self.login_animation_draw_indices.contains(&draw_index)
                        && !self
                            .active_login_animation_draw_indices
                            .contains(&draw_index))
                        || (self.login_button_draw_indices.contains(&draw_index)
                            && !self.active_login_button_draw_indices.contains(&draw_index))
                        || self.map_scene.as_ref().is_some_and(|scene| {
                            (scene.map_art_animation_draw_indices.contains(&draw_index)
                                && !scene.active_map_art_draw_indices.contains(&draw_index))
                                || scene.avatar_indices.contains(&draw_index)
                                || scene.portal_frame_indices.contains(&draw_index)
                                || scene
                                    .inactive_environment_draw_indices
                                    .contains(&draw_index)
                        })
                    {
                        continue;
                    }
                    if let Some(texture) = self.sprite_textures.get(*texture_index) {
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
                if let Some(scene) = self.map_scene.as_ref() {
                    while next_map_layer < scene.layer_insert_indices.len() {
                        append_map_entity_draws(
                            scene,
                            &self.active_buffs,
                            &self.sprite_textures,
                            &self.solid_textures,
                            self.maple_tv_idle_texture.as_ref(),
                            maple_tv_ad_texture,
                            maple_tv_theme_frames,
                            self.maple_tv_text_texture.as_ref(),
                            self.maple_tv_broadcast.as_ref(),
                            self.damage_glyph_texture.as_ref(),
                            &self.remote_speech_textures,
                            &self.pet_speech_textures,
                            self.local_speech_texture.as_ref(),
                            &self.damage_glyphs,
                            MapEntityDrawPass::Layer(next_map_layer as u8),
                            &mut draws,
                        );
                        next_map_layer += 1;
                    }
                    if !map_overlays_inserted {
                        append_map_entity_draws(
                            scene,
                            &self.active_buffs,
                            &self.sprite_textures,
                            &self.solid_textures,
                            self.maple_tv_idle_texture.as_ref(),
                            maple_tv_ad_texture,
                            maple_tv_theme_frames,
                            self.maple_tv_text_texture.as_ref(),
                            self.maple_tv_broadcast.as_ref(),
                            self.damage_glyph_texture.as_ref(),
                            &self.remote_speech_textures,
                            &self.pet_speech_textures,
                            self.local_speech_texture.as_ref(),
                            &self.damage_glyphs,
                            MapEntityDrawPass::Overlays,
                            &mut draws,
                        );
                    }
                }
                if let Some(scene) = self.map_scene.as_ref() {
                    append_map_screen_effect_draws(scene, &self.sprite_textures, &mut draws);
                }
                if self.current_field.is_some()
                    && let Some(texture) = self.weather_texture.as_ref()
                {
                    self.weather_overlay.append_draws(texture, &mut draws);
                }
                if self.current_field.is_some()
                    && let Some(character) = self.active_character.as_ref()
                    && let Some(white) = self.solid_textures.get(1)
                {
                    let mut warning_stats = character.stats.clone();
                    let hyperbody_hp = self.active_buff_value(
                        openstory_protocol::BuffMaskGroup::Second,
                        0x0000_2000_0000_0000,
                    );
                    if hyperbody_hp > 0 {
                        warning_stats.max_hp =
                            ((i32::from(warning_stats.max_hp) * (100 + i32::from(hyperbody_hp)))
                                / 100)
                                .clamp(0, i32::from(u16::MAX)) as u16;
                    }
                    let hyperbody_mp = self.active_buff_value(
                        openstory_protocol::BuffMaskGroup::Second,
                        0x0000_4000_0000_0000,
                    );
                    if hyperbody_mp > 0 {
                        warning_stats.max_mp =
                            ((i32::from(warning_stats.max_mp) * (100 + i32::from(hyperbody_mp)))
                                / 100)
                                .clamp(0, i32::from(u16::MAX)) as u16;
                    }
                    for (tint, opacity) in [
                        (
                            [1.0, 0.0, 0.0],
                            self.gameplay_settings
                                .hp_warning_opacity(warning_stats.hp, warning_stats.max_hp),
                        ),
                        (
                            [0.0, 0.0, 1.0],
                            self.gameplay_settings
                                .mp_warning_opacity(warning_stats.mp, warning_stats.max_mp),
                        ),
                    ] {
                        if let Some(opacity) = opacity {
                            draws.push(SpriteDraw {
                                texture: white,
                                x: 0.0,
                                y: 0.0,
                                width: 1024.0,
                                height: 768.0,
                                uv: [0.0, 0.0, 1.0, 1.0],
                                tint,
                                blend_mode: BlendMode::Alpha,
                                flip_x: false,
                                opacity,
                            });
                        }
                    }
                }
                if self.current_field.is_some()
                    && let Some(notice) = self.scrolling_notice.as_ref()
                {
                    if let Some(background) = self.solid_textures.first() {
                        draws.push(SpriteDraw {
                            texture: background,
                            x: 0.0,
                            y: 0.0,
                            width: 1024.0,
                            height: 23.0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [0.0, 0.0, 0.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 0.535,
                        });
                    }
                    draws.push(SpriteDraw {
                        texture: &notice.texture,
                        x: notice.x,
                        y: -1.0,
                        width: notice.width,
                        height: 24.0,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 0.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.current_field.is_some() && !self.status_bar_draw_indices.is_empty() {
                    for (texture_index, rect) in
                        self.status_bar_draw_indices.iter().take(3).copied()
                    {
                        if let Some(texture) = self.sprite_textures.get(texture_index) {
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
                    if let Some(stats) = self.active_character.as_ref().map(|c| &c.stats) {
                        for (index, gauge) in self.status_bar_gauges.iter().enumerate() {
                            let Some(gauge) = gauge else {
                                continue;
                            };
                            let fraction = match index {
                                0 if stats.max_hp > 0 => {
                                    f32::from(stats.hp.min(stats.max_hp)) / f32::from(stats.max_hp)
                                }
                                1 if stats.max_mp > 0 => {
                                    f32::from(stats.mp.min(stats.max_mp)) / f32::from(stats.max_mp)
                                }
                                2 => v83_exp_fraction(stats.level, stats.experience),
                                _ => 0.0,
                            }
                            .clamp(0.0, 1.0);
                            if fraction <= 0.0 {
                                continue;
                            }
                            if let Some(texture) = self.sprite_textures.get(gauge.texture_index) {
                                draws.push(SpriteDraw {
                                    texture,
                                    x: gauge.x,
                                    y: gauge.y,
                                    width: gauge.width * fraction,
                                    height: gauge.height,
                                    uv: [0.0, 0.0, fraction, 1.0],
                                    tint: [1.0, 1.0, 1.0],
                                    blend_mode: BlendMode::Alpha,
                                    flip_x: false,
                                    opacity: 1.0,
                                });
                            }
                            if let Some(texture_index) = gauge.start_texture_index
                                && let Some(texture) = self.sprite_textures.get(texture_index)
                            {
                                draws.push(SpriteDraw {
                                    texture,
                                    x: gauge.x,
                                    y: gauge.y,
                                    width: 1.0,
                                    height: 13.0,
                                    uv: [0.0, 0.0, 1.0, 1.0],
                                    tint: [1.0, 1.0, 1.0],
                                    blend_mode: BlendMode::Alpha,
                                    flip_x: false,
                                    opacity: 1.0,
                                });
                            }
                            if let Some(texture_index) = gauge.end_texture_index
                                && let Some(texture) = self.sprite_textures.get(texture_index)
                            {
                                draws.push(SpriteDraw {
                                    texture,
                                    x: gauge.x + gauge.width * fraction,
                                    y: gauge.y,
                                    width: 2.0,
                                    height: 13.0,
                                    uv: [0.0, 0.0, 1.0, 1.0],
                                    tint: [1.0, 1.0, 1.0],
                                    blend_mode: BlendMode::Alpha,
                                    flip_x: false,
                                    opacity: 1.0,
                                });
                            }
                        }
                    }
                    for (texture_index, rect) in
                        self.status_bar_draw_indices.iter().skip(3).copied()
                    {
                        if let Some(texture) = self.sprite_textures.get(texture_index) {
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
                    if let Some(texture) = self.map_hud_texture.as_ref() {
                        draws.push(SpriteDraw {
                            texture,
                            x: 0.0,
                            y: self.status_bar_layout.top(),
                            width: 1024.0,
                            height: self.status_bar_layout.height() as f32,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                }
                if self.current_field.is_some()
                    && self.show_quickslots
                    && let Some(texture) = self.quickslot_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: 865.0,
                        y: 615.0,
                        width: 151.0,
                        height: 80.0,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.current_field.is_some()
                    && self.show_status_menu
                    && let Some(texture) = self.status_menu_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: STATUS_MENU_X,
                        y: STATUS_MENU_Y,
                        width: STATUS_MENU_WIDTH as f32,
                        height: STATUS_MENU_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.current_field.is_some()
                    && self.chat_open
                    && let Some(texture) = self.chat_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: 12.0,
                        y: 506.0,
                        width: CHAT_WIDTH as f32,
                        height: CHAT_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.current_field.is_some()
                    && self
                        .cash_item_compose
                        .as_ref()
                        .is_some_and(|compose| compose.kind.is_item_megaphone(compose.item_id))
                    && let Some(texture) = self.item_megaphone_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: ITEM_MEGAPHONE_X,
                        y: ITEM_MEGAPHONE_Y,
                        width: texture.width() as f32,
                        height: texture.height() as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.current_field.is_some()
                    && self.show_party_panel
                    && let Some(texture) = self.party_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: 728.0,
                        y: 238.0,
                        width: PARTY_PANEL_WIDTH as f32,
                        height: PARTY_PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.current_field.is_some()
                    && self.show_buddy_panel
                    && let Some(texture) = self.buddy_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: 728.0,
                        y: 190.0,
                        width: BUDDY_PANEL_WIDTH as f32,
                        height: BUDDY_PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.current_field.is_some()
                    && self.show_guild_panel
                    && let Some(texture) = self.guild_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: 680.0,
                        y: 174.0,
                        width: GUILD_PANEL_WIDTH as f32,
                        height: GUILD_PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.current_field.is_some()
                    && self.show_alliance_panel
                    && let Some(texture) = self.alliance_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: 300.0,
                        y: 190.0,
                        width: ALLIANCE_PANEL_WIDTH as f32,
                        height: ALLIANCE_PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.current_field.is_some()
                    && self.show_family_panel
                    && let Some(texture) = self.family_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: 375.0,
                        y: 165.0,
                        width: FAMILY_PANEL_WIDTH as f32,
                        height: FAMILY_PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.current_field.is_some()
                    && self.show_family_tree
                    && let Some(texture) = self.family_tree_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: 223.0,
                        y: 191.0,
                        width: FAMILY_TREE_PANEL_WIDTH as f32,
                        height: FAMILY_TREE_PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.current_field.is_some()
                    && self.guild_bbs.is_some()
                    && let Some(texture) = self.guild_bbs_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: 100.0,
                        y: 100.0,
                        width: GUILD_BBS_PANEL_WIDTH as f32,
                        height: GUILD_BBS_PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.show_inventory {
                    if let Some(texture) = self.inventory_texture.as_ref() {
                        draws.push(SpriteDraw {
                            texture,
                            x: INVENTORY_PANEL_X,
                            y: INVENTORY_PANEL_Y,
                            width: INVENTORY_PANEL_WIDTH as f32,
                            height: INVENTORY_PANEL_HEIGHT as f32,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                }
                if self.show_pet_equipment {
                    if let Some(texture) = self.pet_equipment_texture.as_ref() {
                        draws.push(SpriteDraw {
                            texture,
                            x: 618.0,
                            y: 128.0,
                            width: 390.0,
                            height: 480.0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                }
                if self.show_skill_book {
                    if let Some(texture) = self.skill_book_texture.as_ref() {
                        draws.push(SpriteDraw {
                            texture,
                            x: 586.0,
                            y: 128.0,
                            width: 430.0,
                            height: 500.0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                }
                if self.show_quest_log {
                    if let Some(texture) = self.quest_log_texture.as_ref() {
                        draws.push(SpriteDraw {
                            texture,
                            x: QUEST_LOG_PANEL_X,
                            y: QUEST_LOG_PANEL_Y,
                            width: QUEST_LOG_PANEL_WIDTH as f32,
                            height: QUEST_LOG_PANEL_HEIGHT as f32,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                }
                if self.show_status {
                    if let Some(texture) = self.status_texture.as_ref() {
                        draws.push(SpriteDraw {
                            texture,
                            x: STATUS_PANEL_X,
                            y: STATUS_PANEL_Y,
                            width: STATUS_PANEL_WIDTH as f32,
                            height: STATUS_PANEL_HEIGHT as f32,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                }
                if self.show_key_config {
                    if let Some(texture) = self.key_config_texture.as_ref() {
                        draws.push(SpriteDraw {
                            texture,
                            x: KEY_CONFIG_PANEL_X,
                            y: KEY_CONFIG_PANEL_Y,
                            width: KEY_CONFIG_PANEL_WIDTH as f32,
                            height: KEY_CONFIG_PANEL_HEIGHT as f32,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                }
                if self.owl_search.is_some()
                    && let Some(texture) = self.owl_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: OWL_PANEL_X,
                        y: OWL_PANEL_Y,
                        width: OWL_PANEL_WIDTH as f32,
                        height: OWL_PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.cash_shop.is_some() {
                    if let Some(texture) = self.cash_shop_texture.as_ref() {
                        draws.push(SpriteDraw {
                            texture,
                            x: 0.0,
                            y: 0.0,
                            width: 1024.0,
                            height: 768.0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                }
                if self.current_field.is_some()
                    && self.active_whisper.is_some()
                    && self.npc_dialogue.is_none()
                    && self.npc_shop.is_none()
                    && self.storage.is_none()
                    && self.cash_shop.is_none()
                    && let Some(texture) = self.whisper_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: WHISPER_PANEL_X,
                        y: WHISPER_PANEL_Y,
                        width: WHISPER_PANEL_WIDTH as f32,
                        height: WHISPER_PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.show_login_fields {
                    for (texture, y) in self.field_textures.iter().zip([ACCOUNT_Y, PASSWORD_Y]) {
                        draws.push(SpriteDraw {
                            texture,
                            x: FIELD_X,
                            y,
                            width: FIELD_WIDTH as f32,
                            height: FIELD_HEIGHT as f32,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                }
                if let Some(dialogue) = self.npc_dialogue.as_ref() {
                    draws.push(SpriteDraw {
                        texture: &dialogue.texture,
                        x: dialogue.panel.x,
                        y: dialogue.panel.y,
                        width: dialogue.panel.width,
                        height: dialogue.panel.height,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if let Some(shop) = self.npc_shop.as_ref() {
                    draws.push(SpriteDraw {
                        texture: &shop.texture,
                        x: 202.0,
                        y: 124.0,
                        width: 620.0,
                        height: 520.0,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.storage.is_some()
                    && let Some(texture) = self.storage_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: 182.0,
                        y: 105.0,
                        width: 660.0,
                        height: 560.0,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.hired_merchant_room.is_some()
                    && let Some(texture) = self.hired_merchant_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: HIRED_MERCHANT_PANEL_X,
                        y: HIRED_MERCHANT_PANEL_Y,
                        width: HIRED_MERCHANT_PANEL_WIDTH as f32,
                        height: HIRED_MERCHANT_PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.hired_merchant_room.is_some()
                    && self.show_inventory
                    && let Some(texture) = self.inventory_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: INVENTORY_PANEL_X,
                        y: INVENTORY_PANEL_Y,
                        width: INVENTORY_PANEL_WIDTH as f32,
                        height: INVENTORY_PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.trade_room.is_some()
                    && let Some(texture) = self.trade_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: TRADE_PANEL_X,
                        y: TRADE_PANEL_Y,
                        width: TRADE_PANEL_WIDTH as f32,
                        height: TRADE_PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.trade_room.is_some()
                    && self.show_inventory
                    && let Some(texture) = self.inventory_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: INVENTORY_PANEL_X,
                        y: INVENTORY_PANEL_Y,
                        width: INVENTORY_PANEL_WIDTH as f32,
                        height: INVENTORY_PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.character_info.is_some()
                    && let Some(texture) = self.character_info_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: CHARACTER_INFO_PANEL_X,
                        y: CHARACTER_INFO_PANEL_Y,
                        width: CHARACTER_INFO_PANEL_WIDTH as f32,
                        height: CHARACTER_INFO_PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.pic_entry.is_some() {
                    if let Some(black) = self.solid_textures.first() {
                        draws.push(SpriteDraw {
                            texture: black,
                            x: 0.0,
                            y: 0.0,
                            width: 1024.0,
                            height: 768.0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [0.0, 0.0, 0.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 0.58,
                        });
                    }
                    if let Some(white) = self.solid_textures.get(1) {
                        draws.push(SpriteDraw {
                            texture: white,
                            x: 248.0,
                            y: 289.0,
                            width: PIC_DIALOG_WIDTH as f32 + 8.0,
                            height: PIC_DIALOG_HEIGHT as f32 + 8.0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [0.68, 0.58, 0.38],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                    if let Some(black) = self.solid_textures.first() {
                        draws.push(SpriteDraw {
                            texture: black,
                            x: 252.0,
                            y: 293.0,
                            width: PIC_DIALOG_WIDTH as f32,
                            height: PIC_DIALOG_HEIGHT as f32,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [0.08, 0.07, 0.05],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 0.98,
                        });
                    }
                    if let Some(texture) = self.pic_dialog_texture.as_ref() {
                        draws.push(SpriteDraw {
                            texture,
                            x: 252.0,
                            y: 293.0,
                            width: PIC_DIALOG_WIDTH as f32,
                            height: PIC_DIALOG_HEIGHT as f32,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                }
                if let Some(texture) = self.inventory_drop_prompt_texture.as_ref() {
                    draws.push(SpriteDraw {
                        texture,
                        x: 0.0,
                        y: 0.0,
                        width: 1024.0,
                        height: 768.0,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.show_settings {
                    if let Some(black) = self.solid_textures.first() {
                        draws.push(SpriteDraw {
                            texture: black,
                            x: 0.0,
                            y: 0.0,
                            width: 1024.0,
                            height: 768.0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [0.0, 0.0, 0.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 0.58,
                        });
                    }
                    if let Some(texture) = self.settings_texture.as_ref() {
                        draws.push(SpriteDraw {
                            texture,
                            x: SETTINGS_PANEL_X,
                            y: SETTINGS_PANEL_Y,
                            width: SETTINGS_PANEL_WIDTH as f32,
                            height: SETTINGS_PANEL_HEIGHT as f32,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                }
                if self.death_notice
                    && let Some(texture) = self.death_notice_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: 0.0,
                        y: 0.0,
                        width: 1024.0,
                        height: 768.0,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.show_login_fields
                    && let Some(index) = self.focused_login_button
                    && let Some(button) = self.login_buttons.get(index)
                    && self.login_button_enabled(index)
                    && button.key_focused_draw.is_none()
                    && let Some(texture) = self.solid_textures.get(1)
                {
                    let rect = button.hit_rect;
                    let border = 2.0;
                    for (x, y, width, height) in [
                        (rect.x, rect.y, rect.width, border),
                        (rect.x, rect.y + rect.height - border, rect.width, border),
                        (rect.x, rect.y, border, rect.height),
                        (rect.x + rect.width - border, rect.y, border, rect.height),
                    ] {
                        draws.push(SpriteDraw {
                            texture,
                            x,
                            y,
                            width,
                            height,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 0.78, 0.32],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                }
                if self.world_map.open
                    && let Some(texture) = self.world_map.texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: self.world_map.x,
                        y: self.world_map.y,
                        width: self.world_map.width as f32,
                        height: self.world_map.height as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if let Some(state) = self.party_search.as_ref()
                    && let Some(texture) = self.party_search_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: state.x,
                        y: state.y,
                        width: party_search::PANEL_WIDTH as f32,
                        height: party_search::PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.monster_book.open
                    && let Some(texture) = self.monster_book.texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: self.monster_book.x,
                        y: self.monster_book.y,
                        width: self.monster_book.width as f32,
                        height: self.monster_book.height as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.monster_book.open
                    && let Some(sprite) = self.monster_book.current_detail_animation_draw()
                    && let Some(texture) = self.monster_book_animation_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: sprite.x,
                        y: sprite.y,
                        width: sprite.width,
                        height: sprite.height,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: sprite.opacity,
                    });
                }
                if let (Some(texture), Some(bounds)) =
                    (self.minimap_texture.as_ref(), self.minimap_bounds)
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: bounds.x,
                        y: bounds.y,
                        width: bounds.width,
                        height: bounds.height,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.channel_switch_open
                    && let Some(texture) = self.channel_switch_texture.as_ref()
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: channel_switch::DEFAULT_X,
                        y: channel_switch::DEFAULT_Y,
                        width: channel_switch::PANEL_WIDTH as f32,
                        height: channel_switch::PANEL_HEIGHT as f32,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if let Some(state) = self.messenger.as_ref()
                    && let Some(texture) = self.messenger_texture.as_ref()
                    && let Some(bounds) = self.messenger_bounds
                {
                    draws.push(SpriteDraw {
                        texture,
                        x: state.x,
                        y: state.y,
                        width: bounds.width,
                        height: bounds.height,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        tint: [1.0, 1.0, 1.0],
                        blend_mode: BlendMode::Alpha,
                        flip_x: false,
                        opacity: 1.0,
                    });
                }
                if self.skill_macro_editor.open {
                    if let Some(black) = self.solid_textures.first() {
                        draws.push(SpriteDraw {
                            texture: black,
                            x: 0.0,
                            y: 0.0,
                            width: 1024.0,
                            height: 768.0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [0.0, 0.0, 0.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 0.48,
                        });
                    }
                    if let Some(texture) = self.skill_macro_texture.as_ref() {
                        draws.push(SpriteDraw {
                            texture,
                            x: skill_macros::PANEL_X,
                            y: skill_macros::PANEL_Y,
                            width: skill_macros::PANEL_WIDTH as f32,
                            height: skill_macros::PANEL_HEIGHT as f32,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [1.0, 1.0, 1.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 1.0,
                        });
                    }
                }
                if self.quit_confirmation {
                    if let Some(black) = self.solid_textures.first() {
                        draws.push(SpriteDraw {
                            texture: black,
                            x: 0.0,
                            y: 0.0,
                            width: 1024.0,
                            height: 768.0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            tint: [0.0, 0.0, 0.0],
                            blend_mode: BlendMode::Alpha,
                            flip_x: false,
                            opacity: 0.52,
                        });
                    }
                    for (texture_index, rect) in self.quit_confirmation_draws.iter().copied() {
                        if let Some(texture) = self.sprite_textures.get(texture_index) {
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
                }
                if let Some(renderer) = self.renderer.as_mut() {
                    let result = renderer.render_sprites(&draws);
                    if let Err(error) = result {
                        eprintln!("OpenStory: rendering stopped: {error}");
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if !self.debug_event_loop_logged
            && std::env::var("OPENSTORY_DEBUG_NETWORK").as_deref() == Ok("1")
        {
            debug_ui_stage("UI event loop is polling session events");
            self.debug_event_loop_logged = true;
        }
        self.process_network_events();
        if self.world_map.open
            && self.current_field.map(|(map_id, _)| map_id) != self.world_map.map_id
        {
            self.refresh_world_map_texture();
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }
        let now = Instant::now();
        if now >= self.next_login_caret_toggle {
            self.next_login_caret_toggle = now + LOGIN_CARET_BLINK_INTERVAL;
            let login_caret_active = self.show_login_fields && self.focused_login_button.is_none();
            let creation_caret_active = self.character_creation.as_ref().is_some_and(|creation| {
                creation.name_focused && !creation.awaiting_name && !creation.awaiting_create
            });
            let item_megaphone_caret_active = self
                .cash_item_compose
                .as_ref()
                .is_some_and(|compose| compose.kind.is_item_megaphone(compose.item_id));
            if login_caret_active || creation_caret_active || item_megaphone_caret_active {
                self.login_caret_visible = !self.login_caret_visible;
                if login_caret_active {
                    self.refresh_field_texture(self.focused_field);
                }
                if creation_caret_active {
                    self.refresh_character_creation_scene();
                }
                if item_megaphone_caret_active {
                    self.refresh_chat_texture();
                }
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
        }
        self.expire_maple_tv_broadcast(now);
        self.expire_character_buffs(now);
        let elapsed = now.saturating_duration_since(self.last_simulation_update);
        self.last_simulation_update = now;
        self.advance_maple_tv_animations(elapsed);
        self.advance_npc_dialogue_text(elapsed);
        self.advance_login_animations(elapsed);
        if self.monster_book.open
            && !self.monster_book.search_open
            && self
                .monster_book
                .advance_detail_animation(elapsed.as_millis().min(1_000) as u64)
        {
            self.refresh_monster_book_animation_texture();
        }
        self.update_map_simulation(elapsed);
        self.advance_skill_macro();
        if now >= self.next_minimap_refresh {
            self.refresh_minimap_texture();
            self.next_minimap_refresh = now + Duration::from_millis(100);
        }
        if now >= self.next_frame {
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
            self.next_frame = now + Duration::from_millis(16);
        }
        _event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }
}
