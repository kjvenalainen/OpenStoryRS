use super::*;

impl ClientApp {
    pub(super) fn open_world_map(&mut self) {
        let Some((map_id, _)) = self.current_field else {
            return;
        };
        if self.world_map.search_catalog.is_none() {
            match world_map::load_search_catalog(&self.map_archive, &self.string_archive) {
                Ok(catalog) => self.world_map.search_catalog = Some(catalog),
                Err(error) => eprintln!("OpenStoryRS: could not index world maps: {error}"),
            }
        }
        self.world_map.open = true;
        self.world_map.search_open = true;
        self.world_map.search_query.clear();
        self.world_map.search_scroll = 0;
        self.world_map.search_focused = false;
        self.world_map.hovered_link = None;
        self.world_map.hovered_marker = None;
        self.world_map.selected_route_map_id = None;
        self.world_map.map_id = Some(map_id);
        self.world_map.map_path = None;
        self.refresh_world_map_texture();
    }

    pub(super) fn channel_switch_channel_count(&self) -> usize {
        self.selected_world
            .and_then(|world_id| self.worlds.iter().find(|world| world.id == world_id))
            .map(|world| world.channels.len())
            .filter(|count| *count > 0)
            .unwrap_or(channel_switch::MAX_VISIBLE_CHANNELS)
            .min(channel_switch::MAX_VISIBLE_CHANNELS)
            .max(1)
    }

    pub(super) fn open_channel_switch(&mut self) {
        if self.current_field.is_none()
            || !self.channel_connected
            || self.active_character.is_none()
        {
            return;
        }
        let maximum = self.channel_switch_channel_count().saturating_sub(1) as u8;
        self.channel_switch_selected = self.selected_channel.min(maximum);
        self.channel_switch_open = true;
        self.refresh_channel_switch_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn close_channel_switch(&mut self) {
        self.channel_switch_open = false;
        self.channel_switch_texture = None;
        self.channel_switch_actions.clear();
    }

    pub(super) fn refresh_channel_switch_texture(&mut self) {
        if !self.channel_switch_open {
            self.channel_switch_texture = None;
            self.channel_switch_actions.clear();
            return;
        }
        let channel_count = self.channel_switch_channel_count();
        self.channel_switch_selected = self
            .channel_switch_selected
            .min(channel_count.saturating_sub(1) as u8);
        let panel = match channel_switch::render(
            &self.ui_archive,
            &self.font,
            self.selected_world.unwrap_or(0),
            channel_count,
            self.selected_channel,
            self.channel_switch_selected,
        ) {
            Ok(panel) => panel,
            Err(error) => {
                eprintln!("OpenStoryRS: could not build channel selector: {error}");
                return;
            }
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        match renderer.upload_bgra(panel.width, panel.height, &panel.pixels) {
            Ok(texture) => {
                self.channel_switch_texture = Some(texture);
                self.channel_switch_actions = panel.actions;
            }
            Err(error) => eprintln!("OpenStoryRS: could not upload channel selector: {error}"),
        }
    }

    pub(super) fn move_channel_switch_selection(&mut self, dx: i8, dy: i8) {
        let count = self.channel_switch_channel_count();
        let current = usize::from(self.channel_switch_selected).min(count - 1);
        let current_channel = usize::from(self.selected_channel).min(count - 1);
        let next = if dx < 0 {
            if current > 0 { current - 1 } else { count - 1 }
        } else if dx > 0 {
            if current + 1 < count { current + 1 } else { 0 }
        } else if dy < 0 {
            let candidate = if current >= 5 {
                current - 5
            } else {
                current + 15
            };
            if candidate >= count {
                current_channel
            } else {
                candidate
            }
        } else if dy > 0 {
            let candidate = current + 5;
            if candidate >= count {
                current % 5
            } else {
                candidate
            }
        } else {
            current
        };
        self.channel_switch_selected = next as u8;
        self.refresh_channel_switch_texture();
    }

    pub(super) fn request_channel_switch(&mut self) {
        let selected = self.channel_switch_selected;
        if selected == self.selected_channel {
            self.close_channel_switch();
            return;
        }
        let packet = openstory_protocol::change_channel_packet(selected);
        let result = self
            .session
            .as_ref()
            .ok_or_else(|| "channel connection is unavailable".to_owned())
            .and_then(|session| session.send_packet(packet));
        match result {
            Ok(()) => {
                self.selected_channel = selected;
                self.close_channel_switch();
                self.channel_connected = false;
            }
            Err(error) => {
                eprintln!("OpenStoryRS: could not request channel change: {error}");
                self.push_chat_line(
                    "[Notice] Channel change request could not be sent.".to_owned(),
                    [255, 190, 125],
                );
                self.close_channel_switch();
                self.channel_connected = false;
            }
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn click_channel_switch(&mut self, cursor: Point) -> bool {
        if !self.channel_switch_open {
            return false;
        }
        let local_x = cursor.x - channel_switch::DEFAULT_X;
        let local_y = cursor.y - channel_switch::DEFAULT_Y;
        let action = self
            .channel_switch_actions
            .iter()
            .find(|hit| hit.bounds.contains(local_x, local_y))
            .map(|hit| hit.action);
        match action {
            Some(channel_switch::Action::Close) => self.close_channel_switch(),
            Some(channel_switch::Action::Change) => self.request_channel_switch(),
            Some(channel_switch::Action::Select(channel)) => {
                self.channel_switch_selected = channel;
                self.refresh_channel_switch_texture();
            }
            None => {}
        }
        true
    }

    pub(super) fn refresh_messenger_texture(&mut self) {
        let Some(state) = self.messenger.as_ref() else {
            self.messenger_texture = None;
            self.messenger_actions.clear();
            self.messenger_bounds = None;
            return;
        };
        let panel = match messenger::render(&self.ui_archive, &self.font, state) {
            Ok(panel) => panel,
            Err(error) => {
                eprintln!("OpenStoryRS: could not build Messenger: {error}");
                return;
            }
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        match renderer.upload_bgra(panel.width, panel.height, &panel.pixels) {
            Ok(texture) => {
                let state = self.messenger.as_ref().expect("messenger state retained");
                self.messenger_bounds = Some(messenger::HitRect {
                    x: state.x,
                    y: state.y,
                    width: panel.width as f32,
                    height: panel.height as f32,
                });
                self.messenger_texture = Some(texture);
                self.messenger_actions = panel.actions;
            }
            Err(error) => eprintln!("OpenStoryRS: could not upload Messenger: {error}"),
        }
    }

    pub(super) fn send_messenger_packet(&mut self, packet: Vec<u8>, context: &str) -> bool {
        let Some(session) = self.session.as_ref() else {
            self.push_chat_line(
                format!("[Messenger] Cannot {context}: the server session is unavailable."),
                [255, 180, 120],
            );
            return false;
        };
        match session.send_packet(packet) {
            Ok(()) => true,
            Err(error) => {
                self.channel_connected = false;
                eprintln!("OpenStoryRS: could not {context}: {error}");
                self.push_chat_line(format!("[Messenger] Could not {context}."), [255, 130, 130]);
                false
            }
        }
    }

    pub(super) fn handle_messenger_action(&mut self, action: messenger::Action) {
        match action {
            messenger::Action::Open => {
                if let Some(state) = self.messenger.as_mut() {
                    state.open();
                }
                self.messenger_chat_focused = false;
            }
            messenger::Action::Close | messenger::Action::Leave => {
                self.send_messenger_packet(
                    openstory_protocol::messenger_leave_packet(),
                    "leave Messenger",
                );
                self.messenger = None;
                self.messenger_chat_focused = false;
                self.messenger_drag_offset = None;
            }
            messenger::Action::Invite => {
                let prompt = self
                    .messenger
                    .as_ref()
                    .and_then(messenger::State::invite_prompt)
                    .cloned();
                if let Some(messenger::InvitePrompt::EnterName { draft }) = prompt {
                    if !draft.is_empty() {
                        match openstory_protocol::messenger_invite_packet(&draft) {
                            Ok(packet) => {
                                if self.send_messenger_packet(packet, "invite that player")
                                    && let Some(state) = self.messenger.as_mut()
                                {
                                    state.dismiss_invite_prompt();
                                }
                            }
                            Err(error) => {
                                eprintln!("OpenStoryRS: could not build Messenger invite: {error}")
                            }
                        }
                    }
                } else if let Some(state) = self.messenger.as_mut() {
                    state.begin_invite();
                    self.messenger_chat_focused = false;
                }
            }
            messenger::Action::Send => {
                let message = self
                    .messenger
                    .as_ref()
                    .map(|state| state.draft().to_owned())
                    .unwrap_or_default();
                let name = self
                    .active_character
                    .as_ref()
                    .map(|character| character.stats.name.clone())
                    .unwrap_or_default();
                if !message.is_empty() && !name.is_empty() {
                    match openstory_protocol::messenger_chat_packet(&format!("{name} : {message}"))
                    {
                        Ok(packet) => {
                            if self.send_messenger_packet(packet, "send Messenger chat") {
                                if let Some(state) = self.messenger.as_mut() {
                                    state.add_chat(&name, &message);
                                    state.set_draft("");
                                }
                                self.push_chat_line(
                                    format!("[Messenger] {name}: {message}"),
                                    [128, 200, 255],
                                );
                                self.refresh_chat_texture();
                            }
                        }
                        Err(error) => {
                            eprintln!("OpenStoryRS: could not build Messenger chat: {error}")
                        }
                    }
                }
            }
            messenger::Action::Accept => {
                if matches!(
                    self.messenger
                        .as_ref()
                        .and_then(messenger::State::invite_prompt),
                    Some(messenger::InvitePrompt::Incoming { .. })
                ) && self.send_messenger_packet(
                    openstory_protocol::messenger_accept_invite_packet(),
                    "accept Messenger invite",
                ) && let Some(state) = self.messenger.as_mut()
                {
                    state.dismiss_invite_prompt();
                }
            }
            messenger::Action::Decline => {
                let incoming_inviter = self
                    .messenger
                    .as_ref()
                    .and_then(messenger::State::invite_prompt)
                    .and_then(|prompt| match prompt {
                        messenger::InvitePrompt::Incoming { inviter } => Some(inviter.clone()),
                        messenger::InvitePrompt::EnterName { .. } => None,
                    });
                if let Some(inviter) = incoming_inviter {
                    if let Ok(packet) =
                        openstory_protocol::messenger_decline_invite_packet(&inviter, "")
                        && self.send_messenger_packet(packet, "decline Messenger invite")
                        && let Some(state) = self.messenger.as_mut()
                    {
                        state.dismiss_invite_prompt();
                    }
                } else if let Some(state) = self.messenger.as_mut() {
                    state.dismiss_invite_prompt();
                }
            }
            messenger::Action::FocusChat => {
                self.messenger_chat_focused = true;
            }
            messenger::Action::FocusInviteName => {
                self.messenger_chat_focused = false;
            }
        }
        self.refresh_messenger_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn click_messenger(&mut self, cursor: Point) -> bool {
        let Some(bounds) = self.messenger_bounds else {
            return self
                .messenger
                .as_ref()
                .is_some_and(|state| state.invite_prompt().is_some());
        };
        if !bounds.contains(cursor.x, cursor.y) {
            return self
                .messenger
                .as_ref()
                .is_some_and(|state| state.invite_prompt().is_some());
        }
        let action = self
            .messenger_actions
            .iter()
            .rev()
            .find(|hit| hit.bounds.contains(cursor.x, cursor.y))
            .map(|hit| hit.action);
        if let Some(action) = action {
            self.handle_messenger_action(action);
        } else if self
            .messenger
            .as_ref()
            .is_some_and(|state| state.invite_prompt().is_none())
            && cursor.y < bounds.y + 22.0
        {
            self.messenger_chat_focused = false;
            if let Some(state) = self.messenger.as_ref() {
                self.messenger_drag_offset = Some((cursor.x - state.x, cursor.y - state.y));
            }
        } else if self
            .messenger
            .as_ref()
            .is_some_and(|state| state.invite_prompt().is_none())
        {
            self.messenger_chat_focused = false;
        }
        true
    }

    pub(super) fn update_messenger_drag(&mut self, cursor: Point) {
        let Some((offset_x, offset_y)) = self.messenger_drag_offset else {
            return;
        };
        let Some(bounds) = self.messenger_bounds else {
            self.messenger_drag_offset = None;
            return;
        };
        if let Some(state) = self.messenger.as_mut() {
            state.x = cursor.x - offset_x;
            state.y = cursor.y - offset_y;
            state.clamp_to_viewport(bounds.width as u32, bounds.height as u32);
        }
        self.refresh_messenger_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn handle_messenger_key(
        &mut self,
        key: PhysicalKey,
        state: ElementState,
        repeat: bool,
    ) -> bool {
        if self.messenger.is_some() && matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
            if state == ElementState::Pressed && !repeat {
                self.handle_messenger_action(messenger::Action::Close);
            }
            return true;
        }

        let prompt = self
            .messenger
            .as_ref()
            .and_then(messenger::State::invite_prompt)
            .cloned();
        if let Some(prompt) = prompt {
            if state == ElementState::Pressed && !repeat {
                match key {
                    PhysicalKey::Code(KeyCode::Escape) => match prompt {
                        messenger::InvitePrompt::Incoming { .. } => {
                            self.handle_messenger_action(messenger::Action::Decline)
                        }
                        messenger::InvitePrompt::EnterName { .. } => {
                            if let Some(state) = self.messenger.as_mut() {
                                state.dismiss_invite_prompt();
                            }
                            self.refresh_messenger_texture();
                        }
                    },
                    PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => match prompt {
                        messenger::InvitePrompt::Incoming { .. } => {
                            self.handle_messenger_action(messenger::Action::Accept)
                        }
                        messenger::InvitePrompt::EnterName { .. } => {
                            self.handle_messenger_action(messenger::Action::Invite)
                        }
                    },
                    PhysicalKey::Code(KeyCode::KeyY)
                        if matches!(&prompt, messenger::InvitePrompt::Incoming { .. }) =>
                    {
                        self.handle_messenger_action(messenger::Action::Accept)
                    }
                    PhysicalKey::Code(KeyCode::KeyN)
                        if matches!(&prompt, messenger::InvitePrompt::Incoming { .. }) =>
                    {
                        self.handle_messenger_action(messenger::Action::Decline)
                    }
                    PhysicalKey::Code(KeyCode::Backspace)
                        if matches!(&prompt, messenger::InvitePrompt::EnterName { .. }) =>
                    {
                        if let Some(state) = self.messenger.as_mut() {
                            state.backspace_invite();
                        }
                        self.refresh_messenger_texture();
                    }
                    _ => {}
                }
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            return true;
        }

        if !self.messenger_chat_focused {
            if state == ElementState::Pressed
                && !repeat
                && matches!(key, PhysicalKey::Code(KeyCode::Escape))
            {
                self.handle_messenger_action(messenger::Action::Leave);
                return true;
            }
            return false;
        }
        if state == ElementState::Pressed && !repeat {
            match key {
                PhysicalKey::Code(KeyCode::Escape) => {
                    self.handle_messenger_action(messenger::Action::Leave);
                }
                PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                    self.handle_messenger_action(messenger::Action::Send);
                }
                PhysicalKey::Code(KeyCode::Backspace) => {
                    if let Some(state) = self.messenger.as_mut() {
                        state.backspace_draft();
                    }
                    self.refresh_messenger_texture();
                }
                _ => {}
            }
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }
        true
    }

    pub(super) fn handle_messenger_text(&mut self, text: &str) -> bool {
        let prompt = self
            .messenger
            .as_ref()
            .and_then(messenger::State::invite_prompt)
            .cloned();
        if let Some(prompt) = prompt {
            if matches!(prompt, messenger::InvitePrompt::EnterName { .. }) {
                if let Some(state) = self.messenger.as_mut() {
                    for character in text.chars() {
                        state.push_invite_char(character);
                    }
                }
                self.refresh_messenger_texture();
            }
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
            return true;
        }
        if !self.messenger_chat_focused {
            return false;
        }
        if let Some(state) = self.messenger.as_mut() {
            for character in text.chars() {
                state.push_draft_char(character);
            }
        }
        self.refresh_messenger_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
        true
    }

    pub(super) fn apply_messenger_event(&mut self, event: openstory_protocol::MessengerEvent) {
        match event {
            openstory_protocol::MessengerEvent::PlayerJoined {
                slot,
                look: _,
                name,
                channel,
                reserved: _,
            } => {
                if let Ok(slot) = usize::try_from(slot) {
                    if let Some(state) = self.messenger.as_mut() {
                        state.set_member(slot, &name);
                    }
                }
                eprintln!("OpenStoryRS: {name} joined Messenger on channel {channel}");
            }
            openstory_protocol::MessengerEvent::SlotAssigned { slot: _ } => {}
            openstory_protocol::MessengerEvent::PlayerLeft { slot } => {
                if let Ok(slot) = usize::try_from(slot)
                    && let Some(state) = self.messenger.as_mut()
                {
                    state.remove_member(slot);
                }
            }
            openstory_protocol::MessengerEvent::Invite { from, .. } => {
                if !self.social_settings.allow_chat_invite {
                    return;
                }
                self.messenger
                    .get_or_insert_with(messenger::State::default)
                    .prompt_incoming_invite(&from);
                self.messenger_chat_focused = false;
                self.push_chat_line(
                    format!("{from} invited you to a Messenger conversation."),
                    [255, 222, 112],
                );
            }
            openstory_protocol::MessengerEvent::Note { text } => {
                self.push_chat_line(text, [255, 128, 128]);
            }
            openstory_protocol::MessengerEvent::Chat { text } => {
                if let Some(state) = self.messenger.as_mut() {
                    if let Some((name, message)) = text.split_once(" : ") {
                        state.add_chat(name, message);
                    } else {
                        state.add_chat("", &text);
                    }
                }
                self.push_chat_line(format!("[Messenger] {text}"), [128, 200, 255]);
            }
            openstory_protocol::MessengerEvent::Unsupported { mode } => {
                eprintln!("OpenStoryRS: unsupported Messenger operation {mode:#04x}");
            }
        }
        if self.messenger.is_some() {
            self.refresh_messenger_texture();
        }
        self.refresh_chat_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn refresh_monster_book_texture(&mut self) {
        if !self.monster_book.open {
            return;
        }
        if self.monster_book.set_card_icons.is_none() {
            self.monster_book.set_card_icons =
                Some(monster_book::load_set_card_icons(&self.item_archive));
        }
        let panel = match monster_book::render(
            &self.ui_archive,
            &self.item_archive,
            &self.character_archive,
            &self.mob_archive,
            &self.string_archive,
            &self.monster_book.card_updates,
            self.monster_book
                .set_card_icons
                .as_ref()
                .expect("Monster Book card icons are initialized before rendering"),
            self.monster_book.cover_item_id,
            self.monster_book.page,
            self.monster_book.category,
            self.monster_book.selected_card,
            Some(self.monster_book.current_detail_animation()),
            self.monster_book.search_open,
            &self.monster_book.search_query,
            self.monster_book.search_notice.as_deref(),
            self.monster_book.set_effect_open,
            self.monster_book.set_scroll,
            self.monster_book.selected_set,
            self.monster_book.drop_scroll,
            &mut self.inventory_icon_cache,
            &self.font,
        ) {
            Ok(panel) => panel,
            Err(error) => {
                eprintln!("OpenStoryRS: could not build Monster Book: {error}");
                return;
            }
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let uploaded = renderer.upload_bgra(panel.width, panel.height, &panel.pixels);
        match uploaded {
            Ok(texture) => {
                self.monster_book.texture = Some(texture);
                self.monster_book.set_layout(panel);
                self.refresh_monster_book_animation_texture();
            }
            Err(error) => eprintln!("OpenStoryRS: could not upload Monster Book: {error}"),
        }
    }

    pub(super) fn refresh_monster_book_animation_texture(&mut self) {
        let sprite = match self
            .monster_book
            .refresh_detail_animation_sprite(&self.mob_archive)
        {
            Ok(sprite) => sprite,
            Err(error) => {
                self.monster_book_animation_texture = None;
                eprintln!("OpenStoryRS: could not decode Monster Book animation: {error}");
                return;
            }
        };
        let Some(sprite) = sprite else {
            self.monster_book_animation_texture = None;
            return;
        };
        let uploaded = self
            .renderer
            .as_ref()
            .map(|renderer| renderer.upload_bgra(sprite.width, sprite.height, &sprite.pixels));
        match uploaded {
            Some(Ok(texture)) => self.monster_book_animation_texture = Some(texture),
            Some(Err(error)) => {
                self.monster_book_animation_texture = None;
                eprintln!("OpenStoryRS: could not upload Monster Book animation: {error}");
            }
            None => self.monster_book_animation_texture = None,
        }
    }

    pub(super) fn click_monster_book(&mut self, cursor: Point) -> bool {
        if !self.monster_book.open {
            return false;
        }
        if self.monster_book.search_open {
            return true;
        }
        let inside_panel = cursor.x >= self.monster_book.x
            && cursor.y >= self.monster_book.y
            && cursor.x < self.monster_book.x + self.monster_book.width as f32
            && cursor.y < self.monster_book.y + self.monster_book.height as f32;
        if !inside_panel {
            return false;
        }

        let mut refresh = false;
        if self
            .monster_book
            .close
            .is_some_and(|bounds| bounds.contains(cursor.x, cursor.y))
        {
            self.monster_book.open = false;
            self.monster_book.texture = None;
            self.monster_book.clear_selected_card();
            self.monster_book_animation_texture = None;
            return true;
        }
        if self
            .monster_book
            .search
            .is_some_and(|bounds| bounds.contains(cursor.x, cursor.y))
        {
            self.monster_book.search_open = true;
            self.monster_book.search_query.clear();
            self.monster_book.search_notice = None;
            refresh = true;
        } else if self
            .monster_book
            .set_effect
            .is_some_and(|bounds| bounds.contains(cursor.x, cursor.y))
        {
            self.monster_book.set_effect_open = !self.monster_book.set_effect_open;
            self.monster_book.selected_set = None;
            self.monster_book.set_scroll = 0;
            self.monster_book.clear_selected_card();
            self.monster_book.drop_scroll = 0;
            self.monster_book.search_open = false;
            refresh = true;
        } else if let Some(set_index) = self
            .monster_book
            .set_cells
            .iter()
            .find(|hit| hit.bounds.contains(cursor.x, cursor.y))
            .map(|hit| hit.index)
        {
            self.monster_book.selected_set =
                (self.monster_book.selected_set != Some(set_index)).then_some(set_index);
            refresh = true;
        } else if self
            .monster_book
            .previous_page
            .is_some_and(|bounds| bounds.contains(cursor.x, cursor.y))
        {
            self.monster_book.page = self.monster_book.page.saturating_sub(1);
            self.monster_book.clear_selected_card();
            self.monster_book.drop_scroll = 0;
            refresh = true;
        } else if self
            .monster_book
            .next_page
            .is_some_and(|bounds| bounds.contains(cursor.x, cursor.y))
        {
            self.monster_book.page = self.monster_book.page.saturating_add(1);
            self.monster_book.clear_selected_card();
            self.monster_book.drop_scroll = 0;
            refresh = true;
        } else if let Some(category) = self
            .monster_book
            .categories
            .iter()
            .find(|hit| hit.bounds.contains(cursor.x, cursor.y))
            .map(|hit| hit.category)
        {
            self.monster_book.category =
                (self.monster_book.category != Some(category)).then_some(category);
            self.monster_book.page = 0;
            self.monster_book.clear_selected_card();
            self.monster_book.drop_scroll = 0;
            self.monster_book.set_effect_open = false;
            self.monster_book.selected_set = None;
            refresh = true;
        } else if let Some((item_id, mob_id)) = self
            .monster_book
            .cards
            .iter()
            .find(|hit| hit.bounds.contains(cursor.x, cursor.y))
            .map(|hit| (hit.item_id, hit.mob_id))
        {
            self.monster_book
                .select_card(item_id, mob_id, &self.mob_archive);
            self.monster_book.drop_scroll = 0;
            self.monster_book.set_effect_open = false;
            self.monster_book.selected_set = None;
            refresh = true;
        } else if self
            .monster_book
            .detail_animation_area
            .is_some_and(|bounds| bounds.contains(cursor.x, cursor.y))
        {
            self.monster_book.cycle_detail_animation();
            refresh = true;
        } else if self
            .monster_book
            .drop_area
            .is_some_and(|bounds| bounds.contains(cursor.x, cursor.y))
        {
            return true;
        } else if self
            .monster_book
            .detail_back
            .is_some_and(|bounds| bounds.contains(cursor.x, cursor.y))
        {
            self.monster_book.clear_selected_card();
            self.monster_book.drop_scroll = 0;
            refresh = true;
        }
        if refresh {
            self.refresh_monster_book_texture();
        }
        true
    }

    pub(super) fn scroll_monster_book_drops(&mut self, cursor: Point, amount: f32) -> bool {
        if !self.monster_book.open || self.monster_book.search_open {
            return false;
        }
        let Some(area) = self.monster_book.drop_area else {
            return false;
        };
        if !area.contains(cursor.x, cursor.y) {
            return false;
        }
        let max_scroll = monster_book::drop_scroll_max(self.monster_book.drop_count);
        let next = if amount > 0.0 {
            self.monster_book.drop_scroll.saturating_sub(1)
        } else if amount < 0.0 {
            self.monster_book
                .drop_scroll
                .saturating_add(1)
                .min(max_scroll)
        } else {
            self.monster_book.drop_scroll
        };
        if next != self.monster_book.drop_scroll {
            self.monster_book.drop_scroll = next;
            self.refresh_monster_book_texture();
        }
        true
    }

    pub(super) fn submit_monster_book_search(&mut self) {
        let query = self.monster_book.search_query.trim();
        if query.is_empty() {
            return;
        }
        if let Some(page) = monster_book::find_card_page(
            &self.item_archive,
            &self.mob_archive,
            &self.string_archive,
            &self.monster_book.card_updates,
            self.monster_book.category,
            query,
        ) {
            self.monster_book.page = page;
            self.monster_book.clear_selected_card();
            self.monster_book.drop_scroll = 0;
            self.monster_book.search_open = false;
            self.monster_book.search_notice = None;
        } else {
            self.monster_book.search_notice = Some("No matching card found.".to_owned());
        }
        self.refresh_monster_book_texture();
    }

    pub(super) fn refresh_world_map_texture(&mut self) {
        if !self.world_map.open {
            return;
        }
        let Some((map_id, _)) = self.current_field else {
            self.world_map.open = false;
            self.world_map.texture = None;
            return;
        };
        if self.world_map.map_id != Some(map_id) {
            self.world_map.map_id = Some(map_id);
            self.world_map.map_path = None;
            self.world_map.hovered_link = None;
            self.world_map.hovered_marker = None;
            self.world_map.selected_route_map_id = None;
        }
        let panel = match world_map::render(
            &self.map_archive,
            &self.ui_archive,
            map_id,
            self.world_map.map_path.as_deref(),
            self.world_map.search_open,
            &self.world_map.search_query,
            self.world_map.search_focused,
            self.world_map.search_catalog.as_ref(),
            self.world_map.search_scroll,
            self.world_map.hovered_link,
            self.world_map.hovered_marker,
            self.world_map.selected_route_map_id,
            &self.string_archive,
            &self.font,
        ) {
            Ok(panel) => panel,
            Err(error) => {
                eprintln!("OpenStoryRS: could not build world map: {error}");
                return;
            }
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        match renderer.upload_bgra(panel.width, panel.height, &panel.pixels) {
            Ok(texture) => {
                self.world_map.texture = Some(texture);
                self.world_map.set_layout(panel);
            }
            Err(error) => eprintln!("OpenStoryRS: could not upload world map: {error}"),
        }
    }

    pub(super) fn click_world_map(&mut self, cursor: Point) -> bool {
        if !self.world_map.open {
            return false;
        }
        let inside_panel = cursor.x >= self.world_map.x
            && cursor.y >= self.world_map.y
            && cursor.x < self.world_map.x + self.world_map.width as f32
            && cursor.y < self.world_map.y + self.world_map.height as f32;
        let mut refresh = false;
        if self
            .world_map
            .close
            .is_some_and(|bounds| bounds.contains(cursor.x, cursor.y))
        {
            self.world_map.open = false;
            self.world_map.search_focused = false;
            self.world_map.hovered_link = None;
            self.world_map.hovered_marker = None;
            return true;
        }
        if self
            .world_map
            .search_close
            .is_some_and(|bounds| bounds.contains(cursor.x, cursor.y))
        {
            self.world_map.search_open = false;
            self.world_map.search_focused = false;
            self.world_map.hovered_link = None;
            self.world_map.hovered_marker = None;
            refresh = true;
        } else if self
            .world_map
            .search_toggle
            .is_some_and(|bounds| bounds.contains(cursor.x, cursor.y))
        {
            self.world_map.search_open = !self.world_map.search_open;
            self.world_map.search_focused = false;
            self.world_map.hovered_link = None;
            self.world_map.hovered_marker = None;
            refresh = true;
        } else if self
            .world_map
            .search_input
            .is_some_and(|bounds| bounds.contains(cursor.x, cursor.y))
        {
            if !self.world_map.search_focused {
                self.world_map.search_focused = true;
                refresh = true;
            }
        } else if let Some(hit) = self
            .world_map
            .search_hits
            .iter()
            .find(|hit| hit.bounds.contains(cursor.x, cursor.y))
            .cloned()
        {
            self.world_map.map_path = Some(hit.world_map);
            self.world_map.selected_route_map_id = Some(hit.map_id);
            self.world_map.search_focused = false;
            self.world_map.hovered_link = None;
            self.world_map.hovered_marker = None;
            refresh = true;
        } else if self
            .world_map
            .search_submit
            .is_some_and(|bounds| bounds.contains(cursor.x, cursor.y))
        {
            if self.world_map.search_focused {
                self.world_map.search_focused = false;
                refresh = true;
            }
        } else if let Some(target) = self
            .world_map
            .links
            .iter()
            .find(|link| link.bounds.contains(cursor.x, cursor.y))
            .map(|link| link.target.clone())
        {
            self.world_map.map_path = Some(target);
            self.world_map.search_focused = false;
            self.world_map.hovered_link = None;
            self.world_map.hovered_marker = None;
            self.world_map.selected_route_map_id = None;
            refresh = true;
        } else if let Some((map_id, clears_selected_route)) = self
            .world_map
            .markers
            .iter()
            .find(|marker| marker.bounds.contains(cursor.x, cursor.y))
            .map(|marker| {
                (
                    marker.map_id,
                    self.world_map
                        .selected_route_map_id
                        .is_some_and(|selected| marker.map_ids.contains(&selected)),
                )
            })
        {
            self.world_map.selected_route_map_id = (!clears_selected_route).then_some(map_id);
            self.world_map.search_focused = false;
            self.world_map.hovered_marker = None;
            refresh = true;
        } else {
            if self.world_map.search_focused {
                self.world_map.search_focused = false;
                refresh = true;
            }
        }
        if refresh {
            self.refresh_world_map_texture();
        }
        inside_panel
    }

    pub(super) fn escape_world_map(&mut self) {
        if self.world_map.search_open {
            self.world_map.search_open = false;
            self.world_map.search_focused = false;
            self.world_map.hovered_link = None;
            self.world_map.hovered_marker = None;
            self.refresh_world_map_texture();
            return;
        }
        if let Some(parent) = self
            .world_map
            .parent_map
            .clone()
            .filter(|parent| !parent.is_empty())
        {
            self.world_map.map_path = Some(parent);
            self.world_map.hovered_link = None;
            self.world_map.hovered_marker = None;
            self.world_map.selected_route_map_id = None;
            self.refresh_world_map_texture();
        } else {
            self.world_map.open = false;
        }
    }

    pub(super) fn edit_world_map_search(&mut self, text: &str) {
        if !self.world_map.open || !self.world_map.search_focused {
            return;
        }
        for character in text.chars() {
            if !character.is_control() && self.world_map.search_query.chars().count() < 8 {
                self.world_map.search_query.push(character);
            }
        }
        self.world_map.search_scroll = 0;
        self.world_map.selected_route_map_id = None;
        self.world_map.hovered_marker = None;
        self.refresh_world_map_texture();
    }

    pub(super) fn backspace_world_map_search(&mut self) {
        if !self.world_map.open || !self.world_map.search_focused {
            return;
        }
        self.world_map.search_query.pop();
        self.world_map.search_scroll = 0;
        self.world_map.selected_route_map_id = None;
        self.world_map.hovered_marker = None;
        self.refresh_world_map_texture();
    }

    pub(super) fn scroll_world_map_results(&mut self, cursor: Point, amount: f32) -> bool {
        if !self.world_map.open
            || !self.world_map.search_open
            || self.world_map.search_query.trim().is_empty()
            || !self.world_map.search_input.is_some_and(|input| {
                cursor.x >= input.x - 6.0
                    && cursor.x < self.world_map.x + self.world_map.width as f32
                    && (self.world_map.y..self.world_map.y + self.world_map.height as f32)
                        .contains(&cursor.y)
            })
        {
            return false;
        }
        let max_scroll = self
            .world_map
            .search_result_count
            .saturating_sub(world_map::MAX_VISIBLE_SEARCH_RESULTS);
        let next = if amount > 0.0 {
            self.world_map.search_scroll.saturating_sub(1)
        } else if amount < 0.0 {
            self.world_map
                .search_scroll
                .saturating_add(1)
                .min(max_scroll)
        } else {
            self.world_map.search_scroll
        };
        if next == self.world_map.search_scroll {
            return true;
        }
        self.world_map.search_scroll = next;
        self.refresh_world_map_texture();
        true
    }

    pub(super) fn update_world_map_hover(&mut self, cursor: Point) -> bool {
        if !self.world_map.open {
            return false;
        }
        let hovered_link = self
            .world_map
            .links
            .iter()
            .position(|link| link.bounds.contains(cursor.x, cursor.y));
        let hovered_marker = self
            .world_map
            .markers
            .iter()
            .position(|marker| marker.bounds.contains(cursor.x, cursor.y));
        if self.world_map.hovered_link == hovered_link
            && self.world_map.hovered_marker == hovered_marker
        {
            return false;
        }
        self.world_map.hovered_link = hovered_link;
        self.world_map.hovered_marker = hovered_marker;
        self.refresh_world_map_texture();
        true
    }

    pub(super) fn open_party_search(&mut self) {
        self.party_search
            .get_or_insert_with(party_search::State::default)
            .set_status("This server does not publish party result rows.");
        self.refresh_party_search_texture();
    }

    pub(super) fn refresh_party_search_texture(&mut self) {
        let Some(state) = self.party_search.as_ref() else {
            self.party_search_texture = None;
            self.party_search_actions.clear();
            return;
        };
        let panel = match party_search::render(&self.ui_archive, &self.font, state) {
            Ok(panel) => panel,
            Err(error) => {
                eprintln!("OpenStoryRS: could not build Party Search: {error}");
                return;
            }
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        match renderer.upload_bgra(panel.width, panel.height, &panel.pixels) {
            Ok(texture) => {
                self.party_search_texture = Some(texture);
                self.party_search_actions = panel.actions;
            }
            Err(error) => eprintln!("OpenStoryRS: could not upload Party Search: {error}"),
        }
    }

    pub(super) fn click_party_search(&mut self, cursor: Point) -> bool {
        let Some(state) = self.party_search.as_ref() else {
            return false;
        };
        let inside_panel = cursor.x >= state.x
            && cursor.y >= state.y
            && cursor.x < state.x + party_search::PANEL_WIDTH as f32
            && cursor.y < state.y + party_search::PANEL_HEIGHT as f32;
        if !inside_panel {
            return false;
        }

        let action = self
            .party_search_actions
            .iter()
            .find(|hit| hit.bounds.contains(cursor.x, cursor.y))
            .map(|hit| hit.action);
        match action {
            Some(party_search::Action::Close | party_search::Action::Cancel) => {
                self.party_search = None;
                self.party_search_texture = None;
                self.party_search_actions.clear();
            }
            Some(party_search::Action::Start) => {
                if let Some(state) = self.party_search.as_mut() {
                    state.set_searching(true);
                    state.set_status("This server does not publish party result rows.");
                }
                self.refresh_party_search_texture();
            }
            Some(party_search::Action::Stop | party_search::Action::Pause) => {
                if let Some(state) = self.party_search.as_mut() {
                    state.set_searching(false);
                    state.set_status("Cosmic does not provide a request to stop party matching.");
                }
                self.refresh_party_search_texture();
            }
            Some(party_search::Action::Register) => {
                let level = self
                    .active_character
                    .as_ref()
                    .map_or(1, |character| i32::from(character.stats.level));
                let criteria = openstory_protocol::PartySearchCriteria::around_level(level);
                let sent = self.send_party_packet(
                    openstory_protocol::party_search_start_packet(criteria),
                    "Party search request sent.",
                );
                if let Some(state) = self.party_search.as_mut() {
                    state.set_searching(sent);
                    if sent {
                        state.set_status(
                            "Request sent; matching leaders may invite you through Party.",
                        );
                    }
                }
                self.refresh_party_search_texture();
            }
            None => {}
        }
        true
    }

    pub(super) fn open_status_window(&mut self) {
        if self.current_field.is_none()
            || self.active_character.is_none()
            || self.storage.is_some()
            || self.npc_shop.is_some()
            || self.npc_dialogue.is_some()
            || self.cash_shop.is_some()
        {
            return;
        }
        self.show_status = !self.show_status;
        self.show_status_detail = false;
        self.status_auto_confirmation = false;
        if self.show_status {
            self.show_inventory = false;
            self.inventory_drag_source = None;
            self.inventory_hover_slot = None;
            self.show_pet_equipment = false;
            self.show_skill_book = false;
            self.show_quest_log = false;
            self.show_party_panel = false;
            self.show_buddy_panel = false;
            self.show_guild_panel = false;
            self.show_alliance_panel = false;
            self.refresh_status_texture();
        }
    }

    pub(super) fn refresh_status_texture(&mut self) {
        if !self.show_status {
            return;
        }
        let Some(character) = self.active_character.as_ref() else {
            self.status_texture = None;
            self.status_detail_texture = None;
            return;
        };
        let pixels = render_status_panel(
            &self.font,
            &self.ui_archive,
            character,
            self.status_waiting_for_stats,
            self.status_auto_confirmation,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.status_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(STATUS_PANEL_WIDTH, STATUS_PANEL_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.status_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update stats panel: {error}"),
        }

        if self.show_status_detail {
            let pixels = render_status_detail_panel(
                &self.font,
                &self.ui_archive,
                character,
                self.inventory.as_ref(),
                &self.character_archive,
                &self.item_archive,
                &self.skill_archive,
                &self.skills,
                &self.active_buffs,
                &self.forced_equipment_stats,
            );
            let result = if let Some(texture) = self.status_detail_texture.as_ref() {
                renderer.update_bgra(texture, &pixels).map(|()| None)
            } else {
                renderer
                    .upload_bgra(STATUS_DETAIL_WIDTH, STATUS_DETAIL_HEIGHT, &pixels)
                    .map(Some)
            };
            match result {
                Ok(Some(texture)) => self.status_detail_texture = Some(texture),
                Ok(None) => {}
                Err(error) => {
                    eprintln!("OpenStoryRS: could not update detailed stats panel: {error}")
                }
            }
        } else {
            self.status_detail_texture = None;
        }
    }

    pub(super) fn send_ability_point_requests(
        &mut self,
        stat: openstory_protocol::AbilityStat,
        count: u16,
    ) {
        let available = self
            .active_character
            .as_ref()
            .map_or(0, |character| character.stats.ability_points);
        let count = count.min(available);
        if count == 0 {
            return;
        }
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let start_tick = self.client_started_at.elapsed().as_millis() as u32;
        for offset in 0..count {
            let client_tick = start_tick.wrapping_add(u32::from(offset)) as i32;
            let packet = openstory_protocol::spend_ability_point_packet(stat, client_tick);
            if let Err(error) = session.send_packet(packet) {
                eprintln!("OpenStoryRS: could not spend ability point: {error}");
                self.channel_connected = false;
                self.status_waiting_for_stats = false;
                self.refresh_status_texture();
                return;
            }
        }
        self.status_waiting_for_stats = true;
        self.refresh_status_texture();
    }

    pub(super) fn resolve_status_auto_allocate(&mut self, accepted: bool) {
        self.status_auto_confirmation = false;
        if accepted {
            if let Some(character) = self.active_character.as_ref() {
                let primary = primary_ability_stat(character);
                let points = character.stats.ability_points;
                self.send_ability_point_requests(primary, points);
            }
        }
        self.refresh_status_texture();
    }

    pub(super) fn click_status_panel(&mut self, cursor: Point) {
        let x = cursor.x - STATUS_PANEL_X;
        let y = cursor.y - STATUS_PANEL_Y;
        if self.status_auto_confirmation {
            if (185.0..215.0).contains(&y) && (20.0..80.0).contains(&x) {
                self.resolve_status_auto_allocate(true);
            } else if (185.0..215.0).contains(&y) && (93.0..153.0).contains(&x) {
                self.resolve_status_auto_allocate(false);
            }
            return;
        }
        if (5.0..22.0).contains(&y) && (153.0..174.0).contains(&x) {
            self.show_status = false;
            self.show_status_detail = false;
            return;
        }
        if (323.0..342.0).contains(&y) && (124.0..173.0).contains(&x) {
            self.show_status_detail = !self.show_status_detail;
            self.refresh_status_texture();
            return;
        }
        if (203.0..238.0).contains(&y) && (98.0..171.0).contains(&x) {
            if self
                .active_character
                .as_ref()
                .is_some_and(|character| character.stats.ability_points > 0)
            {
                self.status_auto_confirmation = true;
                self.refresh_status_texture();
            }
            return;
        }
        if self.status_waiting_for_stats {
            return;
        }
        let stat = if (114.0..132.0).contains(&y) && (56.0..171.0).contains(&x) {
            Some(openstory_protocol::AbilityStat::Hp)
        } else if (132.0..150.0).contains(&y) && (56.0..171.0).contains(&x) {
            Some(openstory_protocol::AbilityStat::Mp)
        } else if (244.0..262.0).contains(&y) && (56.0..171.0).contains(&x) {
            Some(openstory_protocol::AbilityStat::Strength)
        } else if (262.0..280.0).contains(&y) && (56.0..171.0).contains(&x) {
            Some(openstory_protocol::AbilityStat::Dexterity)
        } else if (280.0..298.0).contains(&y) && (56.0..171.0).contains(&x) {
            Some(openstory_protocol::AbilityStat::Intelligence)
        } else if (298.0..316.0).contains(&y) && (56.0..171.0).contains(&x) {
            Some(openstory_protocol::AbilityStat::Luck)
        } else {
            None
        };
        if let Some(stat) = stat {
            self.send_ability_point_requests(stat, 1);
        }
    }
}
