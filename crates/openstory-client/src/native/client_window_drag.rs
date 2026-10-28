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
            DialogWindow::PetEquipment if self.show_pet_equipment => (618.0, 128.0, 390.0, 480.0),
            DialogWindow::SkillBook if self.show_skill_book => (
                SKILL_BOOK_PANEL_X,
                SKILL_BOOK_PANEL_Y,
                SKILL_BOOK_PANEL_WIDTH as f32,
                SKILL_BOOK_PANEL_HEIGHT as f32,
            ),
            DialogWindow::QuestLog if self.show_quest_log => (
                QUEST_LOG_PANEL_X,
                QUEST_LOG_PANEL_Y,
                QUEST_LOG_PANEL_WIDTH as f32,
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
            DialogWindow::NpcShop if self.npc_shop.is_some() => (202.0, 124.0, 620.0, 520.0),
            DialogWindow::Storage if self.storage.is_some() => (182.0, 105.0, 660.0, 560.0),
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
                680.0,
                174.0,
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
            DialogWindow::Wedding if self.wedding_panel_open => (
                (1024.0 - WEDDING_PANEL_WIDTH as f32) * 0.5,
                (768.0 - WEDDING_PANEL_HEIGHT as f32) * 0.5,
                WEDDING_PANEL_WIDTH as f32,
                WEDDING_PANEL_HEIGHT as f32,
            ),
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

    fn dialog_window_order(&self) -> [DialogWindow; 30] {
        [
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
            DialogWindow::NpcShop,
            DialogWindow::NpcDialogue,
            DialogWindow::Whisper,
            DialogWindow::OwlSearch,
            DialogWindow::KeyConfig,
            DialogWindow::Settings,
            DialogWindow::Status,
            DialogWindow::QuestLog,
            DialogWindow::SkillBook,
            DialogWindow::SkillMacros,
            DialogWindow::PetEquipment,
            DialogWindow::EquipmentInventory,
            DialogWindow::Inventory,
        ]
    }

    pub(super) fn has_open_dialog_window(&self) -> bool {
        self.dialog_window_order()
            .into_iter()
            .any(|window| self.dialog_window_rect(window).is_some())
            || self.cash_shop.is_some()
            || self.chat_open
            || self.active_whisper.is_some()
            || self.messenger.is_some()
            || self.monster_carnival.is_some()
            || self.show_status_menu
    }

    fn dialog_window_offset(&self, window: DialogWindow) -> (f32, f32) {
        self.dialog_window_offsets
            .get(&window)
            .copied()
            .unwrap_or((0.0, 0.0))
    }

    pub(super) fn begin_dialog_window_drag(&mut self, cursor: Point) -> bool {
        for window in self.dialog_window_order().into_iter().rev() {
            let Some(base) = self.dialog_window_rect(window) else {
                continue;
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
            if rect_contains(title_bar, cursor) {
                self.dialog_window_drag = Some(DialogWindowDrag {
                    window,
                    grab_x: cursor.x - actual.x,
                    grab_y: cursor.y - actual.y,
                });
                return true;
            }
        }
        false
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
        let clamped_x = origin_x.clamp(0.0, (1024.0 - base.width).max(0.0));
        let clamped_y = origin_y.clamp(0.0, (768.0 - base.height).max(0.0));
        self.dialog_window_offsets
            .insert(drag.window, (clamped_x - base.x, clamped_y - base.y));
        true
    }

    pub(super) fn dialog_window_input_cursor(&self, cursor: Point) -> Point {
        for window in self.dialog_window_order().into_iter().rev() {
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
            (
                DialogWindow::NpcShop,
                self.npc_shop.as_ref().map(|ui| &ui.texture),
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
        ];
        for draw in draws {
            let Some(window) = textures.iter().find_map(|(window, texture)| {
                texture
                    .filter(|texture| std::ptr::eq(*texture, draw.texture))
                    .map(|_| *window)
            }) else {
                continue;
            };
            let (offset_x, offset_y) = self.dialog_window_offset(window);
            draw.x += offset_x;
            draw.y += offset_y;
        }
    }
}
