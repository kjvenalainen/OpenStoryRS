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
            self.display_settings,
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
        self.display_settings_original = self.display_settings;
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
            let assets = build_quit_confirmation_sprites(&self.legacy_ui_archive)
                .or_else(|_| build_quit_confirmation_sprites(&self.ui_archive));
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
        self.display_settings = self.display_settings_original;
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
                self.display_settings_original = self.display_settings;
                self.input_settings_original = self.input_settings;
                self.gameplay_settings_original = self.gameplay_settings;
                self.social_settings_original = self.social_settings;
                self.apply_display_settings();
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

    fn apply_display_settings(&self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if self.display_settings.fullscreen {
            window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(
                window.current_monitor(),
            )));
        } else {
            window.set_fullscreen(None);
            let _ = window.request_inner_size(winit::dpi::LogicalSize::new(
                f64::from(self.display_settings.width),
                f64::from(self.display_settings.height),
            ));
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
            OptionControl::Windowed => self.display_settings.fullscreen = value < 50,
            OptionControl::Resolution800 if value >= 50 => {
                self.display_settings.width = 800;
                self.display_settings.height = 600;
            }
            OptionControl::Resolution1024 if value >= 50 => {
                self.display_settings.width = 1024;
                self.display_settings.height = 768;
            }
            OptionControl::Resolution800 | OptionControl::Resolution1024 => {}
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
            OptionControl::Windowed => bool_to_percent(!self.display_settings.fullscreen),
            OptionControl::Resolution800 => bool_to_percent(
                self.display_settings.width == 800 && self.display_settings.height == 600,
            ),
            OptionControl::Resolution1024 => bool_to_percent(
                self.display_settings.width == 1024 && self.display_settings.height == 768,
            ),
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
        const TRACK_LEFT: f32 = SETTINGS_PANEL_X + 104.0;
        const TRACK_WIDTH: f32 = 152.0;
        if slider.track_y().is_none() {
            return;
        }
        let percent = ((x - TRACK_LEFT) / TRACK_WIDTH * 100.0)
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
        if (98.0..272.0).contains(&local_x)
            && let Some(slider) = [
                OptionControl::GraphicsQuality,
                OptionControl::EffectsQuality,
                OptionControl::Music,
                OptionControl::SoundEffects,
                OptionControl::HpWarning,
                OptionControl::MpWarning,
                OptionControl::MouseSpeed,
            ]
            .into_iter()
            .find(|slider| {
                slider
                    .track_y()
                    .is_some_and(|y| (y as f32 - 12.0..y as f32 + 13.0).contains(&local_y))
            })
        {
            self.settings_selected_slider = slider;
            self.settings_dragging = Some(slider);
            self.set_option_setting_from_cursor(slider, cursor.x);
        } else if (231.0..258.0).contains(&local_y) && (263.0..296.0).contains(&local_x) {
            self.settings_selected_slider = OptionControl::ScreenShake;
            self.toggle_selected_option_setting();
        } else if (63.0..84.0).contains(&local_y) && (263.0..296.0).contains(&local_x) {
            self.settings_selected_slider = OptionControl::MuteMusic;
            self.toggle_selected_option_setting();
        } else if (91.0..112.0).contains(&local_y) && (263.0..296.0).contains(&local_x) {
            self.settings_selected_slider = OptionControl::MuteEffects;
            self.toggle_selected_option_setting();
        } else if (296.0..323.0).contains(&local_y) && (258.0..286.0).contains(&local_x) {
            self.settings_selected_slider = OptionControl::Windowed;
            self.toggle_selected_option_setting();
        } else if (258.0..284.0).contains(&local_y) && (64.0..158.0).contains(&local_x) {
            self.settings_selected_slider = OptionControl::Resolution800;
            self.set_option_setting(OptionControl::Resolution800, 100);
        } else if (258.0..284.0).contains(&local_y) && (164.0..286.0).contains(&local_x) {
            self.settings_selected_slider = OptionControl::Resolution1024;
            self.set_option_setting(OptionControl::Resolution1024, 100);
        } else if let Some(control) = option_permission_at(local_x, local_y) {
            self.settings_selected_slider = control;
            self.toggle_selected_option_setting();
        } else if (338.0..365.0).contains(&local_y) && (204.0..252.0).contains(&local_x) {
            self.save_settings();
        } else if (338.0..365.0).contains(&local_y) && (253.0..296.0).contains(&local_x) {
            self.cancel_settings();
        }
    }

    pub(super) fn refresh_chat_texture(&mut self) {
        let item_megaphone_open = self
            .cash_item_compose
            .as_ref()
            .is_some_and(|compose| compose.kind.is_item_megaphone(compose.item_id));
        let pixels = render_chat_panel(
            &self.ui_archive,
            &self.font,
            &self.chat_lines,
            if item_megaphone_open {
                ""
            } else {
                &self.chat_input
            },
            self.chat_open && !item_megaphone_open,
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
            || self.npc_shop.is_some()
            || self.storage.is_some()
            || self.cash_shop.is_some()
            || !(WHISPER_PANEL_X..WHISPER_PANEL_X + WHISPER_PANEL_WIDTH as f32).contains(&cursor.x)
            || !(WHISPER_PANEL_Y..WHISPER_PANEL_Y + WHISPER_PANEL_HEIGHT as f32).contains(&cursor.y)
        {
            return false;
        }
        let local_x = cursor.x - WHISPER_PANEL_X;
        let local_y = cursor.y - WHISPER_PANEL_Y;
        if local_y < 32.0 && local_x >= WHISPER_PANEL_WIDTH as f32 - 34.0 {
            self.active_whisper = None;
            self.whisper_input_focused = false;
            self.whisper_target_focused = false;
        } else if (34.0..62.0).contains(&local_y)
            && self
                .active_whisper
                .and_then(|index| self.whisper_conversations.get(index))
                .is_some_and(|conversation| conversation.target.is_empty())
        {
            self.whisper_input_focused = true;
            self.whisper_target_focused = true;
            self.gameplay_input = GameplayInput::default();
        } else if local_y >= WHISPER_PANEL_HEIGHT as f32 - 36.0 {
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
        if text.is_empty() {
            return;
        }
        self.status_messages.push_front(StatusMessageRuntime {
            text,
            color,
            created_at: Instant::now(),
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
            let pixels = render_status_message_line(&self.font, &message.text, message.color);
            match renderer.upload_bgra(STATUS_MESSAGE_WIDTH, STATUS_MESSAGE_HEIGHT, &pixels) {
                Ok(texture) => message.texture = Some(texture),
                Err(error) => eprintln!("OpenStoryRS: could not upload status message: {error}"),
            }
        }
    }

    pub(super) fn push_chat_line(&mut self, text: String, color: [u8; 3]) {
        self.chat_lines.push_back(ChatLine { text, color });
        while self.chat_lines.len() > 100 {
            self.chat_lines.pop_front();
        }
        self.refresh_chat_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
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
                let top = 397.0 + index as f32 * 18.0;
                (top..top + 18.0).contains(&y)
            })
            .map(|()| control)
    })
}
