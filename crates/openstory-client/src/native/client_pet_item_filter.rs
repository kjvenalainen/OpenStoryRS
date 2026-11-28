use super::*;

impl ClientApp {
    pub(super) fn pet_item_filter_origin(&self) -> (f32, f32) {
        let (equipment_offset_x, equipment_offset_y) =
            self.dialog_window_offset(DialogWindow::EquipmentInventory);
        (
            PET_ITEM_FILTER_PANEL_X + equipment_offset_x,
            PET_ITEM_FILTER_PANEL_Y + equipment_offset_y,
        )
    }

    fn ensure_pet_item_filter_labels(&mut self) {
        if !self.pet_item_filter.item_labels_loaded {
            for item_id in self.pet_item_filter.ignored_item_ids.clone() {
                let name = item_name(item_id, &self.string_archive, &mut self.item_name_cache);
                self.pet_item_filter
                    .item_labels_lower
                    .insert(item_id, name.to_lowercase());
                self.pet_item_filter.item_labels.insert(item_id, name);
            }
            self.pet_item_filter.item_labels_loaded = true;
            ui_pet_item_filter::sort_item_order(&mut self.pet_item_filter);
        }
    }

    pub(super) fn open_pet_item_filter(&mut self) {
        if self.pet_item_filter.open {
            self.close_pet_item_filter();
            return;
        }
        self.pet_item_filter.open = true;
        self.pet_item_filter.search_focused = true;
        self.pet_item_filter.key_text_for_ime_dedup = None;
        self.pet_item_filter.notice = None;
        self.pet_item_filter.schedule_search();
        self.ensure_pet_item_filter_labels();
        self.raise_dialog_window(DialogWindow::PetItemFilter);
        self.refresh_pet_item_filter_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn close_pet_item_filter(&mut self) {
        self.pet_item_filter.open = false;
        self.pet_item_filter.search_focused = false;
        self.pet_item_filter.key_text_for_ime_dedup = None;
        self.pet_item_filter.search_due_at = None;
        self.pet_item_filter_texture = None;
    }

    pub(super) fn advance_pet_item_filter_search(&mut self, now: Instant) {
        let due = self
            .pet_item_filter
            .search_due_at
            .is_some_and(|deadline| now >= deadline);
        if !self.pet_item_filter.open || !due {
            return;
        }
        self.pet_item_filter.search_due_at = None;
        self.ensure_pet_item_filter_labels();
        if self.owl_name_index.is_none() {
            self.owl_name_index = Some(build_owl_item_name_index(&self.string_archive));
        }
        let (results, result_count) = ui_pet_item_filter::search_item_ids(
            &self.pet_item_filter.query,
            self.owl_name_index.as_deref().unwrap_or_default(),
            &self.pet_item_filter.ignored_item_lookup,
        );
        self.pet_item_filter.search_results = results;
        self.pet_item_filter.search_result_count = result_count;
        self.pet_item_filter.scroll = 0;
        self.refresh_pet_item_filter_texture();
    }

    pub(super) fn refresh_pet_item_filter_texture(&mut self) {
        if !self.pet_item_filter.open {
            return;
        }
        self.ensure_pet_item_filter_labels();
        let pixels = ui_pet_item_filter::render(
            &self.font,
            &self.legacy_ui_archive,
            &self.item_archive,
            &self.character_archive,
            &self.string_archive,
            &self.pet_item_filter,
            &mut self.item_name_cache,
            &mut self.inventory_icon_cache,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.pet_item_filter_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(
                    PET_ITEM_FILTER_PANEL_WIDTH,
                    PET_ITEM_FILTER_PANEL_HEIGHT,
                    &pixels,
                )
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.pet_item_filter_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update pet item filter: {error}"),
        }
    }

    pub(super) fn click_pet_item_filter(&mut self, cursor: Point) -> bool {
        if !self.pet_item_filter.open {
            return false;
        }
        let (origin_x, origin_y) = self.pet_item_filter_origin();
        let local_x = cursor.x - origin_x;
        let local_y = cursor.y - origin_y;
        if ui_pet_item_filter::CLOSE.contains(local_x, local_y) {
            self.close_pet_item_filter();
            return true;
        }
        if ui_pet_item_filter::search_field().contains(local_x, local_y) {
            self.pet_item_filter.search_focused = true;
            self.pet_item_filter.key_text_for_ime_dedup = None;
            self.pet_item_filter.notice = None;
            self.refresh_pet_item_filter_texture();
            return true;
        }
        if ui_pet_item_filter::search_dropdown_contains(local_x, local_y, &self.pet_item_filter) {
            if let Some(item_id) =
                ui_pet_item_filter::search_result_at(local_x, local_y, &self.pet_item_filter)
            {
                self.add_pet_item_filter_id(item_id);
            }
            return true;
        }
        self.pet_item_filter.search_focused = false;
        if let Some(action) = ui_pet_item_filter::action_at(local_x, local_y) {
            match action {
                ui_pet_item_filter::Action::Meso => {
                    self.pet_item_filter.ignore_meso = !self.pet_item_filter.ignore_meso;
                    self.pet_item_filter.notice = None;
                    self.persist_pet_item_filter();
                }
                ui_pet_item_filter::Action::LowLevel => {
                    self.pet_item_filter.ignore_below_level_equipment =
                        !self.pet_item_filter.ignore_below_level_equipment;
                    self.pet_item_filter.notice = None;
                    self.persist_pet_item_filter();
                }
                ui_pet_item_filter::Action::Register => self.register_pet_item_filter_item(),
                ui_pet_item_filter::Action::Delete => self.remove_pet_item_filter_item(),
            }
            self.refresh_pet_item_filter_texture();
            return true;
        }
        if let Some(scroll) =
            ui_pet_item_filter::scrollbar_click_position(local_x, local_y, &self.pet_item_filter)
        {
            self.pet_item_filter.scroll = scroll;
            self.refresh_pet_item_filter_texture();
            return true;
        }
        let selected = ui_pet_item_filter::row_at(local_x, local_y, &self.pet_item_filter);
        self.pet_item_filter.selected_item_id = selected;
        self.refresh_pet_item_filter_texture();
        true
    }

    pub(super) fn scroll_pet_item_filter(&mut self, cursor: Point, amount: f32) -> bool {
        if !self.pet_item_filter.open {
            return false;
        }
        let (origin_x, origin_y) = self.pet_item_filter_origin();
        if !ui_pet_item_filter::list_bounds(&self.pet_item_filter)
            .contains(cursor.x - origin_x, cursor.y - origin_y)
        {
            return false;
        }
        let maximum = ui_pet_item_filter::max_scroll(&self.pet_item_filter);
        self.pet_item_filter.scroll = if amount > 0.0 {
            self.pet_item_filter.scroll.saturating_sub(1)
        } else if amount < 0.0 {
            self.pet_item_filter.scroll.saturating_add(1).min(maximum)
        } else {
            self.pet_item_filter.scroll
        };
        self.refresh_pet_item_filter_texture();
        true
    }

    pub(super) fn handle_pet_item_filter_key(
        &mut self,
        key: PhysicalKey,
        state: ElementState,
        repeat: bool,
        text: Option<&str>,
    ) -> bool {
        if !self.pet_item_filter.open {
            return false;
        }
        if state != ElementState::Pressed {
            return true;
        }
        self.pet_item_filter.key_text_for_ime_dedup = None;
        match key {
            PhysicalKey::Code(KeyCode::Escape) if !repeat => {
                self.close_pet_item_filter();
            }
            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) if !repeat => {
                self.register_pet_item_filter_item();
                self.refresh_pet_item_filter_texture();
            }
            PhysicalKey::Code(KeyCode::Backspace) if self.pet_item_filter.search_focused => {
                self.pet_item_filter.query.pop();
                self.pet_item_filter.scroll = 0;
                self.pet_item_filter.notice = None;
                self.pet_item_filter.schedule_search();
                self.refresh_pet_item_filter_texture();
            }
            _ if self.pet_item_filter.search_focused => {
                if let Some(text) =
                    text.filter(|text| text.chars().any(|character| !character.is_control()))
                {
                    self.pet_item_filter.key_text_for_ime_dedup = Some(text.to_owned());
                    self.edit_pet_item_filter_query(text);
                }
            }
            _ => {}
        }
        true
    }

    pub(super) fn handle_pet_item_filter_text(&mut self, text: &str) -> bool {
        if !self.pet_item_filter.open || !self.pet_item_filter.search_focused {
            return false;
        }
        if self
            .pet_item_filter
            .key_text_for_ime_dedup
            .take()
            .as_deref()
            == Some(text)
        {
            return true;
        }
        self.pet_item_filter.key_text_for_ime_dedup = None;
        self.edit_pet_item_filter_query(text);
        true
    }

    fn edit_pet_item_filter_query(&mut self, text: &str) {
        for character in text.chars() {
            if !character.is_control() && self.pet_item_filter.query.chars().count() < 48 {
                self.pet_item_filter.query.push(character);
            }
        }
        self.pet_item_filter.scroll = 0;
        self.pet_item_filter.notice = None;
        self.pet_item_filter.schedule_search();
        self.refresh_pet_item_filter_texture();
    }

    fn register_pet_item_filter_item(&mut self) {
        let query = self.pet_item_filter.query.trim().to_owned();
        if query.is_empty() {
            self.pet_item_filter.notice = Some("Type an item name first".to_owned());
            return;
        }
        self.ensure_pet_item_filter_labels();
        if self.owl_name_index.is_none() {
            self.owl_name_index = Some(build_owl_item_name_index(&self.string_archive));
        }
        if query.contains('*') {
            let matching_item_ids = self
                .owl_name_index
                .as_deref()
                .map(|index| ui_pet_item_filter::wildcard_item_ids(&query, index))
                .unwrap_or_default();
            if matching_item_ids.is_empty() {
                self.pet_item_filter.notice = Some("No items match that wildcard".to_owned());
                return;
            }
            self.add_pet_item_filter_ids(&matching_item_ids);
            return;
        }
        let Some((item_id, _)) = self
            .owl_name_index
            .as_deref()
            .and_then(|index| ui_pet_item_filter::closest_item(&query, index))
        else {
            self.pet_item_filter.notice = Some("No close item name found".to_owned());
            return;
        };
        self.add_pet_item_filter_id(item_id);
    }

    fn add_pet_item_filter_ids(&mut self, item_ids: &[i32]) {
        self.ensure_pet_item_filter_labels();
        let mut known_ids = self
            .pet_item_filter
            .ignored_item_ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        let mut added_ids = Vec::new();
        for item_id in item_ids.iter().copied().filter(|item_id| *item_id > 0) {
            if !known_ids.insert(item_id) {
                continue;
            }
            let name = item_name(item_id, &self.string_archive, &mut self.item_name_cache);
            self.pet_item_filter.ignored_item_ids.push(item_id);
            self.pet_item_filter
                .item_labels_lower
                .insert(item_id, name.to_lowercase());
            self.pet_item_filter.item_labels.insert(item_id, name);
            added_ids.push(item_id);
        }

        self.pet_item_filter.clear_query();
        self.pet_item_filter.scroll = 0;
        self.pet_item_filter.selected_item_id = added_ids
            .first()
            .copied()
            .or_else(|| item_ids.first().copied());
        if added_ids.is_empty() {
            self.pet_item_filter.notice = Some(format!(
                "All {} matching items are already listed",
                item_ids.len()
            ));
            self.refresh_pet_item_filter_texture();
            return;
        }

        self.pet_item_filter.ignored_item_lookup = std::sync::Arc::new(
            self.pet_item_filter
                .ignored_item_ids
                .iter()
                .copied()
                .collect(),
        );
        ui_pet_item_filter::sort_item_order(&mut self.pet_item_filter);
        self.pet_item_filter.notice = Some(format!(
            "Added {} of {} matching items",
            added_ids.len(),
            item_ids.len()
        ));
        self.persist_pet_item_filter();
        self.refresh_pet_item_filter_texture();
    }

    pub(super) fn add_pet_item_filter_id(&mut self, item_id: i32) {
        if item_id <= 0 {
            return;
        }
        self.ensure_pet_item_filter_labels();
        let name = item_name(item_id, &self.string_archive, &mut self.item_name_cache);
        self.pet_item_filter.clear_query();
        self.pet_item_filter.scroll = 0;
        self.pet_item_filter.selected_item_id = Some(item_id);
        if self.pet_item_filter.ignored_item_ids.contains(&item_id) {
            self.pet_item_filter.notice = Some(format!("Already listed: {name}"));
            self.refresh_pet_item_filter_texture();
            return;
        }
        self.pet_item_filter.ignored_item_ids.push(item_id);
        self.pet_item_filter
            .item_labels_lower
            .insert(item_id, name.to_lowercase());
        self.pet_item_filter
            .item_labels
            .insert(item_id, name.clone());
        self.pet_item_filter.ignored_item_lookup = std::sync::Arc::new(
            self.pet_item_filter
                .ignored_item_ids
                .iter()
                .copied()
                .collect(),
        );
        ui_pet_item_filter::sort_item_order(&mut self.pet_item_filter);
        self.pet_item_filter.notice = Some(format!("Added: {name}"));
        self.persist_pet_item_filter();
        self.refresh_pet_item_filter_texture();
    }

    fn remove_pet_item_filter_item(&mut self) {
        let Some(item_id) = self.pet_item_filter.selected_item_id.take() else {
            self.pet_item_filter.notice = Some("Select an item to delete".to_owned());
            return;
        };
        self.pet_item_filter
            .ignored_item_ids
            .retain(|candidate| *candidate != item_id);
        self.pet_item_filter.item_labels.remove(&item_id);
        self.pet_item_filter.item_labels_lower.remove(&item_id);
        self.pet_item_filter.ignored_item_lookup = std::sync::Arc::new(
            self.pet_item_filter
                .ignored_item_ids
                .iter()
                .copied()
                .collect(),
        );
        ui_pet_item_filter::sort_item_order(&mut self.pet_item_filter);
        let name = item_name(item_id, &self.string_archive, &mut self.item_name_cache);
        self.pet_item_filter.notice = Some(format!("Deleted: {name}"));
        self.persist_pet_item_filter();
    }

    fn persist_pet_item_filter(&mut self) {
        match settings::save_pet_item_filter(
            &self.pet_item_filter.ignored_item_ids,
            self.pet_item_filter.ignore_meso,
            self.pet_item_filter.ignore_below_level_equipment,
        ) {
            Ok(_) => {}
            Err(error) => {
                eprintln!("OpenStoryRS: could not save pet item filter: {error}");
                self.pet_item_filter.notice = Some("Could not save filter settings".to_owned());
            }
        }
    }
}
