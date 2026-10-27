use super::*;

impl ClientApp {
    pub(super) fn toggle_event_list(&mut self) {
        if self.current_field.is_none() {
            return;
        }
        if self.event_list_open {
            self.close_event_list();
            return;
        }
        self.event_list_open = true;
        self.event_list_offset = self
            .event_list_offset
            .min(self.event_list.len().saturating_sub(4));
        self.next_event_second = Instant::now() + Duration::from_secs(1);
        self.request_event_info();
        self.next_event_info_request = Instant::now() + Duration::from_millis(1250);
        self.refresh_event_list_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn close_event_list(&mut self) {
        self.event_list_open = false;
        self.event_list_selected = None;
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn refresh_event_list_texture(&mut self) {
        if !self.event_list_open {
            return;
        }
        let panel = match build_event_list_panel(
            &self.legacy_ui_archive,
            &self.font,
            &self.string_archive,
            &mut self.item_name_cache,
            &self.event_list,
            self.event_list_offset,
            self.event_list_selected,
            self.event_list_received,
        ) {
            Ok(panel) => panel,
            Err(error) => {
                eprintln!("OpenStoryRS: could not build the v83 event list: {error}");
                return;
            }
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.event_list_texture.as_ref() {
            renderer.update_bgra(texture, &panel.pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(
                    EVENT_LIST_PANEL_WIDTH as u32,
                    EVENT_LIST_PANEL_HEIGHT as u32,
                    &panel.pixels,
                )
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.event_list_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update the v83 event list: {error}"),
        }
    }

    pub(super) fn request_event_info(&mut self) {
        if !self.channel_connected {
            return;
        }
        let packet = openstory_protocol::request_event_info_packet();
        let result = self
            .session
            .as_ref()
            .map(|session| session.send_packet(packet));
        if let Some(Err(error)) = result {
            eprintln!("OpenStoryRS: could not request event information: {error}");
            self.channel_connected = false;
        }
    }

    pub(super) fn apply_event_info(&mut self, events: Vec<openstory_protocol::ServerEventInfo>) {
        self.event_list = events
            .into_iter()
            .map(|event| {
                let peak_duration = self
                    .event_list_peak_durations
                    .entry(event.name.clone())
                    .and_modify(|peak| *peak = (*peak).max(event.seconds_remaining.max(0)))
                    .or_insert(event.seconds_remaining.max(0));
                EventListEntry {
                    event,
                    peak_duration: *peak_duration,
                }
            })
            .collect();
        self.event_list_received = true;
        self.event_list_offset = self
            .event_list_offset
            .min(self.event_list.len().saturating_sub(4));
        if self
            .event_list_selected
            .is_some_and(|selected| selected >= self.event_list.len())
        {
            self.event_list_selected = None;
        }
        self.refresh_event_list_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn tick_event_list(&mut self, now: Instant) {
        if !self.event_list_open {
            return;
        }
        if now >= self.next_event_second {
            for entry in &mut self.event_list {
                if entry.event.seconds_remaining > 0 {
                    entry.event.seconds_remaining -= 1;
                }
            }
            self.next_event_second = now + Duration::from_secs(1);
            self.refresh_event_list_texture();
        }
        if now >= self.next_event_info_request {
            self.request_event_info();
            self.next_event_info_request = now + Duration::from_millis(1250);
        }
    }

    pub(super) fn scroll_event_list(&mut self, direction: i32) {
        if !self.event_list_open || self.event_list.is_empty() {
            return;
        }
        let maximum = self.event_list.len().saturating_sub(4);
        self.event_list_offset = self
            .event_list_offset
            .saturating_add_signed(direction as isize)
            .min(maximum);
        self.refresh_event_list_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn click_event_list(&mut self, point: Point) -> bool {
        if !self.event_list_open {
            return false;
        }
        let panel = SpriteRect {
            x: EVENT_LIST_PANEL_X,
            y: EVENT_LIST_PANEL_Y,
            width: EVENT_LIST_PANEL_WIDTH as f32,
            height: EVENT_LIST_PANEL_HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        };
        let close = SpriteRect {
            x: EVENT_LIST_PANEL_X + EVENT_LIST_PANEL_WIDTH as f32 - 36.0,
            y: EVENT_LIST_PANEL_Y + 3.0,
            width: 24.0,
            height: 22.0,
            blend_mode: BlendMode::Alpha,
        };
        if rect_contains(close, point) {
            self.close_event_list();
            return true;
        }
        if !rect_contains(panel, point) {
            self.close_event_list();
            return true;
        }
        let local_y = point.y - EVENT_LIST_PANEL_Y - 48.0;
        if local_y >= 0.0 {
            let row = (local_y / 91.0) as usize;
            let row_position = local_y - row as f32 * 91.0;
            let index = self.event_list_offset + row;
            if row < 4 && row_position < 84.0 && index < self.event_list.len() {
                self.event_list_selected = Some(index);
                self.refresh_event_list_texture();
            }
        }
        true
    }

    pub(super) fn handle_event_list_key(&mut self, key: PhysicalKey) {
        match key {
            PhysicalKey::Code(KeyCode::Escape) => self.close_event_list(),
            PhysicalKey::Code(KeyCode::ArrowUp) => self.scroll_event_list(-1),
            PhysicalKey::Code(KeyCode::ArrowDown) => self.scroll_event_list(1),
            _ => {}
        }
    }
}
