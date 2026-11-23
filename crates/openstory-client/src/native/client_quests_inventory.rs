use super::*;

impl ClientApp {
    pub(super) fn reset_quest_helper_tracking(&mut self) {
        self.quest_helper_order = self
            .quest_log
            .started
            .iter()
            .map(|quest| quest.id)
            .collect();
        self.quest_helper_progress = self
            .quest_log
            .started
            .iter()
            .map(|quest| {
                (
                    quest.id,
                    quest_helper_progress_snapshot(
                        quest,
                        &self.quest_archive,
                        self.inventory.as_ref(),
                    ),
                )
            })
            .collect();
        self.quest_helper_tracking_initialized = true;
    }

    fn register_inventory_equip_click(
        &mut self,
        target: Option<(openstory_protocol::InventoryType, u16)>,
    ) -> bool {
        const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(500);

        let Some((openstory_protocol::InventoryType::Equip, slot)) = target else {
            self.last_inventory_equip_click = None;
            return false;
        };
        let now = Instant::now();
        if self
            .last_inventory_equip_click
            .is_some_and(|(previous_slot, at)| {
                previous_slot == slot && now.saturating_duration_since(at) <= DOUBLE_CLICK_INTERVAL
            })
        {
            self.last_inventory_equip_click = None;
            true
        } else {
            self.last_inventory_equip_click = Some((slot, now));
            false
        }
    }

    fn send_equipment_scroll(&mut self, scroll_slot: u16, target_slot: u16) {
        let Some(inventory) = self.inventory.as_ref() else {
            return;
        };
        let scroll_exists = inventory.items.iter().any(|item| {
            item.inventory_type == openstory_protocol::InventoryType::Use
                && item.slot == scroll_slot
                && item.item_id / 10_000 == 204
        });
        let target_exists = inventory.equipped().any(|item| item.slot == target_slot);
        if !scroll_exists || !target_exists {
            return;
        }
        if !self.channel_connected {
            eprintln!("OpenStoryRS: cannot apply a scroll while disconnected from the field");
            return;
        }
        let (Ok(scroll_slot), Ok(target_slot)) =
            (i16::try_from(scroll_slot), i16::try_from(target_slot))
        else {
            return;
        };
        let client_tick = self.client_started_at.elapsed().as_millis() as i32;
        let Some(packet) =
            openstory_protocol::scroll_equip_packet(scroll_slot, target_slot, client_tick)
        else {
            return;
        };
        if let Some(session) = self.session.as_ref()
            && let Err(error) = session.send_packet(packet)
        {
            eprintln!("OpenStoryRS: could not apply equipment scroll: {error}");
            self.channel_connected = false;
        }
    }

    pub(super) fn apply_quest_update(&mut self, update: openstory_protocol::QuestUpdate) {
        self.quest_log.apply(update);
        self.refresh_npc_quest_marks();
        self.refresh_quest_helper_texture();
        if self.show_quest_log {
            self.refresh_quest_log_texture();
        }
    }

    pub(super) fn handle_quest_timer_update(
        &mut self,
        update: openstory_protocol::QuestTimerUpdate,
    ) {
        match update {
            openstory_protocol::QuestTimerUpdate::Set { timers } => {
                for timer in timers {
                    self.push_chat_line(
                        format!("[Quest] Time limit set: {} seconds", timer.time_ms / 1_000),
                        [255, 222, 112],
                    );
                }
            }
            openstory_protocol::QuestTimerUpdate::Lifted { quest_id, .. } => {
                if let Some(name) = quest_info_text(&self.quest_archive, quest_id, "name") {
                    self.push_chat_line(
                        format!("[Quest] Time limit lifted: {name}"),
                        [255, 222, 112],
                    );
                }
            }
        }
    }

    pub(super) fn resolve_quest_forfeit(&mut self, accepted: bool) {
        let Some(quest_id) = self.quest_forfeit_confirmation.take() else {
            return;
        };
        if accepted {
            if !self
                .quest_log
                .started
                .iter()
                .any(|quest| quest.id == quest_id)
            {
                self.refresh_quest_log_texture();
                return;
            }
            if let Some(session) = self.session.as_ref() {
                if let Err(error) =
                    session.send_packet(openstory_protocol::forfeit_quest_packet(quest_id))
                {
                    eprintln!("OpenStory: could not forfeit quest: {error}");
                    self.channel_connected = false;
                }
            }
        }
        self.refresh_quest_log_texture();
    }

    fn teleport_to_selected_quest_npc(&mut self) {
        if self.quest_log_tab != QuestLogTab::Active {
            return;
        }
        let Some(quest_id) = quest_ids_for_tab(
            &self.quest_log,
            &self.available_quests,
            self.quest_log_tab,
            &self.quest_archive,
            &self.font,
        )
        .get(self.quest_log_selected)
        .copied() else {
            return;
        };
        let Some(quest) = self
            .quest_log
            .started
            .iter()
            .find(|quest| quest.id == quest_id)
        else {
            return;
        };
        if !quest_is_completable(
            quest,
            &self.quest_archive,
            self.inventory.as_ref(),
            &self.quest_log,
        ) {
            return;
        }
        if !self.channel_connected {
            self.push_chat_line(
                "[Quest] Not connected to a game channel.".to_owned(),
                [255, 105, 105],
            );
            return;
        }
        let Some(session) = self.session.as_ref() else {
            self.push_chat_line(
                "[Quest] Not connected to a game channel.".to_owned(),
                [255, 105, 105],
            );
            return;
        };
        let command = format!("@teleportToNpcForQuestCompletion {quest_id}");
        let packet = match openstory_protocol::general_chat_packet(&command, true) {
            Ok(packet) => packet,
            Err(error) => {
                self.push_chat_line(
                    format!("[Quest] Could not request NPC teleport: {error}"),
                    [255, 105, 105],
                );
                return;
            }
        };
        if let Err(error) = session.send_packet(packet) {
            eprintln!("OpenStory: could not request quest NPC teleport: {error}");
            self.channel_connected = false;
            self.push_chat_line(
                format!("[Quest] Could not request NPC teleport: {error}"),
                [255, 105, 105],
            );
            return;
        }

        self.show_quest_log = false;
    }

    pub(super) fn change_selected_quest_reward(&mut self, direction: isize) {
        if self.quest_log_tab != QuestLogTab::Active {
            return;
        }
        let Some(quest_id) = quest_ids_for_tab(
            &self.quest_log,
            &self.available_quests,
            self.quest_log_tab,
            &self.quest_archive,
            &self.font,
        )
        .get(self.quest_log_selected)
        .copied() else {
            return;
        };
        let choice_count = self.active_character.as_ref().map_or(0, |character| {
            quest_reward_choices(
                quest_id,
                &self.quest_archive,
                &self.string_archive,
                character,
                &mut self.item_name_cache,
            )
            .len()
        });
        if choice_count <= 1 {
            return;
        }
        self.quest_reward_selected = if direction < 0 {
            self.quest_reward_selected.saturating_sub(1)
        } else {
            self.quest_reward_selected
                .saturating_add(1)
                .min(choice_count - 1)
        };
        self.refresh_quest_log_texture();
    }

    pub(super) fn quest_log_has_detail(&self) -> bool {
        self.quest_log_detail_open
            && quest_ids_for_tab(
                &self.quest_log,
                &self.available_quests,
                self.quest_log_tab,
                &self.quest_archive,
                &self.font,
            )
            .get(self.quest_log_selected)
            .is_some()
    }

    pub(super) fn move_quest_log_selection(&mut self, direction: isize) {
        let quest_count = quest_ids_for_tab(
            &self.quest_log,
            &self.available_quests,
            self.quest_log_tab,
            &self.quest_archive,
            &self.font,
        )
        .len();
        if quest_count == 0 {
            return;
        }

        let selected = self
            .quest_log_selected
            .saturating_add_signed(direction)
            .min(quest_count - 1);
        if selected == self.quest_log_selected {
            return;
        }
        self.quest_log_selected = selected;
        if self.quest_log_selected < self.quest_log_scroll {
            self.quest_log_scroll = self.quest_log_selected;
        } else if self.quest_log_selected
            >= self
                .quest_log_scroll
                .saturating_add(QUEST_LOG_VISIBLE_ROWS)
        {
            self.quest_log_scroll = self
                .quest_log_selected
                .saturating_add(1)
                .saturating_sub(QUEST_LOG_VISIBLE_ROWS)
                .min(quest_count.saturating_sub(QUEST_LOG_VISIBLE_ROWS));
        }
        self.quest_detail_scroll = 0;
        self.quest_reward_selected = 0;
        self.refresh_quest_log_texture();
    }

    pub(super) fn click_quest_log(&mut self, cursor: Point) -> bool {
        if !self.show_quest_log {
            return false;
        }
        let has_detail = self.quest_log_has_detail();
        let (panel_width, panel_height) = if has_detail || self.quest_forfeit_confirmation.is_some()
        {
            (QUEST_LOG_PANEL_WIDTH as f32, QUEST_LOG_PANEL_HEIGHT as f32)
        } else {
            (QUEST_LOG_LIST_WIDTH as f32, QUEST_LOG_PANEL_HEIGHT as f32)
        };
        if !rect_contains(
            SpriteRect {
                x: QUEST_LOG_PANEL_X,
                y: QUEST_LOG_PANEL_Y,
                width: panel_width,
                height: panel_height,
                blend_mode: BlendMode::Alpha,
            },
            cursor,
        ) {
            return false;
        }
        let x = cursor.x - QUEST_LOG_PANEL_X;
        let y = cursor.y - QUEST_LOG_PANEL_Y;
        if let Some(quest_id) = self.quest_forfeit_confirmation {
            let overlay_x = (QUEST_LOG_PANEL_WIDTH as f32 - 266.0) * 0.5;
            if (218.0..240.0).contains(&y) && (overlay_x + 35.0..overlay_x + 105.0).contains(&x) {
                self.resolve_quest_forfeit(true);
            } else if (218.0..240.0).contains(&y)
                && (overlay_x + 140.0..overlay_x + 215.0).contains(&x)
            {
                self.resolve_quest_forfeit(false);
            } else {
                self.quest_forfeit_confirmation = Some(quest_id);
            }
            return true;
        }
        if has_detail && ui_foundation::QUEST_DETAIL_CLOSE_BUTTON.contains(x, y) {
            self.quest_log_detail_open = false;
            self.quest_detail_scroll = 0;
            self.refresh_quest_log_texture();
            return true;
        }
        if ui_foundation::QUEST_LIST_CLOSE_BUTTON.contains(x, y) {
            self.show_quest_log = false;
            return true;
        }
        let tabs = [
            QuestLogTab::Available,
            QuestLogTab::Active,
            QuestLogTab::Completed,
        ];
        if let Some(index) =
            (0..tabs.len()).find(|index| ui_foundation::quest_tab(*index).contains(x, y))
        {
            self.quest_log_tab = tabs[index];
            self.quest_detail_scroll = 0;
            self.quest_log_detail_open = true;
            self.quest_log_selected = 0;
            self.quest_log_scroll = 0;
            self.quest_reward_selected = 0;
            self.refresh_quest_log_texture();
            return true;
        }
        if let Some(row) = ui_foundation::QUEST_GRID.hit(x, y) {
            let index = self.quest_log_scroll.saturating_add(row);
            if index
                < quest_ids_for_tab(
                    &self.quest_log,
                    &self.available_quests,
                    self.quest_log_tab,
                    &self.quest_archive,
                    &self.font,
                )
                .len()
            {
                self.quest_detail_scroll = 0;
                self.quest_log_selected = index;
                self.quest_log_detail_open = true;
                self.quest_reward_selected = 0;
                self.refresh_quest_log_texture();
            }
            return true;
        }
        if has_detail && self.quest_detail_scroll_max > 0 {
            let scrollbar_x = QUEST_LOG_DETAIL_X as f32 + QUEST_LOG_DETAIL_WIDTH as f32
                - QUEST_SCROLLBAR_WIDTH as f32
                - QUEST_DETAIL_SCROLLBAR_INSET as f32;
            let body_top = QUEST_DETAIL_BODY_TOP as f32;
            let body_height = quest_detail_body_bottom(self.quest_log_tab)
                .saturating_sub(QUEST_DETAIL_BODY_TOP) as f32;
            if (scrollbar_x..scrollbar_x + QUEST_SCROLLBAR_WIDTH as f32).contains(&x)
                && (body_top..body_top + body_height).contains(&y)
            {
                let fraction = ((y - body_top) / body_height).clamp(0.0, 1.0);
                self.quest_detail_scroll =
                    (fraction * self.quest_detail_scroll_max as f32).round() as usize;
                self.refresh_quest_log_texture();
                return true;
            }
        }
        if self.quest_log_tab == QuestLogTab::Active
            && (300.0..335.0).contains(&y)
            && (QUEST_LOG_DETAIL_X as f32 + 18.0
                ..QUEST_LOG_DETAIL_X as f32 + QUEST_LOG_DETAIL_WIDTH as f32 - 18.0)
                .contains(&x)
        {
            if x < QUEST_LOG_DETAIL_X as f32 + 56.0 {
                self.change_selected_quest_reward(-1);
            } else if x > QUEST_LOG_DETAIL_X as f32 + QUEST_LOG_DETAIL_WIDTH as f32 - 40.0 {
                self.change_selected_quest_reward(1);
            }
            return true;
        }
        if has_detail && ui_foundation::QUEST_HELPER_BUTTON.contains(x, y) {
            if self.quest_helper_dismissed || self.quest_helper_texture.is_none() {
                self.quest_helper_dismissed = false;
                self.refresh_quest_helper_texture();
            }
        } else if has_detail && ui_foundation::QUEST_TELEPORT_BUTTON.contains(x, y) {
            self.teleport_to_selected_quest_npc();
        } else if self.quest_log_tab == QuestLogTab::Active {
            if ui_foundation::QUEST_FORFEIT_BUTTON.contains(x, y)
                && let Some(quest_id) = quest_ids_for_tab(
                    &self.quest_log,
                    &self.available_quests,
                    self.quest_log_tab,
                    &self.quest_archive,
                    &self.font,
                )
                .get(self.quest_log_selected)
                .copied()
            {
                self.quest_forfeit_confirmation = Some(quest_id);
                self.refresh_quest_log_texture();
            }
        }
        true
    }

    pub(super) fn begin_skill_book_pointer(&mut self, cursor: Point) -> bool {
        const PANEL_X: f32 = SKILL_BOOK_PANEL_X;
        const PANEL_Y: f32 = SKILL_BOOK_PANEL_Y;
        if !rect_contains(
            SpriteRect {
                x: PANEL_X,
                y: PANEL_Y,
                width: SKILL_BOOK_PANEL_WIDTH as f32,
                height: SKILL_BOOK_PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            cursor,
        ) {
            return false;
        }
        let pending_skill_click = self.skill_book_last_click.take();
        let local_x = cursor.x - PANEL_X;
        let local_y = cursor.y - PANEL_Y;
        if ui_foundation::SKILL_CLOSE_BUTTON.contains(local_x, local_y) {
            self.show_skill_book = false;
            return true;
        }
        if let Some(tab) =
            (0..5).find(|tab| ui_foundation::skill_tab(*tab).contains(local_x, local_y))
        {
            let tab = tab as u8;
            if self
                .active_character
                .as_ref()
                .is_some_and(|character| skill_subjob_for_tab(character.stats.job, tab).is_some())
            {
                self.skill_book_tab = tab;
                self.skill_book_scroll_row = 0;
                self.refresh_skill_book_entries();
                self.refresh_skill_book_texture();
            }
            return true;
        }
        if ui_foundation::SKILL_MACRO_BUTTON.contains(local_x, local_y) {
            self.open_skill_macro_editor();
            return true;
        }
        if let Some(row) = (0..SKILL_BOOK_VISIBLE_ROWS)
            .find(|row| ui_foundation::skill_row(*row).contains(local_x, local_y))
        {
            let Some(entry) = self
                .skill_book_entries
                .get(self.skill_book_scroll_row.saturating_add(row))
                .cloned()
            else {
                return true;
            };
            if ui_foundation::skill_sp_button(row).contains(local_x, local_y) {
                self.spend_skill_point(&entry);
            } else if entry.level <= 0 {
                return true;
            } else if !entry.passive {
                self.skill_book_last_click = pending_skill_click;
                let (offset_x, offset_y) = self.dialog_window_offset(DialogWindow::SkillBook);
                self.skill_book_drag = Some(SkillBookDrag {
                    skill_id: entry.id,
                    start: Point {
                        x: cursor.x + offset_x,
                        y: cursor.y + offset_y,
                    },
                    moved: false,
                });
                self.skill_macro_drag_texture = decode_nx_icon(
                    &self.skill_archive,
                    &format!("{:03}.img/skill/{:07}/icon", entry.id / 10_000, entry.id),
                )
                .and_then(|(width, height, pixels)| {
                    self.renderer
                        .as_ref()
                        .and_then(|renderer| renderer.upload_bgra(width, height, &pixels).ok())
                });
            } else {
                self.use_skill(&entry);
            }
            return true;
        }
        true
    }

    pub(super) fn spend_skill_point(&mut self, skill: &SkillUiEntry) {
        let Some(character) = self.active_character.clone() else {
            return;
        };
        let points = super::client_cash_skills::skill_points_for_book(
            &character.stats,
            self.skill_book_tab,
            &self.skills,
        );
        if !super::client_cash_skills::can_spend_skill_point(skill, points, &self.skills) {
            return;
        }
        let tick = self.client_started_at.elapsed().as_millis() as i32;
        if let Some(session) = self.session.as_ref() {
            if let Err(error) =
                session.send_packet(openstory_protocol::spend_skill_point_packet(skill.id, tick))
            {
                eprintln!("OpenStory: could not request a skill point: {error}");
                self.channel_connected = false;
            }
        }
    }

    pub(super) fn use_skill(&mut self, skill: &SkillUiEntry) {
        if skill.level <= 0 || skill.passive {
            return;
        }
        let now = Instant::now();
        match self.skill_cooldowns.get(&skill.id).copied() {
            Some(cooldown) if cooldown.expires_at > now => {
                self.push_chat_line(
                    "You cannot use this skill as it is on cooldown.".to_owned(),
                    [255, 105, 105],
                );
                return;
            }
            Some(_) => {
                self.skill_cooldowns.remove(&skill.id);
            }
            None => {}
        }
        if character_has_buff(
            &self.active_buffs,
            openstory_protocol::BuffMaskGroup::Second,
            STATUS_STUN,
        ) || character_has_buff(
            &self.active_buffs,
            openstory_protocol::BuffMaskGroup::Second,
            STATUS_SEAL,
        ) {
            return;
        }
        let block_reason = self.active_character.as_ref().and_then(|character| {
            skill_use_block_reason(skill, character, self.inventory.as_ref())
        });
        if let Some(reason) = block_reason {
            self.push_chat_line(format!("[Notice] {reason}"), [255, 215, 128]);
            return;
        }
        if skill.attack {
            self.try_local_skill_attack(skill);
        } else {
            let tick = self.client_started_at.elapsed().as_millis() as i32;
            let packet = openstory_protocol::use_skill_packet(
                skill.id,
                skill.level.clamp(0, 255) as u8,
                tick,
            );
            match self
                .session
                .as_ref()
                .map(|session| session.send_packet(packet))
            {
                Some(Ok(())) => {
                    self.last_local_skill_cast = Some((skill.id, Instant::now()));
                    self.pending_zombify_heal_feedback_until = if skill.id == SKILL_HEAL
                        && character_has_mob_skill_buff(
                            &self.active_buffs,
                            openstory_protocol::BuffMaskGroup::Second,
                            STATUS_ZOMBIFY,
                            MOB_SKILL_TYPE_UNDEAD,
                        ) {
                        Instant::now().checked_add(ZOMBIFY_HEAL_FEEDBACK_WINDOW)
                    } else {
                        None
                    };
                    self.show_local_skill_use_effect(skill.id, skill.level.clamp(0, 255) as u8);
                    if let Some(position) = self
                        .map_scene
                        .as_ref()
                        .map(|scene| (rounded_i16(scene.player.x), rounded_i16(scene.player.y)))
                    {
                        self.audio
                            .play_skill_use(&self.sound_archive, skill.id, position);
                    }
                }
                Some(Err(error)) => {
                    eprintln!("OpenStory: could not use skill {}: {error}", skill.id);
                    self.channel_connected = false;
                }
                None => {}
            }
        }
    }

    pub(super) fn show_local_skill_buff_effect(&mut self, skill_id: i32) {
        const LOCAL_CAST_ECHO_WINDOW: Duration = Duration::from_millis(720);
        let now = Instant::now();
        if self
            .last_local_skill_cast
            .is_some_and(|(cast_id, cast_at)| {
                cast_id == skill_id
                    && now.saturating_duration_since(cast_at) <= LOCAL_CAST_ECHO_WINDOW
            })
        {
            return;
        }
        let Some((look, character_level)) = self
            .active_character
            .as_ref()
            .map(|character| (character.look.clone(), character.stats.level))
        else {
            return;
        };
        let skill_level = self
            .skills
            .get(&skill_id)
            .map_or(0, |skill| skill.level.max(0) as u8);
        let animations = match self.load_character_skill_effects(
            skill_id,
            skill_level,
            character_level,
            character_weapon_id(&look).is_some_and(is_two_handed_weapon),
        ) {
            Ok(animations) => animations,
            Err(error) => {
                eprintln!("OpenStory: could not load skill {skill_id} buff effect: {error}");
                Vec::new()
            }
        };
        if let Some(scene) = self.map_scene.as_mut() {
            scene.show_character_skill_effect(None, animations, 1.1);
        }
    }

    pub(super) fn show_local_skill_use_effect(&mut self, skill_id: i32, skill_level: u8) {
        let Some((look, character_level)) = self
            .active_character
            .as_ref()
            .map(|character| (character.look.clone(), character.stats.level))
        else {
            return;
        };
        let teleport = is_teleport_skill(skill_id);
        let flash_jump = skill_id == 4_111_006;
        let moved = if teleport || flash_jump {
            let teleport_range = teleport_skill_range(&self.skill_archive, skill_id, skill_level);
            self.map_scene.as_mut().is_some_and(|scene| {
                let moved = scene.apply_use_movement(
                    skill_id,
                    teleport_range,
                    self.gameplay_input.left,
                    self.gameplay_input.right,
                    self.gameplay_input.up,
                    self.gameplay_input.down,
                );
                if moved {
                    let camera_delta = scene.update_camera();
                    scene.advance_background_positions(
                        camera_delta.0,
                        camera_delta.1,
                        1,
                        &mut self.sprite_draws,
                    );
                    for index in &scene.world_draw_indices {
                        if let Some((_, rect)) = self.sprite_draws.get_mut(*index) {
                            rect.x += camera_delta.0;
                            rect.y += camera_delta.1;
                        }
                    }
                }
                moved
            })
        } else {
            false
        };
        if teleport && !moved {
            return;
        }
        let animations = if teleport {
            match self.load_teleport_effect_frames() {
                Ok(frames) if !frames.is_empty() => {
                    vec![CharacterSkillEffectAnimation {
                        z: 0,
                        flip_with_facing: true,
                        frames,
                    }]
                }
                Ok(_) => Vec::new(),
                Err(error) => {
                    eprintln!("OpenStory: could not load teleport effect: {error}");
                    Vec::new()
                }
            }
        } else {
            match self.load_character_skill_effects(
                skill_id,
                skill_level,
                character_level,
                character_weapon_id(&look).is_some_and(is_two_handed_weapon),
            ) {
                Ok(animations) => animations,
                Err(error) => {
                    eprintln!("OpenStory: could not load local skill {skill_id} effect: {error}");
                    Vec::new()
                }
            }
        };
        let stance = (!teleport)
            .then(|| {
                character_skill_action_stance(&self.skill_archive, &look, skill_id, skill_level)
                    .ok()
                    .flatten()
            })
            .flatten();
        if let Some(scene) = self.map_scene.as_mut() {
            if let Some(stance) = stance {
                scene.start_attack(stance, &mut self.sprite_draws);
            }
            scene.show_character_skill_effect(None, animations, 1.1);
        }
    }

    pub(super) fn activate_inventory_item(
        &mut self,
        inventory_type: openstory_protocol::InventoryType,
        slot: u16,
    ) {
        let Some(inventory) = self.inventory.as_ref() else {
            return;
        };
        let Some(item) = inventory
            .items
            .iter()
            .find(|item| item.inventory_type == inventory_type && item.slot == slot)
            .cloned()
        else {
            return;
        };
        if item.inventory_type == openstory_protocol::InventoryType::Use
            && item.item_id / 10_000 == 204
        {
            self.pending_scroll_slot = if self.pending_scroll_slot == Some(slot) {
                None
            } else {
                Some(slot)
            };
            self.refresh_inventory_texture();
            return;
        }
        if let Some(scroll_slot) = self.pending_scroll_slot {
            if inventory_type == openstory_protocol::InventoryType::Equipped {
                if !self.channel_connected {
                    eprintln!(
                        "OpenStoryRS: cannot apply a scroll while disconnected from the field"
                    );
                    return;
                }
                let (Ok(scroll_slot), Ok(target_slot)) =
                    (i16::try_from(scroll_slot), i16::try_from(item.slot))
                else {
                    self.pending_scroll_slot = None;
                    self.refresh_inventory_texture();
                    return;
                };
                let client_tick = self.client_started_at.elapsed().as_millis() as i32;
                let Some(packet) =
                    openstory_protocol::scroll_equip_packet(scroll_slot, target_slot, client_tick)
                else {
                    self.pending_scroll_slot = None;
                    self.refresh_inventory_texture();
                    return;
                };
                self.pending_scroll_slot = None;
                if let Some(session) = self.session.as_ref() {
                    if let Err(error) = session.send_packet(packet) {
                        eprintln!("OpenStoryRS: could not apply equipment scroll: {error}");
                        self.channel_connected = false;
                    }
                }
                self.refresh_inventory_texture();
                return;
            }
            if inventory_type == openstory_protocol::InventoryType::Use {
                self.pending_scroll_slot = None;
            }
        }
        if !self.channel_connected {
            eprintln!("OpenStory: cannot change inventory while disconnected from the field");
            return;
        }
        if item.inventory_type == openstory_protocol::InventoryType::Cash {
            if item.item_id / 10_000 == 523 {
                let Ok(slot) = i16::try_from(item.slot) else {
                    return;
                };
                self.open_owl_search(slot, item.item_id);
                return;
            }
            if item.item_id / 10_000 == 503 {
                if self.pending_merchant_permit.is_some() {
                    self.push_chat_line(
                        "[Merchant] A permit check is already pending.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                }
                let Ok(slot) = i16::try_from(item.slot) else {
                    return;
                };
                let Some(session) = self.session.as_ref() else {
                    return;
                };
                self.pending_merchant_permit = Some((slot, item.item_id));
                if let Err(error) =
                    session.send_packet(openstory_protocol::hired_merchant_request_packet())
                {
                    self.pending_merchant_permit = None;
                    self.channel_connected = false;
                    self.push_chat_line(
                        format!("[Merchant] Could not check the permit: {error}"),
                        [255, 105, 105],
                    );
                } else {
                    self.push_chat_line(
                        "[Merchant] Checking the hired-merchant permit…".to_owned(),
                        [255, 222, 112],
                    );
                }
                return;
            }
            let composer_kind = match item.item_id / 10_000 {
                507 => match (item.item_id / 1_000) % 10 {
                    0 | 1 | 2 | 6 | 7 => Some(CashItemComposeKind::Megaphone),
                    5 => Some(CashItemComposeKind::MapleTv),
                    _ => None,
                },
                517 => Some(CashItemComposeKind::PetNameTag),
                539 => Some(CashItemComposeKind::AvatarMegaphone),
                514 => Some(CashItemComposeKind::PersonalShop),
                _ => None,
            };
            if let Some(kind) = composer_kind {
                let Ok(slot) = i16::try_from(item.slot) else {
                    return;
                };
                self.open_cash_item_composer(slot, item.item_id, kind);
                return;
            }
            if item.item_id / 10_000 == 507 {
                self.push_chat_line(
                    "[Notice] This cash megaphone variant is not supported yet.".to_owned(),
                    [255, 222, 112],
                );
                return;
            }
        }
        let client_tick = self.client_started_at.elapsed().as_millis() as i32;
        if item.inventory_type == openstory_protocol::InventoryType::Setup {
            if item.item_id / 10_000 != 301 {
                return;
            }
            if self
                .map_scene
                .as_ref()
                .is_some_and(MapSceneRuntime::is_locally_seated)
            {
                self.stand_from_local_seat();
                return;
            }
            let chair = match self.load_local_chair(item.item_id) {
                Ok(chair) => chair,
                Err(error) => {
                    eprintln!("OpenStory: could not load chair {}: {error}", item.item_id);
                    LocalChairRuntime::empty()
                }
            };
            if let Some(scene) = self.map_scene.as_mut() {
                scene.local_chair = Some(chair);
                scene.player.horizontal_speed = 0.0;
                scene.player.vertical_speed = 0.0;
                scene.player.on_ground = true;
                scene.last_movement = None;
                scene.attack_stance = None;
                scene.desired_avatar_motion = AvatarMotion::Sit;
                scene.active_avatar_frame =
                    first_avatar_frame(&scene.avatar_frames, AvatarMotion::Sit);
                scene.avatar_elapsed = Duration::ZERO;
                scene.refresh_avatar_draws(&mut self.sprite_draws);
            }
            if let Some(session) = self.session.as_ref()
                && let Err(error) =
                    session.send_packet(openstory_protocol::use_chair_packet(item.item_id))
            {
                eprintln!("OpenStory: could not use chair: {error}");
                self.channel_connected = false;
            }
            return;
        }
        let packet = match item.inventory_type {
            openstory_protocol::InventoryType::Use => {
                let Ok(slot) = i16::try_from(item.slot) else {
                    return;
                };
                if item.item_id / 10_000 == 212 {
                    openstory_protocol::pet_food_packet(slot, item.item_id, client_tick)
                } else {
                    openstory_protocol::use_item_packet(slot, item.item_id, client_tick)
                }
            }
            openstory_protocol::InventoryType::Equip
                if (180..=183).contains(&(item.item_id / 10_000)) =>
            {
                let Some(source) = i16::try_from(item.slot).ok().filter(|slot| *slot > 0) else {
                    return;
                };
                let Some(destination) = pet_equipment_destination(item.item_id, inventory) else {
                    self.push_chat_line(
                        "[Pet] The equipment slot for this item is not known yet.".to_owned(),
                        [255, 210, 120],
                    );
                    return;
                };
                openstory_protocol::move_item_packet(
                    openstory_protocol::InventoryType::Equip,
                    source,
                    -destination,
                    1,
                    client_tick,
                )
            }
            openstory_protocol::InventoryType::Cash if item.item_id / 10_000 == 181 => {
                let Ok(slot) = i16::try_from(item.slot) else {
                    return;
                };
                openstory_protocol::use_item_packet(slot, item.item_id, client_tick)
            }
            openstory_protocol::InventoryType::Cash if item.item_id / 10_000 == 500 => {
                let Ok(slot) = i16::try_from(item.slot) else {
                    return;
                };
                openstory_protocol::spawn_pet_packet(slot)
            }
            openstory_protocol::InventoryType::Cash => {
                let Ok(slot) = i16::try_from(item.slot) else {
                    return;
                };
                openstory_protocol::use_item_packet(slot, item.item_id, client_tick)
            }
            openstory_protocol::InventoryType::Equip => {
                let Ok(source) = i16::try_from(item.slot) else {
                    return;
                };
                let Some(destination) = equip_slot_for_item(item.item_id, item.cash, inventory)
                else {
                    eprintln!("OpenStory: this item has no supported equipment slot");
                    return;
                };
                if let Some(reason) = self.active_character.as_ref().and_then(|character| {
                    equipment_block_reason(
                        item.item_id,
                        character,
                        inventory,
                        &self.character_archive,
                    )
                }) {
                    self.push_chat_line(format!("[Notice] {reason}"), [255, 215, 128]);
                    return;
                }
                openstory_protocol::move_item_packet(
                    openstory_protocol::InventoryType::Equip,
                    source,
                    -destination,
                    1,
                    client_tick,
                )
            }
            openstory_protocol::InventoryType::Equipped => {
                let Ok(equipped_slot) = i16::try_from(item.slot) else {
                    return;
                };
                let Some(source) = equipped_slot.checked_neg() else {
                    return;
                };
                let Some(destination) = first_free_equip_inventory_slot(inventory) else {
                    eprintln!("OpenStory: there is no free slot for this equipment item");
                    return;
                };
                openstory_protocol::move_item_packet(
                    openstory_protocol::InventoryType::Equipped,
                    source,
                    destination,
                    1,
                    client_tick,
                )
            }
            _ => return,
        };
        let plays_item_sound = match item.inventory_type {
            openstory_protocol::InventoryType::Use => item.item_id / 10_000 != 212,
            openstory_protocol::InventoryType::Cash => item.item_id / 10_000 != 500,
            _ => false,
        };
        if plays_item_sound {
            self.audio
                .play_item_sound(&self.sound_archive, item.item_id);
        }
        self.show_local_item_use_effect(item.item_id);
        if let Some(session) = self.session.as_ref() {
            if let Err(error) = session.send_packet(packet) {
                eprintln!("OpenStory: could not change inventory: {error}");
                self.channel_connected = false;
            }
        }
    }

    pub(super) fn equipment_auto_potion_at(
        &self,
        cursor: Point,
    ) -> Option<openstory_protocol::AutoPotionSlot> {
        self.equipment_tab.potion_at(
            cursor.x - EQUIPMENT_INVENTORY_PANEL_X,
            cursor.y - EQUIPMENT_INVENTORY_PANEL_Y,
        )
    }

    fn assign_equipment_auto_potion(
        &mut self,
        slot: openstory_protocol::AutoPotionSlot,
        source_type: openstory_protocol::InventoryType,
        source_slot: u16,
    ) {
        use openstory_protocol::{AutoPotionSlot, InventoryType};
        let item = self.inventory.as_ref().and_then(|inventory| {
            inventory.items.iter().find(|item| {
                source_type == InventoryType::Use
                    && item.inventory_type == source_type
                    && item.slot == source_slot
                    && item.count > 0
            })
        });
        let (label, properties) = match slot {
            AutoPotionSlot::Hp => ("HP", ["hp", "hpR"]),
            AutoPotionSlot::Mp => ("MP", ["mp", "mpR"]),
        };
        let id = item.map(|item| item.item_id).filter(|id| {
            self.item_archive
                .root()
                .resolve(&format!("Consume/{:04}.img/{:08}/spec", id / 10_000, id))
                .ok()
                .flatten()
                .is_some_and(|spec| {
                    properties.iter().any(|property| {
                        integer_property(&spec, property).is_some_and(|amount| amount > 0)
                    })
                })
        });
        if let Some(id) = id {
            self.send_auto_potion_assignment(slot, id);
        } else {
            self.push_chat_line(
                format!("[Pet] Drag a Use item that restores {label} into this slot."),
                [255, 215, 128],
            );
        }
    }

    pub(super) fn send_auto_potion_assignment(
        &mut self,
        slot: openstory_protocol::AutoPotionSlot,
        item_id: i32,
    ) {
        let assigned = match slot {
            openstory_protocol::AutoPotionSlot::Hp => self.auto_hp_potion_id,
            openstory_protocol::AutoPotionSlot::Mp => self.auto_mp_potion_id,
        };
        if assigned == item_id {
            return;
        }
        let Some(session) = self.session.as_ref().filter(|_| self.channel_connected) else {
            return;
        };
        let packet = openstory_protocol::change_auto_potion_packet(slot, item_id);
        if let Err(error) = session.send_packet(packet) {
            eprintln!("OpenStoryRS: could not assign a pet auto-potion: {error}");
            self.channel_connected = false;
            return;
        }
        // v83 does not echo this edit. Login-time updates restore the server binding.
        self.apply_auto_potion(slot, item_id);
    }

    pub(super) fn right_click_equipment_slot(&mut self, cursor: Point) -> bool {
        if let Some(slot) = self.equipment_auto_potion_at(cursor) {
            self.send_auto_potion_assignment(slot, 0);
            return true;
        }
        if self.equipment_tab != EquipmentTab::Pet || self.pending_scroll_slot.is_some() {
            return false;
        }
        let slot = self.inventory.as_ref().and_then(|inventory| {
            let x = cursor.x - EQUIPMENT_INVENTORY_PANEL_X;
            let y = cursor.y - EQUIPMENT_INVENTORY_PANEL_Y;
            self.equipment_tab
                .worn_at(inventory, x, y)
                .map(|item| item.slot)
        });
        if let Some(slot) = slot {
            self.activate_inventory_item(openstory_protocol::InventoryType::Equipped, slot);
            true
        } else {
            false
        }
    }

    pub(super) fn begin_equipment_inventory_pointer(&mut self, cursor: Point) -> bool {
        if !rect_contains(
            SpriteRect {
                x: EQUIPMENT_INVENTORY_PANEL_X,
                y: EQUIPMENT_INVENTORY_PANEL_Y,
                width: EQUIPMENT_INVENTORY_PANEL_WIDTH as f32,
                height: EQUIPMENT_INVENTORY_PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            cursor,
        ) {
            return false;
        }
        let local_x = cursor.x - EQUIPMENT_INVENTORY_PANEL_X;
        let local_y = cursor.y - EQUIPMENT_INVENTORY_PANEL_Y;
        if ui_equipment::CLOSE.contains(local_x, local_y) {
            self.toggle_equipment_inventory();
        } else if let Some(tab) = EquipmentTab::ALL
            .into_iter()
            .find(|tab| tab.bounds().contains(local_x, local_y))
        {
            if self.equipment_tab != tab {
                self.equipment_tab = tab;
                self.refresh_equipment_inventory_texture();
                self.update_inventory_hover();
                self.refresh_inventory_tooltip_texture();
            }
        }
        true
    }

    pub(super) fn begin_inventory_pointer(&mut self, cursor: Point) -> bool {
        if !rect_contains(
            SpriteRect {
                x: INVENTORY_PANEL_X,
                y: INVENTORY_PANEL_Y,
                width: INVENTORY_PANEL_WIDTH as f32,
                height: INVENTORY_PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            cursor,
        ) {
            return false;
        }
        if self.inventory_pending_move.take().is_some() {
            self.refresh_inventory_texture();
        }
        self.inventory_drag_source = None;
        let local_x = cursor.x - INVENTORY_PANEL_X;
        let local_y = cursor.y - INVENTORY_PANEL_Y;
        if ui_foundation::INVENTORY_CLOSE_BUTTON.contains(local_x, local_y) {
            self.register_inventory_equip_click(None);
            self.show_inventory = false;
            self.inventory_hover_slot = None;
            self.pending_scroll_slot = None;
            return true;
        }
        if ui_foundation::INVENTORY_GATHER_BUTTON.contains(local_x, local_y) {
            self.register_inventory_equip_click(None);
            if self.inventory_tab != openstory_protocol::InventoryType::Equipped {
                self.request_inventory_arrange(false);
            }
            return true;
        }
        if ui_foundation::INVENTORY_MESO_BUTTON.contains(local_x, local_y) {
            self.register_inventory_equip_click(None);
            self.request_meso_drop();
            return true;
        }
        let sort = ui_foundation::INVENTORY_SORT_BUTTON;
        if (sort.x..sort.x + sort.width + 1.0).contains(&local_x)
            && (sort.y..sort.y + sort.height).contains(&local_y)
        {
            self.register_inventory_equip_click(None);
            if self.inventory_tab != openstory_protocol::InventoryType::Equipped {
                self.request_inventory_arrange(true);
            }
            return true;
        }
        if let Some(tab) = inventory_tab_at(cursor) {
            self.register_inventory_equip_click(None);
            self.inventory_tab = tab;
            self.inventory_scroll_row = 0;
            self.update_inventory_hover();
            self.refresh_inventory_texture();
            return true;
        }
        if let Some(slot) = inventory_slot_at(cursor, self.inventory_scroll_row) {
            let item_exists = self.inventory.as_ref().is_some_and(|inventory| {
                inventory
                    .items
                    .iter()
                    .any(|item| item.inventory_type == self.inventory_tab && item.slot == slot)
            });
            if item_exists {
                self.inventory_drag_source = Some((self.inventory_tab, slot, cursor));
                self.refresh_inventory_drag_texture();
                self.update_inventory_hover();
                self.refresh_inventory_texture();
            } else {
                self.register_inventory_equip_click(None);
            }
        } else {
            self.register_inventory_equip_click(None);
        }
        true
    }

    pub(super) fn request_inventory_arrange(&mut self, sort: bool) {
        if !self.channel_connected {
            self.push_chat_line(
                "[Notice] Inventory actions require a field connection.".to_owned(),
                [255, 215, 128],
            );
            return;
        }
        let tick = self.client_started_at.elapsed().as_millis() as i32;
        let packet = if sort {
            openstory_protocol::sort_items_packet(self.inventory_tab, tick)
        } else {
            openstory_protocol::gather_items_packet(self.inventory_tab, tick)
        };
        let Some(packet) = packet else {
            return;
        };
        if let Some(session) = self.session.as_ref() {
            if let Err(error) = session.send_packet(packet) {
                eprintln!("OpenStoryRS: could not arrange inventory: {error}");
                self.channel_connected = false;
            }
        }
    }

    pub(super) fn finish_inventory_pointer(&mut self, cursor: Option<Point>) {
        let Some((source_type, source_slot, origin)) = self.inventory_drag_source.take() else {
            return;
        };
        self.inventory_drag_texture = None;
        let awaiting_attachment = self.cash_item_compose.as_ref().is_some_and(|compose| {
            compose.awaiting_attachment && compose.kind.uses_item_attachment(compose.item_id)
        });
        if awaiting_attachment {
            self.register_inventory_equip_click(None);
            let selected_item_id = self.inventory.as_ref().and_then(|inventory| {
                inventory
                    .items
                    .iter()
                    .find(|item| item.inventory_type == source_type && item.slot == source_slot)
                    .map(|item| item.item_id)
            });
            if source_type == openstory_protocol::InventoryType::Equipped {
                self.push_chat_line(
                    "[Item Megaphone] Choose an item from an inventory bag, not a worn slot."
                        .to_owned(),
                    [255, 222, 112],
                );
            } else if let Some(item_id) = selected_item_id {
                let current = self
                    .cash_item_compose
                    .as_ref()
                    .and_then(|compose| compose.attached_item);
                let selected = (source_type, source_slot, item_id);
                let next = if current == Some(selected) {
                    None
                } else {
                    Some(selected)
                };
                if let Some(compose) = self.cash_item_compose.as_mut() {
                    compose.attached_item = next;
                }
                if next.is_some() {
                    self.push_chat_line(
                        format!(
                            "[Item Megaphone] Attached {source_type:?} item {item_id} from slot {source_slot}. Press Enter to send."
                        ),
                        [255, 222, 112],
                    );
                } else {
                    self.push_chat_line(
                        "[Item Megaphone] Attachment cleared. Press Enter to send without an item."
                            .to_owned(),
                        [255, 222, 112],
                    );
                }
            }
            self.refresh_inventory_texture();
            self.refresh_chat_texture();
            return;
        }
        let moved = cursor
            .is_none_or(|cursor| (cursor.x - origin.x).abs() + (cursor.y - origin.y).abs() >= 4.0);
        if moved {
            self.register_inventory_equip_click(None);
        }
        if moved && let Some(drop_cursor) = cursor {
            let screen_cursor = self.cursor.unwrap_or(drop_cursor);
            if self.topmost_dialog_window_at(screen_cursor)
                == Some(DialogWindow::EquipmentInventory)
            {
                if let Some(slot) = self.equipment_auto_potion_at(drop_cursor) {
                    self.assign_equipment_auto_potion(slot, source_type, source_slot);
                } else if source_type == openstory_protocol::InventoryType::Use {
                    let target_slot = self.inventory.as_ref().and_then(|inventory| {
                        let is_scroll = inventory.items.iter().any(|item| {
                            item.inventory_type == openstory_protocol::InventoryType::Use
                                && item.slot == source_slot
                                && item.item_id / 10_000 == 204
                        });
                        if !is_scroll {
                            return None;
                        }
                        self.equipment_tab
                            .worn_at(
                                inventory,
                                drop_cursor.x - EQUIPMENT_INVENTORY_PANEL_X,
                                drop_cursor.y - EQUIPMENT_INVENTORY_PANEL_Y,
                            )
                            .map(|item| item.slot)
                    });
                    if let Some(target_slot) = target_slot {
                        self.send_equipment_scroll(source_slot, target_slot);
                    }
                }
                self.refresh_inventory_texture();
                return;
            }
            if self.show_key_config
                && self.topmost_dialog_window_at(screen_cursor) == Some(DialogWindow::KeyConfig)
            {
                let x = drop_cursor.x - KEY_CONFIG_PANEL_X;
                let y = drop_cursor.y - KEY_CONFIG_PANEL_Y;
                if let Some(key) = self.key_config_drop_target(x, y) {
                    self.assign_inventory_item_to_key(source_type, source_slot, key);
                    self.refresh_inventory_texture();
                    return;
                }
            }
            if self.current_field.is_some()
                && self.topmost_dialog_window_at(screen_cursor).is_none()
                && let Some(quickslot) = self.quickslot_at(screen_cursor)
            {
                self.assign_inventory_item_to_quickslot(quickslot, source_type, source_slot);
                self.refresh_inventory_texture();
                return;
            }
        }
        let destination =
            cursor.and_then(|cursor| inventory_slot_at(cursor, self.inventory_scroll_row));
        let merchant_stock_drop = cursor.is_some_and(|cursor| {
            self.hired_merchant_room.as_ref().is_some_and(|room| {
                room.state.my_slot == 0
                    && rect_contains(
                        SpriteRect {
                            x: HIRED_MERCHANT_PANEL_X + 93.0,
                            y: HIRED_MERCHANT_PANEL_Y + 177.0,
                            width: 201.0,
                            height: 200.0,
                            blend_mode: BlendMode::Alpha,
                        },
                        cursor,
                    )
            })
        });
        if moved && merchant_stock_drop {
            self.begin_hired_merchant_stock(source_type, source_slot);
            self.refresh_inventory_texture();
            return;
        }
        let trade_slot = cursor.and_then(|cursor| {
            let trade_open = self.trade_room.as_ref().is_some_and(|room| {
                room.partner.is_some() && !room.own_confirmed && room.pending_item.is_none()
            });
            if !trade_open {
                return None;
            }
            trade_own_grid_slot(cursor.x - TRADE_PANEL_X, cursor.y - TRADE_PANEL_Y)
        });
        if moved && let Some(trade_slot) = trade_slot {
            self.begin_trade_stock(source_type, source_slot, trade_slot);
            self.refresh_inventory_texture();
            return;
        }
        if !moved || destination == Some(source_slot) {
            let double_click =
                self.register_inventory_equip_click(Some((source_type, source_slot)));
            if source_type != openstory_protocol::InventoryType::Equip || double_click {
                self.activate_inventory_item(source_type, source_slot);
            }
            self.refresh_inventory_texture();
            return;
        }
        let Some(cursor) = cursor else {
            self.request_inventory_drop(source_type, source_slot);
            self.refresh_inventory_texture();
            return;
        };
        let Some(destination) = destination.filter(|_| self.inventory_tab == source_type) else {
            if !rect_contains(
                SpriteRect {
                    x: INVENTORY_PANEL_X,
                    y: INVENTORY_PANEL_Y,
                    width: INVENTORY_PANEL_WIDTH as f32,
                    height: INVENTORY_PANEL_HEIGHT as f32,
                    blend_mode: BlendMode::Alpha,
                },
                cursor,
            ) {
                self.request_inventory_drop(source_type, source_slot);
            }
            self.refresh_inventory_texture();
            return;
        };
        let destination_is_available = self
            .inventory
            .as_ref()
            .is_some_and(|inventory| destination <= inventory_slot_max(inventory, source_type));
        if destination == source_slot
            || source_type == openstory_protocol::InventoryType::Equipped
            || !destination_is_available
        {
            self.refresh_inventory_texture();
            return;
        }
        if !self.channel_connected {
            self.refresh_inventory_texture();
            return;
        }
        let Ok(source) = i16::try_from(source_slot) else {
            return;
        };
        let Ok(destination_position) = i16::try_from(destination) else {
            return;
        };
        let client_tick = self.client_started_at.elapsed().as_millis() as i32;
        let packet = openstory_protocol::move_item_packet(
            source_type,
            source,
            destination_position,
            1,
            client_tick,
        );
        if let Some(session) = self.session.as_ref() {
            match session.send_packet(packet) {
                Ok(()) => {
                    self.inventory_pending_move =
                        Some((source_type, source_slot, destination, Instant::now()));
                }
                Err(error) => {
                    eprintln!("OpenStory: could not move inventory item: {error}");
                    self.channel_connected = false;
                }
            }
        }
        self.refresh_inventory_texture();
    }

    pub(super) fn request_inventory_drop(
        &mut self,
        inventory_type: openstory_protocol::InventoryType,
        slot: u16,
    ) {
        let Some(item) = self.inventory.as_ref().and_then(|inventory| {
            inventory
                .items
                .iter()
                .find(|item| item.inventory_type == inventory_type && item.slot == slot)
                .cloned()
        }) else {
            return;
        };
        let maximum = item_drop_maximum(item.count);
        if maximum == 0 {
            return;
        }
        let info =
            inventory_item_info_node(&self.item_archive, &self.character_archive, item.item_id);
        let cash_item = item.cash
            || info
                .as_ref()
                .is_some_and(|node| inventory_item_property_truthy(node, "cash"));
        let step = if cash_item {
            InventoryDropStep::Blocked
        } else if info
            .as_ref()
            .is_some_and(|node| inventory_item_property_truthy(node, "tradeBlock"))
        {
            InventoryDropStep::ConfirmUntradable
        } else if maximum > 1 {
            InventoryDropStep::Quantity
        } else {
            self.send_inventory_drop(inventory_type, slot, item.item_id, 1);
            return;
        };
        self.inventory_drop_prompt = Some(InventoryDropPrompt {
            action: InventoryQuantityAction::Drop,
            inventory_type,
            slot,
            item_id: item.item_id,
            max_quantity: maximum,
            quantity: QuantityInput::new(Some(maximum)),
            step,
            notice: None,
        });
        self.refresh_inventory_drop_prompt_texture();
    }

    fn request_meso_drop(&mut self) {
        let maximum = self
            .inventory
            .as_ref()
            .map_or(0, |inventory| meso_drop_maximum(inventory.meso));
        if maximum < openstory_protocol::MIN_MESO_DROP {
            self.push_chat_line(
                format!(
                    "[Notice] You need at least {} mesos to drop mesos.",
                    openstory_protocol::MIN_MESO_DROP
                ),
                [255, 215, 128],
            );
            return;
        }
        self.inventory_drop_prompt = Some(InventoryDropPrompt {
            action: InventoryQuantityAction::DropMesos,
            inventory_type: openstory_protocol::InventoryType::Use,
            slot: 0,
            item_id: 0,
            max_quantity: maximum,
            quantity: QuantityInput::new(None),
            step: InventoryDropStep::Quantity,
            notice: None,
        });
        self.refresh_inventory_drop_prompt_texture();
    }

    pub(super) fn resolve_inventory_drop_prompt(&mut self, accepted: bool) {
        let Some(mut prompt) = self.inventory_drop_prompt.take() else {
            return;
        };
        if !accepted || prompt.step == InventoryDropStep::Blocked {
            self.inventory_drop_prompt_texture = None;
            return;
        }
        match prompt.step {
            InventoryDropStep::ConfirmUntradable if prompt.max_quantity > 1 => {
                prompt.step = InventoryDropStep::Quantity;
                prompt.notice = None;
                self.inventory_drop_prompt = Some(prompt);
                self.refresh_inventory_drop_prompt_texture();
            }
            InventoryDropStep::ConfirmUntradable => {
                self.inventory_drop_prompt_texture = None;
                self.send_inventory_drop(prompt.inventory_type, prompt.slot, prompt.item_id, 1);
            }
            InventoryDropStep::Quantity => {
                let quantity = prompt.quantity.value();
                let minimum = prompt.minimum();
                let Some(quantity) = quantity
                    .filter(|quantity| *quantity >= minimum && *quantity <= prompt.max_quantity)
                else {
                    prompt.notice = Some(if prompt.action == InventoryQuantityAction::DropMesos {
                        format!(
                            "Enter an amount from {minimum} to {} mesos.",
                            prompt.max_quantity
                        )
                    } else {
                        format!("Enter a quantity from 1 to {}.", prompt.max_quantity)
                    });
                    self.inventory_drop_prompt = Some(prompt);
                    self.refresh_inventory_drop_prompt_texture();
                    return;
                };
                self.inventory_drop_prompt_texture = None;
                match prompt.action {
                    InventoryQuantityAction::Drop => self.send_inventory_drop(
                        prompt.inventory_type,
                        prompt.slot,
                        prompt.item_id,
                        quantity,
                    ),
                    InventoryQuantityAction::DropMesos => self.send_meso_drop(i32::from(quantity)),
                    InventoryQuantityAction::StorageStore => {
                        let item = self.inventory.as_ref().and_then(|inventory| {
                            inventory
                                .items
                                .iter()
                                .find(|item| {
                                    item.inventory_type == prompt.inventory_type
                                        && item.slot == prompt.slot
                                        && item.item_id == prompt.item_id
                                })
                                .cloned()
                        });
                        if let Some(item) = item {
                            self.request_storage_store(&item, quantity);
                        }
                    }
                    InventoryQuantityAction::StorageTake => {
                        let item = self.storage.as_ref().and_then(|storage| {
                            storage
                                .state
                                .items
                                .iter()
                                .find(|item| {
                                    item.inventory_type == prompt.inventory_type
                                        && item.slot == prompt.slot
                                        && item.item_id == prompt.item_id
                                })
                                .cloned()
                        });
                        if let Some(item) = item {
                            self.request_storage_take_out(&item, quantity);
                        }
                    }
                }
            }
            InventoryDropStep::Blocked => unreachable!(),
        }
    }

    pub(super) fn send_inventory_drop(
        &mut self,
        inventory_type: openstory_protocol::InventoryType,
        slot: u16,
        item_id: i32,
        quantity: u16,
    ) {
        if !self.channel_connected {
            eprintln!("OpenStoryRS: cannot drop inventory items while disconnected from the field");
            return;
        }
        let Some(item) = self.inventory.as_ref().and_then(|inventory| {
            inventory
                .items
                .iter()
                .find(|item| item.inventory_type == inventory_type && item.slot == slot)
        }) else {
            return;
        };
        if item.item_id != item_id || item.count < quantity || item.cash {
            return;
        }
        let Ok(source) = i16::try_from(slot) else {
            return;
        };
        let source = if inventory_type == openstory_protocol::InventoryType::Equipped {
            let Some(source) = source.checked_neg() else {
                return;
            };
            source
        } else {
            source
        };
        let Ok(quantity) = i16::try_from(quantity) else {
            return;
        };
        let packet = openstory_protocol::move_item_packet(
            inventory_type,
            source,
            0,
            quantity,
            self.client_started_at.elapsed().as_millis() as i32,
        );
        if let Some(session) = self.session.as_ref() {
            if let Err(error) = session.send_packet(packet) {
                eprintln!("OpenStoryRS: could not drop inventory item: {error}");
                self.channel_connected = false;
            }
        }
    }

    fn send_meso_drop(&mut self, amount: i32) {
        let available = self
            .inventory
            .as_ref()
            .map_or(0, |inventory| inventory.meso);
        if !self.channel_connected || i64::from(amount) > available {
            self.push_chat_line(
                "[Notice] Mesos can only be dropped while connected and within your balance."
                    .to_owned(),
                [255, 215, 128],
            );
            return;
        }
        let Some(packet) = openstory_protocol::drop_meso_packet(
            self.client_started_at.elapsed().as_millis() as i32,
            amount,
        ) else {
            return;
        };
        if let Some(session) = self.session.as_ref()
            && let Err(error) = session.send_packet(packet)
        {
            eprintln!("OpenStoryRS: could not drop mesos: {error}");
            self.channel_connected = false;
        }
    }

    pub(super) fn click_inventory_drop_prompt(&mut self, cursor: Point) {
        let Some(prompt) = self.inventory_drop_prompt.as_mut() else {
            return;
        };
        let Point { x, y } = quantity_window_local(cursor, inventory_quantity_rect());
        let blocked = prompt.step == InventoryDropStep::Blocked;
        let (ok, cancel) = quantity_window_buttons(blocked);
        if ok.contains(x, y) {
            self.resolve_inventory_drop_prompt(!blocked);
        } else if QUANTITY_CLOSE.contains(x, y) || (!blocked && cancel.contains(x, y)) {
            self.resolve_inventory_drop_prompt(false);
        } else if prompt.step == InventoryDropStep::Quantity {
            let min = prompt.minimum();
            if prompt.quantity.press(x, y, min, prompt.max_quantity) {
                prompt.notice = None;
                self.refresh_inventory_drop_prompt_texture();
            }
        }
    }

    pub(super) fn refresh_inventory_drop_prompt_texture(&mut self) {
        let Some(prompt) = self.inventory_drop_prompt.as_mut() else {
            self.inventory_drop_prompt_texture = None;
            return;
        };
        let matching_count = |items: &[openstory_protocol::InventoryItem]| {
            items
                .iter()
                .find(|item| {
                    item.inventory_type == prompt.inventory_type
                        && item.slot == prompt.slot
                        && item.item_id == prompt.item_id
                })
                .map(|item| item.count)
        };
        let maximum = match prompt.action {
            InventoryQuantityAction::DropMesos => self
                .inventory
                .as_ref()
                .map(|inventory| meso_drop_maximum(inventory.meso)),
            InventoryQuantityAction::Drop | InventoryQuantityAction::StorageStore => self
                .inventory
                .as_ref()
                .and_then(|inventory| matching_count(&inventory.items))
                .map(|count| {
                    if prompt.action == InventoryQuantityAction::Drop {
                        item_drop_maximum(count)
                    } else {
                        count.max(1)
                    }
                }),
            InventoryQuantityAction::StorageTake => self
                .storage
                .as_ref()
                .and_then(|storage| matching_count(&storage.state.items))
                .map(|count| count.max(1)),
        };
        let Some(maximum) = maximum.filter(|&maximum| maximum >= prompt.minimum()) else {
            let drop_mesos = prompt.action == InventoryQuantityAction::DropMesos;
            self.resolve_inventory_drop_prompt(false);
            if drop_mesos {
                self.push_chat_line(
                    format!(
                        "[Notice] You need at least {} mesos to drop mesos.",
                        openstory_protocol::MIN_MESO_DROP
                    ),
                    [255, 215, 128],
                );
            }
            return;
        };
        if prompt.max_quantity != maximum {
            prompt.max_quantity = maximum;
            prompt.quantity.clamp_to_max(maximum);
            prompt.notice = None;
        }
        let icon = if prompt.action == InventoryQuantityAction::DropMesos {
            None
        } else {
            inventory_item_icon(
                prompt.item_id,
                &self.item_archive,
                &self.character_archive,
                &mut self.inventory_icon_cache,
            )
        };
        let pixels = match render_inventory_drop_prompt(
            &self.legacy_ui_archive,
            &self.font,
            prompt,
            icon.as_ref(),
            &self.string_archive,
            &mut self.item_name_cache,
        ) {
            Ok(pixels) => pixels,
            Err(error) => {
                eprintln!("OpenStoryRS: could not compose the inventory quantity prompt: {error}");
                return;
            }
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        if let Err(error) = update_quantity_window_texture(
            renderer,
            &mut self.inventory_drop_prompt_texture,
            &pixels,
        ) {
            eprintln!("OpenStoryRS: could not render the inventory quantity prompt: {error}");
        }
    }

    pub(super) fn scroll_inventory(&mut self, amount: f32) {
        if amount == 0.0 {
            return;
        }
        let Some(inventory) = self.inventory.as_ref() else {
            return;
        };
        let maximum_slot = inventory_slot_max(inventory, self.inventory_tab);
        let maximum_row = maximum_slot
            .div_ceil(INVENTORY_SLOT_COLUMNS as u16)
            .saturating_sub(INVENTORY_SLOT_ROWS as u16);
        self.inventory_scroll_row = if amount > 0.0 {
            self.inventory_scroll_row.saturating_sub(1)
        } else {
            self.inventory_scroll_row.saturating_add(1).min(maximum_row)
        };
        self.update_inventory_hover();
        self.refresh_inventory_texture();
    }

    pub(super) fn sync_equipped_look(&mut self) {
        let look_changed = self.update_equipped_look_data();
        let Some(look) = self
            .active_character
            .as_ref()
            .map(|character| character.look.clone())
        else {
            return;
        };
        if self.inventory.is_none() {
            return;
        }
        let ring_effect_id = equipped_ring_effect_id(&look, &self.effect_archive);
        if !look_changed {
            if ring_effect_id > 0
                && self
                    .map_scene
                    .as_ref()
                    .is_some_and(|scene| scene.local_item_aura.is_none())
            {
                if let Some(character_id) =
                    self.active_character.as_ref().map(|character| character.id)
                {
                    self.set_character_item_effect(character_id, ring_effect_id);
                }
            }
            return;
        }
        if self.map_scene.is_some() {
            if let Err(error) = self.refresh_local_avatar_appearance(&look) {
                eprintln!("OpenStory: could not update equipped appearance: {error}");
            }
        }
        if let Some(character_id) = self.active_character.as_ref().map(|character| character.id) {
            self.set_character_item_effect(character_id, ring_effect_id);
        }
    }

    pub(super) fn update_equipped_look_data(&mut self) -> bool {
        let Some(character) = self.active_character.as_ref() else {
            return false;
        };
        let Some(inventory) = self.inventory.as_ref() else {
            return false;
        };
        let mut equips = Vec::new();
        let mut weapon_id = 0;
        for item in inventory.equipped() {
            if (180..=183).contains(&(item.item_id / 10_000)) {
                continue;
            }
            let Ok(positive_slot) = i8::try_from(item.slot) else {
                continue;
            };
            let Some(slot) = positive_slot.checked_neg() else {
                continue;
            };
            equips.push((slot, item.item_id));
            if item.slot == 11 {
                weapon_id = item.item_id;
            }
        }
        equips.sort_unstable_by_key(|(slot, _)| slot.unsigned_abs());
        let look_changed = character.look.equips != equips
            || !character.look.masked_equips.is_empty()
            || character.look.weapon_id != weapon_id;
        if look_changed && let Some(character) = self.active_character.as_mut() {
            character.look.equips = equips;
            character.look.masked_equips.clear();
            character.look.weapon_id = weapon_id;
        }
        look_changed
    }

    pub(super) fn refresh_local_avatar_appearance(
        &mut self,
        look: &openstory_protocol::CharacterLook,
    ) -> Result<(), String> {
        let mut sprites = Vec::new();
        let assets = append_character_animations(
            &mut sprites,
            &self.character_archive,
            look,
            512.0,
            GAMEPLAY_CAMERA_CENTER_Y,
            &mut self.remote_art_cache,
        )?;
        let name = self
            .active_character
            .as_ref()
            .map_or("", |character| character.stats.name.as_str());
        let name_label_index = append_map_name_tag_sprite(
            &mut sprites,
            &self.font,
            &self.ui_archive,
            Some(&self.character_archive),
            Some(look),
            name,
            512.0,
            GAMEPLAY_CAMERA_CENTER_Y,
            MapNameTagKind::Character,
        );
        let draws = self.upload_dynamic_sprites(sprites)?;
        let frames = uploaded_avatar_frames_runtime(assets, &draws);
        let name_label = name_label_index.and_then(|index| draws.get(index).copied());
        if let Some(scene) = self.map_scene.as_mut() {
            scene.avatar_frames = frames;
            scene.local_name_label = name_label;
            scene.attack_stance = None;
            scene.desired_avatar_motion = if scene.is_locally_seated() {
                AvatarMotion::Sit
            } else {
                AvatarMotion::Stand
            };
            scene.active_avatar_frame =
                first_avatar_frame(&scene.avatar_frames, scene.desired_avatar_motion);
            scene.avatar_elapsed = Duration::ZERO;
        }
        self.refresh_local_mount(look);
        Ok(())
    }
}
