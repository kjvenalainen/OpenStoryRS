mod audio;
mod avatar_megaphone;
mod cash_shop_ui;
mod channel_switch;
mod hardware_info;
mod item_megaphone;
mod messenger;
mod minimap;
mod monster_book;
mod monster_carnival;
mod party_search;
mod settings;
mod skill_macros;
mod ui_clock;
mod ui_preview;
mod weather;
mod world_map;

use super::session;

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use settings::{
    AudioSettings, DisplaySettings, GameplaySettings, InputSettings, LoginPreferences,
    SocialSettings, apply_mouse_speed, load_quickslot_keys, save as save_settings,
    save_quickslot_keys,
};

include!("native/config.rs");
include!("native/run.rs");
include!("native/client_state.rs");
include!("native/model.rs");
include!("native/debug.rs");

// Feature-focused inherent impls share native state through this parent module.
mod client_avatar_megaphone;
mod client_cash_skills;
mod client_core;
mod client_events;
mod client_family_guild;
mod client_gameplay;
mod client_guide;
mod client_inventory_storage;
mod client_login;
mod client_map_entities;
mod client_monster_carnival;
mod client_network;
mod client_packets;
mod client_quests_inventory;
mod client_selection;
mod client_settings_chat;
mod client_social;
mod client_world_overlays;
mod event_loop;

// Shared private UI and scene helpers stay in native scope for existing call sites.
include!("native/ui_bootstrap.rs");
include!("native/ui_quests_status.rs");
include!("native/ui_controls.rs");
include!("native/ui_social.rs");
include!("native/ui_family_guild.rs");
include!("native/ui_chat_inventory.rs");
include!("native/ui_cash_storage.rs");
include!("native/ui_trade_merchant.rs");
include!("native/ui_inventory_data.rs");
include!("native/ui_npc.rs");
include!("native/ui_status_bar.rs");
include!("native/ui_wedding.rs");
include!("native/ui_event_list.rs");
include!("native/ui_rps_game.rs");
include!("native/map_effect_draw.rs");
include!("native/map_entity_draw.rs");
include!("native/map_damage_draw.rs");
include!("native/login_scenes.rs");
include!("native/map_scene.rs");
include!("native/character_preview.rs");
include!("native/combat_rules.rs");
include!("native/avatar_render.rs");
include!("native/map_animations.rs");
include!("native/map_assets.rs");
include!("native/icon_assets.rs");
include!("native/ui_sprite_helpers.rs");
