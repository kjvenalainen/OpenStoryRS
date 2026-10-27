use super::*;

impl ClientApp {
    pub(super) fn spawn_reactor(&mut self, spawn: openstory_protocol::MapReactorSpawn) {
        let mut sprites = Vec::new();
        let assets = match append_reactor_animations(
            &mut sprites,
            &self.reactor_archive,
            spawn.reactor_id,
            &mut self.remote_art_cache,
        ) {
            Ok(assets) => assets,
            Err(error) => {
                eprintln!(
                    "OpenStory: could not load reactor {}: {error}",
                    spawn.reactor_id
                );
                return;
            }
        };
        let draws = match self.upload_dynamic_sprites(sprites) {
            Ok(draws) => draws,
            Err(error) => {
                eprintln!(
                    "OpenStory: could not upload reactor {} art: {error}",
                    spawn.reactor_id
                );
                return;
            }
        };
        let Some(idle_frame) = reactor_frame_runtime(&assets.idle, &draws) else {
            eprintln!(
                "OpenStory: reactor {} has no drawable idle frame",
                spawn.reactor_id
            );
            return;
        };
        let Some((_, idle_rect)) = idle_frame.sprites.first() else {
            return;
        };
        let hit_animations = assets
            .hits
            .into_iter()
            .filter_map(|(state, frames)| {
                let frames = frames
                    .iter()
                    .filter_map(|frame| reactor_frame_runtime(frame, &draws))
                    .collect::<Vec<_>>();
                (!frames.is_empty()).then_some((state, frames))
            })
            .collect();
        let x = f64::from(spawn.x);
        let (y, foothold_layer) = self
            .map_scene
            .as_ref()
            .and_then(|scene| {
                let footholds = scene.footholds.as_ref()?;
                let foothold_layer = footholds
                    .find_below(f64::from(spawn.x), f64::from(spawn.y))
                    .and_then(|(foothold_id, _)| footholds.get(foothold_id))
                    .map_or(0, |foothold| foothold.layer);
                Some((
                    f64::from(footholds.y_below(spawn.x, spawn.y)),
                    foothold_layer,
                ))
            })
            .unwrap_or((f64::from(spawn.y), 0));
        let reactor = MapReactorRuntime {
            state: spawn.state,
            x,
            y,
            foothold_layer,
            width: f64::from(idle_rect.width),
            height: f64::from(idle_rect.height),
            hittable: assets.hittable,
            idle_frame,
            hit_animations,
            active_frames: None,
            frame_index: 0,
            frame_elapsed: Duration::ZERO,
            dead: false,
        };
        if let Some(scene) = self.map_scene.as_mut() {
            scene.reactors.insert(spawn.object_id, reactor);
        }
    }

    pub(super) fn spawn_hired_merchant(&mut self, spawn: openstory_protocol::HiredMerchantSpawn) {
        let mut sprites = Vec::new();
        let (frame_assets, sign_index, label_index, skin) = match append_hired_merchant_art(
            &mut sprites,
            &self.item_archive,
            &self.ui_archive,
            spawn.item_id,
            spawn.skin,
            &spawn.owner_name,
            &spawn.description,
            &self.font,
            &mut self.remote_art_cache,
        ) {
            Ok(assets) => assets,
            Err(error) => {
                eprintln!(
                    "OpenStoryRS: could not load hired merchant {} art: {error}",
                    spawn.item_id
                );
                return;
            }
        };
        let draws = match self.upload_dynamic_sprites(sprites) {
            Ok(draws) => draws,
            Err(error) => {
                eprintln!(
                    "OpenStoryRS: could not upload hired merchant {} art: {error}",
                    spawn.item_id
                );
                return;
            }
        };
        let frames = projectile_frames_runtime(frame_assets, &draws);
        let sign = draws.get(sign_index).copied();
        let label = draws.get(label_index).copied();
        if let Some(scene) = self.map_scene.as_mut() {
            scene.hired_merchants.insert(
                spawn.owner_id,
                MapHiredMerchantRuntime {
                    object_id: spawn.object_id,
                    x: f64::from(spawn.x),
                    y: f64::from(spawn.y),
                    owner_name: spawn.owner_name,
                    description: spawn.description,
                    skin,
                    frames,
                    frame_index: 0,
                    elapsed: Duration::ZERO,
                    sign,
                    label,
                    visitors: 0,
                    visitor_capacity: 0,
                },
            );
        }
    }

    pub(super) fn remove_hired_merchant(&mut self, owner_id: i32) {
        if let Some(scene) = self.map_scene.as_mut() {
            scene.hired_merchants.remove(&owner_id);
        }
    }

    pub(super) fn update_hired_merchant(
        &mut self,
        update: openstory_protocol::HiredMerchantUpdate,
    ) {
        let updated_description = update.description.clone();
        let refresh_label = if let Some(merchant) = self
            .map_scene
            .as_mut()
            .and_then(|scene| scene.hired_merchants.get_mut(&update.owner_id))
        {
            let refresh = merchant.description != update.description;
            merchant.object_id = update.object_id;
            merchant.description = update.description;
            merchant.visitors = update.visitors.max(0);
            merchant.visitor_capacity = update.visitor_capacity.max(0);
            refresh
        } else {
            false
        };
        if refresh_label {
            self.refresh_hired_merchant_label(update.owner_id);
        }
        self.push_chat_line(
            format!(
                "[Merchant] {} ({}/{}) visitors",
                updated_description,
                update.visitors.max(0),
                update.visitor_capacity.max(0)
            ),
            [255, 222, 112],
        );
    }

    pub(super) fn refresh_hired_merchant_label(&mut self, owner_id: i32) {
        let Some((text, skin, sign)) = self.map_scene.as_ref().and_then(|scene| {
            scene.hired_merchants.get(&owner_id).map(|merchant| {
                let text = if merchant.description.is_empty() {
                    merchant.owner_name.clone()
                } else {
                    merchant.description.clone()
                };
                (text, merchant.skin, merchant.sign)
            })
        }) else {
            return;
        };
        let text = text.chars().take(14).collect::<String>();
        let Some((_, sign_rect)) = sign else {
            return;
        };
        let mut sprites = Vec::new();
        append_text_sprite(
            &mut sprites,
            &self.font,
            &text,
            centered_text_x(&self.font, &text, 12.0),
            sign_rect.y + if skin == 0 { 9.0 } else { 68.0 },
            12.0,
        );
        let Ok(mut draws) = self.upload_dynamic_sprites(sprites) else {
            return;
        };
        if let (Some(label), Some(merchant)) = (
            draws.pop(),
            self.map_scene
                .as_mut()
                .and_then(|scene| scene.hired_merchants.get_mut(&owner_id)),
        ) {
            merchant.label = Some(label);
        }
    }

    pub(super) fn request_hired_merchant_visit(&mut self, owner_id: i32) {
        let object_id = self
            .map_scene
            .as_ref()
            .and_then(|scene| scene.hired_merchants.get(&owner_id))
            .map(|merchant| merchant.object_id);
        let Some(object_id) = object_id else {
            return;
        };
        let Some(session) = self.session.as_ref() else {
            return;
        };
        if let Err(error) =
            session.send_packet(openstory_protocol::mini_room_visit_packet(object_id))
        {
            eprintln!("OpenStoryRS: could not request hired merchant visit: {error}");
            self.channel_connected = false;
        }
    }

    pub(super) fn request_personal_shop_visit(&mut self, object_id: i32) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        if let Err(error) =
            session.send_packet(openstory_protocol::mini_room_visit_packet(object_id))
        {
            eprintln!("OpenStoryRS: could not request personal shop visit: {error}");
            self.channel_connected = false;
        }
    }

    pub(super) fn request_character_info(&mut self, character_id: i32) {
        let Some(packet) = openstory_protocol::character_info_request_packet(character_id) else {
            return;
        };
        let Some(session) = self.session.as_ref() else {
            return;
        };
        if let Err(error) = session.send_packet(packet) {
            eprintln!("OpenStoryRS: could not request character info: {error}");
            self.channel_connected = false;
        }
    }

    pub(super) fn handle_character_info(&mut self, info: openstory_protocol::CharacterInfoReply) {
        if self
            .active_character
            .as_ref()
            .is_some_and(|character| character.id == info.character_id)
        {
            self.monster_book.set_cover(info.monster_book.cover_item_id);
            if self.monster_book.open {
                self.refresh_monster_book_texture();
            }
        }
        let name = self
            .map_scene
            .as_ref()
            .and_then(|scene| scene.remote_players.get(&info.character_id))
            .map(|player| player.name.clone())
            .or_else(|| {
                self.active_character
                    .as_ref()
                    .filter(|character| character.id == info.character_id)
                    .map(|character| character.stats.name.clone())
            })
            .unwrap_or_else(|| format!("Character {}", info.character_id));
        self.character_info = Some(CharacterInfoUi {
            name,
            info,
            fame_pending: false,
        });
        self.refresh_character_info_texture();
    }

    pub(super) fn request_character_fame(&mut self, raise: bool) {
        let Some((character_id, character_name, fame_pending)) = self
            .character_info
            .as_ref()
            .map(|ui| (ui.info.character_id, ui.name.clone(), ui.fame_pending))
        else {
            return;
        };
        if fame_pending {
            return;
        }
        let Some(player) = self.active_character.as_ref() else {
            return;
        };
        if player.id == character_id {
            self.push_chat_line(
                "[Fame] You can't raise or lower your own fame.".to_owned(),
                [255, 105, 105],
            );
            return;
        }
        if player.stats.level < 15 {
            self.push_chat_line(
                "[Fame] You must be at least Level 15 to give fame.".to_owned(),
                [255, 105, 105],
            );
            return;
        }
        let Some(packet) = openstory_protocol::give_fame_packet(character_id, raise) else {
            return;
        };
        let Some(session) = self.session.as_ref() else {
            self.push_chat_line(
                "[Fame] The server connection is unavailable.".to_owned(),
                [255, 105, 105],
            );
            return;
        };
        if let Err(error) = session.send_packet(packet) {
            eprintln!("OpenStoryRS: could not send fame request: {error}");
            self.channel_connected = false;
            self.push_chat_line(
                "[Fame] Could not send the request.".to_owned(),
                [255, 105, 105],
            );
            return;
        }
        if let Some(ui) = self.character_info.as_mut() {
            ui.fame_pending = true;
        }
        self.push_chat_line(
            format!(
                "[Fame] Requesting to {} {character_name}'s fame…",
                if raise { "raise" } else { "lower" }
            ),
            [255, 222, 112],
        );
        self.refresh_character_info_texture();
    }

    pub(super) fn handle_fame_response(&mut self, response: openstory_protocol::FameResponse) {
        let (message, color) = match response {
            openstory_protocol::FameResponse::Success {
                character_name,
                raised,
                fame,
            } => {
                if let Some(ui) = self.character_info.as_mut() {
                    ui.fame_pending = false;
                    if ui.name == character_name {
                        ui.info.fame = fame;
                    }
                }
                (
                    format!(
                        "You have {} {character_name}'s fame to {fame}.",
                        if raised { "raised" } else { "lowered" }
                    ),
                    [245, 239, 223],
                )
            }
            openstory_protocol::FameResponse::CannotFameSelf => (
                "[Fame] You can't raise or lower your own fame.".to_owned(),
                [255, 105, 105],
            ),
            openstory_protocol::FameResponse::AlreadyFamed => (
                "[Fame] You have already given fame to this character today.".to_owned(),
                [255, 105, 105],
            ),
            openstory_protocol::FameResponse::TargetLevelTooLow => (
                "[Fame] That character's level is too low.".to_owned(),
                [255, 105, 105],
            ),
            openstory_protocol::FameResponse::PlayerLevelTooLow => (
                "[Fame] You must be at least Level 15 to give fame.".to_owned(),
                [255, 105, 105],
            ),
            openstory_protocol::FameResponse::Unknown(mode) => (
                format!("[Fame] The server rejected the request (result {mode})."),
                [255, 105, 105],
            ),
        };
        if let Some(ui) = self.character_info.as_mut() {
            ui.fame_pending = false;
        }
        self.push_chat_line(message, color);
        if self.character_info.is_some() {
            self.refresh_character_info_texture();
        }
    }

    pub(super) fn handle_skill_learn_item_result(
        &mut self,
        result: openstory_protocol::SkillLearnItemResult,
    ) {
        if result.marker != 1 {
            eprintln!(
                "OpenStoryRS: skill-book result for character {} had marker {}, skill {}, max level {}, can-use {}",
                result.character_id,
                result.marker,
                result.skill_id,
                result.max_level,
                result.can_use
            );
        }
        let (message, color) = if result.success {
            (
                "[Skill Book] Used successfully!".to_owned(),
                [238, 232, 214],
            )
        } else {
            ("[Skill Book] Usage failed.".to_owned(), [255, 105, 105])
        };
        self.push_chat_line(message, color);
    }

    pub(super) fn spawn_npc(&mut self, spawn: openstory_protocol::MapNpcSpawn) {
        let flipped = !spawn.flipped;
        let maple_tv_anchors = maple_tv_screen_anchors(&self.npc_archive, spawn.npc_id);
        if maple_tv_anchors.is_some() {
            self.ensure_maple_tv_assets();
        }
        let mut sprites = Vec::new();
        let (assets, traits) = match append_npc_animations(
            &mut sprites,
            &self.npc_archive,
            spawn.npc_id,
            &mut self.remote_art_cache,
        ) {
            Ok(assets) => assets,
            Err(error) => {
                eprintln!("OpenStory: could not load NPC {}: {error}", spawn.npc_id);
                return;
            }
        };
        let draws = match self.upload_dynamic_sprites(sprites) {
            Ok(draws) => draws,
            Err(error) => {
                eprintln!(
                    "OpenStory: could not upload NPC {} art: {error}",
                    spawn.npc_id
                );
                return;
            }
        };
        let mut animations: HashMap<NpcMotion, Vec<NpcFrameRuntime>> = HashMap::new();
        for frame in assets {
            let runtime_frame = NpcFrameRuntime {
                delay: Duration::from_millis(u64::from(frame.delay_ms.max(1))),
                sprites: frame
                    .sprites
                    .filter_map(|index| draws.get(index).copied())
                    .collect(),
            };
            if !runtime_frame.sprites.is_empty() {
                animations
                    .entry(frame.motion)
                    .or_default()
                    .push(runtime_frame);
            }
        }
        let mut animation_order = animations.keys().cloned().collect::<Vec<_>>();
        animation_order.sort_by(|left, right| left.state_name().cmp(right.state_name()));
        if let Some(scene) = self.map_scene.as_mut() {
            let scripted = traits.scripted || scene.scriptable_npc_ids.contains(&spawn.npc_id);
            let x = f64::from(spawn.x);
            let y = f64::from(spawn.y);
            let mut body = PhysicsBody::new(x, y);
            body.foothold_id = spawn.foothold;
            body.on_ground = true;
            scene.npcs.insert(
                spawn.object_id,
                MapNpcRuntime {
                    npc_id: spawn.npc_id,
                    maple_tv_anchors,
                    scripted,
                    x,
                    y,
                    foothold: spawn.foothold,
                    body,
                    flipped,
                    animations,
                    animation_order,
                    random_state: (spawn.npc_id as u32)
                        .wrapping_mul(0x9E37_79B9)
                        .wrapping_add(spawn.object_id as u32)
                        .max(1),
                    motion: NpcMotion::Stand,
                    frame_index: 0,
                    elapsed: Duration::ZERO,
                    quest_mark: NpcQuestMark::None,
                    quest_mark_frames: Vec::new(),
                    quest_mark_frame_index: 0,
                    quest_mark_elapsed: Duration::ZERO,
                },
            );
        }
        self.refresh_npc_quest_marks();
    }

    pub(super) fn refresh_npc_quest_marks(&mut self) {
        let Some(character) = self.active_character.as_ref() else {
            return;
        };
        let Some(scene) = self.map_scene.as_ref() else {
            return;
        };
        let npc_ids = scene
            .npcs
            .values()
            .map(|npc| npc.npc_id)
            .collect::<HashSet<_>>();
        let marks = npc_quest_marks(
            &npc_ids,
            &self.quest_log,
            &self.quest_archive,
            &self.quest_start_npc_index,
            character,
            self.inventory.as_ref(),
        );
        let pending = scene
            .npcs
            .iter()
            .filter_map(|(&object_id, npc)| {
                let mark = marks.get(&npc.npc_id).copied().unwrap_or_default();
                let should_retry = mark != NpcQuestMark::None && npc.quest_mark_frames.is_empty();
                if npc.quest_mark == mark && !should_retry {
                    return None;
                }
                let top = npc
                    .animations
                    .get(&NpcMotion::Stand)
                    .and_then(|frames| frames.first())
                    .or_else(|| npc.animations.values().find_map(|frames| frames.first()))
                    .and_then(|frame| {
                        frame
                            .sprites
                            .iter()
                            .map(|(_, rect)| rect.y)
                            .min_by(f32::total_cmp)
                    })
                    .map(|top| top - 10.0);
                Some((object_id, mark, top))
            })
            .collect::<Vec<_>>();

        for (object_id, mark, anchor_y) in pending {
            let mut decoded = Vec::new();
            let assets = if mark == NpcQuestMark::None {
                Ok(Vec::new())
            } else if let Some(anchor_y) = anchor_y {
                append_npc_quest_mark_animation(
                    &mut decoded,
                    &self.ui_archive,
                    &self.map_archive,
                    mark,
                    534.0,
                    anchor_y,
                    &mut self.remote_art_cache,
                )
            } else {
                Ok(Vec::new())
            };
            let frames = match assets {
                Ok(assets) => match self.upload_dynamic_sprites(decoded) {
                    Ok(draws) => assets
                        .into_iter()
                        .map(|frame| NpcQuestMarkFrameRuntime {
                            delay: Duration::from_millis(u64::from(frame.delay_ms.max(1))),
                            sprites: frame
                                .sprites
                                .filter_map(|index| draws.get(index).copied())
                                .collect(),
                        })
                        .filter(|frame: &NpcQuestMarkFrameRuntime| !frame.sprites.is_empty())
                        .collect(),
                    Err(error) => {
                        eprintln!("OpenStory: could not upload NPC quest mark: {error}");
                        Vec::new()
                    }
                },
                Err(error) => {
                    eprintln!("OpenStory: could not load NPC quest mark: {error}");
                    Vec::new()
                }
            };
            if let Some(npc) = self
                .map_scene
                .as_mut()
                .and_then(|scene| scene.npcs.get_mut(&object_id))
            {
                npc.quest_mark = mark;
                npc.quest_mark_frames = frames;
                npc.quest_mark_frame_index = 0;
                npc.quest_mark_elapsed = Duration::ZERO;
            }
        }
    }

    pub(super) fn show_quest_clear_effect(&mut self) {
        self.audio
            .play_ui_sound(&self.sound_archive, "Game.img/QuestClear");
        if self.map_scene.is_none() {
            return;
        }
        let mut decoded = Vec::new();
        let assets = match append_quest_clear_animation(
            &mut decoded,
            &self.effect_archive,
            &mut self.remote_art_cache,
        ) {
            Ok(assets) => assets,
            Err(error) => {
                eprintln!("OpenStoryRS: could not load quest completion effect: {error}");
                return;
            }
        };
        if assets.is_empty() {
            return;
        }
        let draws = match self.upload_dynamic_sprites(decoded) {
            Ok(draws) => draws,
            Err(error) => {
                eprintln!("OpenStoryRS: could not upload quest completion effect: {error}");
                return;
            }
        };
        let frames = assets
            .into_iter()
            .map(|frame| NpcQuestMarkFrameRuntime {
                delay: Duration::from_millis(u64::from(frame.delay_ms.max(1))),
                sprites: frame
                    .sprites
                    .filter_map(|index| draws.get(index).copied())
                    .collect(),
            })
            .filter(|frame| !frame.sprites.is_empty())
            .collect::<Vec<_>>();
        if let Some(scene) = self.map_scene.as_mut() {
            scene.quest_clear_effect = (!frames.is_empty()).then_some(QuestClearEffectRuntime {
                frames,
                frame_index: 0,
                elapsed: Duration::ZERO,
            });
        }
    }

    pub(super) fn remove_npc(&mut self, object_id: i32) {
        if let Some(scene) = self.map_scene.as_mut() {
            scene.npcs.remove(&object_id);
        }
    }

    pub(super) fn mark_npcs_scriptable(&mut self, npc_ids: &[i32]) {
        self.scriptable_npc_ids.extend(npc_ids.iter().copied());
        if let Some(scene) = self.map_scene.as_mut() {
            scene.mark_npcs_scriptable(npc_ids);
        }
    }

    pub(super) fn request_npc_talk(&mut self, object_id: i32) {
        let now = Instant::now();
        if self
            .last_npc_talk_at
            .is_some_and(|last| now.duration_since(last) < Duration::from_millis(600))
        {
            return;
        }
        let Some(session) = self.session.as_ref() else {
            return;
        };
        match session.send_packet(openstory_protocol::talk_to_npc(object_id)) {
            Ok(()) => self.last_npc_talk_at = Some(now),
            Err(error) => {
                eprintln!("OpenStory: could not request NPC dialogue: {error}");
                self.channel_connected = false;
            }
        }
    }

    pub(super) fn request_nearest_npc_talk(&mut self) {
        let object_id = self.map_scene.as_ref().and_then(|scene| {
            scene
                .npcs
                .iter()
                .filter_map(|(object_id, npc)| {
                    let dx = (npc.x - scene.player.x).abs();
                    let dy = (npc.y - scene.player.y).abs();
                    (dx <= 100.0 && dy <= 60.0).then_some((*object_id, dx * dx + dy * dy))
                })
                .min_by(|left, right| left.1.total_cmp(&right.1))
                .map(|(object_id, _)| object_id)
        });
        if let Some(object_id) = object_id {
            self.request_npc_talk(object_id);
        }
    }

    pub(super) fn show_npc_dialogue(&mut self, dialogue: openstory_protocol::NpcDialogue) {
        self.gameplay_input = GameplayInput::default();
        self.npc_dialogue_scroll_dragging = false;
        self.npc_shop = None;
        let player_name = self
            .active_character
            .as_ref()
            .map_or("", |character| character.stats.name.as_str());
        let (formatted_text, inline_icons) = format_npc_dialogue_text(
            &dialogue.text,
            player_name,
            &self.string_archive,
            &mut self.item_name_cache,
            &self.item_archive,
            &self.character_archive,
            &self.skill_archive,
            &self.ui_archive,
            &mut self.inventory_icon_cache,
        );
        let (text, selections) = if dialogue.message_type == 4 {
            parse_npc_selections(&formatted_text)
        } else {
            (formatted_text.clone(), Vec::new())
        };
        let input = dialogue
            .number_bounds
            .map(|(default, _, _)| default.to_string())
            .unwrap_or_default();
        let visible_text = if dialogue.message_type == 4 {
            text.as_str()
        } else {
            ""
        };
        let (pixels, panel, buttons, text_max_scroll) = render_npc_dialogue(
            &self.font,
            dialogue.npc_id,
            dialogue.speaker,
            dialogue.message_type,
            visible_text,
            0,
            &inline_icons,
            &selections,
            0,
            0,
            &input,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        match renderer.upload_bgra(600, 260, &pixels) {
            Ok(texture) => {
                let visible_text_chars = if dialogue.message_type == 4 {
                    text.chars().count()
                } else {
                    0
                };
                self.npc_dialogue = Some(NpcDialogueUi {
                    state: dialogue,
                    text: formatted_text,
                    visible_text_chars,
                    reveal_elapsed: Duration::ZERO,
                    text_scroll: 0,
                    text_max_scroll,
                    inline_icons,
                    selections,
                    selected_selection: 0,
                    first_visible_selection: 0,
                    input,
                    texture,
                    panel,
                    buttons,
                });
            }
            Err(error) => eprintln!("OpenStory: could not render NPC dialogue: {error}"),
        }
    }

    pub(super) fn show_npc_shop(&mut self, state: openstory_protocol::NpcShop) {
        self.gameplay_input = GameplayInput::default();
        self.npc_dialogue = None;
        self.show_inventory = false;
        self.inventory_hover_slot = None;
        self.inventory_drag_source = None;
        self.pending_scroll_slot = None;
        let inventory = self.inventory.clone();
        let (pixels, _) = render_npc_shop_panel(
            &self.font,
            &state,
            NpcShopMode::Buy,
            0,
            0,
            1,
            inventory.as_ref(),
            None,
            None,
            &self.item_archive,
            &self.string_archive,
            &self.character_archive,
            &mut self.inventory_icon_cache,
            &mut self.item_name_cache,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        match renderer.upload_bgra(620, 520, &pixels) {
            Ok(texture) => {
                self.npc_shop = Some(NpcShopUi {
                    state,
                    mode: NpcShopMode::Buy,
                    selected: 0,
                    first_visible: 0,
                    quantity: 1,
                    notice: None,
                    confirmation: None,
                    texture,
                });
            }
            Err(error) => eprintln!("OpenStory: could not render NPC shop: {error}"),
        }
    }

    pub(super) fn open_cash_shop(&mut self) {
        if self.cash_shop.is_some() {
            return;
        }
        if !self.channel_connected {
            eprintln!("OpenStoryRS: connect to the field before opening the Cash Shop");
            return;
        }
        let Some(session) = self.session.as_ref() else {
            return;
        };
        if let Err(error) = session.send_packet(openstory_protocol::enter_cash_shop_packet()) {
            eprintln!("OpenStoryRS: could not enter the Cash Shop: {error}");
            self.channel_connected = false;
            return;
        }
        self.show_inventory = false;
        self.inventory_hover_slot = None;
        self.show_pet_equipment = false;
        self.show_skill_book = false;
        self.pending_scroll_slot = None;
        self.npc_shop = None;
        self.gameplay_input = GameplayInput::default();
        self.cash_shop = Some(CashShopUi::new());
        self.refresh_cash_shop_texture();
    }

    pub(super) fn exit_cash_shop(&mut self) {
        let Some(state) = self.cash_shop.as_mut() else {
            return;
        };
        if !state.entered {
            self.cash_shop = None;
            self.cash_shop_texture = None;
            return;
        }
        let Some(session) = self.session.as_ref() else {
            self.cash_shop = None;
            self.cash_shop_texture = None;
            return;
        };
        match session.send_packet(openstory_protocol::exit_cash_shop_packet()) {
            Ok(()) => state.notice = Some("Returning to the field…".to_owned()),
            Err(error) => {
                eprintln!("OpenStoryRS: could not leave the Cash Shop: {error}");
                self.cash_shop = None;
                self.cash_shop_texture = None;
                self.channel_connected = false;
            }
        }
    }

    pub(super) fn handle_cash_shop_packet(&mut self, packet: &[u8]) {
        match openstory_protocol::parse_cash_shop_open(packet) {
            Ok(()) => {
                let state = self.cash_shop.get_or_insert_with(CashShopUi::new);
                state.entered = true;
                state.notice = None;
                self.refresh_cash_shop_texture();
            }
            Err(error) => eprintln!("OpenStoryRS: invalid Cash Shop entry packet: {error}"),
        }
    }

    pub(super) fn handle_cash_shop_operation(&mut self, packet: &[u8]) {
        let operation = match openstory_protocol::parse_cash_shop_operation(packet) {
            Ok(operation) => operation,
            Err(error) => {
                eprintln!("OpenStoryRS: invalid Cash Shop operation: {error}");
                return;
            }
        };
        let Some(state) = self.cash_shop.as_mut() else {
            eprintln!("OpenStoryRS: received a Cash Shop update outside the Cash Shop");
            return;
        };
        let mut inventory_changed = false;
        match operation {
            openstory_protocol::CashShopOperation::LockerSnapshot {
                items,
                item_slots,
                character_slots,
            } => {
                state.locker = items;
                state.locker_slots = item_slots;
                state.character_slots = character_slots;
                state.pending_withdrawal = None;
            }
            openstory_protocol::CashShopOperation::PurchaseAccepted => {
                state.notice = Some("Purchase request accepted by the server.".to_owned());
            }
            openstory_protocol::CashShopOperation::ItemPurchased(item) => {
                state
                    .locker
                    .retain(|existing| existing.instance_id != item.instance_id);
                state.locker.push(item);
                state.notice = Some("Item purchased successfully.".to_owned());
                state.confirmation = false;
            }
            openstory_protocol::CashShopOperation::ItemWithdrawn {
                inventory_slot,
                item_record,
            } => {
                if let Some(instance_id) = state.pending_withdrawal {
                    let cash_item = state
                        .locker
                        .iter()
                        .find(|item| item.instance_id == instance_id)
                        .cloned();
                    if let Some(cash_item) = cash_item {
                        match openstory_protocol::parse_cash_shop_withdrawal_item(
                            &item_record,
                            cash_item.item_id,
                            inventory_slot,
                        ) {
                            Ok(item) => {
                                if let Some(inventory) = self.inventory.as_mut() {
                                    inventory.apply(openstory_protocol::InventoryUpdate {
                                        update_tick: true,
                                        movement: openstory_protocol::InventoryMovement::Internal,
                                        modifications: vec![
                                            openstory_protocol::InventoryModification::Add(item),
                                        ],
                                    });
                                    inventory_changed = true;
                                }
                                state.locker.retain(|item| item.instance_id != instance_id);
                                state.pending_withdrawal = None;
                                state.notice =
                                    Some("Item moved to your character inventory.".to_owned());
                            }
                            Err(error) => {
                                eprintln!("OpenStoryRS: invalid withdrawn Cash Shop item: {error}");
                                state.pending_withdrawal = None;
                                state.notice = Some(
                                    "The server transfer arrived, but the item could not be read."
                                        .to_owned(),
                                );
                            }
                        }
                    } else {
                        state.pending_withdrawal = None;
                        state.notice =
                            Some("The server transfer did not match a locker item.".to_owned());
                    }
                } else {
                    state.notice = Some(
                        "Received an item transfer without a matching locker request.".to_owned(),
                    );
                }
            }
            openstory_protocol::CashShopOperation::PurchaseDenied(reason) => {
                state.notice = Some(match reason {
                    0 => "Unknown purchase error.".to_owned(),
                    1 => "You do not have enough NX.".to_owned(),
                    2 => "This item is out of stock.".to_owned(),
                    3 => "Your Cash Shop locker is full.".to_owned(),
                    4 => "This item is not available for purchase.".to_owned(),
                    5 => "You have reached the purchase limit.".to_owned(),
                    6 => "You do not meet the level requirement.".to_owned(),
                    7 => "You already own this item.".to_owned(),
                    code => format!("Purchase failed (code {code})."),
                });
                state.confirmation = false;
            }
            openstory_protocol::CashShopOperation::CouponRedeemed => {
                state.notice = Some("Coupon redeemed successfully.".to_owned());
                state.coupon_code.clear();
                state.coupon_pending = false;
                state.coupon_focused = false;
                state.coupon_open = false;
            }
            openstory_protocol::CashShopOperation::CashShopMessage(code) => {
                state.coupon_pending = false;
                state.notice = Some(match code {
                    0 => "Invalid coupon code.".to_owned(),
                    1 => "This coupon has expired.".to_owned(),
                    2 => "This coupon has already been used.".to_owned(),
                    3 => "This coupon is for a different server.".to_owned(),
                    code => format!("Coupon error (server code {code})."),
                });
            }
            openstory_protocol::CashShopOperation::Unknown(_) => {}
        }
        if inventory_changed {
            self.sync_equipped_look();
            self.refresh_inventory_texture();
            self.refresh_pet_equipment_texture();
            self.refresh_npc_shop();
        }
        self.refresh_cash_shop_texture();
    }

    pub(super) fn handle_cash_balances(&mut self, packet: &[u8]) {
        match openstory_protocol::parse_cash_balances(packet) {
            Ok(balances) => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.balances = balances;
                    self.refresh_cash_shop_texture();
                }
            }
            Err(error) => eprintln!("OpenStoryRS: invalid Cash Shop balance packet: {error}"),
        }
    }

    pub(super) fn buy_selected_cash_item(&mut self) {
        let Some(state) = self.cash_shop.as_ref() else {
            return;
        };
        if !state.entered || !state.confirmation {
            return;
        }
        let currency_index = state.currency.min(2);
        let currency = [1_u8, 2, 4][currency_index];
        let Some(product) = self
            .cash_products
            .iter()
            .find(|product| Some(product.sn) == state.selected_sn)
            .cloned()
        else {
            return;
        };
        if product.sn <= 0 {
            eprintln!("OpenStoryRS: blocked Cash Shop item without a commodity SN");
            if let Some(state) = self.cash_shop.as_mut() {
                state.notice = Some("This item cannot be purchased right now.".to_owned());
                state.confirmation = false;
            }
            self.refresh_cash_shop_texture();
            return;
        }
        if product.price > state.balances[currency_index] {
            if let Some(state) = self.cash_shop.as_mut() {
                state.notice = Some("You do not have enough NX for this item.".to_owned());
                state.confirmation = false;
            }
            self.refresh_cash_shop_texture();
            return;
        }
        let Some(session) = self.session.as_ref() else {
            return;
        };
        match session.send_packet(openstory_protocol::buy_cash_item_packet(
            currency, product.sn,
        )) {
            Ok(()) => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.notice =
                        Some("Purchase sent; waiting for server confirmation.".to_owned());
                    state.confirmation = false;
                }
            }
            Err(error) => {
                eprintln!("OpenStoryRS: could not purchase Cash Shop item: {error}");
                self.channel_connected = false;
            }
        }
        self.refresh_cash_shop_texture();
    }

    pub(super) fn redeem_cash_shop_coupon(&mut self) {
        let Some(state) = self.cash_shop.as_ref() else {
            return;
        };
        if state.coupon_pending {
            return;
        }
        if !state.entered {
            if let Some(state) = self.cash_shop.as_mut() {
                state.notice = Some("Waiting for the server to open the shop.".to_owned());
            }
            self.refresh_cash_shop_texture();
            return;
        }
        let code = state.coupon_code.trim().to_ascii_uppercase();
        if code.is_empty() {
            if let Some(state) = self.cash_shop.as_mut() {
                state.notice = Some("Enter a coupon code first.".to_owned());
                state.coupon_focused = true;
            }
            self.refresh_cash_shop_texture();
            return;
        }
        let packet = match openstory_protocol::cash_shop_coupon_packet(&code) {
            Ok(packet) => packet,
            Err(error) => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.notice = Some(format!("Could not build coupon request: {error}"));
                }
                self.refresh_cash_shop_texture();
                return;
            }
        };
        let Some(session) = self.session.as_ref() else {
            return;
        };
        match session.send_packet(packet) {
            Ok(()) => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.coupon_pending = true;
                    state.coupon_focused = false;
                    state.notice = Some("Coupon sent; waiting for the server.".to_owned());
                }
            }
            Err(error) => {
                eprintln!("OpenStoryRS: could not redeem Cash Shop coupon: {error}");
                self.channel_connected = false;
                if let Some(state) = self.cash_shop.as_mut() {
                    state.notice = Some("Could not send the coupon request.".to_owned());
                }
            }
        }
        self.refresh_cash_shop_texture();
    }

    pub(super) fn withdraw_cash_shop_item(&mut self, instance_id: i64) {
        let Some(state) = self.cash_shop.as_ref() else {
            return;
        };
        if state.pending_withdrawal.is_some()
            || !state
                .locker
                .iter()
                .any(|item| item.instance_id == instance_id)
        {
            return;
        }
        let Some(session) = self.session.as_ref() else {
            return;
        };
        match session.send_packet(openstory_protocol::withdraw_cash_item_packet(instance_id)) {
            Ok(()) => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.pending_withdrawal = Some(instance_id);
                    state.notice =
                        Some("Withdrawal sent; waiting for server confirmation.".to_owned());
                }
            }
            Err(error) => {
                eprintln!("OpenStoryRS: could not withdraw Cash Shop item: {error}");
                self.channel_connected = false;
            }
        }
        self.refresh_cash_shop_texture();
    }

    pub(super) fn click_cash_shop(&mut self, cursor: Point) {
        let Some(action) = self.cash_shop_hit_action(cursor) else {
            return;
        };
        let modal = self.cash_shop.as_ref().map_or(0, |state| {
            if state.confirmation {
                1
            } else if state.coupon_open {
                2
            } else {
                0
            }
        });
        if modal == 1
            && !matches!(
                action,
                cash_shop_ui::HitAction::ConfirmPurchase | cash_shop_ui::HitAction::CancelPurchase
            )
        {
            return;
        }
        if modal == 2
            && !matches!(
                action,
                cash_shop_ui::HitAction::CouponInput
                    | cash_shop_ui::HitAction::CouponSubmit
                    | cash_shop_ui::HitAction::CouponCancel
            )
        {
            return;
        }

        let mut purchase = false;
        let mut redeem_coupon = false;
        let mut withdrawal = None;
        match action {
            cash_shop_ui::HitAction::Close => {
                self.exit_cash_shop();
                self.refresh_cash_shop_texture();
                return;
            }
            cash_shop_ui::HitAction::Help => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.notice = Some("Cash Shop help is not available yet.".to_owned());
                }
            }
            cash_shop_ui::HitAction::Coupon => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.coupon_open = true;
                    state.coupon_focused = !state.coupon_pending;
                    state.search_focused = false;
                }
            }
            cash_shop_ui::HitAction::Search => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.coupon_focused = false;
                    state.search_focused = true;
                }
            }
            cash_shop_ui::HitAction::CatalogView => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.view = CashShopView::Catalog;
                    state.page = 0;
                    state.search_focused = false;
                }
            }
            cash_shop_ui::HitAction::LockerView => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.view = CashShopView::Locker;
                    state.page = 0;
                    state.search_focused = false;
                }
            }
            cash_shop_ui::HitAction::Category(category) => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.category = category;
                    state.page = 0;
                    state.selected_sn = None;
                    state.search_focused = false;
                }
            }
            cash_shop_ui::HitAction::Product(index) => {
                if let (Some(state), Some(product)) =
                    (self.cash_shop.as_mut(), self.cash_products.get(index))
                {
                    state.selected_sn = Some(product.sn);
                    state.search_focused = false;
                }
            }
            cash_shop_ui::HitAction::BuyProduct(index) => {
                if let (Some(state), Some(product)) =
                    (self.cash_shop.as_mut(), self.cash_products.get(index))
                {
                    state.selected_sn = Some(product.sn);
                    state.search_focused = false;
                    if state.entered {
                        state.confirmation = true;
                    } else {
                        state.notice = Some("Waiting for the server to open the shop.".to_owned());
                    }
                }
            }
            cash_shop_ui::HitAction::BuySelected => {
                if let Some(state) = self.cash_shop.as_mut() {
                    if state.entered && state.selected_sn.is_some() {
                        state.confirmation = true;
                    } else {
                        state.notice = Some("Choose an available item first.".to_owned());
                    }
                    state.search_focused = false;
                }
            }
            cash_shop_ui::HitAction::Currency(currency) => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.currency = currency.min(2);
                    state.search_focused = false;
                }
            }
            cash_shop_ui::HitAction::PreviousPage => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.page = state.page.saturating_sub(1);
                    state.search_focused = false;
                }
            }
            cash_shop_ui::HitAction::NextPage => {
                let max_page = self.cash_shop_max_page();
                if let Some(state) = self.cash_shop.as_mut() {
                    state.page = (state.page + 1).min(max_page);
                    state.search_focused = false;
                }
            }
            cash_shop_ui::HitAction::LockerItem(index) => {
                withdrawal = self.cash_shop.as_ref().and_then(|state| {
                    let mut items = state.locker.iter().collect::<Vec<_>>();
                    items.sort_by_key(|item| item.instance_id);
                    items.get(index).map(|item| item.instance_id)
                });
                if let Some(state) = self.cash_shop.as_mut() {
                    state.search_focused = false;
                }
            }
            cash_shop_ui::HitAction::CouponInput => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.coupon_focused = !state.coupon_pending;
                }
            }
            cash_shop_ui::HitAction::CouponSubmit => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.coupon_focused = false;
                }
                redeem_coupon = true;
            }
            cash_shop_ui::HitAction::CouponCancel => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.coupon_open = false;
                    state.coupon_focused = false;
                }
            }
            cash_shop_ui::HitAction::ConfirmPurchase => {
                purchase = true;
            }
            cash_shop_ui::HitAction::CancelPurchase => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.confirmation = false;
                }
            }
        }
        self.refresh_cash_shop_texture();
        if purchase {
            self.buy_selected_cash_item();
        }
        if redeem_coupon {
            self.redeem_cash_shop_coupon();
        }
        if let Some(instance_id) = withdrawal {
            self.withdraw_cash_shop_item(instance_id);
        }
    }

    pub(super) fn cash_shop_hit_action(
        &mut self,
        cursor: Point,
    ) -> Option<cash_shop_ui::HitAction> {
        self.cash_shop.as_ref()?;
        self.cash_shop_hit_regions
            .iter()
            .rev()
            .find(|region| region.bounds.contains(cursor.x, cursor.y))
            .map(|region| region.action)
    }

    pub(super) fn cash_shop_max_page(&mut self) -> usize {
        let Some(state) = self.cash_shop.as_ref() else {
            return 0;
        };
        cash_filtered_indices(
            &self.cash_products,
            state,
            &self.string_archive,
            &mut self.item_name_cache,
        )
        .len()
        .saturating_sub(1)
            / cash_shop_ui::CATALOG_PAGE_SIZE
    }

    pub(super) fn refresh_npc_shop(&mut self) {
        let inventory = self.inventory.clone();
        let Some(shop) = self.npc_shop.as_mut() else {
            return;
        };
        let item_count = match shop.mode {
            NpcShopMode::Buy => shop.state.items.len(),
            NpcShopMode::Sell => inventory.as_ref().map_or(0, |inventory| {
                npc_shop_sellable_items(inventory, &self.item_archive, &self.character_archive)
                    .len()
            }),
        };
        let max_first_visible = item_count.saturating_sub(9);
        shop.first_visible = shop.first_visible.min(max_first_visible);
        if item_count == 0 {
            shop.selected = 0;
            shop.quantity = 1;
        } else {
            shop.selected = shop.selected.min(item_count - 1);
            if shop.selected < shop.first_visible
                || shop.selected >= shop.first_visible.saturating_add(9)
            {
                shop.selected = shop.first_visible;
            }
            let maximum = match shop.mode {
                NpcShopMode::Buy => shop.state.items[shop.selected].buyable.max(1),
                NpcShopMode::Sell => inventory
                    .as_ref()
                    .and_then(|inventory| {
                        npc_shop_sellable_items(
                            inventory,
                            &self.item_archive,
                            &self.character_archive,
                        )
                        .get(shop.selected)
                        .copied()
                    })
                    .map_or(1, |item| {
                        i16::try_from(item.count.max(1)).unwrap_or(i16::MAX)
                    }),
            };
            let maximum =
                if shop.mode == NpcShopMode::Buy && shop.state.items[shop.selected].buyable == 0 {
                    100
                } else {
                    maximum
                };
            shop.quantity = shop.quantity.clamp(1, maximum);
        }
        let state = shop.state.clone();
        let selected = shop.selected;
        let first_visible = shop.first_visible;
        let quantity = shop.quantity;
        let mode = shop.mode;
        let notice = shop.notice.clone();
        let confirmation = shop.confirmation;
        let (pixels, _) = render_npc_shop_panel(
            &self.font,
            &state,
            mode,
            selected,
            first_visible,
            quantity,
            inventory.as_ref(),
            notice.as_deref(),
            confirmation,
            &self.item_archive,
            &self.string_archive,
            &self.character_archive,
            &mut self.inventory_icon_cache,
            &mut self.item_name_cache,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = self
            .npc_shop
            .as_ref()
            .map(|shop| renderer.update_bgra(&shop.texture, &pixels));
        match result {
            Some(Ok(())) => {}
            Some(Err(error)) => eprintln!("OpenStory: could not update NPC shop: {error}"),
            None => {}
        }
    }

    pub(super) fn show_npc_shop_notice(&mut self, notice: impl Into<String>) {
        if let Some(shop) = self.npc_shop.as_mut() {
            shop.notice = Some(notice.into());
            self.refresh_npc_shop();
        }
    }

    pub(super) fn handle_npc_shop_transaction_result(&mut self, code: i8) {
        let notice = match code {
            0 => "Purchase completed.".to_owned(),
            2 => "You don't have enough mesos.".to_owned(),
            3 => "Your inventory is full.".to_owned(),
            5 => "You can't sell that item.".to_owned(),
            8 => "Shop transaction completed.".to_owned(),
            9 => "You don't have enough mesos to recharge.".to_owned(),
            _ => format!("Shop transaction failed (code {code})."),
        };
        if self.npc_shop.is_some() {
            self.show_npc_shop_notice(notice);
        } else if code != 0 && code != 8 {
            eprintln!("OpenStory: {notice}");
        }
    }

    pub(super) fn close_npc_shop(&mut self) {
        if self.npc_shop.take().is_none() {
            return;
        }
        if let Some(session) = self.session.as_ref() {
            if let Err(error) = session.send_packet(openstory_protocol::npc_shop_leave()) {
                eprintln!("OpenStory: could not close NPC shop: {error}");
                self.channel_connected = false;
            }
        }
    }

    pub(super) fn send_npc_shop_action(&mut self, packet: Vec<u8>, pending_notice: &str) {
        let result = self
            .session
            .as_ref()
            .map(|session| session.send_packet(packet));
        match result {
            Some(Ok(())) => self.show_npc_shop_notice(pending_notice),
            Some(Err(error)) => {
                eprintln!("OpenStory: could not send NPC shop action: {error}");
                self.channel_connected = false;
                self.show_npc_shop_notice("The server connection was lost.");
            }
            None => self.show_npc_shop_notice("Cannot transact while disconnected."),
        }
    }

    pub(super) fn buy_selected_npc_shop_item(&mut self) {
        let pitch_shortage = self.npc_shop.as_ref().and_then(|shop| {
            if shop.mode != NpcShopMode::Buy {
                return None;
            }
            let item = shop.state.items.get(shop.selected)?;
            if item.price > 0 || item.pitch <= 0 {
                return None;
            }
            let maximum = if item.buyable == 0 {
                100
            } else {
                item.buyable.max(1)
            };
            let quantity = i64::from(shop.quantity.clamp(1, maximum));
            let required = i64::from(item.pitch).saturating_mul(quantity);
            let available = self.inventory.as_ref().map_or(0, |inventory| {
                inventory_item_count(
                    inventory,
                    openstory_protocol::InventoryType::Etc,
                    PERFECT_PITCH_ITEM_ID,
                )
            });
            (i64::from(available) < required).then_some((required, available))
        });
        if let Some((required, available)) = pitch_shortage {
            self.show_npc_shop_notice(format!(
                "Need {required} Perfect Pitch; you have {available}."
            ));
            return;
        }
        let Some(shop) = self.npc_shop.as_ref() else {
            return;
        };
        let mode = shop.mode;
        let selected = shop.selected;
        let quantity = shop.quantity;
        let confirmation = match mode {
            NpcShopMode::Buy => shop.state.items.get(selected).and_then(|item| {
                (!item.rechargeable && item.buyable == 1 && (item.price > 0 || item.pitch > 0))
                    .then_some(NpcShopConfirmation {
                        mode,
                        slot: item.slot,
                        item_id: item.item_id,
                        quantity: 1,
                        recharge: false,
                    })
            }),
            NpcShopMode::Sell => self.inventory.as_ref().and_then(|inventory| {
                npc_shop_sellable_items(inventory, &self.item_archive, &self.character_archive)
                    .get(selected)
                    .copied()
                    .filter(|item| item.count == 1)
                    .and_then(|item| {
                        Some(NpcShopConfirmation {
                            mode,
                            slot: i16::try_from(item.slot).ok()?,
                            item_id: item.item_id,
                            quantity: 1,
                            recharge: false,
                        })
                    })
            }),
        };
        if let Some(confirmation) = confirmation {
            if let Some(shop) = self.npc_shop.as_mut() {
                shop.confirmation = Some(confirmation);
                shop.notice = None;
            }
            self.refresh_npc_shop();
            return;
        }
        let mut unsupported_currency = false;
        let Some(packet) = (|| match mode {
            NpcShopMode::Buy => {
                let item = shop.state.items.get(selected)?;
                if item.rechargeable {
                    return None;
                }
                if item.price <= 0 && item.pitch <= 0 {
                    unsupported_currency = true;
                    return None;
                }
                let maximum = if item.buyable == 0 {
                    100
                } else {
                    item.buyable.max(1)
                };
                openstory_protocol::npc_shop_item_action(
                    item.slot,
                    item.item_id,
                    quantity.clamp(1, maximum),
                    true,
                )
                .map_err(|error| {
                    eprintln!("OpenStory: invalid NPC shop purchase: {error}");
                })
                .ok()
            }
            NpcShopMode::Sell => {
                let inventory = self.inventory.as_ref()?;
                let item =
                    npc_shop_sellable_items(inventory, &self.item_archive, &self.character_archive)
                        .get(selected)
                        .copied()?;
                let slot = i16::try_from(item.slot).ok()?;
                openstory_protocol::npc_shop_item_action(
                    slot,
                    item.item_id,
                    quantity.clamp(1, i16::try_from(item.count.max(1)).unwrap_or(i16::MAX)),
                    false,
                )
                .map_err(|error| {
                    eprintln!("OpenStory: invalid NPC shop sale: {error}");
                })
                .ok()
            }
        })() else {
            if unsupported_currency {
                self.show_npc_shop_notice(
                    "This item requires a special currency that's not supported.",
                );
            }
            return;
        };
        let pending = if mode == NpcShopMode::Buy {
            "Purchase request sent; waiting for the server."
        } else {
            "Sale request sent; waiting for the server."
        };
        self.send_npc_shop_action(packet, pending);
    }

    pub(super) fn resolve_npc_shop_confirmation(&mut self, accepted: bool) {
        let confirmation = self
            .npc_shop
            .as_mut()
            .and_then(|shop| shop.confirmation.take());
        let Some(confirmation) = confirmation else {
            return;
        };
        if !accepted {
            self.show_npc_shop_notice("Transaction cancelled.");
            return;
        }
        let pitch_shortage = if confirmation.mode == NpcShopMode::Buy && !confirmation.recharge {
            self.npc_shop
                .as_ref()
                .and_then(|shop| {
                    shop.state.items.iter().find(|item| {
                        item.slot == confirmation.slot && item.item_id == confirmation.item_id
                    })
                })
                .filter(|item| item.price <= 0 && item.pitch > 0)
                .and_then(|item| {
                    let required = i64::from(item.pitch)
                        .saturating_mul(i64::from(confirmation.quantity.max(1)));
                    let available = self.inventory.as_ref().map_or(0, |inventory| {
                        inventory_item_count(
                            inventory,
                            openstory_protocol::InventoryType::Etc,
                            PERFECT_PITCH_ITEM_ID,
                        )
                    });
                    (i64::from(available) < required).then_some((required, available))
                })
        } else {
            None
        };
        if let Some((required, available)) = pitch_shortage {
            self.show_npc_shop_notice(format!(
                "Need {required} Perfect Pitch; you have {available}."
            ));
            return;
        }
        let packet = match confirmation.mode {
            NpcShopMode::Buy if confirmation.recharge => self
                .npc_shop
                .as_ref()
                .and_then(|shop| {
                    shop.state.items.iter().find(|item| {
                        item.slot == confirmation.slot
                            && item.item_id == confirmation.item_id
                            && item.rechargeable
                    })
                })
                .and_then(|_| openstory_protocol::npc_shop_recharge(confirmation.slot).ok()),
            NpcShopMode::Buy => self
                .npc_shop
                .as_ref()
                .and_then(|shop| {
                    shop.state.items.iter().find(|item| {
                        item.slot == confirmation.slot
                            && item.item_id == confirmation.item_id
                            && item.buyable == 1
                            && !item.rechargeable
                            && (item.price > 0 || item.pitch > 0)
                    })
                })
                .and_then(|_| {
                    openstory_protocol::npc_shop_item_action(
                        confirmation.slot,
                        confirmation.item_id,
                        confirmation.quantity,
                        true,
                    )
                    .ok()
                }),
            NpcShopMode::Sell => self.inventory.as_ref().and_then(|inventory| {
                npc_shop_sellable_items(inventory, &self.item_archive, &self.character_archive)
                    .into_iter()
                    .find(|item| {
                        i16::try_from(item.slot).ok() == Some(confirmation.slot)
                            && item.item_id == confirmation.item_id
                            && item.count >= confirmation.quantity as u16
                    })
                    .and_then(|_| {
                        openstory_protocol::npc_shop_item_action(
                            confirmation.slot,
                            confirmation.item_id,
                            confirmation.quantity,
                            false,
                        )
                        .ok()
                    })
            }),
        };
        let Some(packet) = packet else {
            self.show_npc_shop_notice("That item changed; select it again.");
            return;
        };
        let pending = if confirmation.recharge {
            "Recharge request sent; waiting for the server."
        } else if confirmation.mode == NpcShopMode::Buy {
            "Purchase request sent; waiting for the server."
        } else {
            "Sale request sent; waiting for the server."
        };
        self.send_npc_shop_action(packet, pending);
    }

    pub(super) fn recharge_selected_npc_shop_item(&mut self) {
        let Some(shop) = self.npc_shop.as_ref() else {
            return;
        };
        let Some(item) = shop.state.items.get(shop.selected) else {
            return;
        };
        if shop.mode != NpcShopMode::Buy || !item.rechargeable {
            return;
        }
        let confirmation = NpcShopConfirmation {
            mode: NpcShopMode::Buy,
            slot: item.slot,
            item_id: item.item_id,
            quantity: 1,
            recharge: true,
        };
        if let Some(shop) = self.npc_shop.as_mut() {
            shop.confirmation = Some(confirmation);
            shop.notice = None;
        }
        self.refresh_npc_shop();
    }

    pub(super) fn click_npc_shop(&mut self, cursor: Point) {
        const PANEL_X: f32 = 202.0;
        const PANEL_Y: f32 = 124.0;
        if !rect_contains(
            SpriteRect {
                x: PANEL_X,
                y: PANEL_Y,
                width: 620.0,
                height: 520.0,
                blend_mode: BlendMode::Alpha,
            },
            cursor,
        ) {
            return;
        }
        let x = cursor.x - PANEL_X;
        let y = cursor.y - PANEL_Y;
        if self
            .npc_shop
            .as_ref()
            .is_some_and(|shop| shop.confirmation.is_some())
        {
            if (280.0..316.0).contains(&y) && (150.0..280.0).contains(&x) {
                self.resolve_npc_shop_confirmation(true);
            } else if (280.0..316.0).contains(&y) && (340.0..470.0).contains(&x) {
                self.resolve_npc_shop_confirmation(false);
            }
            return;
        }
        if (584.0..610.0).contains(&x) && (9.0..35.0).contains(&y) {
            self.close_npc_shop();
            return;
        }
        if (9.0..35.0).contains(&y) && (420.0..482.0).contains(&x) {
            if let Some(shop) = self.npc_shop.as_mut() {
                shop.mode = NpcShopMode::Buy;
                shop.selected = 0;
                shop.first_visible = 0;
                shop.quantity = 1;
                shop.notice = None;
            }
            self.refresh_npc_shop();
            return;
        }
        if (9.0..35.0).contains(&y) && (490.0..550.0).contains(&x) {
            if let Some(shop) = self.npc_shop.as_mut() {
                shop.mode = NpcShopMode::Sell;
                shop.selected = 0;
                shop.first_visible = 0;
                shop.quantity = 1;
                shop.notice = None;
            }
            self.refresh_npc_shop();
            return;
        }
        if (66.0..426.0).contains(&y) && (12.0..608.0).contains(&x) {
            let row = ((y - 66.0) / 40.0) as usize;
            let Some(shop) = self.npc_shop.as_mut() else {
                return;
            };
            let index = shop.first_visible.saturating_add(row);
            let item_count = match shop.mode {
                NpcShopMode::Buy => shop.state.items.len(),
                NpcShopMode::Sell => self.inventory.as_ref().map_or(0, |inventory| {
                    npc_shop_sellable_items(inventory, &self.item_archive, &self.character_archive)
                        .len()
                }),
            };
            if index < item_count {
                shop.selected = index;
                shop.quantity = 1;
                shop.notice = None;
                self.refresh_npc_shop();
            }
            return;
        }
        if !(450.0..494.0).contains(&y) {
            return;
        }
        if (270.0..306.0).contains(&x) {
            if let Some(shop) = self.npc_shop.as_mut() {
                shop.quantity = shop.quantity.saturating_sub(1).max(1);
            }
            self.refresh_npc_shop();
        } else if (310.0..346.0).contains(&x) {
            if let Some(shop) = self.npc_shop.as_mut() {
                let maximum = match shop.mode {
                    NpcShopMode::Buy => shop.state.items.get(shop.selected).map(|item| {
                        if item.buyable == 0 {
                            100
                        } else {
                            item.buyable.max(1)
                        }
                    }),
                    NpcShopMode::Sell => self.inventory.as_ref().and_then(|inventory| {
                        npc_shop_sellable_items(
                            inventory,
                            &self.item_archive,
                            &self.character_archive,
                        )
                        .get(shop.selected)
                        .map(|item| i16::try_from(item.count.max(1)).unwrap_or(i16::MAX))
                    }),
                };
                if let Some(maximum) = maximum {
                    shop.quantity = shop.quantity.saturating_add(1).min(maximum);
                }
            }
            self.refresh_npc_shop();
        } else if (354.0..434.0).contains(&x) {
            if self
                .npc_shop
                .as_ref()
                .is_some_and(|shop| shop.mode == NpcShopMode::Buy)
            {
                self.recharge_selected_npc_shop_item();
            }
        } else if (442.0..520.0).contains(&x) {
            self.buy_selected_npc_shop_item();
        } else if (528.0..608.0).contains(&x) {
            self.close_npc_shop();
        }
    }

    pub(super) fn scroll_npc_shop(&mut self, amount: f32) {
        if amount == 0.0 {
            return;
        }
        let inventory = self.inventory.clone();
        if let Some(shop) = self.npc_shop.as_mut() {
            let item_count = match shop.mode {
                NpcShopMode::Buy => shop.state.items.len(),
                NpcShopMode::Sell => inventory.as_ref().map_or(0, |inventory| {
                    npc_shop_sellable_items(inventory, &self.item_archive, &self.character_archive)
                        .len()
                }),
            };
            let maximum = item_count.saturating_sub(9);
            shop.first_visible = if amount > 0.0 {
                shop.first_visible.saturating_sub(1)
            } else {
                shop.first_visible.saturating_add(1).min(maximum)
            };
            if shop.selected < shop.first_visible
                || shop.selected >= shop.first_visible.saturating_add(9)
            {
                shop.selected = shop.first_visible.min(item_count.saturating_sub(1));
                shop.quantity = 1;
            }
        }
        self.refresh_npc_shop();
    }

    pub(super) fn rebuild_npc_dialogue(&mut self) {
        let Some(ui) = self.npc_dialogue.as_ref() else {
            return;
        };
        let state = ui.state.clone();
        let dialogue_text = ui.text.clone();
        let inline_icons = ui.inline_icons.clone();
        let selections = ui.selections.clone();
        let selected_selection = ui.selected_selection;
        let first_visible_selection = ui.first_visible_selection;
        let input = ui.input.clone();
        let text_scroll = ui.text_scroll;
        let text = if state.message_type == 4 {
            parse_npc_selections(&dialogue_text).0
        } else {
            dialogue_text.chars().take(ui.visible_text_chars).collect()
        };
        let (pixels, panel, buttons, text_max_scroll) = render_npc_dialogue(
            &self.font,
            state.npc_id,
            state.speaker,
            state.message_type,
            &text,
            text_scroll,
            &inline_icons,
            &selections,
            selected_selection,
            first_visible_selection,
            &input,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        match renderer.upload_bgra(600, 260, &pixels) {
            Ok(texture) => {
                if let Some(ui) = self.npc_dialogue.as_mut() {
                    ui.text_max_scroll = text_max_scroll;
                    ui.text_scroll = ui.text_scroll.min(ui.text_max_scroll);
                    ui.texture = texture;
                    ui.panel = panel;
                    ui.buttons = buttons;
                }
            }
            Err(error) => eprintln!("OpenStory: could not update NPC dialogue: {error}"),
        }
    }

    pub(super) fn send_npc_dialogue_action(&mut self, action: NpcDialogueAction) {
        let Some(ui) = self.npc_dialogue.as_ref() else {
            return;
        };
        let packet = match action {
            NpcDialogueAction::More(response) => Ok(openstory_protocol::npc_talk_more(
                ui.state.message_type,
                response,
            )),
            NpcDialogueAction::Select(selection) => {
                Ok(openstory_protocol::npc_talk_selection(selection))
            }
            NpcDialogueAction::SubmitText => openstory_protocol::npc_talk_text(&ui.input),
            NpcDialogueAction::SubmitNumber => {
                let mut value = ui
                    .input
                    .parse::<i32>()
                    .unwrap_or_else(|_| ui.state.number_bounds.map_or(0, |bounds| bounds.0));
                if let Some((_, minimum, maximum)) = ui.state.number_bounds {
                    if minimum <= maximum {
                        value = value.clamp(minimum, maximum);
                    }
                }
                Ok(openstory_protocol::npc_talk_number(value))
            }
            NpcDialogueAction::AdjustNumber(delta) => {
                let mut value = ui.input.parse::<i32>().unwrap_or(0).saturating_add(delta);
                if let Some((_, minimum, maximum)) = ui.state.number_bounds {
                    if minimum <= maximum {
                        value = value.clamp(minimum, maximum);
                    }
                }
                if let Some(ui) = self.npc_dialogue.as_mut() {
                    ui.input = value.to_string();
                }
                self.rebuild_npc_dialogue();
                return;
            }
        };
        match packet {
            Ok(packet) => {
                if let Some(session) = self.session.as_ref() {
                    if let Err(error) = session.send_packet(packet) {
                        eprintln!("OpenStory: could not respond to NPC dialogue: {error}");
                        self.channel_connected = false;
                    }
                }
                self.npc_dialogue = None;
                self.npc_dialogue_scroll_dragging = false;
            }
            Err(error) => eprintln!("OpenStory: invalid NPC dialogue response: {error}"),
        }
    }

    pub(super) fn click_npc_dialogue(&mut self, cursor: Point) {
        if self.set_npc_dialogue_scroll_from_cursor(cursor) {
            self.npc_dialogue_scroll_dragging = true;
            return;
        }
        if self.fast_forward_npc_dialogue_text() {
            return;
        }
        let action = self.npc_dialogue.as_ref().and_then(|ui| {
            ui.buttons
                .iter()
                .find(|button| rect_contains(button.rect, cursor))
                .map(|button| button.action)
        });
        if let Some(action) = action {
            self.send_npc_dialogue_action(action);
        }
    }

    pub(super) fn set_npc_dialogue_scroll_from_cursor(&mut self, cursor: Point) -> bool {
        let scroll_to = self.npc_dialogue.as_ref().and_then(|dialogue| {
            if dialogue.text_max_scroll == 0
                || !(dialogue.panel.x + 578.0..dialogue.panel.x + 592.0).contains(&cursor.x)
                || !(dialogue.panel.y + 50.0
                    ..dialogue.panel.y
                        + 50.0
                        + npc_dialogue_text_view_height(dialogue.state.message_type) as f32)
                    .contains(&cursor.y)
            {
                return None;
            }
            let fraction = ((cursor.y - (dialogue.panel.y + 50.0))
                / npc_dialogue_text_view_height(dialogue.state.message_type) as f32)
                .clamp(0.0, 1.0);
            Some((fraction * dialogue.text_max_scroll as f32).round() as usize)
        });
        if let Some(scroll) = scroll_to {
            if let Some(dialogue) = self.npc_dialogue.as_mut() {
                dialogue.text_scroll = scroll;
            }
            self.rebuild_npc_dialogue();
            return true;
        }
        false
    }

    pub(super) fn scroll_npc_dialogue_text(&mut self, direction: isize) {
        let Some(dialogue) = self.npc_dialogue.as_mut() else {
            return;
        };
        if dialogue.state.message_type == 4 || dialogue.text_max_scroll == 0 {
            return;
        }
        let delta = direction
            .unsigned_abs()
            .saturating_mul(NPC_DIALOGUE_SCROLL_STEP);
        let next = if direction < 0 {
            dialogue.text_scroll.saturating_sub(delta)
        } else {
            dialogue
                .text_scroll
                .saturating_add(delta)
                .min(dialogue.text_max_scroll)
        };
        if next != dialogue.text_scroll {
            dialogue.text_scroll = next;
            self.rebuild_npc_dialogue();
        }
    }

    pub(super) fn fast_forward_npc_dialogue_text(&mut self) -> bool {
        let Some(dialogue) = self.npc_dialogue.as_mut() else {
            return false;
        };
        let total_chars = dialogue.text.chars().count();
        if dialogue.state.message_type == 4 || dialogue.visible_text_chars >= total_chars {
            return false;
        }
        dialogue.visible_text_chars = total_chars;
        dialogue.reveal_elapsed = Duration::ZERO;
        self.rebuild_npc_dialogue();
        true
    }

    pub(super) fn advance_npc_dialogue_text(&mut self, elapsed: Duration) {
        const CHAR_INTERVAL: Duration = Duration::from_millis(48);
        let mut changed = false;
        if let Some(dialogue) = self.npc_dialogue.as_mut()
            && dialogue.state.message_type != 4
            && dialogue.visible_text_chars < dialogue.text.chars().count()
        {
            dialogue.reveal_elapsed = dialogue.reveal_elapsed.saturating_add(elapsed);
            let reveal_count =
                (dialogue.reveal_elapsed.as_millis() / CHAR_INTERVAL.as_millis()) as usize;
            if reveal_count > 0 {
                let reveal_ticks = reveal_count.min(u32::MAX as usize) as u32;
                dialogue.reveal_elapsed = dialogue
                    .reveal_elapsed
                    .saturating_sub(CHAR_INTERVAL.saturating_mul(reveal_ticks));
                dialogue.visible_text_chars = dialogue
                    .visible_text_chars
                    .saturating_add(reveal_count)
                    .min(dialogue.text.chars().count());
                changed = true;
            }
        }
        if changed {
            self.rebuild_npc_dialogue();
        }
    }

    pub(super) fn cancel_npc_dialogue(&mut self) {
        let response = self
            .npc_dialogue
            .as_ref()
            .map(|ui| match ui.state.message_type {
                0 | 1 | 5 | 6 => -1,
                2 | 3 | 4 | 12 => 0,
                _ => 0,
            });
        if let Some(response) = response {
            self.send_npc_dialogue_action(NpcDialogueAction::More(response));
        }
    }

    pub(super) fn advance_npc_dialogue(&mut self) {
        let action = self
            .npc_dialogue
            .as_ref()
            .and_then(|ui| match ui.state.message_type {
                0 | 1 | 5 | 6 | 12 => Some(NpcDialogueAction::More(1)),
                2 => Some(NpcDialogueAction::SubmitText),
                3 => Some(NpcDialogueAction::SubmitNumber),
                4 => ui
                    .selections
                    .get(ui.selected_selection)
                    .map(|(selection, _)| NpcDialogueAction::Select(*selection)),
                _ => ui.buttons.first().map(|button| button.action),
            });
        if let Some(action) = action {
            self.send_npc_dialogue_action(action);
        }
    }

    pub(super) fn move_npc_dialogue_selection(&mut self, direction: isize) {
        let Some(dialogue) = self.npc_dialogue.as_mut() else {
            return;
        };
        if dialogue.state.message_type != 4 || dialogue.selections.is_empty() {
            return;
        }
        let last_selection = dialogue.selections.len().saturating_sub(1);
        dialogue.selected_selection = dialogue
            .selected_selection
            .saturating_add_signed(direction)
            .min(last_selection);
        if dialogue.selected_selection < dialogue.first_visible_selection {
            dialogue.first_visible_selection = dialogue.selected_selection;
        } else if dialogue.selected_selection >= dialogue.first_visible_selection + 5 {
            dialogue.first_visible_selection = dialogue.selected_selection + 1 - 5;
        }
        self.rebuild_npc_dialogue();
    }

    pub(super) fn spawn_drop(&mut self, spawn: openstory_protocol::MapDropSpawn) {
        if self
            .map_scene
            .as_mut()
            .is_some_and(|scene| scene.refresh_existing_drop(spawn))
        {
            return;
        }
        let mut sprites = Vec::with_capacity(4);
        let assets = match append_drop_icons(
            &mut sprites,
            &self.item_archive,
            &self.character_archive,
            spawn,
            &mut self.remote_art_cache,
        ) {
            Ok(assets) => assets,
            Err(error) => {
                eprintln!(
                    "OpenStory: could not load drop item {}: {error}",
                    spawn.item_id
                );
                return;
            }
        };
        let draws = match self.upload_dynamic_sprites(sprites) {
            Ok(draws) => draws,
            Err(error) => {
                eprintln!("OpenStory: could not upload drop art: {error}");
                return;
            }
        };
        let frames = assets
            .into_iter()
            .filter_map(|frame| {
                draws
                    .get(frame.sprite_index)
                    .copied()
                    .map(|sprite| DropFrameRuntime {
                        delay: Duration::from_millis(u64::from(frame.delay_ms.max(1))),
                        sprite,
                    })
            })
            .collect::<Vec<_>>();
        if frames.is_empty() {
            eprintln!("OpenStory: drop item {} has no icon", spawn.item_id);
            return;
        }
        if let Some(scene) = self.map_scene.as_mut() {
            scene.spawn_drop(spawn, frames);
        }
    }

    pub(super) fn remove_drop(&mut self, update: openstory_protocol::MapDropRemove) {
        if let Some(scene) = self.map_scene.as_mut() {
            scene.remove_drop(update);
        }
    }

    pub(super) fn spawn_mob(&mut self, spawn: openstory_protocol::MapMobSpawn) {
        if self
            .map_scene
            .as_mut()
            .is_some_and(|scene| scene.refresh_existing_mob(&spawn))
        {
            return;
        }
        let mut sprites = Vec::new();
        let (assets, no_flip, faces_right, traits, skill_actions) = match append_mob_animations(
            &mut sprites,
            &self.mob_archive,
            spawn.mob_id,
            &mut self.remote_art_cache,
        ) {
            Ok(assets) => assets,
            Err(error) => {
                eprintln!("OpenStory: could not load mob {}: {error}", spawn.mob_id);
                return;
            }
        };
        let draws = match self.upload_dynamic_sprites(sprites) {
            Ok(draws) => draws,
            Err(error) => {
                eprintln!(
                    "OpenStory: could not upload mob {} art: {error}",
                    spawn.mob_id
                );
                return;
            }
        };
        let animations = mob_frames_runtime(assets, &draws);
        let Some(motion) = first_mob_motion(&animations, mob_motion_for_stance(spawn.stance))
        else {
            eprintln!("OpenStory: mob {} has no drawable animation", spawn.mob_id);
            return;
        };
        let mut body = PhysicsBody::new(f64::from(spawn.x), f64::from(spawn.y));
        body.foothold_id = spawn.foothold;
        body.mode = if traits.can_fly {
            PhysicsMode::Flying
        } else {
            PhysicsMode::Normal
        };
        body.flags = PhysicsFlags {
            turn_at_edges: true,
            ..Default::default()
        };
        let mut mob = MapMobRuntime {
            mob_id: spawn.mob_id,
            x: f64::from(spawn.x),
            y: f64::from(spawn.y),
            body,
            foothold: spawn.foothold,
            control_mode: spawn.control_mode,
            server_flip: spawn.stance & 1 == 0,
            no_flip,
            faces_right,
            traits,
            motion,
            server_stance: spawn.stance,
            frame_index: 0,
            elapsed: Duration::ZERO,
            animations,
            skill_actions,
            hit_effects: Vec::with_capacity(2),
            dying: false,
            fade_remaining: None,
            fade_in_remaining: spawn.is_new_spawn.then_some(MOB_FADE_DURATION),
            hp_percent: None,
            hp_bar_remaining: Duration::ZERO,
            statuses: HashSet::new(),
            control_acked: false,
            ai_counter: 0,
            random_state: ((spawn.object_id as u32).wrapping_mul(0x9E37_79B9)
                ^ spawn.mob_id as u32
                ^ 0xA341_316C)
                .max(1),
            fly_direction: 0,
        };
        if spawn.control_mode > 0 && mob.motion == MobMotion::Stand {
            mob.choose_next_move();
        }
        if let Some(scene) = self.map_scene.as_mut() {
            scene.mobs.insert(spawn.object_id, mob);
            if std::env::var("OPENSTORY_DEBUG_MOBS").as_deref() == Ok("1") {
                eprintln!(
                    "OpenStory: spawned mob {} as object {} at ({}, {}) control={} new={}",
                    spawn.mob_id,
                    spawn.object_id,
                    spawn.x,
                    spawn.y,
                    spawn.control_mode,
                    spawn.is_new_spawn
                );
            }
        }
    }

    pub(super) fn spawn_summon(&mut self, spawn: openstory_protocol::MapSummonSpawn) {
        if let Some(summon) = self
            .map_scene
            .as_mut()
            .and_then(|scene| scene.summons.get_mut(&spawn.object_id))
            && summon.owner_id == spawn.owner_id
            && summon.skill_id == spawn.skill_id
        {
            summon.x = f64::from(spawn.x);
            summon.y = f64::from(spawn.y);
            summon.body.x = summon.x;
            summon.body.y = summon.y;
            summon.body.previous_x = summon.x;
            summon.body.previous_y = summon.y;
            summon.stance = spawn.stance;
            summon.skill_level = spawn.skill_level;
            summon.movement_type = spawn.movement_type;
            summon.body.mode = if matches!(spawn.movement_type, 1 | 3) {
                PhysicsMode::Normal
            } else {
                PhysicsMode::Fixated
            };
            summon.attacks = spawn.attacks;
            return;
        }
        let frames = match self.load_summon_frames(spawn.skill_id) {
            Ok(frames) => frames,
            Err(error) => {
                eprintln!(
                    "OpenStory: could not load summon skill {} art: {error}",
                    spawn.skill_id
                );
                return;
            }
        };
        if frames.is_empty() {
            eprintln!(
                "OpenStory: summon skill {} has no drawable animation",
                spawn.skill_id
            );
            return;
        }
        let mut body = PhysicsBody::new(f64::from(spawn.x), f64::from(spawn.y));
        body.mode = if matches!(spawn.movement_type, 1 | 3) {
            PhysicsMode::Normal
        } else {
            PhysicsMode::Fixated
        };
        if let Some(scene) = self.map_scene.as_mut() {
            scene.summons.insert(
                spawn.object_id,
                MapSummonRuntime {
                    owner_id: spawn.owner_id,
                    skill_id: spawn.skill_id,
                    skill_level: spawn.skill_level,
                    x: f64::from(spawn.x),
                    y: f64::from(spawn.y),
                    body,
                    stance: spawn.stance,
                    movement_type: spawn.movement_type,
                    attacks: spawn.attacks,
                    frames,
                    frame_index: 0,
                    elapsed: Duration::ZERO,
                },
            );
        }
    }

    pub(super) fn spawn_pet(&mut self, spawn: openstory_protocol::MapPetSpawn) {
        let animations = match self.load_pet_animations(spawn.item_id) {
            Ok(animations) => animations,
            Err(error) => {
                eprintln!(
                    "OpenStoryRS: could not load pet {} art: {error}",
                    spawn.item_id
                );
                return;
            }
        };
        if animations.is_empty() {
            eprintln!(
                "OpenStoryRS: pet {} has no drawable animation",
                spawn.item_id
            );
            return;
        }
        let name_label = if spawn.name.is_empty() {
            None
        } else {
            let mut sprites = Vec::with_capacity(1);
            append_text_sprite(
                &mut sprites,
                &self.font,
                &spawn.name,
                centered_text_x(&self.font, &spawn.name, 13.0),
                340.0,
                13.0,
            );
            match self.upload_dynamic_sprites(sprites) {
                Ok(draws) => draws.first().copied(),
                Err(error) => {
                    eprintln!("OpenStoryRS: could not load pet name label: {error}");
                    None
                }
            }
        };
        let mut body = PhysicsBody::new(f64::from(spawn.x), f64::from(spawn.y));
        body.on_ground = true;
        body.foothold_id = u16::try_from(spawn.foothold).unwrap_or_default();
        let Some(scene) = self.map_scene.as_mut() else {
            return;
        };
        scene.pets.insert(
            (spawn.character_id, spawn.slot),
            MapPetRuntime {
                owner_id: spawn.character_id,
                slot: spawn.slot,
                item_id: spawn.item_id,
                unique_id: spawn.unique_id,
                name_label,
                x: f64::from(spawn.x),
                y: f64::from(spawn.y),
                body,
                loot_target: None,
                stance: spawn.stance,
                animations,
                frame_index: 0,
                elapsed: Duration::ZERO,
                command_remaining: Duration::ZERO,
                interaction: None,
                speech_bubble_remaining: None,
            },
        );
    }

    pub(super) fn move_pet(&mut self, movement: openstory_protocol::MapPetMovement) {
        let Some(pet) = self
            .map_scene
            .as_mut()
            .and_then(|scene| scene.pets.get_mut(&(movement.character_id, movement.slot)))
        else {
            return;
        };
        if pet.owner_id != movement.character_id || pet.slot != movement.slot {
            return;
        }
        if std::env::var("OPENSTORY_DEBUG_PETS").as_deref() == Ok("1") {
            eprintln!(
                "OpenStoryRS: pet movement owner={} slot={} item={} unique={}",
                pet.owner_id, pet.slot, pet.item_id, pet.unique_id
            );
        }
        if let Some(fragment) = movement.fragments.last() {
            match fragment.kind {
                openstory_protocol::MovementKind::Absolute
                | openstory_protocol::MovementKind::Teleport
                | openstory_protocol::MovementKind::JumpDown => {
                    pet.x = f64::from(fragment.x);
                    pet.y = f64::from(fragment.y);
                }
                openstory_protocol::MovementKind::Relative => {
                    pet.x += f64::from(fragment.x);
                    pet.y += f64::from(fragment.y);
                }
                openstory_protocol::MovementKind::None => {}
            }
            if fragment.kind != openstory_protocol::MovementKind::None {
                pet.set_stance(fragment.stance);
                pet.body.x = pet.x;
                pet.body.y = pet.y;
                pet.body.previous_x = pet.x;
                pet.body.previous_y = pet.y;
                pet.body.foothold_id = fragment.foothold;
            }
        }
    }

    pub(super) fn handle_pet_command(&mut self, update: openstory_protocol::PetCommandUpdate) {
        let key = (update.character_id, update.slot);
        let Some((item_id, x, y)) = self
            .map_scene
            .as_ref()
            .and_then(|scene| scene.pets.get(&key))
            .map(|pet| (pet.item_id, rounded_i16(pet.x), rounded_i16(pet.y)))
        else {
            return;
        };
        let feed = update.interaction_type == 1;
        let success = !update.talk;
        let interaction_frames = match self.load_pet_interaction_frames(
            item_id,
            feed,
            update.animation as u8,
            success,
        ) {
            Ok(frames) => frames,
            Err(error) => {
                eprintln!("OpenStoryRS: could not load pet interaction art: {error}");
                None
            }
        };
        if let Some(pet) = self
            .map_scene
            .as_mut()
            .and_then(|scene| scene.pets.get_mut(&key))
        {
            if !interaction_frames.is_some_and(|frames| pet.play_interaction(frames)) {
                pet.play_command(update.animation);
            }
        }
        let sound = if feed {
            "eat"
        } else if update.talk {
            "chat"
        } else {
            "alert"
        };
        self.audio.play_world_sound(
            &self.sound_archive,
            &format!("Pet.img/{item_id}/{sound}"),
            (x, y),
        );
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn remove_pet(&mut self, character_id: i32, slot: u8) {
        if let Some(scene) = self.map_scene.as_mut() {
            scene.pets.remove(&(character_id, slot));
        }
        self.pet_speech_textures.remove(&(character_id, slot));
    }

    pub(super) fn move_summon(&mut self, movement: openstory_protocol::MapSummonMovement) {
        let Some(scene) = self.map_scene.as_mut() else {
            return;
        };
        let Some(summon) = scene.summons.get_mut(&movement.object_id) else {
            return;
        };
        if summon.owner_id != movement.owner_id {
            return;
        }
        if let Some(fragment) = movement.fragments.last() {
            match fragment.kind {
                openstory_protocol::MovementKind::Absolute
                | openstory_protocol::MovementKind::Teleport
                | openstory_protocol::MovementKind::JumpDown => {
                    summon.x = f64::from(fragment.x);
                    summon.y = f64::from(fragment.y);
                }
                openstory_protocol::MovementKind::Relative => {
                    summon.x += f64::from(fragment.x);
                    summon.y += f64::from(fragment.y);
                }
                openstory_protocol::MovementKind::None => {}
            }
            if fragment.kind != openstory_protocol::MovementKind::None {
                summon.stance = fragment.stance;
                summon.body.foothold_id = fragment.foothold;
                summon.body.x = summon.x;
                summon.body.y = summon.y;
                summon.body.previous_x = summon.x;
                summon.body.previous_y = summon.y;
            }
        }
    }

    pub(super) fn remove_summon(&mut self, update: openstory_protocol::MapSummonRemove) {
        if let Some(scene) = self.map_scene.as_mut()
            && scene
                .summons
                .get(&update.object_id)
                .is_some_and(|summon| summon.owner_id == update.owner_id)
        {
            scene.summons.remove(&update.object_id);
        }
    }

    pub(super) fn update_summon_skill(&mut self, update: openstory_protocol::SummonSkillUpdate) {
        if let Some(scene) = self.map_scene.as_mut() {
            for summon in scene.summons.values_mut().filter(|summon| {
                summon.owner_id == update.owner_id && summon.skill_id == update.skill_id
            }) {
                summon.stance = update.stance;
            }
        }
    }

    pub(super) fn handle_mob_controller(
        &mut self,
        update: openstory_protocol::MobControllerUpdate,
    ) {
        match update {
            openstory_protocol::MobControllerUpdate::Control { object_id, mode } => {
                if let Some(scene) = self.map_scene.as_mut() {
                    scene.set_mob_control(object_id, mode);
                }
            }
            openstory_protocol::MobControllerUpdate::Spawn(spawn) => self.spawn_mob(spawn),
            openstory_protocol::MobControllerUpdate::NoSpawnData { .. } => {}
        }
    }

    pub(super) fn update_remote_character_look(
        &mut self,
        update: openstory_protocol::RemoteCharacterLookUpdate,
    ) {
        if !self
            .map_scene
            .as_ref()
            .is_some_and(|scene| scene.remote_players.contains_key(&update.id))
        {
            return;
        }
        let name = self
            .map_scene
            .as_ref()
            .and_then(|scene| scene.remote_players.get(&update.id))
            .map(|player| player.name.clone())
            .unwrap_or_default();
        let (avatar_frames, name_label) = match self.load_remote_avatar(&update.look, &name) {
            Ok(loaded) => loaded,
            Err(error) => {
                eprintln!("OpenStory: could not update remote character appearance: {error}");
                return;
            }
        };
        let item_effect_id = equipped_ring_effect_id(&update.look, &self.effect_archive);
        let item_aura = match self.load_persistent_item_aura(item_effect_id) {
            Ok(aura) => aura,
            Err(error) => {
                eprintln!(
                    "OpenStory: could not refresh remote item aura {item_effect_id}: {error}"
                );
                None
            }
        };
        if let Some(player) = self
            .map_scene
            .as_mut()
            .and_then(|scene| scene.remote_players.get_mut(&update.id))
        {
            player.look = update.look;
            player.face_expression = None;
            player.item_aura = item_aura;
            player.avatar_frames = avatar_frames;
            player.active_avatar_frame =
                first_avatar_frame(&player.avatar_frames, player.desired_avatar_motion);
            player.avatar_elapsed = Duration::ZERO;
            player.name_label = name_label;
        }
    }

    pub(super) fn update_remote_mini_room(
        &mut self,
        update: openstory_protocol::RemoteCharacterMiniRoomUpdate,
    ) {
        if let Some(player) = self
            .map_scene
            .as_mut()
            .and_then(|scene| scene.remote_players.get_mut(&update.character_id))
        {
            player.mini_room = update.mini_room;
        }
    }

    pub(super) fn load_field_effect_frames(
        &mut self,
        path: &str,
    ) -> Result<Vec<ProjectileFrameRuntime>, String> {
        let archive_path = if let Some(path) = path.strip_prefix("Map/") {
            path.to_owned()
        } else if path.starts_with("Effect.img/") {
            path.to_owned()
        } else {
            format!("Effect.img/{path}")
        };
        if let Some(frames) = self.field_effect_cache.get(&archive_path) {
            return Ok(frames.clone());
        }
        let Some(source) = self
            .map_archive
            .root()
            .resolve(&archive_path)
            .map_err(|error| error.to_string())?
        else {
            return Ok(Vec::new());
        };
        let mut sprites = Vec::new();
        let assets = append_animation_frames_from_node_at(
            &mut sprites,
            &source,
            &format!("field-effect:{archive_path}"),
            512.0,
            250.0,
            &mut self.remote_art_cache,
        )?;
        let frames = if assets.is_empty() {
            Vec::new()
        } else {
            let draws = self.upload_dynamic_sprites(sprites)?;
            projectile_frames_runtime(assets, &draws)
        };
        if self.field_effect_cache.len() >= 64 {
            self.field_effect_cache.clear();
        }
        self.field_effect_cache.insert(archive_path, frames.clone());
        Ok(frames)
    }

    pub(super) fn apply_field_effect(&mut self, effect: openstory_protocol::FieldEffect) {
        match effect {
            openstory_protocol::FieldEffect::ScreenAnimation(path) => {
                let frames = match self.load_field_effect_frames(&path) {
                    Ok(frames) => frames,
                    Err(error) => {
                        eprintln!("OpenStory: could not load field effect {path:?}: {error}");
                        Vec::new()
                    }
                };
                if let Some(scene) = self.map_scene.as_mut() {
                    scene.show_screen_effect(frames);
                }
            }
            openstory_protocol::FieldEffect::Sound(path) => self
                .audio
                .play_field_effect_sound(&self.sound_archive, &path),
            openstory_protocol::FieldEffect::Music(path) => {
                self.audio.play_music(&self.sound_archive, &path);
            }
        }
    }

    pub(super) fn apply_blow_weather(&mut self, update: openstory_protocol::BlowWeatherUpdate) {
        if update.kind == 0 || update.item_id <= 0 {
            self.weather_overlay.clear();
            self.weather_texture = None;
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
            return;
        }

        let loaded = (|| -> Result<(String, GpuTexture), String> {
            let item_info_path = format!("Cash/0512.img/0{}/info", update.item_id);
            let item_info = self
                .item_archive
                .root()
                .resolve(&item_info_path)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("cash weather item {} is missing", update.item_id))?;
            let authored_path = string_property(&item_info, "path")
                .filter(|path| !path.trim().is_empty())
                .ok_or_else(|| format!("cash weather item {} has no map path", update.item_id))?;
            let archive_path = authored_path.strip_prefix("Map/").unwrap_or(&authored_path);
            let weather_node = self
                .map_archive
                .root()
                .resolve(archive_path)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("weather asset {archive_path:?} is missing"))?;
            let frame_node = weather_node
                .child("0")
                .map_err(|error| error.to_string())?
                .unwrap_or(weather_node);
            let bitmap = frame_node
                .bitmap()
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("weather asset {archive_path:?} is not an image"))?;
            let width = u32::from(bitmap.width());
            let height = u32::from(bitmap.height());
            let pixels = bitmap
                .decode_bgra_shared()
                .map_err(|error| error.to_string())?;
            let renderer = self
                .renderer
                .as_ref()
                .ok_or_else(|| "graphics are not ready for weather assets".to_owned())?;
            let texture = renderer
                .upload_bgra(width, height, &pixels)
                .map_err(|error| error.to_string())?;
            Ok((authored_path, texture))
        })();

        match loaded {
            Ok((path, texture)) => {
                self.weather_overlay.start(&path, 1024, 768);
                self.weather_texture = Some(texture);
                if !update.message.trim().is_empty() {
                    self.set_scrolling_notice(update.message.trim());
                }
            }
            Err(error) => {
                eprintln!("OpenStoryRS: could not load weather: {error}");
                self.weather_overlay.clear();
                self.weather_texture = None;
            }
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn set_scrolling_notice(&mut self, message: &str) {
        if message.is_empty() {
            return;
        }
        let font_size = 14.0;
        let width = measured_text_width(&self.font, message, font_size)
            .ceil()
            .clamp(1.0, 8190.0) as u32
            + 2;
        let pixels = rasterize_label(&self.font, message, font_size, width, 24);
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        match renderer.upload_bgra(width, 24, &pixels) {
            Ok(texture) => {
                self.scrolling_notice = Some(ScrollingNoticeState {
                    texture,
                    width: width as f32,
                    x: 1024.0,
                });
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            Err(error) => eprintln!("OpenStoryRS: could not create scrolling notice: {error}"),
        }
    }

    pub(super) fn advance_scrolling_notice(&mut self, updates: u8) {
        if updates == 0 {
            return;
        }
        if let Some(notice) = self.scrolling_notice.as_mut() {
            notice.x -= 0.5 * f32::from(updates);
            if notice.x < -notice.width {
                notice.x = 1024.0;
            }
        }
    }

    pub(super) fn load_foreign_character_effect(
        &mut self,
        effect_type: i8,
        variant: i8,
    ) -> Result<Vec<CharacterSkillEffectAnimation>, String> {
        let cache_key = (effect_type, variant);
        if let Some(animations) = self.character_effect_cache.get(&cache_key) {
            return Ok(animations.clone());
        }
        let effect_name = match (effect_type, variant) {
            (0, _) => "LevelUp",
            (6 | 8, _) => "JobChanged",
            (7 | 9, _) => "QuestClear",
            (13, _) => "MonsterBook/cardGet",
            (15, _) => "Enchant/Success",
            (16, 0) => "ItemMake/Success",
            (16, _) => "ItemMake/Failure",
            _ => return Ok(Vec::new()),
        };
        let path = format!("BasicEff.img/{effect_name}");
        let Some(source) = self
            .effect_archive
            .root()
            .resolve(&path)
            .map_err(|error| error.to_string())?
        else {
            return Ok(Vec::new());
        };
        let z = integer_property(&source, "z")
            .unwrap_or(0)
            .clamp(i64::from(i8::MIN), i64::from(i8::MAX)) as i8;
        let mut sprites = Vec::new();
        let assets = append_summon_frames_from_node(
            &mut sprites,
            &source,
            &format!("character-effect:{path}"),
            &mut self.remote_art_cache,
        )?;
        let frames = if assets.is_empty() {
            Vec::new()
        } else {
            let draws = self.upload_dynamic_sprites(sprites)?;
            projectile_frames_runtime(assets, &draws)
        };
        let animations = if frames.is_empty() {
            Vec::new()
        } else {
            vec![CharacterSkillEffectAnimation { z, frames }]
        };
        if self.character_effect_cache.len() >= 16 {
            self.character_effect_cache.clear();
        }
        self.character_effect_cache
            .insert(cache_key, animations.clone());
        Ok(animations)
    }

    pub(super) fn show_foreign_character_effect(
        &mut self,
        effect: openstory_protocol::ForeignCharacterEffect,
    ) {
        let Some(local_character_id) = self.active_character.as_ref().map(|value| value.id) else {
            return;
        };
        let character_id = if effect.character_id == local_character_id {
            None
        } else if self
            .map_scene
            .as_ref()
            .is_some_and(|scene| scene.remote_players.contains_key(&effect.character_id))
        {
            Some(effect.character_id)
        } else {
            return;
        };
        if let Some(amount) = effect.heal_amount {
            if let Some(scene) = self.map_scene.as_mut() {
                if let Some(character_id) = character_id {
                    scene.show_remote_player_heal(character_id, i32::from(amount));
                } else {
                    scene.show_local_player_heal(i32::from(amount));
                }
            }
            return;
        }
        let animations =
            match self.load_foreign_character_effect(effect.effect_type, effect.variant) {
                Ok(animations) => animations,
                Err(error) => {
                    eprintln!(
                        "OpenStory: could not load character effect {}: {error}",
                        effect.effect_type
                    );
                    Vec::new()
                }
            };
        if let Some(scene) = self.map_scene.as_mut() {
            scene.show_character_skill_effect(character_id, animations, 1.0);
        }
    }

    pub(super) fn show_remote_skill_effect(
        &mut self,
        character_id: i32,
        skill_id: i32,
        level: i8,
        speed: i8,
        direction: i8,
    ) {
        let Some((look, character_level, current_speed, position)) = self
            .map_scene
            .as_ref()
            .and_then(|scene| scene.remote_players.get(&character_id))
            .map(|player| {
                (
                    player.look.clone(),
                    player.level,
                    player.attack_speed,
                    (rounded_i16(player.x), rounded_i16(player.y)),
                )
            })
        else {
            return;
        };
        let teleport = is_teleport_skill(skill_id);
        let skill_level = level.max(0) as u8;
        if !teleport {
            self.audio
                .play_skill_use(&self.sound_archive, skill_id, position);
        }
        let two_handed = character_weapon_id(&look).is_some_and(is_two_handed_weapon);
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
                two_handed,
            ) {
                Ok(animations) => animations,
                Err(error) => {
                    eprintln!("OpenStory: could not load foreign skill {skill_id} effect: {error}");
                    Vec::new()
                }
            }
        };
        let stance = if teleport {
            None
        } else {
            character_skill_action_stance(&self.skill_archive, &look, skill_id, skill_level)
                .ok()
                .flatten()
        };
        let Some(scene) = self.map_scene.as_mut() else {
            return;
        };
        let Some(player) = scene.remote_players.get_mut(&character_id) else {
            return;
        };
        if direction >= 0 {
            player.facing_left = direction == 0;
        }
        if speed != 0 {
            player.attack_speed = speed;
        }
        if let Some(stance) = stance {
            player.attack_stance = Some(stance);
            player.desired_avatar_motion = AvatarMotion::Attack(stance);
            player.active_avatar_frame =
                first_avatar_frame(&player.avatar_frames, player.desired_avatar_motion);
            player.avatar_elapsed = Duration::ZERO;
        }
        let speed = if speed != 0 { speed } else { current_speed };
        let speed_factor = (1.7 - f32::from(speed) / 10.0).clamp(0.2, 3.0);
        scene.show_character_skill_effect(Some(character_id), animations, speed_factor);
    }

    pub(super) fn handle_open_ui_request(&mut self, request: openstory_protocol::OpenUiRequest) {
        if self.current_field.is_none() {
            return;
        }
        match request {
            openstory_protocol::OpenUiRequest::Inventory => {
                self.show_inventory = true;
                self.inventory_tab = openstory_protocol::InventoryType::Equip;
                self.inventory_scroll_row = 0;
                self.show_status = false;
                self.show_pet_equipment = false;
                self.show_skill_book = false;
                self.show_quest_log = false;
                self.inventory_drag_source = None;
                self.inventory_hover_slot = None;
                self.pending_scroll_slot = None;
                self.update_inventory_hover();
                self.refresh_inventory_texture();
            }
            openstory_protocol::OpenUiRequest::Stats => {
                if !self.show_status {
                    self.open_status_window();
                }
            }
            openstory_protocol::OpenUiRequest::SkillBook => {
                if !self.show_skill_book {
                    self.show_skill_book = true;
                    self.show_inventory = false;
                    self.show_status = false;
                    self.show_pet_equipment = false;
                    self.show_quest_log = false;
                    self.inventory_hover_slot = None;
                    self.inventory_drag_source = None;
                    self.pending_scroll_slot = None;
                    self.skill_book_tab = 0;
                    self.skill_book_scroll_row = 0;
                    self.refresh_skill_book_entries();
                    self.refresh_skill_book_texture();
                }
            }
            openstory_protocol::OpenUiRequest::BuddyList => {
                self.show_buddy_panel = true;
                self.show_party_panel = false;
                self.show_guild_panel = false;
                self.show_alliance_panel = false;
                self.refresh_buddy_texture();
            }
            openstory_protocol::OpenUiRequest::Messenger => self.open_messenger(),
            openstory_protocol::OpenUiRequest::MonsterBook => {
                self.show_inventory = false;
                self.show_status = false;
                self.show_pet_equipment = false;
                self.show_skill_book = false;
                self.show_quest_log = false;
                self.inventory_drag_source = None;
                self.inventory_hover_slot = None;
                self.pending_scroll_slot = None;
                self.character_info = None;
                self.world_map.open = false;
                self.world_map.texture = None;
                self.party_search = None;
                self.party_search_texture = None;
                self.party_search_actions.clear();
                self.monster_book.open = true;
                self.monster_book.page = 0;
                self.monster_book.category = None;
                self.monster_book.clear_selected_card();
                self.monster_book.drop_scroll = 0;
                self.monster_book.search_open = false;
                self.monster_book.search_query.clear();
                self.monster_book.search_notice = None;
                self.monster_book.set_effect_open = false;
                self.monster_book.set_scroll = 0;
                self.monster_book.selected_set = None;
                self.refresh_monster_book_texture();
            }
            openstory_protocol::OpenUiRequest::WorldMap => self.open_world_map(),
            openstory_protocol::OpenUiRequest::PartySearch => self.open_party_search(),
            openstory_protocol::OpenUiRequest::QuestLog => {
                if !self.show_quest_log {
                    self.show_quest_log = true;
                    self.show_inventory = false;
                    self.show_status = false;
                    self.show_pet_equipment = false;
                    self.show_skill_book = false;
                    self.inventory_hover_slot = None;
                    self.inventory_drag_source = None;
                    self.pending_scroll_slot = None;
                    self.quest_log_tab = QuestLogTab::Active;
                    self.quest_log_scroll = 0;
                    self.quest_log_selected = 0;
                    self.quest_reward_selected = 0;
                    self.quest_forfeit_confirmation = None;
                    self.refresh_quest_log_texture();
                }
            }
            openstory_protocol::OpenUiRequest::KeyConfig => {
                if !self.show_key_config {
                    self.toggle_key_config();
                }
            }
            unsupported => {
                self.push_chat_line(
                    format!("[Notice] Server requested an unsupported UI: {unsupported:?}."),
                    [255, 215, 128],
                );
            }
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}
