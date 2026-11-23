use super::*;

impl ClientApp {
    pub(super) fn flush_inventory_ui_refresh(&mut self) {
        if !std::mem::take(&mut self.inventory_ui_refresh_pending) {
            return;
        }
        let equipped_items_changed = std::mem::take(&mut self.equipped_look_refresh_pending);
        if equipped_items_changed {
            self.sync_equipped_look();
            self.refresh_status_texture();
        }
        self.refresh_inventory_texture();
        self.refresh_storage_texture();
        self.refresh_npc_shop_texture();
        self.refresh_inventory_drop_prompt_texture();
        self.refresh_key_config_texture();
        self.refresh_npc_quest_marks();
        if std::mem::take(&mut self.inventory_drag_refresh_pending) {
            self.refresh_inventory_drag_texture();
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn inventory_pending_move_for_display(
        &self,
    ) -> Option<(openstory_protocol::InventoryType, u16, u16)> {
        self.inventory_pending_move
            .filter(|(_, _, _, started)| started.elapsed() < INVENTORY_MOVE_PREVIEW_TIMEOUT)
            .map(|(inventory_type, source, destination, _)| (inventory_type, source, destination))
    }
}

impl ClientApp {
    pub(super) fn refresh_inventory_drag_texture(&mut self) {
        self.inventory_drag_texture = None;
        let Some((inventory_type, slot, _)) = self.inventory_drag_source else {
            return;
        };
        let Some(item) = self.inventory.as_ref().and_then(|inventory| {
            inventory
                .items
                .iter()
                .find(|item| item.inventory_type == inventory_type && item.slot == slot)
        }) else {
            return;
        };
        let pixels = render_inventory_item_cell(
            item,
            &self.legacy_ui_archive,
            &self.item_archive,
            &self.character_archive,
            &mut self.inventory_icon_cache,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        match renderer.upload_bgra(
            INVENTORY_SLOT_CELL_WIDTH as u32,
            INVENTORY_SLOT_CELL_HEIGHT as u32,
            &pixels,
        ) {
            Ok(texture) => self.inventory_drag_texture = Some(texture),
            Err(error) => eprintln!("OpenStoryRS: could not create inventory drag icon: {error}"),
        }
    }

    pub(super) fn refresh_inventory_texture(&mut self) {
        self.refresh_quickslot_counts_if_changed();
        self.refresh_quest_helper_texture();
        if self.show_quest_log {
            self.refresh_quest_log_texture();
        }
        if self.show_equipment_inventory {
            self.refresh_equipment_inventory_texture();
        }
        self.update_inventory_hover();
        if !self.show_inventory {
            self.inventory_texture = None;
            self.refresh_inventory_tooltip_texture();
            return;
        }
        let Some(inventory) = self.inventory.as_ref() else {
            self.inventory_texture = None;
            self.refresh_inventory_tooltip_texture();
            return;
        };
        if self.pending_scroll_slot.is_some_and(|slot| {
            !inventory.items.iter().any(|item| {
                item.inventory_type == openstory_protocol::InventoryType::Use
                    && item.slot == slot
                    && item.item_id / 10_000 == 204
            })
        }) {
            self.pending_scroll_slot = None;
        }
        let maximum_row = inventory_slot_max(inventory, self.inventory_tab)
            .div_ceil(INVENTORY_SLOT_COLUMNS as u16)
            .saturating_sub(INVENTORY_SLOT_ROWS as u16);
        self.inventory_scroll_row = self.inventory_scroll_row.min(maximum_row);
        let pixels = render_inventory_panel(
            &self.font,
            inventory,
            self.inventory_tab,
            self.inventory_scroll_row,
            self.inventory_hover_slot,
            self.inventory_drag_source
                .map(|(inventory_type, slot, _)| (inventory_type, slot)),
            self.inventory_pending_move_for_display(),
            self.pending_scroll_slot,
            &self.legacy_ui_archive,
            &self.item_archive,
            &self.character_archive,
            &self.string_archive,
            &mut self.inventory_icon_cache,
            &mut self.item_name_cache,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        const WIDTH: u32 = INVENTORY_PANEL_WIDTH;
        const HEIGHT: u32 = INVENTORY_PANEL_HEIGHT;
        let result = if let Some(texture) = self.inventory_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer.upload_bgra(WIDTH, HEIGHT, &pixels).map(Some)
        };
        match result {
            Ok(Some(texture)) => self.inventory_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStory: could not update inventory panel: {error}"),
        }
        self.refresh_inventory_tooltip_texture();
    }

    pub(super) fn refresh_equipment_inventory_texture(&mut self) {
        if !self.show_equipment_inventory {
            return;
        }
        let empty_inventory = openstory_protocol::InventorySnapshot::default();
        let inventory = self.inventory.as_ref().unwrap_or(&empty_inventory);
        let active_scroll_item_id =
            self.inventory_drag_source
                .and_then(|(inventory_type, slot, _)| {
                    if inventory_type != openstory_protocol::InventoryType::Use {
                        return None;
                    }
                    let item = inventory
                        .items
                        .iter()
                        .find(|item| item.inventory_type == inventory_type && item.slot == slot)?;
                    (item.item_id / 10_000 == 204).then_some(item.item_id)
                });
        let pixels = ui_equipment::render(
            &self.font,
            inventory,
            self.active_character
                .as_ref()
                .map(|character| &character.look),
            self.equipment_tab,
            active_scroll_item_id,
            self.equipment_pet_slots(),
            &self.legacy_ui_archive,
            &self.item_archive,
            &self.character_archive,
            &self.string_archive,
            &mut self.inventory_icon_cache,
            &mut self.item_name_cache,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        const WIDTH: u32 = EQUIPMENT_INVENTORY_PANEL_WIDTH;
        const HEIGHT: u32 = EQUIPMENT_INVENTORY_PANEL_HEIGHT;
        let result = if let Some(texture) = self.equipment_inventory_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer.upload_bgra(WIDTH, HEIGHT, &pixels).map(Some)
        };
        match result {
            Ok(Some(texture)) => self.equipment_inventory_texture = Some(texture),
            Ok(None) => {}
            Err(error) => {
                eprintln!("OpenStory: could not update equipment inventory panel: {error}")
            }
        }
    }

    fn equipment_pet_slots(&self) -> ui_equipment::PetSlots {
        let summoned = self.active_character.as_ref().and_then(|character| {
            self.map_scene
                .as_ref()?
                .pets
                .values()
                .filter(|pet| pet.owner_id == character.id)
                .min_by_key(|pet| pet.slot)
                .map(|pet| (pet.item_id, pet.unique_id))
        });
        ui_equipment::PetSlots {
            summoned,
            hp: self.auto_hp_potion_id,
            mp: self.auto_mp_potion_id,
        }
    }

    pub(super) fn apply_auto_potion(
        &mut self,
        slot: openstory_protocol::AutoPotionSlot,
        item_id: i32,
    ) {
        let assigned = match slot {
            openstory_protocol::AutoPotionSlot::Hp => &mut self.auto_hp_potion_id,
            openstory_protocol::AutoPotionSlot::Mp => &mut self.auto_mp_potion_id,
        };
        if *assigned == item_id {
            return;
        }
        *assigned = item_id;
        self.refresh_equipment_pet_slots();
    }

    pub(super) fn refresh_equipment_pet_slots(&mut self) {
        if self.show_equipment_inventory
            && matches!(self.equipment_tab, EquipmentTab::Equip | EquipmentTab::Pet)
        {
            self.refresh_equipment_inventory_texture();
            self.update_inventory_hover();
            self.refresh_inventory_tooltip_texture();
        }
    }

    pub(super) fn toggle_equipment_pet_tab(&mut self) {
        if self.show_equipment_inventory && self.equipment_tab == EquipmentTab::Pet {
            self.toggle_equipment_inventory();
            return;
        }
        self.show_equipment_inventory = true;
        self.equipment_tab = EquipmentTab::Pet;
        self.raise_dialog_window(DialogWindow::EquipmentInventory);
        self.refresh_equipment_inventory_texture();
        self.update_inventory_hover();
        self.refresh_inventory_tooltip_texture();
    }

    pub(super) fn refresh_inventory_tooltip_texture(&mut self) {
        if self.cash_shop.is_some() {
            self.refresh_cash_shop_tooltip_texture();
            return;
        }
        if self.npc_shop.is_some() {
            self.refresh_npc_shop_tooltip_texture();
            return;
        }
        let comparison = if self.equipment_hover.is_none()
            && self.show_inventory
            && self.inventory_tab == openstory_protocol::InventoryType::Equip
        {
            self.inventory_hover_slot.and_then(|slot| {
                let inventory = self.inventory.as_ref()?;
                let item = inventory_display_item(
                    inventory,
                    self.inventory_tab,
                    slot,
                    self.inventory_pending_move_for_display(),
                )?;
                equipped_comparison_item(item, inventory)
            })
        } else {
            None
        };
        let item = if let Some(hover) = self
            .equipment_hover
            .filter(|_| self.show_equipment_inventory)
        {
            self.inventory
                .as_ref()
                .and_then(|inventory| hover.item(inventory, self.equipment_pet_slots()))
        } else if let Some(slot) = self.inventory_hover_slot.filter(|_| self.show_inventory) {
            self.inventory.as_ref().and_then(|inventory| {
                inventory_display_item(
                    inventory,
                    self.inventory_tab,
                    slot,
                    self.inventory_pending_move_for_display(),
                )
                .cloned()
            })
        } else {
            self.quickslot_hover_slot
                .filter(|_| self.current_field.is_some() && self.show_quickslots)
                .and_then(|slot| self.current_quickslot_binding(slot))
                .filter(|binding| matches!(binding.kind, 2 | 3))
                .and_then(|binding| {
                    self.inventory.as_ref().and_then(|inventory| {
                        inventory
                            .items
                            .iter()
                            .find(|item| item.slot > 0 && item.item_id == binding.action)
                            .cloned()
                    })
                })
        };
        self.update_item_tooltip_with_comparison(item, comparison);
    }

    pub(super) fn update_inventory_hover(&mut self) -> bool {
        let equipment_inventory_is_topmost = self.cursor.is_some_and(|cursor| {
            self.show_equipment_inventory
                && self.topmost_dialog_window_at(cursor) == Some(DialogWindow::EquipmentInventory)
        });
        let cursor = self
            .cursor
            .map(|cursor| self.dialog_window_input_cursor(cursor));
        let inventory_is_topmost = self.cursor.is_some_and(|cursor| {
            self.show_inventory
                && self.topmost_dialog_window_at(cursor) == Some(DialogWindow::Inventory)
        });
        let equipment_hover = if equipment_inventory_is_topmost {
            cursor.and_then(|cursor| {
                let local_x = cursor.x - EQUIPMENT_INVENTORY_PANEL_X;
                let local_y = cursor.y - EQUIPMENT_INVENTORY_PANEL_Y;
                self.inventory.as_ref().and_then(|inventory| {
                    self.equipment_tab.hover(
                        inventory,
                        self.equipment_pet_slots(),
                        local_x,
                        local_y,
                    )
                })
            })
        } else {
            None
        };
        let hovered_slot = if inventory_is_topmost
            && !equipment_inventory_is_topmost
            && self.inventory_drag_source.is_none()
        {
            cursor
                .and_then(|cursor| inventory_slot_at(cursor, self.inventory_scroll_row))
                .filter(|slot| {
                    self.inventory.as_ref().is_some_and(|inventory| {
                        inventory_display_item(
                            inventory,
                            self.inventory_tab,
                            *slot,
                            self.inventory_pending_move_for_display(),
                        )
                        .is_some()
                    })
                })
        } else {
            None
        };
        let quickslot_hovered_slot = self
            .cursor
            .filter(|_| {
                self.current_field.is_some()
                    && self.show_quickslots
                    && self.inventory_drag_source.is_none()
                    && self.dialog_window_drag.is_none()
                    && !self.skill_macro_editor.has_skill_drag()
                    && self.inventory_drop_prompt.is_none()
                    && !self.quit_confirmation
                    && !self.wedding_panel_open
                    && self.server_notice_popup.is_none()
                    && self.rps_game.is_none()
            })
            .filter(|cursor| self.topmost_dialog_window_at(*cursor).is_none())
            .and_then(|cursor| self.quickslot_at(cursor));
        let changed = self.inventory_hover_slot != hovered_slot
            || self.equipment_hover != equipment_hover
            || self.quickslot_hover_slot != quickslot_hovered_slot;
        self.inventory_hover_slot = hovered_slot;
        self.equipment_hover = equipment_hover;
        self.quickslot_hover_slot = quickslot_hovered_slot;
        changed
    }

    pub(super) fn refresh_storage_texture(&mut self) {
        if let (Some(storage), Some(inventory)) = (self.storage.as_mut(), self.inventory.as_ref()) {
            storage.storage_scroll = storage
                .storage_scroll
                .min(storage.state.items.len().saturating_sub(STORAGE_LEFT_ROWS));
            let inventory_count = inventory
                .items
                .iter()
                .filter(|item| item.inventory_type == storage.tab && item.slot > 0)
                .count();
            storage.inventory_scroll = storage
                .inventory_scroll
                .min(inventory_count.saturating_sub(STORAGE_RIGHT_ROWS));
        }
        let (Some(storage), Some(inventory)) = (self.storage.as_ref(), self.inventory.as_ref())
        else {
            self.storage_texture = None;
            return;
        };
        let pixels = render_storage_reference_panel(
            &self.font,
            &storage.state,
            inventory,
            storage.tab,
            storage.storage_scroll,
            storage.inventory_scroll,
            storage.selected_storage,
            storage.selected_inventory,
            &storage.meso_input,
            storage.meso_focused,
            storage.notice.as_deref(),
            &self.legacy_ui_archive,
            &self.npc_archive,
            &self.item_archive,
            &self.character_archive,
            &self.string_archive,
            &mut self.inventory_icon_cache,
            &mut self.item_name_cache,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        const WIDTH: u32 = STORAGE_PANEL_WIDTH as u32;
        const HEIGHT: u32 = STORAGE_PANEL_HEIGHT as u32;
        let result = if let Some(texture) = self.storage_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer.upload_bgra(WIDTH, HEIGHT, &pixels).map(Some)
        };
        match result {
            Ok(Some(texture)) => self.storage_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update storage panel: {error}"),
        }
    }

    pub(super) fn refresh_hired_merchant_room_texture(&mut self) {
        let Some(room) = self.hired_merchant_room.as_ref() else {
            self.hired_merchant_texture = None;
            return;
        };
        let pixels = render_hired_merchant_room_panel(
            &self.font,
            room,
            &self.legacy_ui_archive,
            &self.item_archive,
            &self.character_archive,
            &self.string_archive,
            &mut self.inventory_icon_cache,
            &mut self.item_name_cache,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.hired_merchant_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(
                    HIRED_MERCHANT_PANEL_WIDTH,
                    HIRED_MERCHANT_PANEL_HEIGHT,
                    &pixels,
                )
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.hired_merchant_texture = Some(texture),
            Ok(None) => {}
            Err(error) => {
                eprintln!("OpenStoryRS: could not render hired merchant room: {error}")
            }
        }
    }

    pub(super) fn send_hired_merchant_action(&mut self, packet: Vec<u8>, action: &str) -> bool {
        if !self.channel_connected {
            if let Some(room) = self.hired_merchant_room.as_mut() {
                room.notice = Some("Shop room is disconnected from the server".to_owned());
            }
            self.refresh_hired_merchant_room_texture();
            return false;
        }
        match self
            .session
            .as_ref()
            .map(|session| session.send_packet(packet))
        {
            Some(Ok(())) => true,
            Some(Err(error)) => {
                eprintln!("OpenStoryRS: could not {action}: {error}");
                self.channel_connected = false;
                if let Some(room) = self.hired_merchant_room.as_mut() {
                    room.notice = Some(format!("Could not {action}"));
                }
                self.refresh_hired_merchant_room_texture();
                false
            }
            None => false,
        }
    }

    pub(super) fn leave_hired_merchant_room(&mut self, maintenance: bool) {
        let packet = if maintenance {
            openstory_protocol::merchant_maintenance_off_packet()
        } else {
            openstory_protocol::merchant_exit_packet()
        };
        if self.send_hired_merchant_action(packet, "leave the merchant room") {
            self.hired_merchant_room = None;
            self.hired_merchant_texture = None;
        }
    }

    pub(super) fn send_hired_merchant_chat(&mut self) {
        let Some(message) = self
            .hired_merchant_room
            .as_ref()
            .map(|room| room.chat_input.trim().to_owned())
            .filter(|message| !message.is_empty())
        else {
            return;
        };
        let Some(packet) = openstory_protocol::merchant_chat_packet(&message) else {
            if let Some(room) = self.hired_merchant_room.as_mut() {
                room.notice = Some("Chat messages must be 1–60 characters".to_owned());
            }
            self.refresh_hired_merchant_room_texture();
            return;
        };
        if self.send_hired_merchant_action(packet, "send shop chat") {
            if let Some(room) = self.hired_merchant_room.as_mut() {
                room.chat_input.clear();
                room.chat_focused = true;
                room.notice = None;
            }
            self.refresh_hired_merchant_room_texture();
        }
    }

    pub(super) fn buy_selected_hired_merchant_item(&mut self) {
        let Some((index, quantity)) = self.hired_merchant_room.as_ref().and_then(|room| {
            let index = room.selected_item?;
            let item = room.state.items.get(index)?;
            (!item.sold_out && item.bundles > 0).then_some((index, item.bundles))
        }) else {
            return;
        };
        let Some(slot) = u8::try_from(index).ok() else {
            return;
        };
        let Some(packet) = openstory_protocol::merchant_buy_packet(slot, quantity) else {
            return;
        };
        self.send_hired_merchant_action(packet, "buy from the hired merchant");
    }

    pub(super) fn withdraw_hired_merchant_item(&mut self, index: usize) {
        let room_type = self.hired_merchant_room.as_ref().and_then(|room| {
            (room.state.my_slot == 0
                && room
                    .state
                    .items
                    .get(index)
                    .is_some_and(|item| item.item_id > 0 && !item.sold_out))
            .then_some(room.state.room_type)
        });
        let Some(room_type) = room_type else {
            return;
        };
        let Some(slot) = u8::try_from(index).ok() else {
            return;
        };
        let packet = if room_type == 4 {
            openstory_protocol::personal_shop_remove_item_packet(slot)
        } else {
            openstory_protocol::merchant_take_item_packet(slot)
        };
        let Some(packet) = packet else {
            return;
        };
        let action = if room_type == 4 {
            "withdraw a personal-shop listing"
        } else {
            "withdraw a merchant listing"
        };
        self.send_hired_merchant_action(packet, action);
    }

    pub(super) fn begin_hired_merchant_stock(
        &mut self,
        inventory_type: openstory_protocol::InventoryType,
        slot: u16,
    ) {
        let item = self.inventory.as_ref().and_then(|inventory| {
            inventory
                .items
                .iter()
                .find(|item| item.inventory_type == inventory_type && item.slot == slot)
        });
        let Some(item) = item else {
            return;
        };
        if item.inventory_type == openstory_protocol::InventoryType::Equipped
            || item.item_id <= 0
            || self
                .hired_merchant_room
                .as_ref()
                .is_none_or(|room| room.state.my_slot != 0)
        {
            return;
        }
        if let Some(reason) =
            merchant_stock_block_reason(item, &self.item_archive, &self.character_archive)
        {
            if let Some(room) = self.hired_merchant_room.as_mut() {
                room.stock_source = None;
                room.notice = Some(reason.to_owned());
            }
            self.refresh_hired_merchant_room_texture();
            return;
        }
        let source = HiredMerchantStockSource {
            inventory_type,
            slot,
            item_id: item.item_id,
            count: item.count.max(1),
        };
        if let Some(room) = self.hired_merchant_room.as_mut() {
            room.stock_source = Some(source);
            room.stock_price.clear();
            room.chat_focused = false;
            room.notice = None;
        }
        self.refresh_hired_merchant_room_texture();
    }

    pub(super) fn submit_hired_merchant_stock(&mut self) {
        if self
            .hired_merchant_room
            .as_ref()
            .is_none_or(|room| room.state.my_slot != 0)
        {
            if let Some(room) = self.hired_merchant_room.as_mut() {
                room.stock_source = None;
                room.notice = Some("Only the shop owner can list items".to_owned());
            }
            self.refresh_hired_merchant_room_texture();
            return;
        }
        let Some((source, price)) = self
            .hired_merchant_room
            .as_ref()
            .and_then(|room| Some((room.stock_source?, room.stock_price.parse::<i32>().ok()?)))
        else {
            if let Some(room) = self.hired_merchant_room.as_mut() {
                room.notice = Some("Enter a price between 1 and 999,999,999 mesos".to_owned());
            }
            self.refresh_hired_merchant_room_texture();
            return;
        };
        let current = self.inventory.as_ref().and_then(|inventory| {
            inventory.items.iter().find(|item| {
                item.inventory_type == source.inventory_type && item.slot == source.slot
            })
        });
        if let Some(reason) = current.and_then(|item| {
            (item.item_id == source.item_id && item.count >= source.count)
                .then(|| {
                    merchant_stock_block_reason(item, &self.item_archive, &self.character_archive)
                })
                .flatten()
        }) {
            if let Some(room) = self.hired_merchant_room.as_mut() {
                room.notice = Some(reason.to_owned());
                room.stock_source = None;
            }
            self.refresh_hired_merchant_room_texture();
            return;
        }
        if !current.is_some_and(|item| item.item_id == source.item_id && item.count >= source.count)
        {
            if let Some(room) = self.hired_merchant_room.as_mut() {
                room.notice = Some("That inventory item has changed".to_owned());
                room.stock_source = None;
            }
            self.refresh_hired_merchant_room_texture();
            return;
        }
        let (Ok(slot), Ok(per_bundle)) = (
            i16::try_from(source.slot),
            i16::try_from(source.count.max(1)),
        ) else {
            return;
        };
        let Some(packet) = openstory_protocol::merchant_add_item_packet(
            source.inventory_type,
            slot,
            1,
            per_bundle,
            price,
        ) else {
            if let Some(room) = self.hired_merchant_room.as_mut() {
                room.notice = Some("Enter a price between 1 and 999,999,999 mesos".to_owned());
            }
            self.refresh_hired_merchant_room_texture();
            return;
        };
        if self.send_hired_merchant_action(packet, "stock an item") {
            if let Some(room) = self.hired_merchant_room.as_mut() {
                room.stock_source = None;
                room.notice = Some("Item submitted to the server".to_owned());
            }
            self.refresh_hired_merchant_room_texture();
        }
    }

    pub(super) fn click_hired_merchant_room(&mut self, cursor: Point) {
        let x = cursor.x - HIRED_MERCHANT_PANEL_X;
        let y = cursor.y - HIRED_MERCHANT_PANEL_Y;
        if self
            .hired_merchant_room
            .as_ref()
            .is_some_and(|room| room.stock_source.is_some())
        {
            if (274.0..339.0).contains(&x) && (313.0..337.0).contains(&y) {
                self.submit_hired_merchant_stock();
            } else if (347.0..412.0).contains(&x) && (313.0..337.0).contains(&y) {
                if let Some(room) = self.hired_merchant_room.as_mut() {
                    room.stock_source = None;
                    room.notice = None;
                }
                self.refresh_hired_merchant_room_texture();
            }
            return;
        }
        if (536.0..575.0).contains(&x) && (365.0..382.0).contains(&y) {
            self.send_hired_merchant_chat();
            return;
        }
        if (329.0..531.0).contains(&x) && (365.0..383.0).contains(&y) {
            if let Some(room) = self.hired_merchant_room.as_mut() {
                room.chat_focused = true;
                room.notice = None;
            }
            self.refresh_hired_merchant_room_texture();
            return;
        }
        if let Some(row) = merchant_listing_row(x, y) {
            let target = self.hired_merchant_room.as_ref().and_then(|room| {
                let index = room.item_scroll.saturating_add(row);
                (index < room.state.items.len()).then_some(index)
            });
            if let Some(index) = target {
                let already_selected = self
                    .hired_merchant_room
                    .as_ref()
                    .is_some_and(|room| room.selected_item == Some(index));
                if let Some(room) = self.hired_merchant_room.as_mut() {
                    room.selected_item = Some(index);
                    room.chat_focused = false;
                    room.notice = None;
                }
                if already_selected
                    && self
                        .hired_merchant_room
                        .as_ref()
                        .is_some_and(|room| room.state.my_slot == 0)
                {
                    self.withdraw_hired_merchant_item(index);
                }
                self.refresh_hired_merchant_room_texture();
            }
            return;
        }
        let action = self.hired_merchant_room.as_ref().and_then(|room| {
            merchant_controls(&room.state)
                .into_iter()
                .find_map(|(action, _, bx, by)| {
                    let width = match action {
                        MerchantControl::Collect => 14.0,
                        MerchantControl::Arrange => 69.0,
                        _ => 80.0,
                    };
                    ((bx as f32..bx as f32 + width).contains(&x)
                        && (by as f32..by as f32 + 18.0).contains(&y))
                    .then_some(action)
                })
        });
        match action {
            Some(MerchantControl::Start) => {
                let hired = self
                    .hired_merchant_room
                    .as_ref()
                    .is_some_and(|room| room.state.room_type == 5);
                if self.send_hired_merchant_action(
                    openstory_protocol::merchant_buy_open_packet(),
                    "open the shop",
                ) && hired
                {
                    self.hired_merchant_room = None;
                    self.hired_merchant_texture = None;
                }
            }
            Some(MerchantControl::Arrange) => {
                self.send_hired_merchant_action(
                    openstory_protocol::merchant_organize_packet(),
                    "organize hired merchant stock",
                );
            }
            Some(MerchantControl::Collect) => {
                self.send_hired_merchant_action(
                    openstory_protocol::merchant_take_meso_packet(),
                    "collect shop mesos",
                );
            }
            Some(MerchantControl::Exit) => {
                let maintenance = self.hired_merchant_room.as_ref().is_some_and(|room| {
                    room.state.my_slot == 0 && room.state.room_type == 5 && !room.state.first_time
                });
                self.leave_hired_merchant_room(maintenance);
            }
            Some(MerchantControl::Close) => {
                if self.send_hired_merchant_action(
                    openstory_protocol::merchant_close_packet(),
                    "close the hired merchant",
                ) {
                    self.hired_merchant_room = None;
                    self.hired_merchant_texture = None;
                }
            }
            Some(MerchantControl::Buy) => self.buy_selected_hired_merchant_item(),
            None => return,
        }
        self.refresh_hired_merchant_room_texture();
    }

    pub(super) fn scroll_hired_merchant_room(&mut self, cursor: Point, amount: f32) {
        let local_x = cursor.x - HIRED_MERCHANT_PANEL_X;
        let local_y = cursor.y - HIRED_MERCHANT_PANEL_Y;
        if merchant_listing_row(local_x, local_y).is_none() {
            return;
        }
        let Some(room) = self.hired_merchant_room.as_mut() else {
            return;
        };
        let maximum = room.state.items.len().saturating_sub(5);
        room.item_scroll = room
            .item_scroll
            .saturating_add_signed(if amount > 0.0 { -1_isize } else { 1 })
            .min(maximum);
        self.refresh_hired_merchant_room_texture();
    }

    pub(super) fn refresh_character_info_texture(&mut self) {
        let Some(info) = self.character_info.as_ref() else {
            self.character_info_texture = None;
            return;
        };
        let look = self
            .map_scene
            .as_ref()
            .and_then(|scene| scene.remote_players.get(&info.info.character_id))
            .map(|player| &player.look)
            .or_else(|| {
                self.active_character
                    .as_ref()
                    .filter(|character| character.id == info.info.character_id)
                    .map(|character| &character.look)
            })
            .or(info.look.as_ref());
        let pixels = render_character_info_panel(
            &self.font,
            info,
            look,
            &self.legacy_ui_archive,
            &self.character_archive,
            &self.item_archive,
            &self.string_archive,
            &mut self.item_name_cache,
            &mut self.inventory_icon_cache,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.character_info_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(
                    CHARACTER_INFO_PANEL_WIDTH,
                    CHARACTER_INFO_PANEL_HEIGHT,
                    &pixels,
                )
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.character_info_texture = Some(texture),
            Ok(None) => {}
            Err(error) => {
                eprintln!("OpenStoryRS: could not render character info: {error}")
            }
        }
    }

    pub(super) fn click_character_info(&mut self, cursor: Point) {
        let x = cursor.x - CHARACTER_INFO_PANEL_X;
        let y = cursor.y - CHARACTER_INFO_PANEL_Y;
        if (255.0..267.0).contains(&x) && (5.0..17.0).contains(&y) {
            self.character_info = None;
            self.character_info_texture = None;
            return;
        }
        if (179.0..206.0).contains(&y) && (167.0..269.0).contains(&x) {
            let index = ((x - 167.0) / 34.0) as usize;
            if (x - 167.0) % 34.0 >= 30.0 {
                return;
            }
            if let Some(ui) = self.character_info.as_mut() {
                if index < ui.info.pets.len().min(3) {
                    ui.selected_pet = index;
                }
            }
            self.refresh_character_info_texture();
            return;
        }
        if (146.0..158.0).contains(&y) && (488.0..518.0).contains(&x) {
            if let Some(ui) = self.character_info.as_mut() {
                if x < 500.0 {
                    ui.wishlist_offset = ui.wishlist_offset.saturating_sub(1);
                } else if x >= 506.0 {
                    ui.wishlist_offset = ui
                        .wishlist_offset
                        .saturating_add(1)
                        .min(ui.info.wishlist.len().saturating_sub(3));
                }
            }
            self.refresh_character_info_texture();
            return;
        }
        let Some(name) = self.character_info.as_ref().map(|ui| ui.name.clone()) else {
            return;
        };
        if (382.0..394.0).contains(&y) {
            if (443.0..455.0).contains(&x) {
                self.request_character_fame(true);
            } else if (461.0..473.0).contains(&x) {
                self.request_character_fame(false);
            }
            return;
        }
        if (326.0..343.0).contains(&y) {
            if (306.0..365.0).contains(&x) {
                if let Ok(packet) = openstory_protocol::invite_to_party_packet(&name) {
                    self.send_party_packet(packet, "Party invitation sent.");
                }
                self.character_info = None;
                self.character_info_texture = None;
            } else if (372.0..431.0).contains(&x) {
                self.execute_trade_command(TradeCommand::Invite(name));
                self.character_info = None;
                self.character_info_texture = None;
            }
            return;
        }
        if (443.0..508.0).contains(&x) && (348.0..372.0).contains(&y) {
            if self.guild_info.is_none() {
                self.push_chat_line(
                    "[Guild] You must be in a guild to invite someone.".to_owned(),
                    [255, 105, 105],
                );
            } else if let Ok(packet) = openstory_protocol::invite_to_guild_packet(&name) {
                self.send_guild_packet(packet, "Guild invitation sent.");
            }
        }
    }

    pub(super) fn refresh_trade_texture(&mut self) {
        let Some(room) = self.trade_room.as_ref() else {
            self.trade_texture = None;
            return;
        };
        let pixels = render_trade_room_panel(
            &self.font,
            room,
            &self.legacy_ui_archive,
            &self.item_archive,
            &self.character_archive,
            &self.string_archive,
            &mut self.inventory_icon_cache,
            &mut self.item_name_cache,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.trade_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(TRADE_PANEL_WIDTH, TRADE_PANEL_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.trade_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not render player trade: {error}"),
        }
    }

    pub(super) fn send_trade_chat(&mut self) {
        let Some(message) = self
            .trade_room
            .as_ref()
            .map(|room| room.chat_input.trim().to_owned())
            .filter(|message| !message.is_empty())
        else {
            return;
        };
        let Some(packet) = openstory_protocol::trade_chat_packet(&message) else {
            if let Some(room) = self.trade_room.as_mut() {
                room.notice = Some("Trade chat is limited to 60 characters".to_owned());
            }
            self.refresh_trade_texture();
            return;
        };
        if self.send_trade_action(packet, "send trade chat") {
            if let Some(room) = self.trade_room.as_mut() {
                room.chat_input.clear();
                room.chat_focused = true;
                room.notice = None;
            }
        }
        self.refresh_trade_texture();
    }

    pub(super) fn submit_trade_meso(&mut self) {
        let input = self
            .trade_room
            .as_ref()
            .map(|room| room.meso_input.trim().to_owned())
            .unwrap_or_default();
        let meso = if input.is_empty() {
            0
        } else {
            match input.parse::<i32>() {
                Ok(value) => value,
                Err(_) => {
                    if let Some(room) = self.trade_room.as_mut() {
                        room.notice = Some("Enter a valid meso amount".to_owned());
                    }
                    self.refresh_trade_texture();
                    return;
                }
            }
        };
        let available = self.inventory.as_ref().map_or(0, |inventory| {
            inventory.meso.clamp(0, i64::from(i32::MAX)) as i32
        });
        if meso < 0 || meso > available {
            if let Some(room) = self.trade_room.as_mut() {
                room.notice = Some(format!("Offer 0–{available} mesos"));
            }
            self.refresh_trade_texture();
            return;
        }
        let Some(packet) = openstory_protocol::trade_set_meso_packet(meso) else {
            return;
        };
        if self.send_trade_action(packet, "set the meso offer")
            && let Some(room) = self.trade_room.as_mut()
        {
            room.meso_input.clear();
            room.meso_focused = false;
            room.notice = Some("Meso offer sent".to_owned());
        }
        self.refresh_trade_texture();
    }

    pub(super) fn begin_trade_stock(
        &mut self,
        inventory_type: openstory_protocol::InventoryType,
        inventory_slot: u16,
        trade_slot: u8,
    ) {
        if self
            .trade_room
            .as_ref()
            .is_none_or(|room| room.own_confirmed || room.partner.is_none())
        {
            return;
        }
        let item = self.inventory.as_ref().and_then(|inventory| {
            inventory
                .items
                .iter()
                .find(|item| item.inventory_type == inventory_type && item.slot == inventory_slot)
        });
        let Some(item) = item else {
            return;
        };
        if inventory_type == openstory_protocol::InventoryType::Equipped || item.item_id <= 0 {
            return;
        }
        let Ok(inventory_slot) = i16::try_from(inventory_slot) else {
            return;
        };
        let max_quantity = if inventory_type == openstory_protocol::InventoryType::Equip {
            1
        } else {
            i16::try_from(item.count.max(1))
                .unwrap_or(i16::MAX)
                .min(i16::MAX)
        };
        let source = TradeStockSource {
            inventory_type,
            inventory_slot: inventory_slot as u16,
            trade_slot,
            item_id: item.item_id,
            max_quantity,
        };
        if max_quantity == 1 {
            self.submit_trade_item(source, 1);
        } else if let Some(room) = self.trade_room.as_mut() {
            room.pending_item = Some(source);
            room.quantity_input.clear();
            room.chat_focused = false;
            room.meso_focused = false;
            room.notice = Some(format!("Enter quantity, up to {max_quantity}"));
            self.refresh_trade_texture();
        }
    }

    pub(super) fn submit_trade_item(&mut self, source: TradeStockSource, quantity: i16) {
        let current = self.inventory.as_ref().and_then(|inventory| {
            inventory.items.iter().find(|item| {
                item.inventory_type == source.inventory_type
                    && item.slot == source.inventory_slot
                    && item.item_id == source.item_id
            })
        });
        if !current.is_some_and(|item| i32::from(quantity) <= i32::from(item.count.max(1))) {
            if let Some(room) = self.trade_room.as_mut() {
                room.pending_item = None;
                room.notice = Some("That inventory item changed before it was offered".to_owned());
            }
            self.refresh_trade_texture();
            return;
        }
        let Some(server_slot) = source.trade_slot.checked_add(1) else {
            return;
        };
        let Some(packet) = openstory_protocol::trade_set_item_packet(
            source.inventory_type,
            source.inventory_slot as i16,
            quantity,
            server_slot,
        ) else {
            return;
        };
        if self.send_trade_action(packet, "offer the item")
            && let Some(room) = self.trade_room.as_mut()
        {
            room.pending_item = None;
            room.quantity_input.clear();
            room.notice = Some("Item offer sent".to_owned());
        }
        self.refresh_trade_texture();
    }

    pub(super) fn submit_trade_quantity(&mut self) {
        let Some((source, value)) = self.trade_room.as_ref().and_then(|room| {
            let source = room.pending_item?;
            let value = room.quantity_input.parse::<i16>().ok()?;
            Some((source, value))
        }) else {
            if let Some(room) = self.trade_room.as_mut() {
                room.notice = Some("Enter a valid quantity for the selected item".to_owned());
            }
            self.refresh_trade_texture();
            return;
        };
        if value <= 0 || value > source.max_quantity {
            if let Some(room) = self.trade_room.as_mut() {
                room.notice = Some(format!(
                    "Enter a quantity from 1 to {}",
                    source.max_quantity
                ));
            }
            self.refresh_trade_texture();
            return;
        }
        self.submit_trade_item(source, value);
    }

    pub(super) fn click_trade_room(&mut self, cursor: Point) {
        let local_x = cursor.x - TRADE_PANEL_X;
        let local_y = cursor.y - TRADE_PANEL_Y;
        if !(0.0..TRADE_PANEL_WIDTH as f32).contains(&local_x)
            || !(0.0..TRADE_PANEL_HEIGHT as f32).contains(&local_y)
        {
            return;
        }
        if self
            .trade_room
            .as_ref()
            .is_some_and(|room| room.pending_item.is_some())
        {
            if (284.0..349.0).contains(&local_x) && (309.0..333.0).contains(&local_y) {
                self.submit_trade_quantity();
            } else if (357.0..422.0).contains(&local_x) && (309.0..333.0).contains(&local_y) {
                if let Some(room) = self.trade_room.as_mut() {
                    room.pending_item = None;
                    room.quantity_input.clear();
                    room.notice = None;
                }
                self.refresh_trade_texture();
            }
            return;
        }
        if (451.0..511.0).contains(&local_x) && (290.0..309.0).contains(&local_y) {
            if self.send_trade_action(openstory_protocol::trade_exit_packet(), "cancel the trade") {
                self.trade_room = None;
                self.trade_texture = None;
            }
            return;
        }
        if (373.0..568.0).contains(&local_x) && (264.0..282.0).contains(&local_y) {
            if let Some(room) = self.trade_room.as_mut() {
                room.chat_focused = true;
                room.meso_focused = false;
                room.notice = None;
            }
            self.refresh_trade_texture();
            return;
        }
        if (247.0..261.0).contains(&local_x) && (280.0..294.0).contains(&local_y) {
            self.submit_trade_meso();
            return;
        }
        if (247.0..361.0).contains(&local_x) && (264.0..294.0).contains(&local_y) {
            if let Some(room) = self.trade_room.as_mut() {
                room.meso_focused = true;
                room.chat_focused = false;
                room.notice = None;
            }
            self.refresh_trade_texture();
            return;
        }
        if let Some(slot) = trade_own_grid_slot(local_x, local_y) {
            if let Some(room) = self.trade_room.as_mut() {
                if room
                    .own_items
                    .get(slot as usize)
                    .is_some_and(Option::is_some)
                {
                    room.notice =
                        Some("An offered item cannot be removed. Cancel to reclaim it.".to_owned());
                } else {
                    room.notice = None;
                }
                room.chat_focused = false;
            }
            self.refresh_trade_texture();
            return;
        }
        if (385.0..445.0).contains(&local_x) && (290.0..309.0).contains(&local_y) {
            let eligible = self
                .trade_room
                .as_ref()
                .is_some_and(|room| room.partner.is_some() && !room.own_confirmed);
            if !eligible {
                if let Some(room) = self.trade_room.as_mut() {
                    room.notice = Some(if room.partner.is_none() {
                        "Waiting for the other player to join".to_owned()
                    } else {
                        "You already confirmed this trade".to_owned()
                    });
                }
            } else if self.send_trade_action(
                openstory_protocol::trade_confirm_packet(),
                "confirm the trade",
            ) && let Some(room) = self.trade_room.as_mut()
            {
                room.own_confirmed = true;
                room.notice = Some("Waiting for the other player to confirm…".to_owned());
            }
            self.refresh_trade_texture();
        }
    }

    pub(super) fn send_storage_action(&mut self, packet: Vec<u8>, action: &str) -> bool {
        if !self.channel_connected {
            if let Some(storage) = self.storage.as_mut() {
                storage.notice = Some("Storage is disconnected from the server".to_owned());
            }
            self.refresh_storage_texture();
            return false;
        }
        let result = self
            .session
            .as_ref()
            .map(|session| session.send_packet(packet));
        match result {
            Some(Ok(())) => true,
            Some(Err(error)) => {
                eprintln!("OpenStoryRS: could not {action}: {error}");
                self.channel_connected = false;
                if let Some(storage) = self.storage.as_mut() {
                    storage.notice = Some(format!("Could not {action}"));
                }
                self.refresh_storage_texture();
                false
            }
            None => false,
        }
    }

    pub(super) fn open_storage_quantity_prompt(
        &mut self,
        action: InventoryQuantityAction,
        item: &openstory_protocol::InventoryItem,
        initial_quantity: u16,
    ) {
        let max_quantity = item.count.max(1);
        self.inventory_drop_prompt = Some(InventoryDropPrompt {
            action,
            inventory_type: item.inventory_type,
            slot: item.slot,
            item_id: item.item_id,
            max_quantity,
            quantity: QuantityInput::new(Some(initial_quantity.clamp(1, max_quantity))),
            step: InventoryDropStep::Quantity,
            notice: None,
        });
        self.refresh_inventory_drop_prompt_texture();
    }

    pub(super) fn request_storage_store(
        &mut self,
        item: &openstory_protocol::InventoryItem,
        quantity: u16,
    ) {
        let current_item = self.inventory.as_ref().and_then(|inventory| {
            inventory.items.iter().find(|current| {
                current.inventory_type == item.inventory_type
                    && current.slot == item.slot
                    && current.item_id == item.item_id
            })
        });
        if !current_item.is_some_and(|current| quantity > 0 && current.count >= quantity) {
            if let Some(storage) = self.storage.as_mut() {
                storage.notice = Some("That inventory stack has changed".to_owned());
            }
            self.refresh_storage_texture();
            return;
        }
        let Some(packet) =
            openstory_protocol::storage_store_packet(item.slot, item.item_id, quantity)
        else {
            if let Some(storage) = self.storage.as_mut() {
                storage.notice = Some("That item cannot be stored".to_owned());
            }
            self.refresh_storage_texture();
            return;
        };
        self.send_storage_action(packet, "store an item");
    }

    pub(super) fn request_storage_take_out(
        &mut self,
        item: &openstory_protocol::InventoryItem,
        keep_quantity: u16,
    ) {
        let current_item = self.storage.as_ref().and_then(|storage| {
            storage.state.items.iter().find(|current| {
                current.inventory_type == item.inventory_type
                    && current.slot == item.slot
                    && current.item_id == item.item_id
            })
        });
        let Some(current_item) = current_item
            .cloned()
            .filter(|current| keep_quantity > 0 && keep_quantity <= current.count.max(1))
        else {
            if let Some(storage) = self.storage.as_mut() {
                storage.notice = Some("That stored stack has changed".to_owned());
            }
            self.refresh_storage_texture();
            return;
        };
        let Some(slot) = u8::try_from(current_item.slot).ok() else {
            return;
        };
        let Some(packet) =
            openstory_protocol::storage_take_out_packet(current_item.inventory_type, slot)
        else {
            return;
        };
        let full_quantity = current_item.count.max(1);
        let needs_restore = keep_quantity < full_quantity;
        if needs_restore {
            if self.pending_storage_restores.iter().any(|pending| {
                pending.inventory_type == current_item.inventory_type
                    && pending.item_id == current_item.item_id
            }) {
                if let Some(storage) = self.storage.as_mut() {
                    storage.notice = Some(
                        "A partial retrieval of this item is still being processed".to_owned(),
                    );
                }
                self.refresh_storage_texture();
                return;
            }
            let before_counts = self
                .inventory
                .as_ref()
                .map(|inventory| {
                    inventory
                        .items
                        .iter()
                        .filter(|entry| {
                            entry.inventory_type == current_item.inventory_type
                                && entry.item_id == current_item.item_id
                        })
                        .map(|entry| (entry.slot, entry.count))
                        .collect()
                })
                .unwrap_or_default();
            self.pending_storage_restores.push(PendingStorageRestore {
                inventory_type: current_item.inventory_type,
                item_id: current_item.item_id,
                full_quantity,
                keep_quantity,
                before_counts,
            });
        }
        if !self.send_storage_action(packet, "take an item from storage") && needs_restore {
            self.pending_storage_restores.pop();
        }
    }

    pub(super) fn resolve_pending_storage_restores(&mut self) {
        let Some(inventory) = self.inventory.as_ref() else {
            return;
        };
        let mut pending_restores = std::mem::take(&mut self.pending_storage_restores);
        let mut waiting = Vec::new();
        let mut excess_items = Vec::new();
        for pending in pending_restores.drain(..) {
            let arrivals = inventory
                .items
                .iter()
                .filter(|item| {
                    item.inventory_type == pending.inventory_type && item.item_id == pending.item_id
                })
                .filter_map(|item| {
                    let before = pending.before_counts.get(&item.slot).copied().unwrap_or(0);
                    let added = item.count.saturating_sub(before);
                    (added > 0).then_some((item.slot, added))
                })
                .collect::<Vec<_>>();
            let arrived = arrivals
                .iter()
                .map(|(_, count)| u32::from(*count))
                .sum::<u32>();
            if arrived < u32::from(pending.full_quantity) {
                waiting.push(pending);
                continue;
            }

            let mut remaining_to_keep = pending.keep_quantity;
            for (slot, added) in arrivals {
                let keep_here = remaining_to_keep.min(added);
                remaining_to_keep -= keep_here;
                let excess = added.saturating_sub(keep_here);
                if excess > 0 {
                    excess_items.push((slot, pending.item_id, excess));
                }
            }
        }
        self.pending_storage_restores = waiting;
        for (slot, item_id, quantity) in excess_items {
            if let Some(packet) = openstory_protocol::storage_store_packet(slot, item_id, quantity)
            {
                self.send_storage_action(packet, "return excess items to storage");
            }
        }
        if self.pending_storage_restores.is_empty()
            && self
                .storage
                .as_ref()
                .is_some_and(|storage| storage.close_when_transfer_finishes)
        {
            self.close_storage();
        }
    }

    pub(super) fn close_storage(&mut self) {
        if !self.pending_storage_restores.is_empty() {
            if let Some(storage) = self.storage.as_mut() {
                storage.close_when_transfer_finishes = true;
                storage.notice = Some("Finishing the partial withdrawal before closing".to_owned());
            }
            self.refresh_storage_texture();
            return;
        }
        if self.storage.is_some() {
            self.send_storage_action(openstory_protocol::storage_close_packet(), "close storage");
        }
        self.storage = None;
        self.storage_texture = None;
    }

    pub(super) fn click_storage(&mut self, cursor: Point) {
        let x = cursor.x - 182.0;
        let y = cursor.y - 105.0;
        if !(0.0..STORAGE_PANEL_WIDTH as f32).contains(&x)
            || !(0.0..STORAGE_PANEL_HEIGHT as f32).contains(&y)
        {
            return;
        }
        if let Some(storage) = self.storage.as_mut() {
            storage.meso_focused = false;
        }
        if (383.0..453.0).contains(&x) && (56.0..74.0).contains(&y) {
            self.close_storage();
            return;
        }
        if (240.0..449.0).contains(&x) && (91.0..119.0).contains(&y) {
            let tabs = [
                openstory_protocol::InventoryType::Equip,
                openstory_protocol::InventoryType::Use,
                openstory_protocol::InventoryType::Setup,
                openstory_protocol::InventoryType::Etc,
                openstory_protocol::InventoryType::Cash,
            ];
            if let Some(index) =
                (0..5).find(|index| ui_foundation::storage_tab(*index).contains(x, y))
            {
                if let Some(storage) = self.storage.as_mut() {
                    storage.tab = tabs[index];
                    storage.inventory_scroll = 0;
                    storage.selected_inventory = None;
                }
                self.refresh_storage_texture();
            }
            return;
        }
        if (263.0..367.0).contains(&x) && (52.0..69.0).contains(&y) {
            if let Some(storage) = self.storage.as_mut() {
                storage.meso_focused = true;
                storage.notice = None;
            }
            self.refresh_storage_texture();
            return;
        }
        if (293.0..311.0).contains(&y) {
            if let Some(amount) = self
                .storage
                .as_ref()
                .and_then(|storage| storage.meso_input.parse::<i32>().ok())
                .filter(|amount| *amount > 0)
            {
                if (3.0..39.0).contains(&x) {
                    self.send_storage_action(
                        openstory_protocol::storage_meso_packet(-amount),
                        "deposit mesos",
                    );
                } else if (424.0..460.0).contains(&x) {
                    self.send_storage_action(
                        openstory_protocol::storage_meso_packet(amount),
                        "withdraw mesos",
                    );
                }
            }
            return;
        }
        if let Some(storage) = self.storage.as_mut() {
            let inventory_count = self.inventory.as_ref().map_or(0, |inventory| {
                inventory
                    .items
                    .iter()
                    .filter(|item| item.inventory_type == storage.tab && item.slot > 0)
                    .count()
            });
            if let Some(position) = storage_scrollbar_position(
                x,
                y,
                false,
                storage.state.items.len(),
                storage.storage_scroll,
            ) {
                storage.storage_scroll = position;
                self.refresh_storage_texture();
                return;
            }
            if let Some(position) =
                storage_scrollbar_position(x, y, true, inventory_count, storage.inventory_scroll)
            {
                storage.inventory_scroll = position;
                self.refresh_storage_texture();
                return;
            }
        }
        if let Some(row) = storage_row_at(x, y, false) {
            if let Some(storage) = self.storage.as_mut() {
                storage.selected_storage = storage
                    .state
                    .items
                    .get(storage.storage_scroll + row)
                    .map(|item| (item.inventory_type, item.slot, item.item_id));
            }
            self.refresh_storage_texture();
            return;
        }
        if let Some(row) = storage_row_at(x, y, true) {
            if let Some(storage) = self.storage.as_mut() {
                storage.selected_inventory = self
                    .inventory
                    .as_ref()
                    .and_then(|inventory| {
                        inventory
                            .items
                            .iter()
                            .filter(|item| item.inventory_type == storage.tab && item.slot > 0)
                            .nth(storage.inventory_scroll + row)
                    })
                    .map(|item| (item.inventory_type, item.slot, item.item_id));
            }
            self.refresh_storage_texture();
            return;
        }
        if (151.0..221.0).contains(&x) && (12.0..30.0).contains(&y) {
            let item = self
                .storage
                .as_ref()
                .and_then(|storage| {
                    let key = storage.selected_storage?;
                    storage
                        .state
                        .items
                        .iter()
                        .find(|item| (item.inventory_type, item.slot, item.item_id) == key)
                })
                .cloned();
            if let Some(item) = item {
                if item.inventory_type != openstory_protocol::InventoryType::Equip && item.count > 1
                {
                    self.open_storage_quantity_prompt(
                        InventoryQuantityAction::StorageTake,
                        &item,
                        item.count,
                    );
                } else {
                    self.request_storage_take_out(&item, item.count.max(1));
                }
            }
            return;
        }
        if (383.0..453.0).contains(&x) && (12.0..30.0).contains(&y) {
            let item = self
                .storage
                .as_ref()
                .and_then(|storage| {
                    let key = storage.selected_inventory?;
                    self.inventory
                        .as_ref()?
                        .items
                        .iter()
                        .find(|item| (item.inventory_type, item.slot, item.item_id) == key)
                })
                .cloned();
            if let Some(item) = item {
                if item.inventory_type != openstory_protocol::InventoryType::Equip && item.count > 1
                {
                    self.open_storage_quantity_prompt(
                        InventoryQuantityAction::StorageStore,
                        &item,
                        1,
                    );
                } else {
                    self.request_storage_store(&item, item.count.max(1));
                }
            }
        }
    }

    pub(super) fn scroll_storage(&mut self, cursor: Point, amount: f32) {
        let x = cursor.x - 182.0;
        let y = cursor.y - 105.0;
        let Some(storage) = self.storage.as_mut() else {
            return;
        };
        let step = if amount > 0.0 { -1_isize } else { 1 };
        if (4.0..225.0).contains(&x) && (86.0..286.0).contains(&y) {
            storage.storage_scroll = storage
                .storage_scroll
                .saturating_add_signed(step)
                .min(storage.state.items.len().saturating_sub(STORAGE_LEFT_ROWS));
        } else if (234.0..455.0).contains(&x) && (126.0..286.0).contains(&y) {
            let count = self.inventory.as_ref().map_or(0, |inventory| {
                inventory
                    .items
                    .iter()
                    .filter(|item| item.inventory_type == storage.tab && item.slot > 0)
                    .count()
            });
            storage.inventory_scroll = storage
                .inventory_scroll
                .saturating_add_signed(step)
                .min(count.saturating_sub(STORAGE_RIGHT_ROWS));
        } else {
            return;
        }
        self.refresh_storage_texture();
    }

    pub(super) fn apply_storage_event(&mut self, event: openstory_protocol::StorageEvent) {
        match event {
            openstory_protocol::StorageEvent::Open(state) => {
                self.show_inventory = false;
                self.inventory_hover_slot = None;
                self.show_skill_book = false;
                self.show_quest_log = false;
                self.storage = Some(StorageUi {
                    state,
                    tab: openstory_protocol::InventoryType::Equip,
                    storage_scroll: 0,
                    inventory_scroll: 0,
                    selected_storage: None,
                    selected_inventory: None,
                    meso_input: "1000".to_owned(),
                    meso_focused: false,
                    notice: None,
                    close_when_transfer_finishes: false,
                });
                self.storage_texture = None;
                self.refresh_storage_texture();
            }
            openstory_protocol::StorageEvent::ItemsUpdated {
                inventory_type,
                slot_capacity,
                items,
            } => {
                if let Some(storage) = self.storage.as_mut() {
                    storage.state.slot_capacity = slot_capacity;
                    storage
                        .state
                        .items
                        .retain(|item| item.inventory_type != inventory_type);
                    storage.state.items.extend(items);
                    storage.selected_storage = None;
                    storage.notice = Some("Storage updated".to_owned());
                }
                self.refresh_storage_texture();
            }
            openstory_protocol::StorageEvent::Arranged {
                slot_capacity,
                items,
            } => {
                if let Some(storage) = self.storage.as_mut() {
                    storage.state.slot_capacity = slot_capacity;
                    storage.state.items = items;
                    storage.selected_storage = None;
                }
                self.refresh_storage_texture();
            }
            openstory_protocol::StorageEvent::MesoUpdated {
                slot_capacity,
                meso,
            } => {
                if let Some(storage) = self.storage.as_mut() {
                    storage.state.slot_capacity = slot_capacity;
                    storage.state.meso = meso;
                    storage.notice = Some("Meso balance updated".to_owned());
                }
                self.refresh_storage_texture();
            }
            openstory_protocol::StorageEvent::Error(code) => {
                if !self.pending_storage_restores.is_empty() {
                    self.pending_storage_restores.remove(0);
                }
                let message = match code {
                    0x0a => "Your inventory is full.",
                    0x0b => "You do not have enough mesos.",
                    0x0c => "You already have that one-of-a-kind item.",
                    0x11 => "Storage is full.",
                    _ => "Storage request was rejected.",
                };
                if let Some(storage) = self.storage.as_mut() {
                    storage.notice = Some(message.to_owned());
                }
                self.push_chat_line(format!("[Storage] {message}"), [255, 190, 115]);
                self.refresh_storage_texture();
                if self.pending_storage_restores.is_empty()
                    && self
                        .storage
                        .as_ref()
                        .is_some_and(|storage| storage.close_when_transfer_finishes)
                {
                    self.close_storage();
                }
            }
        }
    }
}
