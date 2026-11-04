//! Offline UI screenshot and geometry preview support.
//!
//! This module deliberately calls the same scene builders and decoded sprites as
//! the live client. It does not create a window or connect to a game server.

use super::*;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

include!("ui_preview_artifacts.rs");

const CANVAS_WIDTH: u32 = super::ui_foundation::LOGICAL_WIDTH as u32;
const CANVAS_HEIGHT: u32 = super::ui_foundation::LOGICAL_HEIGHT as u32;
const PREVIEW_SIZES: [(u32, u32); 2] = [(1024, 768), (1440, 900)];

struct KeyboardPreviewFixture {
    bindings: Vec<openstory_protocol::KeyBinding>,
    actions: Vec<KeyConfigAction>,
    capture: Option<usize>,
    quickslot_capture: Option<usize>,
    first_row: usize,
}

fn keyboard_preview_fixture(name: &str) -> KeyboardPreviewFixture {
    let reference = keyboard_settings::reference_bindings();
    let empty = name.ends_with("-empty") || name.ends_with("-scroll");
    let bindings = if empty {
        vec![openstory_protocol::KeyBinding::default(); 90]
    } else {
        reference
    };
    let mut actions = static_key_config_actions()
        .into_iter()
        .filter(|action| {
            empty
                || bindings
                    .iter()
                    .any(|binding| binding.kind == action.kind && binding.action == action.action)
                || (action.kind == 5 && matches!(action.action, 51 | 52))
        })
        .collect::<Vec<_>>();
    if !empty {
        actions.push(KeyConfigAction {
            label: "Messenger".to_owned(),
            kind: 4,
            action: 6,
            assignable: true,
        });
    }
    KeyboardPreviewFixture {
        bindings,
        actions,
        capture: name.ends_with("-capture").then_some(0),
        quickslot_capture: name.ends_with("-quickslots").then_some(2),
        first_row: if name.ends_with("-scroll") { 2 } else { 0 },
    }
}

fn keyboard_preview_regions(fixture: &KeyboardPreviewFixture) -> Vec<ui_foundation::UiRect> {
    let mut regions = keyboard_settings::buttons()
        .into_iter()
        .map(|(_, rect, _)| rect)
        .collect::<Vec<_>>();
    regions.extend(
        fixture
            .bindings
            .iter()
            .enumerate()
            .filter(|(_, binding)| binding.kind != 0)
            .filter_map(|(key, _)| keyboard_settings::key_rect(key as u8)),
    );
    if fixture.quickslot_capture.is_some() {
        regions.extend((0..8).map(keyboard_settings::quickslot_rect));
    } else {
        regions.extend(
            keyboard_settings::tray_entries(&fixture.actions, &fixture.bindings)
                .into_iter()
                .filter(|(_, cell)| {
                    *cell >= fixture.first_row * 18 && *cell < (fixture.first_row + 2) * 18
                })
                .map(|(_, cell)| keyboard_settings::TRAY.cell(cell - fixture.first_row * 18)),
        );
    }
    regions
}

pub(super) fn run(
    screen: &str,
    output_dir: &Path,
    include_hitboxes: bool,
    map: &NxArchive,
    legacy_ui: &NxArchive,
    ui: &NxArchive,
    characters: &NxArchive,
    npcs: &NxArchive,
    skills: &NxArchive,
    etc: &NxArchive,
    items: &NxArchive,
    mobs: &NxArchive,
    quests: &NxArchive,
    strings: &NxArchive,
    font: &fontdue::Font,
) -> Result<(), String> {
    fs::create_dir_all(output_dir)
        .map_err(|error| format!("could not create {}: {error}", output_dir.display()))?;

    if screen == "all" || screen.starts_with("panel-keyboard-settings") {
        write_keyboard_art_catalog(ui, skills, items, characters, font, output_dir)?;
    }
    write_asset_catalog(ui, output_dir)?;
    write_typography_preview(font, output_dir)?;

    let screens: &[&str] = if screen == "all" {
        super::ui_catalog::CASES
    } else if super::ui_catalog::CASES.contains(&screen) {
        std::slice::from_ref(&screen)
    } else {
        return Err(format!(
            "unknown UI preview {screen:?}; use --help to list preview names"
        ));
    };

    for name in screens {
        if name.starts_with("page-cash-shop") {
            let variant = match *name {
                "page-cash-shop" => "catalog",
                "page-cash-shop-locker" => "locker",
                "page-cash-shop-coupon" => "coupon",
                "page-cash-shop-confirmation" => "confirmation",
                _ => unreachable!("cash shop screen name was validated above"),
            };
            let scene = super::cash_shop_ui::render_preview(
                variant, ui, font, etc, items, characters, strings,
            )?;
            let base = scene.pixels;
            write_reference_artifacts(output_dir, name, &base, None)?;
            let annotated = include_hitboxes.then(|| {
                let mut image = base.clone();
                for (index, region) in scene.hit_regions.iter().enumerate() {
                    draw_rect_outline(
                        &mut image,
                        CANVAS_WIDTH,
                        CANVAS_HEIGHT,
                        SpriteRect {
                            x: region.bounds.x,
                            y: region.bounds.y,
                            width: region.bounds.width,
                            height: region.bounds.height,
                            blend_mode: BlendMode::Alpha,
                        },
                        hitbox_color(index),
                    );
                }
                image
            });
            let report = format!(
                "Screen: {name}\nCanvas: {CANVAS_WIDTH}x{CANVAS_HEIGHT}\nRenderer: live Cash Shop {variant} renderer\nInteractive regions: {}\n",
                scene.hit_regions.len()
            );
            let preview_name = preview_stem(name);
            let report_path = output_dir.join(format!("{preview_name}-layout.txt"));
            fs::write(&report_path, report.as_bytes())
                .map_err(|error| format!("could not write {}: {error}", report_path.display()))?;
            print!("{report}");

            for (width, height) in PREVIEW_SIZES {
                let resized = scale_to_viewport(&base, width, height);
                let path = preview_path(output_dir, name, width, height);
                write_png(&path, width, height, &resized)
                    .map_err(|error| format!("could not write {}: {error}", path.display()))?;
            }
            if let Some(annotated) = annotated.as_ref() {
                let path = output_dir.join(format!("{preview_name}-hitboxes-1024x768.png"));
                write_png(&path, CANVAS_WIDTH, CANVAS_HEIGHT, annotated)
                    .map_err(|error| format!("could not write {}: {error}", path.display()))?;
            }
            continue;
        }
        let detail_animation_mode = match *name {
            "panel-monster-book-detail" => Some(0),
            "panel-monster-book-detail-move" => Some(1),
            "panel-monster-book-detail-die" => Some(2),
            _ => None,
        };
        let scene = if let Some(mode) = detail_animation_mode {
            build_monster_book_detail_scene(map, ui, items, characters, mobs, strings, font, mode)?
        } else if *name == "panel-monster-book" {
            build_monster_book_detail_scene(
                map,
                ui,
                items,
                characters,
                mobs,
                strings,
                font,
                usize::MAX,
            )?
        } else if *name == "panel-item-inventory-tabs" {
            build_inventory_tabs_scene(map, legacy_ui, characters, items, strings, font)?
        } else if name.starts_with("panel-item-inventory") {
            build_inventory_panel_scene(name, map, legacy_ui, characters, items, strings, font)?
        } else if *name == "panel-equipment-inventory" {
            build_equipment_panel_scene(map, legacy_ui, characters, items, font)?
        } else if *name == "panel-trade-room" {
            build_trade_panel_scene(map, legacy_ui, characters, font)?
        } else if *name == "panel-npc-storage" {
            build_storage_panel_scene(map, legacy_ui, npcs, characters, items, strings, font)?
        } else if name.starts_with("panel-npc-store") {
            build_npc_store_panel_scene(name, map, legacy_ui, npcs, characters, items, strings, font)?
        } else if *name == "panel-item-tooltip" {
            build_item_tooltip_scene(map, characters, items, strings, ui, font)?
        } else if name.starts_with("panel-pet-equipment") {
            build_pet_equipment_scene(name, map, legacy_ui, characters, items, strings, font)?
        } else if matches!(*name, "panel-quests" | "panel-quests-detail") {
            build_quest_log_panel_scene(
                name, map, legacy_ui, quests, strings, characters, npcs, font,
            )?
        } else if matches!(
            *name,
            "panel-npc-chat"
                | "panel-npc-chat-tutorial"
                | "panel-npc-chat-menu"
                | "panel-npc-chat-option"
                | "panel-npc-chat-quiz"
                | "panel-npc-chat-feedback"
                | "panel-npc-chat-navigation"
                | "panel-npc-chat-text-input"
                | "panel-npc-chat-number-input"
                | "panel-npc-chat-unimplemented"
        ) {
            let mut scene = build_scene(
                "page-first-map",
                map,
                legacy_ui,
                ui,
                characters,
                items,
                skills,
                etc,
                strings,
                font,
            )?;
            let (pixels, (x, y, width, height)) = if *name != "panel-npc-chat" {
                let (npc_id, message_type, navigation, raw_text) = match *name {
                    "panel-npc-chat-unimplemented" => (2004, 0, None,
                        super::client_npc_interaction::UNIMPLEMENTED_NPC_TEXT.to_owned()),
                    "panel-npc-chat-menu" => {
                        let choices = vec![NpcQuestChoice { quest_id: 1038, side: 1, ready: true },
                            NpcQuestChoice { quest_id: 1040, side: 0, ready: true }];
                        let (text, _) = super::client_npc_interaction::npc_quest_menu(quests, 1, choices);
                        (12000, 4, None, text)
                    }
                    "panel-npc-chat-option" => (fixture_named_id(strings, "Npc.img", "Shammos", 2007), 4, None,
                        "Hehe...I need to gather up the strength right now!\\n\\n#r! QUEST AVAILABLE#k\\n\\n#L0#Shammo's Request#l".to_owned()),
                    "panel-npc-chat-quiz" => (2003, 4, None,
                        quest_dialogue_pages(quests, "Say.img/1036/1")[2].clone()),
                    "panel-npc-chat-feedback" => (12101, 0, None,
                        nx_string_value(quests, "Say.img/1009/1/stop/0/1").unwrap()),
                    "panel-npc-chat-text-input" => (2004, 2, None,
                        "What would you like to say?".to_owned()),
                    "panel-npc-chat-number-input" => (2004, 3, None,
                        "How many would you like?".to_owned()),
                    "panel-npc-chat-navigation" => (22000, 0, Some((true, true)),
                        "Take this ship to Victoria Island. Once you leave Maple Island, you cannot return.".to_owned()),
                    _ => (2004, 12, None,
                        quest_dialogue_pages(quests, "Say.img/1035/0").last().cloned()
                            .ok_or("missing Todd tutorial dialogue")?),
                };
                let (text, icons) = format_npc_dialogue_text(
                    &raw_text,
                    "Mapler",
                    strings,
                    &mut HashMap::new(),
                    items,
                    characters,
                    skills,
                    ui,
                    &mut HashMap::new(),
                );
                let (plain, selections) = if message_type == 4 {
                    parse_npc_selections(&text)
                } else {
                    (text, Vec::new())
                };
                let (pixels, panel, _, _) = render_npc_dialogue(
                    font,
                    legacy_ui,
                    npcs,
                    strings,
                    npc_id,
                    0,
                    message_type,
                    navigation,
                    &plain,
                    0,
                    &icons,
                    &selections,
                    0,
                    0,
                    None,
                    match *name {
                        "panel-npc-chat-text-input" => "Hello Maple World",
                        "panel-npc-chat-number-input" => "25",
                        _ => "",
                    },
                );
                (pixels, (panel.x, panel.y, panel.width, panel.height))
            } else {
                super::render_npc_dialogue_preview(font, legacy_ui, npcs, strings)
            };
            scene.sprites.push(DecodedSprite {
                texture_key: "native-npc-dialogue-preview".to_owned(),
                rect: SpriteRect {
                    x,
                    y,
                    width,
                    height,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: width.round() as u32,
                texture_height: height.round() as u32,
                pixels: Arc::from(pixels),
            });
            scene
        } else if *name == "panel-revive-character" {
            let mut scene = build_scene(
                "page-first-map",
                map,
                legacy_ui,
                ui,
                characters,
                items,
                skills,
                etc,
                strings,
                font,
            )?;
            let pixels = super::render_death_notice(font, legacy_ui, false);
            scene.sprites.push(DecodedSprite {
                texture_key: "native-revive-preview".to_owned(),
                rect: SpriteRect {
                    x: 0.0,
                    y: 0.0,
                    width: CANVAS_WIDTH as f32,
                    height: CANVAS_HEIGHT as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: CANVAS_WIDTH,
                texture_height: CANVAS_HEIGHT,
                pixels: Arc::from(pixels),
            });
            scene
        } else if *name == "hud-clock" {
            let panel = super::ui_clock::render(
                ui,
                super::ui_clock::ClockState::Countdown {
                    remaining_seconds: 83 * 60 + 7,
                    elapsed_ms: 0,
                },
            )?;
            let mut sprites = gameplay_backdrop(map, characters, font)?;
            sprites.push(DecodedSprite {
                texture_key: "native-clock-preview".to_owned(),
                rect: SpriteRect {
                    x: panel.x,
                    y: panel.y,
                    width: panel.width as f32,
                    height: panel.height as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: panel.width,
                texture_height: panel.height,
                pixels: Arc::from(panel.pixels),
            });
            LoginSceneAssets {
                sprites,
                actions: Vec::new(),
                map_scene: None,
                animated_art: Vec::new(),
                login_buttons: Vec::new(),
                save_id_indicator_draws: None,
            }
        } else if *name == "panel-wedding" {
            let panel = build_wedding_panel(ui, font, 83)?;
            let mut sprites = gameplay_backdrop(map, characters, font)?;
            sprites.push(DecodedSprite {
                texture_key: "native-wedding-preview".to_owned(),
                rect: SpriteRect {
                    x: (CANVAS_WIDTH - WEDDING_INVITATION_WIDTH as u32) as f32 * 0.5,
                    y: (CANVAS_HEIGHT - WEDDING_PANEL_HEIGHT as u32) as f32 * 0.5,
                    width: WEDDING_PANEL_WIDTH as f32,
                    height: WEDDING_PANEL_HEIGHT as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: WEDDING_PANEL_WIDTH as u32,
                texture_height: WEDDING_PANEL_HEIGHT as u32,
                pixels: Arc::from(panel.pixels),
            });
            LoginSceneAssets {
                sprites,
                actions: Vec::new(),
                map_scene: None,
                animated_art: Vec::new(),
                login_buttons: Vec::new(),
                save_id_indicator_draws: None,
            }
        } else if *name == "panel-rps" {
            let mut game = RpsGameRuntime::new();
            game.phase = RpsGamePhase::Result;
            game.player_choice = Some(0);
            game.opponent_choice = Some(2);
            game.result = Some(1);
            game.wins = 1;
            game.hovered = Some(RpsGameButton::Retry);
            let pixels = render_rps_game_panel(ui, font, &game)?;
            let mut sprites = gameplay_backdrop(map, characters, font)?;
            sprites.push(DecodedSprite {
                texture_key: "native-rps-preview".to_owned(),
                rect: SpriteRect {
                    x: 0.0,
                    y: 0.0,
                    width: CANVAS_WIDTH as f32,
                    height: CANVAS_HEIGHT as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: CANVAS_WIDTH,
                texture_height: CANVAS_HEIGHT,
                pixels: Arc::from(pixels),
            });
            LoginSceneAssets {
                sprites,
                actions: Vec::new(),
                map_scene: None,
                animated_art: Vec::new(),
                login_buttons: Vec::new(),
                save_id_indicator_draws: None,
            }
        } else if *name == "panel-event-list" {
            let events = vec![
                EventListEntry {
                    event: openstory_protocol::ServerEventInfo {
                        type_id: 0,
                        name: "Double EXP Weekend".to_owned(),
                        description: "Earn bonus experience while exploring Maple World."
                            .to_owned(),
                        seconds_remaining: 5_840,
                        multiplier: 200,
                        has_item_rewards: false,
                        rewards: Vec::new(),
                    },
                    peak_duration: 14_400,
                },
                EventListEntry {
                    event: openstory_protocol::ServerEventInfo {
                        type_id: 1,
                        name: "Mushroom Festival".to_owned(),
                        description: "Collect festival tokens and trade them for prizes."
                            .to_owned(),
                        seconds_remaining: 3_000,
                        multiplier: 100,
                        has_item_rewards: true,
                        rewards: vec![
                            openstory_protocol::EventReward {
                                item_id: 2_000_000,
                                quantity: 5,
                            },
                            openstory_protocol::EventReward {
                                item_id: 2_040_000,
                                quantity: 2,
                            },
                        ],
                    },
                    peak_duration: 7_200,
                },
                EventListEntry {
                    event: openstory_protocol::ServerEventInfo {
                        type_id: 2,
                        name: "Maple Story Anniversary".to_owned(),
                        description: "A celebration is coming soon to every town.".to_owned(),
                        seconds_remaining: -1,
                        multiplier: 100,
                        has_item_rewards: false,
                        rewards: Vec::new(),
                    },
                    peak_duration: 0,
                },
            ];
            let panel = build_event_list_panel(
                legacy_ui,
                font,
                strings,
                &mut HashMap::new(),
                &events,
                0,
                Some(0),
                true,
            )?;
            let mut sprites = gameplay_backdrop(map, characters, font)?;
            sprites.push(DecodedSprite {
                texture_key: "native-event-list-preview".to_owned(),
                rect: SpriteRect {
                    x: EVENT_LIST_PANEL_X,
                    y: EVENT_LIST_PANEL_Y,
                    width: EVENT_LIST_PANEL_WIDTH as f32,
                    height: EVENT_LIST_PANEL_HEIGHT as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: EVENT_LIST_PANEL_WIDTH as u32,
                texture_height: EVENT_LIST_PANEL_HEIGHT as u32,
                pixels: Arc::from(panel.pixels),
            });
            LoginSceneAssets {
                sprites,
                actions: Vec::new(),
                map_scene: None,
                animated_art: Vec::new(),
                login_buttons: Vec::new(),
                save_id_indicator_draws: None,
            }
        } else if *name == "panel-dragon" {
            let mut dragon_sprites = Vec::new();
            let frames = append_dragon_animation(
                &mut dragon_sprites,
                mobs,
                2215,
                &mut MapArtCache::default(),
            )?;
            let range = frames
                .first()
                .map(|frame| frame.sprites.clone())
                .ok_or_else(|| "v83 Evan dragon has no drawable stand frame".to_owned())?;
            let mut sprites = gameplay_backdrop(map, characters, font)?;
            sprites.extend(
                dragon_sprites
                    .into_iter()
                    .skip(range.start)
                    .take(range.len()),
            );
            LoginSceneAssets {
                sprites,
                actions: Vec::new(),
                map_scene: None,
                animated_art: Vec::new(),
                login_buttons: Vec::new(),
                save_id_indicator_draws: None,
            }
        } else if matches!(
            *name,
            "panel-monster-carnival" | "panel-monster-carnival-side"
        ) {
            let mut state = super::monster_carnival::State::default();
            state.side_view = *name == "panel-monster-carnival-side";
            state.team = Some(0);
            state.my_cp = 32;
            state.my_total_cp = 50;
            state.enemy_cp = 27;
            state.enemy_total_cp = 50;
            state.summon_counts = [2, 1, 3, 0];
            let panel = super::monster_carnival::render_state(legacy_ui, font, &state)?;
            let mut sprites = gameplay_backdrop(map, characters, font)?;
            sprites.push(DecodedSprite {
                texture_key: "native-monster-carnival-preview".to_owned(),
                rect: SpriteRect {
                    x: state.x,
                    y: state.y,
                    width: panel.width as f32,
                    height: panel.height as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: panel.width,
                texture_height: panel.height,
                pixels: Arc::from(panel.pixels),
            });
            LoginSceneAssets {
                sprites,
                actions: Vec::new(),
                map_scene: None,
                animated_art: Vec::new(),
                login_buttons: Vec::new(),
                save_id_indicator_draws: None,
            }
        } else if *name == "hud-avatar-megaphone" {
            let mut sprites = gameplay_backdrop(map, characters, font)?;
            let x = CANVAS_WIDTH as f32 - super::avatar_megaphone::PANEL_WIDTH as f32 + 4.0;
            let y = 4.0;
            super::append_sprite(
                &mut sprites,
                map,
                "MapHelper.img/AvatarMegaphone/Burning/0",
                x,
                y,
                1.0,
            )?;
            let lines = vec![
                "Greetings from MapleStory!".to_owned(),
                "The first three rows appear".to_owned(),
                "in this v83 banner.".to_owned(),
                "This fourth line is dropped.".to_owned(),
            ];
            let text_pixels =
                super::avatar_megaphone::render_text(map, font, "PreviewHero", &lines)?;
            sprites.push(DecodedSprite {
                texture_key: "native-avatar-megaphone-preview:text".to_owned(),
                rect: SpriteRect {
                    x,
                    y,
                    width: super::avatar_megaphone::PANEL_WIDTH as f32,
                    height: super::avatar_megaphone::PANEL_HEIGHT as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: super::avatar_megaphone::PANEL_WIDTH,
                texture_height: super::avatar_megaphone::PANEL_HEIGHT,
                pixels: Arc::from(text_pixels),
            });
            append_character_preview(
                &mut sprites,
                characters,
                &preview_character().look,
                x + 70.0,
                y + 92.0,
                1.0,
                false,
                &mut MapArtCache::default(),
            )?;
            LoginSceneAssets {
                sprites,
                actions: Vec::new(),
                map_scene: None,
                animated_art: Vec::new(),
                login_buttons: Vec::new(),
                save_id_indicator_draws: None,
            }
        } else if *name == "panel-quit-confirmation" {
            let mut sprites = gameplay_backdrop(map, characters, font)?;
            let dim_pixels: Arc<[u8]> = Arc::from([0_u8, 0, 0, 140]);
            sprites.push(DecodedSprite {
                texture_key: "native-quit-confirmation-preview:dim".to_owned(),
                rect: SpriteRect {
                    x: 0.0,
                    y: 0.0,
                    width: CANVAS_WIDTH as f32,
                    height: CANVAS_HEIGHT as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: 1,
                texture_height: 1,
                pixels: dim_pixels,
            });
            let (dialog, _) = build_quit_confirmation_sprites(legacy_ui)
                .or_else(|_| build_quit_confirmation_sprites(ui))?;
            sprites.extend(dialog);
            LoginSceneAssets {
                sprites,
                actions: Vec::new(),
                map_scene: None,
                animated_art: Vec::new(),
                login_buttons: Vec::new(),
                save_id_indicator_draws: None,
            }
        } else if *name == "panel-server-notice" || *name == "panel-tos-acceptance" {
            let mut sprites = gameplay_backdrop(map, characters, font)?;
            let dim_pixels: Arc<[u8]> = Arc::from([0_u8, 0, 0, 140]);
            sprites.push(DecodedSprite {
                texture_key: "native-server-notice-preview:dim".to_owned(),
                rect: SpriteRect {
                    x: 0.0,
                    y: 0.0,
                    width: CANVAS_WIDTH as f32,
                    height: CANVAS_HEIGHT as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: 1,
                texture_height: 1,
                pixels: dim_pixels,
            });
            let message = if *name == "panel-tos-acceptance" {
                "Server Terms of Service required. Review them first. Click OK or Enter to accept; Escape cancels."
            } else {
                "The server will restart in five minutes. Please move to a safe place."
            };
            let (width, height, pixels, panel, _) =
                build_server_notice_art(legacy_ui, font, message)
                    .or_else(|_| build_server_notice_art(ui, font, message))?;
            sprites.push(DecodedSprite {
                texture_key: "native-server-notice-preview:dialog".to_owned(),
                rect: panel,
                depth_order: 0,
                texture_width: width,
                texture_height: height,
                pixels: Arc::from(pixels),
            });
            LoginSceneAssets {
                sprites,
                actions: Vec::new(),
                map_scene: None,
                animated_art: Vec::new(),
                login_buttons: Vec::new(),
                save_id_indicator_draws: None,
            }
        } else if name.starts_with("hud-minimap-") {
            build_minimap_scene(name, map, ui, strings, font)?
        } else {
            build_scene(
                name, map, legacy_ui, ui, characters, items, skills, etc, strings, font,
            )?
        };
        let base = compose_scene(&scene.sprites, CANVAS_WIDTH, CANVAS_HEIGHT);
        write_reference_artifacts(output_dir, name, &base, Some(&scene))?;
        let annotated = include_hitboxes.then(|| {
            let mut image = base.clone();
            for (index, action) in scene.actions.iter().enumerate() {
                draw_rect_outline(
                    &mut image,
                    CANVAS_WIDTH,
                    CANVAS_HEIGHT,
                    action.rect,
                    hitbox_color(index),
                );
            }
            for sprite in &scene.sprites {
                if sprite.texture_key.starts_with("nx-window:") {
                    draw_rect_outline(
                        &mut image,
                        CANVAS_WIDTH,
                        CANVAS_HEIGHT,
                        sprite.rect,
                        [240, 240, 240, 220],
                    );
                }
            }
            if name.starts_with("panel-keyboard-settings") {
                let fixture = keyboard_preview_fixture(name);
                for (index, rect) in keyboard_preview_regions(&fixture).into_iter().enumerate() {
                    draw_rect_outline(
                        &mut image,
                        CANVAS_WIDTH,
                        CANVAS_HEIGHT,
                        SpriteRect {
                            x: KEY_CONFIG_PANEL_X + rect.x,
                            y: KEY_CONFIG_PANEL_Y + rect.y,
                            width: rect.width,
                            height: rect.height,
                            blend_mode: BlendMode::Alpha,
                        },
                        hitbox_color(index),
                    );
                }
            }
            if *name == "hud-quickslot" {
                for slot in 0..8 {
                    draw_rect_outline(
                        &mut image,
                        CANVAS_WIDTH,
                        CANVAS_HEIGHT,
                        SpriteRect {
                            x: 872.0 + (slot % 4) as f32 * 35.0,
                            y: 623.0 + (slot / 4) as f32 * 34.0,
                            width: 32.0,
                            height: 30.0,
                            blend_mode: BlendMode::Alpha,
                        },
                        hitbox_color(slot),
                    );
                }
            }
            if *name == "panel-item-inventory" {
                for tab in 0..5 {
                    draw_rect_outline(
                        &mut image,
                        CANVAS_WIDTH,
                        CANVAS_HEIGHT,
                        SpriteRect {
                            x: super::INVENTORY_PANEL_X + ui_foundation::inventory_tab(tab).x,
                            y: super::INVENTORY_PANEL_Y + ui_foundation::inventory_tab(tab).y,
                            width: ui_foundation::inventory_tab(tab).width,
                            height: ui_foundation::inventory_tab(tab).height,
                            blend_mode: BlendMode::Alpha,
                        },
                        hitbox_color(tab),
                    );
                }
                for row in 0..6 {
                    for column in 0..4 {
                        let index = 5 + row * 4 + column;
                        draw_rect_outline(
                            &mut image,
                            CANVAS_WIDTH,
                            CANVAS_HEIGHT,
                            SpriteRect {
                                x: super::INVENTORY_PANEL_X
                                    + super::INVENTORY_SLOT_X as f32
                                    + column as f32 * super::INVENTORY_SLOT_STEP_X as f32,
                                y: super::INVENTORY_PANEL_Y
                                    + super::INVENTORY_SLOT_Y as f32
                                    + row as f32 * super::INVENTORY_SLOT_STEP_Y as f32,
                                width: 34.0,
                                height: 33.0,
                                blend_mode: BlendMode::Alpha,
                            },
                            hitbox_color(index),
                        );
                    }
                }
            }
            if matches!(*name, "panel-quests" | "panel-quests-detail") {
                for (index, (left, width)) in [(10.0, 66.0), (78.0, 70.0), (150.0, 70.0)]
                    .into_iter()
                    .enumerate()
                {
                    draw_rect_outline(
                        &mut image,
                        CANVAS_WIDTH,
                        CANVAS_HEIGHT,
                        SpriteRect {
                            x: super::QUEST_LOG_PANEL_X + left,
                            y: super::QUEST_LOG_PANEL_Y + 21.0,
                            width,
                            height: 27.0,
                            blend_mode: BlendMode::Alpha,
                        },
                        hitbox_color(index),
                    );
                }
                for row in 0..super::QUEST_LOG_VISIBLE_ROWS {
                    draw_rect_outline(
                        &mut image,
                        CANVAS_WIDTH,
                        CANVAS_HEIGHT,
                        SpriteRect {
                            x: super::QUEST_LOG_PANEL_X + 8.0,
                            y: super::QUEST_LOG_PANEL_Y + ui_foundation::QUEST_GRID.cell(row).y,
                            width: 236.0,
                            height: ui_foundation::QUEST_GRID.cell(row).height,
                            blend_mode: BlendMode::Alpha,
                        },
                        hitbox_color(3 + row),
                    );
                }
            }
            image
        });

        let mut report = geometry_report(name, &scene, CANVAS_WIDTH, CANVAS_HEIGHT);
        if name.starts_with("panel-keyboard-settings") {
            let fixture = keyboard_preview_fixture(name);
            report.push_str(&format!(
                "Keyboard controls: {} (shared live geometry)\nNative panel: {}x{}\n",
                keyboard_preview_regions(&fixture).len(),
                KEY_CONFIG_PANEL_WIDTH,
                KEY_CONFIG_PANEL_HEIGHT
            ));
        }
        let preview_name = preview_stem(name);
        let report_path = output_dir.join(format!("{preview_name}-layout.txt"));
        fs::write(&report_path, report.as_bytes())
            .map_err(|error| format!("could not write {}: {error}", report_path.display()))?;
        print!("{report}");

        for (width, height) in PREVIEW_SIZES {
            let resized = scale_to_viewport(&base, width, height);
            let path = preview_path(output_dir, name, width, height);
            write_png(&path, width, height, &resized)
                .map_err(|error| format!("could not write {}: {error}", path.display()))?;
        }
        if let Some(annotated) = annotated.as_ref() {
            let path = output_dir.join(format!("{preview_name}-hitboxes-1024x768.png"));
            write_png(&path, CANVAS_WIDTH, CANVAS_HEIGHT, annotated)
                .map_err(|error| format!("could not write {}: {error}", path.display()))?;
        }
    }

    println!("UI previews saved in {}", output_dir.display());
    Ok(())
}

fn build_scene(
    name: &str,
    map: &NxArchive,
    legacy_ui: &NxArchive,
    ui: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    skill_archive: &NxArchive,
    etc: &NxArchive,
    strings: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    match name {
        "panel-system-options" => {
            let mut sprites = gameplay_backdrop(map, character_archive, font)?;
            let pixels = render_options_panel(
                font,
                ui,
                AudioSettings {
                    music_volume: 50,
                    effects_volume: 70,
                    mute_music: false,
                    mute_effects: false,
                },
                DisplaySettings::default(),
                InputSettings { mouse_speed: 50 },
                GameplaySettings::default(),
                SocialSettings::default(),
                OptionControl::Music,
                None,
            );
            sprites.push(DecodedSprite {
                texture_key: "native-options-preview:panel".to_owned(),
                rect: SpriteRect {
                    x: SETTINGS_PANEL_X,
                    y: SETTINGS_PANEL_Y,
                    width: SETTINGS_PANEL_WIDTH as f32,
                    height: SETTINGS_PANEL_HEIGHT as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: SETTINGS_PANEL_WIDTH,
                texture_height: SETTINGS_PANEL_HEIGHT,
                pixels: Arc::from(pixels),
            });
            Ok(LoginSceneAssets {
                sprites,
                actions: Vec::new(),
                map_scene: None,
                animated_art: Vec::new(),
                login_buttons: Vec::new(),
                save_id_indicator_draws: None,
            })
        }
        "hud-bottom-bar" | "page-first-map" | "page-second-map" => {
            let map_id = match name {
                "page-first-map" => Some(10_000),
                "page-second-map" => Some(20_000),
                _ => None,
            };
            let mut sprites = if let Some(map_id) = map_id {
                gameplay_backdrop_for_map(map, character_archive, font, map_id)?
            } else {
                gameplay_backdrop(map, character_archive, font)?
            };
            let (bar_sprites, gauge_assets, layout) = build_status_bar_sprites(legacy_ui)?;
            let status_bar_preview = name == "hud-bottom-bar";
            let stats = openstory_protocol::CharacterStats {
                name: if status_bar_preview {
                    "mmochi"
                } else {
                    "Bobby"
                }
                .to_owned(),
                level: if status_bar_preview { 89 } else { 12 },
                job: if status_bar_preview { 200 } else { 0 },
                hp: if status_bar_preview { 920 } else { 50 },
                max_hp: if status_bar_preview { 1_109 } else { 50 },
                mp: if status_bar_preview { 5_405 } else { 50 },
                max_mp: if status_bar_preview { 5_772 } else { 78 },
                experience: if status_bar_preview { 76_083 } else { 82 },
                ..Default::default()
            };
            let fractions = if status_bar_preview {
                [
                    stats.hp as f32 / stats.max_hp.max(1) as f32,
                    stats.mp as f32 / stats.max_mp.max(1) as f32,
                    0.0133,
                ]
            } else {
                [1.0_f32, 50.0 / 78.0, 0.18]
            };
            let gauge_rects = gauge_assets.map(|gauge| bar_sprites[gauge.fill_index].rect);
            for (index, mut sprite) in bar_sprites.into_iter().enumerate() {
                if let Some(gauge) = gauge_assets
                    .iter()
                    .position(|asset| asset.fill_index == index)
                {
                    let fill_fraction = fractions[gauge];
                    if fill_fraction <= 0.0 {
                        continue;
                    }
                    if layout == StatusBarLayout::V83 {
                        let source_width = sprite.texture_width as usize;
                        let fill_width = ((source_width as f32 * fill_fraction).round() as usize)
                            .clamp(1, source_width);
                        let mut cropped =
                            Vec::with_capacity(fill_width * sprite.texture_height as usize * 4);
                        for row in 0..sprite.texture_height as usize {
                            let start = row * source_width * 4;
                            cropped
                                .extend_from_slice(&sprite.pixels[start..start + fill_width * 4]);
                        }
                        sprite.texture_width = fill_width as u32;
                        sprite.pixels = Arc::from(cropped);
                        sprite.rect.width = fill_width as f32;
                    } else {
                        sprite.rect.width *= fill_fraction;
                    }
                } else if let Some(gauge) = gauge_assets
                    .iter()
                    .position(|asset| asset.start_index == Some(index))
                {
                    if fractions[gauge] <= 0.0 {
                        continue;
                    }
                } else if let Some(gauge) = gauge_assets
                    .iter()
                    .position(|asset| asset.end_index == Some(index))
                {
                    let fill_fraction = fractions[gauge];
                    if fill_fraction <= 0.0 {
                        continue;
                    }
                    let rect = gauge_rects[gauge];
                    sprite.rect.x = rect.x + rect.width * fill_fraction;
                }
                sprites.push(sprite);
            }
            let number_path = match layout {
                StatusBarLayout::Modern => "StatusBar2.img/mainBar/gauge/number",
                StatusBarLayout::V83 => "StatusBar.img/number",
            };
            let number_archive = match layout {
                StatusBarLayout::Modern => ui,
                StatusBarLayout::V83 => legacy_ui,
            };
            let number_font = load_status_bar_number_font(number_archive, number_path)?;
            let level_font = if layout == StatusBarLayout::V83 {
                Some(load_v83_status_bar_level_font(legacy_ui)?)
            } else {
                None
            };
            let hud_pixels = render_map_hud(
                font,
                Some(&number_font),
                level_font.as_ref(),
                layout,
                &stats,
                &CharacterBuffs::default(),
                Some(("[Welcome] Welcome to MapleStory!", [255, 220, 44])),
                status_bar_preview.then_some(0.0133),
            );
            sprites.push(DecodedSprite {
                texture_key: "native-status-bar-preview:hud-text".to_owned(),
                rect: SpriteRect {
                    x: 0.0,
                    y: layout.top(),
                    width: 1024.0,
                    height: layout.height() as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: 1024,
                texture_height: layout.height(),
                pixels: Arc::from(hud_pixels),
            });
            Ok(LoginSceneAssets {
                sprites,
                actions: Vec::new(),
                map_scene: None,
                animated_art: Vec::new(),
                login_buttons: Vec::new(),
                save_id_indicator_draws: None,
            })
        }
        "hud-item-megaphone" => {
            let mut sprites = gameplay_backdrop(map, character_archive, font)?;
            let panel = item_megaphone::render(
                legacy_ui,
                font,
                "Maple Leaf",
                None,
                true,
                Some("Red Potion"),
                None,
                true,
            )?;
            sprites.push(DecodedSprite {
                texture_key: "native-item-megaphone-preview".to_owned(),
                rect: SpriteRect {
                    x: ITEM_MEGAPHONE_X,
                    y: ITEM_MEGAPHONE_Y,
                    width: panel.width as f32,
                    height: panel.height as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: panel.width,
                texture_height: panel.height,
                pixels: panel.pixels.into(),
            });
            Ok(LoginSceneAssets {
                sprites,
                actions: Vec::new(),
                map_scene: None,
                animated_art: Vec::new(),
                login_buttons: Vec::new(),
                save_id_indicator_draws: None,
            })
        }
        "page-login" | "page-login-active" => {
            let mut scene = load_login_scene(map, legacy_ui)?;
            scene.sprites.push(field_sprite(
                font,
                "sample",
                FIELD_X,
                ACCOUNT_Y,
                "preview:account",
            ));
            scene.sprites.push(field_sprite(
                font,
                "******",
                FIELD_X,
                PASSWORD_Y,
                "preview:password",
            ));
            if name == "page-login-active" {
                let mut button_draws = HashSet::new();
                let mut active_draws = HashSet::new();
                for (index, button) in scene.login_buttons.iter().enumerate() {
                    button_draws.insert(button.normal_draw);
                    for draw in [
                        button.mouse_over_draw,
                        button.pressed_draw,
                        button.disabled_draw,
                        button.key_focused_draw,
                    ]
                    .into_iter()
                    .flatten()
                    {
                        button_draws.insert(draw);
                    }
                    let active = if index <= 1 {
                        button.mouse_over_draw.unwrap_or(button.normal_draw)
                    } else if button.action.is_none() {
                        button.disabled_draw.unwrap_or(button.normal_draw)
                    } else {
                        button.normal_draw
                    };
                    active_draws.insert(active);
                }
                for (index, sprite) in scene.sprites.iter_mut().enumerate() {
                    if button_draws.contains(&index) && !active_draws.contains(&index) {
                        sprite.rect.width = 0.0;
                        sprite.rect.height = 0.0;
                    }
                }
            }
            Ok(scene)
        }
        "page-world-map"
        | "page-world-map-link"
        | "page-world-map-marker"
        | "page-world-map-search"
        | "page-world-map-result" => {
            let query = if matches!(name, "page-world-map-search" | "page-world-map-result") {
                "Orbis"
            } else {
                ""
            };
            let search_catalog = world_map::load_search_catalog(map, strings)?;
            let panel = world_map::render(
                map,
                ui,
                100_000_000,
                (name == "page-world-map-result").then_some("WorldMap020"),
                true,
                query,
                name == "page-world-map-search",
                Some(&search_catalog),
                0,
                (name == "page-world-map-link").then_some(0),
                (name == "page-world-map-marker").then_some(0),
                (name == "page-world-map-result").then_some(200_000_000),
                strings,
                font,
            )?;
            let sprite = DecodedSprite {
                texture_key: format!("world-map-preview:{name}"),
                rect: SpriteRect {
                    x: panel.x,
                    y: panel.y,
                    width: panel.width as f32,
                    height: panel.height as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: panel.width,
                texture_height: panel.height,
                pixels: Arc::from(panel.pixels),
            };
            Ok(LoginSceneAssets {
                sprites: vec![sprite],
                actions: Vec::new(),
                map_scene: None,
                animated_art: Vec::new(),
                login_buttons: Vec::new(),
                save_id_indicator_draws: None,
            })
        }
        "page-world-select" => {
            let worlds = vec![openstory_protocol::WorldInfo {
                id: 0,
                name: "Scania".to_owned(),
                flag: 0,
                message: String::new(),
                channels: (1..=6)
                    .map(|index| openstory_protocol::WorldChannel {
                        name: format!("Scania-{index}"),
                        load: 0,
                    })
                    .collect(),
            }];
            load_world_selection_scene(map, legacy_ui, &worlds, Some(0), 0)
        }
        "page-character-select-empty" | "page-character-select-populated" => {
            let characters = if name == "page-character-select-empty" {
                openstory_protocol::CharacterList {
                    channel_id: 0,
                    characters: Vec::new(),
                    pic_state: 0,
                    slots: 6,
                }
            } else {
                let look = fixture_look();
                // One selected character plus five open slots shows the native
                // parchment, selection glow, silhouettes, and page arrows.
                let characters = vec![openstory_protocol::CharacterInfo {
                    id: 1,
                    stats: openstory_protocol::CharacterStats {
                        name: "Ommlettes".to_owned(),
                        level: 12,
                        job: 400,
                        strength: 4,
                        dexterity: 25,
                        intelligence: 4,
                        luck: 47,
                        fame: 0,
                        rank: Some((1, 0)),
                        ..Default::default()
                    },
                    look,
                }];
                openstory_protocol::CharacterList {
                    channel_id: 0,
                    characters,
                    pic_state: 0,
                    slots: 6,
                }
            };
            load_character_selection_scene(
                map,
                legacy_ui,
                character_archive,
                &characters,
                0,
                0,
                None,
                font,
                None,
            )
        }
        "page-character-create-class" => load_character_class_scene(map, legacy_ui, font),
        "page-character-create-explorer" => {
            let pools = load_character_creation_options(etc, 1, false)?;
            let creation = CharacterCreationState::customize(1, pools);
            load_character_customization_scene(
                map,
                legacy_ui,
                character_archive,
                strings,
                &mut HashMap::new(),
                true,
                &creation,
                font,
            )
        }
        "page-character-create-error" => {
            let pools = load_character_creation_options(etc, 1, false)?;
            let mut creation = CharacterCreationState::customize(1, pools);
            creation.notice =
                Some("Character creation failed (9). Review choices and retry.".to_owned());
            load_character_customization_scene(
                map,
                legacy_ui,
                character_archive,
                strings,
                &mut HashMap::new(),
                true,
                &creation,
                font,
            )
        }
        "panel-keyboard-settings"
        | "panel-keyboard-settings-empty"
        | "panel-keyboard-settings-capture"
        | "panel-keyboard-settings-scroll"
        | "panel-keyboard-settings-quickslots"
        | "hud-quickslot"
        | "panel-skill-macros"
        | "panel-item-inventory"
        | "panel-equipment-inventory"
        | "panel-skill-inventory"
        | "panel-npc-store"
        | "panel-npc-store-sell"
        | "panel-npc-store-confirmation"
        | "page-cash-shop"
        | "panel-monster-book"
        | "panel-stats"
        | "panel-stats-detail"
        | "panel-quests"
        | "hud-chat"
        | "panel-whisper"
        | "panel-party"
        | "panel-buddy"
        | "panel-guild"
        | "panel-alliance"
        | "panel-audio-settings" => build_native_window_scene(
            name,
            map,
            legacy_ui,
            ui,
            character_archive,
            item_archive,
            skill_archive,
            font,
        ),
        "panel-character-info" => {
            build_character_info_scene(map, character_archive, legacy_ui, strings, font)
        }
        "panel-family" | "panel-family-tree" => build_family_scene(
            map,
            character_archive,
            legacy_ui,
            font,
            name == "panel-family-tree",
        ),
        _ => Err(format!("unsupported UI preview {name:?}")),
    }
}

fn build_family_scene(
    map_archive: &NxArchive,
    character_archive: &NxArchive,
    ui_archive: &NxArchive,
    font: &fontdue::Font,
    tree: bool,
) -> Result<LoginSceneAssets, String> {
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    let (pixels, width, height) = if tree {
        let entries = [
            (1, 0, "MapleElder"),
            (2, 1, "OakSenior"),
            (3, 2, "PreviewHero"),
            (4, 3, "Maya"),
            (5, 3, "Rin"),
            (6, 4, "Alex"),
            (7, 4, "Leaf"),
            (8, 5, "Milo"),
            (9, 5, "Ash"),
        ]
        .into_iter()
        .map(
            |(character_id, parent_id, name)| openstory_protocol::FamilyChartEntry {
                character_id,
                parent_id,
                name: name.to_owned(),
                job: 100,
                level: 30,
                online: character_id % 2 != 0,
                current_reputation: 125,
                total_reputation: 200,
                reputation_to_senior: 0,
                todays_reputation: 0,
                channel: 0,
                time_online: 0,
            },
        )
        .collect();
        let state = FamilyUiState {
            chart: Some(openstory_protocol::FamilyChart {
                viewed_character_id: 3,
                entries,
                member_counters: Vec::new(),
                entitlement_usage: Vec::new(),
                add_button: None,
            }),
            selected_member: Some(3),
            ..FamilyUiState::default()
        };
        (
            render_family_tree_panel(font, ui_archive, &state),
            FAMILY_TREE_PANEL_WIDTH,
            FAMILY_TREE_PANEL_HEIGHT,
        )
    } else {
        (
            render_family_panel(font, ui_archive, &FamilyUiState::default()),
            FAMILY_PANEL_WIDTH,
            FAMILY_PANEL_HEIGHT,
        )
    };
    sprites.push(DecodedSprite {
        texture_key: "family-preview".to_owned(),
        rect: SpriteRect {
            x: (1024.0 - width as f32) / 2.0,
            y: 165.0,
            width: width as f32,
            height: height as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: width,
        texture_height: height,
        pixels: Arc::from(pixels),
    });
    Ok(LoginSceneAssets {
        sprites,
        actions: Vec::new(),
        map_scene: None,
        animated_art: Vec::new(),
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn build_character_info_scene(
    map_archive: &NxArchive,
    character_archive: &NxArchive,
    ui_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    let ui = CharacterInfoUi {
        name: "PreviewHero".to_owned(),
        info: openstory_protocol::CharacterInfoReply {
            character_id: 2,
            level: 32,
            job: 100,
            fame: 7,
            married: false,
            guild: "Maple Union".to_owned(),
            alliance: "Leaf Alliance".to_owned(),
            pets: vec![openstory_protocol::CharacterInfoPet {
                item_id: 5_000_000,
                name: "Milo".to_owned(),
                level: 12,
                closeness: 1_245,
                fullness: 76,
                label_ring_item_id: 0,
            }],
            mount: Some(openstory_protocol::CharacterMountInfo {
                level: 3,
                experience: 2_500,
                tiredness: 11,
            }),
            wishlist: vec![1_000_002, 1_010_002, 1_050_002],
            monster_book: openstory_protocol::CharacterMonsterBookInfo {
                level: 4,
                normal_cards: 18,
                special_cards: 2,
                total_cards: 20,
                cover_item_id: 0,
            },
            medal: Some(openstory_protocol::CharacterMedalInfo {
                item_id: 1_142_000,
                quest_ids: vec![1000, 1001, 1002],
            }),
        },
        fame_pending: false,
    };
    let pixels =
        render_character_info_panel(font, &ui, ui_archive, string_archive, &mut HashMap::new());
    sprites.push(DecodedSprite {
        texture_key: "character-info-preview".to_owned(),
        rect: SpriteRect {
            x: CHARACTER_INFO_PANEL_X,
            y: CHARACTER_INFO_PANEL_Y,
            width: CHARACTER_INFO_PANEL_WIDTH as f32,
            height: CHARACTER_INFO_PANEL_HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: CHARACTER_INFO_PANEL_WIDTH,
        texture_height: CHARACTER_INFO_PANEL_HEIGHT,
        pixels: Arc::from(pixels),
    });
    Ok(LoginSceneAssets {
        sprites,
        actions: Vec::new(),
        map_scene: None,
        animated_art: Vec::new(),
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn build_minimap_scene(
    name: &str,
    map_archive: &NxArchive,
    ui_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let map_id = 211_000_000;
    let data = minimap::MiniMapData::load(map_archive, map_id)?
        .ok_or_else(|| format!("map {map_id} has no minimap to preview"))?;
    let ui_assets = minimap::UiAssets::load(ui_archive)?;
    let presentation = match name {
        "hud-minimap-collapsed" => minimap::Presentation::Collapsed,
        "hud-minimap-normal" => minimap::Presentation::Normal,
        "hud-minimap-expanded" => minimap::Presentation::Expanded,
        _ => return Err(format!("unknown minimap preview {name:?}")),
    };
    let units = 2.0_f32.powi(i32::from(data.magnification));
    let markers = [
        minimap::Marker {
            kind: minimap::MarkerKind::Player,
            x: -data.center_x + 32.0 * units,
            y: -data.center_y + 26.0 * units,
            facing_left: false,
            sprite: None,
        },
        minimap::Marker {
            kind: minimap::MarkerKind::Npc,
            x: -data.center_x + 26.0 * units,
            y: -data.center_y + 29.0 * units,
            facing_left: false,
            sprite: None,
        },
        minimap::Marker {
            kind: minimap::MarkerKind::Mob,
            x: -data.center_x + 42.0 * units,
            y: -data.center_y + 18.0 * units,
            facing_left: true,
            sprite: None,
        },
    ];
    let map_name =
        map_string_name(string_archive, map_id).unwrap_or_else(|| format!("Map {map_id}"));
    let street_name =
        map_string_street_name(string_archive, map_id).unwrap_or_else(|| map_name.clone());
    let panel = minimap::render(
        &data,
        &ui_assets,
        LogicalSize::LEGACY_1024X768,
        &map_name,
        &street_name,
        presentation,
        &markers,
        font,
    )?;
    Ok(LoginSceneAssets {
        sprites: vec![DecodedSprite {
            texture_key: format!("minimap-preview:{name}"),
            rect: SpriteRect {
                x: panel.x,
                y: panel.y,
                width: panel.width as f32,
                height: panel.height as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: panel.width,
            texture_height: panel.height,
            pixels: Arc::from(panel.pixels),
        }],
        actions: Vec::new(),
        map_scene: None,
        animated_art: Vec::new(),
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn build_monster_book_detail_scene(
    map_archive: &NxArchive,
    ui_archive: &NxArchive,
    item_archive: &NxArchive,
    character_archive: &NxArchive,
    mob_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
    detail_animation_mode: usize,
) -> Result<LoginSceneAssets, String> {
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    let mut icon_cache = HashMap::new();
    let mut animation_state = monster_book::State::default();
    let detail = detail_animation_mode != usize::MAX;
    if detail {
        animation_state.select_card(2_380_000, 100_100, mob_archive);
        for _ in 0..detail_animation_mode {
            animation_state.cycle_detail_animation();
        }
    }
    let cards = item_archive
        .root()
        .resolve("Consume/0238.img")
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "missing monster cards".to_owned())?
        .children()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter_map(|node| node.name().ok()?.parse::<i32>().ok().map(|id| (id, 1)))
        .take(75)
        .collect::<Vec<_>>();
    let panel = monster_book::render(
        ui_archive,
        item_archive,
        character_archive,
        mob_archive,
        string_archive,
        &cards,
        &HashMap::new(),
        2_380_000,
        if detail { 0 } else { 1 },
        None,
        detail.then_some(2_380_000),
        detail.then(|| animation_state.current_detail_animation()),
        false,
        "",
        None,
        false,
        0,
        None,
        0,
        &mut icon_cache,
        font,
    )?;
    sprites.push(DecodedSprite {
        texture_key: "monster-book-detail-preview".to_owned(),
        rect: SpriteRect {
            x: panel.x,
            y: panel.y,
            width: panel.width as f32,
            height: panel.height as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: panel.width,
        texture_height: panel.height,
        pixels: Arc::from(panel.pixels),
    });
    if detail
        && let Some(frame) = animation_state.refresh_detail_animation_sprite(mob_archive)?
        && let Some(draw) = animation_state.current_detail_animation_draw()
    {
        let mut pixels = frame.pixels.to_vec();
        for pixel in pixels.chunks_exact_mut(4) {
            pixel[3] = (f32::from(pixel[3]) * draw.opacity).round() as u8;
        }
        sprites.push(DecodedSprite {
            texture_key: format!("monster-book-detail-animation-{detail_animation_mode}"),
            rect: SpriteRect {
                x: draw.x,
                y: draw.y,
                width: draw.width,
                height: draw.height,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: frame.width,
            texture_height: frame.height,
            pixels: Arc::from(pixels),
        });
    }
    Ok(LoginSceneAssets {
        sprites,
        actions: Vec::new(),
        map_scene: None,
        animated_art: Vec::new(),
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn gameplay_backdrop(
    map_archive: &NxArchive,
    character_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<Vec<DecodedSprite>, String> {
    gameplay_backdrop_for_map(map_archive, character_archive, font, 100_000_000)
}

fn gameplay_backdrop_for_map(
    map_archive: &NxArchive,
    character_archive: &NxArchive,
    font: &fontdue::Font,
    map_id: i32,
) -> Result<Vec<DecodedSprite>, String> {
    let mut map_scene = load_map_scene(map_archive, None, map_id, 0, None, font)?;
    let map_label = format!("label:15:Map {map_id}");
    map_scene
        .sprites
        .retain(|sprite| sprite.texture_key != map_label);
    append_character_preview(
        &mut map_scene.sprites,
        character_archive,
        &preview_character().look,
        420.0,
        super::GAMEPLAY_CAMERA_CENTER_Y,
        1.0,
        true,
        &mut MapArtCache::default(),
    )?;
    Ok(map_scene.sprites)
}

pub(super) fn fixture_named_id(
    archive: &NxArchive,
    path: &str,
    wanted: &str,
    fallback: i32,
) -> i32 {
    archive
        .root()
        .resolve(path)
        .ok()
        .flatten()
        .and_then(|node| {
            node.children().ok()?.into_iter().find_map(|node| {
                let name = node.child("name").ok().flatten()?.value().ok()?;
                if matches!(name, NodeValue::String(ref name) if name == wanted) {
                    node.name().ok()?.parse::<i32>().ok()
                } else {
                    None
                }
            })
        })
        .unwrap_or(fallback)
}

pub(super) fn preview_inventory() -> openstory_protocol::InventorySnapshot {
    use openstory_protocol::{InventoryItem, InventorySnapshot, InventoryType};
    let mut inventory = InventorySnapshot {
        meso: 1_765_592,
        slot_maxima: [48; 5],
        ..Default::default()
    };
    let sets: [(InventoryType, &[i32]); 5] = [
        (
            InventoryType::Equip,
            &[
                1002000, 1002001, 1002002, 1002003, 1002004, 1002005, 1040002, 1060002, 1072001,
                1082000, 1302000, 1332000,
            ],
        ),
        (
            InventoryType::Use,
            &[
                2000000, 2000001, 2000002, 2000003, 2000006, 2020000, 2020001, 2030000, 2040000,
                2060000, 2061000, 2070000,
            ],
        ),
        (
            InventoryType::Setup,
            &[3010000, 3010001, 3010002, 3010003, 3010004, 3010005],
        ),
        (
            InventoryType::Etc,
            &[
                4000000, 4000001, 4000002, 4000003, 4000004, 4000005, 4000006, 4000007, 4000008,
                4000009, 4000010, 4000011, 4000012, 4000013, 4000014, 4000015, 4000016, 4000017,
                4000018, 4000019, 4000020, 4000021, 4000022, 4000023,
            ],
        ),
        (
            InventoryType::Cash,
            &[
                5000000, 5000001, 5010000, 5020000, 5030000, 5040000, 5050000, 5060000,
            ],
        ),
    ];
    for (inventory_type, ids) in sets {
        for (index, item_id) in ids.iter().copied().enumerate() {
            inventory.items.push(InventoryItem {
                inventory_type,
                slot: index as u16 + 1,
                item_id,
                count: if inventory_type == InventoryType::Equip {
                    1
                } else {
                    [8, 1, 10, 5, 2, 1][index % 6]
                },
                ..Default::default()
            });
        }
    }
    inventory
}

fn build_inventory_tabs_scene(
    map: &NxArchive,
    ui: &NxArchive,
    characters: &NxArchive,
    items: &NxArchive,
    strings: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let mut scene = build_inventory_panel_scene(
        "panel-item-inventory",
        map,
        ui,
        characters,
        items,
        strings,
        font,
    )?;
    scene.sprites.pop();
    let inventory = preview_inventory();
    for (index, tab) in inventory_tabs().into_iter().enumerate() {
        let pixels = render_inventory_panel(
            font,
            &inventory,
            tab,
            0,
            None,
            None,
            None,
            ui,
            items,
            characters,
            strings,
            &mut HashMap::new(),
            &mut HashMap::new(),
        );
        scene.sprites.push(DecodedSprite {
            texture_key: format!("inventory-tab:{index}"),
            rect: SpriteRect {
                x: index as f32 * 180.0,
                y: 20.0,
                width: INVENTORY_PANEL_WIDTH as f32,
                height: INVENTORY_PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: INVENTORY_PANEL_WIDTH,
            texture_height: INVENTORY_PANEL_HEIGHT,
            pixels: Arc::from(pixels),
        });
    }
    Ok(scene)
}

fn build_inventory_panel_scene(
    variant: &str,
    map_archive: &NxArchive,
    legacy_ui_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let mut inventory = preview_inventory();
    if variant.ends_with("-empty") {
        inventory.items.clear();
    }
    if variant.ends_with("-large-values") {
        inventory.meso = i64::MAX;
        for item in &mut inventory.items {
            item.count = u16::MAX;
        }
    }
    let scroll = if variant.ends_with("-scroll") { 3 } else { 0 };
    let drag = variant
        .ends_with("-drag")
        .then_some((openstory_protocol::InventoryType::Etc, 1));
    let pixels = super::render_inventory_panel(
        font,
        &inventory,
        openstory_protocol::InventoryType::Etc,
        scroll,
        None,
        drag,
        None,
        legacy_ui_archive,
        item_archive,
        character_archive,
        string_archive,
        &mut HashMap::new(),
        &mut HashMap::new(),
    );
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    sprites.push(DecodedSprite {
        texture_key: "nx-window:live-inventory".to_owned(),
        rect: SpriteRect {
            x: super::INVENTORY_PANEL_X,
            y: super::INVENTORY_PANEL_Y,
            width: super::INVENTORY_PANEL_WIDTH as f32,
            height: super::INVENTORY_PANEL_HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: super::INVENTORY_PANEL_WIDTH,
        texture_height: super::INVENTORY_PANEL_HEIGHT,
        pixels: Arc::from(pixels),
    });
    Ok(LoginSceneAssets {
        sprites,
        actions: Vec::new(),
        map_scene: None,
        animated_art: Vec::new(),
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn build_trade_panel_scene(
    map_archive: &NxArchive,
    ui_archive: &NxArchive,
    character_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let mut pixels = vec![0; 720 * 600 * 4];
    super::composite_nx_ui_bitmap(&mut pixels, 720, 600, ui_archive,
        "UIWindow.img/TradingRoom/backgrnd", 78, 20)?;
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    sprites.push(DecodedSprite { texture_key: "nx-window:live-trade".to_owned(),
        rect: SpriteRect { x: 152.0, y: 84.0, width: 720.0, height: 600.0, blend_mode: BlendMode::Alpha },
        depth_order: 0, texture_width: 720, texture_height: 600, pixels: Arc::from(pixels) });
    Ok(LoginSceneAssets { sprites, actions: Vec::new(), map_scene: None,
        animated_art: Vec::new(), login_buttons: Vec::new(), save_id_indicator_draws: None })
}

fn build_npc_store_panel_scene(
    variant: &str,
    map_archive: &NxArchive,
    ui_archive: &NxArchive,
    npc_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let shop = openstory_protocol::NpcShop {
        npc_id: fixture_named_id(string_archive, "Npc.img", "Mr. Wang", 1022005),
        currency_item_id: 0,
        items: (0..12)
            .map(|index| openstory_protocol::NpcShopItem {
                item_id: 2_000_000 + index,
                price: 50 * (index + 1),
                pitch: 0,
                time: 0,
                charge_price: 0,
                buyable: 100,
                rechargeable: false,
                slot: index as i16,
            })
            .collect(),
    };
    let (pixels, rect) = super::render_npc_shop_panel(
        font,
        &shop,
        if variant == "panel-npc-store-sell" { super::NpcShopMode::Sell } else { super::NpcShopMode::Buy },
        0,
        0,
        1,
        Some(&preview_inventory()),
        None,
        (variant == "panel-npc-store-confirmation").then_some(super::NpcShopConfirmation { mode: super::NpcShopMode::Buy, slot: 0, item_id: 2_000_000, quantity: 1, recharge: false }),
        ui_archive,
        npc_archive,
        item_archive,
        string_archive,
        character_archive,
        &mut HashMap::new(),
        &mut HashMap::new(),
    );
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    sprites.push(DecodedSprite {
        texture_key: "nx-window:live-npc-store".to_owned(),
        rect,
        depth_order: 0,
        texture_width: 620,
        texture_height: 520,
        pixels: Arc::from(pixels),
    });
    Ok(LoginSceneAssets {
        sprites,
        actions: Vec::new(),
        map_scene: None,
        animated_art: Vec::new(),
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn build_pet_equipment_scene(
    name: &str,
    map_archive: &NxArchive,
    ui_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let mut inventory = openstory_protocol::InventorySnapshot::default();
    if name != "panel-pet-equipment-empty" {
        inventory.items = vec![
                openstory_protocol::InventoryItem {
                    inventory_type: openstory_protocol::InventoryType::Cash,
                    slot: 1,
                    item_id: 5_000_000,
                    pet: Some(openstory_protocol::PetInventoryData {
                        name: "Kitty".to_owned(), level: 12, fullness: 85, ..Default::default()
                    }),
                    ..Default::default()
                },
                openstory_protocol::InventoryItem {
                    inventory_type: openstory_protocol::InventoryType::Equip,
                    slot: 4,
                    item_id: 1_812_000,
                    ..Default::default()
                },
            ];
    }
    if name == "panel-pet-equipment-scroll" {
        for slot in 5..=11 {
            inventory.items.push(openstory_protocol::InventoryItem {
                inventory_type: openstory_protocol::InventoryType::Equip,
                slot,
                item_id: 1_812_000,
                ..Default::default()
            });
        }
    }
    let pixels = super::render_pet_equipment_panel(
        font,
        &inventory,
        ui_archive,
        item_archive,
        character_archive,
        string_archive,
        &mut HashMap::new(),
        &mut HashMap::new(),
        if name == "panel-pet-equipment-scroll" { 3 } else { 0 },
    );
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    sprites.push(DecodedSprite {
        texture_key: "rendered-window:pet-equipment".to_owned(),
        rect: SpriteRect {
            x: 618.0,
            y: 128.0,
            width: 390.0,
            height: 480.0,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: 390,
        texture_height: 480,
        pixels: Arc::from(pixels),
    });
    Ok(LoginSceneAssets {
        sprites,
        actions: Vec::new(),
        map_scene: None,
        animated_art: Vec::new(),
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn build_storage_panel_scene(
    map_archive: &NxArchive,
    ui_archive: &NxArchive,
    npc_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let storage = openstory_protocol::StorageSnapshot {
        npc_id: fixture_named_id(string_archive, "Npc.img", "Mr. Wang", 1022005),
        slot_capacity: 24,
        meso: 96_000,
        items: (0..22)
            .map(|index| openstory_protocol::InventoryItem {
                inventory_type: openstory_protocol::InventoryType::Equip,
                slot: index + 1,
                item_id: 1_002_000 + i32::from(index % 20),
                count: if index % 4 == 0 { 100 } else { 1 },
                ..Default::default()
            })
            .collect(),
    };
    let pixels = super::render_storage_reference_panel(
        font,
        &storage,
        openstory_protocol::InventoryType::Equip,
        0,
        "",
        false,
        None,
        ui_archive,
        npc_archive,
        item_archive,
        character_archive,
        &mut HashMap::new(),
    );
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    sprites.push(DecodedSprite {
        texture_key: "nx-window:live-storage".to_owned(),
        rect: SpriteRect {
            x: 182.0,
            y: 105.0,
            width: 206.0,
            height: 355.0,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: 206,
        texture_height: 355,
        pixels: Arc::from(pixels),
    });
    let _ = string_archive;
    Ok(LoginSceneAssets {
        sprites,
        actions: Vec::new(),
        map_scene: None,
        animated_art: Vec::new(),
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn build_item_tooltip_scene(
    map_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    string_archive: &NxArchive,
    ui_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let item = openstory_protocol::InventoryItem {
        inventory_type: openstory_protocol::InventoryType::Equip,
        slot: 1,
        item_id: fixture_named_id(string_archive, "Eqp.img/Eqp/Coat", "Grey T-Shirt", 1040010),
        count: 1,
        equip: Some({
            let mut data = openstory_protocol::EquipInventoryData::default();
            data.upgrade_slots = 7;
            data.stats[8] = 3;
            data
        }),
        ..Default::default()
    };
    let pixels = super::render_inventory_tooltip(
        font,
        &item,
        ui_archive,
        item_archive,
        character_archive,
        string_archive,
        &mut HashMap::new(),
        &mut HashMap::new(),
    );
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    sprites.push(DecodedSprite {
        texture_key: "inventory-tooltip:preview".to_owned(),
        rect: SpriteRect {
            x: 520.0,
            y: 240.0,
            width: super::INVENTORY_TOOLTIP_WIDTH as f32,
            height: super::INVENTORY_TOOLTIP_HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: super::INVENTORY_TOOLTIP_WIDTH,
        texture_height: super::INVENTORY_TOOLTIP_HEIGHT,
        pixels: Arc::from(pixels),
    });
    Ok(LoginSceneAssets {
        sprites,
        actions: Vec::new(),
        map_scene: None,
        animated_art: Vec::new(),
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn build_equipment_panel_scene(
    map_archive: &NxArchive,
    ui_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let inventory = preview_equipped_inventory();
    let pixels = super::render_equipment_inventory_panel(
        &inventory,
        ui_archive,
        item_archive,
        character_archive,
        &mut HashMap::new(),
    );
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    sprites.push(DecodedSprite {
        texture_key: "nx-window:live-equipment".to_owned(),
        rect: SpriteRect {
            x: super::EQUIPMENT_INVENTORY_PANEL_X,
            y: super::EQUIPMENT_INVENTORY_PANEL_Y,
            width: super::EQUIPMENT_INVENTORY_PANEL_WIDTH as f32,
            height: super::EQUIPMENT_INVENTORY_PANEL_HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: super::EQUIPMENT_INVENTORY_PANEL_WIDTH,
        texture_height: super::EQUIPMENT_INVENTORY_PANEL_HEIGHT,
        pixels: Arc::from(pixels),
    });
    Ok(LoginSceneAssets {
        sprites,
        actions: Vec::new(),
        map_scene: None,
        animated_art: Vec::new(),
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn build_quest_log_panel_scene(
    name: &str,
    map_archive: &NxArchive,
    legacy_ui_archive: &NxArchive,
    quest_archive: &NxArchive,
    string_archive: &NxArchive,
    character_archive: &NxArchive,
    npcs: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let detail_preview = name == "panel-quests-detail";
    let desired = [
        "The Right Path",
        "Stomping the Castle",
        "The Brewing Storm",
        "The Mark of Heroism",
        "Bounty Hunter",
        "Bounty Hunting",
        "Paving the Way",
        "Temple of Time",
        "World Tour",
    ];
    let root = quest_archive
        .root()
        .resolve("QuestInfo.img")
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "missing quest catalogue".to_owned())?;
    let mut ids = Vec::new();
    for node in root.children().map_err(|e| e.to_string())? {
        let Some(id) = node.name().ok().and_then(|name| name.parse::<i16>().ok()) else {
            continue;
        };
        if let Some(title) = quest_info_text(quest_archive, id, "name")
            && desired.iter().any(|name| title.contains(name))
        {
            ids.push(id);
        }
        if ids.len() >= 20 {
            break;
        }
    }
    if ids.is_empty() {
        ids.extend([1000, 1001, 1002]);
    }
    let log = openstory_protocol::QuestLog {
        started: ids
            .iter()
            .copied()
            .map(|id| openstory_protocol::QuestEntry {
                id,
                data: String::new(),
            })
            .collect(),
        completed: ids
            .iter()
            .copied()
            .map(|id| openstory_protocol::CompletedQuest { id, timestamp: 0 })
            .collect(),
    };
    let tab = if detail_preview {
        super::QuestLogTab::Active
    } else {
        super::QuestLogTab::Completed
    };
    let available = Vec::new();
    let pixels = super::render_quest_log_panel(
        font,
        &log,
        &available,
        quest_archive,
        string_archive,
        None,
        &mut HashMap::new(),
        tab,
        0,
        0,
        0,
        None,
        None,
        legacy_ui_archive,
        npcs,
    );
    let canvas_width = super::QUEST_LOG_PANEL_WIDTH as usize;
    let (panel_width, panel_height) = (
        QUEST_LOG_PANEL_WIDTH as usize,
        QUEST_LOG_PANEL_HEIGHT as usize,
    );
    let mut visible_pixels = Vec::with_capacity(panel_width * panel_height * 4);
    for row in 0..panel_height {
        let source = row * canvas_width * 4;
        visible_pixels.extend_from_slice(&pixels[source..source + panel_width * 4]);
    }
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    sprites.push(DecodedSprite {
        texture_key: "nx-window:live-quest-log".to_owned(),
        rect: SpriteRect {
            x: super::QUEST_LOG_PANEL_X,
            y: super::QUEST_LOG_PANEL_Y,
            width: panel_width as f32,
            height: panel_height as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: panel_width as u32,
        texture_height: panel_height as u32,
        pixels: Arc::from(visible_pixels),
    });
    Ok(LoginSceneAssets {
        sprites,
        actions: Vec::new(),
        map_scene: None,
        animated_art: Vec::new(),
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

fn build_native_window_scene(
    name: &str,
    map_archive: &NxArchive,
    legacy_ui_archive: &NxArchive,
    ui_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    skill_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let mut sprites = if name == "page-cash-shop" {
        Vec::new()
    } else {
        gameplay_backdrop(map_archive, character_archive, font)?
    };

    // These are the original client window layers and logical positions, not
    // approximations generated by the Rust panel renderer. Related layers
    // share the same anchor; their NX origins provide the authored offsets.
    let layers: &[(&str, f32, f32)] = match name {
        "panel-keyboard-settings"
        | "panel-keyboard-settings-empty"
        | "panel-keyboard-settings-capture"
        | "panel-keyboard-settings-scroll"
        | "panel-keyboard-settings-quickslots" => &[],
        "hud-quickslot" | "panel-skill-macros" => &[],
        "panel-item-inventory" => &[("UIWindow.img/Item/backgrnd", 300.0, 160.0)],
        "panel-equipment-inventory" => &[],
        "panel-skill-inventory" => &[
            // Compose the same scaled v83 Skill window used by the live panel below.
        ],
        "panel-npc-store" => &[
            ("UIWindow2.img/Shop2/backgrnd", 400.0, 20.0),
            ("UIWindow2.img/Shop2/backgrnd2", 400.0, 20.0),
            ("UIWindow2.img/Shop2/backgrnd3", 400.0, 20.0),
        ],
        "page-cash-shop" => &[("CashShop.img/Base/backgrnd", 0.0, 0.0)],
        "panel-monster-book" => &[("UIWindow2.img/MonsterBook/backgrnd", 276.5, 20.0)],
        "panel-stats" | "panel-stats-detail" => &[
            // Render the live v83 status sheets below so preview and gameplay
            // share the same NX art and layout.
        ],
        "panel-quests" => &[("UIWindow.img/Quest/backgrnd3", 280.0, 20.0)],
        // The v83 UI archive has StatusBar.img, not the newer StatusBar2
        // chat-bar tree. Preview the live Rust fallback panel below.
        "hud-chat" | "panel-whisper" => &[],
        "panel-party" | "panel-buddy" => &[],
        // The C++ v83 client uses the UserList tree. Newer GuildUI.img art is
        // absent from the installed v83 UI.nx archive.
        "panel-guild" | "panel-alliance" => &[],
        "panel-audio-settings" => &[
            ("UIWindow2.img/OptionMenu/backgrnd", 370.0, 105.0),
            ("UIWindow2.img/OptionMenu/backgrnd2", 370.0, 105.0),
            ("UIWindow2.img/OptionMenu/backgrnd3", 370.0, 105.0),
        ],
        _ => return Err(format!("no NX native window fixture for {name:?}")),
    };

    for (path, anchor_x, anchor_y) in layers {
        let first_sprite = sprites.len();
        let archive = if matches!(name, "panel-item-inventory" | "panel-quests") {
            legacy_ui_archive
        } else {
            ui_archive
        };
        let sprite = if name == "panel-keyboard-settings" {
            append_optional_sprite_at_top_left(
                &mut sprites,
                archive,
                path,
                *anchor_x,
                *anchor_y,
                1.0,
            )?
        } else {
            append_optional_sprite(&mut sprites, archive, path, *anchor_x, *anchor_y, 1.0)?
        };
        if sprite.is_some() {
            sprites[first_sprite].texture_key = format!("nx-window:{path}");
        }
    }

    // Compose the live Rust panels into previews so v83 assets and dimensions
    // are audited through the same renderers used in game.
    if name == "panel-guild" {
        let pixels = render_guild_panel(
            font,
            ui_archive,
            None,
            None,
            None,
            0,
            &GuildPanelMode::Overview,
            None,
            None,
        );
        sprites.push(DecodedSprite {
            texture_key: "nx-window:rendered-v83-guild".to_owned(),
            rect: SpriteRect {
                x: 680.0,
                y: 174.0,
                width: GUILD_PANEL_WIDTH as f32,
                height: GUILD_PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: GUILD_PANEL_WIDTH,
            texture_height: GUILD_PANEL_HEIGHT,
            pixels: Arc::from(pixels),
        });
    } else if name == "panel-alliance" {
        let pixels = render_alliance_panel(font, ui_archive, None, None, None);
        sprites.push(DecodedSprite {
            texture_key: "nx-window:rendered-v83-alliance".to_owned(),
            rect: SpriteRect {
                x: 300.0,
                y: 190.0,
                width: ALLIANCE_PANEL_WIDTH as f32,
                height: ALLIANCE_PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: ALLIANCE_PANEL_WIDTH,
            texture_height: ALLIANCE_PANEL_HEIGHT,
            pixels: Arc::from(pixels),
        });
    } else if name == "panel-party" {
        let party = openstory_protocol::PartySnapshot {
            party_id: 83,
            leader_id: 1,
            members: vec![
                openstory_protocol::PartyMember {
                    character_id: 1,
                    name: "PreviewHero".to_owned(),
                    job: 0,
                    level: 30,
                    channel: 0,
                    map_id: 100_000_000,
                    hp: Some((850, 1_000)),
                },
                openstory_protocol::PartyMember {
                    character_id: 2,
                    name: "Maya".to_owned(),
                    job: 100,
                    level: 31,
                    channel: 0,
                    map_id: 100_000_000,
                    hp: Some((670, 900)),
                },
                openstory_protocol::PartyMember {
                    character_id: 3,
                    name: "Alex".to_owned(),
                    job: 200,
                    level: 29,
                    channel: -1,
                    map_id: 0,
                    hp: None,
                },
            ],
        };
        let pixels = render_party_panel(font, ui_archive, Some(&party), None, Some(1));
        sprites.push(DecodedSprite {
            texture_key: "nx-window:rendered-v83-party".to_owned(),
            rect: SpriteRect {
                x: PARTY_PANEL_X,
                y: PARTY_PANEL_Y,
                width: PARTY_PANEL_WIDTH as f32,
                height: PARTY_PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: PARTY_PANEL_WIDTH,
            texture_height: PARTY_PANEL_HEIGHT,
            pixels: Arc::from(pixels),
        });
    } else if name == "panel-buddy" {
        let buddies = [
            openstory_protocol::BuddyEntry {
                character_id: 2,
                name: "Maya".to_owned(),
                status: 0,
                channel: 0,
                group: "Friends".to_owned(),
            },
            openstory_protocol::BuddyEntry {
                character_id: 4,
                name: "Rin".to_owned(),
                status: 0,
                channel: 2,
                group: "Friends".to_owned(),
            },
            openstory_protocol::BuddyEntry {
                character_id: 5,
                name: "Alex".to_owned(),
                status: 0,
                channel: -1,
                group: "Guild".to_owned(),
            },
        ];
        let pixels =
            render_buddy_panel(font, ui_archive, &buddies, 20, buddies.len(), None, None, 0);
        sprites.push(DecodedSprite {
            texture_key: "nx-window:rendered-v83-buddy".to_owned(),
            rect: SpriteRect {
                x: BUDDY_PANEL_X,
                y: BUDDY_PANEL_Y,
                width: BUDDY_PANEL_WIDTH as f32,
                height: BUDDY_PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: BUDDY_PANEL_WIDTH,
            texture_height: BUDDY_PANEL_HEIGHT,
            pixels: Arc::from(pixels),
        });
    } else if matches!(name, "panel-stats" | "panel-stats-detail") {
        let pixels = render_status_panel(font, ui_archive, &preview_character(), false, false);
        sprites.push(DecodedSprite {
            texture_key: "rendered-window:status-panel".to_owned(),
            rect: SpriteRect {
                x: STATUS_PANEL_X,
                y: STATUS_PANEL_Y,
                width: STATUS_PANEL_WIDTH as f32,
                height: STATUS_PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: STATUS_PANEL_WIDTH,
            texture_height: STATUS_PANEL_HEIGHT,
            pixels: Arc::from(pixels),
        });
        if matches!(name, "panel-stats" | "panel-stats-detail") {
            let character = preview_character();
            let inventory = preview_equipped_inventory();
            let skills = HashMap::new();
            let active_buffs = CharacterBuffs::default();
            let forced_stats = ForcedEquipmentStats::default();
            let pixels = render_status_detail_panel(
                font,
                ui_archive,
                &character,
                Some(&inventory),
                character_archive,
                item_archive,
                skill_archive,
                &skills,
                &active_buffs,
                &forced_stats,
            );
            sprites.push(DecodedSprite {
                texture_key: "rendered-window:status-detail-panel".to_owned(),
                rect: SpriteRect {
                    x: STATUS_PANEL_X + STATUS_DETAIL_OFFSET_X,
                    y: STATUS_PANEL_Y + STATUS_DETAIL_OFFSET_Y,
                    width: STATUS_DETAIL_WIDTH as f32,
                    height: STATUS_DETAIL_HEIGHT as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: STATUS_DETAIL_WIDTH,
                texture_height: STATUS_DETAIL_HEIGHT,
                pixels: Arc::from(pixels),
            });
        }
    } else if name == "hud-chat" {
        let lines = (0..9).map(|index| ChatLine {
            text: if index % 3 == 0 { "The fairy's Gifts is now level 2 - 1WATK" } else { "The scroll lights up, and then its mysterious power has been transferred to the item." }.to_owned(),
            color: if index % 3 == 0 { [80, 200, 255] } else { [245, 170, 95] },
        }).collect::<VecDeque<_>>();
        let pixels =
            render_chat_panel(ui_archive, font, &lines, "", true, ChatTarget::Public, None);
        sprites.push(DecodedSprite {
            texture_key: "rendered-window:chat-panel".to_owned(),
            rect: SpriteRect {
                x: CHAT_X,
                y: CHAT_Y,
                width: CHAT_WIDTH as f32,
                height: CHAT_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: CHAT_WIDTH,
            texture_height: CHAT_HEIGHT,
            pixels: Arc::from(pixels),
        });
    } else if name == "panel-whisper" {
        let conversation = WhisperConversation {
            target: "Maya".to_owned(),
            lines: VecDeque::from([
                ChatLine {
                    text: "Maya: Are you at Henesys?".to_owned(),
                    color: [40, 40, 40],
                },
                ChatLine {
                    text: "You: Yes, near the portal.".to_owned(),
                    color: [40, 40, 40],
                },
            ]),
            input: "See you soon!".to_owned(),
        };
        let pixels = render_whisper_panel(ui_archive, font, &conversation, true, false);
        sprites.push(DecodedSprite {
            texture_key: "rendered-window:whisper-panel".to_owned(),
            rect: SpriteRect {
                x: WHISPER_PANEL_X,
                y: WHISPER_PANEL_Y,
                width: WHISPER_PANEL_WIDTH as f32,
                height: WHISPER_PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: WHISPER_PANEL_WIDTH,
            texture_height: WHISPER_PANEL_HEIGHT,
            pixels: Arc::from(pixels),
        });
    }

    if name == "hud-quickslot" {
        let keys = DEFAULT_QUICKSLOT_KEYS;
        let bindings = keyboard_settings::defaults();
        let mut icon_cache = HashMap::new();
        let icons = std::array::from_fn(|slot| {
            let binding = bindings
                .get(usize::from(keys[slot]))
                .copied()
                .unwrap_or_default();
            match binding.kind {
                1 => decode_nx_icon(
                    skill_archive,
                    &format!(
                        "{:03}.img/skill/{}/icon",
                        binding.action / 10_000,
                        binding.action
                    ),
                ),
                2 | 3 => inventory_item_icon(
                    binding.action,
                    item_archive,
                    character_archive,
                    &mut icon_cache,
                ),
                4 | 5 => {
                    let path = format!("UIWindow.img/KeyConfig/icon/{}", binding.action);
                    decode_nx_icon(ui_archive, &path).or_else(|| {
                        decode_nx_icon(
                            ui_archive,
                            &format!("UIWindow2.img/KeyConfig/icon/{}", binding.action),
                        )
                    })
                }
                8 => decode_nx_icon(
                    ui_archive,
                    &format!("UIWindow.img/SkillMacro/Macroicon/{}/icon", binding.action),
                ),
                _ => None,
            }
        });
        let pixels = render_quickslot_panel_pixels(
            quickslot_background_image(legacy_ui_archive),
            ui_archive,
            &keys,
            &icons,
        );
        sprites.push(DecodedSprite {
            texture_key: "quickslots:preview".to_owned(),
            rect: SpriteRect {
                x: 865.0,
                y: 615.0,
                width: 151.0,
                height: 80.0,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: 151,
            texture_height: 80,
            pixels: Arc::from(pixels),
        });
    }

    if name.starts_with("panel-keyboard-settings") {
        let fixture = keyboard_preview_fixture(name);
        let pixels = render_key_config_panel(
            font,
            ui_archive,
            skill_archive,
            item_archive,
            character_archive,
            &fixture.bindings,
            &DEFAULT_QUICKSLOT_KEYS,
            fixture.capture,
            fixture.quickslot_capture,
            fixture.first_row,
            &fixture.actions,
            &mut HashMap::new(),
            &HashMap::from([(2_000_006, 245), (2_000_002, 251)]),
        );
        sprites.push(DecodedSprite {
            texture_key: "key-config-preview".to_owned(),
            rect: SpriteRect {
                x: KEY_CONFIG_PANEL_X,
                y: KEY_CONFIG_PANEL_Y,
                width: KEY_CONFIG_PANEL_WIDTH as f32,
                height: KEY_CONFIG_PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: KEY_CONFIG_PANEL_WIDTH,
            texture_height: KEY_CONFIG_PANEL_HEIGHT,
            pixels: Arc::from(pixels),
        });
    }

    if name == "panel-skill-inventory" {
        let mut entries = preview_skill_entries();
        for (index, (entry, (id, skill_name))) in entries
            .iter_mut()
            .zip([
                (1_100_000, "Sword Mastery"),
                (1_100_001, "Axe Mastery"),
                (1_100_002, "Final Attack: Sword"),
                (1_100_003, "Final Attack: Axe"),
            ])
            .enumerate()
        {
            entry.id = id;
            entry.name = skill_name.to_owned();
            entry.level = 0;
            entry.max_level = if index < 2 { 20 } else { 0 };
        }
        entries.truncate(4);
        let book_icon = decode_nx_icon(skill_archive, "110.img/info/icon");
        let pixels = render_skill_book_panel(
            font,
            ui_archive,
            skill_archive,
            &entries,
            "Fighter Techniques",
            book_icon,
            2,
            2,
            3,
            0,
        );
        sprites.push(DecodedSprite {
            texture_key: "nx-window:rendered-v83-skills".to_owned(),
            rect: SpriteRect {
                x: SKILL_BOOK_PANEL_X,
                y: SKILL_BOOK_PANEL_Y,
                width: SKILL_BOOK_PANEL_WIDTH as f32,
                height: SKILL_BOOK_PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: SKILL_BOOK_PANEL_WIDTH,
            texture_height: SKILL_BOOK_PANEL_HEIGHT,
            pixels: Arc::from(pixels),
        });
    }

    if name == "panel-skill-macros" {
        let skills = vec![
            (1_001_004, "Power Strike".to_owned()),
            (1_001_005, "Slash Blast".to_owned()),
            (1_001_003, "Iron Body".to_owned()),
            (1_101_003, "Booster".to_owned()),
            (1_101_004, "Soul Arrow".to_owned()),
            (1_101_005, "Final Attack".to_owned()),
            (1_201_003, "Magic Guard".to_owned()),
            (1_201_004, "Magic Armor".to_owned()),
            (1_301_003, "Critical Shot".to_owned()),
            (1_301_004, "Double Shot".to_owned()),
        ];
        let macros = [
            openstory_protocol::SkillMacro {
                name: "Power Combo".to_owned(),
                shout: true,
                skills: [1_001_004, 1_001_005, 1_001_003],
            },
            openstory_protocol::SkillMacro {
                name: "Ranged Setup".to_owned(),
                shout: false,
                skills: [1_101_003, 1_101_004, 0],
            },
            openstory_protocol::SkillMacro {
                name: "Magic Guard".to_owned(),
                shout: false,
                skills: [1_201_003, 1_201_004, 0],
            },
        ];
        let mut editor = skill_macros::State::default();
        editor.open(&macros);
        editor.selected_macro = 1;
        editor.selected_slot = 2;
        let pixels = skill_macros::render(font, ui_archive, skill_archive, &editor, &skills);
        sprites.push(DecodedSprite {
            texture_key: "skill-macros-preview".to_owned(),
            rect: SpriteRect {
                x: skill_macros::PANEL_X,
                y: skill_macros::PANEL_Y,
                width: skill_macros::PANEL_WIDTH as f32,
                height: skill_macros::PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: skill_macros::PANEL_WIDTH,
            texture_height: skill_macros::PANEL_HEIGHT,
            pixels: Arc::from(pixels),
        });
    }

    Ok(LoginSceneAssets {
        sprites,
        actions: Vec::new(),
        map_scene: None,
        animated_art: Vec::new(),
        login_buttons: Vec::new(),
        save_id_indicator_draws: None,
    })
}

pub(super) fn preview_character() -> openstory_protocol::CharacterInfo {
    openstory_protocol::CharacterInfo {
        id: 1,
        stats: openstory_protocol::CharacterStats {
            name: "PreviewHero".to_owned(),
            level: 30,
            job: 0,
            strength: 35,
            dexterity: 28,
            intelligence: 10,
            luck: 12,
            hp: 850,
            max_hp: 1000,
            mp: 270,
            max_mp: 450,
            ability_points: 4,
            skill_points: 3,
            fame: 7,
            meso: Some(123_456),
            ..Default::default()
        },
        look: fixture_look(),
    }
}

fn preview_equipped_inventory() -> openstory_protocol::InventorySnapshot {
    openstory_protocol::InventorySnapshot {
        slot_maxima: [24; 5],
        equipped_slot_maximum: 59,
        items: [
            (1, 1_002_002),  // hat
            (5, 1_040_002),  // top
            (6, 1_060_002),  // bottom
            (7, 1_072_001),  // shoes
            (11, 1_302_000), // weapon
        ]
        .into_iter()
        .map(|(slot, item_id)| openstory_protocol::InventoryItem {
            inventory_type: openstory_protocol::InventoryType::Equipped,
            slot,
            item_id,
            count: 1,
            ..Default::default()
        })
        .collect(),
        ..Default::default()
    }
}

fn preview_skill_entry() -> SkillUiEntry {
    SkillUiEntry {
        id: 1_001_004,
        name: "Power Strike".to_owned(),
        level: 1,
        master_level: 0,
        max_level: 20,
        required_weapon: None,
        passive: false,
        attack: true,
        magic_damage: false,
        magic_damage_multiplier: 1.0,
        magic_mastery: 0.0,
        element: None,
        attack_count: 1,
        bullet_count: 0,
        mob_count: 1,
        damage_multiplier: 1.0,
        horizontal_range_multiplier: 1.0,
        attack_range: None,
        fixed_damage: None,
        mp_cost: 8,
        hp_cost: 0,
        bullet_cost: 0,
        requirements: Vec::new(),
    }
}

fn preview_skill_entries() -> Vec<SkillUiEntry> {
    [
        (1_001_004, "Power Strike", 1),
        (1_001_005, "Slash Blast", 5),
        (1_001_003, "Iron Body", 3),
        (1_000_003, "Improved MaxHP", 2),
        (1_000_002, "Improved Recovery", 1),
    ]
    .into_iter()
    .map(|(id, name, level)| {
        let mut entry = preview_skill_entry();
        entry.id = id;
        entry.name = name.to_owned();
        entry.level = level;
        entry
    })
    .collect()
}

fn fixture_look() -> openstory_protocol::CharacterLook {
    openstory_protocol::CharacterLook {
        skin: 0,
        face_id: 20_000,
        hair_id: 30_030,
        equips: vec![
            (-5, 1_040_002),
            (-6, 1_060_002),
            (-7, 1_072_001),
            (-11, 1_302_000),
        ],
        weapon_id: 1_302_000,
        ..Default::default()
    }
}

fn field_sprite(
    font: &fontdue::Font,
    text: &str,
    x: f32,
    y: f32,
    texture_key: &str,
) -> DecodedSprite {
    DecodedSprite {
        texture_key: texture_key.to_owned(),
        rect: SpriteRect {
            x,
            y,
            width: FIELD_WIDTH as f32,
            height: FIELD_HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: FIELD_WIDTH,
        texture_height: FIELD_HEIGHT,
        pixels: rasterize_field(font, text, false).into(),
    }
}

fn compose_scene(sprites: &[DecodedSprite], width: u32, height: u32) -> Vec<u8> {
    let mut canvas = vec![0_u8; width as usize * height as usize * 4];
    for sprite in sprites {
        composite_sprite(&mut canvas, width, height, sprite);
    }
    canvas
}

fn composite_sprite(canvas: &mut [u8], width: u32, height: u32, sprite: &DecodedSprite) {
    let rect = sprite.rect;
    if !rect.x.is_finite()
        || !rect.y.is_finite()
        || !rect.width.is_finite()
        || !rect.height.is_finite()
        || rect.width <= 0.0
        || rect.height <= 0.0
        || sprite.texture_width == 0
        || sprite.texture_height == 0
    {
        return;
    }
    let expected = sprite.texture_width as usize * sprite.texture_height as usize * 4;
    if sprite.pixels.len() != expected {
        return;
    }
    let left = rect.x.floor().max(0.0) as u32;
    let top = rect.y.floor().max(0.0) as u32;
    let right = (rect.x + rect.width).ceil().clamp(0.0, width as f32) as u32;
    let bottom = (rect.y + rect.height).ceil().clamp(0.0, height as f32) as u32;
    if left >= right || top >= bottom {
        return;
    }

    for y in top..bottom {
        let v = ((y as f32 + 0.5 - rect.y) / rect.height).clamp(0.0, 1.0);
        let source_y = (v * sprite.texture_height as f32) as u32;
        for x in left..right {
            let u = ((x as f32 + 0.5 - rect.x) / rect.width).clamp(0.0, 1.0);
            let source_x = (u * sprite.texture_width as f32) as u32;
            let source = (source_y.min(sprite.texture_height - 1) as usize
                * sprite.texture_width as usize
                + source_x.min(sprite.texture_width - 1) as usize)
                * 4;
            let destination = (y as usize * width as usize + x as usize) * 4;
            blend_pixel(
                &mut canvas[destination..destination + 4],
                &sprite.pixels[source..source + 4],
                sprite.rect.blend_mode,
            );
        }
    }
}

fn blend_pixel(destination: &mut [u8], source: &[u8], mode: BlendMode) {
    let alpha = u32::from(source[3]);
    if alpha == 0 {
        return;
    }
    match mode {
        BlendMode::Alpha => {
            let inverse = 255 - alpha;
            for channel in 0..3 {
                destination[channel] = ((u32::from(source[channel]) * alpha
                    + u32::from(destination[channel]) * inverse)
                    / 255) as u8;
            }
            destination[3] = (alpha + u32::from(destination[3]) * inverse / 255) as u8;
        }
        BlendMode::Additive => {
            for channel in 0..3 {
                let contribution = u32::from(source[channel]) * alpha / 255;
                destination[channel] =
                    (u32::from(destination[channel]) + contribution).min(255) as u8;
            }
            destination[3] = destination[3].max(source[3]);
        }
    }
}

fn draw_rect_outline(pixels: &mut [u8], width: u32, height: u32, rect: SpriteRect, color: [u8; 4]) {
    if !rect.x.is_finite() || !rect.y.is_finite() || rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let left = rect.x.floor() as i32;
    let top = rect.y.floor() as i32;
    let right = (rect.x + rect.width).ceil() as i32 - 1;
    let bottom = (rect.y + rect.height).ceil() as i32 - 1;
    for x in left..=right {
        put_pixel(pixels, width, height, x, top, color);
        put_pixel(pixels, width, height, x, bottom, color);
    }
    for y in top..=bottom {
        put_pixel(pixels, width, height, left, y, color);
        put_pixel(pixels, width, height, right, y, color);
    }
}

fn put_pixel(pixels: &mut [u8], width: u32, height: u32, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }
    let index = (y as usize * width as usize + x as usize) * 4;
    blend_pixel(&mut pixels[index..index + 4], &color, BlendMode::Alpha);
}

fn hitbox_color(index: usize) -> [u8; 4] {
    const COLORS: [[u8; 4]; 4] = [
        [32, 64, 255, 255],
        [32, 220, 48, 255],
        [255, 128, 24, 255],
        [220, 40, 220, 255],
    ];
    COLORS[index % COLORS.len()]
}

fn geometry_report(name: &str, scene: &LoginSceneAssets, width: u32, height: u32) -> String {
    let mut report = format!(
        "Screen: {name}\nCanvas: {width}x{height}\nSprites: {}\nInteractive regions: {}\n",
        scene.sprites.len(),
        scene.actions.len()
    );
    let native_window_preview = matches!(
        name,
        "panel-keyboard-settings"
            | "panel-keyboard-settings-empty"
            | "panel-keyboard-settings-capture"
            | "panel-keyboard-settings-scroll"
            | "panel-keyboard-settings-quickslots"
            | "panel-skill-macros"
            | "panel-item-inventory"
            | "panel-equipment-inventory"
            | "panel-skill-inventory"
            | "panel-pet-equipment"
            | "panel-pet-equipment-empty"
            | "panel-pet-equipment-scroll"
            | "panel-npc-store"
            | "panel-npc-store-sell"
            | "panel-npc-store-confirmation"
            | "page-cash-shop"
            | "panel-monster-book"
            | "panel-stats"
            | "panel-stats-detail"
            | "panel-quests"
            | "panel-quests-detail"
            | "hud-chat"
            | "panel-party"
            | "panel-buddy"
            | "panel-guild"
            | "panel-alliance"
            | "panel-audio-settings"
    );
    if native_window_preview {
        let nx_sprite_count = scene
            .sprites
            .iter()
            .filter(|sprite| sprite.texture_key.starts_with("nx-window:"))
            .count();
        report.push_str(&format!("NX native window layers: {nx_sprite_count}\n"));
        let rendered_panel_count = scene
            .sprites
            .iter()
            .filter(|sprite| sprite.texture_key.starts_with("rendered-window:"))
            .count();
        if rendered_panel_count > 0 {
            report.push_str(&format!(
                "Live Rust-rendered panels: {rendered_panel_count}\n"
            ));
        }
    }
    let mut outside_count = 0;
    for (index, sprite) in scene.sprites.iter().enumerate() {
        let is_nx_ui_sprite = sprite.texture_key.starts_with("nx-window:");
        let is_rendered_ui_panel = sprite.texture_key.starts_with("rendered-window:");
        let is_audited_ui_sprite = is_nx_ui_sprite || is_rendered_ui_panel;
        if native_window_preview && !is_audited_ui_sprite {
            continue;
        }
        let Some(visible_rect) = visible_sprite_rect(sprite) else {
            continue;
        };
        if native_window_preview && is_nx_ui_sprite {
            report.push_str(&format!(
                "NX native layer {:?}: {:.1},{:.1} {:.1}x{:.1}\n",
                sprite.texture_key,
                visible_rect.x,
                visible_rect.y,
                visible_rect.width,
                visible_rect.height
            ));
        } else if native_window_preview && is_rendered_ui_panel {
            report.push_str(&format!(
                "Live Rust panel {:?}: {:.1},{:.1} {:.1}x{:.1}\n",
                sprite.texture_key,
                visible_rect.x,
                visible_rect.y,
                visible_rect.width,
                visible_rect.height
            ));
        }
        if !rect_inside(visible_rect, width, height) {
            outside_count += 1;
            report.push_str(&format!(
                "visible sprite pixels {index} {:?}: {:.1},{:.1} {:.1}x{:.1} outside canvas\n",
                sprite.texture_key,
                visible_rect.x,
                visible_rect.y,
                visible_rect.width,
                visible_rect.height
            ));
        }
    }
    for (index, action) in scene.actions.iter().enumerate() {
        if !rect_inside(action.rect, width, height) {
            report.push_str(&format!(
                "interactive region {index}: {:.1},{:.1} {:.1}x{:.1} outside canvas\n",
                action.rect.x, action.rect.y, action.rect.width, action.rect.height
            ));
        }
    }
    if outside_count == 0
        && scene
            .actions
            .iter()
            .all(|action| rect_inside(action.rect, width, height))
    {
        let subject = if native_window_preview {
            "NX native window bounds and interactive regions"
        } else {
            "All sprite bounds and interactive regions"
        };
        report.push_str(&format!("{subject} fit the logical canvas.\n"));
    }
    report.push('\n');
    report
}

fn visible_sprite_rect(sprite: &DecodedSprite) -> Option<SpriteRect> {
    let expected = sprite.texture_width as usize * sprite.texture_height as usize * 4;
    if sprite.pixels.len() != expected || sprite.texture_width == 0 || sprite.texture_height == 0 {
        return None;
    }
    let mut bounds = None::<(u32, u32, u32, u32)>;
    for y in 0..sprite.texture_height {
        for x in 0..sprite.texture_width {
            let alpha_index = (y as usize * sprite.texture_width as usize + x as usize) * 4 + 3;
            if sprite.pixels[alpha_index] == 0 {
                continue;
            }
            bounds = Some(match bounds {
                Some((left, top, right, bottom)) => {
                    (left.min(x), top.min(y), right.max(x), bottom.max(y))
                }
                None => (x, y, x, y),
            });
        }
    }
    let (left, top, right, bottom) = bounds?;
    Some(SpriteRect {
        x: sprite.rect.x + left as f32 / sprite.texture_width as f32 * sprite.rect.width,
        y: sprite.rect.y + top as f32 / sprite.texture_height as f32 * sprite.rect.height,
        width: (right - left + 1) as f32 / sprite.texture_width as f32 * sprite.rect.width,
        height: (bottom - top + 1) as f32 / sprite.texture_height as f32 * sprite.rect.height,
        blend_mode: sprite.rect.blend_mode,
    })
}

fn rect_inside(rect: SpriteRect, width: u32, height: u32) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width >= 0.0
        && rect.height >= 0.0
        && rect.x >= 0.0
        && rect.y >= 0.0
        && rect.x + rect.width <= width as f32
        && rect.y + rect.height <= height as f32
}

fn scale_to_viewport(source: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut output = vec![0_u8; width as usize * height as usize * 4];
    let Some(viewport) =
        Viewport::fit(LogicalSize::LEGACY_1024X768, PhysicalSize { width, height })
    else {
        return output;
    };
    let left = viewport.x.floor().max(0.0) as u32;
    let top = viewport.y.floor().max(0.0) as u32;
    let right = (viewport.x + viewport.width).ceil().min(width as f32) as u32;
    let bottom = (viewport.y + viewport.height).ceil().min(height as f32) as u32;
    for y in top..bottom {
        let source_y = (((y as f32 + 0.5 - viewport.y) / viewport.scale).floor() as u32)
            .min(CANVAS_HEIGHT - 1);
        for x in left..right {
            let source_x = (((x as f32 + 0.5 - viewport.x) / viewport.scale).floor() as u32)
                .min(CANVAS_WIDTH - 1);
            let source_index = (source_y as usize * CANVAS_WIDTH as usize + source_x as usize) * 4;
            let destination_index = (y as usize * width as usize + x as usize) * 4;
            output[destination_index..destination_index + 4]
                .copy_from_slice(&source[source_index..source_index + 4]);
        }
    }
    output
}

fn preview_path(output_dir: &Path, screen: &str, width: u32, height: u32) -> PathBuf {
    output_dir.join(format!("{}-{width}x{height}.png", preview_stem(screen)))
}

fn preview_stem(screen: &str) -> String {
    screen.to_owned()
}

fn write_png(path: &Path, width: u32, height: u32, bgra: &[u8]) -> io::Result<()> {
    let expected = width as usize * height as usize * 4;
    if bgra.len() != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "BGRA image dimensions do not match pixel data",
        ));
    }
    let mut rgba = Vec::with_capacity(expected);
    for pixel in bgra.chunks_exact(4) {
        rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }
    let file = fs::File::create(path)?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    let mut writer = encoder
        .write_header()
        .map_err(|error| io::Error::other(error.to_string()))?;
    writer
        .write_image_data(&rgba)
        .map_err(|error| io::Error::other(error.to_string()))
}

#[cfg(test)]
#[path = "combat_preview_tests.rs"]
mod combat_preview_tests;

#[cfg(test)]
#[path = "portal_preview_tests.rs"]
mod portal_preview_tests;
