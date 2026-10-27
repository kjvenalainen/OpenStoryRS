use super::*;

impl ClientApp {
    pub(super) fn apply_quest_update(&mut self, update: openstory_protocol::QuestUpdate) {
        self.quest_log.apply(update);
        self.refresh_npc_quest_marks();
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

    pub(super) fn accept_selected_quest(&mut self) {
        let Some((quest_id, npc_id)) = self.available_quests.get(self.quest_log_selected).copied()
        else {
            return;
        };
        if !quest_is_available_for_player(
            quest_id,
            npc_id,
            &self.quest_log,
            &self.quest_archive,
            match self.active_character.as_ref() {
                Some(character) => character,
                None => return,
            },
        ) {
            self.refresh_quest_log_texture();
            return;
        }
        let scripted = quest_requires_script(&self.quest_archive, quest_id, 0, "startscript");
        let packet = openstory_protocol::start_quest_packet(quest_id, npc_id, scripted);
        if let Some(session) = self.session.as_ref() {
            if let Err(error) = session.send_packet(packet) {
                eprintln!("OpenStoryRS: could not start quest {quest_id}: {error}");
                self.channel_connected = false;
            }
        }
    }

    pub(super) fn complete_selected_quest(&mut self) {
        let Some(quest_id) = self
            .quest_log
            .started
            .get(self.quest_log_selected)
            .map(|quest| quest.id)
        else {
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
        if !quest_is_completable(quest, &self.quest_archive, self.inventory.as_ref()) {
            self.refresh_quest_log_texture();
            return;
        }
        let npc_id = self
            .quest_archive
            .root()
            .resolve(&format!("Check.img/{quest_id}/1"))
            .ok()
            .flatten()
            .and_then(|node| quest_node_integer(&node, "npc"));
        let Some(npc_id) = npc_id.filter(|id| *id > 0) else {
            return;
        };
        let reward_choices = self
            .active_character
            .as_ref()
            .map_or_else(Vec::new, |character| {
                quest_reward_choices(
                    quest_id,
                    &self.quest_archive,
                    &self.string_archive,
                    character,
                    &mut self.item_name_cache,
                )
            });
        let reward_selection = if reward_choices.is_empty() {
            None
        } else {
            let Some(choice) = reward_choices.get(self.quest_reward_selected) else {
                self.refresh_quest_log_texture();
                return;
            };
            Some(choice.selection)
        };
        let packet = openstory_protocol::complete_quest_packet(
            quest_id,
            npc_id,
            reward_selection,
            quest_requires_script(&self.quest_archive, quest_id, 1, "endscript"),
        );
        if let Some(session) = self.session.as_ref() {
            if let Err(error) = session.send_packet(packet) {
                eprintln!("OpenStoryRS: could not complete quest {quest_id}: {error}");
                self.channel_connected = false;
            }
        }
    }

    pub(super) fn change_selected_quest_reward(&mut self, direction: isize) {
        if self.quest_log_tab != QuestLogTab::Active {
            return;
        }
        let Some(quest_id) = self
            .quest_log
            .started
            .get(self.quest_log_selected)
            .map(|quest| quest.id)
        else {
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

    pub(super) fn click_quest_log(&mut self, cursor: Point) -> bool {
        if !self.show_quest_log {
            return false;
        }
        let has_detail =
            quest_ids_for_tab(&self.quest_log, &self.available_quests, self.quest_log_tab)
                .get(self.quest_log_selected)
                .is_some();
        let (panel_width, panel_height) = if has_detail || self.quest_forfeit_confirmation.is_some()
        {
            (QUEST_LOG_PANEL_WIDTH as f32, QUEST_LOG_PANEL_HEIGHT as f32)
        } else {
            (296.0, 289.0)
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
        if (5.0..20.0).contains(&y) && (256.0..289.0).contains(&x) {
            self.show_quest_log = false;
            return true;
        }
        if (21.0..48.0).contains(&y) {
            let tab = if (10.0..76.0).contains(&x) {
                Some(QuestLogTab::Available)
            } else if (78.0..148.0).contains(&x) {
                Some(QuestLogTab::Active)
            } else if (150.0..220.0).contains(&x) {
                Some(QuestLogTab::Completed)
            } else {
                None
            };
            if let Some(tab) = tab {
                self.quest_log_tab = tab;
                self.quest_log_selected = 0;
                self.quest_log_scroll = 0;
                self.quest_reward_selected = 0;
                self.refresh_quest_log_texture();
            }
            return true;
        }
        if (50.0..254.0).contains(&y) && (8.0..244.0).contains(&x) {
            let row = ((y - 54.0).max(0.0) / 25.0).floor() as usize;
            let index = self.quest_log_scroll.saturating_add(row);
            if index
                < quest_ids_for_tab(&self.quest_log, &self.available_quests, self.quest_log_tab)
                    .len()
            {
                self.quest_log_selected = index;
                self.quest_reward_selected = 0;
                self.refresh_quest_log_texture();
            }
            return true;
        }
        if self.quest_log_tab == QuestLogTab::Active
            && (300.0..335.0).contains(&y)
            && (QUEST_LOG_DETAIL_X as f32 + 18.0..QUEST_LOG_DETAIL_X as f32 + 285.0).contains(&x)
        {
            if x < QUEST_LOG_DETAIL_X as f32 + 56.0 {
                self.change_selected_quest_reward(-1);
            } else if x > QUEST_LOG_DETAIL_X as f32 + 258.0 {
                self.change_selected_quest_reward(1);
            }
            return true;
        }
        if (365.0..389.0).contains(&y) {
            match self.quest_log_tab {
                QuestLogTab::Available
                    if (QUEST_LOG_DETAIL_X as f32 + 36.0..QUEST_LOG_DETAIL_X as f32 + 110.0)
                        .contains(&x) =>
                {
                    self.accept_selected_quest();
                }
                QuestLogTab::Active
                    if (QUEST_LOG_DETAIL_X as f32 + 36.0..QUEST_LOG_DETAIL_X as f32 + 110.0)
                        .contains(&x) =>
                {
                    self.complete_selected_quest();
                }
                QuestLogTab::Active
                    if (QUEST_LOG_DETAIL_X as f32 + 120.0..QUEST_LOG_DETAIL_X as f32 + 190.0)
                        .contains(&x) =>
                {
                    if let Some(quest_id) = self
                        .quest_log
                        .started
                        .get(self.quest_log_selected)
                        .map(|quest| quest.id)
                    {
                        self.quest_forfeit_confirmation = Some(quest_id);
                        self.refresh_quest_log_texture();
                    }
                }
                _ => {}
            }
            return true;
        }
        true
    }

    pub(super) fn begin_skill_book_pointer(&mut self, cursor: Point) -> bool {
        const PANEL_X: f32 = 586.0;
        const PANEL_Y: f32 = 128.0;
        if !rect_contains(
            SpriteRect {
                x: PANEL_X,
                y: PANEL_Y,
                width: 430.0,
                height: 500.0,
                blend_mode: BlendMode::Alpha,
            },
            cursor,
        ) {
            return false;
        }
        let local_x = cursor.x - PANEL_X;
        let local_y = cursor.y - PANEL_Y;
        if (397.0..425.0).contains(&local_x) && (7.0..35.0).contains(&local_y) {
            self.show_skill_book = false;
            return true;
        }
        if (42.0..72.0).contains(&local_y) {
            let tab = ((local_x - 10.0) / 82.0).floor() as i32;
            if (0..5).contains(&tab) {
                let tab = tab as u8;
                if self.active_character.as_ref().is_some_and(|character| {
                    skill_subjob_for_tab(character.stats.job, tab).is_some()
                }) {
                    self.skill_book_tab = tab;
                    self.skill_book_scroll_row = 0;
                    self.refresh_skill_book_entries();
                    self.refresh_skill_book_texture();
                }
            }
            return true;
        }
        if (72.0..91.0).contains(&local_y) && (292.0..418.0).contains(&local_x) {
            self.open_skill_macro_editor();
            return true;
        }
        if (91.0..439.0).contains(&local_y) {
            let row = ((local_y - 91.0) / 34.0).floor() as usize;
            let Some(entry) = self
                .skill_book_entries
                .get(self.skill_book_scroll_row.saturating_add(row))
                .cloned()
            else {
                return true;
            };
            if (386.0..424.0).contains(&local_x) {
                self.spend_skill_point(&entry);
            } else if (333.0..383.0).contains(&local_x) {
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
        let points = character
            .stats
            .skill_points_by_job
            .iter()
            .find(|(tab, _)| *tab == self.skill_book_tab)
            .map_or(character.stats.skill_points, |(_, points)| {
                u16::from(*points)
            });
        let cap = if skill.master_level > 0 {
            skill.max_level.min(skill.master_level)
        } else {
            skill.max_level
        };
        if points == 0
            || skill.level >= cap
            || skill.requirements.iter().any(|(id, required)| {
                self.skills.get(id).map_or(0, |learned| learned.level) < *required
            })
        {
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
            Some(expires_at) if expires_at > now => {
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
                    vec![CharacterSkillEffectAnimation { z: 0, frames }]
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
            openstory_protocol::InventoryType::Equip if item.item_id / 10_000 == 181 => {
                let Ok(slot) = i16::try_from(item.slot) else {
                    return;
                };
                openstory_protocol::move_item_packet(
                    openstory_protocol::InventoryType::Equip,
                    slot,
                    0,
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
                let Some(destination) = equip_slot_for_item(item.item_id, inventory) else {
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
        if (139.0..171.0).contains(&local_x) && (5.0..20.0).contains(&local_y) {
            self.show_equipment_inventory = false;
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
        self.inventory_drag_source = None;
        let local_x = cursor.x - INVENTORY_PANEL_X;
        let local_y = cursor.y - INVENTORY_PANEL_Y;
        if (139.0..171.0).contains(&local_x) && (5.0..20.0).contains(&local_y) {
            self.show_inventory = false;
            self.inventory_hover_slot = None;
            self.pending_scroll_slot = None;
            return true;
        }
        if (142.0..155.0).contains(&local_x) && (269.0..284.0).contains(&local_y) {
            if self.inventory_tab != openstory_protocol::InventoryType::Equipped {
                self.request_inventory_arrange(false);
            }
            return true;
        }
        if (160.0..173.0).contains(&local_x) && (269.0..284.0).contains(&local_y) {
            if self.inventory_tab != openstory_protocol::InventoryType::Equipped {
                self.request_inventory_arrange(true);
            }
            return true;
        }
        if let Some(tab) = inventory_tab_at(cursor) {
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
                self.update_inventory_hover();
                self.refresh_inventory_texture();
            }
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

    pub(super) fn begin_pet_equipment_pointer(&mut self, cursor: Point) -> bool {
        if !rect_contains(
            SpriteRect {
                x: 618.0,
                y: 128.0,
                width: 390.0,
                height: 480.0,
                blend_mode: BlendMode::Alpha,
            },
            cursor,
        ) {
            return false;
        }
        if (976.0..1002.0).contains(&cursor.x) && (136.0..162.0).contains(&cursor.y) {
            self.show_pet_equipment = false;
            return true;
        }
        if !(838.0..998.0).contains(&cursor.x) || !(348.0..593.0).contains(&cursor.y) {
            return true;
        }
        let index = ((cursor.y - 348.0) / 49.0).floor() as usize;
        let selected = self.inventory.as_ref().and_then(|inventory| {
            pet_equipment_items(inventory)
                .get(self.pet_equipment_scroll_row.saturating_add(index))
                .map(|item| (item.inventory_type, item.slot))
        });
        if let Some((inventory_type, slot)) = selected {
            self.activate_inventory_item(inventory_type, slot);
            self.refresh_pet_equipment_texture();
        }
        true
    }

    pub(super) fn finish_inventory_pointer(&mut self, cursor: Option<Point>) {
        let Some((source_type, source_slot, origin)) = self.inventory_drag_source.take() else {
            return;
        };
        let awaiting_attachment = self.cash_item_compose.as_ref().is_some_and(|compose| {
            compose.awaiting_attachment && compose.kind.uses_item_attachment(compose.item_id)
        });
        if awaiting_attachment {
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
        if moved
            && let Some(cursor) = cursor
            && let Some(quickslot) = self.quickslot_at(cursor)
        {
            self.assign_inventory_item_to_quickslot(quickslot, source_type, source_slot);
            self.refresh_inventory_texture();
            return;
        }
        let destination =
            cursor.and_then(|cursor| inventory_slot_at(cursor, self.inventory_scroll_row));
        let merchant_stock_drop = cursor.is_some_and(|cursor| {
            self.hired_merchant_room.as_ref().is_some_and(|room| {
                room.state.my_slot == 0
                    && rect_contains(
                        SpriteRect {
                            x: HIRED_MERCHANT_PANEL_X + 14.0,
                            y: HIRED_MERCHANT_PANEL_Y + 142.0,
                            width: 354.0,
                            height: 336.0,
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
            let local_x = cursor.x - (TRADE_PANEL_X + 174.0);
            let local_y = cursor.y - (TRADE_PANEL_Y + 136.0);
            if !(0.0..138.0).contains(&local_x) || !(0.0..138.0).contains(&local_y) {
                return None;
            }
            let column = (local_x / 46.0).floor() as u8;
            let row = (local_y / 46.0).floor() as u8;
            if local_x % 46.0 >= 42.0 || local_y % 46.0 >= 42.0 {
                return None;
            }
            Some(row * 3 + column)
        });
        if moved && let Some(trade_slot) = trade_slot {
            self.begin_trade_stock(source_type, source_slot, trade_slot);
            self.refresh_inventory_texture();
            return;
        }
        if !moved || destination == Some(source_slot) {
            self.activate_inventory_item(source_type, source_slot);
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
        let Ok(destination) = i16::try_from(destination) else {
            return;
        };
        let client_tick = self.client_started_at.elapsed().as_millis() as i32;
        let packet =
            openstory_protocol::move_item_packet(source_type, source, destination, 1, client_tick);
        if let Some(session) = self.session.as_ref() {
            if let Err(error) = session.send_packet(packet) {
                eprintln!("OpenStory: could not move inventory item: {error}");
                self.channel_connected = false;
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
        } else if item.count > 1 {
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
            max_quantity: item.count.max(1),
            quantity: item.count.max(1).to_string(),
            step,
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
                let quantity = prompt.quantity.trim().parse::<u16>().ok();
                let Some(quantity) =
                    quantity.filter(|quantity| *quantity > 0 && *quantity <= prompt.max_quantity)
                else {
                    prompt.notice = Some(format!(
                        "Enter a quantity from 1 to {}.",
                        prompt.max_quantity
                    ));
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

    pub(super) fn click_inventory_drop_prompt(&mut self, cursor: Point) {
        let Some(prompt) = self.inventory_drop_prompt.as_ref() else {
            return;
        };
        let inside = (312.0..712.0).contains(&cursor.x) && (240.0..528.0).contains(&cursor.y);
        if !inside || !(472.0..512.0).contains(&cursor.y) {
            return;
        }
        if prompt.step == InventoryDropStep::Blocked {
            if (452.0..572.0).contains(&cursor.x) {
                self.resolve_inventory_drop_prompt(false);
            }
        } else if (365.0..485.0).contains(&cursor.x) {
            self.resolve_inventory_drop_prompt(true);
        } else if (539.0..659.0).contains(&cursor.x) {
            self.resolve_inventory_drop_prompt(false);
        }
    }

    pub(super) fn refresh_inventory_drop_prompt_texture(&mut self) {
        let Some(prompt) = self.inventory_drop_prompt.as_ref() else {
            self.inventory_drop_prompt_texture = None;
            return;
        };
        let item = match prompt.action {
            InventoryQuantityAction::Drop | InventoryQuantityAction::StorageStore => {
                self.inventory.as_ref().and_then(|inventory| {
                    inventory
                        .items
                        .iter()
                        .find(|item| {
                            item.inventory_type == prompt.inventory_type
                                && item.slot == prompt.slot
                                && item.item_id == prompt.item_id
                        })
                        .cloned()
                })
            }
            InventoryQuantityAction::StorageTake => self.storage.as_ref().and_then(|storage| {
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
            }),
        };
        let Some(item) = item.filter(|item| item.item_id == prompt.item_id) else {
            self.inventory_drop_prompt = None;
            self.inventory_drop_prompt_texture = None;
            return;
        };
        if let Some(prompt) = self.inventory_drop_prompt.as_mut() {
            prompt.max_quantity = item.count.max(1);
            if prompt
                .quantity
                .parse::<u16>()
                .is_ok_and(|quantity| quantity > prompt.max_quantity)
            {
                prompt.quantity = prompt.max_quantity.to_string();
            }
        }
        let Some(prompt) = self.inventory_drop_prompt.as_ref() else {
            self.inventory_drop_prompt_texture = None;
            return;
        };
        let pixels = render_inventory_drop_prompt(
            &self.font,
            prompt,
            &self.string_archive,
            &mut self.item_name_cache,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.inventory_drop_prompt_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer.upload_bgra(1024, 768, &pixels).map(Some)
        };
        match result {
            Ok(Some(texture)) => self.inventory_drop_prompt_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not render item-drop prompt: {error}"),
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
        let Some(mut look) = self
            .active_character
            .as_ref()
            .map(|character| character.look.clone())
        else {
            return;
        };
        let Some(inventory) = self.inventory.as_ref() else {
            return;
        };
        let mut equips = Vec::new();
        let mut weapon_id = 0;
        for item in inventory.equipped() {
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
        equips.sort_unstable();
        let look_changed = look.equips != equips || look.weapon_id != weapon_id;
        let mut equipped_look = look.clone();
        equipped_look.equips = equips.clone();
        let ring_effect_id = equipped_ring_effect_id(&equipped_look, &self.effect_archive);
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
        look.equips = equips;
        look.weapon_id = weapon_id;
        if let Some(character) = self.active_character.as_mut() {
            character.look.equips = look.equips.clone();
            character.look.weapon_id = look.weapon_id;
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
        let draws = self.upload_dynamic_sprites(sprites)?;
        let frames = uploaded_avatar_frames_runtime(assets, &draws);
        if let Some(scene) = self.map_scene.as_mut() {
            scene.avatar_frames = frames;
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
