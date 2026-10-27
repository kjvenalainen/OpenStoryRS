use super::*;

impl ClientApp {
    pub(super) fn refresh_pet_equipment_texture(&mut self) {
        if !self.show_pet_equipment {
            return;
        }
        let empty_inventory = openstory_protocol::InventorySnapshot::default();
        let inventory = self.inventory.as_ref().unwrap_or(&empty_inventory);
        self.pet_equipment_scroll_row = self
            .pet_equipment_scroll_row
            .min(pet_equipment_items(inventory).len().saturating_sub(5));
        let pixels = render_pet_equipment_panel(
            &self.font,
            inventory,
            &self.item_archive,
            &self.character_archive,
            &self.string_archive,
            &mut self.inventory_icon_cache,
            &mut self.item_name_cache,
            self.pet_equipment_scroll_row,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        const WIDTH: u32 = 390;
        const HEIGHT: u32 = 480;
        let result = if let Some(texture) = self.pet_equipment_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer.upload_bgra(WIDTH, HEIGHT, &pixels).map(Some)
        };
        match result {
            Ok(Some(texture)) => self.pet_equipment_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStory: could not update pet equipment panel: {error}"),
        }
    }

    pub(super) fn scroll_pet_equipment_by(&mut self, amount: isize) {
        if amount == 0 {
            return;
        }
        let gear_count = self
            .inventory
            .as_ref()
            .map_or(0, |inventory| pet_equipment_items(inventory).len());
        let maximum = gear_count.saturating_sub(5);
        let next = self
            .pet_equipment_scroll_row
            .saturating_add_signed(amount)
            .min(maximum);
        if next != self.pet_equipment_scroll_row {
            self.pet_equipment_scroll_row = next;
            self.refresh_pet_equipment_texture();
        }
    }

    pub(super) fn refresh_cash_shop_texture(&mut self) {
        let Some(state) = self.cash_shop.as_ref() else {
            self.cash_shop_texture = None;
            self.cash_shop_hit_regions.clear();
            return;
        };
        let scene = match cash_shop_ui::render(
            &self.ui_archive,
            &self.font,
            state,
            &self.cash_products,
            &self.item_archive,
            &self.character_archive,
            &self.string_archive,
            &mut self.inventory_icon_cache,
            &mut self.item_name_cache,
        ) {
            Ok(scene) => scene,
            Err(error) => {
                eprintln!("OpenStoryRS: could not draw Cash Shop: {error}");
                return;
            }
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        const WIDTH: u32 = 1024;
        const HEIGHT: u32 = 768;
        let result = if let Some(texture) = self.cash_shop_texture.as_ref() {
            renderer.update_bgra(texture, &scene.pixels).map(|()| None)
        } else {
            renderer.upload_bgra(WIDTH, HEIGHT, &scene.pixels).map(Some)
        };
        match result {
            Ok(Some(texture)) => {
                self.cash_shop_texture = Some(texture);
                self.cash_shop_hit_regions = scene.hit_regions;
            }
            Ok(None) => self.cash_shop_hit_regions = scene.hit_regions,
            Err(error) => eprintln!("OpenStoryRS: could not update Cash Shop: {error}"),
        }
    }

    pub(super) fn refresh_owl_texture(&mut self) {
        let Some(owl) = self.owl_search.as_ref() else {
            self.owl_texture = None;
            return;
        };
        let pixels = render_owl_panel(
            &self.font,
            owl,
            &self.string_archive,
            &mut self.item_name_cache,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.owl_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(OWL_PANEL_WIDTH, OWL_PANEL_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.owl_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update Owl panel: {error}"),
        }
    }

    pub(super) fn open_owl_search(&mut self, slot: i16, item_id: i32) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        if let Err(error) = session.send_packet(openstory_protocol::owl_top_items_packet()) {
            eprintln!("OpenStoryRS: could not request Owl top items: {error}");
            self.channel_connected = false;
            return;
        }
        self.owl_search = Some(OwlSearchUi {
            slot,
            item_id,
            query: String::new(),
            top_item_ids: Vec::new(),
            top_items_loaded: false,
            results: Vec::new(),
            result_item_id: None,
            selected_result: None,
            result_offset: 0,
            notice: Some("Enter an item name or numeric item ID.".to_owned()),
        });
        self.gameplay_input = GameplayInput::default();
        self.refresh_owl_texture();
    }

    pub(super) fn submit_owl_search(&mut self) {
        let Some((query, slot, owl_item_id)) = self
            .owl_search
            .as_ref()
            .map(|owl| (owl.query.trim().to_owned(), owl.slot, owl.item_id))
        else {
            return;
        };
        if query.is_empty() {
            if let Some(owl) = self.owl_search.as_mut() {
                owl.notice = Some("Enter an item name or numeric item ID.".to_owned());
            }
            self.refresh_owl_texture();
            return;
        }
        let target = if let Ok(item_id) = query.parse::<i32>() {
            (item_id > 0).then_some(item_id)
        } else {
            if self.owl_name_index.is_none() {
                self.owl_name_index = Some(build_owl_item_name_index(&self.string_archive));
            }
            let needle = query.to_lowercase();
            let index = self.owl_name_index.as_deref().unwrap_or_default();
            index
                .iter()
                .find(|(name, _)| name == &needle)
                .or_else(|| index.iter().find(|(name, _)| name.contains(&needle)))
                .map(|(_, item_id)| *item_id)
        };
        let Some(target) = target else {
            if let Some(owl) = self.owl_search.as_mut() {
                owl.notice = Some(format!("No item matches {query:?}."));
            }
            self.refresh_owl_texture();
            return;
        };
        let Some(packet) = openstory_protocol::owl_search_packet(slot, owl_item_id, target) else {
            if let Some(owl) = self.owl_search.as_mut() {
                owl.notice = Some("The Owl item or search item is invalid.".to_owned());
            }
            self.refresh_owl_texture();
            return;
        };
        let Some(session) = self.session.as_ref() else {
            return;
        };
        match session.send_packet(packet) {
            Ok(()) => {
                if let Some(owl) = self.owl_search.as_mut() {
                    owl.results.clear();
                    owl.result_item_id = None;
                    owl.selected_result = None;
                    owl.result_offset = 0;
                    owl.notice = Some(format!("Searching for {query}…"));
                }
            }
            Err(error) => {
                eprintln!("OpenStoryRS: could not search with Owl: {error}");
                self.channel_connected = false;
                if let Some(owl) = self.owl_search.as_mut() {
                    owl.notice = Some("Search could not be sent.".to_owned());
                }
            }
        }
        self.refresh_owl_texture();
    }

    pub(super) fn warp_to_owl_result(&mut self) {
        let Some((owner_id, map_id)) = self
            .owl_search
            .as_ref()
            .and_then(|owl| owl.selected_result.and_then(|index| owl.results.get(index)))
            .map(|result| (result.owner_id, result.map_id))
        else {
            return;
        };
        let Some(packet) = openstory_protocol::owl_warp_packet(owner_id, map_id) else {
            if let Some(owl) = self.owl_search.as_mut() {
                owl.notice = Some("The selected shop has invalid location data.".to_owned());
            }
            self.refresh_owl_texture();
            return;
        };
        if let Some(session) = self.session.as_ref() {
            match session.send_packet(packet) {
                Ok(()) => {
                    self.owl_search = None;
                    self.owl_texture = None;
                }
                Err(error) => {
                    eprintln!("OpenStoryRS: could not warp to Owl shop: {error}");
                    self.channel_connected = false;
                }
            }
        }
    }

    pub(super) fn click_owl_panel(&mut self, cursor: Point) -> bool {
        if self.owl_search.is_none() {
            return false;
        }
        let local_x = cursor.x - OWL_PANEL_X;
        let local_y = cursor.y - OWL_PANEL_Y;
        if local_x < 0.0
            || local_y < 0.0
            || local_x >= OWL_PANEL_WIDTH as f32
            || local_y >= OWL_PANEL_HEIGHT as f32
        {
            return true;
        }
        if local_y < 36.0 && local_x >= OWL_PANEL_WIDTH as f32 - 48.0 {
            self.owl_search = None;
            self.owl_texture = None;
            return true;
        }
        let results_mode = self
            .owl_search
            .as_ref()
            .is_some_and(|owl| owl.result_item_id.is_some());
        if results_mode {
            if (410.0..442.0).contains(&local_y) {
                if local_x < 150.0 {
                    if let Some(owl) = self.owl_search.as_mut() {
                        owl.result_item_id = None;
                        owl.results.clear();
                        owl.selected_result = None;
                        owl.notice = None;
                    }
                    self.refresh_owl_texture();
                } else if (146.0..222.0).contains(&local_x) {
                    if let Some(owl) = self.owl_search.as_mut() {
                        owl.result_offset = owl.result_offset.saturating_sub(8);
                        owl.selected_result = Some(owl.result_offset);
                    }
                    self.refresh_owl_texture();
                } else if (232.0..306.0).contains(&local_x) {
                    if let Some(owl) = self.owl_search.as_mut() {
                        let maximum = owl.results.len().saturating_sub(8);
                        owl.result_offset = owl.result_offset.saturating_add(8).min(maximum);
                        owl.selected_result = Some(owl.result_offset);
                    }
                    self.refresh_owl_texture();
                } else if local_x >= 300.0 {
                    self.warp_to_owl_result();
                }
            } else if (78.0..390.0).contains(&local_y) {
                let row = ((local_y - 78.0) / 39.0) as usize
                    + self
                        .owl_search
                        .as_ref()
                        .map_or(0, |state| state.result_offset);
                if self.owl_search.as_ref().is_some_and(|state| {
                    row < state.results.len() && row < state.result_offset.saturating_add(8)
                }) {
                    if let Some(owl) = self.owl_search.as_mut() {
                        owl.selected_result = Some(row);
                        owl.notice = None;
                    }
                    self.refresh_owl_texture();
                }
            }
        } else if (78.0..110.0).contains(&local_y) && local_x >= 340.0 {
            self.submit_owl_search();
        } else if (151.0..391.0).contains(&local_y) {
            let row = ((local_y - 151.0) / 24.0) as usize;
            let selected_id = self
                .owl_search
                .as_ref()
                .and_then(|state| state.top_item_ids.get(row).copied());
            if let Some(item_id) = selected_id {
                let name = item_name(item_id, &self.string_archive, &mut self.item_name_cache);
                if let Some(owl) = self.owl_search.as_mut() {
                    owl.query = name;
                    owl.notice = None;
                }
                self.refresh_owl_texture();
            }
        }
        true
    }

    pub(super) fn handle_owl_key(&mut self, key: PhysicalKey) {
        match key {
            PhysicalKey::Code(KeyCode::Escape) => {
                self.owl_search = None;
                self.owl_texture = None;
            }
            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                if self
                    .owl_search
                    .as_ref()
                    .is_some_and(|owl| owl.result_item_id.is_some())
                {
                    self.warp_to_owl_result();
                } else {
                    self.submit_owl_search();
                }
            }
            PhysicalKey::Code(KeyCode::Backspace) => {
                if let Some(owl) = self.owl_search.as_mut()
                    && owl.result_item_id.is_none()
                {
                    owl.query.pop();
                    owl.notice = None;
                }
                self.refresh_owl_texture();
            }
            PhysicalKey::Code(KeyCode::ArrowUp | KeyCode::ArrowDown) => {
                if let Some(owl) = self.owl_search.as_mut()
                    && owl.result_item_id.is_some()
                    && !owl.results.is_empty()
                {
                    let max = owl.results.len() - 1;
                    let current = owl.selected_result.unwrap_or(0).min(max);
                    let selected = match key {
                        PhysicalKey::Code(KeyCode::ArrowUp) => current.saturating_sub(1),
                        _ => (current + 1).min(max),
                    };
                    owl.selected_result = Some(selected);
                    if selected < owl.result_offset {
                        owl.result_offset = selected;
                    } else if selected >= owl.result_offset.saturating_add(8) {
                        owl.result_offset = selected.saturating_sub(7);
                    }
                }
                self.refresh_owl_texture();
            }
            _ => {}
        }
    }

    pub(super) fn handle_owl_search_packet(&mut self, packet: &[u8]) {
        match openstory_protocol::parse_owl_search_event(packet) {
            Ok(Some(openstory_protocol::OwlSearchEvent::TopItems(item_ids))) => {
                if let Some(owl) = self.owl_search.as_mut() {
                    owl.top_item_ids = item_ids;
                    owl.top_items_loaded = true;
                    owl.notice = None;
                }
                self.refresh_owl_texture();
            }
            Ok(Some(openstory_protocol::OwlSearchEvent::Results { item_id, shops })) => {
                if let Some(owl) = self.owl_search.as_mut() {
                    owl.result_item_id = Some(item_id);
                    owl.results = shops;
                    owl.selected_result = (!owl.results.is_empty()).then_some(0);
                    owl.result_offset = 0;
                    owl.notice = None;
                }
                self.refresh_owl_texture();
            }
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: invalid Owl result packet: {error}"),
        }
    }

    pub(super) fn handle_owl_link_packet(&mut self, packet: &[u8]) {
        match openstory_protocol::parse_owl_link_result(packet) {
            Ok(Some(code)) => {
                let message = match code {
                    1 => "That shop was closed.".to_owned(),
                    2 => "The shop is at full capacity.".to_owned(),
                    3 => "You cannot go to that shop right now.".to_owned(),
                    17 => "You may not enter that store.".to_owned(),
                    18 => "That store is closed or under maintenance.".to_owned(),
                    _ => format!("Unable to visit that shop right now ({code})."),
                };
                if let Some(owl) = self.owl_search.as_mut() {
                    owl.notice = Some(message.clone());
                    self.refresh_owl_texture();
                }
                self.push_chat_line(format!("[Owl] {message}"), [255, 105, 105]);
            }
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: invalid Owl link packet: {error}"),
        }
    }

    pub(super) fn handle_hired_merchant_check_packet(&mut self, packet: &[u8]) {
        match openstory_protocol::parse_hired_merchant_check_result(packet) {
            Ok(7) => {
                let Some((slot, item_id)) = self.pending_merchant_permit.take() else {
                    return;
                };
                let still_owned = self.inventory.as_ref().is_some_and(|inventory| {
                    inventory.items.iter().any(|item| {
                        item.inventory_type == openstory_protocol::InventoryType::Cash
                            && item.slot == slot as u16
                            && item.item_id == item_id
                    })
                });
                if still_owned {
                    self.open_cash_item_composer(slot, item_id, CashItemComposeKind::HiredMerchant);
                } else {
                    self.push_chat_line(
                        "[Merchant] The selected permit is no longer in that Cash slot.".to_owned(),
                        [255, 222, 112],
                    );
                }
            }
            Ok(9) => {
                self.pending_merchant_permit = None;
                self.push_chat_line(
                    "[Merchant] Retrieve your stored items from Fredrick before opening a new store."
                        .to_owned(),
                    [255, 105, 105],
                );
            }
            Ok(code) => {
                self.pending_merchant_permit = None;
                if code != 0 {
                    self.push_chat_line(
                        format!("[Merchant] The permit check failed ({code})."),
                        [255, 105, 105],
                    );
                }
            }
            Err(error) => {
                self.pending_merchant_permit = None;
                eprintln!("OpenStoryRS: invalid hired-merchant check response: {error}");
            }
        }
    }

    pub(super) fn handle_hired_merchant_room_packet(&mut self, packet: &[u8]) {
        let active_room_type = self
            .hired_merchant_room
            .as_ref()
            .map(|room| room.state.room_type)
            .or_else(|| self.trade_room.as_ref().map(|_| 3));
        match openstory_protocol::parse_player_interaction_room_reply(packet, active_room_type) {
            Ok(
                openstory_protocol::PlayerInteractionRoomReply::HiredMerchant(room)
                | openstory_protocol::PlayerInteractionRoomReply::PersonalShop(room),
            ) => {
                self.show_inventory = false;
                self.show_pet_equipment = false;
                self.show_skill_book = false;
                self.show_quest_log = false;
                self.npc_dialogue = None;
                self.npc_shop = None;
                self.storage = None;
                self.storage_texture = None;
                self.trade_room = None;
                self.trade_texture = None;
                self.hired_merchant_room = Some(HiredMerchantRoomUi {
                    state: room,
                    selected_item: None,
                    item_scroll: 0,
                    chat_input: String::new(),
                    chat_focused: false,
                    stock_source: None,
                    stock_price: String::new(),
                    notice: None,
                });
                self.hired_merchant_texture = None;
                self.refresh_hired_merchant_room_texture();
            }
            Ok(openstory_protocol::PlayerInteractionRoomReply::TradeInvite {
                inviter,
                room_id,
            }) => {
                self.pending_trade_invite = Some(PendingTradeInvite {
                    inviter: inviter.clone(),
                    room_id,
                });
                self.push_chat_line(
                    format!("[Trade] {inviter} invited you. Use /trade accept or /trade decline."),
                    [230, 210, 160],
                );
            }
            Ok(openstory_protocol::PlayerInteractionRoomReply::TradeRoom(setup)) => {
                self.trade_request_pending = false;
                self.pending_trade_invite = None;
                self.show_inventory = false;
                self.show_pet_equipment = false;
                self.show_skill_book = false;
                self.show_quest_log = false;
                self.npc_dialogue = None;
                self.npc_shop = None;
                self.storage = None;
                self.storage_texture = None;
                self.hired_merchant_room = None;
                self.hired_merchant_texture = None;
                self.trade_room = Some(TradeRoomUi {
                    self_slot: setup.self_slot,
                    partner: setup.partner,
                    own_items: vec![None; 9],
                    partner_items: vec![None; 9],
                    own_meso: 0,
                    partner_meso: 0,
                    own_confirmed: false,
                    partner_confirmed: false,
                    chat_lines: VecDeque::with_capacity(32),
                    chat_input: String::new(),
                    chat_focused: false,
                    meso_input: String::new(),
                    meso_focused: false,
                    pending_item: None,
                    quantity_input: String::new(),
                    notice: None,
                });
                self.trade_texture = None;
                self.refresh_trade_texture();
            }
            Ok(openstory_protocol::PlayerInteractionRoomReply::TradePartnerJoined(partner)) => {
                if let Some(room) = self.trade_room.as_mut() {
                    room.partner = Some(partner);
                    room.own_confirmed = false;
                    room.partner_confirmed = false;
                    room.notice = Some("The other player joined the trade".to_owned());
                }
                self.refresh_trade_texture();
            }
            Ok(openstory_protocol::PlayerInteractionRoomReply::TradeItem(update)) => {
                if let Some(room) = self.trade_room.as_mut() {
                    let items = if update.player == room.self_slot {
                        &mut room.own_items
                    } else {
                        &mut room.partner_items
                    };
                    if let Some(slot) = items.get_mut(usize::from(update.slot)) {
                        *slot = (update.item_id > 0).then_some(TradeRoomItem {
                            item_id: update.item_id,
                            quantity: update.quantity,
                        });
                    }
                    room.own_confirmed = false;
                    room.partner_confirmed = false;
                    room.notice = Some("Trade items updated".to_owned());
                }
                self.refresh_trade_texture();
            }
            Ok(openstory_protocol::PlayerInteractionRoomReply::TradeMeso { player, meso }) => {
                if let Some(room) = self.trade_room.as_mut() {
                    if player == room.self_slot {
                        room.own_meso = meso;
                    } else {
                        room.partner_meso = meso;
                    }
                    room.own_confirmed = false;
                    room.partner_confirmed = false;
                    room.notice = Some("Trade meso offers updated".to_owned());
                }
                self.refresh_trade_texture();
            }
            Ok(openstory_protocol::PlayerInteractionRoomReply::TradeConfirmed) => {
                if let Some(room) = self.trade_room.as_mut() {
                    room.partner_confirmed = true;
                    room.notice = Some("The other player confirmed".to_owned());
                }
                self.refresh_trade_texture();
            }
            Ok(openstory_protocol::PlayerInteractionRoomReply::TradeEnded {
                player: _,
                result,
            }) => {
                let partner = self
                    .trade_room
                    .as_ref()
                    .and_then(|room| room.partner.as_ref())
                    .map(|partner| partner.name.as_str());
                let message = match result {
                    1 => "The trade was cancelled because the other player did not respond."
                        .to_owned(),
                    2 => partner.map_or_else(
                        || "The other player declined or cancelled the trade.".to_owned(),
                        |name| format!("{name} declined or cancelled the trade."),
                    ),
                    7 => "Trade completed successfully.".to_owned(),
                    8 => "The trade could not be completed.".to_owned(),
                    9 => "Trade failed because an item limit would be exceeded.".to_owned(),
                    12 => "Trade failed because a player changed maps.".to_owned(),
                    13 => "Trade failed because of a data error.".to_owned(),
                    _ => format!("The trade ended ({result})."),
                };
                self.push_chat_line(format!("[Trade] {message}"), [230, 210, 160]);
                self.trade_request_pending = false;
                self.trade_room = None;
                self.trade_texture = None;
                self.show_inventory = false;
            }
            Ok(openstory_protocol::PlayerInteractionRoomReply::Error(code)) => {
                let trade_context = self.trade_room.is_some()
                    || self.trade_request_pending
                    || self.pending_trade_invite.is_some();
                let merchant_context =
                    self.hired_merchant_room.is_some() || self.pending_merchant_permit.is_some();
                let message = match code {
                    1 => "That room is already closed.",
                    2 => "The room is full.",
                    4 => "You can't do that while dead.",
                    17 => "You may not enter this room.",
                    6 if merchant_context => "You need a store permit to do that.",
                    10 if merchant_context => "You can't open a store this close to a portal.",
                    13 if merchant_context => "You can't set up a store right here.",
                    15 if merchant_context => "Stores can only be opened in the Free Market.",
                    18 if merchant_context => "The owner is doing store maintenance.",
                    22 if merchant_context => "Incorrect password.",
                    _ => "The room request was rejected by the server.",
                };
                let label = if trade_context {
                    "Trade"
                } else if merchant_context {
                    "Shop"
                } else {
                    "Interaction"
                };
                self.push_chat_line(
                    format!("[{label}] {message} (code {code})"),
                    [255, 150, 130],
                );
                if trade_context {
                    self.trade_request_pending = false;
                }
                if merchant_context {
                    self.hired_merchant_room = None;
                    self.hired_merchant_texture = None;
                }
            }
            Ok(openstory_protocol::PlayerInteractionRoomReply::VisitorJoined(visitor)) => {
                if let Some(room) = self.hired_merchant_room.as_mut() {
                    room.state
                        .visitors
                        .retain(|entry| entry.slot != visitor.slot);
                    room.state.visitors.push(visitor.clone());
                    room.state
                        .messages
                        .push(openstory_protocol::MerchantChatMessage {
                            text: format!("{} entered the shop.", visitor.name),
                            speaker: 0,
                        });
                    if room.state.visitors.len() > 3 {
                        room.state.visitors.remove(0);
                    }
                }
                self.refresh_hired_merchant_room_texture();
            }
            Ok(openstory_protocol::PlayerInteractionRoomReply::VisitorLeft(slot)) => {
                if let Some(room) = self.hired_merchant_room.as_mut() {
                    room.state.visitors.retain(|visitor| visitor.slot != slot);
                }
                self.refresh_hired_merchant_room_texture();
            }
            Ok(openstory_protocol::PlayerInteractionRoomReply::Exited) => {
                self.hired_merchant_room = None;
                self.hired_merchant_texture = None;
            }
            Ok(openstory_protocol::PlayerInteractionRoomReply::Chat(message)) => {
                if let Some(room) = self.trade_room.as_mut() {
                    room.chat_lines.push_back(message.text);
                    while room.chat_lines.len() > 32 {
                        room.chat_lines.pop_front();
                    }
                } else if let Some(room) = self.hired_merchant_room.as_mut() {
                    room.state.messages.push(message);
                    if room.state.messages.len() > 60 {
                        room.state.messages.remove(0);
                    }
                }
                if self.trade_room.is_some() {
                    self.refresh_trade_texture();
                } else {
                    self.refresh_hired_merchant_room_texture();
                }
            }
            Ok(openstory_protocol::PlayerInteractionRoomReply::ItemsUpdated { meso, items }) => {
                if let Some(room) = self.hired_merchant_room.as_mut() {
                    if let Some(meso) = meso {
                        room.state.meso = meso;
                    }
                    room.state.items = items;
                    room.selected_item = None;
                    room.item_scroll = 0;
                    room.notice = Some(if room.state.room_type == 4 {
                        "Shop listings updated".to_owned()
                    } else {
                        "Merchant stock updated".to_owned()
                    });
                }
                self.refresh_hired_merchant_room_texture();
            }
            Ok(openstory_protocol::PlayerInteractionRoomReply::ItemSold {
                slot,
                quantity: _,
                buyer,
            }) => {
                if let Some(room) = self.hired_merchant_room.as_mut() {
                    if let Ok(index) = usize::try_from(slot) {
                        if let Some(item) = room.state.items.get_mut(index) {
                            item.sold_out = true;
                        }
                    }
                    room.state
                        .messages
                        .push(openstory_protocol::MerchantChatMessage {
                            text: format!("{buyer} bought a listing."),
                            speaker: 3,
                        });
                    if room.state.messages.len() > 60 {
                        room.state.messages.remove(0);
                    }
                }
                self.refresh_hired_merchant_room_texture();
            }
            Ok(
                openstory_protocol::PlayerInteractionRoomReply::OtherRoom(_)
                | openstory_protocol::PlayerInteractionRoomReply::OtherAction(_),
            ) => {}
            Err(error) => {
                eprintln!("OpenStoryRS: invalid hired-merchant room packet: {error}");
            }
        }
    }

    pub(super) fn refresh_skill_book_entries(&mut self) {
        let Some(character) = self.active_character.as_ref() else {
            self.skill_book_entries.clear();
            return;
        };
        let job = character.stats.job;
        let Some(subjob) = skill_subjob_for_tab(job, self.skill_book_tab) else {
            self.skill_book_entries.clear();
            return;
        };
        let mut entries = BTreeMap::new();
        let mut prefixes = vec![subjob];
        if self.skill_book_tab == 0 && subjob != 0 {
            prefixes.push(0);
        }
        for prefix in prefixes {
            let path = format!("{prefix:03}.img/skill");
            let Ok(Some(tree)) = self.skill_archive.root().resolve(&path) else {
                continue;
            };
            let Ok(children) = tree.children() else {
                continue;
            };
            for node in children {
                let Ok(name) = node.name() else { continue };
                let Ok(id) = name.parse::<i32>() else {
                    continue;
                };
                if id / 10_000 != i32::from(prefix) {
                    continue;
                }
                let level = self.skills.get(&id).map_or(0, |skill| skill.level);
                let invisible = integer_property(&node, "invisible").unwrap_or(0) != 0;
                if invisible && level == 0 {
                    continue;
                }
                let Ok(Some(levels)) = node.child("level") else {
                    continue;
                };
                let Ok(level_nodes) = levels.children() else {
                    continue;
                };
                if level_nodes.is_empty() {
                    continue;
                }
                let max_level = level_nodes.len() as i32;
                let string_path = format!("Skill.img/{id:07}");
                let skill_string = self
                    .string_archive
                    .root()
                    .resolve(&string_path)
                    .ok()
                    .flatten();
                let skill_name = skill_string
                    .as_ref()
                    .and_then(|node| string_property(node, "name"))
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| format!("Skill {id}"));
                let learned = self.skills.get(&id).copied().unwrap_or_default();
                let skill_level = learned.level.max(0);
                let required_weapon = integer_property(&node, "weapon")
                    .and_then(|value| i32::try_from(value.saturating_add(100)).ok())
                    .filter(|weapon_type| is_valid_weapon_type(*weapon_type));
                let element = string_property(&node, "elemAttr")
                    .and_then(|value| value.chars().next())
                    .map(|element| element.to_ascii_lowercase())
                    .filter(|element| matches!(element, 'f' | 'i' | 'l' | 's' | 'h' | 'p'));
                let level_node_name = skill_level.max(1).to_string();
                let stats = levels
                    .child(&level_node_name)
                    .ok()
                    .flatten()
                    .or_else(|| level_nodes.first().cloned());
                let (
                    attack,
                    magic_damage,
                    attack_count,
                    bullet_count,
                    mob_count,
                    damage_multiplier,
                    horizontal_range_multiplier,
                    attack_range,
                    magic_damage_multiplier,
                    magic_mastery,
                    fixed_damage,
                    mp_cost,
                    hp_cost,
                    bullet_cost,
                ) = stats.map_or(
                    (
                        false, false, 1, 1, 1, 0.0, 1.0, None, 0.0, 0.1, None, 0, 0, 0,
                    ),
                    |stats| {
                        let damage = integer_property(&stats, "damage").unwrap_or(0);
                        let mad = integer_property(&stats, "mad").unwrap_or(0);
                        let mastery = integer_property(&stats, "mastery").unwrap_or(0);
                        // Mage skill NX stores mastery as a rank: 1 means 15%,
                        // 10 means 60%, matching the localized level descriptions.
                        let magic_mastery = (10 + mastery.clamp(0, 18) * 5) as f64 / 100.0;
                        let fixed = integer_property(&stats, "fixdamage").unwrap_or(0);
                        let bounded_count = |key| {
                            integer_property(&stats, key)
                                .and_then(|value| u8::try_from(value).ok())
                                .filter(|value| *value > 0)
                                .unwrap_or(1)
                                .min(15)
                        };
                        let attack_count = bounded_count("attackCount");
                        let bullet_count = bounded_count("bulletCount");
                        let mob_count = bounded_count("mobCount");
                        let horizontal_range_multiplier = number_property(&stats, "range")
                            .filter(|range| range.is_finite())
                            .map_or(1.0, |range| (f64::from(range) / 100.0).clamp(0.0, 10.0));
                        let read_point = |name: &str| {
                            let node = stats.child(name).ok().flatten()?;
                            match node.value().ok()? {
                                NodeValue::Vector { x, y } => Some((x, y)),
                                _ => None,
                            }
                        };
                        let attack_range = read_point("lt")
                            .zip(read_point("rb"))
                            .filter(|((left, top), (right, bottom))| {
                                *left != 0 || *top != 0 || *right != 0 || *bottom != 0
                            })
                            .map(|((left, top), (right, bottom))| {
                                [left, right, top, bottom].map(|value| {
                                    value.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
                                })
                            });
                        let mp_cost = integer_property(&stats, "mpCon")
                            .map(|value| value.clamp(0, i64::from(u16::MAX)) as u16)
                            .unwrap_or(0);
                        let hp_cost = integer_property(&stats, "hpCon")
                            .map(|value| value.clamp(0, i64::from(u16::MAX)) as u16)
                            .unwrap_or(0);
                        let bullet_cost = integer_property(&stats, "bulletConsume")
                            .map(|value| value.clamp(0, i64::from(u16::MAX)) as u16)
                            .unwrap_or_else(|| u16::from(bullet_count));
                        (
                            is_known_attack_skill(id) || damage > 0 || mad > 0 || fixed > 0,
                            mad > 0,
                            attack_count,
                            bullet_count,
                            mob_count,
                            damage as f64 / 100.0,
                            horizontal_range_multiplier,
                            attack_range,
                            mad.max(0) as f64,
                            magic_mastery,
                            (fixed > 0).then(|| i32::try_from(fixed).unwrap_or(i32::MAX)),
                            mp_cost,
                            hp_cost,
                            bullet_cost,
                        )
                    },
                );
                let passive = (id % 10_000) / 1_000 == 0;
                let requirements = node
                    .child("req")
                    .ok()
                    .flatten()
                    .and_then(|req| req.children().ok())
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|requirement| {
                        let skill_id = requirement.name().ok()?.parse().ok()?;
                        let level = match requirement.value().ok()? {
                            NodeValue::Integer(value) => i32::try_from(value).ok()?,
                            _ => return None,
                        };
                        Some((skill_id, level))
                    })
                    .collect();
                entries.insert(
                    id,
                    SkillUiEntry {
                        id,
                        name: skill_name,
                        level: skill_level,
                        master_level: learned.master_level,
                        max_level,
                        required_weapon,
                        passive,
                        attack,
                        magic_damage,
                        magic_damage_multiplier,
                        magic_mastery,
                        element,
                        attack_count,
                        bullet_count,
                        mob_count,
                        damage_multiplier,
                        horizontal_range_multiplier,
                        attack_range,
                        fixed_damage,
                        mp_cost,
                        hp_cost,
                        bullet_cost,
                        requirements,
                    },
                );
            }
        }
        // The server can grant skills that are not present in the local NX
        // archive. Keep those visible and usable as generic active skills.
        for skill in self.skills.values() {
            if skill.id / 10_000 != i32::from(subjob)
                && !(self.skill_book_tab == 0 && skill.id / 10_000 == 0)
            {
                continue;
            }
            entries.entry(skill.id).or_insert_with(|| SkillUiEntry {
                id: skill.id,
                name: format!("Skill {}", skill.id),
                level: skill.level.max(0),
                master_level: skill.master_level,
                max_level: skill.master_level.max(skill.level).max(1),
                required_weapon: None,
                passive: (skill.id % 10_000) / 1_000 == 0,
                attack: false,
                magic_damage: false,
                magic_damage_multiplier: 0.0,
                magic_mastery: 0.1,
                element: None,
                attack_count: 1,
                bullet_count: 1,
                mob_count: 1,
                damage_multiplier: 0.0,
                horizontal_range_multiplier: 1.0,
                attack_range: None,
                fixed_damage: None,
                mp_cost: 0,
                hp_cost: 0,
                bullet_cost: 0,
                requirements: Vec::new(),
            });
        }
        self.skill_book_entries = entries.into_values().collect();
        self.skill_book_scroll_row = self
            .skill_book_scroll_row
            .min(self.skill_book_entries.len().saturating_sub(1));
    }

    pub(super) fn refresh_skill_book_texture(&mut self) {
        if !self.show_skill_book {
            return;
        }
        let Some(character) = self.active_character.as_ref() else {
            self.skill_book_texture = None;
            return;
        };
        let skill_points = character
            .stats
            .skill_points_by_job
            .iter()
            .find(|(tab, _)| *tab == self.skill_book_tab)
            .map_or(character.stats.skill_points, |(_, points)| {
                u16::from(*points)
            });
        let pixels = render_skill_book_panel(
            &self.font,
            &self.skill_book_entries,
            skill_points,
            self.skill_book_tab,
            self.skill_book_scroll_row,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        const WIDTH: u32 = 430;
        const HEIGHT: u32 = 500;
        let result = if let Some(texture) = self.skill_book_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer.upload_bgra(WIDTH, HEIGHT, &pixels).map(Some)
        };
        match result {
            Ok(Some(texture)) => self.skill_book_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStory: could not update skill book: {error}"),
        }
    }

    pub(super) fn refresh_quest_log_texture(&mut self) {
        if !self.show_quest_log {
            return;
        }
        self.available_quests = available_quest_entries(
            &self.quest_log,
            &self.quest_archive,
            &self.quest_start_npc_index,
            self.active_character.as_ref(),
        );
        let quest_ids =
            quest_ids_for_tab(&self.quest_log, &self.available_quests, self.quest_log_tab);
        if quest_ids.is_empty() {
            self.quest_log_selected = 0;
            self.quest_log_scroll = 0;
        } else {
            self.quest_log_selected = self.quest_log_selected.min(quest_ids.len() - 1);
            self.quest_log_scroll = self
                .quest_log_scroll
                .min(quest_ids.len().saturating_sub(QUEST_LOG_VISIBLE_ROWS));
            if self.quest_log_selected < self.quest_log_scroll
                || self.quest_log_selected
                    >= self.quest_log_scroll.saturating_add(QUEST_LOG_VISIBLE_ROWS)
            {
                self.quest_log_scroll = self
                    .quest_log_selected
                    .min(quest_ids.len() - QUEST_LOG_VISIBLE_ROWS);
            }
        }
        if self.quest_log_tab == QuestLogTab::Active {
            let choice_count = self
                .quest_log
                .started
                .get(self.quest_log_selected)
                .zip(self.active_character.as_ref())
                .map(|(quest, character)| {
                    quest_reward_choices(
                        quest.id,
                        &self.quest_archive,
                        &self.string_archive,
                        character,
                        &mut self.item_name_cache,
                    )
                    .len()
                })
                .unwrap_or(0);
            self.quest_reward_selected = self
                .quest_reward_selected
                .min(choice_count.saturating_sub(1));
        } else {
            self.quest_reward_selected = 0;
        }
        let forfeit_still_active = self.quest_forfeit_confirmation.is_some_and(|quest_id| {
            self.quest_log
                .started
                .iter()
                .any(|quest| quest.id == quest_id)
        });
        if !forfeit_still_active {
            self.quest_forfeit_confirmation = None;
        }
        let pixels = render_quest_log_panel(
            &self.font,
            &self.quest_log,
            &self.available_quests,
            &self.quest_archive,
            &self.string_archive,
            self.inventory.as_ref(),
            &mut self.item_name_cache,
            self.quest_log_tab,
            self.quest_log_selected,
            self.quest_log_scroll,
            self.quest_reward_selected,
            self.active_character.as_ref(),
            self.quest_forfeit_confirmation,
            &self.legacy_ui_archive,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        const WIDTH: u32 = QUEST_LOG_PANEL_WIDTH;
        const HEIGHT: u32 = QUEST_LOG_PANEL_HEIGHT;
        let result = if let Some(texture) = self.quest_log_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer.upload_bgra(WIDTH, HEIGHT, &pixels).map(Some)
        };
        match result {
            Ok(Some(texture)) => self.quest_log_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStory: could not update quest log: {error}"),
        }
    }
}
