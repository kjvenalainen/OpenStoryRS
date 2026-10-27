fn prepare_login_buttons(
    buttons: Vec<LoginButtonAsset>,
) -> (Vec<LoginButtonAsset>, HashSet<usize>, HashSet<usize>) {
    let mut all_draw_indices = HashSet::new();
    let mut active_draw_indices = HashSet::new();
    for button in &buttons {
        // These actions are available before the network handshake, so
        // their idle art should not look disabled while the server connects.
        let initial_draw = if matches!(button.action, Some(LoginAction::Login | LoginAction::Guest))
        {
            button.mouse_over_draw.unwrap_or(button.normal_draw)
        } else {
            button.normal_draw
        };
        active_draw_indices.insert(initial_draw);
        all_draw_indices.insert(button.normal_draw);
        for draw_index in [
            button.mouse_over_draw,
            button.pressed_draw,
            button.disabled_draw,
            button.key_focused_draw,
        ]
        .into_iter()
        .flatten()
        {
            all_draw_indices.insert(draw_index);
        }
    }
    (buttons, all_draw_indices, active_draw_indices)
}

#[derive(Clone, Copy)]
enum LoginAction {
    Login,
    Guest,
    Quit,
    ToggleSaveId,
    SelectWorld(u8),
    SelectChannel(u8),
    EnterWorld,
    HighlightCharacter(usize),
    SelectCharacter(usize),
    DeleteCharacter(usize),
    ReturnToWorld,
    PreviousCharacterPage,
    NextCharacterPage,
    BeginCharacterCreation,
    ChooseCreationClass(u16),
    CycleCreationOption(usize, i8),
    ToggleCreationGender(bool),
    FocusCreationName,
    SubmitCharacterCreation,
    CancelCharacterCreation,
}

struct CharacterCreationState {
    job: Option<u16>,
    female: bool,
    name: String,
    name_focused: bool,
    pools: Vec<Vec<i32>>,
    indices: [usize; 8],
    awaiting_name: bool,
    awaiting_create: bool,
    notice: Option<String>,
}

impl CharacterCreationState {
    fn class_select() -> Self {
        Self {
            job: None,
            female: false,
            name: String::new(),
            name_focused: false,
            pools: Vec::new(),
            indices: [0; 8],
            awaiting_name: false,
            awaiting_create: false,
            notice: None,
        }
    }

    fn customize(job: u16, pools: Vec<Vec<i32>>) -> Self {
        Self {
            job: Some(job),
            female: false,
            name: String::new(),
            name_focused: true,
            pools,
            indices: [0; 8],
            awaiting_name: false,
            awaiting_create: false,
            notice: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PicEntryStep {
    Register,
    ConfirmRegistration,
    Verify,
    DeleteConfirm,
    Notice,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PicPurpose {
    SelectCharacter,
    DeleteCharacter,
}

struct PicEntry {
    character_id: i32,
    step: PicEntryStep,
    purpose: PicPurpose,
    pic_state: u8,
    input: String,
    first_input: String,
    notice: Option<String>,
}

fn load_login_scene(map: &NxArchive, legacy_ui: &NxArchive) -> Result<LoginSceneAssets, String> {
    let mut sprites = Vec::with_capacity(24);
    let mut actions = Vec::with_capacity(4);
    let mut login_buttons = Vec::with_capacity(8);
    let mut animated_art = Vec::with_capacity(6);
    let mut cache = MapArtCache::default();

    append_sprite(
        &mut sprites,
        map,
        "Back/login.img/back/11",
        512.0,
        384.0,
        1.28,
    )?;
    append_sprite(
        &mut sprites,
        map,
        "Obj/login.img/Title/logo/0/0",
        521.0,
        228.0,
        1.0,
    )?;
    append_sprite(
        &mut sprites,
        map,
        "Obj/login.img/Title/signboard/0/0",
        622.0,
        414.0,
        1.0,
    )?;

    for effect in 0..6 {
        let path = format!("Login.img/Title/effect/{effect}");
        let source = legacy_ui
            .root()
            .resolve(&path)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("login animation {path:?} is missing"))?;
        let first_sprite = sprites.len();
        if let Some(animation) = append_map_art_animation(
            &mut sprites,
            &source,
            &path,
            847.0,
            74.0,
            255,
            false,
            &mut cache,
        )? {
            animated_art.push(animation);
        }
        for sprite in &mut sprites[first_sprite..] {
            sprite.rect.blend_mode = BlendMode::Additive;
        }
    }
    append_sprite(
        &mut sprites,
        legacy_ui,
        "Login.img/Common/frame",
        512.0,
        384.0,
        1.28,
    )?;

    append_login_button(
        &mut sprites,
        &mut actions,
        &mut login_buttons,
        legacy_ui,
        "BtLogin",
        707.0,
        309.0,
        Some(LoginAction::Login),
    )?;
    append_login_button(
        &mut sprites,
        &mut actions,
        &mut login_buttons,
        legacy_ui,
        "BtGuestLogin",
        707.0,
        352.0,
        Some(LoginAction::Guest),
    )?;
    append_login_button(
        &mut sprites,
        &mut actions,
        &mut login_buttons,
        legacy_ui,
        "BtLoginIDSave",
        522.0,
        389.0,
        Some(LoginAction::ToggleSaveId),
    )?;
    append_login_button(
        &mut sprites,
        &mut actions,
        &mut login_buttons,
        legacy_ui,
        "BtLoginIDLost",
        602.0,
        389.0,
        None,
    )?;
    append_login_button(
        &mut sprites,
        &mut actions,
        &mut login_buttons,
        legacy_ui,
        "BtPasswdLost",
        692.0,
        389.0,
        None,
    )?;
    append_login_button(
        &mut sprites,
        &mut actions,
        &mut login_buttons,
        legacy_ui,
        "BtNew",
        502.0,
        434.0,
        None,
    )?;
    append_login_button(
        &mut sprites,
        &mut actions,
        &mut login_buttons,
        legacy_ui,
        "BtHomePage",
        602.0,
        434.0,
        None,
    )?;
    append_login_button(
        &mut sprites,
        &mut actions,
        &mut login_buttons,
        legacy_ui,
        "BtQuit",
        702.0,
        434.0,
        Some(LoginAction::Quit),
    )?;
    let unchecked_index = sprites.len();
    append_sprite(
        &mut sprites,
        &legacy_ui,
        "Login.img/Title/check/0",
        502.0,
        389.0,
        1.0,
    )?;
    let unchecked_rect = sprites[unchecked_index].rect;
    let checked_index = sprites.len();
    append_sprite(
        &mut sprites,
        &legacy_ui,
        "Login.img/Title/check/1",
        502.0,
        389.0,
        1.0,
    )?;
    let checked_rect = sprites[checked_index].rect;

    Ok(LoginSceneAssets {
        sprites,
        actions,
        map_scene: None,
        animated_art,
        login_buttons,
        save_id_indicator_draws: Some((
            unchecked_index,
            unchecked_rect,
            checked_index,
            checked_rect,
        )),
    })
}

fn load_world_selection_scene(
    map: &NxArchive,
    legacy_ui: &NxArchive,
    worlds: &[openstory_protocol::WorldInfo],
    selected_world: Option<u8>,
    selected_channel: u8,
) -> Result<LoginSceneAssets, String> {
    const UI_SCALE: f32 = 1.28;
    let design_point = |x: f32, y: f32| (x * UI_SCALE, y * UI_SCALE);
    let mut sprites = Vec::with_capacity(32);
    let mut actions = Vec::with_capacity(24);
    let mut animated_art = Vec::with_capacity(1);

    let backdrop = map
        .root()
        .resolve("Obj/login.img/WorldSelect/neoCity/0/0")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "world selection backdrop was not found in Map.nx".to_owned())?;
    let backdrop_bitmap = backdrop
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "world selection backdrop is not a bitmap".to_owned())?;
    let backdrop_scale = (1024.0 / f32::from(backdrop_bitmap.width()))
        .max(768.0 / f32::from(backdrop_bitmap.height()));
    append_sprite(
        &mut sprites,
        map,
        "Obj/login.img/WorldSelect/neoCity/0/0",
        512.0,
        384.0,
        backdrop_scale,
    )?;
    append_sprite(
        &mut sprites,
        legacy_ui,
        "Login.img/Common/frame",
        512.0,
        384.0,
        UI_SCALE,
    )?;

    if let Some(world_id) = selected_world {
        let (scroll_x, scroll_y) = design_point(143.0, 100.0);
        append_sprite(
            &mut sprites,
            legacy_ui,
            "Login.img/WorldSelect/scroll/0/3",
            scroll_x,
            scroll_y,
            UI_SCALE,
        )?;
        let (divider_x, divider_y) = design_point(143.0 + 37.0, 100.0 + 136.0);
        append_optional_sprite_at_top_left(
            &mut sprites,
            legacy_ui,
            "Login.img/WorldSelect/chBackgrn",
            divider_x,
            divider_y,
            UI_SCALE,
        )?;

        let title_path = format!("Login.img/WorldSelect/world/{world_id}");
        if let Some(title) = legacy_ui
            .root()
            .resolve(&title_path)
            .map_err(|error| error.to_string())?
        {
            let title_width = title
                .bitmap()
                .map_err(|error| error.to_string())?
                .map(|bitmap| f32::from(bitmap.width()))
                .unwrap_or(0.0);
            let title_design_x = 143.0 + 37.0 + (449.0 - title_width) / 2.0;
            let (title_x, title_y) = design_point(title_design_x, 100.0 + 135.0);
            append_node_sprite(
                &mut sprites,
                &title,
                &title_path,
                title_x,
                title_y,
                UI_SCALE,
            )?;
        }
    }

    // The hanging rod and world planks render over the scroll in the
    // legacy client, so keep them above the open parchment here too.
    let (hanger_x, hanger_y) = design_point(400.0, 80.0);
    append_optional_sprite(
        &mut sprites,
        map,
        "Obj/login.img/WorldSelect/signboard/0/0",
        hanger_x,
        hanger_y,
        UI_SCALE,
    )?;

    let mut world_index = 0;
    for world in worlds
        .iter()
        .filter(|world| !world.channels.is_empty())
        .take(20)
    {
        let path = format!("Login.img/WorldSelect/BtWorld/{}/normal/0", world.id);
        let (anchor_x, anchor_y) = design_point(182.0 + world_index as f32 * 27.0, 158.0);
        if let Some(rect) =
            append_optional_sprite(&mut sprites, legacy_ui, &path, anchor_x, anchor_y, UI_SCALE)?
        {
            actions.push(LoginActionRegion {
                rect,
                action: LoginAction::SelectWorld(world.id),
            });
            world_index += 1;
        }
    }

    if let Some(world_id) = selected_world {
        if let Some(world) = worlds.iter().find(|world| world.id == world_id) {
            for (channel_index, _channel) in world.channels.iter().take(20).enumerate() {
                let channel_index = channel_index as u8;
                let column = i16::from(channel_index % 4);
                let row = i16::from(channel_index / 4);
                let (anchor_x, anchor_y) = design_point(
                    143.0 + 55.0 + f32::from(column * 106),
                    100.0 + 205.0 + f32::from(row * 35),
                );
                let path = format!("Login.img/WorldSelect/channel/{channel_index}/normal");
                if let Some(rect) = append_optional_sprite(
                    &mut sprites,
                    legacy_ui,
                    &path,
                    anchor_x,
                    anchor_y,
                    UI_SCALE,
                )? {
                    actions.push(LoginActionRegion {
                        rect,
                        action: LoginAction::SelectChannel(channel_index),
                    });
                }
                if channel_index == selected_channel {
                    let (highlight_x, highlight_y) = design_point(
                        143.0 + 55.0 + f32::from(column * 106) + 46.0,
                        100.0 + 205.0 + f32::from(row * 35) + 15.0,
                    );
                    if let Some(mut animation) = append_scaled_ui_animation(
                        &mut sprites,
                        legacy_ui,
                        "Login.img/WorldSelect/channel/chSelect",
                        highlight_x,
                        highlight_y,
                        UI_SCALE,
                    )? {
                        // The native channel highlight animates once on
                        // selection, then holds its final red swirl frame.
                        animation.hold_last = true;
                        animated_art.push(animation);
                    }
                }
            }
        }
        let (enter_x, enter_y) = design_point(143.0 + 361.0, 100.0 + 344.0);
        if let Some(rect) = append_optional_sprite(
            &mut sprites,
            legacy_ui,
            "Login.img/WorldSelect/BtGoworld/normal/0",
            enter_x,
            enter_y,
            UI_SCALE,
        )? {
            actions.push(LoginActionRegion {
                rect,
                action: LoginAction::EnterWorld,
            });
        }
    }

    let (exit_x, exit_y) = design_point(8.0, 515.0);
    if let Some(rect) = append_optional_sprite(
        &mut sprites,
        legacy_ui,
        "Login.img/Common/BtExit/normal/0",
        exit_x,
        exit_y,
        UI_SCALE,
    )? {
        actions.push(LoginActionRegion {
            rect,
            action: LoginAction::Quit,
        });
    }
    Ok(LoginSceneAssets {
        sprites,
        actions,
        map_scene: None,
        animated_art,
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn load_character_selection_scene(
    map: &NxArchive,
    legacy_ui: &NxArchive,
    character_archive: &NxArchive,
    characters: &openstory_protocol::CharacterList,
    page: usize,
    selected_character: usize,
    status: Option<&str>,
    font: &fontdue::Font,
    action_focus: Option<usize>,
) -> Result<LoginSceneAssets, String> {
    const UI_SCALE: f32 = 1.28;
    let mut sprites = Vec::with_capacity(24);
    let mut actions = Vec::with_capacity(CHARACTER_PAGE_SIZE + 7);
    let mut animated_art = Vec::with_capacity(2);

    append_tiled_background(
        &mut sprites,
        map,
        "Back/login.img/back/1",
        1024,
        768,
        UI_SCALE,
    )?;
    append_optional_sprite(
        &mut sprites,
        map,
        "Back/login.img/back/10",
        77.0 * UI_SCALE,
        32.0 * UI_SCALE,
        UI_SCALE,
    )?;
    append_optional_sprite(
        &mut sprites,
        map,
        "Back/login.img/back/15",
        143.0 * UI_SCALE,
        81.0 * UI_SCALE,
        UI_SCALE,
    )?;
    append_optional_sprite(
        &mut sprites,
        map,
        "Back/login.img/back/13",
        141.0 * UI_SCALE,
        695.0 * UI_SCALE,
        UI_SCALE,
    )?;
    append_optional_sprite(
        &mut sprites,
        map,
        "Obj/login.img/CharSelect/signboard/0/0",
        571.0 * UI_SCALE,
        257.0 * UI_SCALE,
        UI_SCALE,
    )?;
    append_optional_sprite(
        &mut sprites,
        map,
        "Back/login.img/back/14",
        315.0 * UI_SCALE,
        393.0 * UI_SCALE,
        UI_SCALE,
    )?;

    if let Some(status) = status {
        append_text_sprite(&mut sprites, font, status, 300.0, 140.0, 12.0);
    }

    let mut avatar_cache = MapArtCache::default();
    let slots = usize::try_from(characters.slots.max(0))
        .unwrap_or(0)
        .max(characters.characters.len());
    let start = page.saturating_mul(CHARACTER_PAGE_SIZE);
    for visible_index in 0..CHARACTER_PAGE_SIZE {
        let index = start + visible_index;
        if index >= slots {
            continue;
        }
        let feet_x = (278.0 + visible_index as f32 * 72.0) * UI_SCALE;
        let feet_y = 374.0 * UI_SCALE;
        if let Some(character) = characters.characters.get(index) {
            let selected = index == selected_character;
            if selected {
                if let Some(animation) = append_scaled_ui_animation(
                    &mut sprites,
                    legacy_ui,
                    "Login.img/CharSelect/effect/1",
                    feet_x,
                    feet_y - 385.0 * UI_SCALE,
                    UI_SCALE,
                )? {
                    animated_art.push(animation);
                }

                let tall = character.stats.job >= 1000;
                let scroll_path = if tall {
                    "Login.img/CharSelect/scroll/2/3"
                } else {
                    "Login.img/CharSelect/scroll/0/3"
                };
                let sheet_path = if tall {
                    "Login.img/CharSelect/charInfo1"
                } else {
                    "Login.img/CharSelect/charInfo"
                };
                let scroll_x = (278.0 + visible_index as f32 * 72.0 - 108.0) * UI_SCALE;
                let scroll_y = 125.0 * UI_SCALE;
                append_optional_sprite(
                    &mut sprites,
                    legacy_ui,
                    scroll_path,
                    scroll_x,
                    scroll_y,
                    UI_SCALE,
                )?;
                let sheet_x = (278.0 + visible_index as f32 * 72.0 - 108.0 + 17.0) * UI_SCALE;
                let sheet_y = (125.0 + 36.0) * UI_SCALE;
                append_optional_sprite_at_top_left(
                    &mut sprites,
                    legacy_ui,
                    sheet_path,
                    sheet_x,
                    sheet_y,
                    UI_SCALE,
                )?;

                let info_size = 12.0 * UI_SCALE;
                let info_color = [112, 79, 55];
                let info_value = |sprites: &mut Vec<DecodedSprite>,
                                  value: &str,
                                  x: f32,
                                  y: f32,
                                  centered: bool| {
                    let width = measured_text_width(font, value, info_size);
                    let left = if centered { x - width * 0.5 } else { x - width };
                    append_character_creation_option_text(
                        sprites, font, value, left, y, info_size, info_color,
                    );
                };
                info_value(
                    &mut sprites,
                    &character_job_name(character.stats.job),
                    sheet_x + 176.0 * UI_SCALE,
                    sheet_y - UI_SCALE,
                    false,
                );
                info_value(
                    &mut sprites,
                    &character.stats.level.to_string(),
                    sheet_x + 88.0 * UI_SCALE,
                    sheet_y + 18.0 * UI_SCALE,
                    false,
                );
                info_value(
                    &mut sprites,
                    &character.stats.fame.to_string(),
                    sheet_x + 176.0 * UI_SCALE,
                    sheet_y + 18.0 * UI_SCALE,
                    false,
                );
                info_value(
                    &mut sprites,
                    &character.stats.strength.to_string(),
                    sheet_x + 88.0 * UI_SCALE,
                    sheet_y + 37.0 * UI_SCALE,
                    false,
                );
                info_value(
                    &mut sprites,
                    &character.stats.intelligence.to_string(),
                    sheet_x + 176.0 * UI_SCALE,
                    sheet_y + 37.0 * UI_SCALE,
                    false,
                );
                info_value(
                    &mut sprites,
                    &character.stats.dexterity.to_string(),
                    sheet_x + 88.0 * UI_SCALE,
                    sheet_y + 56.0 * UI_SCALE,
                    false,
                );
                info_value(
                    &mut sprites,
                    &character.stats.luck.to_string(),
                    sheet_x + 176.0 * UI_SCALE,
                    sheet_y + 56.0 * UI_SCALE,
                    false,
                );
                let rank = character
                    .stats
                    .rank
                    .filter(|(rank, _)| *rank > 0)
                    .map(|(rank, _)| rank.to_string())
                    .unwrap_or_else(|| "--".to_owned());
                info_value(
                    &mut sprites,
                    &rank,
                    sheet_x + 92.0 * UI_SCALE,
                    sheet_y + 93.0 * UI_SCALE,
                    true,
                );
            }

            append_character_preview(
                &mut sprites,
                character_archive,
                &character.look,
                feet_x,
                feet_y,
                UI_SCALE,
                true,
                &mut avatar_cache,
            )?;
            append_character_selection_name_tag(
                &mut sprites,
                legacy_ui,
                font,
                &character.stats.name,
                feet_x,
                feet_y,
                UI_SCALE,
                selected,
            )?;
            if selected {
                if let Some(animation) = append_scaled_ui_animation(
                    &mut sprites,
                    legacy_ui,
                    "Login.img/CharSelect/effect/0",
                    feet_x,
                    feet_y - 385.0 * UI_SCALE,
                    UI_SCALE,
                )? {
                    animated_art.push(animation);
                }
            }

            let slot_width = 70.0 * UI_SCALE;
            actions.push(LoginActionRegion {
                rect: SpriteRect {
                    // Expand the legacy slot hitbox upward to cover the
                    // full character preview without overlapping neighbors.
                    x: feet_x - slot_width * 0.5,
                    y: feet_y - 150.0,
                    width: slot_width,
                    height: 192.0,
                    blend_mode: BlendMode::Alpha,
                },
                action: LoginAction::HighlightCharacter(index),
            });
        } else {
            append_optional_sprite(
                &mut sprites,
                legacy_ui,
                "Login.img/CharSelect/character/1/0",
                feet_x,
                feet_y,
                UI_SCALE,
            )?;
        }
    }

    let page_count = slots.max(1).div_ceil(CHARACTER_PAGE_SIZE);
    // The native arrows frame the three slots even when there is only one
    // page. They become interactive only when additional slots exist.
    let previous = append_optional_sprite(
        &mut sprites,
        legacy_ui,
        "Login.img/CharSelect/pageL/0/0",
        188.0 * UI_SCALE,
        337.0 * UI_SCALE,
        UI_SCALE,
    )?;
    let next = append_optional_sprite(
        &mut sprites,
        legacy_ui,
        "Login.img/CharSelect/pageR/0/0",
        499.0 * UI_SCALE,
        337.0 * UI_SCALE,
        UI_SCALE,
    )?;
    if page_count > 1 {
        if let Some(rect) = previous {
            actions.push(LoginActionRegion {
                rect,
                action: LoginAction::PreviousCharacterPage,
            });
        }
        if let Some(rect) = next {
            actions.push(LoginActionRegion {
                rect,
                action: LoginAction::NextCharacterPage,
            });
        }
    }

    if let Some(rect) = append_optional_sprite(
        &mut sprites,
        legacy_ui,
        "Login.img/Common/BtStart/normal/0",
        -6.0 * UI_SCALE,
        515.0 * UI_SCALE,
        UI_SCALE,
    )? {
        actions.push(LoginActionRegion {
            rect,
            action: LoginAction::ReturnToWorld,
        });
    }
    let can_create = characters.characters.len()
        < usize::try_from(characters.slots.max(0)).unwrap_or(0)
        && characters.characters.len() < 64;
    if can_create {
        if let Some(rect) = append_optional_sprite(
            &mut sprites,
            legacy_ui,
            "Login.img/CharSelect/BtNew/normal/0",
            521.0 * UI_SCALE,
            192.0 * UI_SCALE,
            UI_SCALE,
        )? {
            if action_focus == Some(1) {
                append_ui_outline_rect(
                    &mut sprites,
                    "character-action-focus:new",
                    rect,
                    3.0,
                    [255, 220, 132, 255],
                );
            }
            actions.push(LoginActionRegion {
                rect: padded_hit_rect(rect, 16.0, 12.0),
                action: LoginAction::BeginCharacterCreation,
            });
        }
    } else {
        append_optional_sprite(
            &mut sprites,
            legacy_ui,
            "Login.img/CharSelect/BtNew/disabled/0",
            521.0 * UI_SCALE,
            192.0 * UI_SCALE,
            UI_SCALE,
        )?;
    }
    if let Some(index) = characters
        .characters
        .get(selected_character)
        .map(|_| selected_character)
    {
        let select_rect = append_optional_sprite(
            &mut sprites,
            legacy_ui,
            "Login.img/CharSelect/BtSelect/normal/0",
            521.0 * UI_SCALE,
            154.0 * UI_SCALE,
            UI_SCALE,
        )?;
        // Derive the target from the decoded sprite: NX origins can place
        // the painted sign well above its anchor, so a separately
        // hard-coded rectangle can miss the visible button after scaling.
        // A small margin makes the full wooden sign easy to click.
        let select_hit_rect = select_rect.map_or(
            SpriteRect {
                x: 632.0,
                y: 169.0,
                width: 224.0,
                height: 82.0,
                blend_mode: BlendMode::Alpha,
            },
            |rect| SpriteRect {
                x: rect.x - 16.0,
                y: rect.y - 12.0,
                width: rect.width + 32.0,
                height: rect.height + 24.0,
                blend_mode: BlendMode::Alpha,
            },
        );
        if action_focus == Some(0) {
            append_ui_outline_rect(
                &mut sprites,
                "character-action-focus:select",
                select_rect.unwrap_or(select_hit_rect),
                3.0,
                [255, 220, 132, 255],
            );
        }
        actions.push(LoginActionRegion {
            rect: select_hit_rect,
            action: LoginAction::SelectCharacter(index),
        });
        if let Some(rect) = append_optional_sprite(
            &mut sprites,
            legacy_ui,
            "Login.img/CharSelect/BtDelete/normal/0",
            521.0 * UI_SCALE,
            240.0 * UI_SCALE,
            UI_SCALE,
        )? {
            if action_focus == Some(2) {
                append_ui_outline_rect(
                    &mut sprites,
                    "character-action-focus:delete",
                    rect,
                    3.0,
                    [255, 220, 132, 255],
                );
            }
            actions.push(LoginActionRegion {
                rect: padded_hit_rect(rect, 16.0, 12.0),
                action: LoginAction::DeleteCharacter(index),
            });
        }
    } else {
        append_optional_sprite(
            &mut sprites,
            legacy_ui,
            "Login.img/CharSelect/BtSelect/disabled/0",
            521.0 * UI_SCALE,
            154.0 * UI_SCALE,
            UI_SCALE,
        )?;
        append_optional_sprite(
            &mut sprites,
            legacy_ui,
            "Login.img/CharSelect/BtDelete/disabled/0",
            521.0 * UI_SCALE,
            240.0 * UI_SCALE,
            UI_SCALE,
        )?;
    }
    Ok(LoginSceneAssets {
        sprites,
        actions,
        map_scene: None,
        animated_art,
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn load_character_class_scene(
    map: &NxArchive,
    legacy_ui: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let mut sprites = Vec::with_capacity(16);
    let mut actions = Vec::with_capacity(4);
    append_sprite(
        &mut sprites,
        map,
        "Back/login.img/back/11",
        512.0,
        384.0,
        1.28,
    )?;
    append_text_sprite(
        &mut sprites,
        font,
        "Choose a character class",
        270.0,
        96.0,
        20.0,
    );

    let classes = [
        (1_u16, "Explorer", "normal/BtNormal"),
        (0_u16, "Cygnus Knight", "knight/BtKnight"),
        (2_u16, "Aran", "aran/BtAran"),
    ];
    let positions = [(102.0_f32, 192.0_f32), (404.0, 192.0), (673.0, 192.0)];
    for ((job, label, asset), (x, y)) in classes.into_iter().zip(positions) {
        let base = format!("Login.img/RaceSelect/{asset}/normal/0");
        if let Some(rect) = append_optional_sprite(&mut sprites, legacy_ui, &base, x, y, 1.28)? {
            actions.push(LoginActionRegion {
                rect,
                action: LoginAction::ChooseCreationClass(job),
            });
        }
        append_text_sprite(&mut sprites, font, label, x + 14.0, y + 290.0, 15.0);
    }
    if let Some(rect) = append_optional_sprite(
        &mut sprites,
        legacy_ui,
        "Login.img/Common/BtStart/normal/0",
        0.0,
        515.0 * 1.28,
        1.28,
    )? {
        actions.push(LoginActionRegion {
            rect,
            action: LoginAction::CancelCharacterCreation,
        });
    }
    Ok(LoginSceneAssets {
        sprites,
        actions,
        map_scene: None,
        animated_art: Vec::new(),
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn load_character_customization_scene(
    map: &NxArchive,
    legacy_ui: &NxArchive,
    character_archive: &NxArchive,
    string_archive: &NxArchive,
    item_name_cache: &mut HashMap<i32, Option<String>>,
    caret_visible: bool,
    creation: &CharacterCreationState,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let job = creation
        .job
        .ok_or_else(|| "creation class is not selected".to_owned())?;
    let (job_name, customize_asset) = match job {
        0 => ("Cygnus Knight", "NewCharKnight"),
        1 => ("Explorer", "NewChar"),
        2 => ("Aran", "NewCharAran"),
        _ => return Err(format!("unsupported character-creation job {job}")),
    };
    let custom_path = format!("Login.img/{customize_asset}");
    let mut sprites = Vec::with_capacity(48);
    let mut actions = Vec::with_capacity(24);
    append_tiled_background(&mut sprites, map, "Back/login.img/back/1", 1024, 768, 1.28)?;
    append_sprite(
        &mut sprites,
        legacy_ui,
        "Login.img/Common/frame",
        512.0,
        384.0,
        1.28,
    )?;
    append_text_sprite(&mut sprites, font, "Create Character", 385.0, 125.0, 22.0);
    append_text_sprite(&mut sprites, font, job_name, 385.0, 155.0, 16.0);

    // These are the actual v83 class-specific customization boards. The
    // newer C++ branch refers to a `CustomizeChar` node absent from the
    // supplied UI-v83.nx; these `NewChar*` assets are the compatible nodes.
    append_optional_sprite(
        &mut sprites,
        legacy_ui,
        &format!("{custom_path}/charSet"),
        579.0,
        143.0,
        1.28,
    )?;

    let selected = |pool: usize| -> Option<i32> {
        creation
            .pools
            .get(pool)?
            .get(*creation.indices.get(pool)?)
            .copied()
    };
    if let (
        Some(face),
        Some(hair),
        Some(color),
        Some(skin),
        Some(top),
        Some(bottom),
        Some(shoes),
        Some(weapon),
    ) = (
        selected(0),
        selected(1),
        selected(2),
        selected(3),
        selected(4),
        selected(5),
        selected(6),
        selected(7),
    ) {
        let hair_id = hair.saturating_add(color);
        let look = openstory_protocol::CharacterLook {
            female: creation.female,
            skin: u8::try_from(skin).unwrap_or(0),
            face_id: face,
            hair_id,
            equips: vec![(-5, top), (-6, bottom), (-7, shoes), (-11, weapon)],
            weapon_id: weapon,
            ..Default::default()
        };
        let mut avatar_cache = MapArtCache::default();
        append_character_preview(
            &mut sprites,
            character_archive,
            &look,
            390.0,
            490.0,
            1.2,
            true,
            &mut avatar_cache,
        )?;
    }

    let gender_y = 250.0;
    for (female, label, x) in [(false, "Male", 590.0), (true, "Female", 700.0)] {
        append_text_sprite(&mut sprites, font, label, x, gender_y, 14.0);
        actions.push(LoginActionRegion {
            rect: SpriteRect {
                x: x - 8.0,
                y: gender_y - 3.0,
                width: 100.0,
                height: 26.0,
                blend_mode: BlendMode::Alpha,
            },
            action: LoginAction::ToggleCreationGender(female),
        });
    }

    let labels = [
        "Face",
        "Hair",
        "Hair color",
        "Skin",
        "Top",
        "Bottom",
        "Shoes",
        "Weapon",
    ];
    for (index, label) in labels.iter().enumerate() {
        let y = 277.0 + index as f32 * 24.0;
        let value = selected(index).unwrap_or_default();
        let value_name =
            character_creation_option_name(index, value, string_archive, item_name_cache);
        append_character_creation_option_text(
            &mut sprites,
            font,
            &format!("{label}: {value_name}"),
            593.0,
            y,
            13.0,
            [82, 53, 29],
        );
        append_character_creation_option_text(
            &mut sprites,
            font,
            "<",
            808.0,
            y,
            16.0,
            [82, 53, 29],
        );
        append_character_creation_option_text(
            &mut sprites,
            font,
            ">",
            847.0,
            y,
            16.0,
            [82, 53, 29],
        );
        for (direction, x) in [(-1_i8, 800.0), (1_i8, 839.0)] {
            actions.push(LoginActionRegion {
                rect: SpriteRect {
                    x,
                    y: y - 2.0,
                    width: 34.0,
                    height: 25.0,
                    blend_mode: BlendMode::Alpha,
                },
                action: LoginAction::CycleCreationOption(index, direction),
            });
        }
    }

    let pending = creation.awaiting_name || creation.awaiting_create;
    let name_text = if creation.name.is_empty() {
        "Enter name".to_owned()
    } else {
        creation.name.clone()
    };
    let caret = if creation.name_focused && !pending && caret_visible {
        "|"
    } else {
        ""
    };
    append_character_creation_option_text(
        &mut sprites,
        font,
        "Character name",
        610.0,
        539.0,
        12.0,
        [82, 53, 29],
    );
    append_character_creation_option_text(
        &mut sprites,
        font,
        &format!("{name_text}{caret}"),
        624.0,
        578.0,
        14.0,
        [54, 39, 26],
    );
    actions.push(LoginActionRegion {
        rect: SpriteRect {
            x: 610.0,
            y: 557.0,
            width: 228.0,
            height: 65.0,
            blend_mode: BlendMode::Alpha,
        },
        action: LoginAction::FocusCreationName,
    });

    if let Some(notice) = creation.notice.as_deref() {
        append_character_creation_option_text(
            &mut sprites,
            font,
            notice,
            584.0,
            630.0,
            12.0,
            [142, 47, 33],
        );
    }
    let yes_state = if pending { "disabled" } else { "normal" };
    let no_state = if pending { "disabled" } else { "normal" };
    let yes_rect = append_optional_sprite(
        &mut sprites,
        legacy_ui,
        &format!("{custom_path}/BtYes/{yes_state}/0"),
        614.0,
        477.0,
        1.0,
    )?;
    if !pending && let Some(rect) = yes_rect {
        actions.push(LoginActionRegion {
            rect,
            action: LoginAction::SubmitCharacterCreation,
        });
    }
    let no_rect = append_optional_sprite(
        &mut sprites,
        legacy_ui,
        &format!("{custom_path}/BtNo/{no_state}/0"),
        713.0,
        477.0,
        1.0,
    )?;
    if !pending && let Some(rect) = no_rect {
        actions.push(LoginActionRegion {
            rect,
            action: LoginAction::CancelCharacterCreation,
        });
    }
    if pending {
        actions.retain(|action| {
            !matches!(
                action.action,
                LoginAction::CycleCreationOption(_, _)
                    | LoginAction::ToggleCreationGender(_)
                    | LoginAction::FocusCreationName
                    | LoginAction::SubmitCharacterCreation
                    | LoginAction::CancelCharacterCreation
            )
        });
    }

    Ok(LoginSceneAssets {
        sprites,
        actions,
        map_scene: None,
        animated_art: Vec::new(),
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn append_character_creation_option_text(
    sprites: &mut Vec<DecodedSprite>,
    font: &fontdue::Font,
    text: &str,
    x: f32,
    y: f32,
    font_size: f32,
    color: [u8; 3],
) {
    const WIDTH: u32 = 360;
    const HEIGHT: u32 = 28;
    let mut pixels = rasterize_label(font, text, font_size, WIDTH, HEIGHT);
    for pixel in pixels.chunks_exact_mut(4) {
        if pixel[3] > 0 {
            pixel[..3].copy_from_slice(&color);
        }
    }
    sprites.push(DecodedSprite {
        texture_key: format!(
            "character-create-label:{font_size}:{text}:{:02x}{:02x}{:02x}",
            color[0], color[1], color[2]
        ),
        rect: SpriteRect {
            x,
            y,
            width: WIDTH as f32,
            height: HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: WIDTH,
        texture_height: HEIGHT,
        pixels: pixels.into(),
    });
}

fn append_character_selection_name_tag(
    sprites: &mut Vec<DecodedSprite>,
    legacy_ui: &NxArchive,
    font: &fontdue::Font,
    name: &str,
    feet_x: f32,
    feet_y: f32,
    ui_scale: f32,
    selected: bool,
) -> Result<(), String> {
    let tag_path = if selected {
        "Login.img/CharSelect/nameTag/1"
    } else {
        "Login.img/CharSelect/nameTag/0"
    };
    let text_size = 12.0 * ui_scale;
    let text_width = measured_text_width(font, name, text_size);
    let tag_center_x = feet_x + 2.0 * ui_scale;
    let tag_x = tag_center_x - (6.0 * ui_scale + text_width * 0.5);
    let tag_y = feet_y + 2.0 * ui_scale;

    append_optional_sprite_at_top_left(
        sprites,
        legacy_ui,
        &format!("{tag_path}/0"),
        tag_x,
        tag_y,
        ui_scale,
    )?;
    let mut offset = 0.0;
    while offset < text_width {
        append_optional_sprite_at_top_left(
            sprites,
            legacy_ui,
            &format!("{tag_path}/1"),
            tag_x + 6.0 * ui_scale + offset,
            tag_y,
            ui_scale,
        )?;
        offset += 9.0 * ui_scale;
    }
    append_optional_sprite_at_top_left(
        sprites,
        legacy_ui,
        &format!("{tag_path}/2"),
        tag_x + 6.0 * ui_scale + text_width,
        tag_y,
        ui_scale,
    )?;

    let text_x = tag_center_x - text_width * 0.5;
    let shadow_color = if selected { [65, 49, 40] } else { [42, 35, 29] };
    let text_color = if selected {
        [255, 255, 255]
    } else {
        [194, 183, 156]
    };
    append_character_creation_option_text(
        sprites,
        font,
        name,
        text_x + 0.7,
        tag_y + 0.7,
        text_size,
        shadow_color,
    );
    append_character_creation_option_text(
        sprites, font, name, text_x, tag_y, text_size, text_color,
    );
    Ok(())
}

fn character_creation_option_name(
    option: usize,
    item_id: i32,
    string_archive: &NxArchive,
    item_name_cache: &mut HashMap<i32, Option<String>>,
) -> String {
    fn lookup_name(archive: &NxArchive, path: &str) -> Option<String> {
        let node = archive.root().resolve(path).ok().flatten()?;
        match node.value().ok()? {
            NodeValue::String(value) if !value.trim().is_empty() => Some(value),
            _ => None,
        }
    }

    match option {
        0 => lookup_name(string_archive, &format!("Eqp.img/Eqp/Face/{item_id}/name"))
            .unwrap_or_else(|| format!("Face {item_id}")),
        1 => lookup_name(string_archive, &format!("Eqp.img/Eqp/Hair/{item_id}/name"))
            .unwrap_or_else(|| format!("Hair {item_id}")),
        2 => [
            "Black", "Red", "Orange", "Blonde", "Green", "Blue", "Violet", "Brown",
        ]
        .get(usize::try_from(item_id).unwrap_or(usize::MAX))
        .copied()
        .unwrap_or("Unknown color")
        .to_owned(),
        3 => [
            "Light",
            "Tan",
            "Dark",
            "Pale",
            "Blue",
            "Green",
            "Unknown skin",
            "Unknown skin",
            "Unknown skin",
            "Grey",
            "Pink",
            "Red",
        ]
        .get(usize::try_from(item_id).unwrap_or(usize::MAX))
        .copied()
        .unwrap_or("Unknown skin")
        .to_owned(),
        4..=7 => item_name(item_id, string_archive, item_name_cache),
        _ => format!("Appearance {item_id}"),
    }
}

fn append_ui_solid_rect(
    sprites: &mut Vec<DecodedSprite>,
    texture_key: &str,
    x: f32,
    y: f32,
    width: u32,
    height: u32,
    color: [u8; 4],
) {
    let mut pixels = vec![0; width as usize * height as usize * 4];
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.copy_from_slice(&color);
    }
    sprites.push(DecodedSprite {
        texture_key: texture_key.to_owned(),
        rect: SpriteRect {
            x,
            y,
            width: width as f32,
            height: height as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: width,
        texture_height: height,
        pixels: pixels.into(),
    });
}

fn padded_hit_rect(rect: SpriteRect, horizontal: f32, vertical: f32) -> SpriteRect {
    SpriteRect {
        x: rect.x - horizontal,
        y: rect.y - vertical,
        width: rect.width + horizontal * 2.0,
        height: rect.height + vertical * 2.0,
        blend_mode: rect.blend_mode,
    }
}

fn append_ui_outline_rect(
    sprites: &mut Vec<DecodedSprite>,
    texture_key: &str,
    rect: SpriteRect,
    thickness: f32,
    color: [u8; 4],
) {
    let thickness = thickness.ceil().max(1.0) as u32;
    let width = rect.width.ceil().max(1.0) as u32;
    let height = rect.height.ceil().max(1.0) as u32;
    let vertical = height.saturating_sub(thickness * 2);
    append_ui_solid_rect(
        sprites,
        &format!("{texture_key}:top"),
        rect.x,
        rect.y,
        width,
        thickness,
        color,
    );
    append_ui_solid_rect(
        sprites,
        &format!("{texture_key}:bottom"),
        rect.x,
        rect.y + height as f32 - thickness as f32,
        width,
        thickness,
        color,
    );
    append_ui_solid_rect(
        sprites,
        &format!("{texture_key}:left"),
        rect.x,
        rect.y + thickness as f32,
        thickness,
        vertical,
        color,
    );
    append_ui_solid_rect(
        sprites,
        &format!("{texture_key}:right"),
        rect.x + width as f32 - thickness as f32,
        rect.y + thickness as f32,
        thickness,
        vertical,
        color,
    );
}

fn load_character_creation_options(
    archive: &NxArchive,
    job: u16,
    female: bool,
) -> Result<Vec<Vec<i32>>, String> {
    let base = match job {
        0 => "MakeCharInfo.img/PremiumChar",
        1 => "MakeCharInfo.img/Info/Char",
        2 => "MakeCharInfo.img/OrientChar",
        _ => return Err(format!("unsupported character-creation job {job}")),
    };
    let gender = if female { "Female" } else { "Male" };
    let root = archive
        .root()
        .resolve(&format!("{base}{gender}"))
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("character option node {base}{gender} is missing"))?;
    let mut pools = vec![Vec::new(); 8];
    for category in root.children().map_err(|error| error.to_string())? {
        let category_id = category
            .name()
            .map_err(|error| error.to_string())?
            .parse::<usize>()
            .map_err(|_| "character option category is not numeric".to_owned())?;
        if category_id >= pools.len() {
            continue;
        }
        for value in category.children().map_err(|error| error.to_string())? {
            if let NodeValue::Integer(value) = value.value().map_err(|error| error.to_string())? {
                if let Ok(value) = i32::try_from(value) {
                    pools[category_id].push(value);
                }
            }
        }
    }
    if pools.iter().any(Vec::is_empty) {
        return Err("one or more character appearance pools are empty".to_owned());
    }
    Ok(pools)
}

fn creation_name_is_forbidden(archive: &NxArchive, name: &str) -> bool {
    let Ok(Some(root)) = archive.root().resolve("ForbiddenName.img") else {
        return false;
    };
    let folded_name = name.to_lowercase();
    root.children().ok().into_iter().flatten().any(|entry| {
        let value = match entry.value() {
            Ok(NodeValue::String(value)) => value,
            _ => entry.name().unwrap_or_default(),
        };
        !value.is_empty() && folded_name.contains(&value.to_lowercase())
    })
}
