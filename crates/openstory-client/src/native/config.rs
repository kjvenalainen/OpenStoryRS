fn character_selection_hardware_info() -> (String, String) {
    let configured_macs = std::env::var("OPENSTORY_MACS").ok();
    let configured_hwid = std::env::var("OPENSTORY_HWID").ok();
    if let (Some(macs), Some(hwid)) = (configured_macs.as_ref(), configured_hwid.as_ref()) {
        return (macs.clone(), hwid.clone());
    }
    let defaults = hardware_info::legacy_hardware_info();
    (
        configured_macs.unwrap_or(defaults.macs),
        configured_hwid.unwrap_or(defaults.hwid),
    )
}

use openstory_assets::{AssetDirectory, Bitmap, NodeValue, NxArchive, NxNode, REQUIRED_NX_FILES};
use openstory_core::foothold::{Foothold, FootholdTree};
use openstory_core::physics::{PhysicsBody, PhysicsEngine, PhysicsFlags, PhysicsMode};
use openstory_core::simulation::FixedStepClock;
use openstory_core::viewport::{LogicalSize, PhysicalSize, Point, Viewport};
use openstory_render::{BgraImage, BlendMode, GpuRenderer, GpuTexture, SpriteDraw};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
use winit::window::{Window, WindowId};

const FIELD_WIDTH: u32 = 150;
const FIELD_HEIGHT: u32 = 45;
// Keep the player in the lower third of the field so platforms and
// footholds below the camera target do not dominate the view.
const GAMEPLAY_CAMERA_CENTER_Y: f32 = 640.0;
const LOGIN_CARET_BLINK_INTERVAL: Duration = Duration::from_millis(530);
const FIELD_X: f32 = 567.0;
const ACCOUNT_Y: f32 = 319.0;
const PASSWORD_Y: f32 = 349.0;
const PIC_DIALOG_WIDTH: u32 = 520;
const PIC_DIALOG_HEIGHT: u32 = 190;
const CHARACTER_PAGE_SIZE: usize = 3;
const CASH_SHOP_PAGE_SIZE: usize = cash_shop_ui::CATALOG_PAGE_SIZE;
const OWL_PANEL_WIDTH: u32 = 464;
const OWL_PANEL_HEIGHT: u32 = 460;
const OWL_PANEL_X: f32 = 280.0;
const OWL_PANEL_Y: f32 = 154.0;
const HIRED_MERCHANT_PANEL_WIDTH: u32 = 700;
const HIRED_MERCHANT_PANEL_HEIGHT: u32 = 600;
const HIRED_MERCHANT_PANEL_X: f32 = 162.0;
const HIRED_MERCHANT_PANEL_Y: f32 = 84.0;
const TRADE_PANEL_WIDTH: u32 = 720;
const TRADE_PANEL_HEIGHT: u32 = 600;
const TRADE_PANEL_X: f32 = 152.0;
const TRADE_PANEL_Y: f32 = 84.0;
const CHARACTER_INFO_PANEL_WIDTH: u32 = 392;
const CHARACTER_INFO_PANEL_HEIGHT: u32 = 492;
const CHARACTER_INFO_PANEL_X: f32 = 316.0;
const CHARACTER_INFO_PANEL_Y: f32 = 157.0;
const CHAT_WIDTH: u32 = 470;
const CHAT_HEIGHT: u32 = 166;
const ITEM_MEGAPHONE_X: f32 = (1024.0 - item_megaphone::WIDTH as f32) * 0.5;
const ITEM_MEGAPHONE_Y: f32 = (768.0 - item_megaphone::HEIGHT as f32) * 0.5;
const WHISPER_PANEL_WIDTH: u32 = 368;
const WHISPER_PANEL_HEIGHT: u32 = 284;
const WHISPER_PANEL_X: f32 = 644.0;
const WHISPER_PANEL_Y: f32 = 410.0;
const MAX_WHISPER_CONVERSATIONS: usize = 32;
const MAX_WHISPER_LINES: usize = 100;
const MAX_WHISPER_LINE_CHARS: usize = 200;
const PARTY_PANEL_WIDTH: u32 = 284;
const PARTY_PANEL_HEIGHT: u32 = 238;
const GUILD_PANEL_WIDTH: u32 = 336;
const GUILD_PANEL_HEIGHT: u32 = 420;
const FAMILY_PANEL_WIDTH: u32 = 274;
const FAMILY_PANEL_HEIGHT: u32 = 439;
const FAMILY_TREE_PANEL_WIDTH: u32 = 578;
const FAMILY_TREE_PANEL_HEIGHT: u32 = 386;
const ALLIANCE_PANEL_WIDTH: u32 = 420;
const ALLIANCE_PANEL_HEIGHT: u32 = 386;
const GUILD_BBS_PANEL_WIDTH: u32 = 824;
const GUILD_BBS_PANEL_HEIGHT: u32 = 548;
const GUILD_BBS_TITLE_LIMIT: usize = openstory_protocol::MAX_BBS_TITLE_LENGTH;
const GUILD_BBS_CONTENT_LIMIT: usize = openstory_protocol::MAX_BBS_CONTENT_LENGTH;
const GUILD_BBS_REPLY_LIMIT: usize = openstory_protocol::MAX_BBS_REPLY_LENGTH;
const BUDDY_PANEL_WIDTH: u32 = 284;
const BUDDY_PANEL_HEIGHT: u32 = 292;
const SETTINGS_PANEL_X: f32 = 272.0;
const SETTINGS_PANEL_Y: f32 = 219.0;
const SETTINGS_PANEL_WIDTH: u32 = 480;
const SETTINGS_PANEL_HEIGHT: u32 = 330;
const STATUS_PANEL_WIDTH: u32 = 212;
const STATUS_PANEL_HEIGHT: u32 = 318;
const STATUS_PANEL_X: f32 = 212.0;
const STATUS_PANEL_Y: f32 = 20.0;
const INVENTORY_PANEL_X: f32 = 172.0;
const INVENTORY_PANEL_Y: f32 = 20.0;
const INVENTORY_PANEL_WIDTH: u32 = 175;
const INVENTORY_PANEL_HEIGHT: u32 = 289;
const INVENTORY_SLOT_COLUMNS: usize = 4;
const INVENTORY_SLOT_ROWS: usize = 6;
const INVENTORY_SLOT_X: usize = 10;
const INVENTORY_SLOT_Y: usize = 51;
const INVENTORY_SLOT_STEP_X: usize = 36;
const INVENTORY_SLOT_STEP_Y: usize = 35;
const QUEST_LOG_PANEL_X: f32 = 280.0;
const QUEST_LOG_PANEL_Y: f32 = 20.0;
const QUEST_LOG_PANEL_WIDTH: u32 = 598;
const QUEST_LOG_PANEL_HEIGHT: u32 = 396;
const QUEST_LOG_DETAIL_X: usize = 293;
const QUEST_LOG_VISIBLE_ROWS: usize = 8;
const KEY_CONFIG_PANEL_WIDTH: u32 = 424;
const KEY_CONFIG_PANEL_HEIGHT: u32 = 456;
const KEY_CONFIG_PANEL_X: f32 = 300.0;
const KEY_CONFIG_PANEL_Y: f32 = 176.0;
const DEFAULT_QUICKSLOT_KEYS: [u8; 8] = [42, 82, 71, 73, 29, 83, 79, 81];
const KEY_CONFIG_VISIBLE_ROWS: usize = 8;
const KEY_CONFIG_ROW_TOP: i32 = 80;
const KEY_CONFIG_ROW_HEIGHT: i32 = 32;
const MAX_CHAT_MESSAGE_CHARS: usize = 100;
const STATUS_BAR_TOP: f32 = 697.0;
const STATUS_MENU_WIDTH: u32 = 126;
const STATUS_MENU_ROW_HEIGHT: usize = 18;
const STATUS_MENU_ROWS: [&str; 12] = [
    "Stats",
    "Skills",
    "Quests",
    "Inventory",
    "Equipment",
    "Party",
    "Buddy List",
    "Guild",
    "Monster Book",
    "World Map",
    "Key Settings",
    "Options",
];
const STATUS_MENU_HEIGHT: u32 = STATUS_MENU_ROWS.len() as u32 * STATUS_MENU_ROW_HEIGHT as u32 + 8;
const STATUS_MENU_X: f32 = 794.0;
const STATUS_MENU_Y: f32 = STATUS_BAR_TOP - STATUS_MENU_HEIGHT as f32;
const SPEECH_BUBBLE_WIDTH: u32 = 360;
const SPEECH_BUBBLE_HEIGHT: u32 = 132;
const SPEECH_BUBBLE_ANCHOR: (i32, i32) = (180, 110);
const SPEECH_BUBBLE_DURATION: Duration = Duration::from_secs(4);
// Mob.cpp advances opacity by 0.025 per 8 ms client update, which is a
// 40-step (320 ms) full-opacity transition on the fixed simulation clock.
const MOB_FADE_DURATION: Duration = Duration::from_millis(320);
const STATUS_SLOW: u64 = 0x0000_0000_0000_0001;
const STATUS_SEDUCE: u64 = 0x0000_0000_0000_0080;
// v83 aliases Zombify disease and the Puppet buff on this mask. Disease
// packets carry a mob-skill type that distinguishes Zombify from Puppet.
const STATUS_ZOMBIFY: u64 = 0x0000_0000_0000_4000;
const MOB_SKILL_TYPE_UNDEAD: u16 = 133;
const SKILL_HEAL: i32 = 2_301_002;
const ZOMBIFY_HEAL_FEEDBACK_WINDOW: Duration = Duration::from_millis(1_500);
const STATUS_CONFUSE: u64 = 0x0000_0000_0008_0000;
const STATUS_STUN: u64 = 0x0002_0000_0000_0000;
const STATUS_POISON: u64 = 0x0004_0000_0000_0000;
const STATUS_SEAL: u64 = 0x0008_0000_0000_0000;
const STATUS_DARKNESS: u64 = 0x0010_0000_0000_0000;
const STATUS_WEAKEN: u64 = 0x4000_0000_0000_0000;
const STATUS_CURSE: u64 = 0x8000_0000_0000_0000;
const STATUS_DARKSIGHT: u64 = 0x0000_0400_0000_0000;
fn mob_disease_skill_parts(encoded_skill_id: i32) -> Option<(u16, u16)> {
    let encoded = encoded_skill_id as u32;
    let skill_type = encoded as u16;
    let skill_level = (encoded >> 16) as u16;
    let is_disease = matches!(
        skill_type,
        120 | 121 | 122 | 123 | 124 | 125 | 126 | 128 | 132 | 133
    );
    (is_disease && skill_level != 0).then_some((skill_type, skill_level))
}

const NPC_DIALOGUE_SCROLL_STEP: usize = 16;
const CASH_SHOP_CATEGORIES: [&str; 9] = [
    "All items",
    "Equipment",
    "Use",
    "Set-up",
    "Etc.",
    "Cash",
    "Pets",
    "Pet gear",
    "Pet misc",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoginField {
    Account,
    Password,
}
