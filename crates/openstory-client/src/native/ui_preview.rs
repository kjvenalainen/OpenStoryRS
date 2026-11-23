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
const PREVIEW_DROP_STACK_COUNT: u16 = 37;
const PREVIEW_MESO_DROP_BALANCE: i64 = 3_750;

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
        if *name == "panel-menu-comparison" {
            write_menu_comparison_preview(
                output_dir, map, legacy_ui, ui, characters, npcs, skills, etc, items, mobs, quests,
                strings, font,
            )?;
            continue;
        }
        if name.starts_with("page-cash-shop") {
            let variant = match *name {
                "page-cash-shop" => "catalog",
                "page-cash-shop-remove-all" => "remove-all",
                "page-cash-shop-tooltip" => "catalog-tooltip",
                "page-cash-shop-locker" => "locker",
                "page-cash-shop-coupon" => "coupon",
                "page-cash-shop-coupon-pending" => "coupon-pending",
                "page-cash-shop-coupon-error" => "coupon-error",
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
        } else if *name == "page-map-name-tags" {
            build_map_name_tags_preview(map, ui, characters, npcs, items, strings, font)?
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
        } else if let Some(tab) = equipment_preview_tab(name) {
            build_equipment_panel_scene(
                name,
                tab,
                (*name == "panel-equipment-inventory-scroll-drop").then_some(2_043_000),
                map,
                legacy_ui,
                characters,
                items,
                strings,
                font,
            )?
        } else if *name == "panel-pet-item-filter" {
            build_pet_item_filter_scene(map, legacy_ui, characters, items, strings, font)?
        } else if name.starts_with("panel-merchant-") || *name == "panel-personal-shop" {
            build_merchant_panel_scene(name, map, legacy_ui, characters, items, strings, font)?
        } else if name.starts_with("panel-trade-room") {
            build_trade_panel_scene(name, map, legacy_ui, characters, items, strings, font)?
        } else if name.starts_with("panel-npc-shop") {
            build_npc_shop_panel_scene(
                name, map, legacy_ui, npcs, characters, items, strings, font,
            )?
        } else if *name == "panel-npc-storage" {
            build_storage_panel_scene(map, legacy_ui, npcs, characters, items, strings, font)?
        } else if name.starts_with("panel-item-tooltip") {
            build_item_tooltip_scene(name, map, characters, items, strings, font)?
        } else if *name == "panel-quest-helper" {
            build_quest_helper_panel_scene(map, legacy_ui, characters, font)?
        } else if *name == "panel-quest-helper-with-quest-log" {
            build_quest_helper_with_quest_log_scene(
                map, legacy_ui, quests, strings, characters, npcs, font,
            )?
        } else if matches!(
            *name,
            "panel-quests"
                | "panel-quests-available"
                | "panel-quests-collapsed"
                | "panel-quests-detail"
                | "panel-quests-detail-flow"
                | "panel-quests-detail-scrolled"
        ) {
            build_quest_log_panel_scene(
                name, map, legacy_ui, quests, strings, characters, npcs, font,
            )?
        } else if matches!(
            *name,
            "panel-npc-chat"
                | "panel-npc-chat-inventory"
                | "panel-npc-chat-long"
                | "panel-npc-chat-long-scrolled"
                | "panel-npc-chat-tutorial"
                | "panel-npc-chat-menu"
                | "panel-npc-chat-option"
                | "panel-npc-chat-multiline-options"
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
                    "panel-npc-chat-long" | "panel-npc-chat-long-scrolled" => (2004, 0, None,
                        (1..=18).map(|index| format!("Line {index}: A longer dialogue should remain inside the window.\n")).collect()),
                    "panel-npc-chat-unimplemented" => (2004, 0, None,
                        super::client_npc_interaction::UNIMPLEMENTED_NPC_TEXT.to_owned()),
                    "panel-npc-chat-menu" => {
                        let choices = vec![NpcQuestChoice { quest_id: 1038, side: 1, ready: true },
                            NpcQuestChoice { quest_id: 1040, side: 0, ready: true }];
                        let (text, _) = super::client_npc_interaction::npc_quest_menu(quests, 1, choices);
                        (12000, 4, None, text)
                    }
                    "panel-npc-chat-option" => (fixture_named_id(strings, "Npc.img", "Robin", 2007), 4, None,
                        "That is correct! You can only start raising your HP and MP using your AP after making the first job advancement. You can't do that right now. Next question! You may be a beginner right now, but after you reach a certain level, you can make a job advancement. Normally, you have to be at Level 10 to choose a job. Which one do you want?\\n\\n#L0#Warrior#l\\n#L1#Magician#l\\n#L2#Bowman#l\\n#L3#Thief#l".to_owned()),
                    "panel-npc-chat-multiline-options" => (fixture_named_id(strings, "Npc.img", "Robin", 2007), 4, None,
                        "Choose a topic:\\n#L0#The Reason Behind the Mushroom\\nStudies#l\\n#L1#Talk#l".to_owned()),
                    "panel-npc-chat-quiz" => (2003, 4, None,
                        quest_dialogue_pages(quests, "Say.img/1036/1")[2].clone()),
                    "panel-npc-chat-feedback" => (12101, 0, None,
                        nx_string_value(quests, "Say.img/1009/1/stop/0/1").unwrap()),
                    "panel-npc-chat-text-input" => (2004, 2, None,
                        "What would you like to say?".to_owned()),
                    "panel-npc-chat-number-input" => (2004, 3, None,
                        "So, you want me to make some Adamantium Plates? In that case, how many do you want me to make?".to_owned()),
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
                let npc_id = if *name == "panel-npc-chat-number-input" {
                    fixture_named_id(strings, "Npc.img", "Mr. Thunder", 2004)
                } else {
                    npc_id
                };
                let number_bounds = (*name == "panel-npc-chat-number-input").then_some((1, 1, 7));
                let (pixels, panel, _, _, _) = render_npc_dialogue(
                    font,
                    legacy_ui,
                    npcs,
                    strings,
                    npc_id,
                    0,
                    message_type,
                    navigation,
                    number_bounds,
                    &plain,
                    if *name == "panel-npc-chat-long-scrolled" {
                        usize::MAX
                    } else {
                        0
                    },
                    &icons,
                    &selections,
                    0,
                    0,
                    None,
                    match *name {
                        "panel-npc-chat-text-input" => "Hello Maple World",
                        "panel-npc-chat-number-input" => "1",
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
            if *name == "panel-npc-chat-inventory" {
                let inventory = preview_inventory();
                let pixels = super::render_inventory_panel(
                    font,
                    &inventory,
                    openstory_protocol::InventoryType::Use,
                    0,
                    None,
                    None,
                    None,
                    None,
                    legacy_ui,
                    items,
                    characters,
                    strings,
                    &mut HashMap::new(),
                    &mut HashMap::new(),
                );
                scene.sprites.push(DecodedSprite {
                    texture_key: "native-npc-chat-inventory-preview".to_owned(),
                    rect: SpriteRect {
                        x: super::INVENTORY_PANEL_X,
                        y: super::INVENTORY_PANEL_Y,
                        width: super::INVENTORY_PANEL_WIDTH as f32,
                        height: super::INVENTORY_PANEL_HEIGHT as f32,
                        blend_mode: BlendMode::Alpha,
                    },
                    depth_order: 1,
                    texture_width: super::INVENTORY_PANEL_WIDTH,
                    texture_height: super::INVENTORY_PANEL_HEIGHT,
                    pixels: Arc::from(pixels),
                });
            }
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
        } else if name.starts_with("panel-rps") {
            let mut game = RpsGameRuntime::new();
            game.phase = RpsGamePhase::Result;
            game.player_choice = Some(0);
            game.opponent_choice = Some(match *name {
                "panel-rps-lose" => 1,
                "panel-rps-draw" => 0,
                _ => 2,
            });
            game.result = Some(match *name {
                "panel-rps-lose" => 0,
                "panel-rps-draw" => 2,
                _ => 1,
            });
            if *name == "panel-rps-selecting" {
                game.phase = RpsGamePhase::Selecting;
            }
            game.wins = 1;
            if *name == "panel-rps-waiting" {
                game.phase = RpsGamePhase::Waiting;
                game.wins = 0;
            }
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
            let (dialog, _) = build_quit_confirmation_sprites(legacy_ui, font)
                .or_else(|_| build_quit_confirmation_sprites(ui, font))?;
            sprites.extend(dialog);
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
            "panel-character-delete-confirmation" | "panel-character-delete-final-confirmation"
        ) {
            let character_list = openstory_protocol::CharacterList {
                channel_id: 0,
                characters: vec![openstory_protocol::CharacterInfo {
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
                    look: fixture_look(),
                }],
                pic_state: 2,
                slots: 6,
            };
            let mut scene = load_character_selection_scene(
                map,
                legacy_ui,
                characters,
                items,
                &character_list,
                0,
                0,
                None,
                font,
                None,
            )?;
            scene.sprites.push(DecodedSprite {
                texture_key: format!("native-character-delete-preview:{name}:dim"),
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
                pixels: Arc::from([0_u8, 0, 0, 140]),
            });
            let prompt = if *name == "panel-character-delete-confirmation" {
                "This will delete your character, do you want to continue?"
            } else {
                "Are you sure you want to delete your character?"
            };
            let (dialog, _) = build_character_delete_confirmation_sprites(legacy_ui, font, prompt)
                .or_else(|_| build_character_delete_confirmation_sprites(ui, font, prompt))?;
            scene.sprites.extend(dialog);
            scene
        } else if name.starts_with("panel-server-notice") || *name == "panel-tos-acceptance" {
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
            } else if name.starts_with("panel-server-notice-long") {
                "The world will restart shortly for maintenance. Please finish your current activity and find a safe place. Characters will be disconnected during the restart. Your progress is saved by the server. Visit the announcements page for more information. Thank you for playing."
            } else {
                "The server will restart in five minutes. Please move to a safe place."
            };
            let page = usize::from(name.ends_with("-page2"));
            let (width, height, pixels, panel, _, _) =
                build_server_notice_art(legacy_ui, font, message, page)
                    .or_else(|_| build_server_notice_art(ui, font, message, page))?;
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
            if *name == "panel-game-menu"
                && let Some(menu) = scene
                    .sprites
                    .iter()
                    .find(|sprite| sprite.texture_key == "rendered-window:game-menu")
            {
                let regions = (0..GAME_MENU_ROWS.len())
                    .map(game_menu_row)
                    .chain([GAME_MENU_CLOSE]);
                for (index, rect) in regions.enumerate() {
                    draw_rect_outline(
                        &mut image,
                        CANVAS_WIDTH,
                        CANVAS_HEIGHT,
                        SpriteRect {
                            x: menu.rect.x + rect.x,
                            y: menu.rect.y + rect.y,
                            width: rect.width,
                            height: rect.height,
                            blend_mode: BlendMode::Alpha,
                        },
                        hitbox_color(index),
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
            if name.starts_with("hud-quickslot") {
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
            if equipment_preview_tab(name).is_some()
                || matches!(
                    *name,
                    "panel-skill-inventory"
                        | "panel-skill-inventory-disabled"
                        | "panel-skill-inventory-beginner"
                        | "panel-skill-inventory-beginner-job"
                        | "panel-stats"
                        | "panel-stats-auto-assign"
                        | "panel-stats-detail"
                )
            {
                let (x, y, regions) = if *name == "panel-stats-auto-assign" {
                    let dialog = status_auto_assign_dialog_rect();
                    let [accept, cancel] = status_auto_assign_buttons();
                    (
                        dialog.x,
                        dialog.y,
                        vec![accept, cancel, STATUS_AUTO_ASSIGN_CLOSE],
                    )
                } else if matches!(
                    *name,
                    "panel-skill-inventory"
                        | "panel-skill-inventory-disabled"
                        | "panel-skill-inventory-beginner"
                        | "panel-skill-inventory-beginner-job"
                ) {
                    let mut regions = (0..3).map(ui_foundation::skill_tab).collect::<Vec<_>>();
                    regions
                        .extend((0..SKILL_BOOK_VISIBLE_ROWS).map(ui_foundation::skill_sp_button));
                    regions.push(ui_foundation::SKILL_MACRO_BUTTON);
                    regions.push(ui_foundation::SKILL_CLOSE_BUTTON);
                    (SKILL_BOOK_PANEL_X, SKILL_BOOK_PANEL_Y, regions)
                } else if let Some(tab) = equipment_preview_tab(name) {
                    (
                        EQUIPMENT_INVENTORY_PANEL_X,
                        EQUIPMENT_INVENTORY_PANEL_Y,
                        tab.hitboxes(&preview_equipment_tabs_inventory(name)),
                    )
                } else {
                    let mut regions = ui_foundation::status_ap_buttons()
                        .map(|(_, rect)| rect)
                        .to_vec();
                    regions.extend([
                        ui_foundation::STATUS_CLOSE_BUTTON,
                        ui_foundation::STATUS_AUTO_BUTTON,
                        ui_foundation::STATUS_DETAIL_BUTTON,
                    ]);
                    (STATUS_PANEL_X, STATUS_PANEL_Y, regions)
                };
                for (index, rect) in regions.into_iter().enumerate() {
                    draw_rect_outline(
                        &mut image,
                        CANVAS_WIDTH,
                        CANVAS_HEIGHT,
                        SpriteRect {
                            x: x + rect.x,
                            y: y + rect.y,
                            width: rect.width,
                            height: rect.height,
                            blend_mode: BlendMode::Alpha,
                        },
                        hitbox_color(index),
                    );
                }
            }
            if *name == "panel-pet-item-filter" {
                for (index, rect) in ui_pet_item_filter::hitboxes().into_iter().enumerate() {
                    draw_rect_outline(
                        &mut image,
                        CANVAS_WIDTH,
                        CANVAS_HEIGHT,
                        SpriteRect {
                            x: PET_ITEM_FILTER_PANEL_X + rect.x,
                            y: PET_ITEM_FILTER_PANEL_Y + rect.y,
                            width: rect.width,
                            height: rect.height,
                            blend_mode: BlendMode::Alpha,
                        },
                        hitbox_color(index),
                    );
                }
            }
            if *name == "panel-item-inventory" {
                let controls = [
                    ui_foundation::INVENTORY_GATHER_BUTTON,
                    ui_foundation::INVENTORY_SORT_BUTTON,
                    ui_foundation::INVENTORY_CLOSE_BUTTON,
                ];
                for (index, rect) in (0..5)
                    .map(ui_foundation::inventory_tab)
                    .chain(controls)
                    .enumerate()
                {
                    draw_rect_outline(
                        &mut image,
                        CANVAS_WIDTH,
                        CANVAS_HEIGHT,
                        SpriteRect {
                            x: super::INVENTORY_PANEL_X + rect.x,
                            y: super::INVENTORY_PANEL_Y + rect.y,
                            width: rect.width,
                            height: rect.height,
                            blend_mode: BlendMode::Alpha,
                        },
                        hitbox_color(index),
                    );
                }
                for row in 0..INVENTORY_SLOT_ROWS {
                    for column in 0..4 {
                        let index = 5 + row * 4 + column;
                        let cell = ui_foundation::INVENTORY_GRID.cell(row * 4 + column);
                        draw_rect_outline(
                            &mut image,
                            CANVAS_WIDTH,
                            CANVAS_HEIGHT,
                            SpriteRect {
                                x: super::INVENTORY_PANEL_X + cell.x,
                                y: super::INVENTORY_PANEL_Y + cell.y,
                                width: cell.width,
                                height: cell.height,
                                blend_mode: BlendMode::Alpha,
                            },
                            hitbox_color(index),
                        );
                    }
                }
                let meso = ui_foundation::INVENTORY_MESO_BUTTON;
                draw_rect_outline(
                    &mut image,
                    CANVAS_WIDTH,
                    CANVAS_HEIGHT,
                    SpriteRect {
                        x: super::INVENTORY_PANEL_X + meso.x,
                        y: super::INVENTORY_PANEL_Y + meso.y,
                        width: meso.width,
                        height: meso.height,
                        blend_mode: BlendMode::Alpha,
                    },
                    hitbox_color(29),
                );
            }
            if matches!(
                *name,
                "panel-quests"
                    | "panel-quests-available"
                    | "panel-quests-collapsed"
                    | "panel-quests-detail"
                    | "panel-quests-detail-flow"
                    | "panel-quests-detail-scrolled"
            ) {
                let mut bounds = vec![ui_foundation::QUEST_LIST_CLOSE_BUTTON];
                bounds.extend((0..3).map(ui_foundation::quest_tab));
                if *name != "panel-quests-collapsed" {
                    bounds.extend([
                        ui_foundation::QUEST_DETAIL_CLOSE_BUTTON,
                        ui_foundation::QUEST_HELPER_BUTTON,
                        ui_foundation::QUEST_TELEPORT_BUTTON,
                    ]);
                    if *name == "panel-quests" {
                        bounds.push(ui_foundation::QUEST_FORFEIT_BUTTON);
                    }
                }
                for (index, bounds) in bounds.into_iter().enumerate() {
                    draw_rect_outline(
                        &mut image,
                        CANVAS_WIDTH,
                        CANVAS_HEIGHT,
                        SpriteRect {
                            x: super::QUEST_LOG_PANEL_X + bounds.x,
                            y: super::QUEST_LOG_PANEL_Y + bounds.y,
                            width: bounds.width,
                            height: bounds.height,
                            blend_mode: BlendMode::Alpha,
                        },
                        hitbox_color(3 + super::QUEST_LOG_VISIBLE_ROWS + index),
                    );
                }
                for row in 0..super::QUEST_LOG_VISIBLE_ROWS {
                    let cell = ui_foundation::QUEST_GRID.cell(row);
                    draw_rect_outline(
                        &mut image,
                        CANVAS_WIDTH,
                        CANVAS_HEIGHT,
                        SpriteRect {
                            x: super::QUEST_LOG_PANEL_X + cell.x,
                            y: super::QUEST_LOG_PANEL_Y + cell.y,
                            width: cell.width,
                            height: cell.height,
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

fn write_menu_comparison_preview(
    output_dir: &Path,
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
    const COLUMNS: usize = 3;
    const ROWS: usize = 3;
    const CELL_WIDTH: u32 = 680;
    const CELL_HEIGHT: u32 = 500;
    const GAP: u32 = 32;
    const HEADER_HEIGHT: i32 = 28;
    const PANEL_PADDING: u32 = 20;

    let panels = vec![
        (
            "Item Inventory",
            build_inventory_panel_scene(
                "panel-item-inventory",
                map,
                legacy_ui,
                characters,
                items,
                strings,
                font,
            )?,
            "nx-window:live-inventory",
        ),
        (
            "Equipment Inventory",
            build_equipment_panel_scene(
                "panel-equipment-inventory",
                EquipmentTab::Equip,
                None,
                map,
                ui,
                characters,
                items,
                strings,
                font,
            )?,
            "nx-window:live-equipment",
        ),
        (
            "Equipment Inventory (Scroll Drop)",
            build_equipment_panel_scene(
                "panel-equipment-inventory-scroll-drop",
                EquipmentTab::Equip,
                Some(2_043_000),
                map,
                ui,
                characters,
                items,
                strings,
                font,
            )?,
            "nx-window:live-equipment",
        ),
        (
            "Equipment Inventory (Pet)",
            build_equipment_panel_scene(
                "panel-equipment-inventory-pet",
                EquipmentTab::Pet,
                None,
                map,
                legacy_ui,
                characters,
                items,
                strings,
                font,
            )?,
            "nx-window:live-equipment",
        ),
        (
            "Skill Inventory",
            build_scene(
                "panel-skill-inventory",
                map,
                legacy_ui,
                ui,
                characters,
                items,
                skills,
                etc,
                strings,
                font,
            )?,
            "nx-window:rendered-v83-skills",
        ),
        (
            "Skill Macros",
            build_scene(
                "panel-skill-macros",
                map,
                legacy_ui,
                ui,
                characters,
                items,
                skills,
                etc,
                strings,
                font,
            )?,
            "skill-macros-preview",
        ),
        (
            "Character Stats",
            build_scene(
                "panel-stats",
                map,
                legacy_ui,
                ui,
                characters,
                items,
                skills,
                etc,
                strings,
                font,
            )?,
            "rendered-window:status-panel",
        ),
        (
            "Quest Log",
            build_quest_log_panel_scene(
                "panel-quests",
                map,
                legacy_ui,
                quests,
                strings,
                characters,
                npcs,
                font,
            )?,
            "nx-window:live-quest-log",
        ),
        (
            "Monster Book",
            build_monster_book_detail_scene(
                map,
                ui,
                items,
                characters,
                mobs,
                strings,
                font,
                usize::MAX,
            )?,
            "monster-book-detail-preview",
        ),
        (
            "Keyboard Settings",
            build_scene(
                "panel-keyboard-settings",
                map,
                legacy_ui,
                ui,
                characters,
                items,
                skills,
                etc,
                strings,
                font,
            )?,
            "key-config-preview",
        ),
    ];

    let width = CELL_WIDTH * COLUMNS as u32 + GAP * (COLUMNS as u32 + 1);
    let height = CELL_HEIGHT * ROWS as u32 + GAP * (ROWS as u32 + 1);
    let mut pixels = vec![0_u8; width as usize * height as usize * 4];
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[24, 24, 30, 255]);
    }

    for (index, (title, scene, texture_key)) in panels.into_iter().enumerate() {
        let row = index / COLUMNS;
        let column = index % COLUMNS;
        let cell_x = GAP + column as u32 * (CELL_WIDTH + GAP);
        let cell_y = GAP + row as u32 * (CELL_HEIGHT + GAP);
        fill_preview_rect(
            &mut pixels,
            width,
            height,
            cell_x,
            cell_y,
            CELL_WIDTH,
            CELL_HEIGHT,
            [42, 40, 36, 255],
        );
        fill_preview_rect(
            &mut pixels,
            width,
            height,
            cell_x,
            cell_y,
            CELL_WIDTH,
            1,
            [88, 84, 76, 255],
        );
        ui_foundation::draw_text_role(
            font,
            &mut pixels,
            width as usize,
            height as usize,
            title,
            (cell_x + 16) as i32,
            (cell_y + 7) as i32,
            ui_foundation::UiTextRole::Emphasis,
            CELL_WIDTH as usize - 32,
            HEADER_HEIGHT as usize,
            Some([226, 224, 215]),
        );

        let detail_key = (texture_key == "rendered-window:status-panel")
            .then_some("rendered-window:status-detail-panel");
        let sprites: Vec<_> = scene
            .sprites
            .into_iter()
            .filter(|sprite| {
                sprite.texture_key == texture_key || detail_key == Some(sprite.texture_key.as_str())
            })
            .collect();
        let first = sprites
            .first()
            .ok_or_else(|| format!("menu comparison is missing panel {texture_key:?}"))?;
        let origin = first.rect;
        let panel_width = sprites
            .iter()
            .map(|sprite| sprite.rect.x + sprite.rect.width - origin.x)
            .fold(origin.width, f32::max);
        let panel_height = sprites
            .iter()
            .map(|sprite| sprite.rect.y + sprite.rect.height - origin.y)
            .fold(origin.height, f32::max);
        let available_width = CELL_WIDTH.saturating_sub(PANEL_PADDING * 2);
        let available_height = CELL_HEIGHT.saturating_sub(HEADER_HEIGHT as u32 + PANEL_PADDING);
        let scale = (available_width as f32 / panel_width)
            .min(available_height as f32 / panel_height)
            .min(1.0);
        let draw_x = cell_x as f32 + (CELL_WIDTH as f32 - panel_width * scale) * 0.5;
        let draw_y = cell_y as f32
            + HEADER_HEIGHT as f32
            + (available_height as f32 - panel_height * scale) * 0.5;
        for mut sprite in sprites {
            sprite.rect.x = draw_x + (sprite.rect.x - origin.x) * scale;
            sprite.rect.y = draw_y + (sprite.rect.y - origin.y) * scale;
            sprite.rect.width *= scale;
            sprite.rect.height *= scale;
            composite_sprite(&mut pixels, width, height, &sprite);
        }
    }

    let image_path = output_dir.join("panel-menu-comparison.png");
    write_png(&image_path, width, height, &pixels)
        .map_err(|error| format!("could not write {}: {error}", image_path.display()))?;
    let report = format!(
        "Screen: panel-menu-comparison\nCanvas: {width}x{height} contact sheet\nPanels: Item Inventory, Equipment Inventory, Equipment Inventory (Pet), Skill Inventory, Skill Macros, Character Stats, Quest Log, Monster Book, Keyboard Settings\nPanel artwork uses native dimensions, scaled down only when a panel exceeds its comparison cell.\n"
    );
    let report_path = output_dir.join("panel-menu-comparison-layout.txt");
    fs::write(&report_path, report.as_bytes())
        .map_err(|error| format!("could not write {}: {error}", report_path.display()))?;
    println!("{report}");
    Ok(())
}

fn fill_preview_rect(
    pixels: &mut [u8],
    canvas_width: u32,
    canvas_height: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: [u8; 4],
) {
    let right = x.saturating_add(width).min(canvas_width);
    let bottom = y.saturating_add(height).min(canvas_height);
    for row in y.min(canvas_height)..bottom {
        for column in x.min(canvas_width)..right {
            let index = (row as usize * canvas_width as usize + column as usize) * 4;
            pixels[index..index + 4].copy_from_slice(&color);
        }
    }
}

fn preview_guild() -> openstory_protocol::GuildInfo {
    openstory_protocol::GuildInfo {
        id: 1,
        name: "Maple Heroes".to_owned(),
        rank_titles: ["Master", "Jr. Master", "Member", "Member", "New Member"].map(str::to_owned),
        members: (0..12)
            .map(|i| openstory_protocol::GuildMember {
                character_id: i + 1,
                name: if i == 0 {
                    "PreviewHero".to_owned()
                } else {
                    format!("Mapler{}", i)
                },
                job: 100,
                level: 30 + i,
                rank: if i == 0 { 1 } else { 3 },
                online: i % 3 != 0,
            })
            .collect(),
        capacity: 30,
        emblem: openstory_protocol::GuildEmblem::default(),
        notice: "Welcome! Meet the guild in Henesys for our weekend event.".to_owned(),
        gp: 12345,
        alliance_id: 7,
    }
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
        "panel-channel-switch" => {
            let mut sprites = gameplay_backdrop(map, character_archive, font)?;
            let channels = [0, 5, 15].map(|load| openstory_protocol::WorldChannel {
                name: String::new(),
                load,
            });
            let panel = super::channel_switch::render(ui, legacy_ui, font, 0, &channels, 0, 1)?;
            sprites.push(DecodedSprite {
                texture_key: "channel-switch-preview".to_owned(),
                rect: SpriteRect {
                    x: super::channel_switch::DEFAULT_X,
                    y: super::channel_switch::DEFAULT_Y,
                    width: panel.width as f32,
                    height: panel.height as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: panel.width,
                texture_height: panel.height,
                pixels: Arc::from(panel.pixels),
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
        "panel-system-options" | "panel-audio-settings" => {
            let mut sprites = gameplay_backdrop(map, character_archive, font)?;
            let pixels = render_options_panel(
                font,
                ui,
                AudioSettings {
                    music_volume: 50,
                    effects_volume: 70,
                    mute_music: name == "panel-audio-settings",
                    mute_effects: false,
                },
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
        "page-map-loading" => build_map_loading_preview(map, character_archive, font),
        "hud-bottom-bar" | "panel-game-menu" | "hud-gain-feed" | "page-first-map"
        | "page-second-map" => {
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
            let status_bar_preview = matches!(name, "hud-bottom-bar" | "hud-gain-feed");
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
            if name == "hud-gain-feed" {
                let quickslot_pixels = render_quickslot_panel_pixels(
                    quickslot_background_image(legacy_ui),
                    ui,
                    &DEFAULT_QUICKSLOT_KEYS,
                    &std::array::from_fn(|_| None),
                    &[0; 8],
                    &[0.0; 8],
                );
                sprites.push(DecodedSprite {
                    texture_key: "quickslots:gain-feed-preview".to_owned(),
                    rect: SpriteRect {
                        x: 865.0,
                        y: QUICKSLOT_PANEL_Y,
                        width: 151.0,
                        height: 80.0,
                        blend_mode: BlendMode::Alpha,
                    },
                    depth_order: 0,
                    texture_width: 151,
                    texture_height: 80,
                    pixels: Arc::from(quickslot_pixels),
                });
            }
            if name == "panel-game-menu" {
                let (x, y) = game_menu_position_for_layout(layout);
                sprites.push(DecodedSprite {
                    texture_key: "rendered-window:game-menu".to_owned(),
                    rect: SpriteRect {
                        x,
                        y,
                        width: GAME_MENU_WIDTH as f32,
                        height: GAME_MENU_HEIGHT as f32,
                        blend_mode: BlendMode::Alpha,
                    },
                    depth_order: 0,
                    texture_width: GAME_MENU_WIDTH,
                    texture_height: GAME_MENU_HEIGHT,
                    pixels: Arc::from(render_game_menu(ui, font)?),
                });
            }
            if name == "hud-gain-feed" {
                let mut icon_cache = HashMap::new();
                let item_id = 2_000_000;
                let item_icon =
                    inventory_item_icon(item_id, item_archive, character_archive, &mut icon_cache);
                let item_label = item_name(item_id, strings, &mut HashMap::new());
                let messages = [
                    (
                        format!("Gained an item: {item_label} (+3)"),
                        [255, 255, 255],
                        item_icon.as_ref(),
                    ),
                    (
                        "You have gained mesos (+2500)".to_owned(),
                        [255, 255, 255],
                        None,
                    ),
                    ("+ Bonus EXP (+250)".to_owned(), [255, 220, 44], None),
                    (
                        "You have gained experience (+1,250)".to_owned(),
                        [255, 220, 44],
                        None,
                    ),
                ];
                for (index, (text, color, icon)) in messages.into_iter().enumerate() {
                    let pixels = render_status_message_line(font, &text, color, icon);
                    sprites.push(DecodedSprite {
                        texture_key: format!("native-status-feed-preview:{index}"),
                        rect: SpriteRect {
                            x: STATUS_MESSAGE_RIGHT - STATUS_MESSAGE_WIDTH as f32,
                            y: status_message_y(name == "hud-gain-feed")
                                - index as f32 * STATUS_MESSAGE_ROW_STEP,
                            width: STATUS_MESSAGE_WIDTH as f32,
                            height: STATUS_MESSAGE_HEIGHT as f32,
                            blend_mode: BlendMode::Alpha,
                        },
                        depth_order: 0,
                        texture_width: STATUS_MESSAGE_WIDTH,
                        texture_height: STATUS_MESSAGE_HEIGHT,
                        pixels: Arc::from(pixels),
                    });
                }
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
                let mut look = fixture_look();
                look.masked_equips.push((1, 1_002_812));
                look.weapon_sticker_id = 1_702_000;
                look.pet_ids[0] = 5_000_000;
                // One selected character plus five open slots shows the native
                // parchment, selection glow, silhouettes, page arrows, and
                // cash equipment over the underlying gear.
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
                item_archive,
                &characters,
                0,
                0,
                None,
                font,
                None,
            )
        }
        "page-character-create-explorer"
        | "page-character-create-name"
        | "page-character-create-name-available"
        | "page-character-create-name-taken"
        | "page-character-create-error" => {
            let pools = load_character_creation_options(etc, 1, false)?;
            let mut creation = CharacterCreationState::customize(1, pools);
            if name != "page-character-create-explorer" {
                creation.name = "MapleHero".to_owned();
                creation.name_available = name == "page-character-create-name-available";
            }
            let mut scene = load_character_customization_scene(
                map,
                legacy_ui,
                character_archive,
                strings,
                &mut HashMap::new(),
                true,
                &creation,
                font,
                Some(("Bera", 4)),
            )?;
            let modal_message = match name {
                "page-character-create-name-taken" => Some(CREATION_NAME_TAKEN),
                "page-character-create-error" => Some("That name is not allowed."),
                _ => None,
            };
            if let Some(message) = modal_message {
                append_ui_solid_rect(
                    &mut scene.sprites,
                    "character-create-notice-dim",
                    0.0,
                    0.0,
                    CANVAS_WIDTH,
                    CANVAS_HEIGHT,
                    [0, 0, 0, 133],
                );
                let (width, height, pixels, panel, _, _) =
                    build_server_notice_art(legacy_ui, font, message, 0)?;
                scene.sprites.push(DecodedSprite {
                    texture_key: "character-create-notice-preview".to_owned(),
                    rect: panel,
                    depth_order: 0,
                    texture_width: width,
                    texture_height: height,
                    pixels: pixels.into(),
                });
                scene.actions.clear();
            }
            Ok(scene)
        }
        "panel-keyboard-settings"
        | "panel-keyboard-settings-empty"
        | "panel-keyboard-settings-capture"
        | "panel-keyboard-settings-scroll"
        | "panel-keyboard-settings-quickslots"
        | "panel-keyboard-settings-large-values"
        | "hud-quickslot"
        | "hud-quickslot-item-tooltip"
        | "hud-quickslot-skill-tooltip"
        | "hud-active-skill-tooltip"
        | "hud-quickslot-cooldown"
        | "panel-skill-macros"
        | "panel-skill-macros-drag"
        | "panel-skill-macros-book"
        | "panel-item-inventory"
        | "panel-inventory-drop"
        | "panel-meso-drop"
        | "panel-equipment-inventory"
        | "panel-equipment-inventory-scroll-drop"
        | "panel-skill-inventory"
        | "panel-skill-inventory-disabled"
        | "panel-skill-inventory-beginner"
        | "panel-skill-inventory-beginner-job"
        | "panel-skill-tooltip"
        | "panel-skill-tooltip-unlearned"
        | "panel-skill-tooltip-max-level"
        | "panel-skill-tooltip-edge"
        | "page-cash-shop"
        | "panel-monster-book"
        | "panel-stats"
        | "panel-stats-auto-assign"
        | "panel-stats-detail"
        | "panel-quests"
        | "panel-quests-available"
        | "panel-quests-collapsed"
        | "hud-chat"
        | "panel-whisper"
        | "panel-owl"
        | "panel-owl-results"
        | "panel-party"
        | "panel-buddy"
        | "panel-guild"
        | "panel-guild-populated"
        | "panel-guild-scrolled"
        | "panel-guild-bbs"
        | "panel-guild-bbs-draft"
        | "panel-guild-bbs-confirm-delete"
        | "panel-alliance"
        | "panel-alliance-populated"
        | "panel-alliance-scrolled" => build_native_window_scene(
            name,
            map,
            legacy_ui,
            ui,
            character_archive,
            item_archive,
            skill_archive,
            strings,
            font,
        ),
        "panel-character-info" | "panel-character-info-long" => build_character_info_scene(
            name,
            map,
            character_archive,
            item_archive,
            legacy_ui,
            strings,
            font,
        ),
        "panel-family"
        | "panel-family-populated"
        | "panel-family-invite"
        | "panel-family-precept"
        | "panel-family-input-error"
        | "panel-family-leave"
        | "panel-family-tree" => build_family_scene(
            map,
            character_archive,
            legacy_ui,
            font,
            name == "panel-family-tree",
            name,
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
    variant: &str,
) -> Result<LoginSceneAssets, String> {
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    let (pixels, width, height) = if tree {
        let entries = [
            (1, 0, "MapleElder"),
            (2, 1, "OakSenior"),
            (3, 2, "PreviewHero"),
            (10, 2, "Sibling"),
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
                member_counters: vec![openstory_protocol::FamilyChartCounter {
                    id_or_flag: -1,
                    count: 37,
                }],
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
        let populated = variant != "panel-family";
        let state = FamilyUiState {
            info: populated.then(|| openstory_protocol::FamilyInfo {
                current_reputation: 1250,
                total_reputation: 8750,
                todays_reputation: 85,
                junior_count: 1,
                juniors_allowed: 2,
                unknown: 0,
                leader_id: 1,
                name: "Maple Family".to_owned(),
                message: "Welcome to the family!".to_owned(),
                entitlement_usage: vec![openstory_protocol::FamilyEntitlementUsage {
                    ordinal: 0,
                    used: 1,
                }],
            }),
            privileges: if populated {
                vec![openstory_protocol::FamilyPrivilege {
                    kind: 0,
                    reputation_cost: 500,
                    usage_limit: 3,
                    name: "Teleport to Member".to_owned(),
                    description: "Teleport to a selected family member.".to_owned(),
                }]
            } else {
                Vec::new()
            },
            input: match variant {
                "panel-family-invite" | "panel-family-input-error" => Some(FamilyInput { kind: FamilyInputKind::InviteJunior, text: if variant == "panel-family-invite" { "MapleFriend".to_owned() } else { String::new() }, key_text_for_ime_dedup: None }),
                "panel-family-precept" => Some(FamilyInput { kind: FamilyInputKind::EditPrecepts, text: "A long family precept that remains readable while typing at the end of the field.".to_owned(), key_text_for_ime_dedup: None }),
                _ => None,
            },
            notice: (variant == "panel-family-input-error").then(|| "Enter a character name first.".to_owned()),
            confirm_leave: variant == "panel-family-leave",
            ..FamilyUiState::default()
        };
        (
            render_family_panel(font, ui_archive, &state),
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
    variant: &str,
    map_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    ui_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    let ui = CharacterInfoUi {
        name: if variant.ends_with("-long") {
            "AnExceptionallyLongCharacterName"
        } else {
            "PreviewHero"
        }
        .to_owned(),
        look: Some(preview_character().look),
        wishlist_offset: usize::from(variant.ends_with("-long")),
        selected_pet: 0,
        info: openstory_protocol::CharacterInfoReply {
            character_id: 2,
            level: 32,
            job: 100,
            fame: 7,
            married: false,
            guild: if variant.ends_with("-long") {
                "TheVeryLongGuildName"
            } else {
                "Maple Union"
            }
            .to_owned(),
            alliance: if variant.ends_with("-long") {
                "TheVeryLongAllianceName"
            } else {
                "Leaf Alliance"
            }
            .to_owned(),
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
            wishlist: if variant.ends_with("-long") {
                vec![1_000_002, 1_010_002, 1_050_002, 1_000_003, 1_010_003]
            } else {
                vec![1_000_002, 1_010_002, 1_050_002]
            },
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
    let pixels = render_character_info_panel(
        font,
        &ui,
        Some(&preview_character().look),
        ui_archive,
        character_archive,
        item_archive,
        string_archive,
        &mut HashMap::new(),
        &mut HashMap::new(),
    );
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
            name: None,
            sprite: None,
        },
        minimap::Marker {
            kind: minimap::MarkerKind::Npc,
            x: -data.center_x + 26.0 * units,
            y: -data.center_y + 29.0 * units,
            facing_left: false,
            name: Some("Maple Guide"),
            sprite: None,
        },
        minimap::Marker {
            kind: minimap::MarkerKind::Mob,
            x: -data.center_x + 42.0 * units,
            y: -data.center_y + 18.0 * units,
            facing_left: true,
            name: None,
            sprite: None,
        },
    ];
    let map_name =
        map_string_name(string_archive, map_id).unwrap_or_else(|| format!("Map {map_id}"));
    let street_name =
        map_string_street_name(string_archive, map_id).unwrap_or_else(|| map_name.clone());
    let panel = minimap::render(
        &data,
        ui_archive,
        &ui_assets,
        LogicalSize::LEGACY_1024X768,
        &map_name,
        &street_name,
        presentation,
        Some(minimap::WorldViewport {
            left: markers[0].x - 512.0,
            top: markers[0].y - GAMEPLAY_CAMERA_CENTER_Y,
            width: 1024.0,
            height: GAMEPLAY_VIEW_HEIGHT,
        }),
        &markers,
        font,
    )?;
    let mut sprites = vec![DecodedSprite {
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
    }];
    if let Some(npc_region) = panel.npc_hit_regions.first()
        && let Ok(tooltip) = minimap::render_npc_tooltip(&npc_region.name, font)
    {
        let (x, y) = minimap::npc_tooltip_position(
            npc_region.bounds,
            tooltip.width,
            tooltip.height,
            LogicalSize::LEGACY_1024X768,
        );
        sprites.push(DecodedSprite {
            texture_key: format!("minimap-preview:{name}:npc-tooltip"),
            rect: SpriteRect {
                x,
                y,
                width: tooltip.width as f32,
                height: tooltip.height as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 1,
            texture_width: tooltip.width,
            texture_height: tooltip.height,
            pixels: Arc::from(tooltip.pixels),
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

fn build_map_loading_preview(
    map_archive: &NxArchive,
    character_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    sprites.push(DecodedSprite {
        texture_key: "map-loading-preview:dim".to_owned(),
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
        pixels: Arc::from([0, 0, 0, 148]),
    });

    let phase = 4;
    for index in 0..12 {
        let angle = (index as f32 / 12.0) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let fade = 1.0 - ((index + 12 - phase) % 12) as f32 / 12.0;
        let size = 6.0 + fade * 2.0;
        sprites.push(DecodedSprite {
            texture_key: format!("map-loading-preview:spinner-{index}"),
            rect: SpriteRect {
                x: 512.0 + angle.cos() * 27.0 - size * 0.5,
                y: 354.0 + angle.sin() * 27.0 - size * 0.5,
                width: size,
                height: size,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: 1,
            texture_height: 1,
            pixels: Arc::from([255, 255, 255, (255.0 * (0.25 + fade * 0.75)) as u8]),
        });
    }
    let label = "Loading map...";
    let label_width = ui_foundation::measure_text(font, label, 18.0);
    sprites.push(DecodedSprite {
        texture_key: "map-loading-preview:label".to_owned(),
        rect: SpriteRect {
            x: 512.0 - label_width / 2.0,
            y: 397.0,
            width: 180.0,
            height: 28.0,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: 180,
        texture_height: 28,
        pixels: rasterize_label(font, label, 18.0, 180, 28).into(),
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

fn build_map_name_tags_preview(
    map_archive: &NxArchive,
    ui_archive: &NxArchive,
    character_archive: &NxArchive,
    npc_archive: &NxArchive,
    item_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let mut sprites = gameplay_backdrop_for_map(map_archive, character_archive, font, 10_000)?;
    let mut look = fixture_look();
    look.equips.push((-12, 1_112_101));
    append_map_name_tag_sprite(
        &mut sprites,
        font,
        ui_archive,
        Some(character_archive),
        Some(&look),
        "Cash Tag Hero",
        420.0,
        GAMEPLAY_CAMERA_CENTER_Y,
        MapNameTagKind::Character,
    );

    let pet_center_x = 560.0;
    if let Some(pet_item_id) = item_archive
        .root()
        .resolve("Pet")
        .ok()
        .flatten()
        .and_then(|pets| pets.children().ok())
        .and_then(|pets| {
            pets.into_iter().find_map(|pet| {
                let name = pet.name().ok()?;
                name.strip_suffix(".img")?.parse::<i32>().ok()
            })
        })
    {
        let mut pet_sprites = Vec::new();
        let pet_animations = append_pet_animations(
            &mut pet_sprites,
            item_archive,
            pet_item_id,
            &mut MapArtCache::default(),
        )?;
        if let Some(frame) = pet_animations
            .get(&1)
            .and_then(|frames| frames.first())
            .or_else(|| {
                (0..=6)
                    .find_map(|state| pet_animations.get(&state).and_then(|frames| frames.first()))
            })
        {
            let frame_start = frame.sprites.start;
            let frame_end = frame.sprites.end;
            for mut sprite in pet_sprites
                .into_iter()
                .skip(frame_start)
                .take(frame_end - frame_start)
            {
                sprite.rect.x += pet_center_x - 512.0;
                sprites.push(sprite);
            }
        }
    }
    append_map_name_tag_sprite(
        &mut sprites,
        font,
        ui_archive,
        None,
        None,
        "Bobby",
        pet_center_x,
        GAMEPLAY_CAMERA_CENTER_Y,
        MapNameTagKind::Pet,
    );

    let npc_id = fixture_named_id(string_archive, "Npc.img", "Silver", 1_001_000);
    let mut npc_sprites = Vec::new();
    let (frames, _) = append_npc_animations(
        &mut npc_sprites,
        npc_archive,
        npc_id,
        &mut MapArtCache::default(),
    )?;
    if let Some(frame) = frames
        .iter()
        .find(|frame| frame.motion == NpcMotion::Stand)
        .or_else(|| frames.first())
    {
        for sprite in &mut npc_sprites[frame.sprites.clone()] {
            sprite.rect.x += 250.0;
        }
        let first = frame.sprites.start;
        let end = frame.sprites.end;
        sprites.extend(npc_sprites.into_iter().skip(first).take(end - first));
    }
    if let Some(name) = nx_string_value(string_archive, &format!("Npc.img/{npc_id}/name")) {
        append_map_name_tag_sprite(
            &mut sprites,
            font,
            ui_archive,
            None,
            None,
            &name,
            762.0,
            GAMEPLAY_CAMERA_CENTER_Y,
            MapNameTagKind::Npc,
        );
    }
    if let Some(function) = nx_string_value(string_archive, &format!("Npc.img/{npc_id}/func")) {
        append_map_name_tag_sprite(
            &mut sprites,
            font,
            ui_archive,
            None,
            None,
            &function,
            762.0,
            GAMEPLAY_CAMERA_CENTER_Y + 19.0,
            MapNameTagKind::Npc,
        );
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
            None,
            ui,
            items,
            characters,
            strings,
            &mut HashMap::new(),
            &mut HashMap::new(),
            false,
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
    if variant == "panel-inventory-drop"
        && let Some(item) = inventory
            .items
            .iter_mut()
            .find(|item| item.item_id == 2_000_000)
    {
        item.count = PREVIEW_DROP_STACK_COUNT;
    }
    if variant == "panel-meso-drop" {
        inventory.meso = PREVIEW_MESO_DROP_BALANCE;
    }
    let active_tab = if variant == "panel-item-inventory" {
        let bottom_row_item = inventory
            .items
            .iter()
            .find(|item| {
                item.inventory_type == openstory_protocol::InventoryType::Equip && item.slot == 7
            })
            .cloned();
        inventory.items.retain(|item| {
            item.inventory_type == openstory_protocol::InventoryType::Equip && item.slot <= 2
        });
        if let Some(mut item) = bottom_row_item {
            item.slot = 24;
            inventory.items.push(item);
        }
        openstory_protocol::InventoryType::Equip
    } else if variant.ends_with("-stacks")
        || matches!(variant, "panel-inventory-drop" | "panel-meso-drop")
    {
        openstory_protocol::InventoryType::Use
    } else {
        openstory_protocol::InventoryType::Etc
    };
    if variant.ends_with("-stacks") {
        for (index, item) in inventory
            .items
            .iter_mut()
            .filter(|i| i.inventory_type == active_tab)
            .enumerate()
        {
            item.count = [1, 9, 10, 99, 100, 999, 1000, 9999, 10000, 65535, 8888, 11][index % 12];
        }
    }
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
    let pending_move = variant.ends_with("-pending-move").then_some((
        openstory_protocol::InventoryType::Etc,
        1,
        4,
    ));
    let pixels = super::render_inventory_panel(
        font,
        &inventory,
        active_tab,
        scroll,
        None,
        drag,
        pending_move,
        None,
        legacy_ui_archive,
        item_archive,
        character_archive,
        string_archive,
        &mut HashMap::new(),
        &mut HashMap::new(),
        false,
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
    if variant.ends_with("-drag")
        && let Some(item) = inventory.items.iter().find(|item| {
            item.inventory_type == openstory_protocol::InventoryType::Etc && item.slot == 1
        })
    {
        let drag_pixels = render_inventory_item_cell(
            item,
            legacy_ui_archive,
            item_archive,
            character_archive,
            &mut HashMap::new(),
        );
        sprites.push(DecodedSprite {
            texture_key: "inventory-drag-item:preview".to_owned(),
            rect: SpriteRect {
                x: super::INVENTORY_PANEL_X + 65.0,
                y: super::INVENTORY_PANEL_Y + 102.0,
                width: INVENTORY_SLOT_CELL_WIDTH as f32,
                height: INVENTORY_SLOT_CELL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 1,
            texture_width: INVENTORY_SLOT_CELL_WIDTH as u32,
            texture_height: INVENTORY_SLOT_CELL_HEIGHT as u32,
            pixels: Arc::from(drag_pixels),
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

fn build_merchant_panel_scene(
    variant: &str,
    map_archive: &NxArchive,
    ui_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let room = super::HiredMerchantRoomUi {
        state: openstory_protocol::HiredMerchantRoom {
            room_type: if variant == "panel-personal-shop" {
                4
            } else {
                5
            },
            my_slot: if variant == "panel-merchant-visitor" {
                1
            } else {
                0
            },
            item_id: 5030000,
            owner_name: "Mapler".to_owned(),
            owner_look: None,
            description: "Potions and supplies".to_owned(),
            first_time: false,
            meso: 12345,
            visitors: Vec::new(),
            messages: vec![openstory_protocol::MerchantChatMessage {
                text: "Welcome to my shop!".to_owned(),
                speaker: 0,
            }],
            items: (0..7)
                .map(|i| openstory_protocol::MerchantStockItem {
                    bundles: 20 - i,
                    per_bundle: 5,
                    price: 1250,
                    item_id: 2_000_000 + i as i32,
                    sold_out: i == 2,
                })
                .collect(),
        },
        selected_item: Some(1),
        item_scroll: 0,
        chat_input: String::new(),
        chat_focused: false,
        stock_source: (variant == "panel-merchant-stock").then_some(
            super::HiredMerchantStockSource {
                inventory_type: openstory_protocol::InventoryType::Use,
                slot: 1,
                item_id: 2_000_000,
                count: 10,
            },
        ),
        stock_price: "500".to_owned(),
        notice: None,
    };
    let pixels = super::render_hired_merchant_room_panel(
        font,
        &room,
        ui_archive,
        item_archive,
        character_archive,
        string_archive,
        &mut HashMap::new(),
        &mut HashMap::new(),
    );
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    sprites.push(DecodedSprite {
        texture_key: "nx-window:merchant-preview".to_owned(),
        rect: SpriteRect {
            x: super::HIRED_MERCHANT_PANEL_X,
            y: super::HIRED_MERCHANT_PANEL_Y,
            width: super::HIRED_MERCHANT_PANEL_WIDTH as f32,
            height: super::HIRED_MERCHANT_PANEL_HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: super::HIRED_MERCHANT_PANEL_WIDTH,
        texture_height: super::HIRED_MERCHANT_PANEL_HEIGHT,
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
    variant: &str,
    map_archive: &NxArchive,
    ui_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let trade = super::TradeRoomUi {
        self_slot: 0,
        partner: None,
        own_items: if variant == "panel-trade-room-populated" {
            vec![
                Some(super::TradeRoomItem {
                    item_id: 2_000_000,
                    quantity: 50,
                }),
                None,
                None,
                None,
                Some(super::TradeRoomItem {
                    item_id: 2_000_001,
                    quantity: 3,
                }),
                None,
                None,
                None,
                None,
            ]
        } else {
            vec![None; 9]
        },
        partner_items: if variant == "panel-trade-room-populated" {
            vec![
                Some(super::TradeRoomItem {
                    item_id: 2_000_002,
                    quantity: 15,
                }),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ]
        } else {
            vec![None; 9]
        },
        own_meso: 0,
        partner_meso: 0,
        own_confirmed: false,
        partner_confirmed: false,
        chat_lines: Default::default(),
        chat_input: String::new(),
        chat_focused: false,
        meso_input: String::new(),
        meso_focused: false,
        pending_item: (variant == "panel-trade-room-quantity").then_some(super::TradeStockSource {
            inventory_type: openstory_protocol::InventoryType::Use,
            inventory_slot: 1,
            trade_slot: 0,
            item_id: 2_000_000,
            max_quantity: 100,
        }),
        quantity_input: "10".to_owned(),
        notice: None,
    };
    let pixels = super::render_trade_room_panel(
        font,
        &trade,
        ui_archive,
        item_archive,
        character_archive,
        string_archive,
        &mut HashMap::new(),
        &mut HashMap::new(),
    );
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    sprites.push(DecodedSprite {
        texture_key: "nx-window:live-trade".to_owned(),
        rect: SpriteRect {
            x: 152.0,
            y: 84.0,
            width: 720.0,
            height: 600.0,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: 720,
        texture_height: 600,
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

fn build_npc_shop_panel_scene(
    variant: &str,
    map: &NxArchive,
    ui: &NxArchive,
    npcs: &NxArchive,
    characters: &NxArchive,
    items: &NxArchive,
    strings: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let empty = variant == "panel-npc-shop-empty";
    let recharge = variant == "panel-npc-shop-recharge";
    let recommended = variant == "panel-npc-shop-recommended";
    let no_zero_price_refills = variant == "panel-npc-shop-no-zero-price-refills";
    let shop_item_ids: &[i32] = if recommended {
        &[1_332_000, 1_472_000, 1_472_001, 1_472_003, 1_472_004]
    } else {
        &[
            2_000_000, 2_000_001, 2_000_002, 2_000_003, 2_000_006, 2_000_004, 2_020_001, 2_020_002,
            2_030_000, 2_050_000, 2_050_001, 2_050_002,
        ]
    };
    let mut shop_items = if empty {
        Vec::new()
    } else {
        shop_item_ids
            .iter()
            .copied()
            .enumerate()
            .map(|(index, id)| openstory_protocol::NpcShopItem {
                item_id: if recharge && index == 0 {
                    2_070_000
                } else {
                    id
                },
                price: if recommended {
                    [4_000, 3_000, 8_000, 10_000, 12_000][index]
                } else {
                    [50, 160, 320, 200, 620][index % 5]
                },
                pitch: 0,
                time: 0,
                charge_price: if recharge && index == 0 { 20 } else { 0 },
                buyable: 100,
                rechargeable: recharge && index == 0,
                slot: index as i16,
            })
            .collect::<Vec<_>>()
    };
    if no_zero_price_refills {
        shop_items.insert(
            2,
            openstory_protocol::NpcShopItem {
                item_id: 2_070_001,
                price: 0,
                pitch: 0,
                time: 0,
                charge_price: 20,
                buyable: 1_000,
                rechargeable: true,
                slot: 2,
            },
        );
    }
    for (slot, item) in shop_items.iter_mut().enumerate() {
        item.slot = slot as i16;
    }
    let mut state = NpcShopUi::new(openstory_protocol::NpcShop {
        npc_id: if recommended {
            1_051_000
        } else {
            fixture_named_id(strings, "Npc.img", "Lina", 1011100)
        },
        currency_item_id: 0,
        items: shop_items,
    });
    state.buy_recommended = recommended;
    let mut inventory = if empty {
        openstory_protocol::InventorySnapshot::default()
    } else {
        preview_inventory()
    };
    if !recommended {
        state.tab = openstory_protocol::InventoryType::Use;
    }
    if recommended {
        inventory.meso = 37_508;
        inventory.items = [1_002_008, 1_002_610, 1_002_610, 1_040_002, 1_040_017]
            .into_iter()
            .enumerate()
            .map(|(index, item_id)| openstory_protocol::InventoryItem {
                inventory_type: state.tab,
                slot: index as u16 + 1,
                item_id,
                count: 1,
                ..Default::default()
            })
            .collect();
        state.buy_selection = Some(2);
        state.sell_selection = Some(1);
    } else if !empty {
        inventory.items = (0..12)
            .map(|i| openstory_protocol::InventoryItem {
                inventory_type: state.tab,
                slot: i + 1,
                item_id: if no_zero_price_refills && i == 1 {
                    2_070_001
                } else if recharge && i == 1 {
                    2_070_000
                } else {
                    2_000_000 + i32::from(i % 4)
                },
                count: if (recharge || no_zero_price_refills) && i == 1 {
                    80
                } else {
                    100 - i * 3
                },
                ..Default::default()
            })
            .collect();
        state.buy_selection = Some(1);
        state.sell_selection = Some(2);
    }
    if variant == "panel-npc-shop-hover-buy" {
        state.hovered_item = Some(NpcShopItemHover::Buy(1));
    } else if variant == "panel-npc-shop-hover-sell" {
        state.hovered_item = Some(NpcShopItemHover::Sell(2));
    }
    if variant == "panel-npc-shop-scrolled" {
        state.buy_scroll = 5;
        state.sell_scroll = 5;
        state.buy_selection = Some(6);
        state.sell_selection = Some(7);
    }
    if variant == "panel-npc-shop-stacks" {
        for (index, item) in inventory.items.iter_mut().enumerate() {
            item.count = [1, 99, 100, 8888, 65535][index % 5];
        }
    }
    if variant == "panel-npc-shop-quantity" {
        state.quantity = Some(NpcShopQuantity {
            buy: true,
            slot: 1,
            item_id: 2_000_001,
            maximum: 100,
            input: QuantityInput::new(Some(25)),
            invalid: false,
        });
    }
    if variant == "panel-npc-shop-sell-quantity"
        && let Some(item) = inventory.items.get(1)
    {
        state.quantity = npc_shop_sell_quantity(item);
    }
    if variant == "panel-npc-shop-large-values" {
        inventory.meso = i64::MAX;
    }
    let mut character = preview_character();
    if recommended {
        character.stats.job = 400;
        character.stats.level = 17;
        character.stats.dexterity = 40;
        character.stats.luck = 50;
        state.update_buy_item_indices(Some(&character), &inventory, characters);
    }
    let pixels = render_npc_shop_panel(
        font,
        &state,
        &inventory,
        Some(&character),
        ui,
        npcs,
        items,
        characters,
        strings,
        &mut HashMap::new(),
        &mut HashMap::new(),
    )?;
    let mut sprites = if recommended {
        gameplay_backdrop_for_map(map, characters, font, 103_000_001)?
    } else {
        gameplay_backdrop(map, characters, font)?
    };
    sprites.push(DecodedSprite {
        texture_key: "nx-window:live-npc-shop".to_owned(),
        rect: SpriteRect {
            x: NPC_SHOP_X,
            y: NPC_SHOP_Y,
            width: NPC_SHOP_WIDTH as f32,
            height: NPC_SHOP_HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: NPC_SHOP_WIDTH as u32,
        texture_height: NPC_SHOP_HEIGHT as u32,
        pixels: Arc::from(pixels),
    });
    if let Some(item) = npc_shop_tooltip_item(&state, &inventory) {
        let (cursor_x, cursor_y) = match state.hovered_item {
            Some(NpcShopItemHover::Buy(_)) => (NPC_SHOP_X + 130.0, NPC_SHOP_Y + 144.0),
            Some(NpcShopItemHover::Sell(_)) => (NPC_SHOP_X + 350.0, NPC_SHOP_Y + 144.0),
            None => unreachable!(),
        };
        let tooltip = item_tooltip::render(
            font,
            &item,
            items,
            characters,
            strings,
            Some(&character.stats),
            Some(&inventory),
            &mut HashMap::new(),
            &mut HashMap::new(),
            false,
        );
        let point = item_tooltip::position(
            Point {
                x: cursor_x,
                y: cursor_y,
            },
            tooltip.width,
            tooltip.height,
        );
        sprites.push(item_tooltip_preview_sprite(
            "npc-shop-item-tooltip:preview",
            tooltip,
            point,
            1,
        ));
    }
    if let Some(quantity) = state.quantity.as_ref() {
        let pixels = render_npc_shop_quantity(
            font,
            quantity,
            ui,
            items,
            characters,
            strings,
            &mut HashMap::new(),
            &mut HashMap::new(),
        )?;
        sprites.push(DecodedSprite {
            texture_key: "nx-window:npc-shop-quantity".to_owned(),
            rect: npc_shop_quantity_rect(),
            depth_order: 1,
            texture_width: QUANTITY_WINDOW_WIDTH as u32,
            texture_height: QUANTITY_WINDOW_HEIGHT as u32,
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
        &preview_inventory(),
        openstory_protocol::InventoryType::Equip,
        0,
        0,
        Some((openstory_protocol::InventoryType::Equip, 2, 1_002_001)),
        None,
        "1000",
        true,
        None,
        ui_archive,
        npc_archive,
        item_archive,
        character_archive,
        string_archive,
        &mut HashMap::new(),
        &mut HashMap::new(),
    );
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    sprites.push(DecodedSprite {
        texture_key: "nx-window:live-storage".to_owned(),
        rect: SpriteRect {
            x: 182.0,
            y: 105.0,
            width: STORAGE_PANEL_WIDTH as f32,
            height: STORAGE_PANEL_HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 0,
        texture_width: STORAGE_PANEL_WIDTH as u32,
        texture_height: STORAGE_PANEL_HEIGHT as u32,
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

fn item_tooltip_preview_sprite(
    key: &str,
    tooltip: item_tooltip::Tooltip,
    point: Point,
    depth_order: i16,
) -> DecodedSprite {
    DecodedSprite {
        texture_key: key.to_owned(),
        rect: SpriteRect {
            x: point.x,
            y: point.y,
            width: tooltip.width as f32,
            height: tooltip.height as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order,
        texture_width: tooltip.width,
        texture_height: tooltip.height,
        pixels: tooltip.pixels.into(),
    }
}

fn build_item_tooltip_scene(
    preview_case: &str,
    map_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    use openstory_protocol::{CharacterStats, EquipInventoryData, InventoryItem, InventoryType};

    let comparison_case = preview_case.starts_with("panel-item-tooltip-equipped-comparison");
    let mut stats = CharacterStats {
        level: 30,
        strength: 30,
        dexterity: 65,
        ..Default::default()
    };
    let mut item = InventoryItem {
        inventory_type: InventoryType::Equip,
        slot: 1,
        count: 1,
        ..Default::default()
    };
    let mut equip = EquipInventoryData {
        upgrade_slots: 7,
        ..Default::default()
    };
    item.item_id = match preview_case {
        "panel-item-tooltip-gloves" => {
            stats.level = 15;
            stats.dexterity = 40;
            equip.upgrade_slots = 5;
            equip.stats[1] = 1;
            equip.stats[8] = 8;
            fixture_named_id(
                string_archive,
                "Eqp.img/Eqp/Glove",
                "Basic Archer Gloves",
                1_082_000,
            )
        }
        "panel-item-tooltip-bow" => {
            equip.stats[0] = 1;
            equip.stats[6] = 50;
            fixture_named_id(string_archive, "Eqp.img/Eqp/Weapon", "Ryden", 1_452_005)
        }
        "panel-item-tooltip-thief" => {
            stats.level = 5;
            stats.dexterity = 20;
            equip.stats[6] = 31;
            fixture_named_id(
                string_archive,
                "Eqp.img/Eqp/Weapon",
                "Beginner Thief Wrist Guard",
                1_472_000,
            )
        }
        "panel-item-tooltip-spear" => {
            stats.level = 70;
            equip.level = 5;
            equip.upgrade_slots = 2;
            equip.stats[0] = 9;
            equip.stats[6] = 113;
            equip.stats[8] = 4;
            equip.stats[10] = 37;
            1_432_040
        }
        "panel-item-tooltip-wand" => {
            equip.level = 7;
            equip.upgrade_slots = 0;
            equip.stats[2] = 16;
            equip.stats[3] = 3;
            equip.stats[6] = 84;
            equip.stats[7] = 118;
            equip.stats[9] = 4;
            equip.stats[10] = 6;
            equip.stats[13] = 2;
            1_372_038
        }
        "panel-item-tooltip-cover" => {
            item.cash = true;
            equip.upgrade_slots = 0;
            1_702_181
        }
        "panel-item-tooltip-pet" => {
            item.cash = true;
            equip.upgrade_slots = 0;
            equip.stats[4] = 100;
            1_802_029
        }
        "panel-item-tooltip-mount" => {
            item.cash = true;
            equip.upgrade_slots = 0;
            1_902_008
        }
        "panel-item-tooltip-consumable" => {
            item.inventory_type = InventoryType::Use;
            2_022_179
        }
        _ => {
            stats.level = 5;
            stats.dexterity = 20;
            equip.stats[8] = 3;
            fixture_named_id(
                string_archive,
                "Eqp.img/Eqp/Coat",
                "Grey T-Shirt",
                1_040_010,
            )
        }
    };
    item.equip = (item.inventory_type == InventoryType::Equip).then_some(equip);

    let tooltip = item_tooltip::render(
        font,
        &item,
        item_archive,
        character_archive,
        string_archive,
        Some(&stats),
        None,
        &mut HashMap::new(),
        &mut HashMap::new(),
        false,
    );
    let mut sprites = gameplay_backdrop(map_archive, character_archive, font)?;
    if comparison_case {
        let mut equipped = item.clone();
        if let Some(equip) = equipped.equip.as_mut() {
            equip.level = 3;
            equip.upgrade_slots = 0;
            equip.stats[0] = 6;
            equip.stats[1] = 4;
            equip.stats[8] = 7;
        }
        let equipped_tooltip = item_tooltip::render(
            font,
            &equipped,
            item_archive,
            character_archive,
            string_archive,
            Some(&stats),
            None,
            &mut HashMap::new(),
            &mut HashMap::new(),
            true,
        );
        let (item_point, equipped_point) = item_tooltip::comparison_positions(
            if preview_case.ends_with("-edge") {
                Point { x: 930.0, y: 160.0 }
            } else if preview_case.ends_with("-overflow") {
                Point { x: 900.0, y: 160.0 }
            } else {
                Point { x: 470.0, y: 240.0 }
            },
            tooltip.width,
            tooltip.height,
            equipped_tooltip.width,
            equipped_tooltip.height,
        );
        sprites.push(item_tooltip_preview_sprite(
            "inventory-tooltip:preview",
            tooltip,
            item_point,
            0,
        ));
        sprites.push(item_tooltip_preview_sprite(
            "inventory-tooltip:equipped-preview",
            equipped_tooltip,
            equipped_point,
            1,
        ));
    } else {
        sprites.push(item_tooltip_preview_sprite(
            "inventory-tooltip:preview",
            tooltip,
            Point { x: 520.0, y: 240.0 },
            0,
        ));
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

fn equipment_preview_tab(name: &str) -> Option<EquipmentTab> {
    Some(match name {
        "panel-equipment-inventory" | "panel-equipment-inventory-scroll-drop" => {
            EquipmentTab::Equip
        }
        "panel-equipment-inventory-deco" => EquipmentTab::Deco,
        "panel-equipment-inventory-beauty" => EquipmentTab::Beauty,
        "panel-equipment-inventory-pet" | "panel-equipment-inventory-pet-empty" => {
            EquipmentTab::Pet
        }
        _ => return None,
    })
}

fn preview_equipment_tabs_inventory(name: &str) -> openstory_protocol::InventorySnapshot {
    use openstory_protocol::{InventoryItem, InventoryType, PetInventoryData};

    let mut inventory = preview_equipped_inventory();
    if name == "panel-equipment-inventory-pet-empty" {
        return inventory;
    }
    inventory
        .items
        .retain(|item| !matches!(item.slot, 5 | 6 | 11));
    inventory.items.extend([
        InventoryItem {
            inventory_type: InventoryType::Cash,
            slot: 1,
            item_id: 5_000_054,
            cash_id: Some(1),
            pet: Some(PetInventoryData {
                name: "Snail".to_owned(),
                level: 5,
                closeness: 1,
                fullness: 3,
            }),
            ..Default::default()
        },
        InventoryItem {
            inventory_type: InventoryType::Use,
            slot: 1,
            item_id: 2_000_000,
            count: 183,
            ..Default::default()
        },
        InventoryItem {
            inventory_type: InventoryType::Use,
            slot: 2,
            item_id: 2_000_003,
            count: 208,
            ..Default::default()
        },
        InventoryItem {
            inventory_type: InventoryType::Equip,
            slot: 3,
            item_id: 1_812_001,
            ..Default::default()
        },
        InventoryItem {
            inventory_type: InventoryType::Equip,
            slot: 4,
            item_id: 1_802_007,
            ..Default::default()
        },
    ]);
    // The base and cash layers include items in the same hat and top slots.
    let items = [
        (5, 1_050_058),
        (
            11,
            if name == "panel-equipment-inventory-scroll-drop" {
                1_302_000
            } else {
                1_442_005
            },
        ),
        (12, 1_112_001),
        (13, 1_112_002),
        (15, 1_112_000),
        (16, 1_112_005),
        (17, 1_122_000),
        (49, 1_142_000),
        (50, 1_132_000),
        (2, 1_012_000),
        (3, 1_022_000),
        (4, 1_032_000),
        (8, 1_082_002),
        (9, 1_102_000),
        (101, 1_002_187),
        (105, 1_042_037),
        (106, 1_062_026),
        (107, 1_072_151),
        (114, 1_822_000),
        (117, 1_802_006),
        (123, 1_812_000),
        (124, 1_812_002),
        (125, 1_812_003),
        (126, 1_812_004),
        (127, 1_812_005),
        (128, 1_812_006),
        (129, 1_812_007),
    ];
    inventory
        .items
        .extend(items.into_iter().map(|(slot, item_id)| {
            let mut item = openstory_protocol::InventoryItem {
                inventory_type: openstory_protocol::InventoryType::Equipped,
                slot,
                item_id,
                count: 1,
                cash: slot > 100,
                ..Default::default()
            };
            if name == "panel-equipment-inventory-scroll-drop" {
                item.equip = Some(openstory_protocol::EquipInventoryData {
                    upgrade_slots: 2,
                    ..Default::default()
                });
            }
            item
        }));
    inventory
}

fn build_equipment_panel_scene(
    name: &str,
    tab: EquipmentTab,
    active_scroll_item_id: Option<i32>,
    map_archive: &NxArchive,
    ui_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let inventory = preview_equipment_tabs_inventory(name);
    let pet = if name == "panel-equipment-inventory-pet-empty" {
        ui_equipment::PetSlots::default()
    } else {
        ui_equipment::PetSlots {
            summoned: Some((5_000_054, 1)),
            hp: 2_000_000,
            mp: 2_000_003,
        }
    };
    let pixels = super::ui_equipment::render(
        font,
        &inventory,
        Some(&fixture_look()),
        tab,
        active_scroll_item_id,
        pet,
        ui_archive,
        item_archive,
        character_archive,
        string_archive,
        &mut HashMap::new(),
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

fn build_pet_item_filter_scene(
    map_archive: &NxArchive,
    legacy_ui_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let mut scene = build_equipment_panel_scene(
        "panel-equipment-inventory-pet",
        EquipmentTab::Pet,
        None,
        map_archive,
        legacy_ui_archive,
        character_archive,
        item_archive,
        string_archive,
        font,
    )?;
    let mut ignored_item_ids = vec![1_050_058, 1_112_001, 1_442_005, 2_000_000];
    for (_, item_id) in build_owl_item_name_index(string_archive) {
        if ignored_item_ids.len() >= 14 {
            break;
        }
        if !ignored_item_ids.contains(&item_id) {
            ignored_item_ids.push(item_id);
        }
    }
    let mut filter = ui_pet_item_filter::State {
        open: true,
        ignored_item_lookup: Arc::new(ignored_item_ids.iter().copied().collect()),
        ignored_item_ids,
        ignore_meso: true,
        ignore_below_level_equipment: false,
        query: String::new(),
        scroll: 0,
        selected_item_id: Some(1_050_058),
        search_focused: true,
        key_text_for_ime_dedup: None,
        notice: None,
        item_labels: HashMap::new(),
        item_labels_lower: HashMap::new(),
        item_labels_loaded: true,
        display_order: Vec::new(),
    };
    let mut name_cache = HashMap::new();
    for item_id in &filter.ignored_item_ids {
        let label = item_name(*item_id, string_archive, &mut name_cache);
        filter
            .item_labels_lower
            .insert(*item_id, label.to_lowercase());
        filter.item_labels.insert(*item_id, label);
    }
    ui_pet_item_filter::sort_item_order(&mut filter);
    filter.selected_item_id = filter.display_order.get(3).copied();
    let pixels = ui_pet_item_filter::render(
        font,
        legacy_ui_archive,
        item_archive,
        character_archive,
        string_archive,
        &filter,
        &mut name_cache,
        &mut HashMap::new(),
    );
    scene.sprites.push(DecodedSprite {
        texture_key: "nx-window:live-pet-item-filter".to_owned(),
        rect: SpriteRect {
            x: PET_ITEM_FILTER_PANEL_X,
            y: PET_ITEM_FILTER_PANEL_Y,
            width: PET_ITEM_FILTER_PANEL_WIDTH as f32,
            height: PET_ITEM_FILTER_PANEL_HEIGHT as f32,
            blend_mode: BlendMode::Alpha,
        },
        depth_order: 1,
        texture_width: PET_ITEM_FILTER_PANEL_WIDTH,
        texture_height: PET_ITEM_FILTER_PANEL_HEIGHT,
        pixels: Arc::from(pixels),
    });
    Ok(scene)
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
    let detail_preview = matches!(
        name,
        "panel-quests-detail" | "panel-quests-detail-flow" | "panel-quests-detail-scrolled"
    );
    let detail_flow_preview = matches!(
        name,
        "panel-quests-detail-flow" | "panel-quests-detail-scrolled"
    );
    let detail_scrolled_preview = name == "panel-quests-detail-scrolled";
    let available_preview = name == "panel-quests-available";
    let collapsed_preview = name == "panel-quests-collapsed";
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
    let mut desired_ids = Vec::new();
    let mut all_ids = Vec::new();
    for node in root.children().map_err(|e| e.to_string())? {
        let Some(id) = node.name().ok().and_then(|name| name.parse::<i16>().ok()) else {
            continue;
        };
        if id > 0 {
            all_ids.push(id);
        }
        if let Some(title) = quest_info_text(quest_archive, id, "name")
            && desired.iter().any(|name| title.contains(name))
        {
            desired_ids.push(id);
        }
    }
    all_ids.sort_unstable();
    all_ids.dedup();
    let mut ids = if detail_flow_preview {
        all_ids
    } else if detail_preview {
        desired_ids
    } else if available_preview {
        all_ids
    } else {
        all_ids.into_iter().take(20).collect()
    };
    if ids.is_empty() {
        ids.extend([1000, 1001, 1002]);
    }
    let log = openstory_protocol::QuestLog {
        started: if available_preview {
            Vec::new()
        } else {
            ids.iter()
                .copied()
                .map(|id| openstory_protocol::QuestEntry {
                    id,
                    data: String::new(),
                })
                .collect()
        },
        completed: if available_preview {
            Vec::new()
        } else {
            ids.iter()
                .copied()
                .map(|id| openstory_protocol::CompletedQuest { id, timestamp: 0 })
                .collect()
        },
    };
    let tab = if detail_flow_preview {
        QuestLogTab::Active
    } else if detail_preview {
        QuestLogTab::Completed
    } else if available_preview {
        QuestLogTab::Available
    } else {
        QuestLogTab::Active
    };
    let available_quests = if available_preview {
        ids.iter().map(|id| (*id, 0)).collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let visible_ids = quest_ids_for_tab(&log, &available_quests, tab, quest_archive, font);
    let selected = if detail_flow_preview {
        visible_ids
            .iter()
            .enumerate()
            .max_by_key(|(_, id)| {
                let description = quest_info_text(quest_archive, **id, "1")
                    .filter(|description| quest_description_is_displayable(description))
                    .or_else(|| quest_info_text(quest_archive, **id, "0"))
                    .unwrap_or_default();
                dialogue_text_content_height(
                    font,
                    &clean_npc_markup(&description),
                    10.0,
                    250,
                    &EMPTY_NPC_INLINE_ICONS,
                )
            })
            .map(|(index, _)| index)
            .unwrap_or(0)
    } else if detail_preview {
        visible_ids
            .iter()
            .position(|id| {
                quest_info_text(quest_archive, *id, "name").as_deref() == Some("Paving the Way")
            })
            .unwrap_or(0)
    } else if available_preview {
        visible_ids
            .iter()
            .position(|id| {
                quest_info_text(quest_archive, *id, "name")
                    .is_some_and(|name| name.contains("Helping Out Yoona"))
            })
            .unwrap_or_else(|| visible_ids.len().saturating_sub(1).min(10))
    } else {
        10.min(visible_ids.len().saturating_sub(1))
    };
    let scroll = if available_preview {
        selected.saturating_sub(super::QUEST_LOG_VISIBLE_ROWS - 1)
    } else {
        0
    };
    let (pixels, _) = super::render_quest_log_panel(
        font,
        &log,
        &available_quests,
        quest_archive,
        string_archive,
        None,
        &mut HashMap::new(),
        tab,
        selected,
        !collapsed_preview,
        scroll,
        if detail_scrolled_preview { 72 } else { 0 },
        0,
        None,
        None,
        legacy_ui_archive,
        npcs,
    );
    let canvas_width = super::QUEST_LOG_PANEL_WIDTH as usize;
    let panel_width = if collapsed_preview {
        QUEST_LOG_LIST_WIDTH as usize
    } else {
        QUEST_LOG_PANEL_WIDTH as usize
    };
    let panel_height = QUEST_LOG_PANEL_HEIGHT as usize;
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

fn build_quest_helper_panel_scene(
    map_archive: &NxArchive,
    legacy_ui_archive: &NxArchive,
    character_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let entries = [
        QuestHelperDisplayEntry {
            name: "Mr. Thunder in Need of an Apprentice".to_owned(),
            objectives: vec![
                "1 / 1 Bronze Ingot".to_owned(),
                "0 / 1 Iron Ingot".to_owned(),
            ],
        },
        QuestHelperDisplayEntry {
            name: "I Need to Find My Daughter 1".to_owned(),
            objectives: vec!["1 / 1 Ayan's Toy Sword".to_owned()],
        },
        QuestHelperDisplayEntry {
            name: "Asking Peron for Help".to_owned(),
            objectives: vec!["1 / 1 Eres Letter".to_owned()],
        },
        QuestHelperDisplayEntry {
            name: "Shumi's Request".to_owned(),
            objectives: vec!["4 / 5 Leaf".to_owned(), "0 / 25 Dark Stump".to_owned()],
        },
    ];
    let mut sprites = gameplay_backdrop_for_map(map_archive, character_archive, font, 101_000_000)?;
    let (width, height, pixels) = render_quest_helper_panel(font, &entries, legacy_ui_archive);
    sprites.push(DecodedSprite {
        texture_key: "nx-window:rendered-v83-quest-helper".to_owned(),
        rect: SpriteRect {
            x: QUEST_HELPER_PANEL_X,
            y: QUEST_HELPER_PANEL_Y,
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

fn build_quest_helper_with_quest_log_scene(
    map_archive: &NxArchive,
    legacy_ui_archive: &NxArchive,
    quest_archive: &NxArchive,
    string_archive: &NxArchive,
    character_archive: &NxArchive,
    npcs: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let mut helper =
        build_quest_helper_panel_scene(map_archive, legacy_ui_archive, character_archive, font)?;
    let quest_log = build_quest_log_panel_scene(
        "panel-quests",
        map_archive,
        legacy_ui_archive,
        quest_archive,
        string_archive,
        character_archive,
        npcs,
        font,
    )?;
    let quest_log_panel = quest_log
        .sprites
        .into_iter()
        .find(|sprite| sprite.texture_key == "nx-window:live-quest-log")
        .ok_or_else(|| "quest log preview did not produce its window sprite".to_owned())?;
    let helper_panel = helper
        .sprites
        .pop()
        .ok_or_else(|| "quest helper preview did not produce its window sprite".to_owned())?;
    helper.sprites.push(quest_log_panel);
    helper.sprites.push(helper_panel);
    Ok(helper)
}

fn build_native_window_scene(
    name: &str,
    map_archive: &NxArchive,
    legacy_ui_archive: &NxArchive,
    ui_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    skill_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let mut sprites = if name == "page-cash-shop" {
        Vec::new()
    } else if matches!(name, "panel-inventory-drop" | "panel-meso-drop") {
        build_inventory_panel_scene(
            name,
            map_archive,
            legacy_ui_archive,
            character_archive,
            item_archive,
            string_archive,
            font,
        )?
        .sprites
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
        | "panel-keyboard-settings-quickslots"
        | "panel-keyboard-settings-large-values" => &[],
        "hud-quickslot"
        | "hud-quickslot-item-tooltip"
        | "hud-quickslot-skill-tooltip"
        | "hud-active-skill-tooltip"
        | "hud-quickslot-cooldown"
        | "panel-skill-macros"
        | "panel-skill-macros-drag"
        | "panel-skill-macros-book" => &[],
        "panel-item-inventory" => &[("UIWindow.img/Item/backgrnd", 300.0, 160.0)],
        "panel-inventory-drop" => &[],
        "panel-meso-drop" => &[],
        name if equipment_preview_tab(name).is_some() => &[],
        "panel-skill-inventory"
        | "panel-skill-inventory-disabled"
        | "panel-skill-inventory-beginner"
        | "panel-skill-inventory-beginner-job" => &[
            // Compose the same scaled v83 Skill window used by the live panel below.
        ],
        name if name.starts_with("panel-skill-tooltip") => &[],
        "page-cash-shop" => &[("CashShop.img/Base/backgrnd", 0.0, 0.0)],
        "panel-monster-book" => &[("UIWindow2.img/MonsterBook/backgrnd", 276.5, 20.0)],
        "panel-stats" | "panel-stats-auto-assign" | "panel-stats-detail" => &[
            // Render the live v83 status sheets below so preview and gameplay
            // share the same NX art and layout.
        ],
        "panel-quests" | "panel-quests-available" | "panel-quests-collapsed" => &[],
        // The v83 UI archive has StatusBar.img, not the newer StatusBar2
        // chat-bar tree. Preview the live Rust fallback panel below.
        "hud-chat" | "panel-whisper" => &[],
        "panel-owl" | "panel-owl-results" => &[],
        "panel-party" | "panel-buddy" => &[],
        // The C++ v83 client uses the UserList tree. Newer GuildUI.img art is
        // absent from the installed v83 UI.nx archive.
        "panel-guild"
        | "panel-guild-populated"
        | "panel-guild-scrolled"
        | "panel-guild-bbs"
        | "panel-guild-bbs-draft"
        | "panel-guild-bbs-confirm-delete"
        | "panel-alliance"
        | "panel-alliance-populated"
        | "panel-alliance-scrolled" => &[],
        _ => return Err(format!("no NX native window fixture for {name:?}")),
    };

    for (path, anchor_x, anchor_y) in layers {
        let first_sprite = sprites.len();
        let archive = if matches!(
            name,
            "panel-item-inventory"
                | "panel-quests"
                | "panel-quests-available"
                | "panel-quests-collapsed"
        ) {
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
    if matches!(
        name,
        "panel-guild" | "panel-guild-populated" | "panel-guild-scrolled"
    ) {
        let guild = (name != "panel-guild").then(preview_guild);
        let pixels = render_guild_panel(
            font,
            ui_archive,
            guild.as_ref(),
            None,
            None,
            if name == "panel-guild-scrolled" { 2 } else { 0 },
            &GuildPanelMode::Overview,
            None,
            None,
        );
        sprites.push(DecodedSprite {
            texture_key: "nx-window:rendered-v83-guild".to_owned(),
            rect: SpriteRect {
                x: GUILD_PANEL_X,
                y: GUILD_PANEL_Y,
                width: GUILD_PANEL_WIDTH as f32,
                height: GUILD_PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: GUILD_PANEL_WIDTH,
            texture_height: GUILD_PANEL_HEIGHT,
            pixels: Arc::from(pixels),
        });
    } else if matches!(
        name,
        "panel-alliance" | "panel-alliance-populated" | "panel-alliance-scrolled"
    ) {
        let guild = (name != "panel-alliance").then(preview_guild);
        let alliance = guild.as_ref().map(|_| openstory_protocol::AllianceInfo {
            id: 7,
            name: "Maple Friends".to_owned(),
            rank_titles: std::array::from_fn(|i| format!("Rank {}", i + 1)),
            capacity: 15,
            notice: "Welcome to the alliance! Meet us in Henesys this weekend.".to_owned(),
            guilds: (1..=12)
                .map(|id| openstory_protocol::AllianceGuild {
                    id,
                    name: if id == 1 {
                        "Maple Heroes".to_owned()
                    } else {
                        format!("Mushroom Guild {id}")
                    },
                })
                .collect(),
        });
        let pixels = render_alliance_panel(
            font,
            ui_archive,
            guild.as_ref(),
            alliance.as_ref(),
            None,
            if name == "panel-alliance-scrolled" {
                4
            } else {
                0
            },
        );
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
    } else if matches!(
        name,
        "panel-guild-bbs" | "panel-guild-bbs-draft" | "panel-guild-bbs-confirm-delete"
    ) {
        let thread = openstory_protocol::BbsThread {
            id: 101,
            poster_id: 1,
            title: "Maple World meetup this weekend".to_owned(),
            timestamp: 0,
            icon: 0,
            reply_count: 1,
        };
        let bbs = GuildBbsUi {
            page: 0,
            total_threads: 3,
            notice: Some(openstory_protocol::BbsThread {
                id: 100,
                poster_id: 1,
                title: "Welcome to the guild".to_owned(),
                timestamp: 0,
                icon: 0,
                reply_count: 0,
            }),
            threads: vec![thread.clone()],
            thread: Some(openstory_protocol::BbsThreadView {
                id: thread.id,
                poster_id: thread.poster_id,
                timestamp: thread.timestamp,
                title: thread.title.clone(),
                content: "Meet near the Henesys market after the event. Bring your friends and share the drops.".to_owned(),
                icon: thread.icon,
                replies: vec![openstory_protocol::BbsReply {
                    id: 201,
                    poster_id: 2,
                    timestamp: 0,
                    text: "I'll be there!".to_owned(),
                }],
            }),
            draft: (name == "panel-guild-bbs-draft").then(|| GuildBbsDraft {
                title: "Maple World meetup".to_owned(),
                content: "Meet near the Henesys market after the event. Bring your friends and share the drops.".to_owned(),
                is_notice: false,
                focus: GuildBbsDraftFocus::Content,
            }),
            reply_input: String::new(),
            reply_focused: false,
            confirm_delete: name == "panel-guild-bbs-confirm-delete",
            feedback: None,
        };
        let pixels = render_guild_bbs_panel(font, ui_archive, &bbs, Some(&preview_guild()));
        sprites.push(DecodedSprite {
            texture_key: "nx-window:rendered-v83-guild-bbs".to_owned(),
            rect: SpriteRect {
                x: 100.0,
                y: 100.0,
                width: GUILD_BBS_PANEL_WIDTH as f32,
                height: GUILD_BBS_PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: GUILD_BBS_PANEL_WIDTH,
            texture_height: GUILD_BBS_PANEL_HEIGHT,
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
    } else if matches!(
        name,
        "panel-stats" | "panel-stats-auto-assign" | "panel-stats-detail"
    ) {
        let character = preview_character();
        let pixels = render_status_panel(font, ui_archive, &character, false);
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
        if name == "panel-stats-auto-assign" {
            let pixels = render_status_auto_assign_dialog(font, ui_archive, &character)?;
            let rect = status_auto_assign_dialog_rect();
            sprites.push(DecodedSprite {
                texture_key: "rendered-window:status-auto-assign-dialog".to_owned(),
                rect,
                depth_order: 1,
                texture_width: STATUS_AUTO_ASSIGN_DIALOG_WIDTH as u32,
                texture_height: STATUS_AUTO_ASSIGN_DIALOG_HEIGHT as u32,
                pixels: Arc::from(pixels),
            });
        }
        if matches!(name, "panel-stats" | "panel-stats-detail") {
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
        sprites.extend(build_status_bar_sprites(legacy_ui_archive)?.0);
        let lines = (0..9).map(|index| ChatLine {
            text: if index % 3 == 0 { "The fairy's Gifts is now level 2 - 1WATK" } else { "The scroll lights up, and then its mysterious power has been transferred to the item." }.to_owned(),
            color: if index % 3 == 0 { [80, 200, 255] } else { [245, 170, 95] },
        }).collect::<VecDeque<_>>();
        let pixels = render_chat_panel(
            legacy_ui_archive,
            font,
            &lines,
            "",
            true,
            0,
            false,
            ChatTarget::Public,
            None,
        );
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
    } else if name == "panel-owl" || name == "panel-owl-results" {
        let results_mode = name == "panel-owl-results";
        let mut owl = OwlSearchUi {
            slot: 1,
            item_id: 5_022_000,
            query: "Red Potion".to_owned(),
            top_item_ids: vec![2_000_000, 2_000_001, 2_000_002, 2_000_003],
            top_items_loaded: true,
            results: Vec::new(),
            result_item_id: results_mode.then_some(2_000_000),
            selected_result: None,
            result_offset: 0,
            notice: None,
        };
        if results_mode {
            owl.results = vec![
                openstory_protocol::OwlShopResult {
                    owner: "MapleSeller".to_owned(),
                    map_id: 100_000_000,
                    description: "Red Potion x100".to_owned(),
                    bundles: 4,
                    per_bundle: 100,
                    price: 2_000,
                    owner_id: 1001,
                    channel: 0,
                },
                openstory_protocol::OwlShopResult {
                    owner: "MushroomMart".to_owned(),
                    map_id: 101_000_000,
                    description: "Red Potion x50".to_owned(),
                    bundles: 6,
                    per_bundle: 50,
                    price: 1_500,
                    owner_id: 1002,
                    channel: 1,
                },
                openstory_protocol::OwlShopResult {
                    owner: "HenesysShop".to_owned(),
                    map_id: 100_000_000,
                    description: "Red Potion x100".to_owned(),
                    bundles: 2,
                    per_bundle: 100,
                    price: 1_800,
                    owner_id: 1003,
                    channel: 2,
                },
            ];
            owl.selected_result = Some(1);
        }
        let pixels = render_owl_panel(font, ui_archive, &owl, string_archive, &mut HashMap::new());
        sprites.push(DecodedSprite {
            texture_key: "nx-window:rendered-v83-owl".to_owned(),
            rect: SpriteRect {
                x: OWL_PANEL_X,
                y: OWL_PANEL_Y,
                width: OWL_PANEL_WIDTH as f32,
                height: OWL_PANEL_HEIGHT as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: OWL_PANEL_WIDTH,
            texture_height: OWL_PANEL_HEIGHT,
            pixels: Arc::from(pixels),
        });
    } else if name == "panel-inventory-drop" || name == "panel-meso-drop" {
        let drop_mesos = name == "panel-meso-drop";
        let maximum = if drop_mesos {
            meso_drop_maximum(PREVIEW_MESO_DROP_BALANCE)
        } else {
            item_drop_maximum(PREVIEW_DROP_STACK_COUNT)
        };
        let mut prompt = InventoryDropPrompt {
            action: if drop_mesos {
                InventoryQuantityAction::DropMesos
            } else {
                InventoryQuantityAction::Drop
            },
            inventory_type: openstory_protocol::InventoryType::Use,
            slot: if drop_mesos { 0 } else { 1 },
            item_id: if drop_mesos { 0 } else { 2_000_000 },
            max_quantity: maximum,
            quantity: QuantityInput::new(None),
            step: InventoryDropStep::Quantity,
            notice: None,
        };
        let minimum = prompt.minimum();
        prompt
            .quantity
            .set_from_slider(QUANTITY_TRACK_RIGHT, minimum, maximum);
        let icon = if drop_mesos {
            None
        } else {
            inventory_item_icon(
                prompt.item_id,
                item_archive,
                character_archive,
                &mut HashMap::new(),
            )
        };
        let pixels = render_inventory_drop_prompt(
            ui_archive,
            font,
            &prompt,
            icon.as_ref(),
            string_archive,
            &mut HashMap::new(),
        )?;
        sprites.push(DecodedSprite {
            texture_key: if drop_mesos {
                "nx-window:rendered-v83-meso-drop".to_owned()
            } else {
                "nx-window:rendered-v83-inventory-drop".to_owned()
            },
            rect: inventory_quantity_rect(),
            depth_order: 0,
            texture_width: QUANTITY_WINDOW_WIDTH as u32,
            texture_height: QUANTITY_WINDOW_HEIGHT as u32,
            pixels: Arc::from(pixels),
        });
    }

    if name.starts_with("hud-quickslot") {
        let keys = DEFAULT_QUICKSLOT_KEYS;
        let mut bindings = keyboard_settings::defaults();
        const PREVIEW_ITEM_ID: i32 = 2_000_000;
        let preview_key = usize::from(keys[1]);
        bindings[preview_key] = openstory_protocol::KeyBinding {
            kind: if matches!(
                name,
                "hud-quickslot-skill-tooltip" | "hud-quickslot-cooldown"
            ) {
                1
            } else {
                2
            },
            action: if matches!(
                name,
                "hud-quickslot-skill-tooltip" | "hud-quickslot-cooldown"
            ) {
                1_001_004
            } else {
                PREVIEW_ITEM_ID
            },
        };
        let inventory = openstory_protocol::InventorySnapshot {
            items: vec![
                openstory_protocol::InventoryItem {
                    inventory_type: openstory_protocol::InventoryType::Use,
                    slot: 1,
                    item_id: PREVIEW_ITEM_ID,
                    count: 5,
                    ..Default::default()
                },
                openstory_protocol::InventoryItem {
                    inventory_type: openstory_protocol::InventoryType::Use,
                    slot: 2,
                    item_id: PREVIEW_ITEM_ID,
                    count: 6,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
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
                        "{:03}.img/skill/{:07}/icon",
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
        let item_counts = std::array::from_fn(|slot| {
            let binding = bindings
                .get(usize::from(keys[slot]))
                .copied()
                .unwrap_or_default();
            if matches!(binding.kind, 2 | 3) {
                inventory.item_count(binding.action)
            } else {
                0
            }
        });
        let pixels = render_quickslot_panel_pixels(
            quickslot_background_image(legacy_ui_archive),
            ui_archive,
            &keys,
            &icons,
            &item_counts,
            &std::array::from_fn(|slot| {
                if name == "hud-quickslot-cooldown" && slot == 1 {
                    0.56
                } else {
                    0.0
                }
            }),
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
        let hovered_cell = ui_foundation::QUICKSLOT_GRID.cell(1);
        let cursor = Point {
            x: 865.0 + hovered_cell.x + hovered_cell.width * 0.5,
            y: 615.0 + hovered_cell.y + hovered_cell.height * 0.5,
        };
        if name == "hud-quickslot-item-tooltip"
            && let Some(item) = inventory.items.first()
        {
            let tooltip = item_tooltip::render(
                font,
                item,
                item_archive,
                character_archive,
                string_archive,
                None,
                Some(&inventory),
                &mut HashMap::new(),
                &mut HashMap::new(),
                false,
            );
            let point = item_tooltip::position(cursor, tooltip.width, tooltip.height);
            sprites.push(item_tooltip_preview_sprite(
                "inventory-tooltip:quickslot-preview",
                tooltip,
                point,
                0,
            ));
        } else if name == "hud-quickslot-skill-tooltip" {
            let mut skill = preview_skill_entry();
            skill.id = 1_001;
            skill.name = "Recovery".to_owned();
            let (height, pixels) =
                skill_tooltip::render(font, &skill, skill_archive, string_archive);
            let position = skill_tooltip::position(cursor, height);
            sprites.push(DecodedSprite {
                texture_key: "skill-tooltip:quickslot-preview".to_owned(),
                rect: SpriteRect {
                    x: position.x,
                    y: position.y,
                    width: skill_tooltip::WIDTH as f32,
                    height: height as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: skill_tooltip::WIDTH,
                texture_height: height,
                pixels: Arc::from(pixels),
            });
        }
    }

    if name == "hud-active-skill-tooltip" {
        let mut skill = preview_skill_entry();
        skill.id = 1_001_004;
        skill.name = "Power Guard".to_owned();
        if let Some((width, height, pixels)) = decode_nx_icon(
            skill_archive,
            &format!("{:03}.img/skill/{:07}/icon", skill.id / 10_000, skill.id),
        ) {
            let pixels = super::render_buff_list_pixels(&[(
                width,
                height,
                pixels,
                Some(std::time::Duration::from_secs(83)),
                180_000,
                std::time::Duration::from_secs(97),
            )]);
            sprites.push(DecodedSprite {
                texture_key: "buff-icon:active-preview".to_owned(),
                rect: SpriteRect {
                    x: 0.0,
                    y: 24.0,
                    width: CANVAS_WIDTH as f32,
                    height: 32.0,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: CANVAS_WIDTH,
                texture_height: 32,
                pixels: Arc::from(pixels),
            });
        }
        let cursor = Point { x: 1005.0, y: 40.0 };
        let (height, pixels) = skill_tooltip::render_with_remaining(
            font,
            &skill,
            skill_archive,
            string_archive,
            Some("1:23"),
        );
        let position = skill_tooltip::position(cursor, height);
        sprites.push(DecodedSprite {
            texture_key: "skill-tooltip:tooltip-active-preview".to_owned(),
            rect: SpriteRect {
                x: position.x,
                y: position.y,
                width: skill_tooltip::WIDTH as f32,
                height: height as f32,
                blend_mode: BlendMode::Alpha,
            },
            depth_order: 0,
            texture_width: skill_tooltip::WIDTH,
            texture_height: height,
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
            &if name.ends_with("-large-values") {
                HashMap::from([(2_000_006, 65_535), (2_000_002, 123_456)])
            } else {
                HashMap::from([(2_000_006, 245), (2_000_002, 251)])
            },
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

    if name == "panel-skill-inventory"
        || name == "panel-skill-inventory-disabled"
        || name == "panel-skill-inventory-beginner"
        || name == "panel-skill-inventory-beginner-job"
        || name == "panel-skill-macros-book"
        || name.starts_with("panel-skill-tooltip")
    {
        let beginner_preview = name == "panel-skill-inventory-beginner";
        let beginner_job_preview = name == "panel-skill-inventory-beginner-job";
        let disabled_preview = name == "panel-skill-inventory-disabled";
        let mut entries = preview_skill_entries();
        if disabled_preview {
            for (entry, (id, skill_name)) in entries.iter_mut().zip([
                (4_000_000, "Nimble Body"),
                (4_000_001, "Keen Eyes"),
                (4_001_002, "Disorder"),
                (4_001_003, "Dark Sight"),
            ]) {
                entry.id = id;
                entry.name = skill_name.to_owned();
                entry.level = 0;
                entry.master_level = 0;
                entry.max_level = 20;
                entry.passive = matches!(id, 4_000_000 | 4_000_001);
                entry.requirements.clear();
            }
            entries[3].requirements = vec![(4_001_002, 3)];
            entries.truncate(4);
        } else if beginner_preview || beginner_job_preview {
            for (entry, (id, skill_name)) in entries.iter_mut().zip([
                (1_000, "Three Snails"),
                (1_001, "Recovery"),
                (1_002, "Nimble Feet"),
            ]) {
                entry.id = id;
                entry.name = skill_name.to_owned();
                entry.level = 0;
                entry.max_level = 3;
            }
            entries.truncate(3);
        } else {
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
        }
        if name == "panel-skill-macros-book" {
            for (entry, (id, name)) in entries.iter_mut().zip([
                (1_001_004, "Power Strike"),
                (1_001_005, "Slash Blast"),
                (1_001_003, "Iron Body"),
                (1_101_003, "Booster"),
            ]) {
                entry.id = id;
                entry.name = name.to_owned();
                entry.level = 1;
                entry.max_level = 20;
                entry.passive = false;
            }
        }
        if name.starts_with("panel-skill-tooltip") {
            if !name.ends_with("-unlearned") {
                entries = preview_skill_entries();
            }
            entries[0].id = 1_001_004;
            entries[0].name = "Power Strike".to_owned();
            entries[0].level = 5;
            entries[0].max_level = 20;
            entries[0].master_level = 0;
            entries[0].passive = false;
            if name.ends_with("-unlearned") {
                entries[0].id = 1_100_000;
                entries[0].name = "Sword Mastery".to_owned();
                entries[0].level = 0;
                entries[0].passive = true;
            } else if name.ends_with("-max-level") {
                entries[0].level = 20;
            }
        }
        let (book_title, book_icon, skill_points, tab, visible_tabs) = if disabled_preview {
            (
                "Thief Skills",
                decode_nx_icon(skill_archive, "400.img/info/icon"),
                3,
                1,
                2,
            )
        } else if beginner_preview {
            let beginner_stats = openstory_protocol::CharacterStats {
                level: 3,
                job: 0,
                ..Default::default()
            };
            let points = super::client_cash_skills::skill_points_for_book(
                &beginner_stats,
                0,
                &HashMap::new(),
            );
            (
                "Beginner's Basics",
                decode_nx_icon(skill_archive, "000.img/info/icon"),
                points,
                0,
                1,
            )
        } else if beginner_job_preview {
            let advanced_stats = openstory_protocol::CharacterStats {
                level: 30,
                job: 110,
                skill_points: 8,
                ..Default::default()
            };
            let points = super::client_cash_skills::skill_points_for_book(
                &advanced_stats,
                0,
                &HashMap::new(),
            );
            (
                "Beginner's Basics",
                decode_nx_icon(skill_archive, "000.img/info/icon"),
                points,
                0,
                3,
            )
        } else if name.starts_with("panel-skill-tooltip") && !name.ends_with("-unlearned") {
            (
                "Warrior Basics",
                decode_nx_icon(skill_archive, "100.img/info/icon"),
                2,
                1,
                2,
            )
        } else {
            (
                "Fighter Techniques",
                decode_nx_icon(skill_archive, "110.img/info/icon"),
                2,
                2,
                3,
            )
        };
        let preview_skills: HashMap<i32, openstory_protocol::SkillEntry> = if disabled_preview {
            [openstory_protocol::SkillEntry {
                id: 4_001_002,
                level: 0,
                ..Default::default()
            }]
            .into_iter()
            .map(|skill| (skill.id, skill))
            .collect()
        } else {
            HashMap::new()
        };
        let pixels = render_skill_book_panel(
            font,
            ui_archive,
            skill_archive,
            &entries,
            book_title,
            book_icon,
            skill_points,
            &preview_skills,
            tab,
            visible_tabs,
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
        if name.starts_with("panel-skill-tooltip") {
            let (height, pixels) =
                skill_tooltip::render(font, &entries[0], skill_archive, string_archive);
            let cursor = if name.ends_with("-edge") {
                let book = sprites.last_mut().expect("skill book preview");
                book.rect.x = 1024.0 - SKILL_BOOK_PANEL_WIDTH as f32;
                book.rect.y = 768.0 - SKILL_BOOK_PANEL_HEIGHT as f32;
                Point {
                    x: book.rect.x + 100.0,
                    y: book.rect.y + 116.0,
                }
            } else {
                Point {
                    x: SKILL_BOOK_PANEL_X + 100.0,
                    y: SKILL_BOOK_PANEL_Y + 116.0,
                }
            };
            let position = skill_tooltip::position(cursor, height);
            sprites.push(DecodedSprite {
                texture_key: "skill-tooltip:preview".to_owned(),
                rect: SpriteRect {
                    x: position.x,
                    y: position.y,
                    width: skill_tooltip::WIDTH as f32,
                    height: height as f32,
                    blend_mode: BlendMode::Alpha,
                },
                depth_order: 0,
                texture_width: skill_tooltip::WIDTH,
                texture_height: height,
                pixels: Arc::from(pixels),
            });
        }
    }

    if name.starts_with("panel-skill-macros") {
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
        if name == "panel-skill-macros-drag" {
            editor.begin_skill_drag(1_001_005, 12.0, 295.0, true);
            editor.update_skill_drag(104.0, 105.0);
        }
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
    let native_window_preview = equipment_preview_tab(name).is_some()
        || name == "panel-pet-item-filter"
        || matches!(
            name,
            "panel-game-menu"
                | "panel-keyboard-settings"
                | "panel-keyboard-settings-empty"
                | "panel-keyboard-settings-capture"
                | "panel-keyboard-settings-scroll"
                | "panel-keyboard-settings-quickslots"
                | "panel-keyboard-settings-large-values"
                | "panel-skill-macros"
                | "panel-item-inventory"
                | "panel-inventory-drop"
                | "panel-meso-drop"
                | "panel-skill-inventory"
                | "panel-skill-inventory-disabled"
                | "panel-skill-inventory-beginner"
                | "panel-skill-inventory-beginner-job"
                | "panel-skill-tooltip"
                | "panel-skill-tooltip-unlearned"
                | "panel-skill-tooltip-max-level"
                | "panel-skill-tooltip-edge"
                | "page-cash-shop"
                | "page-cash-shop-tooltip"
                | "panel-monster-book"
                | "panel-stats"
                | "panel-stats-auto-assign"
                | "panel-stats-detail"
                | "panel-quest-helper"
                | "panel-quests"
                | "panel-quests-available"
                | "panel-quests-collapsed"
                | "panel-quests-detail"
                | "panel-quests-detail-flow"
                | "panel-quests-detail-scrolled"
                | "hud-chat"
                | "panel-owl"
                | "panel-owl-results"
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

#[cfg(test)]
#[path = "menu_preview_tests.rs"]
mod menu_preview_tests;
