use super::*;

impl ClientApp {
    pub(super) fn refresh_settings_texture(&mut self) {
        if !self.show_settings {
            return;
        }
        let pixels = render_options_panel(
            &self.font,
            &self.ui_archive,
            self.settings,
            self.input_settings,
            self.gameplay_settings,
            self.social_settings,
            self.settings_selected_slider,
            self.settings_notice.as_deref(),
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.settings_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(SETTINGS_PANEL_WIDTH, SETTINGS_PANEL_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.settings_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update settings panel: {error}"),
        }
    }

    pub(super) fn open_settings(&mut self) {
        self.settings_original = self.settings;
        self.input_settings_original = self.input_settings;
        self.gameplay_settings_original = self.gameplay_settings;
        self.social_settings_original = self.social_settings;
        self.settings_selected_slider = OptionControl::Music;
        self.settings_dragging = None;
        self.settings_notice = None;
        self.show_settings = true;
        self.refresh_settings_texture();
    }

    pub(super) fn open_quit_confirmation(&mut self) {
        if self.quit_confirmation {
            return;
        }
        if self.quit_confirmation_draws.is_empty() {
            let assets = build_quit_confirmation_sprites(&self.legacy_ui_archive, &self.font)
                .or_else(|_| build_quit_confirmation_sprites(&self.ui_archive, &self.font));
            match assets {
                Ok((sprites, buttons)) => match self.upload_dynamic_sprites(sprites) {
                    Ok(draws) => {
                        self.quit_confirmation_draws = draws;
                        self.quit_confirmation_buttons = buttons.map(Some);
                    }
                    Err(error) => {
                        eprintln!("OpenStoryRS: could not upload the native quit dialog: {error}")
                    }
                },
                Err(error) => {
                    eprintln!("OpenStoryRS: native quit dialog is unavailable: {error}")
                }
            }
        }
        self.quit_requested = false;
        self.quit_confirmation = true;
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn cancel_settings(&mut self) {
        self.settings = self.settings_original;
        self.input_settings = self.input_settings_original;
        self.gameplay_settings = self.gameplay_settings_original;
        self.social_settings = self.social_settings_original;
        self.audio
            .set_music_volume(self.settings.effective_music_volume());
        self.audio
            .set_sound_volume(self.settings.effective_effects_volume());
        self.show_settings = false;
        self.settings_dragging = None;
        self.settings_notice = None;
        self.settings_texture = None;
    }

    pub(super) fn save_settings(&mut self) {
        match save_settings(
            self.settings,
            self.display_settings,
            self.input_settings,
            self.gameplay_settings,
            self.social_settings,
        ) {
            Ok(path) => {
                self.settings_original = self.settings;
                self.input_settings_original = self.input_settings;
                self.gameplay_settings_original = self.gameplay_settings;
                self.social_settings_original = self.social_settings;
                apply_mouse_speed(self.input_settings.mouse_speed);
                self.show_settings = false;
                self.settings_dragging = None;
                self.settings_notice = None;
                self.settings_texture = None;
                eprintln!("OpenStoryRS: saved settings to {}", path.display());
            }
            Err(error) => {
                self.settings_notice = Some(format!("Could not save: {error}"));
                self.refresh_settings_texture();
            }
        }
    }

    pub(super) fn set_option_setting(&mut self, control: OptionControl, value: u8) {
        match control {
            OptionControl::GraphicsQuality => {
                self.gameplay_settings.graphics_quality = value.min(100);
            }
            OptionControl::EffectsQuality => {
                self.gameplay_settings.effects_quality = value.min(100);
            }
            OptionControl::Music => {
                self.settings.music_volume = value.min(100);
                self.audio
                    .set_music_volume(self.settings.effective_music_volume());
            }
            OptionControl::MuteMusic => {
                self.settings.mute_music = value >= 50;
                self.audio
                    .set_music_volume(self.settings.effective_music_volume());
            }
            OptionControl::SoundEffects => {
                self.settings.effects_volume = value.min(100);
                self.audio
                    .set_sound_volume(self.settings.effective_effects_volume());
            }
            OptionControl::MuteEffects => {
                self.settings.mute_effects = value >= 50;
                self.audio
                    .set_sound_volume(self.settings.effective_effects_volume());
            }
            OptionControl::HpWarning => {
                self.gameplay_settings.hp_warning = value.min(100);
            }
            OptionControl::MpWarning => {
                self.gameplay_settings.mp_warning = value.min(100);
            }
            OptionControl::MouseSpeed => {
                self.input_settings.mouse_speed = value.min(100);
            }
            OptionControl::ScreenShake => {
                self.gameplay_settings.screen_shake_enabled = value >= 50;
            }
            OptionControl::AllowWhisper => self.social_settings.allow_whisper = value >= 50,
            OptionControl::AllowChatInvite => self.social_settings.allow_chat_invite = value >= 50,
            OptionControl::AllowPartyInvite => {
                self.social_settings.allow_party_invite = value >= 50
            }
            OptionControl::AllowGuildChat => self.social_settings.allow_guild_chat = value >= 50,
            OptionControl::AllowAllianceChat => {
                self.social_settings.allow_alliance_chat = value >= 50
            }
            OptionControl::AllowFamilyInvite => {
                self.social_settings.allow_family_invite = value >= 50
            }
            OptionControl::AllowFriendChat => self.social_settings.allow_friend_chat = value >= 50,
            OptionControl::AllowTradeRequest => {
                self.social_settings.allow_trade_request = value >= 50
            }
            OptionControl::AllowExpeditionInvite => {
                self.social_settings.allow_expedition_invite = value >= 50
            }
            OptionControl::AllowGuildInvite => {
                self.social_settings.allow_guild_invite = value >= 50
            }
            OptionControl::AllowAllianceInvite => {
                self.social_settings.allow_alliance_invite = value >= 50
            }
            OptionControl::AllowFollow => self.social_settings.allow_follow = value >= 50,
        }
        self.settings_notice = None;
        self.refresh_settings_texture();
    }

    pub(super) fn adjust_selected_option_setting(&mut self, amount: i16) {
        let value = if self.settings_selected_slider.is_toggle() {
            if amount < 0 { 0 } else { 100 }
        } else {
            let current = self.option_setting_value(self.settings_selected_slider);
            (i16::from(current) + amount).clamp(0, 100) as u8
        };
        self.set_option_setting(self.settings_selected_slider, value);
    }

    pub(super) fn toggle_selected_option_setting(&mut self) {
        if self.settings_selected_slider.is_toggle() {
            let current = self.option_setting_value(self.settings_selected_slider);
            self.set_option_setting(
                self.settings_selected_slider,
                if current >= 50 { 0 } else { 100 },
            );
        }
    }

    fn option_setting_value(&self, control: OptionControl) -> u8 {
        match control {
            OptionControl::GraphicsQuality => self.gameplay_settings.graphics_quality,
            OptionControl::EffectsQuality => self.gameplay_settings.effects_quality,
            OptionControl::Music => self.settings.music_volume,
            OptionControl::MuteMusic => bool_to_percent(self.settings.mute_music),
            OptionControl::SoundEffects => self.settings.effects_volume,
            OptionControl::MuteEffects => bool_to_percent(self.settings.mute_effects),
            OptionControl::HpWarning => self.gameplay_settings.hp_warning,
            OptionControl::MpWarning => self.gameplay_settings.mp_warning,
            OptionControl::MouseSpeed => self.input_settings.mouse_speed,
            OptionControl::ScreenShake => {
                bool_to_percent(self.gameplay_settings.screen_shake_enabled)
            }
            OptionControl::AllowWhisper => bool_to_percent(self.social_settings.allow_whisper),
            OptionControl::AllowChatInvite => {
                bool_to_percent(self.social_settings.allow_chat_invite)
            }
            OptionControl::AllowPartyInvite => {
                bool_to_percent(self.social_settings.allow_party_invite)
            }
            OptionControl::AllowGuildChat => bool_to_percent(self.social_settings.allow_guild_chat),
            OptionControl::AllowAllianceChat => {
                bool_to_percent(self.social_settings.allow_alliance_chat)
            }
            OptionControl::AllowFamilyInvite => {
                bool_to_percent(self.social_settings.allow_family_invite)
            }
            OptionControl::AllowFriendChat => {
                bool_to_percent(self.social_settings.allow_friend_chat)
            }
            OptionControl::AllowTradeRequest => {
                bool_to_percent(self.social_settings.allow_trade_request)
            }
            OptionControl::AllowExpeditionInvite => {
                bool_to_percent(self.social_settings.allow_expedition_invite)
            }
            OptionControl::AllowGuildInvite => {
                bool_to_percent(self.social_settings.allow_guild_invite)
            }
            OptionControl::AllowAllianceInvite => {
                bool_to_percent(self.social_settings.allow_alliance_invite)
            }
            OptionControl::AllowFollow => bool_to_percent(self.social_settings.allow_follow),
        }
    }

    pub(super) fn set_option_setting_from_cursor(&mut self, slider: OptionControl, x: f32) {
        let track_left = SETTINGS_PANEL_X + slider.track_left() as f32;
        if slider.track_y().is_none() {
            return;
        }
        let percent = ((x - track_left) / slider.track_width() as f32 * 100.0)
            .round()
            .clamp(0.0, 100.0);
        self.settings_selected_slider = slider;
        self.set_option_setting(slider, percent as u8);
    }

    pub(super) fn click_settings(&mut self, cursor: Point) {
        if !(SETTINGS_PANEL_X..SETTINGS_PANEL_X + SETTINGS_PANEL_WIDTH as f32).contains(&cursor.x)
            || !(SETTINGS_PANEL_Y..SETTINGS_PANEL_Y + SETTINGS_PANEL_HEIGHT as f32)
                .contains(&cursor.y)
        {
            return;
        }
        let local_x = cursor.x - SETTINGS_PANEL_X;
        let local_y = cursor.y - SETTINGS_PANEL_Y;
        if SETTINGS_CLOSE_BUTTON.contains(local_x, local_y) {
            self.cancel_settings();
            return;
        }
        if let Some(slider) = [
            OptionControl::GraphicsQuality,
            OptionControl::Music,
            OptionControl::SoundEffects,
            OptionControl::MouseSpeed,
            OptionControl::HpWarning,
            OptionControl::MpWarning,
            OptionControl::EffectsQuality,
        ]
        .into_iter()
        .find(|slider| {
            let left = slider.track_left() as f32;
            let right = left + slider.track_width() as f32;
            (left - 11.0..right + 11.0).contains(&local_x)
                && slider
                    .track_y()
                    .is_some_and(|y| (local_y - y as f32).abs() <= 12.0)
        }) {
            self.settings_dragging = Some(slider);
            self.set_option_setting_from_cursor(slider, cursor.x);
        } else if (SYSTEM_OPTIONS_MUTE_CELL_X as f32..SYSTEM_OPTIONS_ROW_RIGHT as f32)
            .contains(&local_x)
            && let Some(control) = [
                OptionControl::MuteMusic,
                OptionControl::MuteEffects,
                OptionControl::ScreenShake,
            ]
            .into_iter()
            .find(|control| (local_y - control.system_checkbox_y() as f32).abs() <= 12.0)
        {
            self.settings_selected_slider = control;
            self.toggle_selected_option_setting();
        } else if let Some(control) = option_permission_at(local_x, local_y) {
            self.settings_selected_slider = control;
            self.toggle_selected_option_setting();
        } else if SYSTEM_OPTIONS_OK_BUTTON.contains(local_x, local_y) {
            self.save_settings();
        } else if SYSTEM_OPTIONS_CANCEL_BUTTON.contains(local_x, local_y) {
            self.cancel_settings();
        }
    }

    pub(super) fn refresh_chat_texture(&mut self) {
        let item_megaphone_open = self
            .cash_item_compose
            .as_ref()
            .is_some_and(|compose| compose.kind.is_item_megaphone(compose.item_id));
        let pixels = render_chat_panel(
            &self.legacy_ui_archive,
            &self.font,
            &self.chat_lines,
            if item_megaphone_open {
                ""
            } else {
                &self.chat_input
            },
            self.chat_open && !item_megaphone_open,
            self.chat_scroll,
            self.chat_input_focused && !item_megaphone_open,
            self.chat_target,
            (!item_megaphone_open)
                .then(|| {
                    self.cash_item_compose
                        .as_ref()
                        .map(|compose| compose.kind.label(compose.item_id))
                })
                .flatten(),
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.chat_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(CHAT_WIDTH, CHAT_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.chat_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update chat panel: {error}"),
        }
        self.refresh_item_megaphone_texture();
        self.refresh_map_hud();
    }

    pub(super) fn refresh_item_megaphone_texture(&mut self) {
        let Some((whisper, saved_message, attachment)) = self
            .cash_item_compose
            .as_ref()
            .filter(|compose| compose.kind.is_item_megaphone(compose.item_id))
            .map(|compose| {
                (
                    compose.whisper,
                    compose.lines.first().cloned(),
                    compose.attached_item,
                )
            })
        else {
            self.item_megaphone_texture = None;
            return;
        };
        let attachment_icon = attachment.and_then(|(_, _, item_id)| {
            inventory_item_icon(
                item_id,
                &self.item_archive,
                &self.character_archive,
                &mut self.inventory_icon_cache,
            )
        });
        let attachment_name = attachment.map(|(_, _, item_id)| {
            item_name(item_id, &self.string_archive, &mut self.item_name_cache)
        });
        let panel = match item_megaphone::render(
            &self.ui_archive,
            &self.font,
            &self.chat_input,
            saved_message.as_deref(),
            whisper,
            attachment_name.as_deref(),
            attachment_icon,
            self.login_caret_visible,
        ) {
            Ok(panel) => panel,
            Err(error) => {
                eprintln!("OpenStoryRS: could not render Item Megaphone: {error}");
                self.item_megaphone_texture = None;
                return;
            }
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.item_megaphone_texture.as_ref() {
            if texture.width() == panel.width && texture.height() == panel.height {
                renderer.update_bgra(texture, &panel.pixels).map(|()| None)
            } else {
                renderer
                    .upload_bgra(panel.width, panel.height, &panel.pixels)
                    .map(Some)
            }
        } else {
            renderer
                .upload_bgra(panel.width, panel.height, &panel.pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.item_megaphone_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not upload Item Megaphone: {error}"),
        }
    }

    pub(super) fn click_item_megaphone(&mut self, cursor: Point) -> bool {
        let active = self.current_field.is_some()
            && self
                .cash_item_compose
                .as_ref()
                .is_some_and(|compose| compose.kind.is_item_megaphone(compose.item_id));
        if !active {
            return false;
        }
        let local_x = cursor.x - ITEM_MEGAPHONE_X;
        let local_y = cursor.y - ITEM_MEGAPHONE_Y;
        let inside = (0.0..item_megaphone::WIDTH as f32).contains(&local_x)
            && (0.0..item_megaphone::HEIGHT as f32).contains(&local_y);
        if !inside {
            return false;
        }
        let whisper_box = (10.0..22.0).contains(&local_x) && (154.0..166.0).contains(&local_y);
        if whisper_box {
            if let Some(compose) = self.cash_item_compose.as_mut() {
                compose.whisper = !compose.whisper;
            }
            self.refresh_chat_texture();
            return true;
        }
        let attachment_box = (16.0..48.0).contains(&local_x) && (90.0..122.0).contains(&local_y);
        if attachment_box {
            let awaiting_attachment = self.cash_item_compose.as_ref().is_some_and(|compose| {
                compose.awaiting_attachment && compose.kind.uses_item_attachment(compose.item_id)
            });
            if let Some(compose) = self.cash_item_compose.as_mut() {
                if compose.attached_item.is_some() {
                    compose.attached_item = None;
                } else if !awaiting_attachment {
                    self.push_chat_line(
                        "[Item Megaphone] Enter a message first, then press Enter to choose an attachment."
                            .to_owned(),
                        [255, 222, 112],
                    );
                }
            }
            if awaiting_attachment
                && self
                    .cash_item_compose
                    .as_ref()
                    .is_some_and(|compose| compose.attached_item.is_none())
            {
                self.show_inventory = true;
                self.update_inventory_hover();
                self.refresh_inventory_texture();
            }
            self.refresh_chat_texture();
            return true;
        }
        if let Some(button) = item_megaphone::button_at(&self.ui_archive, local_x, local_y) {
            match button {
                item_megaphone::Button::Send => self.submit_chat_message(),
                item_megaphone::Button::Cancel => {
                    self.cash_item_compose = None;
                    self.chat_input.clear();
                    self.chat_open = false;
                    self.chat_input_focused = false;
                    self.item_megaphone_texture = None;
                    self.refresh_chat_texture();
                }
            }
            return true;
        }
        true
    }

    pub(super) fn ensure_whisper_conversation(&mut self, target: &str) -> usize {
        if let Some(index) = self
            .whisper_conversations
            .iter()
            .position(|conversation| conversation.target.eq_ignore_ascii_case(target))
        {
            return index;
        }
        if self.whisper_conversations.len() == MAX_WHISPER_CONVERSATIONS {
            self.whisper_conversations.remove(0);
            self.active_whisper = self.active_whisper.and_then(|index| match index {
                0 => None,
                index => Some(index - 1),
            });
        }
        self.whisper_conversations.push(WhisperConversation {
            target: target.to_owned(),
            lines: VecDeque::new(),
            input: String::new(),
        });
        self.whisper_conversations.len() - 1
    }

    pub(super) fn open_whisper_window(&mut self, target: &str, focus_input: bool) {
        let target = target.trim();
        if !(4..=13).contains(&target.chars().count()) {
            self.push_chat_line(
                "[Whisper] Character names must be 4–13 characters.".to_owned(),
                [255, 222, 112],
            );
            return;
        }
        let index = self.ensure_whisper_conversation(target);
        self.active_whisper = Some(index);
        self.whisper_input_focused = focus_input;
        self.whisper_target_focused = false;
        if focus_input {
            self.chat_open = false;
            self.chat_input_focused = false;
            self.gameplay_input = GameplayInput::default();
        }
        self.refresh_whisper_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn open_new_whisper_window(&mut self) {
        let index = self.ensure_whisper_conversation("");
        self.active_whisper = Some(index);
        self.whisper_input_focused = true;
        self.whisper_target_focused = true;
        self.chat_open = false;
        self.chat_input_focused = false;
        self.gameplay_input = GameplayInput::default();
        self.refresh_whisper_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn append_whisper_line(&mut self, target: &str, text: String, color: [u8; 3]) {
        let index = self.ensure_whisper_conversation(target);
        let text = if text.chars().count() > MAX_WHISPER_LINE_CHARS {
            let mut truncated = text
                .chars()
                .take(MAX_WHISPER_LINE_CHARS.saturating_sub(1))
                .collect::<String>();
            truncated.push('…');
            truncated
        } else {
            text
        };
        if let Some(conversation) = self.whisper_conversations.get_mut(index) {
            conversation.lines.push_back(ChatLine { text, color });
            while conversation.lines.len() > MAX_WHISPER_LINES {
                conversation.lines.pop_front();
            }
        }
        if self.active_whisper.is_none() || !self.whisper_input_focused {
            self.active_whisper = Some(index);
        }
        self.refresh_whisper_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn refresh_whisper_texture(&mut self) {
        let Some(index) = self.active_whisper else {
            return;
        };
        let Some(conversation) = self.whisper_conversations.get(index) else {
            self.active_whisper = None;
            return;
        };
        let pixels = render_whisper_panel(
            &self.ui_archive,
            &self.font,
            conversation,
            self.whisper_input_focused,
            self.whisper_target_focused,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.whisper_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(WHISPER_PANEL_WIDTH, WHISPER_PANEL_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.whisper_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update whisper window: {error}"),
        }
    }

    pub(super) fn send_whisper_message(
        &mut self,
        target: &str,
        message: &str,
        focus_input: bool,
    ) -> bool {
        let target = target.trim();
        let message = message.trim();
        if !(4..=13).contains(&target.chars().count()) {
            if self.active_whisper.is_some_and(|index| {
                self.whisper_conversations
                    .get(index)
                    .is_some_and(|conversation| conversation.target.is_empty())
            }) {
                self.append_whisper_line(
                    "",
                    "[Notice] Enter a recipient name of 4–13 characters.".to_owned(),
                    [255, 222, 112],
                );
            } else {
                self.push_chat_line(
                    "[Whisper] Character names must be 4–13 characters.".to_owned(),
                    [255, 222, 112],
                );
            }
            return false;
        }
        self.open_whisper_window(target, focus_input);
        let Some(index) = self.active_whisper else {
            return false;
        };
        if message.is_empty() {
            return false;
        }
        if message.chars().count() > MAX_CHAT_MESSAGE_CHARS {
            self.append_whisper_line(
                target,
                format!("[Notice] Messages are limited to {MAX_CHAT_MESSAGE_CHARS} characters."),
                [255, 222, 112],
            );
            return false;
        }
        if !self.channel_connected {
            self.append_whisper_line(
                target,
                "[Notice] Not connected to a game channel.".to_owned(),
                [255, 222, 112],
            );
            return false;
        }
        let packet = match openstory_protocol::whisper_packet(target, message) {
            Ok(packet) => packet,
            Err(error) => {
                self.append_whisper_line(
                    target,
                    format!("[Notice] Could not send whisper: {error}"),
                    [255, 105, 105],
                );
                return false;
            }
        };
        let send_result = self
            .session
            .as_ref()
            .ok_or_else(|| "Not connected to a game channel.".to_owned())
            .and_then(|session| {
                session
                    .send_packet(packet)
                    .map_err(|error| error.to_string())
            });
        if let Err(error) = send_result {
            self.channel_connected = false;
            self.append_whisper_line(
                target,
                format!("[Notice] Could not send whisper: {error}"),
                [255, 105, 105],
            );
            return false;
        }
        if let Some(conversation) = self.whisper_conversations.get_mut(index) {
            conversation.lines.push_back(ChatLine {
                text: format!("You: {message}"),
                color: [165, 200, 255],
            });
            while conversation.lines.len() > MAX_WHISPER_LINES {
                conversation.lines.pop_front();
            }
        }
        self.whisper_input_focused = focus_input;
        self.refresh_whisper_texture();
        true
    }

    pub(super) fn submit_whisper_input(&mut self) {
        let Some(index) = self.active_whisper else {
            return;
        };
        let Some(conversation) = self.whisper_conversations.get(index) else {
            return;
        };
        let target = conversation.target.clone();
        let message = conversation.input.trim().to_owned();
        if message.is_empty() {
            return;
        }
        if self.send_whisper_message(&target, &message, true)
            && let Some(index) = self.active_whisper
            && let Some(conversation) = self.whisper_conversations.get_mut(index)
        {
            conversation.input.clear();
            self.refresh_whisper_texture();
        }
    }

    pub(super) fn click_whisper_window(&mut self, cursor: Point) -> bool {
        if self.active_whisper.is_none()
            || self.current_field.is_none()
            || self.npc_dialogue.is_some()
            || self.storage.is_some()
            || self.cash_shop.is_some()
            || !(WHISPER_PANEL_X..WHISPER_PANEL_X + WHISPER_PANEL_WIDTH as f32).contains(&cursor.x)
            || !(WHISPER_PANEL_Y..WHISPER_PANEL_Y + WHISPER_PANEL_HEIGHT as f32).contains(&cursor.y)
        {
            return false;
        }
        let local_x = cursor.x - WHISPER_PANEL_X;
        let local_y = cursor.y - WHISPER_PANEL_Y;
        if WHISPER_CLOSE.contains(local_x, local_y) {
            self.active_whisper = None;
            self.whisper_input_focused = false;
            self.whisper_target_focused = false;
        } else if WHISPER_SEND.contains(local_x, local_y) {
            if self
                .active_whisper
                .and_then(|index| self.whisper_conversations.get(index))
                .is_some_and(whisper_can_send)
            {
                self.submit_whisper_input();
            }
        } else if (34.0..62.0).contains(&local_y)
            && self
                .active_whisper
                .and_then(|index| self.whisper_conversations.get(index))
                .is_some_and(|conversation| conversation.target.is_empty())
        {
            self.whisper_input_focused = true;
            self.whisper_target_focused = true;
            self.gameplay_input = GameplayInput::default();
        } else if local_y >= 310.0 {
            self.whisper_input_focused = true;
            self.whisper_target_focused = false;
            self.gameplay_input = GameplayInput::default();
        }
        self.refresh_whisper_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
        true
    }

    pub(super) fn show_status_message(&mut self, text: String, color: [u8; 3]) {
        self.push_status_message(text, color, None);
    }

    pub(super) fn show_item_status_message(&mut self, text: String, color: [u8; 3], item_id: i32) {
        let icon = inventory_item_icon(
            item_id,
            &self.item_archive,
            &self.character_archive,
            &mut self.inventory_icon_cache,
        );
        self.push_status_message(text, color, icon);
    }

    fn push_status_message(
        &mut self,
        text: String,
        color: [u8; 3],
        icon: Option<(u32, u32, Arc<[u8]>)>,
    ) {
        if text.is_empty() {
            return;
        }
        self.status_messages.push_front(StatusMessageRuntime {
            text,
            color,
            created_at: Instant::now(),
            icon,
            texture: None,
        });
        while self.status_messages.len() > STATUS_MESSAGE_LIMIT {
            self.status_messages.pop_back();
        }
        self.refresh_status_message_textures();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn refresh_status_message_textures(&mut self) {
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        for message in &mut self.status_messages {
            if message.texture.is_some() {
                continue;
            }
            let pixels = render_status_message_line(
                &self.font,
                &message.text,
                message.color,
                message.icon.as_ref(),
            );
            match renderer.upload_bgra(STATUS_MESSAGE_WIDTH, STATUS_MESSAGE_HEIGHT, &pixels) {
                Ok(texture) => message.texture = Some(texture),
                Err(error) => eprintln!("OpenStoryRS: could not upload status message: {error}"),
            }
        }
    }

    pub(super) fn push_chat_line(&mut self, text: String, color: [u8; 3]) {
        let text = self.unfurl_chat_mob_references(&text);
        self.chat_lines.push_back(ChatLine { text, color });
        while self.chat_lines.len() > 100 {
            self.chat_lines.pop_front();
        }
        self.refresh_chat_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn unfurl_chat_mob_references(&self, text: &str) -> String {
        let characters = text.chars().collect::<Vec<_>>();
        let mut output = String::with_capacity(text.len());
        let mut index = 0;

        while index < characters.len() {
            if characters[index] == '#'
                && characters
                    .get(index + 1)
                    .is_some_and(|code| code.eq_ignore_ascii_case(&'o'))
                && let Some(end_offset) = characters[index + 2..]
                    .iter()
                    .position(|character| *character == '#')
            {
                let end = index + 2 + end_offset;
                let id = characters[index + 2..end].iter().collect::<String>();
                if !id.is_empty()
                    && id.chars().all(|character| character.is_ascii_digit())
                    && let Some(name) = chat_mob_name(&self.string_archive, &id)
                {
                    output.push_str(&name);
                    index = end + 1;
                    continue;
                }
            }

            // Some channel messages spell a seven-digit mob ID as spaced digits,
            // for example "MOB 0 2 1 0 1 0 0" for mob 0210100.
            if index + 3 <= characters.len()
                && characters[index].eq_ignore_ascii_case(&'M')
                && characters[index + 1].eq_ignore_ascii_case(&'O')
                && characters[index + 2].eq_ignore_ascii_case(&'B')
                && (index == 0
                    || (!characters[index - 1].is_ascii_alphanumeric()
                        && characters[index - 1] != '_'))
                && characters
                    .get(index + 3)
                    .is_some_and(|character| character.is_whitespace())
                && let Some((id, end)) = spaced_chat_mob_id(&characters, index + 3)
                && let Some(name) = chat_mob_name(&self.string_archive, &id)
            {
                output.push_str(&name);
                index = end;
                continue;
            }

            output.push(characters[index]);
            index += 1;
        }

        output
    }
}

fn chat_mob_name(string_archive: &NxArchive, id: &str) -> Option<String> {
    let numeric_id = id.parse::<u32>().ok()?;
    let canonical_id = numeric_id.to_string();
    let padded_id = format!("{numeric_id:07}");
    for candidate in [id, canonical_id.as_str(), padded_id.as_str()] {
        let path = format!("Mob.img/{candidate}/name");
        let Some(node) = string_archive.root().resolve(&path).ok().flatten() else {
            continue;
        };
        if let Ok(NodeValue::String(name)) = node.value()
            && !name.trim().is_empty()
        {
            return Some(name);
        }
    }
    None
}

fn spaced_chat_mob_id(characters: &[char], start: usize) -> Option<(String, usize)> {
    let mut index = start;
    while characters
        .get(index)
        .is_some_and(|character| character.is_whitespace())
    {
        index += 1;
    }

    let mut digits = String::new();
    let mut end = index;
    while let Some(character) = characters.get(index) {
        if character.is_ascii_digit() {
            digits.push(*character);
            index += 1;
            end = index;
            continue;
        }
        if character.is_whitespace() {
            let mut next_digit = index;
            while characters
                .get(next_digit)
                .is_some_and(|next| next.is_whitespace())
            {
                next_digit += 1;
            }
            if characters
                .get(next_digit)
                .is_some_and(|next| next.is_ascii_digit())
            {
                index = next_digit;
                continue;
            }
        }
        break;
    }

    ((6..=7).contains(&digits.len())).then_some((digits, end))
}

fn bool_to_percent(value: bool) -> u8 {
    if value { 100 } else { 0 }
}

fn option_permission_at(x: f32, y: f32) -> Option<OptionControl> {
    [
        OptionControl::AllowWhisper,
        OptionControl::AllowFriendChat,
        OptionControl::AllowChatInvite,
        OptionControl::AllowTradeRequest,
        OptionControl::AllowPartyInvite,
        OptionControl::AllowExpeditionInvite,
        OptionControl::AllowFollow,
        OptionControl::AllowGuildChat,
        OptionControl::AllowGuildInvite,
        OptionControl::AllowAllianceChat,
        OptionControl::AllowAllianceInvite,
        OptionControl::AllowFamilyInvite,
    ]
    .into_iter()
    .enumerate()
    .find_map(|(index, control)| {
        (300.0..458.0)
            .contains(&x)
            .then_some(())
            .filter(|()| {
                let top = 31.0 + index as f32 * 18.0;
                (top..top + 18.0).contains(&y)
            })
            .map(|()| control)
    })
}

#[cfg(test)]
mod options_layout_tests {
    use super::*;

    #[test]
    fn social_rows_match_the_aligned_game_options_frame() {
        assert_eq!(
            option_permission_at(310.0, 31.0),
            Some(OptionControl::AllowWhisper)
        );
        assert_eq!(
            option_permission_at(310.0, 229.0),
            Some(OptionControl::AllowFamilyInvite)
        );
        assert_eq!(option_permission_at(310.0, 247.0), None);
        assert_eq!(option_permission_at(299.0, 31.0), None);
    }
}
