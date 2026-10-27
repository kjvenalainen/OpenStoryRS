use super::*;

impl ClientApp {
    pub(super) fn show_map_scene(&mut self, map_id: i32, portal: u8) {
        self.weather_overlay.clear();
        self.weather_texture = None;
        let music_path = map_music_path(&self.map_archive, map_id);
        let scene = load_map_scene(
            &self.map_archive,
            Some(&self.character_archive),
            map_id,
            portal,
            self.active_character.as_ref(),
            &self.font,
        );
        match scene.and_then(|scene| self.replace_scene(scene)) {
            Ok(()) => {
                if let Some(path) = music_path {
                    self.audio.play_music(&self.sound_archive, &path);
                }
                self.current_field = Some((map_id, portal));
                // Replacing the map scene resets sprite_textures. Reload the
                // native HUD afterward so its draw indices refer to this scene.
                self.load_native_status_bar();
                self.refresh_map_hud();
                self.show_status_menu = false;
                self.chat_open = false;
                self.chat_input.clear();
                self.refresh_chat_texture();
                self.npc_dialogue = None;
                self.npc_shop = None;
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
                eprintln!("OpenStory: loaded map {map_id} at portal {portal}");
            }
            Err(error) => eprintln!("OpenStory: could not load map {map_id}: {error}"),
        }
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
        match pic_state {
            0 => self.begin_pic_entry(character.id, PicEntryStep::Register),
            1 => self.begin_pic_entry(character.id, PicEntryStep::Verify),
            2 => self.send_character_selection(character),
            state => eprintln!("OpenStory: unsupported PIC state {state}"),
        }
    }

    pub(super) fn delete_character(&mut self, index: usize) {
        let Some((character_id, pic_state)) = self.character_list.as_ref().and_then(|list| {
            list.characters
                .get(index)
                .map(|character| (character.id, list.pic_state))
        }) else {
            return;
        };
        self.begin_pic_entry_for(
            character_id,
            PicEntryStep::DeleteConfirm,
            PicPurpose::DeleteCharacter,
        );
        if pic_state == 0 {
            if let Some(entry) = self.pic_entry.as_mut() {
                entry.notice = Some("A PIC must be set before deleting characters".to_owned());
            }
            self.refresh_pic_dialog_texture();
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
        let pic_state = self
            .character_list
            .as_ref()
            .map_or(2, |list| list.pic_state);
        self.pic_entry = Some(PicEntry {
            character_id,
            step,
            purpose,
            pic_state,
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
            PicEntryStep::DeleteConfirm => (
                "Delete this character?",
                "Press Enter to confirm or Escape to cancel",
            ),
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
        if matches!(
            entry.step,
            PicEntryStep::DeleteConfirm | PicEntryStep::Notice
        ) {
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
        if entry.step == PicEntryStep::DeleteConfirm {
            match entry.pic_state {
                0 => {
                    self.pic_entry = None;
                    self.pic_dialog_texture = None;
                }
                1 => {
                    entry.step = PicEntryStep::Verify;
                    entry.notice = Some("Enter the PIC to confirm deletion".to_owned());
                    self.refresh_pic_dialog_texture();
                }
                2 => {
                    let character_id = entry.character_id;
                    self.send_delete_character(character_id, "");
                }
                state => {
                    eprintln!("OpenStory: unsupported PIC state {state}");
                    self.pic_entry = None;
                    self.pic_dialog_texture = None;
                }
            }
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
            PicEntryStep::DeleteConfirm | PicEntryStep::Notice => {
                unreachable!("handled before PIC validation")
            }
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
            | LoginAction::HighlightCharacter(_)
            | LoginAction::SelectCharacter(_)
            | LoginAction::DeleteCharacter(_)
            | LoginAction::ReturnToWorld
            | LoginAction::PreviousCharacterPage
            | LoginAction::NextCharacterPage
            | LoginAction::BeginCharacterCreation
            | LoginAction::ChooseCreationClass(_)
            | LoginAction::CycleCreationOption(_, _)
            | LoginAction::ToggleCreationGender(_)
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
        }
    }
}
