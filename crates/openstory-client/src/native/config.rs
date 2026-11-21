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
// Keep camera bounds above the status bar instead of using the full canvas.
const GAMEPLAY_VIEW_HEIGHT: f32 = STATUS_BAR_TOP;
// Keep the player's feet above the HUD with enough room to see the ground below.
// All map actors share this anchor when translating world coordinates to screen.
const GAMEPLAY_CAMERA_CENTER_Y: f32 = 620.0;
const LOGIN_CARET_BLINK_INTERVAL: Duration = Duration::from_millis(530);
const FIELD_X: f32 = 567.0;
const ACCOUNT_Y: f32 = 319.0;
const PASSWORD_Y: f32 = 349.0;
const PIC_DIALOG_WIDTH: u32 = 520;
const PIC_DIALOG_HEIGHT: u32 = 190;
const CHARACTER_PAGE_SIZE: usize = 3;
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
const CHARACTER_INFO_PANEL_WIDTH: u32 = 550;
const CHARACTER_INFO_PANEL_HEIGHT: u32 = 434;
const CHARACTER_INFO_PANEL_X: f32 = 237.0;
const CHARACTER_INFO_PANEL_Y: f32 = 167.0;
const CHAT_X: f32 = 0.0;
const CHAT_ROWS: usize = 5;
const CHAT_HISTORY_HEIGHT: usize = 70;
const CHAT_Y: f32 = STATUS_BAR_TOP - CHAT_HISTORY_HEIGHT as f32;
const CHAT_TOGGLE_X: usize = 535;
const CHAT_TOGGLE_Y: usize = CHAT_HISTORY_HEIGHT + 10;
const CHAT_WIDTH: u32 = 575;
const CHAT_HEIGHT: u32 = CHAT_HISTORY_HEIGHT as u32 + 31;
const ITEM_MEGAPHONE_X: f32 = (1024.0 - item_megaphone::WIDTH as f32) * 0.5;
const ITEM_MEGAPHONE_Y: f32 = (768.0 - item_megaphone::HEIGHT as f32) * 0.5;
const WHISPER_PANEL_WIDTH: u32 = 295;
const WHISPER_PANEL_HEIGHT: u32 = 364;
const WHISPER_PANEL_X: f32 = 610.0;
const WHISPER_PANEL_Y: f32 = 235.0;
const MAX_WHISPER_CONVERSATIONS: usize = 32;
const MAX_WHISPER_LINES: usize = 100;
const MAX_WHISPER_LINE_CHARS: usize = 200;
const PARTY_PANEL_WIDTH: u32 = 312;
const PARTY_PANEL_HEIGHT: u32 = 389;
const PARTY_PANEL_X: f32 = 350.0;
const PARTY_PANEL_Y: f32 = 180.0;
const GUILD_PANEL_WIDTH: u32 = 322;
const GUILD_PANEL_HEIGHT: u32 = 420;
const GUILD_PANEL_X: f32 = 305.0;
const GUILD_PANEL_Y: f32 = 155.0;
const FAMILY_PANEL_WIDTH: u32 = 274;
const FAMILY_PANEL_HEIGHT: u32 = 439;
const FAMILY_TREE_PANEL_WIDTH: u32 = 578;
const FAMILY_TREE_PANEL_HEIGHT: u32 = 386;
const ALLIANCE_PANEL_WIDTH: u32 = 296;
const ALLIANCE_PANEL_HEIGHT: u32 = 386;
const GUILD_BBS_PANEL_WIDTH: u32 = 824;
const GUILD_BBS_PANEL_HEIGHT: u32 = 548;
const GUILD_BBS_TITLE_LIMIT: usize = openstory_protocol::MAX_BBS_TITLE_LENGTH;
const GUILD_BBS_CONTENT_LIMIT: usize = openstory_protocol::MAX_BBS_CONTENT_LENGTH;
const GUILD_BBS_REPLY_LIMIT: usize = openstory_protocol::MAX_BBS_REPLY_LENGTH;
const BUDDY_PANEL_WIDTH: u32 = 312;
const BUDDY_PANEL_HEIGHT: u32 = 389;
const BUDDY_PANEL_X: f32 = 520.0;
const BUDDY_PANEL_Y: f32 = 175.0;
const SETTINGS_PANEL_X: f32 = 283.0;
const SETTINGS_PANEL_Y: f32 = 201.0;
const SETTINGS_PANEL_WIDTH: u32 = 458;
const SYSTEM_OPTIONS_FOOTER_Y: usize = 270;
const SETTINGS_PANEL_HEIGHT: u32 = SYSTEM_OPTIONS_FOOTER_Y as u32 + 36;
const SYSTEM_OPTIONS_MUTE_CELL_X: usize = 239;
const SYSTEM_OPTIONS_ROW_RIGHT: usize = 288;
const SYSTEM_OPTIONS_CHECKBOX_RIGHT: usize = SYSTEM_OPTIONS_ROW_RIGHT - 8;
const SYSTEM_OPTIONS_BUTTONS: [ui_foundation::UiRect; 2] = ui_dialog::right_aligned_footer(
    SYSTEM_OPTIONS_ROW_RIGHT as f32,
    SETTINGS_PANEL_HEIGHT as f32,
    [47.0, 47.0],
);
const SYSTEM_OPTIONS_OK_BUTTON: ui_foundation::UiRect = SYSTEM_OPTIONS_BUTTONS[0];
const SYSTEM_OPTIONS_CANCEL_BUTTON: ui_foundation::UiRect = SYSTEM_OPTIONS_BUTTONS[1];
const SKILL_BOOK_PANEL_WIDTH: u32 = 177;
const SKILL_BOOK_PANEL_HEIGHT: u32 = 292;
const SKILL_BOOK_PANEL_X: f32 = 608.0;
const SKILL_BOOK_PANEL_Y: f32 = 286.0;
const SKILL_BOOK_VISIBLE_ROWS: usize = 4;
const STATUS_PANEL_WIDTH: u32 = 175;
const STATUS_PANEL_HEIGHT: u32 = 347;
const STATUS_PANEL_X: f32 = 365.0;
const STATUS_PANEL_Y: f32 = 105.0;
const STATUS_DETAIL_WIDTH: u32 = 177;
const STATUS_DETAIL_HEIGHT: u32 = 203;
const STATUS_DETAIL_OFFSET_X: f32 = 174.0;
const STATUS_DETAIL_OFFSET_Y: f32 = (STATUS_PANEL_HEIGHT - STATUS_DETAIL_HEIGHT) as f32;
const STATUS_MESSAGE_LIMIT: usize = 6;
const STATUS_MESSAGE_DURATION: Duration = Duration::from_secs(8);
const STATUS_MESSAGE_WIDTH: u32 = 520;
const STATUS_MESSAGE_HEIGHT: u32 = 18;
const STATUS_MESSAGE_RIGHT: f32 = 1018.0;
const STATUS_MESSAGE_Y: f32 = 660.0;
const STATUS_MESSAGE_ROW_STEP: f32 = 14.0;
const QUICKSLOT_PANEL_Y: f32 = 615.0;
const RPS_GAME_PANEL_WIDTH: u32 = 310;
const RPS_GAME_PANEL_X: f32 = 512.0;
const RPS_GAME_PANEL_Y: f32 = 334.0;
const PET_EQUIPMENT_PANEL_WIDTH: u32 = 260;
const PET_EQUIPMENT_PANEL_HEIGHT: u32 = EQUIPMENT_INVENTORY_PANEL_HEIGHT;
const PET_EQUIPMENT_PANEL_X: f32 = EQUIPMENT_INVENTORY_PANEL_X + EQUIPMENT_INVENTORY_PANEL_WIDTH as f32;
const PET_EQUIPMENT_PANEL_Y: f32 = EQUIPMENT_INVENTORY_PANEL_Y;
const PET_EQUIPMENT_GEAR_LIST_Y: f32 = 158.0;
const PET_EQUIPMENT_GEAR_ROW_HEIGHT: f32 = 39.0;
const PET_EQUIPMENT_VISIBLE_ROWS: usize = 3;
const INVENTORY_PANEL_X: f32 = 590.0;
const INVENTORY_PANEL_Y: f32 = 176.0;
const INVENTORY_PANEL_WIDTH: u32 = 176;
const INVENTORY_PANEL_HEIGHT: u32 = 291;
const EQUIPMENT_INVENTORY_PANEL_X: f32 = 390.0;
const EQUIPMENT_INVENTORY_PANEL_Y: f32 = 260.0;
const EQUIPMENT_INVENTORY_PANEL_WIDTH: u32 = 175;
const EQUIPMENT_INVENTORY_PANEL_HEIGHT: u32 = 304;
const INVENTORY_TOOLTIP_WIDTH: u32 = 237;
const INVENTORY_TOOLTIP_HEIGHT: u32 = 224;
const INVENTORY_SLOT_COLUMNS: usize = 4;
const INVENTORY_SLOT_ROWS: usize = 6;
const INVENTORY_SLOT_X: usize = 10;
const INVENTORY_SLOT_Y: usize = 51;
const INVENTORY_SLOT_CELL_WIDTH: usize = 33;
const INVENTORY_SLOT_CELL_HEIGHT: usize = 32;
const INVENTORY_SLOT_STEP_X: usize = 35;
const INVENTORY_SLOT_STEP_Y: usize = 34;
const QUEST_LOG_PANEL_X: f32 = 238.0;
const QUEST_LOG_PANEL_Y: f32 = 186.0;
const QUEST_LOG_PANEL_WIDTH: u32 = 547;
const QUEST_LOG_PANEL_HEIGHT: u32 = 396;
const QUEST_LOG_DETAIL_X: usize = 242;
const QUEST_LOG_DETAIL_WIDTH: u32 = 305;
const QUEST_LOG_LIST_WIDTH: u32 = 245;
const QUEST_LOG_VISIBLE_ROWS: usize = 15;
const QUEST_HELPER_PANEL_WIDTH: u32 = 202;
const QUEST_HELPER_PANEL_X: f32 = 806.0;
const QUEST_HELPER_PANEL_Y: f32 = 90.0;
const KEY_CONFIG_PANEL_WIDTH: u32 = 633;
const KEY_CONFIG_PANEL_HEIGHT: u32 = 343;
const KEY_CONFIG_PANEL_X: f32 = (1024.0 - KEY_CONFIG_PANEL_WIDTH as f32) * 0.5;
const KEY_CONFIG_PANEL_Y: f32 = (768.0 - KEY_CONFIG_PANEL_HEIGHT as f32) * 0.5;
const DEFAULT_QUICKSLOT_KEYS: [u8; 8] = [42, 82, 71, 73, 29, 83, 79, 81];
const KEY_CONFIG_VISIBLE_ROWS: usize = 2;
const MAX_CHAT_MESSAGE_CHARS: usize = 100;
const STATUS_BAR_TOP: f32 = 697.0;
const V83_STATUS_BAR_WIDTH: f32 = 1024.0;
const V83_LEVEL_PANEL_EXTRA_WIDTH: usize = 12;
const V83_STATUS_BAR_EDGE_PAD: f32 = 8.0;
const V83_STATUS_BAR_BIG_BUTTON_WIDTH: f32 = 54.0;
const V83_STATUS_BAR_BIG_BUTTON_STEP: f32 = 75.0;
const V83_STATUS_BAR_BIG_BUTTON_X: f32 = V83_STATUS_BAR_WIDTH
    - V83_STATUS_BAR_EDGE_PAD
    - (3.0 * V83_STATUS_BAR_BIG_BUTTON_WIDTH
        + 2.0 * (V83_STATUS_BAR_BIG_BUTTON_STEP - V83_STATUS_BAR_BIG_BUTTON_WIDTH));
const GAME_MENU_WIDTH: u32 = 174;
const GAME_MENU_HEIGHT: u32 = 103;
const GAME_MENU_ROWS: [&str; 3] = ["Change Channel", "System Menu", "Quit Game"];
const EVENT_LIST_PANEL_X: f32 = (1024.0 - EVENT_LIST_PANEL_WIDTH as f32) * 0.5;
const EVENT_LIST_PANEL_Y: f32 = (768.0 - EVENT_LIST_PANEL_HEIGHT as f32) * 0.5;
const SPEECH_BUBBLE_WIDTH: u32 = 360;
const SPEECH_BUBBLE_HEIGHT: u32 = 132;
const SPEECH_BUBBLE_ANCHOR: (i32, i32) = (180, 110);
const SPEECH_BUBBLE_DURATION: Duration = Duration::from_secs(4);
const CHALKBOARD_WIDTH: u32 = 360;
const CHALKBOARD_HEIGHT: u32 = 52;
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
const STATUS_MONSTER_RIDING: u64 = 0x0020_0000_0000_0000;
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
    "Setup",
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
