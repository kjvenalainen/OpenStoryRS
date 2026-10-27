//! Offline UI screenshot and geometry preview support.
//!
//! This module deliberately calls the same scene builders and decoded sprites as
//! the live client. It does not create a window or connect to a game server.

use super::*;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const CANVAS_WIDTH: u32 = 1024;
const CANVAS_HEIGHT: u32 = 768;
const PREVIEW_SIZES: [(u32, u32); 2] = [(1024, 768), (1440, 900)];

pub(super) fn run(
    screen: &str,
    output_dir: &Path,
    include_hitboxes: bool,
    map: &NxArchive,
    legacy_ui: &NxArchive,
    ui: &NxArchive,
    characters: &NxArchive,
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

    let screens: &[&str] = match screen {
        "all" => &[
            "login",
            "login-active",
            "world-select",
            "character-select-empty",
            "character-select-populated",
            "character-create-class",
            "character-create-explorer",
            "native-keybindings",
            "native-quickslots",
            "native-skill-macros",
            "native-items",
            "native-equipment",
            "native-skills",
            "native-pet-equipment",
            "native-minimap-collapsed",
            "native-minimap-normal",
            "native-minimap-expanded",
            "native-npc-store",
            "native-cash-shop",
            "native-cash-shop-locker",
            "native-cash-shop-coupon",
            "native-cash-shop-confirmation",
            "native-monster-book",
            "native-monster-book-detail",
            "native-monster-book-detail-move",
            "native-monster-book-detail-die",
            "native-status",
            "native-status-bar",
            "native-quit-confirmation",
            "native-quest-log",
            "native-quest-log-detail",
            "native-chat",
            "native-item-megaphone",
            "native-party",
            "native-buddy",
            "native-guild",
            "native-alliance",
            "native-character-info",
            "native-world-map",
            "native-world-map-link",
            "native-options",
            "native-audio-settings",
        ],
        "login"
        | "login-active"
        | "world-select"
        | "character-select-empty"
        | "character-select-populated"
        | "character-create-class"
        | "character-create-explorer"
        | "native-keybindings"
        | "native-quickslots"
        | "native-skill-macros"
        | "native-items"
        | "native-equipment"
        | "native-skills"
        | "native-pet-equipment"
        | "native-minimap-collapsed"
        | "native-minimap-normal"
        | "native-minimap-expanded"
        | "native-npc-store"
        | "native-cash-shop"
        | "native-cash-shop-locker"
        | "native-cash-shop-coupon"
        | "native-cash-shop-confirmation"
        | "native-monster-book"
        | "native-monster-book-detail"
        | "native-monster-book-detail-move"
        | "native-monster-book-detail-die"
        | "native-status"
        | "native-status-bar"
        | "native-quit-confirmation"
        | "native-quest-log"
        | "native-quest-log-detail"
        | "native-chat"
        | "native-item-megaphone"
        | "native-party"
        | "native-buddy"
        | "native-guild"
        | "native-alliance"
        | "native-character-info"
        | "native-world-map"
        | "native-world-map-link"
        | "native-world-map-marker"
        | "native-world-map-search"
        | "native-world-map-result"
        | "native-options"
        | "native-audio-settings" => std::slice::from_ref(&screen),
        _ => {
            return Err(format!(
                "unknown UI preview {screen:?}; use --help to list preview names"
            ));
        }
    };

    for name in screens {
        if name.starts_with("native-cash-shop") {
            let variant = match *name {
                "native-cash-shop" => "catalog",
                "native-cash-shop-locker" => "locker",
                "native-cash-shop-coupon" => "coupon",
                "native-cash-shop-confirmation" => "confirmation",
                _ => unreachable!("cash shop screen name was validated above"),
            };
            let scene = super::cash_shop_ui::render_preview(
                variant,
                ui,
                font,
                etc,
                items,
                characters,
                strings,
            )?;
            let base = scene.pixels;
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
            let report_path = output_dir.join(format!("{name}-layout.txt"));
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
                let path = output_dir.join(format!("{name}-hitboxes-1024x768.png"));
                write_png(&path, CANVAS_WIDTH, CANVAS_HEIGHT, annotated)
                    .map_err(|error| format!("could not write {}: {error}", path.display()))?;
            }
            continue;
        }
        let detail_animation_mode = match *name {
            "native-monster-book-detail" => Some(0),
            "native-monster-book-detail-move" => Some(1),
            "native-monster-book-detail-die" => Some(2),
            _ => None,
        };
        let scene = if let Some(mode) = detail_animation_mode {
            build_monster_book_detail_scene(map, ui, items, characters, mobs, strings, font, mode)?
        } else if *name == "native-items" {
            build_inventory_panel_scene(map, legacy_ui, characters, items, strings, font)?
        } else if matches!(*name, "native-quest-log" | "native-quest-log-detail") {
            build_quest_log_panel_scene(name, map, legacy_ui, quests, strings, characters, font)?
        } else if *name == "native-quit-confirmation" {
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
        } else if name.starts_with("native-minimap-") {
            build_minimap_scene(name, map, ui, strings, font)?
        } else {
            build_scene(
                name, map, legacy_ui, ui, characters, items, skills, etc, strings, font,
            )?
        };
        let base = compose_scene(&scene.sprites, CANVAS_WIDTH, CANVAS_HEIGHT);
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
            if *name == "native-quickslots" {
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
            if *name == "native-items" {
                for tab in 0..5 {
                    draw_rect_outline(
                        &mut image,
                        CANVAS_WIDTH,
                        CANVAS_HEIGHT,
                        SpriteRect {
                            x: super::INVENTORY_PANEL_X + 10.0 + tab as f32 * 31.0,
                            y: super::INVENTORY_PANEL_Y + 27.0,
                            width: 31.0,
                            height: 16.0,
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
            if matches!(*name, "native-quest-log" | "native-quest-log-detail") {
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
                            y: super::QUEST_LOG_PANEL_Y + 50.0 + row as f32 * 25.0,
                            width: 236.0,
                            height: 25.0,
                            blend_mode: BlendMode::Alpha,
                        },
                        hitbox_color(3 + row),
                    );
                }
            }
            image
        });

        let report = geometry_report(name, &scene, CANVAS_WIDTH, CANVAS_HEIGHT);
        let report_path = output_dir.join(format!("{name}-layout.txt"));
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
            let path = output_dir.join(format!("{name}-hitboxes-1024x768.png"));
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
        "native-options" => {
            let mut sprites = gameplay_backdrop(map, character_archive, font)?;
            let pixels = render_options_panel(
                font,
                AudioSettings {
                    music_volume: 50,
                    effects_volume: 70,
                },
                GameplaySettings::default(),
                OptionSlider::Music,
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
        "native-status-bar" => {
            let mut sprites = gameplay_backdrop(map, character_archive, font)?;
            let (bar_sprites, gauge_assets, layout) = build_status_bar_sprites(legacy_ui)?;
            let stats = openstory_protocol::CharacterStats {
                name: "Bobby".to_owned(),
                level: 12,
                job: 0,
                hp: 50,
                max_hp: 50,
                mp: 50,
                max_mp: 78,
                experience: 82,
                ..Default::default()
            };
            let fractions = [1.0_f32, 50.0 / 78.0, 0.18];
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
                    sprite.rect.width *= fill_fraction;
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
            let hud_pixels = render_map_hud(
                font,
                Some(&number_font),
                layout,
                &stats,
                &HashMap::new(),
                Some(("[Welcome] Welcome to MapleStory!", [255, 220, 44])),
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
        "native-item-megaphone" => {
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
        "login" | "login-active" => {
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
            if name == "login-active" {
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
        "native-world-map"
        | "native-world-map-link"
        | "native-world-map-marker"
        | "native-world-map-search"
        | "native-world-map-result" => {
            let query = if matches!(name, "native-world-map-search" | "native-world-map-result") {
                "Orbis"
            } else {
                ""
            };
            let search_catalog = world_map::load_search_catalog(map, strings)?;
            let panel = world_map::render(
                map,
                ui,
                100_000_000,
                (name == "native-world-map-result").then_some("WorldMap020"),
                true,
                query,
                name == "native-world-map-search",
                Some(&search_catalog),
                0,
                (name == "native-world-map-link").then_some(0),
                (name == "native-world-map-marker").then_some(0),
                (name == "native-world-map-result").then_some(200_000_000),
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
        "world-select" => {
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
        "character-select-empty" | "character-select-populated" => {
            let characters = if name == "character-select-empty" {
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
        "character-create-class" => load_character_class_scene(map, legacy_ui, font),
        "character-create-explorer" => {
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
        "native-keybindings"
        | "native-quickslots"
        | "native-skill-macros"
        | "native-items"
        | "native-equipment"
        | "native-skills"
        | "native-pet-equipment"
        | "native-npc-store"
        | "native-cash-shop"
        | "native-monster-book"
        | "native-status"
        | "native-quest-log"
        | "native-chat"
        | "native-party"
        | "native-buddy"
        | "native-guild"
        | "native-alliance"
        | "native-audio-settings" => build_native_window_scene(
            name,
            map,
            legacy_ui,
            ui,
            character_archive,
            item_archive,
            skill_archive,
            font,
        ),
        "native-character-info" => {
            build_character_info_scene(map, character_archive, strings, font)
        }
        _ => Err(format!("unsupported UI preview {name:?}")),
    }
}

fn build_character_info_scene(
    map_archive: &NxArchive,
    character_archive: &NxArchive,
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
    let pixels = render_character_info_panel(font, &ui, string_archive, &mut HashMap::new());
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
    let map_id = 100_000_000;
    let data = minimap::MiniMapData::load(map_archive, map_id)?
        .ok_or_else(|| format!("map {map_id} has no minimap to preview"))?;
    let ui_assets = minimap::UiAssets::load(ui_archive)?;
    let presentation = match name {
        "native-minimap-collapsed" => minimap::Presentation::Collapsed,
        "native-minimap-normal" => minimap::Presentation::Normal,
        "native-minimap-expanded" => minimap::Presentation::Expanded,
        _ => return Err(format!("unknown minimap preview {name:?}")),
    };
    let markers = [
        minimap::Marker {
            kind: minimap::MarkerKind::Player,
            x: -520.0,
            y: -310.0,
            facing_left: false,
            sprite: None,
        },
        minimap::Marker {
            kind: minimap::MarkerKind::Npc,
            x: -650.0,
            y: -250.0,
            facing_left: false,
            sprite: None,
        },
        minimap::Marker {
            kind: minimap::MarkerKind::Mob,
            x: -420.0,
            y: -360.0,
            facing_left: true,
            sprite: None,
        },
    ];
    let map_name =
        map_string_name(string_archive, map_id).unwrap_or_else(|| format!("Map {map_id}"));
    let panel = minimap::render(
        &data,
        &ui_assets,
        LogicalSize::LEGACY_1024X768,
        &map_name,
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
    animation_state.select_card(2_380_000, 100_100, mob_archive);
    for _ in 0..detail_animation_mode {
        animation_state.cycle_detail_animation();
    }
    let panel = monster_book::render(
        ui_archive,
        item_archive,
        character_archive,
        mob_archive,
        string_archive,
        &[(2_380_000, 1)],
        &HashMap::new(),
        2_380_000,
        0,
        None,
        Some(2_380_000),
        Some(animation_state.current_detail_animation()),
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
    if let Some(frame) = animation_state.refresh_detail_animation_sprite(mob_archive)?
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
    let mut map_scene = load_map_scene(map_archive, None, 100_000_000, 0, None, font)?;
    map_scene
        .sprites
        .retain(|sprite| sprite.texture_key != "label:15:Map 100000000");
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

fn build_inventory_panel_scene(
    map_archive: &NxArchive,
    legacy_ui_archive: &NxArchive,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let inventory = openstory_protocol::InventorySnapshot {
        meso: 1_234_567,
        slot_maxima: [24; 5],
        items: vec![
            openstory_protocol::InventoryItem {
                inventory_type: openstory_protocol::InventoryType::Use,
                slot: 1,
                item_id: 2_000_000,
                count: 50,
                ..Default::default()
            },
            openstory_protocol::InventoryItem {
                inventory_type: openstory_protocol::InventoryType::Use,
                slot: 2,
                item_id: 2_000_001,
                count: 25,
                ..Default::default()
            },
            openstory_protocol::InventoryItem {
                inventory_type: openstory_protocol::InventoryType::Use,
                slot: 5,
                item_id: 2_000_002,
                count: 10,
                ..Default::default()
            },
        ],
    };
    let pixels = super::render_inventory_panel(
        font,
        &inventory,
        openstory_protocol::InventoryType::Use,
        0,
        None,
        None,
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

fn build_quest_log_panel_scene(
    name: &str,
    map_archive: &NxArchive,
    legacy_ui_archive: &NxArchive,
    quest_archive: &NxArchive,
    string_archive: &NxArchive,
    character_archive: &NxArchive,
    font: &fontdue::Font,
) -> Result<LoginSceneAssets, String> {
    let detail_preview = name == "native-quest-log-detail";
    let log = if detail_preview {
        openstory_protocol::QuestLog {
            started: vec![openstory_protocol::QuestEntry {
                id: 1_000,
                data: String::new(),
            }],
            completed: Vec::new(),
        }
    } else {
        openstory_protocol::QuestLog::default()
    };
    let tab = if detail_preview {
        super::QuestLogTab::Available
    } else {
        super::QuestLogTab::Active
    };
    let available = detail_preview
        .then_some((1_000, 20))
        .into_iter()
        .collect::<Vec<_>>();
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
    );
    let canvas_width = super::QUEST_LOG_PANEL_WIDTH as usize;
    let (panel_width, panel_height) = if detail_preview {
        (
            super::QUEST_LOG_PANEL_WIDTH as usize,
            super::QUEST_LOG_PANEL_HEIGHT as usize,
        )
    } else {
        (296, 289)
    };
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
    let mut sprites = if name == "native-cash-shop" {
        Vec::new()
    } else {
        gameplay_backdrop(map_archive, character_archive, font)?
    };

    // These are the original client window layers and logical positions, not
    // approximations generated by the Rust panel renderer. Related layers
    // share the same anchor; their NX origins provide the authored offsets.
    let layers: &[(&str, f32, f32)] = match name {
        "native-keybindings" => &[
            ("UIWindow2.img/KeyConfig/backgrnd", 201.0, 197.0),
            ("UIWindow2.img/KeyConfig/backgrnd2", 201.0, 197.0),
            ("UIWindow2.img/KeyConfig/backgrnd3", 201.0, 197.0),
        ],
        "native-quickslots" | "native-skill-macros" => &[],
        "native-items" => &[("UIWindow.img/Item/backgrnd", 300.0, 160.0)],
        "native-equipment" => &[
            ("UIWindow2.img/Equip/character/backgrnd", 184.0, 20.0),
            ("UIWindow2.img/Equip/character/backgrnd2", 184.0, 20.0),
            ("UIWindow2.img/Equip/character/backgrnd3", 184.0, 20.0),
        ],
        "native-skills" => &[
            ("UIWindow2.img/Skill/main/backgrnd", 174.0, 20.0),
            ("UIWindow2.img/Skill/main/backgrnd2", 174.0, 20.0),
            ("UIWindow2.img/Skill/main/backgrnd3", 174.0, 20.0),
        ],
        "native-pet-equipment" => &[("UIWindow.img/Equip/pet", 184.0, 20.0)],
        "native-npc-store" => &[
            ("UIWindow2.img/Shop2/backgrnd", 400.0, 20.0),
            ("UIWindow2.img/Shop2/backgrnd2", 400.0, 20.0),
            ("UIWindow2.img/Shop2/backgrnd3", 400.0, 20.0),
        ],
        "native-cash-shop" => &[("CashShop.img/Base/backgrnd", 0.0, 0.0)],
        "native-monster-book" => &[("UIWindow2.img/MonsterBook/backgrnd", 276.5, 20.0)],
        "native-status" => &[
            ("UIWindow4.img/Stat/main/backgrnd", 212.0, 20.0),
            ("UIWindow4.img/Stat/main/backgrnd2", 212.0, 20.0),
            ("UIWindow4.img/Stat/main/backgrnd3", 212.0, 20.0),
        ],
        "native-quest-log" => &[("UIWindow.img/Quest/backgrnd3", 280.0, 20.0)],
        "native-chat" => &[
            ("StatusBar2.img/mainBar/backgrnd", 512.0, 768.0),
            ("StatusBar2.img/mainBar/chatEnter", 512.0, 768.0),
            ("StatusBar2.img/mainBar/chatSpace2", 512.0, 768.0),
            ("StatusBar2.img/mainBar/chatCover", 512.0, 768.0),
            ("StatusBar2.img/mainBar/chatTarget/all", 512.0, 770.0),
            ("StatusBar2.img/chat/tapBarOver", -64.0, 639.0),
        ],
        "native-party" => &[
            ("UIWindow2.img/UserList/Main/backgrnd", 260.0, 20.0),
            ("UIWindow2.img/UserList/Main/backgrnd2", 260.0, 20.0),
            ("UIWindow2.img/UserList/Main/Party/partyOn", 260.0, 20.0),
            ("UIWindow2.img/UserList/Sheet2/0", 270.0, 135.0),
            ("UIWindow2.img/UserList/Sheet2/1", 270.0, 153.0),
        ],
        "native-buddy" => &[
            ("UIWindow2.img/UserList/Main/backgrnd", 260.0, 20.0),
            ("UIWindow2.img/UserList/Main/backgrnd2", 260.0, 20.0),
            ("UIWindow2.img/UserList/Main/Friend/title", 260.0, 20.0),
            (
                "UIWindow2.img/UserList/Main/Friend/CbCondition/text",
                260.0,
                20.0,
            ),
            ("UIWindow2.img/UserList/line", 392.0, 135.0),
        ],
        "native-guild" => &[
            ("GuildUI.img/backgrnd1", 200.0, 80.0),
            ("GuildUI.img/backgrnd2", 200.0, 80.0),
            ("GuildUI.img/top/flag/0", 200.0, 80.0),
            ("GuildUI.img/top/noGuildMark", 200.0, 80.0),
            ("GuildUI.img/guildInfo/layer:cover", 200.0, 80.0),
            ("GuildUI.img/member/layer:head", 212.0, 196.0),
            ("GuildUI.img/member/table/list", 225.0, 232.0),
        ],
        "native-alliance" => &[
            ("GuildUI.img/backgrnd1", 200.0, 80.0),
            ("GuildUI.img/backgrnd2", 200.0, 80.0),
            ("GuildUI.img/guildInfo/layer:cover", 200.0, 80.0),
            ("GuildUI.img/union/unionOff/layer:noUnion", 212.0, 196.0),
        ],
        "native-audio-settings" => &[
            ("UIWindow2.img/OptionMenu/backgrnd", 370.0, 105.0),
            ("UIWindow2.img/OptionMenu/backgrnd2", 370.0, 105.0),
            ("UIWindow2.img/OptionMenu/backgrnd3", 370.0, 105.0),
        ],
        _ => return Err(format!("no NX native window fixture for {name:?}")),
    };

    for (path, anchor_x, anchor_y) in layers {
        let first_sprite = sprites.len();
        let archive = if matches!(name, "native-items" | "native-quest-log") {
            legacy_ui_archive
        } else {
            ui_archive
        };
        append_sprite(&mut sprites, archive, path, *anchor_x, *anchor_y, 1.0)?;
        sprites[first_sprite].texture_key = format!("nx-window:{path}");
    }

    if name == "native-quickslots" {
        let keys = DEFAULT_QUICKSLOT_KEYS;
        let mut bindings = default_keymap_bindings();
        for (slot, binding) in [
            openstory_protocol::KeyBinding {
                kind: 2,
                action: 2_000_000,
            },
            openstory_protocol::KeyBinding {
                kind: 1,
                action: 1_001_004,
            },
            openstory_protocol::KeyBinding { kind: 8, action: 0 },
            openstory_protocol::KeyBinding {
                kind: 4,
                action: 52,
            },
            openstory_protocol::KeyBinding {
                kind: 3,
                action: 5_010_000,
            },
            openstory_protocol::KeyBinding::default(),
            openstory_protocol::KeyBinding::default(),
            openstory_protocol::KeyBinding::default(),
        ]
        .into_iter()
        .enumerate()
        {
            bindings[usize::from(keys[slot])] = binding;
        }
        let mut icon_cache = HashMap::new();
        let icons: [Option<(u32, u32, Arc<[u8]>)>; 8] = std::array::from_fn(|slot| {
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
                4 | 5 => decode_nx_icon(
                    ui_archive,
                    &format!("UIWindow.img/KeyConfig/icon/{}", binding.action),
                ),
                8 => decode_nx_icon(
                    ui_archive,
                    &format!("UIWindow.img/SkillMacro/Macroicon/{}/icon", binding.action),
                ),
                _ => None,
            }
        });
        let pixels = render_quickslot_panel_pixels(
            font,
            quickslot_background_image(legacy_ui_archive),
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

    if name == "native-keybindings" {
        let pixels = render_key_config_panel(
            font,
            &default_keymap_bindings(),
            &DEFAULT_QUICKSLOT_KEYS,
            None,
            None,
            0,
            &static_key_config_actions(),
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

    if name == "native-skill-macros" {
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
        let pixels = skill_macros::render(font, &editor, &skills);
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

fn preview_character() -> openstory_protocol::CharacterInfo {
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
        "native-keybindings"
            | "native-skill-macros"
            | "native-items"
            | "native-equipment"
            | "native-skills"
            | "native-pet-equipment"
            | "native-npc-store"
            | "native-cash-shop"
            | "native-monster-book"
            | "native-status"
            | "native-quest-log"
            | "native-quest-log-detail"
            | "native-chat"
            | "native-party"
            | "native-buddy"
            | "native-guild"
            | "native-alliance"
            | "native-audio-settings"
    );
    if native_window_preview {
        let nx_sprite_count = scene
            .sprites
            .iter()
            .filter(|sprite| sprite.texture_key.starts_with("nx-window:"))
            .count();
        report.push_str(&format!("NX native window layers: {nx_sprite_count}\n"));
    }
    let mut outside_count = 0;
    for (index, sprite) in scene.sprites.iter().enumerate() {
        let is_audited_ui_sprite = sprite.texture_key.starts_with("nx-window:");
        if native_window_preview && !is_audited_ui_sprite {
            continue;
        }
        let Some(visible_rect) = visible_sprite_rect(sprite) else {
            continue;
        };
        if native_window_preview {
            report.push_str(&format!(
                "NX native layer {:?}: {:.1},{:.1} {:.1}x{:.1}\n",
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
    output_dir.join(format!("{screen}-{width}x{height}.png"))
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
