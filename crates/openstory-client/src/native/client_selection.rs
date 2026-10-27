use super::*;

impl ClientApp {
    pub(super) fn show_world_selection(&mut self, worlds: Vec<openstory_protocol::WorldInfo>) {
        self.worlds = worlds;
        self.selected_world = self
            .preferred_world
            .filter(|id| {
                self.worlds
                    .iter()
                    .any(|world| world.id == *id && !world.channels.is_empty())
            })
            .or_else(|| {
                self.worlds
                    .iter()
                    .find(|world| !world.channels.is_empty())
                    .map(|world| world.id)
            });
        self.selected_channel = 0;
        self.show_login_fields = false;
        self.refresh_world_scene();
    }

    pub(super) fn refresh_world_scene(&mut self) {
        let scene = load_world_selection_scene(
            &self.map_archive,
            &self.legacy_ui_archive,
            &self.worlds,
            self.selected_world,
            self.selected_channel,
        );
        match scene.and_then(|scene| self.replace_scene(scene)) {
            Ok(()) => {}
            Err(error) => eprintln!("OpenStory: could not display world selection: {error}"),
        }
    }

    pub(super) fn select_world(&mut self, world_id: u8) {
        if self
            .worlds
            .iter()
            .any(|world| world.id == world_id && !world.channels.is_empty())
            && self.selected_world != Some(world_id)
        {
            self.selected_world = Some(world_id);
            self.selected_channel = 0;
            self.refresh_world_scene();
        }
    }

    pub(super) fn select_channel(&mut self, channel_id: u8) {
        let Some(world) = self
            .selected_world
            .and_then(|id| self.worlds.iter().find(|world| world.id == id))
        else {
            return;
        };
        if usize::from(channel_id) < world.channels.len() && self.selected_channel != channel_id {
            self.selected_channel = channel_id;
            self.refresh_world_scene();
        }
    }

    pub(super) fn change_character_page(&mut self, direction: i8) {
        let Some(characters) = self.character_list.as_ref() else {
            return;
        };
        let character_count = characters.characters.len();
        let page_count = character_count.div_ceil(CHARACTER_PAGE_SIZE);
        if page_count <= 1 {
            return;
        }
        self.character_page = if direction < 0 {
            if self.character_page == 0 {
                page_count - 1
            } else {
                self.character_page - 1
            }
        } else if self.character_page + 1 == page_count {
            0
        } else {
            self.character_page + 1
        };
        let page_start = self.character_page * CHARACTER_PAGE_SIZE;
        self.selected_character_index = if direction < 0 {
            (page_start + CHARACTER_PAGE_SIZE).min(character_count) - 1
        } else {
            page_start
        };
        self.refresh_character_scene();
    }

    pub(super) fn focus_character(&mut self, index: usize) {
        let Some(characters) = self.character_list.as_ref() else {
            return;
        };
        if index >= characters.characters.len() {
            return;
        }
        self.selected_character_index = index;
        self.character_page = index / CHARACTER_PAGE_SIZE;
        self.refresh_character_scene();
    }

    pub(super) fn click_character_slot(&mut self, index: usize) {
        self.focus_character(index);
    }

    pub(super) fn character_action_enabled(&self, action: usize) -> bool {
        let Some(characters) = self.character_list.as_ref() else {
            return false;
        };
        match action {
            0 | 2 => self.selected_character_index < characters.characters.len(),
            1 => {
                characters.characters.len() < usize::try_from(characters.slots.max(0)).unwrap_or(0)
                    && characters.characters.len() < 64
            }
            _ => false,
        }
    }

    pub(super) fn advance_character_action_focus(&mut self) {
        let enabled = (0..3)
            .filter(|action| self.character_action_enabled(*action))
            .collect::<Vec<_>>();
        let next = self
            .character_action_focus
            .and_then(|current| enabled.iter().position(|action| *action == current))
            .map_or(0, |index| index + 1);
        self.character_action_focus = enabled.get(next).copied();
        self.refresh_character_scene();
    }

    pub(super) fn activate_focused_character_action(&mut self) {
        let Some(action) = self.character_action_focus else {
            self.select_character(self.selected_character_index);
            return;
        };
        if !self.character_action_enabled(action) {
            return;
        }
        match action {
            0 => self.select_character(self.selected_character_index),
            1 => self.begin_character_creation(),
            2 => self.delete_character(self.selected_character_index),
            _ => {}
        }
    }

    pub(super) fn click_character_selection(&mut self, cursor: Point) -> bool {
        if self.character_list.is_none()
            || self.character_creation.is_some()
            || self.show_login_fields
        {
            return false;
        }

        let action = self
            .login_actions
            .iter()
            .rev()
            .find(|hit| rect_contains(hit.rect, cursor))
            .map(|hit| hit.action);
        if let Some(action) = action {
            self.audio
                .play_ui_sound(&self.sound_archive, "UI.img/BtMouseClick");
            match action {
                LoginAction::HighlightCharacter(index) => self.click_character_slot(index),
                // The action region is rebuilt after every selection change. Read the
                // current index here as well so a stale draw cannot send the old one.
                LoginAction::SelectCharacter(_) => {
                    self.select_character(self.selected_character_index)
                }
                LoginAction::DeleteCharacter(index) => self.delete_character(index),
                LoginAction::PreviousCharacterPage => self.change_character_page(-1),
                LoginAction::NextCharacterPage => self.change_character_page(1),
                LoginAction::BeginCharacterCreation => self.begin_character_creation(),
                LoginAction::ReturnToWorld => {
                    self.character_list = None;
                    self.character_selection_status = None;
                    self.refresh_world_scene();
                    self.show_login_fields = false;
                }
                _ => {}
            }
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
        // Character selection is a top-level screen. Do not let an unrelated
        // gameplay panel consume clicks while this screen is active.
        true
    }

    pub(super) fn move_character_selection(&mut self, direction: isize) {
        let Some(character_count) = self
            .character_list
            .as_ref()
            .map(|characters| characters.characters.len())
        else {
            return;
        };
        if character_count == 0 {
            return;
        }
        let index = self
            .selected_character_index
            .saturating_add_signed(direction)
            .min(character_count - 1);
        self.focus_character(index);
    }

    pub(super) fn request_characters(&self) {
        let (Some(session), Some(world_id)) = (self.session.as_ref(), self.selected_world) else {
            return;
        };
        if let Err(error) = session.send_packet(openstory_protocol::request_characters(
            world_id,
            self.selected_channel,
        )) {
            eprintln!("OpenStory: could not request characters: {error}");
        }
    }

    pub(super) fn show_character_selection(
        &mut self,
        characters: openstory_protocol::CharacterList,
    ) {
        self.character_creation = None;
        self.character_page = 0;
        self.selected_character_index = 0;
        self.character_action_focus = None;
        self.character_selection_status = None;
        self.character_list = Some(characters);
        self.show_login_fields = false;
        self.refresh_character_scene();
    }

    pub(super) fn begin_character_creation(&mut self) {
        self.character_creation = Some(CharacterCreationState::class_select());
        self.refresh_character_creation_scene();
    }

    pub(super) fn choose_creation_class(&mut self, job: u16) {
        match load_character_creation_options(&self.etc_archive, job, false) {
            Ok(options) => {
                self.character_creation = Some(CharacterCreationState::customize(job, options));
                self.refresh_character_creation_scene();
            }
            Err(error) => {
                eprintln!("OpenStory: could not load character creation options: {error}");
                if let Some(creation) = self.character_creation.as_mut() {
                    creation.notice =
                        Some("Character appearance data could not be loaded".to_owned());
                }
                self.refresh_character_creation_scene();
            }
        }
    }

    pub(super) fn refresh_character_creation_scene(&mut self) {
        let Some(creation) = self.character_creation.as_ref() else {
            return;
        };
        let scene = if creation.job.is_some() {
            load_character_customization_scene(
                &self.map_archive,
                &self.legacy_ui_archive,
                &self.character_archive,
                &self.string_archive,
                &mut self.item_name_cache,
                self.login_caret_visible,
                creation,
                &self.font,
            )
        } else {
            load_character_class_scene(&self.map_archive, &self.legacy_ui_archive, &self.font)
        };
        match scene.and_then(|scene| self.replace_scene(scene)) {
            Ok(()) => self.show_login_fields = false,
            Err(error) => eprintln!("OpenStory: could not display character creation: {error}"),
        }
    }

    pub(super) fn cancel_character_creation(&mut self) {
        if self
            .character_creation
            .as_ref()
            .is_some_and(|creation| creation.awaiting_name || creation.awaiting_create)
        {
            return;
        }
        self.character_creation = None;
        self.refresh_character_scene();
    }

    pub(super) fn focus_creation_name(&mut self) {
        if let Some(creation) = self.character_creation.as_mut() {
            if !creation.awaiting_name && !creation.awaiting_create {
                creation.name_focused = true;
                creation.notice = None;
                self.reset_login_caret();
            }
        }
        self.refresh_character_creation_scene();
    }

    pub(super) fn cycle_creation_option(&mut self, option: usize, direction: i8) {
        let Some(creation) = self.character_creation.as_mut() else {
            return;
        };
        if creation.awaiting_name || creation.awaiting_create || option >= creation.pools.len() {
            return;
        }
        let pool = &creation.pools[option];
        if pool.is_empty() {
            return;
        }
        let index = &mut creation.indices[option];
        *index = if direction < 0 {
            if *index == 0 {
                pool.len() - 1
            } else {
                *index - 1
            }
        } else {
            (*index + 1) % pool.len()
        };
        creation.notice = None;
        self.refresh_character_creation_scene();
    }

    pub(super) fn toggle_creation_gender(&mut self, female: bool) {
        let Some(job) = self
            .character_creation
            .as_ref()
            .filter(|creation| !creation.awaiting_name && !creation.awaiting_create)
            .and_then(|creation| creation.job)
        else {
            return;
        };
        match load_character_creation_options(&self.etc_archive, job, female) {
            Ok(options) => {
                if let Some(creation) = self.character_creation.as_mut() {
                    creation.female = female;
                    creation.pools = options;
                    creation.indices = [0; 8];
                    creation.notice = None;
                }
                self.refresh_character_creation_scene();
            }
            Err(error) => eprintln!("OpenStory: could not switch character gender: {error}"),
        }
    }

    pub(super) fn submit_character_creation(&mut self) {
        let Some(creation) = self.character_creation.as_ref() else {
            return;
        };
        if creation.awaiting_name || creation.awaiting_create {
            return;
        }
        let name = creation.name.trim();
        let name_length = name.chars().count();
        if !(4..=12).contains(&name_length) || !name.is_ascii() {
            if let Some(creation) = self.character_creation.as_mut() {
                creation.notice = Some("Name must be 4–12 ASCII characters".to_owned());
            }
            self.refresh_character_creation_scene();
            return;
        }
        if creation_name_is_forbidden(&self.etc_archive, name) {
            if let Some(creation) = self.character_creation.as_mut() {
                creation.notice = Some("That name is not allowed".to_owned());
            }
            self.refresh_character_creation_scene();
            return;
        }
        let packet = match openstory_protocol::request_character_name(name) {
            Ok(packet) => packet,
            Err(error) => {
                if let Some(creation) = self.character_creation.as_mut() {
                    creation.notice = Some(format!("Could not check name: {error}"));
                }
                self.refresh_character_creation_scene();
                return;
            }
        };
        let Some(session) = self.session.as_ref() else {
            if let Some(creation) = self.character_creation.as_mut() {
                creation.notice =
                    Some("Connect to a server before creating a character".to_owned());
            }
            self.refresh_character_creation_scene();
            return;
        };
        match session.send_packet(packet) {
            Ok(()) => {
                if let Some(creation) = self.character_creation.as_mut() {
                    creation.awaiting_name = true;
                    creation.name_focused = false;
                    creation.notice = Some("Checking name…".to_owned());
                }
            }
            Err(error) => {
                if let Some(creation) = self.character_creation.as_mut() {
                    creation.notice = Some(format!("Could not check name: {error}"));
                }
            }
        }
        self.refresh_character_creation_scene();
    }

    pub(super) fn handle_character_name_result(&mut self, packet: &[u8]) {
        let (name, used) = match openstory_protocol::parse_character_name_result(packet) {
            Ok(result) => result,
            Err(error) => {
                eprintln!("OpenStory: invalid character-name response: {error}");
                self.fail_character_creation("Invalid name-check response".to_owned());
                return;
            }
        };
        let Some(creation) = self.character_creation.as_ref() else {
            return;
        };
        if !creation.awaiting_name || !creation.name.eq_ignore_ascii_case(&name) {
            eprintln!("OpenStory: ignored an unexpected character-name response");
            return;
        }
        if used {
            if let Some(creation) = self.character_creation.as_mut() {
                creation.awaiting_name = false;
                creation.name_focused = true;
                creation.notice = Some("That name is already in use".to_owned());
            }
            self.refresh_character_creation_scene();
            return;
        }
        let Some(job) = creation.job else {
            return;
        };
        let selected = |pool: usize| -> Option<i32> {
            let values = creation.pools.get(pool)?;
            values.get(*creation.indices.get(pool)?).copied()
        };
        let values = (
            selected(0),
            selected(1),
            selected(2).and_then(|value| u8::try_from(value).ok()),
            selected(3).and_then(|value| u8::try_from(value).ok()),
            selected(4),
            selected(5),
            selected(6),
            selected(7),
        );
        let (
            Some(face),
            Some(hair),
            Some(hair_color),
            Some(skin),
            Some(top),
            Some(bottom),
            Some(shoes),
            Some(weapon),
        ) = values
        else {
            self.fail_character_creation("Selected appearance data is incomplete".to_owned());
            return;
        };
        let create_packet = openstory_protocol::create_character(
            &name,
            job,
            face,
            hair,
            hair_color,
            skin,
            top,
            bottom,
            shoes,
            weapon,
            creation.female,
        );
        let Some(session) = self.session.as_ref() else {
            self.fail_character_creation("Server connection is unavailable".to_owned());
            return;
        };
        match create_packet {
            Err(error) => {
                eprintln!("OpenStory: could not encode character creation request: {error}");
                if let Some(creation) = self.character_creation.as_mut() {
                    creation.awaiting_name = false;
                    creation.name_focused = true;
                    creation.notice = Some("Could not create the character request".to_owned());
                }
            }
            Ok(packet) => match session.send_packet(packet) {
                Ok(()) => {
                    if let Some(creation) = self.character_creation.as_mut() {
                        creation.awaiting_name = false;
                        creation.awaiting_create = true;
                        creation.notice = Some("Creating character…".to_owned());
                    }
                }
                Err(error) => {
                    eprintln!("OpenStory: could not send character creation request: {error}");
                    if let Some(creation) = self.character_creation.as_mut() {
                        creation.awaiting_name = false;
                        creation.name_focused = true;
                        creation.notice =
                            Some("Could not send character creation request".to_owned());
                    }
                }
            },
        }
        self.refresh_character_creation_scene();
    }

    pub(super) fn fail_character_creation(&mut self, message: String) {
        if let Some(creation) = self.character_creation.as_mut() {
            creation.awaiting_name = false;
            creation.awaiting_create = false;
            creation.name_focused = true;
            creation.notice = Some(message);
        }
        self.refresh_character_creation_scene();
    }

    pub(super) fn handle_created_character(&mut self, packet: &[u8]) {
        match openstory_protocol::parse_created_character(packet) {
            Ok(character) => {
                if let Some(list) = self.character_list.as_mut() {
                    if !list.characters.iter().any(|item| item.id == character.id) {
                        list.characters.push(character);
                    }
                    self.character_page =
                        list.characters.len().saturating_sub(1) / CHARACTER_PAGE_SIZE;
                    self.selected_character_index = list.characters.len().saturating_sub(1);
                    // Character creation returns to the selected new slot;
                    // put keyboard focus on Select so the next Enter keeps
                    // the legacy action flow instead of reopening creation.
                    self.character_action_focus = Some(0);
                    self.character_creation = None;
                    self.refresh_character_scene();
                } else {
                    eprintln!(
                        "OpenStory: received a new character without an active character list"
                    );
                }
            }
            Err(error) => {
                eprintln!("OpenStory: invalid created-character response: {error}");
                if let Some(creation) = self.character_creation.as_mut() {
                    creation.awaiting_create = false;
                    creation.notice = Some("The server rejected character creation".to_owned());
                    self.refresh_character_creation_scene();
                }
            }
        }
    }

    pub(super) fn refresh_character_scene(&mut self) {
        let page_count = self
            .character_list
            .as_ref()
            .map(|characters| characters.characters.len().div_ceil(CHARACTER_PAGE_SIZE))
            .unwrap_or(0);
        self.character_page = if page_count == 0 {
            0
        } else {
            self.character_page.min(page_count - 1)
        };
        let Some(characters) = self.character_list.as_ref() else {
            return;
        };
        if characters.characters.is_empty() {
            self.selected_character_index = 0;
        } else {
            self.selected_character_index = self
                .selected_character_index
                .min(characters.characters.len() - 1);
            self.character_page = self.selected_character_index / CHARACTER_PAGE_SIZE;
        }
        let scene = load_character_selection_scene(
            &self.map_archive,
            &self.legacy_ui_archive,
            &self.character_archive,
            characters,
            self.character_page,
            self.selected_character_index,
            self.character_selection_status.as_deref(),
            &self.font,
            self.character_action_focus,
        );
        match scene.and_then(|scene| self.replace_scene(scene)) {
            Ok(()) => {}
            Err(error) => {
                eprintln!("OpenStory: could not display character selection: {error}")
            }
        }
    }

    pub(super) fn apply_character_stat_update(
        &mut self,
        update: openstory_protocol::CharacterStatUpdate,
    ) {
        let hp_changed_to_zero = update
            .changes
            .iter()
            .any(|change| matches!(change, openstory_protocol::CharacterStatChange::Hp(0)));
        let hp_delta = self.active_character.as_ref().and_then(|character| {
            update.changes.iter().find_map(|change| match change {
                openstory_protocol::CharacterStatChange::Hp(value) => {
                    Some(i32::from(*value) - i32::from(character.stats.hp))
                }
                _ => None,
            })
        });
        let now = Instant::now();
        let heal_feedback_pending = self
            .pending_zombify_heal_feedback_until
            .is_some_and(|until| until > now);
        let zombify_active = character_has_mob_skill_buff(
            &self.active_buffs,
            openstory_protocol::BuffMaskGroup::Second,
            STATUS_ZOMBIFY,
            MOB_SKILL_TYPE_UNDEAD,
        );
        let hp_healed = hp_delta.filter(|delta| *delta > 0);
        let zombify_heal_backlash = (heal_feedback_pending && zombify_active)
            .then(|| hp_delta.filter(|delta| *delta < 0).map(i32::abs))
            .flatten();
        if hp_delta.is_some() || !heal_feedback_pending {
            self.pending_zombify_heal_feedback_until = None;
        }
        let Some(character) = self.active_character.as_mut() else {
            return;
        };
        for change in update.changes {
            use openstory_protocol::CharacterStatChange as Change;
            match change {
                Change::Skin(value) => character.look.skin = value,
                Change::Face(value) => character.look.face_id = value,
                Change::Hair(value) => character.look.hair_id = value,
                Change::Level(value) => character.stats.level = value,
                Change::Job(value) => character.stats.job = value,
                Change::Strength(value) => character.stats.strength = value,
                Change::Dexterity(value) => character.stats.dexterity = value,
                Change::Intelligence(value) => character.stats.intelligence = value,
                Change::Luck(value) => character.stats.luck = value,
                Change::Hp(value) => character.stats.hp = value,
                Change::MaxHp(value) => character.stats.max_hp = value,
                Change::Mp(value) => character.stats.mp = value,
                Change::MaxMp(value) => character.stats.max_mp = value,
                Change::AbilityPoints(value) => character.stats.ability_points = value,
                Change::SkillPoints(value) => character.stats.skill_points = value,
                Change::SkillPointTable(values) => {
                    character.stats.skill_points =
                        values.iter().map(|(_, points)| u16::from(*points)).sum();
                    character.stats.skill_points_by_job = values;
                }
                Change::Experience(value) => character.stats.experience = value,
                Change::Fame(value) => character.stats.fame = value,
                Change::Meso(value) => {
                    character.stats.meso = Some(value);
                    if let Some(inventory) = self.inventory.as_mut() {
                        inventory.meso = i64::from(value);
                    }
                }
                Change::Pet(_) | Change::GachaExperience(_) => {}
            }
        }
        if hp_delta.is_some_and(|delta| delta < 0)
            && let Some(scene) = self.map_scene.as_mut()
        {
            scene.idle_regen_elapsed = Duration::ZERO;
            scene.regen_hit_lockout_remaining = Duration::from_secs(3);
        }
        if let Some(amount) = hp_healed
            && let Some(scene) = self.map_scene.as_mut()
        {
            scene.show_local_player_heal(amount);
        }
        if let Some(amount) = zombify_heal_backlash
            && let Some(scene) = self.map_scene.as_mut()
        {
            scene.show_local_heal_backlash(amount);
        }
        self.refresh_map_hud();
        self.refresh_inventory_texture();
        self.refresh_skill_book_entries();
        self.refresh_skill_book_texture();
        self.refresh_npc_quest_marks();
        self.status_waiting_for_stats = false;
        self.refresh_status_texture();
        if hp_changed_to_zero {
            self.enter_death_state();
        }
    }

    pub(super) fn apply_forced_stat_set(&mut self, update: openstory_protocol::ForcedStatSet) {
        let Some(character) = self.active_character.as_mut() else {
            eprintln!("OpenStoryRS: ignored forced stats without an active character");
            return;
        };
        if let Some(value) = update.strength {
            character.stats.strength = value;
        }
        if let Some(value) = update.dexterity {
            character.stats.dexterity = value;
        }
        if let Some(value) = update.intelligence {
            character.stats.intelligence = value;
        }
        if let Some(value) = update.luck {
            character.stats.luck = value;
        }

        let forced = &mut self.forced_equipment_stats;
        if let Some(value) = update.weapon_attack {
            forced.weapon_attack = Some(value.min(999));
        }
        if let Some(value) = update.weapon_defense {
            forced.weapon_defense = Some(value.min(999));
        }
        if let Some(value) = update.magic_attack {
            forced.magic_attack = Some(value.min(2000));
        }
        if let Some(value) = update.magic_defense {
            forced.magic_defense = Some(value.min(999));
        }
        if let Some(value) = update.accuracy {
            forced.accuracy = Some(value.min(999));
        }
        if let Some(value) = update.avoidability {
            forced.avoidability = Some(value.min(999));
        }
        if let Some(value) = update.speed {
            forced.speed = Some(value.min(140));
        }
        if let Some(value) = update.jump {
            forced.jump = Some(value.min(123));
        }
        self.refresh_map_hud();
        self.refresh_status_texture();
    }

    pub(super) fn recalculate_forced_stats(&mut self) {
        self.forced_equipment_stats = ForcedEquipmentStats::default();
        self.refresh_map_hud();
        self.refresh_status_texture();
    }
}
