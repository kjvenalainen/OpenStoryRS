use super::*;

impl ClientApp {
    fn dialog_window_rect(&self, window: DialogWindow) -> Option<SpriteRect> {
        let (x, y, width, height) = match window {
            DialogWindow::Inventory if self.show_inventory => (
                INVENTORY_PANEL_X,
                INVENTORY_PANEL_Y,
                INVENTORY_PANEL_WIDTH as f32,
                INVENTORY_PANEL_HEIGHT as f32,
            ),
            DialogWindow::EquipmentInventory if self.show_equipment_inventory => (
                EQUIPMENT_INVENTORY_PANEL_X,
                EQUIPMENT_INVENTORY_PANEL_Y,
                EQUIPMENT_INVENTORY_PANEL_WIDTH as f32,
                EQUIPMENT_INVENTORY_PANEL_HEIGHT as f32,
            ),
            DialogWindow::PetEquipment if self.show_pet_equipment => (
                PET_EQUIPMENT_PANEL_X,
                PET_EQUIPMENT_PANEL_Y,
                PET_EQUIPMENT_PANEL_WIDTH as f32,
                PET_EQUIPMENT_PANEL_HEIGHT as f32,
            ),
            DialogWindow::SkillBook if self.show_skill_book => (
                SKILL_BOOK_PANEL_X,
                SKILL_BOOK_PANEL_Y,
                SKILL_BOOK_PANEL_WIDTH as f32,
                SKILL_BOOK_PANEL_HEIGHT as f32,
            ),
            DialogWindow::QuestHelper
                if self.current_field.is_some()
                    && self.cash_shop.is_none()
                    && !self.quest_helper_dismissed
                    && let Some((width, height)) = self.quest_helper_texture_size =>
            {
                (
                    QUEST_HELPER_PANEL_X,
                    QUEST_HELPER_PANEL_Y,
                    width as f32,
                    height as f32,
                )
            }
            DialogWindow::QuestLog if self.show_quest_log => (
                QUEST_LOG_PANEL_X,
                QUEST_LOG_PANEL_Y,
                if self.quest_log_has_detail() {
                    QUEST_LOG_PANEL_WIDTH as f32
                } else {
                    QUEST_LOG_LIST_WIDTH as f32
                },
                QUEST_LOG_PANEL_HEIGHT as f32,
            ),
            DialogWindow::Status if self.show_status => (
                STATUS_PANEL_X,
                STATUS_PANEL_Y,
                STATUS_PANEL_WIDTH as f32,
                STATUS_PANEL_HEIGHT as f32,
            ),
            DialogWindow::Settings if self.show_settings => (
                SETTINGS_PANEL_X,
                SETTINGS_PANEL_Y,
                SETTINGS_PANEL_WIDTH as f32,
                SETTINGS_PANEL_HEIGHT as f32,
            ),
            DialogWindow::KeyConfig if self.show_key_config => (
                KEY_CONFIG_PANEL_X,
                KEY_CONFIG_PANEL_Y,
                KEY_CONFIG_PANEL_WIDTH as f32,
                KEY_CONFIG_PANEL_HEIGHT as f32,
            ),
            DialogWindow::SkillMacros if self.skill_macro_editor.open => (
                skill_macros::PANEL_X,
                skill_macros::PANEL_Y,
                skill_macros::PANEL_WIDTH as f32,
                skill_macros::PANEL_HEIGHT as f32,
            ),
            DialogWindow::OwlSearch if self.owl_search.is_some() => (
                OWL_PANEL_X,
                OWL_PANEL_Y,
                OWL_PANEL_WIDTH as f32,
                OWL_PANEL_HEIGHT as f32,
            ),
            DialogWindow::NpcDialogue => {
                let panel = self.npc_dialogue.as_ref()?.panel;
                (panel.x, panel.y, panel.width, panel.height)
            }
            DialogWindow::Storage if self.storage.is_some() => (
                182.0,
                105.0,
                STORAGE_PANEL_WIDTH as f32,
                STORAGE_PANEL_HEIGHT as f32,
            ),
            DialogWindow::HiredMerchant if self.hired_merchant_room.is_some() => (
                HIRED_MERCHANT_PANEL_X,
                HIRED_MERCHANT_PANEL_Y,
                HIRED_MERCHANT_PANEL_WIDTH as f32,
                HIRED_MERCHANT_PANEL_HEIGHT as f32,
            ),
            DialogWindow::TradeRoom if self.trade_room.is_some() => (
                TRADE_PANEL_X,
                TRADE_PANEL_Y,
                TRADE_PANEL_WIDTH as f32,
                TRADE_PANEL_HEIGHT as f32,
            ),
            DialogWindow::CharacterInfo if self.character_info.is_some() => (
                CHARACTER_INFO_PANEL_X,
                CHARACTER_INFO_PANEL_Y,
                CHARACTER_INFO_PANEL_WIDTH as f32,
                CHARACTER_INFO_PANEL_HEIGHT as f32,
            ),
            DialogWindow::Whisper if self.active_whisper.is_some() => (
                WHISPER_PANEL_X,
                WHISPER_PANEL_Y,
                WHISPER_PANEL_WIDTH as f32,
                WHISPER_PANEL_HEIGHT as f32,
            ),
            DialogWindow::Party if self.show_party_panel => (
                PARTY_PANEL_X,
                PARTY_PANEL_Y,
                PARTY_PANEL_WIDTH as f32,
                PARTY_PANEL_HEIGHT as f32,
            ),
            DialogWindow::Buddy if self.show_buddy_panel => (
                BUDDY_PANEL_X,
                BUDDY_PANEL_Y,
                BUDDY_PANEL_WIDTH as f32,
                BUDDY_PANEL_HEIGHT as f32,
            ),
            DialogWindow::Guild if self.show_guild_panel => (
                GUILD_PANEL_X,
                GUILD_PANEL_Y,
                GUILD_PANEL_WIDTH as f32,
                GUILD_PANEL_HEIGHT as f32,
            ),
            DialogWindow::Family if self.show_family_panel => (
                375.0,
                165.0,
                FAMILY_PANEL_WIDTH as f32,
                FAMILY_PANEL_HEIGHT as f32,
            ),
            DialogWindow::FamilyTree if self.show_family_tree => (
                223.0,
                191.0,
                FAMILY_TREE_PANEL_WIDTH as f32,
                FAMILY_TREE_PANEL_HEIGHT as f32,
            ),
            DialogWindow::Alliance if self.show_alliance_panel => (
                300.0,
                190.0,
                ALLIANCE_PANEL_WIDTH as f32,
                ALLIANCE_PANEL_HEIGHT as f32,
            ),
            DialogWindow::GuildBbs if self.guild_bbs.is_some() => (
                100.0,
                100.0,
                GUILD_BBS_PANEL_WIDTH as f32,
                GUILD_BBS_PANEL_HEIGHT as f32,
            ),
            DialogWindow::EventList if self.event_list_open => (
                EVENT_LIST_PANEL_X,
                EVENT_LIST_PANEL_Y,
                EVENT_LIST_PANEL_WIDTH as f32,
                EVENT_LIST_PANEL_HEIGHT as f32,
            ),
            DialogWindow::Wedding if self.wedding_panel_open => {
                let canvas = ui_foundation::UiRect::new(
                    0.0,
                    0.0,
                    ui_foundation::LOGICAL_WIDTH,
                    ui_foundation::LOGICAL_HEIGHT,
                );
                let panel = ui_foundation::UiRect::new(
                    0.0,
                    0.0,
                    WEDDING_PANEL_WIDTH as f32,
                    WEDDING_PANEL_HEIGHT as f32,
                )
                .centered_in(canvas);
                (panel.x, panel.y, panel.width, panel.height)
            }
            DialogWindow::ChannelSwitch if self.channel_switch_open => (
                channel_switch::DEFAULT_X,
                channel_switch::DEFAULT_Y,
                channel_switch::PANEL_WIDTH as f32,
                channel_switch::PANEL_HEIGHT as f32,
            ),
            DialogWindow::WorldMap if self.world_map.open => (
                self.world_map.x,
                self.world_map.y,
                self.world_map.width as f32,
                self.world_map.height as f32,
            ),
            DialogWindow::PartySearch => {
                let state = self.party_search.as_ref()?;
                (
                    state.x,
                    state.y,
                    party_search::PANEL_WIDTH as f32,
                    party_search::PANEL_HEIGHT as f32,
                )
            }
            DialogWindow::MonsterBook if self.monster_book.open => (
                self.monster_book.x,
                self.monster_book.y,
                self.monster_book.width as f32,
                self.monster_book.height as f32,
            ),
            DialogWindow::Minimap if self.cash_shop.is_none() => {
                let bounds = self.minimap_bounds?;
                (bounds.x, bounds.y, bounds.width, bounds.height)
            }
            _ => return None,
        };
        Some(SpriteRect {
            x,
            y,
            width,
            height,
            blend_mode: BlendMode::Alpha,
        })
    }

    fn dialog_window_order(&self) -> Vec<DialogWindow> {
        let default_order = [
            DialogWindow::EventList,
            DialogWindow::Wedding,
            DialogWindow::GuildBbs,
            DialogWindow::Alliance,
            DialogWindow::FamilyTree,
            DialogWindow::Family,
            DialogWindow::Guild,
            DialogWindow::Buddy,
            DialogWindow::Party,
            DialogWindow::ChannelSwitch,
            DialogWindow::MonsterBook,
            DialogWindow::WorldMap,
            DialogWindow::PartySearch,
            DialogWindow::CharacterInfo,
            DialogWindow::TradeRoom,
            DialogWindow::HiredMerchant,
            DialogWindow::Storage,
            DialogWindow::NpcDialogue,
            DialogWindow::Whisper,
            DialogWindow::OwlSearch,
            DialogWindow::KeyConfig,
            DialogWindow::Settings,
            DialogWindow::Status,
            DialogWindow::QuestHelper,
            DialogWindow::QuestLog,
            DialogWindow::SkillBook,
            DialogWindow::SkillMacros,
            DialogWindow::PetEquipment,
            DialogWindow::EquipmentInventory,
            DialogWindow::Inventory,
            DialogWindow::Minimap,
        ];
        let mut order = self.raised_dialog_windows.clone();
        for window in default_order {
            if !order.contains(&window) {
                order.push(window);
            }
        }
        order
    }

    pub(super) fn has_open_dialog_window(&self) -> bool {
        self.dialog_window_order().into_iter().any(|window| {
            window != DialogWindow::Minimap && self.dialog_window_rect(window).is_some()
        }) || self.cash_shop.is_some()
            || self.chat_open
            || self.active_whisper.is_some()
            || self.messenger.is_some()
            || self.monster_carnival.is_some()
            || self.show_game_menu
    }

    pub(super) fn escape_topmost_dialog(&mut self) -> bool {
        if self.cash_shop.is_some() {
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
            return true;
        }
        if self.chat_open {
            self.chat_open = false;
            self.chat_input_focused = false;
            self.cash_item_compose = None;
            self.refresh_chat_texture();
            return true;
        }
        if self.messenger.is_some() {
            self.handle_messenger_key(
                PhysicalKey::Code(KeyCode::Escape),
                ElementState::Pressed,
                false,
            );
            return true;
        }
        if self.show_game_menu {
            self.show_game_menu = false;
            self.game_menu_texture = None;
            return true;
        }
        if self.monster_carnival.is_some() {
            self.monster_carnival = None;
            return true;
        }

        let Some(window) = self.dialog_window_order().into_iter().find(|window| {
            *window != DialogWindow::QuestHelper
                && *window != DialogWindow::Minimap
                && self.dialog_window_rect(*window).is_some()
        }) else {
            return false;
        };

        match window {
            DialogWindow::Inventory => {
                if self.pending_scroll_slot.take().is_some() {
                    self.refresh_inventory_texture();
                } else {
                    self.show_inventory = false;
                    self.inventory_drag_source = None;
                    self.inventory_hover_slot = None;
                }
            }
            DialogWindow::EquipmentInventory => self.show_equipment_inventory = false,
            DialogWindow::PetEquipment => self.show_pet_equipment = false,
            DialogWindow::SkillBook => {
                self.show_skill_book = false;
                self.skill_book_texture = None;
            }
            DialogWindow::QuestLog => {
                if self.quest_forfeit_confirmation.take().is_some() {
                    self.refresh_quest_log_texture();
                } else {
                    self.show_quest_log = false;
                }
            }
            DialogWindow::Status => {
                if self.status_auto_confirmation {
                    self.resolve_status_auto_allocate(false);
                } else {
                    self.show_status = false;
                }
            }
            DialogWindow::Settings => self.cancel_settings(),
            DialogWindow::KeyConfig => {
                if self.quickslot_key_capture.take().is_some()
                    || self.key_config_capture.take().is_some()
                {
                    self.refresh_key_config_texture();
                } else {
                    self.close_key_config(false);
                }
            }
            DialogWindow::SkillMacros => {
                self.handle_skill_macro_editor_key(
                    PhysicalKey::Code(KeyCode::Escape),
                    ElementState::Pressed,
                    false,
                    None,
                );
            }
            DialogWindow::QuestHelper => return false,
            DialogWindow::OwlSearch => {
                self.owl_search = None;
                self.owl_texture = None;
            }
            DialogWindow::NpcDialogue => self.cancel_npc_dialogue(),
            DialogWindow::Storage => self.close_storage(),
            DialogWindow::HiredMerchant => {
                if let Some(room) = self.hired_merchant_room.as_mut()
                    && room.stock_source.take().is_some()
                {
                    self.refresh_hired_merchant_room_texture();
                } else {
                    self.leave_hired_merchant_room(false);
                }
            }
            DialogWindow::TradeRoom => {
                if let Some(room) = self.trade_room.as_mut()
                    && room.pending_item.take().is_some()
                {
                    room.quantity_input.clear();
                    room.notice = None;
                    self.refresh_trade_texture();
                } else if self
                    .send_trade_action(openstory_protocol::trade_exit_packet(), "cancel the trade")
                {
                    self.trade_room = None;
                    self.trade_texture = None;
                    self.show_inventory = false;
                }
            }
            DialogWindow::CharacterInfo => {
                self.character_info = None;
                self.character_info_texture = None;
            }
            DialogWindow::Whisper => {
                if self.whisper_input_focused {
                    self.whisper_input_focused = false;
                    self.whisper_target_focused = false;
                    self.refresh_whisper_texture();
                } else {
                    self.active_whisper = None;
                    self.whisper_texture = None;
                }
            }
            DialogWindow::Party => self.show_party_panel = false,
            DialogWindow::Buddy => self.show_buddy_panel = false,
            DialogWindow::Guild => {
                if matches!(&self.guild_panel_mode, GuildPanelMode::Overview) {
                    self.show_guild_panel = false;
                } else {
                    self.guild_panel_mode = GuildPanelMode::Overview;
                    self.refresh_guild_texture();
                }
            }
            DialogWindow::Family => {
                if self.family_ui.input.take().is_some() {
                    self.refresh_family_texture();
                } else if self.family_ui.confirm_leave {
                    self.resolve_family_leave(false);
                } else {
                    self.show_family_panel = false;
                }
            }
            DialogWindow::FamilyTree => {
                self.show_family_tree = false;
                self.refresh_family_tree_texture();
            }
            DialogWindow::Alliance => self.show_alliance_panel = false,
            DialogWindow::GuildBbs => {
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
                } else {
                    self.refresh_guild_bbs_texture();
                }
            }
            DialogWindow::EventList => self.close_event_list(),
            DialogWindow::Wedding => self.close_wedding_panel(),
            DialogWindow::ChannelSwitch => self.close_channel_switch(),
            DialogWindow::WorldMap => self.escape_world_map(),
            DialogWindow::PartySearch => {
                self.party_search = None;
                self.party_search_texture = None;
                self.party_search_actions.clear();
            }
            DialogWindow::MonsterBook => {
                if self.monster_book.search_open {
                    self.monster_book.search_open = false;
                    self.monster_book.search_notice = None;
                    self.refresh_monster_book_texture();
                } else {
                    self.monster_book.open = false;
                    self.monster_book.texture = None;
                    self.monster_book.clear_selected_card();
                    self.monster_book_animation_texture = None;
                }
            }
            DialogWindow::Minimap => return false,
        }
        true
    }

    pub(super) fn dialog_window_offset(&self, window: DialogWindow) -> (f32, f32) {
        let offset = self
            .dialog_window_offsets
            .get(&window)
            .copied()
            .unwrap_or((0.0, 0.0));
        let Some(base) = self.dialog_window_rect(window) else {
            return offset;
        };
        let position = ui_foundation::UiRect::new(
            base.x + offset.0,
            base.y + offset.1,
            base.width,
            base.height,
        )
        .clamped_to_canvas();
        (position.x - base.x, position.y - base.y)
    }

    pub(super) fn dialog_window_is_topmost_at(&self, target: DialogWindow, cursor: Point) -> bool {
        let order = self.dialog_window_order();
        let Some(target_index) = order.iter().position(|window| *window == target) else {
            return false;
        };
        !order.iter().take(target_index).any(|window| {
            let Some(base) = self.dialog_window_rect(*window) else {
                return false;
            };
            let (offset_x, offset_y) = self.dialog_window_offset(*window);
            rect_contains(
                SpriteRect {
                    x: base.x + offset_x,
                    y: base.y + offset_y,
                    ..base
                },
                cursor,
            )
        })
    }

    pub(super) fn dialog_window_hit_test(&self, target: DialogWindow, cursor: Point) -> bool {
        let Some(base) = self.dialog_window_rect(target) else {
            return false;
        };
        let (offset_x, offset_y) = self.dialog_window_offset(target);
        rect_contains(
            SpriteRect {
                x: base.x + offset_x,
                y: base.y + offset_y,
                ..base
            },
            cursor,
        ) && self.dialog_window_is_topmost_at(target, cursor)
    }

    pub(super) fn topmost_dialog_window_at(&self, cursor: Point) -> Option<DialogWindow> {
        self.dialog_window_order()
            .into_iter()
            .find(|window| self.dialog_window_hit_test(*window, cursor))
    }

    pub(super) fn raise_dialog_window(&mut self, window: DialogWindow) -> bool {
        let Some(index) = self
            .raised_dialog_windows
            .iter()
            .position(|raised| *raised == window)
        else {
            self.raised_dialog_windows.insert(0, window);
            return true;
        };
        if index == 0 {
            return false;
        }
        self.raised_dialog_windows.remove(index);
        self.raised_dialog_windows.insert(0, window);
        true
    }

    pub(super) fn begin_dialog_window_drag(&mut self, cursor: Point) -> bool {
        let Some(window) = self.topmost_dialog_window_at(cursor) else {
            return false;
        };
        if window == DialogWindow::Minimap {
            return false;
        }
        let Some(base) = self.dialog_window_rect(window) else {
            return false;
        };
        let (offset_x, offset_y) = self.dialog_window_offset(window);
        let actual = SpriteRect {
            x: base.x + offset_x,
            y: base.y + offset_y,
            ..base
        };
        let title_bar = SpriteRect {
            x: actual.x + 3.0,
            y: actual.y + 2.0,
            // Leave the right edge free for close/sort/window controls.
            width: (actual.width - 54.0).max(0.0),
            height: 24.0_f32.min(actual.height),
            blend_mode: BlendMode::Alpha,
        };
        if !rect_contains(title_bar, cursor) {
            return false;
        }
        self.dialog_window_drag = Some(DialogWindowDrag {
            window,
            grab_x: cursor.x - actual.x,
            grab_y: cursor.y - actual.y,
        });
        true
    }

    pub(super) fn update_dialog_window_drag(&mut self, cursor: Point) -> bool {
        let Some(drag) = self.dialog_window_drag else {
            return false;
        };
        let Some(base) = self.dialog_window_rect(drag.window) else {
            self.dialog_window_drag = None;
            return true;
        };
        let origin_x = cursor.x - drag.grab_x;
        let origin_y = cursor.y - drag.grab_y;
        let clamped = ui_foundation::UiRect::new(origin_x, origin_y, base.width, base.height)
            .clamped_to_canvas();
        self.dialog_window_offsets
            .insert(drag.window, (clamped.x - base.x, clamped.y - base.y));
        true
    }

    pub(super) fn persist_dialog_window_position(&self, window: DialogWindow) {
        if window == DialogWindow::Minimap {
            return;
        }
        let (offset_x, offset_y) = self.dialog_window_offset(window);
        let name = format!("{window:?}");
        if let Err(error) = settings::save_dialog_window_offset(&name, offset_x, offset_y) {
            eprintln!("OpenStoryRS: could not save {name} window position: {error}");
        }
    }

    pub(super) fn dialog_window_input_cursor(&self, cursor: Point) -> Point {
        for window in self.dialog_window_order() {
            let Some(base) = self.dialog_window_rect(window) else {
                continue;
            };
            let (offset_x, offset_y) = self.dialog_window_offset(window);
            if offset_x == 0.0 && offset_y == 0.0 {
                continue;
            }
            let actual = SpriteRect {
                x: base.x + offset_x,
                y: base.y + offset_y,
                ..base
            };
            if rect_contains(actual, cursor) {
                return Point {
                    x: cursor.x - offset_x,
                    y: cursor.y - offset_y,
                };
            }
        }
        cursor
    }

    pub(super) fn apply_dialog_window_draw_offsets(&self, draws: &mut [SpriteDraw<'_>]) {
        let textures = [
            (DialogWindow::Inventory, self.inventory_texture.as_ref()),
            (
                DialogWindow::EquipmentInventory,
                self.equipment_inventory_texture.as_ref(),
            ),
            (
                DialogWindow::PetEquipment,
                self.pet_equipment_texture.as_ref(),
            ),
            (DialogWindow::SkillBook, self.skill_book_texture.as_ref()),
            (
                DialogWindow::QuestHelper,
                (!self.quest_helper_dismissed)
                    .then_some(self.quest_helper_texture.as_ref())
                    .flatten(),
            ),
            (DialogWindow::QuestLog, self.quest_log_texture.as_ref()),
            (DialogWindow::Status, self.status_texture.as_ref()),
            (DialogWindow::Status, self.status_detail_texture.as_ref()),
            (DialogWindow::Settings, self.settings_texture.as_ref()),
            (DialogWindow::KeyConfig, self.key_config_texture.as_ref()),
            (DialogWindow::SkillMacros, self.skill_macro_texture.as_ref()),
            (DialogWindow::OwlSearch, self.owl_texture.as_ref()),
            (
                DialogWindow::NpcDialogue,
                self.npc_dialogue.as_ref().map(|ui| &ui.texture),
            ),
            (DialogWindow::Storage, self.storage_texture.as_ref()),
            (
                DialogWindow::HiredMerchant,
                self.hired_merchant_texture.as_ref(),
            ),
            (DialogWindow::TradeRoom, self.trade_texture.as_ref()),
            (
                DialogWindow::CharacterInfo,
                self.character_info_texture.as_ref(),
            ),
            (DialogWindow::Whisper, self.whisper_texture.as_ref()),
            (DialogWindow::Party, self.party_texture.as_ref()),
            (DialogWindow::Buddy, self.buddy_texture.as_ref()),
            (DialogWindow::Guild, self.guild_texture.as_ref()),
            (DialogWindow::Family, self.family_texture.as_ref()),
            (DialogWindow::FamilyTree, self.family_tree_texture.as_ref()),
            (DialogWindow::Alliance, self.alliance_texture.as_ref()),
            (DialogWindow::GuildBbs, self.guild_bbs_texture.as_ref()),
            (DialogWindow::EventList, self.event_list_texture.as_ref()),
            (DialogWindow::Wedding, self.wedding_panel_texture.as_ref()),
            (
                DialogWindow::ChannelSwitch,
                self.channel_switch_texture.as_ref(),
            ),
            (DialogWindow::WorldMap, self.world_map.texture.as_ref()),
            (
                DialogWindow::PartySearch,
                self.party_search_texture.as_ref(),
            ),
            (
                DialogWindow::MonsterBook,
                self.monster_book.texture.as_ref(),
            ),
            (
                DialogWindow::MonsterBook,
                self.monster_book_animation_texture.as_ref(),
            ),
            (DialogWindow::Minimap, self.minimap_texture.as_ref()),
        ];
        let window_for_draw = |draw: &SpriteDraw<'_>| {
            textures
                .iter()
                .find_map(|(window, candidate)| {
                    candidate
                        .filter(|candidate| std::ptr::eq(*candidate, draw.texture))
                        .map(|_| *window)
                })
                .or_else(|| {
                    let is_settings_dimmer = self.show_settings
                        && self.solid_textures.first().is_some_and(|texture| {
                            std::ptr::eq(texture, draw.texture)
                                && draw.x == 0.0
                                && draw.y == 0.0
                                && draw.width == 1024.0
                                && draw.height == 768.0
                                && draw.tint == [0.0, 0.0, 0.0]
                                && draw.opacity == 0.58
                        });
                    is_settings_dimmer.then_some(DialogWindow::Settings)
                })
        };
        let mut draw_slots = Vec::new();
        let mut window_draws = Vec::new();
        for (index, draw) in draws.iter_mut().enumerate() {
            let Some(window) = window_for_draw(draw) else {
                continue;
            };
            let (offset_x, offset_y) = self.dialog_window_offset(window);
            draw.x += offset_x;
            draw.y += offset_y;
            draw_slots.push(index);
            window_draws.push((
                window,
                index,
                SpriteDraw {
                    texture: draw.texture,
                    x: draw.x,
                    y: draw.y,
                    width: draw.width,
                    height: draw.height,
                    uv: draw.uv,
                    tint: draw.tint,
                    blend_mode: draw.blend_mode,
                    flip_x: draw.flip_x,
                    opacity: draw.opacity,
                },
            ));
        }

        let mut layer_rank = HashMap::new();
        for (rank, window) in self.dialog_window_order().into_iter().rev().enumerate() {
            layer_rank.insert(window, rank);
        }
        window_draws.sort_by_key(|(window, original_index, _)| {
            (
                layer_rank.get(window).copied().unwrap_or(usize::MAX),
                *original_index,
            )
        });
        for (slot, (_, _, draw)) in draw_slots.into_iter().zip(window_draws) {
            draws[slot] = draw;
        }
    }
}
