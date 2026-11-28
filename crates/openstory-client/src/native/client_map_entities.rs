use super::*;

#[derive(Clone, Copy)]
struct NpcDialogueScrollbarGeometry {
    track_y: f32,
    track_height: usize,
    thumb_top: f32,
    thumb_height: usize,
    maximum: usize,
    offset: usize,
    page_step: usize,
}

fn npc_dialogue_scrollbar_geometry(
    dialogue: &NpcDialogueUi,
    target: NpcDialogueScrollTarget,
) -> Option<NpcDialogueScrollbarGeometry> {
    match target {
        NpcDialogueScrollTarget::Text => {
            let track_height = dialogue.text_view_height;
            let maximum = dialogue.text_max_scroll;
            if maximum == 0 || !(51..=2048).contains(&track_height) {
                return None;
            }
            let thumb_height = 25;
            let offset = dialogue.text_scroll.min(maximum);
            let travel = track_height - 51;
            let thumb_top = dialogue.panel.y
                + NpcDialogueLayout::TEXT_Y as f32
                + 13.0
                + (travel as f32 * offset as f32 / maximum as f32).round();
            Some(NpcDialogueScrollbarGeometry {
                track_y: dialogue.panel.y + NpcDialogueLayout::TEXT_Y as f32,
                track_height,
                thumb_top,
                thumb_height,
                maximum,
                offset,
                page_step: track_height.saturating_sub(26).max(1),
            })
        }
        NpcDialogueScrollTarget::Selection => {
            let visible = dialogue
                .buttons
                .iter()
                .filter(|button| matches!(button.action, NpcDialogueAction::Select(_)))
                .count();
            let total = dialogue.selections.len();
            if visible == 0 || total <= visible {
                return None;
            }
            let selection_start_y = NpcDialogueLayout::TEXT_Y as usize
                + dialogue.text_view_height
                + NpcDialogueLayout::SELECTION_TEXT_GAP as usize;
            let track_height = (NpcDialogueLayout::SELECTION_BOTTOM_Y as usize)
                .saturating_sub(selection_start_y)
                .max(51);
            if track_height > 2048 {
                return None;
            }
            let track_y = NpcDialogueLayout::SELECTION_BOTTOM_Y as f32 - track_height as f32;
            let inner_height = track_height - 26;
            let thumb_height = ((inner_height as f32 * visible.min(total) as f32 / total as f32)
                .round() as usize)
                .clamp(8.min(inner_height), inner_height);
            let maximum = total.saturating_sub(visible);
            let offset = dialogue.first_visible_selection.min(maximum);
            let travel = inner_height - thumb_height;
            let thumb_top = dialogue.panel.y
                + track_y
                + 13.0
                + if maximum == 0 {
                    0.0
                } else {
                    (travel as f32 * offset as f32 / maximum as f32).round()
                };
            Some(NpcDialogueScrollbarGeometry {
                track_y: dialogue.panel.y + track_y,
                track_height,
                thumb_top,
                thumb_height,
                maximum,
                offset,
                page_step: visible.max(1),
            })
        }
    }
}

fn append_mob_nameplate_sprite(
    sprites: &mut Vec<DecodedSprite>,
    font: &fontdue::Font,
    name: &str,
    level: i32,
) -> usize {
    const FONT_SIZE: f32 = 12.0;
    const MAX_WIDTH: u32 = 512;
    const HEIGHT: u32 = 20;

    let name = name.chars().take(48).collect::<String>();
    let text = format!("{name} Lv. {level}");
    let width =
        (measured_text_width(font, &text, FONT_SIZE).ceil() as u32).clamp(1, MAX_WIDTH - 4) + 4;
    let index = sprites.len();
    sprites.push(DecodedSprite {
        texture_key: format!("mob-nameplate:{FONT_SIZE}:{text}"),
        rect: SpriteRect {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: width,
        texture_height: HEIGHT,
        pixels: rasterize_label(font, &text, FONT_SIZE, width, HEIGHT).into(),
    });
    index
}

impl ClientApp {
    fn pet_name_matches_owner(&self, character_id: i32, pet_name: &str) -> bool {
        let pet_name = pet_name.trim();
        if pet_name.is_empty() {
            return false;
        }
        self.active_character
            .as_ref()
            .filter(|character| character.id == character_id)
            .is_some_and(|character| character.stats.name.trim() == pet_name)
            || self
                .map_scene
                .as_ref()
                .and_then(|scene| scene.remote_players.get(&character_id))
                .is_some_and(|character| character.name.trim() == pet_name)
    }

    pub(super) fn spawn_map_door(&mut self, spawn: openstory_protocol::MapDoorSpawn) {
        if let Some(scene) = self.map_scene.as_mut() {
            scene.spawn_door(spawn.owner_id, spawn.x, spawn.y);
        }
    }

    pub(super) fn remove_map_door(&mut self, owner_id: i32) {
        if let Some(scene) = self.map_scene.as_mut() {
            scene.remove_door(owner_id);
        }
    }

    pub(super) fn update_map_town_portal(
        &mut self,
        update: openstory_protocol::MapTownPortalUpdate,
    ) {
        const TOWN_PORTAL_OWNER_ID: i32 = -777;

        if let Some(scene) = self.map_scene.as_mut() {
            match update {
                openstory_protocol::MapTownPortalUpdate::Spawn { x, y, .. } => {
                    scene.remove_door(TOWN_PORTAL_OWNER_ID);
                    scene.spawn_door(TOWN_PORTAL_OWNER_ID, x, y);
                }
                openstory_protocol::MapTownPortalUpdate::Remove => {
                    scene.remove_door(TOWN_PORTAL_OWNER_ID);
                }
            }
        }
    }

    pub(super) fn apply_remote_foreign_buff(
        &mut self,
        update: openstory_protocol::ForeignCharacterBuffUpdate,
    ) {
        let mount = update.mount_item_id.map(|item_id| {
            self.load_remote_mount(item_id).unwrap_or_else(|error| {
                eprintln!("OpenStoryRS: could not load remote mount {item_id}: {error}");
                None
            })
        });
        if update.hidden.is_none() && update.mount_item_id.is_none() {
            return;
        }
        if let Some(player) = self
            .map_scene
            .as_mut()
            .and_then(|scene| scene.remote_players.get_mut(&update.character_id))
        {
            if let Some(hidden) = update.hidden {
                player.hidden = hidden;
            }
            if update.mount_item_id.is_some() {
                player.mount = mount.flatten();
            }
        }
    }

    pub(super) fn cancel_remote_foreign_buff(
        &mut self,
        update: openstory_protocol::ForeignCharacterBuffCancel,
    ) {
        if !update.cancel_hidden && !update.cancel_mount {
            return;
        }
        if let Some(player) = self
            .map_scene
            .as_mut()
            .and_then(|scene| scene.remote_players.get_mut(&update.character_id))
        {
            if update.cancel_hidden {
                player.hidden = false;
            }
            if update.cancel_mount {
                player.mount = None;
            }
        }
    }

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
            reactor_id: spawn.reactor_id,
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

    pub(super) fn apply_taming_mob_info(&mut self, update: openstory_protocol::TamingMobInfo) {
        if self
            .active_character
            .as_ref()
            .is_none_or(|character| character.id != update.character_id)
        {
            return;
        }
        self.taming_mount_stats = Some(update.mount);
        let refresh = if let Some(ui) = self.character_info.as_mut() {
            if ui.info.character_id == update.character_id {
                ui.info.mount = Some(update.mount);
                true
            } else {
                false
            }
        } else {
            false
        };
        if refresh {
            self.refresh_character_info_texture();
        }
    }

    pub(super) fn handle_character_info(
        &mut self,
        mut info: openstory_protocol::CharacterInfoReply,
    ) {
        let is_local_character = self
            .active_character
            .as_ref()
            .is_some_and(|character| character.id == info.character_id);
        if is_local_character {
            self.monster_book.set_cover(info.monster_book.cover_item_id);
            if self.monster_book.open {
                self.refresh_monster_book_texture();
            }
            if let Some(mount) = info.mount {
                self.taming_mount_stats = Some(mount);
            } else {
                info.mount = self.taming_mount_stats;
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
        let look = self
            .map_scene
            .as_ref()
            .and_then(|scene| scene.remote_players.get(&info.character_id))
            .map(|player| player.look.clone())
            .or_else(|| {
                self.active_character
                    .as_ref()
                    .filter(|character| character.id == info.character_id)
                    .map(|character| character.look.clone())
            });
        self.character_info = Some(CharacterInfoUi {
            name,
            look,
            wishlist_offset: 0,
            selected_pet: 0,
            info,
            fame_pending: false,
        });
        self.raise_dialog_window(DialogWindow::CharacterInfo);
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
        if self.map_initial_npc_loading_until.is_some() {
            self.initial_npc_spawns
                .retain(|queued| queued.object_id != spawn.object_id);
            self.initial_npc_spawns.push_back(spawn);
            self.map_initial_npc_loading_until =
                Some(Instant::now() + INITIAL_NPC_LOADING_QUIET_WINDOW);
            return;
        }
        // A later spawn/controller update supersedes a distant initial spawn
        // that has not reached the background load queue yet.
        self.initial_npc_spawns
            .retain(|queued| queued.object_id != spawn.object_id);
        self.spawn_npc_inner(spawn);
    }

    pub(super) fn spawn_npc_inner(&mut self, spawn: openstory_protocol::MapNpcSpawn) {
        let flipped = !spawn.flipped;
        let maple_tv_anchors = maple_tv_screen_anchors(&self.npc_archive, spawn.npc_id);
        if maple_tv_anchors.is_some() {
            self.ensure_maple_tv_assets();
        }
        let npc_id = spawn.npc_id.max(0);
        let info_path = format!("{:07}.img/info", npc_id);
        let hide_name = self
            .npc_archive
            .root()
            .resolve(&info_path)
            .ok()
            .flatten()
            .and_then(|info| integer_property(&info, "hideName"))
            .is_some_and(|value| value != 0);
        let mut name_tag_sprites = Vec::with_capacity(2);
        let name_label_index = if hide_name {
            None
        } else {
            nx_string_value(&self.string_archive, &format!("Npc.img/{npc_id}/name")).and_then(
                |name| {
                    append_map_name_tag_sprite(
                        &mut name_tag_sprites,
                        &self.font,
                        &self.ui_archive,
                        None,
                        None,
                        &name,
                        512.0,
                        GAMEPLAY_CAMERA_CENTER_Y,
                        MapNameTagKind::Npc,
                    )
                },
            )
        };
        let function_label_index = if hide_name {
            None
        } else {
            nx_string_value(&self.string_archive, &format!("Npc.img/{npc_id}/func")).and_then(
                |function| {
                    append_map_name_tag_sprite(
                        &mut name_tag_sprites,
                        &self.font,
                        &self.ui_archive,
                        None,
                        None,
                        &function,
                        512.0,
                        GAMEPLAY_CAMERA_CENTER_Y + 19.0,
                        MapNameTagKind::Npc,
                    )
                },
            )
        };
        let name_tag_draws = if name_tag_sprites.is_empty() {
            Vec::new()
        } else {
            match self.upload_dynamic_sprites(name_tag_sprites) {
                Ok(draws) => draws,
                Err(error) => {
                    eprintln!("OpenStoryRS: could not load NPC name tags: {error}");
                    Vec::new()
                }
            }
        };
        let name_label = name_label_index.and_then(|index| name_tag_draws.get(index).copied());
        let function_label =
            function_label_index.and_then(|index| name_tag_draws.get(index).copied());
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
                    imitated_look_sprites: None,
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
                    name_label,
                    function_label,
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
        if let Some(index) = self
            .initial_npc_spawns
            .iter()
            .position(|spawn| spawn.object_id == object_id)
        {
            self.initial_npc_spawns.remove(index);
            if index < self.initial_npc_nearby_pending {
                self.initial_npc_nearby_pending -= 1;
                if self.initial_npc_nearby_pending == 0
                    && self.map_initial_npc_loading_until.is_none()
                    && self.map_loading_started_at.is_some()
                {
                    self.release_map_loading_screen();
                }
            }
        }
        if let Some(scene) = self.map_scene.as_mut() {
            scene.npcs.remove(&object_id);
        }
    }

    pub(super) fn apply_imitated_npc_update(
        &mut self,
        update: openstory_protocol::ImitatedNpcUpdate,
    ) {
        match update {
            openstory_protocol::ImitatedNpcUpdate::Remove { object_id } => {
                self.remove_npc(object_id);
            }
            openstory_protocol::ImitatedNpcUpdate::Ignore => {}
            openstory_protocol::ImitatedNpcUpdate::SetPlayerLook { npc_id, look } => {
                let object_ids = self
                    .map_scene
                    .as_ref()
                    .map(|scene| {
                        scene
                            .npcs
                            .iter()
                            .filter_map(|(&object_id, npc)| {
                                (npc.npc_id == npc_id).then_some(object_id)
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if object_ids.is_empty() {
                    return;
                }

                let stance = if character_weapon_id(&look).is_some_and(is_two_handed_weapon) {
                    "stand2"
                } else {
                    "stand1"
                };
                let mut sprites = Vec::new();
                if let Err(error) = append_character_avatar(
                    &mut sprites,
                    &self.character_archive,
                    &look,
                    stance,
                    0,
                    512.0,
                    GAMEPLAY_CAMERA_CENTER_Y,
                    &mut self.remote_art_cache,
                ) {
                    eprintln!("OpenStoryRS: could not render imitated NPC {npc_id}: {error}");
                    return;
                }
                if sprites.is_empty() {
                    return;
                }
                let draws = match self.upload_dynamic_sprites(sprites) {
                    Ok(draws) if !draws.is_empty() => draws,
                    Ok(_) => return,
                    Err(error) => {
                        eprintln!("OpenStoryRS: could not upload imitated NPC {npc_id}: {error}");
                        return;
                    }
                };
                if let Some(scene) = self.map_scene.as_mut() {
                    for object_id in object_ids {
                        if let Some(npc) = scene.npcs.get_mut(&object_id) {
                            npc.imitated_look_sprites = Some(draws.clone());
                        }
                    }
                }
            }
        }
    }

    pub(super) fn mark_npcs_scriptable(&mut self, npc_ids: &[i32]) {
        self.scriptable_npc_ids.extend(npc_ids.iter().copied());
        if let Some(scene) = self.map_scene.as_mut() {
            scene.mark_npcs_scriptable(npc_ids);
        }
    }

    pub(super) fn show_npc_dialogue(&mut self, dialogue: openstory_protocol::NpcDialogue) {
        let mut dialogue = dialogue;
        if let Some(inventory) = self.inventory.as_ref() {
            restrict_mr_thunder_quantity_to_inventory(
                &mut dialogue,
                &self.string_archive,
                inventory,
            );
        }
        self.pending_npc_talk = None;
        self.pending_scripted_quest_end = None;
        self.gameplay_input = GameplayInput::default();
        self.npc_dialogue_scroll_dragging = None;
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
        let (pixels, panel, buttons, text_max_scroll, text_view_height) = render_npc_dialogue(
            &self.font,
            &self.legacy_ui_archive,
            &self.npc_archive,
            &self.string_archive,
            dialogue.npc_id,
            dialogue.speaker,
            dialogue.message_type,
            dialogue.navigation,
            dialogue.number_bounds,
            visible_text,
            0,
            &inline_icons,
            &selections,
            0,
            0,
            None,
            &input,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        match renderer.upload_bgra(
            panel.width.round() as u32,
            panel.height.round() as u32,
            &pixels,
        ) {
            Ok(texture) => {
                let visible_text_chars = if dialogue.message_type == 4 {
                    text.chars().count()
                } else {
                    0
                };
                let number_input_selected = dialogue.message_type == 3;
                self.npc_dialogue = Some(NpcDialogueUi {
                    state: dialogue,
                    local_quest: None,
                    local_menu: Vec::new(),
                    text: formatted_text,
                    visible_text_chars,
                    reveal_elapsed: Duration::ZERO,
                    text_scroll: 0,
                    text_max_scroll,
                    text_view_height,
                    inline_icons,
                    selections,
                    selected_selection: 0,
                    first_visible_selection: 0,
                    hovered_selection: None,
                    input,
                    number_input_selected,
                    texture,
                    panel,
                    buttons,
                });
            }
            Err(error) => eprintln!("OpenStory: could not render NPC dialogue: {error}"),
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
        self.close_skill_book();
        self.pending_scroll_slot = None;
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
            self.refresh_cash_shop_texture();
            return;
        }
        let Some(session) = self.session.as_ref() else {
            self.cash_shop = None;
            self.refresh_cash_shop_texture();
            return;
        };
        match session.send_packet(openstory_protocol::exit_cash_shop_packet()) {
            Ok(()) => state.notice = Some("Returning to the field…".to_owned()),
            Err(error) => {
                eprintln!("OpenStoryRS: could not leave the Cash Shop: {error}");
                self.cash_shop = None;
                self.refresh_cash_shop_texture();
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
            openstory_protocol::CashShopOperation::ItemsPurchased(items) => {
                let purchased_count = items.len();
                for item in items {
                    state
                        .locker
                        .retain(|existing| existing.instance_id != item.instance_id);
                    state.locker.push(item);
                }
                state.notice = Some(if purchased_count == 1 {
                    "Item purchased successfully.".to_owned()
                } else {
                    format!("{purchased_count} package items purchased successfully.")
                });
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
            openstory_protocol::CashShopOperation::CouponRedeemed {
                items,
                maple_points,
            } => {
                for item in items {
                    state
                        .locker
                        .retain(|existing| existing.instance_id != item.instance_id);
                    state.locker.push(item);
                }
                if maple_points > 0 {
                    state.balances[1] = state.balances[1].saturating_add(maple_points);
                }
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
        }
        self.refresh_cash_shop_texture();
    }

    pub(super) fn handle_cash_balances(&mut self, packet: &[u8]) {
        match openstory_protocol::parse_cash_balances(packet) {
            Ok(balances) => {
                if let Some(state) = self.cash_shop.as_mut() {
                    let previous_nx = state.balances[0];
                    let charge_pending = state.pending_nx_charge;
                    let cash_query_pending = state.pending_cash_query;
                    let withdrawal_failed = state.pending_withdrawal.take().is_some();
                    state.balances = balances;
                    state.pending_nx_charge = false;
                    state.pending_cash_query = false;
                    if charge_pending {
                        state.notice = Some(if balances[0] > previous_nx {
                            "NX Credit added.".to_owned()
                        } else {
                            "No NX Credit was added. Check the server's free NX charge setting."
                                .to_owned()
                        });
                    } else if withdrawal_failed {
                        state.notice = Some(
                            "The item could not be transferred. Check inventory space and try again."
                                .to_owned(),
                        );
                    } else if cash_query_pending {
                        state.notice = Some("Cash balance updated.".to_owned());
                    }
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
        let Some(cash_id) = i32::try_from(instance_id).ok().filter(|id| *id > 0) else {
            if let Some(state) = self.cash_shop.as_mut() {
                state.notice = Some("This locker item has an invalid server ID.".to_owned());
            }
            self.refresh_cash_shop_texture();
            return;
        };
        let Some(session) = self.session.as_ref() else {
            return;
        };
        match session.send_packet(openstory_protocol::withdraw_cash_item_packet(cash_id)) {
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
            cash_shop_ui::HitAction::BuyPreviewedItem => {
                if let Some(state) = self.cash_shop.as_mut() {
                    if state.selected_sn.is_some() && state.entered {
                        state.confirmation = true;
                        state.search_focused = false;
                    } else if state.selected_sn.is_some() {
                        state.notice = Some("Waiting for the server to open the shop.".to_owned());
                    } else {
                        state.notice = Some("Select an item to preview first.".to_owned());
                    }
                }
            }
            cash_shop_ui::HitAction::DefaultAvatarPreview => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.preview_remove_all = false;
                    state.selected_sn = None;
                    state.try_on_items.clear();
                    state.try_on_pet_equipment = None;
                    state.try_on_pet_label_ring = false;
                    state.try_on_pet_quote_ring = false;
                    state.search_focused = false;
                }
            }
            cash_shop_ui::HitAction::RemoveAvatarEquipment => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.preview_remove_all = true;
                    state.selected_sn = None;
                    state.try_on_items.clear();
                    state.try_on_pet_equipment = None;
                    state.try_on_pet_label_ring = false;
                    state.try_on_pet_quote_ring = false;
                    state.search_focused = false;
                }
            }
            cash_shop_ui::HitAction::Charge => {
                let result = self.session.as_ref().map(|session| {
                    session.send_packet(openstory_protocol::charge_nx_credit_packet())
                });
                match result {
                    Some(Ok(())) => {
                        if let Some(state) = self.cash_shop.as_mut() {
                            state.pending_nx_charge = true;
                            state.notice = Some(
                                "Charge request sent; waiting for the updated balance.".to_owned(),
                            );
                        }
                    }
                    Some(Err(error)) => {
                        eprintln!("OpenStoryRS: could not request NX Credit: {error}");
                        self.channel_connected = false;
                        if let Some(state) = self.cash_shop.as_mut() {
                            state.notice = Some("Could not send the NX request.".to_owned());
                        }
                    }
                    None => {
                        if let Some(state) = self.cash_shop.as_mut() {
                            state.notice = Some("Not connected to the server.".to_owned());
                        }
                    }
                }
            }
            cash_shop_ui::HitAction::CheckCash => {
                let result = self.session.as_ref().map(|session| {
                    session.send_packet(openstory_protocol::query_cash_balance_packet())
                });
                match result {
                    Some(Ok(())) => {
                        if let Some(state) = self.cash_shop.as_mut() {
                            state.pending_cash_query = true;
                            state.notice = Some("Cash balance request sent.".to_owned());
                        }
                    }
                    Some(Err(error)) => {
                        eprintln!("OpenStoryRS: could not request cash balances: {error}");
                        self.channel_connected = false;
                        if let Some(state) = self.cash_shop.as_mut() {
                            state.notice = Some("Could not request cash balances.".to_owned());
                        }
                    }
                    None => {
                        if let Some(state) = self.cash_shop.as_mut() {
                            state.notice = Some("Not connected to the server.".to_owned());
                        }
                    }
                }
            }
            cash_shop_ui::HitAction::Coupon => {
                if let Some(state) = self.cash_shop.as_mut() {
                    state.coupon_open = true;
                    state.notice = None;
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
                    cash_shop_ui::add_try_on_item(state, product.item_id);
                    state.preview_remove_all = false;
                    state.search_focused = false;
                }
            }
            cash_shop_ui::HitAction::BuyProduct(index) => {
                if let (Some(state), Some(product)) =
                    (self.cash_shop.as_mut(), self.cash_products.get(index))
                {
                    state.selected_sn = Some(product.sn);
                    cash_shop_ui::add_try_on_item(state, product.item_id);
                    state.preview_remove_all = false;
                    state.search_focused = false;
                    if state.entered {
                        state.confirmation = true;
                    } else {
                        state.notice = Some("Waiting for the server to open the shop.".to_owned());
                    }
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
                withdrawal = self
                    .cash_shop
                    .as_ref()
                    .and_then(|state| state.locker.get(index).map(|item| item.instance_id));
                if let Some(state) = self.cash_shop.as_mut() {
                    state.search_focused = false;
                }
            }
            cash_shop_ui::HitAction::CashInventoryScroll => {
                let maximum = self
                    .inventory
                    .as_ref()
                    .map_or(0, cash_shop_ui::cash_inventory_scroll_max);
                if let Some(state) = self.cash_shop.as_mut() {
                    state.cash_inventory_scroll_row =
                        cash_shop_ui::cash_inventory_scroll_from_click(cursor.y, maximum);
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
        cash_shop_ui::hit_test(&self.cash_shop_hit_regions, cursor.x, cursor.y)
    }

    pub(super) fn scroll_cash_shop_inventory(&mut self, cursor: Point, amount: f32) -> bool {
        if !self.cash_shop.as_ref().is_some_and(|_| {
            cash_shop_ui::cash_inventory_scroll_area().contains(cursor.x, cursor.y)
        }) {
            return false;
        }
        let maximum = self
            .inventory
            .as_ref()
            .map_or(0, cash_shop_ui::cash_inventory_scroll_max);
        let Some(state) = self.cash_shop.as_mut() else {
            return false;
        };
        let current = state.cash_inventory_scroll_row.min(maximum);
        let next = if amount > 0.0 {
            current.saturating_sub(1)
        } else if amount < 0.0 {
            current.saturating_add(1).min(maximum)
        } else {
            current
        };
        if next != state.cash_inventory_scroll_row {
            state.cash_inventory_scroll_row = next;
            self.refresh_cash_shop_texture();
        }
        true
    }

    pub(super) fn update_cash_shop_hover(&mut self) -> bool {
        let hovered_item = self
            .cursor
            .and_then(|cursor| self.cash_shop_hit_action(cursor))
            .and_then(cash_shop_ui::ItemHover::from_action);
        if hovered_item == self.cash_shop_hovered_item {
            return false;
        }
        self.cash_shop_hovered_item = hovered_item;
        self.refresh_cash_shop_tooltip_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
        true
    }

    fn cash_shop_tooltip_item(&self) -> Option<openstory_protocol::InventoryItem> {
        let shop = self.cash_shop.as_ref()?;
        let (item_id, count, cash, cash_id, expiration) = match self.cash_shop_hovered_item? {
            cash_shop_ui::ItemHover::Product(index) => {
                let product = self.cash_products.get(index)?;
                (product.item_id, product.count.max(1), false, None, 0)
            }
            cash_shop_ui::ItemHover::Locker(index) => {
                let item = shop.locker.get(index)?;
                if let Some(inventory_item) = self.inventory.as_ref().and_then(|inventory| {
                    inventory
                        .items
                        .iter()
                        .find(|inventory_item| inventory_item.cash_id == Some(item.instance_id))
                }) {
                    return Some(inventory_item.clone());
                }
                (
                    item.item_id,
                    item.count.max(1),
                    true,
                    Some(item.instance_id),
                    item.expiration,
                )
            }
        };
        Some(openstory_protocol::InventoryItem {
            inventory_type: cash_shop_ui::cash_shop_inventory_type(item_id),
            item_id,
            count,
            cash,
            cash_id,
            expiration,
            ..Default::default()
        })
    }

    pub(super) fn refresh_cash_shop_tooltip_texture(&mut self) {
        self.update_item_tooltip(self.cash_shop_tooltip_item());
    }

    pub(super) fn cash_shop_max_page(&mut self) -> usize {
        let Some(state) = self.cash_shop.as_ref() else {
            return 0;
        };
        if state.view == CashShopView::Locker {
            state.locker.len().saturating_sub(1) / cash_shop_ui::LOCKER_PAGE_SIZE
        } else {
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
        let hovered_selection = ui.hovered_selection;
        let input = ui.input.clone();
        let text_scroll = ui.text_scroll;
        let text = if state.message_type == 4 {
            parse_npc_selections(&dialogue_text).0
        } else {
            dialogue_text.chars().take(ui.visible_text_chars).collect()
        };
        let (pixels, panel, buttons, text_max_scroll, text_view_height) = render_npc_dialogue(
            &self.font,
            &self.legacy_ui_archive,
            &self.npc_archive,
            &self.string_archive,
            state.npc_id,
            state.speaker,
            state.message_type,
            state.navigation,
            state.number_bounds,
            &text,
            text_scroll,
            &inline_icons,
            &selections,
            selected_selection,
            first_visible_selection,
            hovered_selection,
            &input,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        match renderer.upload_bgra(
            panel.width.round() as u32,
            panel.height.round() as u32,
            &pixels,
        ) {
            Ok(texture) => {
                if let Some(ui) = self.npc_dialogue.as_mut() {
                    ui.text_max_scroll = text_max_scroll;
                    ui.text_view_height = text_view_height;
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
        if self.handle_local_npc_menu(action) {
            return;
        }
        if self
            .npc_dialogue
            .as_ref()
            .is_some_and(|ui| ui.local_quest.is_some())
        {
            let Some(mut ui) = self.npc_dialogue.take() else {
                return;
            };
            let Some(mut conversation) = ui.local_quest.take() else {
                return;
            };
            self.npc_dialogue_scroll_dragging = None;
            let step = match action {
                NpcDialogueAction::More(response) => conversation.respond(response),
                NpcDialogueAction::Select(selection) => conversation.select(selection),
                _ => {
                    ui.local_quest = Some(conversation);
                    self.npc_dialogue = Some(ui);
                    return;
                }
            };
            let (close, packet) = match step {
                LocalQuestStep::Show(packet) => (false, packet),
                LocalQuestStep::Close(packet) => (true, packet),
            };
            if let Some(packet) = packet {
                let Some(session) = self.session.as_ref() else {
                    return;
                };
                if let Err(error) = session.send_packet(packet) {
                    eprintln!("OpenStory: could not submit quest action: {error}");
                    self.channel_connected = false;
                    return;
                }
            }
            if close {
                return;
            }
            self.show_local_quest_conversation(ui.state.npc_id, conversation);
            return;
        }
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
                let mut state = ui.state.clone();
                if let Some(inventory) = self.inventory.as_ref() {
                    restrict_mr_thunder_quantity_to_inventory(
                        &mut state,
                        &self.string_archive,
                        inventory,
                    );
                }
                let value = npc_dialogue_number(&ui.input, state.number_bounds);
                Ok(openstory_protocol::npc_talk_number(value))
            }
            NpcDialogueAction::AdjustNumber(delta) => {
                let value =
                    npc_dialogue_number(&ui.input, ui.state.number_bounds).saturating_add(delta);
                let value = npc_dialogue_number(&value.to_string(), ui.state.number_bounds);
                if let Some(ui) = self.npc_dialogue.as_mut() {
                    ui.input = value.to_string();
                    ui.number_input_selected = false;
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
                self.npc_dialogue_scroll_dragging = None;
            }
            Err(error) => eprintln!("OpenStory: invalid NPC dialogue response: {error}"),
        }
    }

    pub(super) fn click_npc_dialogue(&mut self, cursor: Point) {
        if self.click_npc_dialogue_scrollbar(cursor) {
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

    pub(super) fn update_npc_dialogue_hover(&mut self, cursor: Point) -> bool {
        let Some(dialogue) = self.npc_dialogue.as_mut() else {
            return false;
        };
        let hovered_selection =
            dialogue
                .buttons
                .iter()
                .enumerate()
                .find_map(|(button_index, button)| {
                    if !rect_contains(button.rect, cursor) {
                        return None;
                    }
                    let NpcDialogueAction::Select(_) = button.action else {
                        return None;
                    };
                    let visible_row = dialogue.buttons[..button_index]
                        .iter()
                        .filter(|button| matches!(button.action, NpcDialogueAction::Select(_)))
                        .count();
                    let selection = dialogue.first_visible_selection.saturating_add(visible_row);
                    (selection < dialogue.selections.len()).then_some(selection)
                });
        if dialogue.hovered_selection == hovered_selection {
            return false;
        }
        dialogue.hovered_selection = hovered_selection;
        self.rebuild_npc_dialogue();
        true
    }

    pub(super) fn click_npc_dialogue_scrollbar(&mut self, cursor: Point) -> bool {
        let Some(dialogue) = self.npc_dialogue.as_ref() else {
            return false;
        };
        let scrollbar_x = dialogue.panel.x + NpcDialogueLayout::SCROLL_X as f32;
        if !(scrollbar_x..scrollbar_x + 15.0).contains(&cursor.x) {
            return false;
        }

        for target in [
            NpcDialogueScrollTarget::Text,
            NpcDialogueScrollTarget::Selection,
        ] {
            let Some(geometry) = npc_dialogue_scrollbar_geometry(dialogue, target) else {
                continue;
            };
            let track_bottom = geometry.track_y + geometry.track_height as f32;
            if !(geometry.track_y..track_bottom).contains(&cursor.y) {
                continue;
            }
            let track_top = geometry.track_y + 13.0;
            let arrow_bottom = track_bottom - 13.0;
            let arrow_step = match target {
                NpcDialogueScrollTarget::Text => NPC_DIALOGUE_SCROLL_STEP,
                NpcDialogueScrollTarget::Selection => 1,
            };
            if cursor.y < track_top {
                self.set_npc_dialogue_scroll_offset(
                    target,
                    geometry.offset.saturating_sub(arrow_step),
                );
                return true;
            }
            if cursor.y >= arrow_bottom {
                self.set_npc_dialogue_scroll_offset(
                    target,
                    geometry.offset.saturating_add(arrow_step),
                );
                return true;
            }
            if (geometry.thumb_top..geometry.thumb_top + geometry.thumb_height as f32)
                .contains(&cursor.y)
            {
                self.npc_dialogue_scroll_dragging = Some(NpcDialogueScrollDrag {
                    target,
                    grab_offset: cursor.y - geometry.thumb_top,
                });
                return true;
            }
            let next = if cursor.y < geometry.thumb_top {
                geometry.offset.saturating_sub(geometry.page_step)
            } else {
                geometry.offset.saturating_add(geometry.page_step)
            };
            self.set_npc_dialogue_scroll_offset(target, next);
            return true;
        }
        false
    }

    pub(super) fn update_npc_dialogue_scroll_drag(&mut self, cursor: Point) -> bool {
        let Some(drag) = self.npc_dialogue_scroll_dragging else {
            return false;
        };
        let Some(geometry) = self
            .npc_dialogue
            .as_ref()
            .and_then(|dialogue| npc_dialogue_scrollbar_geometry(dialogue, drag.target))
        else {
            self.npc_dialogue_scroll_dragging = None;
            return false;
        };
        let travel = match drag.target {
            NpcDialogueScrollTarget::Text => geometry.track_height.saturating_sub(51),
            NpcDialogueScrollTarget::Selection => geometry
                .track_height
                .saturating_sub(26 + geometry.thumb_height),
        };
        if travel == 0 || geometry.maximum == 0 {
            return false;
        }
        let thumb_top_min = geometry.track_y + 13.0;
        let thumb_top =
            (cursor.y - drag.grab_offset).clamp(thumb_top_min, thumb_top_min + travel as f32);
        let fraction = (thumb_top - thumb_top_min) / travel as f32;
        let offset = (fraction * geometry.maximum as f32).round() as usize;
        self.set_npc_dialogue_scroll_offset(drag.target, offset)
    }

    fn set_npc_dialogue_scroll_offset(
        &mut self,
        target: NpcDialogueScrollTarget,
        offset: usize,
    ) -> bool {
        let Some(dialogue) = self.npc_dialogue.as_mut() else {
            return false;
        };
        match target {
            NpcDialogueScrollTarget::Text => {
                let next = offset.min(dialogue.text_max_scroll);
                if dialogue.text_scroll == next {
                    return false;
                }
                dialogue.text_scroll = next;
            }
            NpcDialogueScrollTarget::Selection => {
                let visible = dialogue
                    .buttons
                    .iter()
                    .filter(|button| matches!(button.action, NpcDialogueAction::Select(_)))
                    .count()
                    .max(1);
                let maximum = dialogue.selections.len().saturating_sub(visible);
                let next = offset.min(maximum);
                if dialogue.first_visible_selection == next {
                    return false;
                }
                dialogue.first_visible_selection = next;
                let selection_end = next
                    .saturating_add(visible)
                    .saturating_sub(1)
                    .min(dialogue.selections.len().saturating_sub(1));
                dialogue.selected_selection =
                    dialogue.selected_selection.clamp(next, selection_end);
            }
        }
        self.rebuild_npc_dialogue();
        true
    }

    pub(super) fn scroll_npc_dialogue_text(&mut self, direction: isize) {
        let Some(dialogue) = self.npc_dialogue.as_mut() else {
            return;
        };
        if dialogue.text_max_scroll == 0 {
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

    pub(super) fn scroll_npc_dialogue_selection_at(
        &mut self,
        cursor: Point,
        direction: isize,
    ) -> bool {
        let Some(dialogue) = self.npc_dialogue.as_ref() else {
            return false;
        };
        let Some(geometry) =
            npc_dialogue_scrollbar_geometry(dialogue, NpcDialogueScrollTarget::Selection)
        else {
            return false;
        };
        let scrollbar_x = dialogue.panel.x + NpcDialogueLayout::SCROLL_X as f32;
        if !(scrollbar_x..scrollbar_x + 15.0).contains(&cursor.x)
            || !(geometry.track_y..geometry.track_y + geometry.track_height as f32)
                .contains(&cursor.y)
        {
            return false;
        }

        let offset = if direction < 0 {
            geometry.offset.saturating_sub(1)
        } else {
            geometry.offset.saturating_add(1)
        };
        self.set_npc_dialogue_scroll_offset(NpcDialogueScrollTarget::Selection, offset);
        true
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
            .map(|ui| npc_dialogue_cancel_response(ui.state.message_type));
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
        let visible_selection_count = dialogue
            .buttons
            .iter()
            .filter(|button| matches!(button.action, NpcDialogueAction::Select(_)))
            .count()
            .max(1);
        let last_selection = dialogue.selections.len().saturating_sub(1);
        dialogue.selected_selection = dialogue
            .selected_selection
            .saturating_add_signed(direction)
            .min(last_selection);
        if dialogue.selected_selection < dialogue.first_visible_selection {
            dialogue.first_visible_selection = dialogue.selected_selection;
        } else if dialogue.selected_selection
            >= dialogue.first_visible_selection + visible_selection_count
        {
            dialogue.first_visible_selection =
                dialogue.selected_selection + 1 - visible_selection_count;
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
            let required_level = equipment_required_level(spawn.item_id, &self.character_archive);
            scene.spawn_drop(spawn, frames, required_level);
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
        let name = nx_string_value(
            &self.string_archive,
            &format!("Mob.img/{}/name", spawn.mob_id),
        )
        .unwrap_or_else(|| format!("Monster {}", spawn.mob_id));
        let nameplate_sprite_index =
            append_mob_nameplate_sprite(&mut sprites, &self.font, &name, traits.level);
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
            nameplate: draws.get(nameplate_sprite_index).copied(),
            hit_effects: Vec::with_capacity(2),
            dying: false,
            fade_remaining: None,
            fade_in_remaining: spawn.is_new_spawn.then_some(MOB_FADE_DURATION),
            hp_percent: None,
            hp_bar_remaining: Duration::ZERO,
            statuses: HashSet::new(),
            control_acked: false,
            movement_dirty: false,
            ai_counter: 0,
            random_state: ((spawn.object_id as u32).wrapping_mul(0x9E37_79B9)
                ^ spawn.mob_id as u32
                ^ 0xA341_316C)
                .max(1),
            fly_direction: 0,
            aggro_target: None,
            aggro_remaining: Duration::ZERO,
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

    fn pet_owner_look(&self, character_id: i32) -> Option<openstory_protocol::CharacterLook> {
        self.active_character
            .as_ref()
            .filter(|character| character.id == character_id)
            .map(|character| character.look.clone())
            .or_else(|| {
                self.map_scene
                    .as_ref()?
                    .remote_players
                    .get(&character_id)
                    .map(|player| player.look.clone())
            })
    }

    pub(super) fn refresh_pet_name_labels(&mut self, character_id: i32) {
        let Some(owner_look) = self.pet_owner_look(character_id) else {
            return;
        };
        let pets = self
            .map_scene
            .as_ref()
            .map(|scene| {
                scene
                    .pets
                    .iter()
                    .filter(|((owner_id, _), _)| *owner_id == character_id)
                    .map(|(key, pet)| (*key, pet.item_id, pet.name.clone()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if pets.is_empty() {
            return;
        }
        let first_pet_slot = pets.iter().map(|((_, slot), _, _)| *slot).min();
        let label_ring_equipped = self
            .active_character
            .as_ref()
            .is_some_and(|character| character.id == character_id)
            && self.inventory.as_ref().is_some_and(|inventory| {
                inventory
                    .equipped()
                    .any(|item| item.slot == 114 && item.item_id / 10_000 == 182)
            });

        let mut sprites = Vec::new();
        let mut labels = Vec::with_capacity(pets.len());
        for (key, pet_id, name) in pets {
            let label_index =
                if name.trim().is_empty() || self.pet_name_matches_owner(character_id, &name) {
                    None
                } else if label_ring_equipped && Some(key.1) == first_pet_slot {
                    append_pet_name_tag_sprite_scaled(
                        &mut sprites,
                        &self.font,
                        &self.ui_archive,
                        &self.item_archive,
                        pet_id,
                        true,
                        &name,
                        512.0,
                        GAMEPLAY_CAMERA_CENTER_Y,
                        1.0,
                    )
                } else {
                    append_map_name_tag_sprite(
                        &mut sprites,
                        &self.font,
                        &self.ui_archive,
                        Some(&self.character_archive),
                        Some(&owner_look),
                        &name,
                        512.0,
                        GAMEPLAY_CAMERA_CENTER_Y,
                        MapNameTagKind::Pet,
                    )
                };
            labels.push((key, label_index));
        }

        let draws = if sprites.is_empty() {
            Vec::new()
        } else {
            match self.upload_dynamic_sprites(sprites) {
                Ok(draws) => draws,
                Err(error) => {
                    eprintln!("OpenStoryRS: could not refresh pet name labels: {error}");
                    return;
                }
            }
        };
        if let Some(scene) = self.map_scene.as_mut() {
            for (key, label_index) in labels {
                if let Some(pet) = scene.pets.get_mut(&key) {
                    pet.name_label = label_index.and_then(|index| draws.get(index).copied());
                }
            }
        }
    }

    pub(super) fn spawn_pet(&mut self, spawn: openstory_protocol::MapPetSpawn) {
        let owner_id = spawn.character_id;
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
        let owner_look = self.pet_owner_look(spawn.character_id);
        let name_label = if spawn.name.is_empty()
            || self.pet_name_matches_owner(spawn.character_id, &spawn.name)
        {
            None
        } else {
            let mut sprites = Vec::with_capacity(1);
            append_map_name_tag_sprite(
                &mut sprites,
                &self.font,
                &self.ui_archive,
                Some(&self.character_archive),
                owner_look.as_ref(),
                &spawn.name,
                512.0,
                GAMEPLAY_CAMERA_CENTER_Y,
                MapNameTagKind::Pet,
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
                name: spawn.name,
                name_label,
                x: f64::from(spawn.x),
                y: f64::from(spawn.y),
                body,
                loot_target: None,
                loot_excursion: false,
                owner_was_climbing: false,
                follow_stuck_ticks: 0,
                best_follow_distance: f64::INFINITY,
                loot_progress_target: None,
                best_loot_distance: f64::INFINITY,
                loot_stuck_ticks: 0,
                movement_ticks: 0,
                last_sent_movement: None,
                stance: spawn.stance,
                animations,
                equipment_animations: PetEquipmentAnimations::default(),
                frame_index: 0,
                elapsed: Duration::ZERO,
                command_remaining: Duration::ZERO,
                interaction: None,
                speech_bubble_remaining: None,
            },
        );
        self.refresh_pet_name_labels(owner_id);
        if self
            .active_character
            .as_ref()
            .is_some_and(|character| character.id == owner_id)
        {
            self.refresh_local_pet_equipment_art();
        }
    }

    pub(super) fn refresh_local_pet_equipment_art(&mut self) {
        let Some(owner_id) = self.active_character.as_ref().map(|character| character.id) else {
            return;
        };
        let equipment_id = self.inventory.as_ref().and_then(|inventory| {
            inventory
                .equipped()
                .find(|item| item.item_id / 10_000 == 180)
                .map(|item| item.item_id)
        });
        let Some(scene) = self.map_scene.as_ref() else {
            return;
        };
        let pets = scene
            .pets
            .iter()
            .filter(|((pet_owner_id, _), _)| *pet_owner_id == owner_id)
            .map(|(&(pet_owner_id, slot), pet)| (pet_owner_id, slot, pet.item_id))
            .collect::<Vec<_>>();
        if pets.is_empty() {
            return;
        }
        // The v83 equipped inventory has one Pet Outfit slot; render it on
        // the first summoned pet, matching the Cash Shop preview.
        let outfit_slot = pets.iter().map(|(_, slot, _)| *slot).min();

        let mut updates = Vec::with_capacity(pets.len());
        for (pet_owner_id, slot, pet_id) in pets {
            let animations = match equipment_id.filter(|_| Some(slot) == outfit_slot) {
                Some(equipment_id) => self
                    .load_pet_equipment_animations(pet_id, equipment_id)
                    .unwrap_or_else(|error| {
                        eprintln!(
                            "OpenStoryRS: could not load pet equipment {equipment_id} for pet {pet_id}: {error}"
                        );
                        PetEquipmentAnimations::default()
                    }),
                None => PetEquipmentAnimations::default(),
            };
            updates.push(((pet_owner_id, slot), animations));
        }
        if let Some(scene) = self.map_scene.as_mut() {
            for (key, animations) in updates {
                if let Some(pet) = scene.pets.get_mut(&key) {
                    pet.equipment_animations = animations;
                }
            }
        }
    }

    pub(super) fn update_pet_name(&mut self, character_id: i32, name: &str) {
        let Some(pet) = self
            .map_scene
            .as_mut()
            .and_then(|scene| scene.pets.get_mut(&(character_id, 0)))
        else {
            return;
        };
        pet.name = name.to_owned();
        self.refresh_pet_name_labels(character_id);
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
            if !interaction_frames
                .is_some_and(|(action, frames)| pet.play_interaction(action, frames))
            {
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
        if self
            .active_character
            .as_ref()
            .is_some_and(|character| character.id == character_id)
        {
            self.refresh_local_pet_equipment_art();
            self.refresh_pet_name_labels(character_id);
        }
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

    pub(super) fn spawn_dragon(&mut self, spawn: openstory_protocol::MapDragonSpawn) {
        let frames = match self.load_dragon_frames(spawn.job) {
            Ok(frames) => frames,
            Err(error) => {
                eprintln!(
                    "OpenStoryRS: could not load Evan dragon for job {}: {error}",
                    spawn.job
                );
                return;
            }
        };
        if frames.is_empty() {
            eprintln!(
                "OpenStoryRS: Evan dragon for job {} has no drawable animation",
                spawn.job
            );
            return;
        }
        let mut body = PhysicsBody::new(f64::from(spawn.x), f64::from(spawn.y));
        body.mode = PhysicsMode::Normal;
        body.foothold_id = 0;
        if let Some(scene) = self.map_scene.as_mut() {
            scene.dragons.insert(
                spawn.owner_id,
                MapDragonRuntime {
                    x: f64::from(spawn.x),
                    y: f64::from(spawn.y),
                    body,
                    flipped: false,
                    frames,
                    frame_index: 0,
                    elapsed: Duration::ZERO,
                },
            );
        }
    }

    pub(super) fn move_dragon(&mut self, movement: openstory_protocol::MapDragonMovement) {
        let Some(fragment) = movement.fragments.last() else {
            return;
        };
        let Some(dragon) = self
            .map_scene
            .as_mut()
            .and_then(|scene| scene.dragons.get_mut(&movement.owner_id))
        else {
            return;
        };
        match fragment.kind {
            openstory_protocol::MovementKind::Absolute
            | openstory_protocol::MovementKind::Teleport
            | openstory_protocol::MovementKind::JumpDown => {
                dragon.x = f64::from(fragment.x);
                dragon.y = f64::from(fragment.y);
            }
            openstory_protocol::MovementKind::Relative => {
                dragon.x += f64::from(fragment.x);
                dragon.y += f64::from(fragment.y);
            }
            openstory_protocol::MovementKind::None => return,
        }
        dragon.flipped = fragment.stance & 1 == 1;
        dragon.body.foothold_id = fragment.foothold;
        dragon.body.x = dragon.x;
        dragon.body.y = dragon.y;
        dragon.body.previous_x = dragon.x;
        dragon.body.previous_y = dragon.y;
    }

    pub(super) fn remove_dragon(&mut self, owner_id: i32) {
        if let Some(scene) = self.map_scene.as_mut() {
            scene.dragons.remove(&owner_id);
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
        let mut death_art = self.load_character_death_art(&update.look);
        if let Some(player) = self
            .map_scene
            .as_mut()
            .and_then(|scene| scene.remote_players.get_mut(&update.id))
        {
            if player.dead
                && let (Some(new_art), Some(previous_art)) =
                    (death_art.as_mut(), player.death_art.as_ref())
            {
                new_art.retain_timeline_from(previous_art);
            }
            player.look = update.look;
            player.face_expression = None;
            player.item_aura = item_aura;
            if death_art.is_some() {
                player.death_art = death_art;
            }
            player.avatar_frames = avatar_frames;
            player.active_avatar_frame =
                first_avatar_frame(&player.avatar_frames, player.desired_avatar_motion);
            player.avatar_elapsed = Duration::ZERO;
            player.name_label = name_label;
        }
        self.refresh_pet_name_labels(update.id);
    }

    pub(super) fn update_remote_guild_name(
        &mut self,
        update: openstory_protocol::RemoteGuildNameUpdate,
    ) {
        if !self
            .map_scene
            .as_ref()
            .is_some_and(|scene| scene.remote_players.contains_key(&update.character_id))
        {
            return;
        }
        let guild_label = match self.load_remote_guild_label(&update.name) {
            Ok(label) => label,
            Err(error) => {
                eprintln!("OpenStoryRS: could not update remote guild label: {error}");
                None
            }
        };
        if let Some(player) = self
            .map_scene
            .as_mut()
            .and_then(|scene| scene.remote_players.get_mut(&update.character_id))
        {
            player.guild_name = update.name;
            player.guild_label = guild_label;
        }
    }

    pub(super) fn update_remote_guild_mark(
        &mut self,
        update: openstory_protocol::RemoteGuildMarkUpdate,
    ) {
        if let Some(mark) = update.mark
            && let Some(player) = self
                .map_scene
                .as_mut()
                .and_then(|scene| scene.remote_players.get_mut(&update.character_id))
        {
            player._guild_mark = mark;
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

    pub(super) fn show_server_notice_popup(&mut self, message: &str) {
        self.pending_world_map_teleport = None;
        self.audio
            .play_ui_sound(&self.sound_archive, "UI.img/DlgNotice");
        let (width, height, pixels, panel, button, page_count) =
            match build_server_notice_art(&self.legacy_ui_archive, &self.font, message, 0) {
                Ok(art) => art,
                Err(error) => {
                    eprintln!("OpenStoryRS: could not build server notice: {error}");
                    return;
                }
            };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        match renderer.upload_bgra(width, height, &pixels) {
            Ok(texture) => {
                self.quit_confirmation = false;
                self.server_notice_popup = Some(ServerNoticePopup {
                    message: message.to_owned(),
                    page: 0,
                    page_count,
                    texture,
                    panel,
                    button,
                });
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            Err(error) => eprintln!("OpenStoryRS: could not upload server notice: {error}"),
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
            (15, 0) => "Enchant/Success",
            (15, _) => "Enchant/Failure",
            (16, 0) => "ItemMake/Success",
            (16, _) => "ItemMake/Failure",
            _ => return Ok(Vec::new()),
        };
        let path = format!("BasicEff.img/{effect_name}");
        self.load_character_effect_path(cache_key, &path)
    }

    pub(super) fn load_character_effect_path(
        &mut self,
        cache_key: (i8, i8),
        path: &str,
    ) -> Result<Vec<CharacterSkillEffectAnimation>, String> {
        if let Some(animations) = self.character_effect_cache.get(&cache_key) {
            return Ok(animations.clone());
        }
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
            vec![CharacterSkillEffectAnimation {
                z,
                flip_with_facing: false,
                frames,
            }]
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
                    self.refresh_skill_book_entries();
                    self.refresh_skill_book_texture();
                }
            }
            openstory_protocol::OpenUiRequest::BuddyList => {
                self.show_buddy_panel = true;
                self.refresh_buddy_texture();
            }
            openstory_protocol::OpenUiRequest::Messenger => self.open_messenger(),
            openstory_protocol::OpenUiRequest::MonsterBook => {
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
                    self.quest_log_tab = QuestLogTab::Active;
                    self.quest_log_scroll = 0;
                    self.quest_detail_scroll = 0;
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
