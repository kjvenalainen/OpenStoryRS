#![forbid(unsafe_code)]

//! Safe, bounded MapleStory packet primitives and v83 framing/cipher support.

mod alliance;
mod capture;
mod cash_shop;
mod events;
mod family;
mod feedback;
mod gameplay;
mod guide;
mod guild;
mod guild_bbs;
mod inventory;
mod keymap;
mod login;
mod maker;
mod maple;
mod messenger;
mod monster_book;
mod packet;
mod party_search;
mod quest;
mod rps;
mod session;
mod skill_macros;
mod skills;
mod social;
mod stream;
mod teleport_rock;
mod trade;
mod ui;

pub use alliance::{
    AllianceEvent, AllianceGuild, AllianceInfo, OPCODE_ALLIANCE_OPERATION,
    OPCODE_ALLIANCE_OPERATION_SEND, OPCODE_DENY_ALLIANCE_REQUEST, alliance_accept_invite_packet,
    alliance_expel_guild_packet, alliance_info_request_packet, alliance_invite_packet,
    alliance_leave_packet, alliance_notice_packet, alliance_rank_titles_packet,
    deny_alliance_invite_packet, parse_alliance_operation,
};
pub use capture::{
    CatchResult, OPCODE_BRIDLE_MOB_CATCH_FAIL, OPCODE_CATCH_MONSTER,
    OPCODE_CATCH_MONSTER_WITH_ITEM, parse_catch_result,
};
pub use cash_shop::{
    CashShopItem, CashShopOperation, OPCODE_BUY_CASH_ITEM, OPCODE_CASH_SHOP_OPERATION,
    OPCODE_COUPON_CODE, OPCODE_ENTER_CASH_SHOP, OPCODE_QUERY_CASH_RESULT, OPCODE_SET_CASH_SHOP,
    buy_cash_item_packet, cash_shop_coupon_packet, enter_cash_shop_packet, exit_cash_shop_packet,
    parse_cash_balances, parse_cash_shop_open, parse_cash_shop_operation,
    withdraw_cash_item_packet,
};
pub use events::{
    EventReward, OPCODE_EVENT_INFO, OPCODE_REQUEST_EVENT_INFO, ServerEventInfo, parse_event_info,
    request_event_info_packet,
};
pub use family::{
    FamilyChart, FamilyChartCounter, FamilyChartEntry, FamilyEntitlementUsage, FamilyEvent,
    FamilyInfo, FamilyPrivilege, OPCODE_ACCEPT_FAMILY, OPCODE_ADD_FAMILY,
    OPCODE_CHANGE_FAMILY_MESSAGE, OPCODE_FAMILY_CHART_RESULT, OPCODE_FAMILY_INFO,
    OPCODE_FAMILY_JOIN_ACCEPTED, OPCODE_FAMILY_JOIN_REQUEST, OPCODE_FAMILY_JOIN_REQUEST_RESULT,
    OPCODE_FAMILY_NOTIFY_LOGIN, OPCODE_FAMILY_PRIVILEGE_LIST, OPCODE_FAMILY_REP_GAIN,
    OPCODE_FAMILY_RESULT, OPCODE_FAMILY_SUMMON_REQUEST, OPCODE_FAMILY_SUMMON_RESPONSE,
    OPCODE_OPEN_FAMILY, OPCODE_OPEN_FAMILY_PEDIGREE, OPCODE_SEPARATE_FAMILY_BY_JUNIOR,
    OPCODE_SEPARATE_FAMILY_BY_SENIOR, OPCODE_USE_FAMILY, family_accept_request_packet,
    family_add_junior_packet, family_leave_packet, family_pedigree_request_packet,
    family_precepts_packet, family_summon_response_packet, family_use_packet, open_family_packet,
    parse_family_packet,
};
pub use feedback::{
    CancelRequestKind, CancelRequestResponse, CashShopFeedback, OPCODE_BLOCKED_MAP,
    OPCODE_BLOCKED_SERVER, OPCODE_CANCEL_NAME_CHANGE, OPCODE_CANCEL_WORLD_TRANSFER,
    OPCODE_CANNOT_SPAWN_KITE, OPCODE_CLAIM_STATUS_CHANGED, OPCODE_CONTI_MOVE, OPCODE_CONTI_STATE,
    OPCODE_CS_CASH_GACHAPON_RESULT, OPCODE_CS_CHECK_NAME_CHANGE, OPCODE_CS_NAME_CHANGE_POSSIBLE,
    OPCODE_CS_TRANSFER_WORLD, OPCODE_FORCED_MAP_EQUIP, OPCODE_INCUBATOR_RESULT,
    OPCODE_MAPLELIFE_ERROR, OPCODE_MAPLELIFE_RESULT, OPCODE_NEW_YEAR_CARD, OPCODE_PING,
    OPCODE_REMOVE_KITE, OPCODE_SCRIPT_PROGRESS_MESSAGE, OPCODE_SPAWN_KITE,
    OPCODE_SUE_CHARACTER_RESULT, OPCODE_TRADE_MONEY_LIMIT, OPCODE_VEGA_SCROLL,
    OPCODE_VICIOUS_HAMMER, OPCODE_WEEK_EVENT_MESSAGE, ServerFeedback, parse_server_feedback,
};
pub use gameplay::{
    AbilityStat, AttackDamageTarget, AttackType, AvatarMegaphone, BlowWeatherUpdate, BuddyEntry,
    BuddyEvent, BuffMaskGroup, BuffStatKey, CharacterBuffCancel, CharacterBuffEntry,
    CharacterBuffUpdate, CharacterInfoPet, CharacterInfoReply, CharacterMedalInfo,
    CharacterMiniRoom, CharacterMonsterBookInfo, CharacterMountInfo, CharacterStatChange,
    CharacterStatUpdate, ChatMessage, ClockUpdate, EventNotification, FameResponse, FieldEffect,
    FieldObstacleToggle, ForcedStatSet, ForeignCharacterBuffCancel, ForeignCharacterBuffUpdate,
    ForeignCharacterEffect, ForeignMobDiseaseEffect, ForeignSkillEffect, GrenadeEffect, GuildMark,
    ImitatedNpcUpdate, ItemGainInChat, MapChairCancel, MapChairShow, MapDoorSpawn,
    MapDragonMovement, MapDragonSpawn, MapDropRemove, MapDropSpawn, MapMistSpawn, MapMobMovement,
    MapMobSpawn, MapNpcSpawn, MapPetMovement, MapPetSpawn, MapReactorSpawn, MapReactorState,
    MapSummonMovement, MapSummonRemove, MapSummonSpawn, MapTownPortalUpdate, MapleTvBroadcast,
    MobControllerUpdate, MobDamage, MobHpUpdate, MobMovementResponse, MobStatusCancel,
    MobStatusEntry, MobStatusMaskGroup, MobStatusUpdate, MonsterCarnivalEvent, MovementFragment,
    MovementKind, MultiChatMessage, MultiChatType, NpcAction, NpcControllerUpdate, NpcDialogue,
    NpcShop, NpcShopItem, OPCODE_ADD_COOLDOWN, OPCODE_APPLY_MOB_STATUS,
    OPCODE_ARIANT_ARENA_SHOW_RESULT, OPCODE_ARIANT_ARENA_USER_SCORE, OPCODE_ATTACKED_CLOSE,
    OPCODE_ATTACKED_MAGIC, OPCODE_ATTACKED_RANGED, OPCODE_AUTO_HP_POT, OPCODE_AUTO_MP_POT,
    OPCODE_BLOW_WEATHER, OPCODE_BROADCAST_MSG, OPCODE_BUDDY_LIST, OPCODE_BUDDY_LIST_MODIFY,
    OPCODE_CANCEL_BUFF, OPCODE_CANCEL_CHAIR_BROADCAST, OPCODE_CANCEL_FOREIGN_BUFF,
    OPCODE_CANCEL_MOB_STATUS, OPCODE_CANCEL_SKILL_EFFECT, OPCODE_CHALKBOARD, OPCODE_CHANGE_CHANNEL,
    OPCODE_CHANGE_MAP, OPCODE_CHARACTER_INFO, OPCODE_CHARACTER_INFO_REQUEST,
    OPCODE_CHARACTER_MOVED, OPCODE_CHARACTER_STAT_UPDATE, OPCODE_CHAT_RECEIVED,
    OPCODE_CLEAR_AVATAR_MEGAPHONE, OPCODE_CLOCK, OPCODE_CLOSE_ATTACK, OPCODE_COCONUT_HIT,
    OPCODE_COCONUT_SCORE, OPCODE_CONFIRM_NPC_SHOP_TRANSACTION, OPCODE_DAMAGE_MOB,
    OPCODE_DAMAGE_PLAYER, OPCODE_DAMAGE_REACTOR, OPCODE_DAMAGE_SUMMON, OPCODE_DENY_PARTY_INVITE,
    OPCODE_DOJO_WARP_UP, OPCODE_DROP_LOOT, OPCODE_ENABLE_TV, OPCODE_FACE_EXPRESSION,
    OPCODE_FACIAL_EXPRESSION, OPCODE_FAKE_GM_NOTICE, OPCODE_FAME_RESPONSE, OPCODE_FIELD_EFFECT,
    OPCODE_FIELD_OBSTACLE_ONE, OPCODE_FIELD_OBSTACLE_ONOFF, OPCODE_FORCED_STAT_SET,
    OPCODE_GENERAL_CHAT, OPCODE_GIVE_BUFF, OPCODE_GIVE_FAME, OPCODE_GIVE_FOREIGN_BUFF,
    OPCODE_GUILD_MARK_CHANGED, OPCODE_GUILD_NAME_CHANGED, OPCODE_HEAL_OVER_TIME,
    OPCODE_HIT_REACTOR, OPCODE_HIT_SNOWBALL, OPCODE_IMITATED_NPC, OPCODE_KILL_MOB,
    OPCODE_LEFT_KNOCK_BACK, OPCODE_MAGIC_ATTACK, OPCODE_MOB_MOVED, OPCODE_MONSTER_CARNIVAL_CP,
    OPCODE_MONSTER_CARNIVAL_DIED, OPCODE_MONSTER_CARNIVAL_MESSAGE,
    OPCODE_MONSTER_CARNIVAL_PARTY_CP, OPCODE_MONSTER_CARNIVAL_START,
    OPCODE_MONSTER_CARNIVAL_SUMMON, OPCODE_MOVE_DRAGON, OPCODE_MOVE_MOB, OPCODE_MOVE_MOB_RESPONSE,
    OPCODE_MOVE_PET, OPCODE_MOVE_PLAYER, OPCODE_MOVE_SUMMON, OPCODE_MULTI_CHAT_RECEIVED,
    OPCODE_MULTI_CHAT_SEND, OPCODE_NPC_ACTION, OPCODE_NPC_DIALOGUE, OPCODE_NPC_SHOP_ACTION,
    OPCODE_NPC_TALK_MORE, OPCODE_OPEN_NPC_SHOP, OPCODE_OX_QUIZ, OPCODE_PARTY_OPERATION,
    OPCODE_PARTY_OPERATION_SEND, OPCODE_PET_CHAT_RECEIVED, OPCODE_PET_COMMAND_RECEIVED,
    OPCODE_PET_EXCEPTION_LIST, OPCODE_PET_LOOT, OPCODE_PET_NAME_CHANGE, OPCODE_PICKUP_ITEM,
    OPCODE_PLAYER_HINT, OPCODE_PYRAMID_GAUGE, OPCODE_PYRAMID_SCORE, OPCODE_RANGED_ATTACK,
    OPCODE_RECALCULATE_STATS, OPCODE_REMOVE_CHARACTER, OPCODE_REMOVE_DOOR, OPCODE_REMOVE_DRAGON,
    OPCODE_REMOVE_LOOT, OPCODE_REMOVE_MIST, OPCODE_REMOVE_NPC, OPCODE_REMOVE_REACTOR,
    OPCODE_REMOVE_SUMMON, OPCODE_REMOVE_TV, OPCODE_SCROLL_RESULT, OPCODE_SEND_TV,
    OPCODE_SERVER_MESSAGE, OPCODE_SET_AVATAR_MEGAPHONE, OPCODE_SET_BACK_EFFECT,
    OPCODE_SET_NPC_SCRIPTABLE, OPCODE_SET_TAMING_MOB_INFO, OPCODE_SET_TRACTION, OPCODE_SHOW_CHAIR,
    OPCODE_SHOW_COMBO, OPCODE_SHOW_FOREIGN_EFFECT, OPCODE_SHOW_ITEM_EFFECT,
    OPCODE_SHOW_ITEM_GAIN_INCHAT, OPCODE_SHOW_MOB_HP, OPCODE_SKILL_EFFECT,
    OPCODE_SKILL_LEARN_ITEM_RESULT, OPCODE_SNOWBALL_MESSAGE, OPCODE_SNOWBALL_STATE,
    OPCODE_SPAWN_CHARACTER, OPCODE_SPAWN_DOOR, OPCODE_SPAWN_DRAGON, OPCODE_SPAWN_MIST,
    OPCODE_SPAWN_MOB, OPCODE_SPAWN_MOB_CONTROLLER, OPCODE_SPAWN_NPC, OPCODE_SPAWN_NPC_CONTROLLER,
    OPCODE_SPAWN_PET_RECEIVED, OPCODE_SPAWN_PORTAL, OPCODE_SPAWN_REACTOR, OPCODE_SPAWN_SUMMON,
    OPCODE_SPEND_ABILITY_POINT, OPCODE_STOP_CLOCK, OPCODE_SUMMON_ATTACK, OPCODE_SUMMON_SKILL,
    OPCODE_TAKE_DAMAGE, OPCODE_TALK_TO_NPC, OPCODE_THROW_GRENADE, OPCODE_TOURNAMENT,
    OPCODE_TOURNAMENT_MATCH_TABLE, OPCODE_TOURNAMENT_SET_PRIZE, OPCODE_TOURNAMENT_UEW,
    OPCODE_UPDATE_CHARACTER_BOX, OPCODE_UPDATE_CHARACTER_LOOK, OPCODE_UPDATE_PARTY_MEMBER_HP,
    OPCODE_WHISPER_RECEIVED, OPCODE_WHISPER_SEND, PartyEvent, PartyMember, PartyMemberHp,
    PartySnapshot, PetChatUpdate, PetCommandUpdate, PetExceptionList, PetNameChange, PetSpawnEvent,
    PlayerAttackPacket, PlayerMovement, RemoteAttackDamage, RemoteAttackTarget, RemoteAttackType,
    RemoteCharacterAttack, RemoteCharacterChalkboardUpdate, RemoteCharacterItemEffect,
    RemoteCharacterLookUpdate, RemoteCharacterMiniRoomUpdate, RemoteCharacterMovement,
    RemoteCharacterSpawn, RemoteFacialExpression, RemoteGuildMarkUpdate, RemoteGuildNameUpdate,
    RemotePlayerDamage, ScrollResult, ServerMessage, ServerMessageStyle, SkillLearnItemResult,
    SummonSkillUpdate, TamingMobInfo, WhisperMessage, accept_buddy_packet, add_buddy_packet,
    build_player_attack_packet, change_channel_packet, change_map_packet,
    change_party_leader_packet, character_info_request_packet, create_party_packet,
    damage_reactor_packet, delete_buddy_packet, deny_party_invite_packet,
    expel_party_member_packet, face_expression, find_player_packet, general_chat_packet,
    give_fame_packet, heal_over_time_packet, invite_to_party_packet, join_party_packet,
    leave_party_packet, move_mob, move_mob_action, move_player, multi_chat_packet,
    npc_shop_item_action, npc_shop_leave, npc_shop_recharge, npc_talk_more, npc_talk_number,
    npc_talk_selection, npc_talk_text, parse_add_cooldown, parse_auto_hp_pot, parse_auto_mp_pot,
    parse_blow_weather, parse_buddy_list, parse_chair_cancel, parse_chair_show,
    parse_character_buff_cancel, parse_character_buff_update, parse_character_info,
    parse_character_stat_update, parse_chat_received, parse_clear_avatar_megaphone, parse_clock,
    parse_dragon_movement, parse_dragon_remove, parse_dragon_spawn, parse_drop_remove,
    parse_drop_spawn, parse_enable_tv, parse_event_notification, parse_facial_expression,
    parse_fake_gm_notice, parse_fame_response, parse_field_effect, parse_field_obstacle_list,
    parse_field_obstacle_one, parse_forced_stat_set, parse_foreign_character_buff,
    parse_foreign_character_buff_cancel, parse_foreign_mob_disease_effect,
    parse_foreign_skill_effect, parse_grenade_effect, parse_imitated_npc, parse_item_gain_in_chat,
    parse_left_knockback, parse_map_door_remove, parse_map_door_spawn, parse_map_town_portal,
    parse_mist_remove, parse_mist_spawn, parse_mob_controller, parse_mob_damage, parse_mob_hp,
    parse_mob_kill, parse_mob_movement, parse_mob_movement_response, parse_mob_spawn,
    parse_mob_status, parse_mob_status_cancel, parse_monster_carnival, parse_movements,
    parse_multi_chat, parse_npc_action, parse_npc_controller, parse_npc_dialogue, parse_npc_remove,
    parse_npc_scriptable, parse_npc_shop, parse_npc_shop_transaction_result, parse_npc_spawn,
    parse_party_member_hp, parse_party_operation, parse_pet_chat, parse_pet_command,
    parse_pet_exception_list, parse_pet_movement, parse_pet_name_change, parse_pet_spawn,
    parse_reactor_hit, parse_reactor_remove, parse_reactor_spawn, parse_remote_character_attack,
    parse_remote_character_chalkboard, parse_remote_character_item_effect,
    parse_remote_character_look, parse_remote_character_mini_room, parse_remote_character_movement,
    parse_remote_character_remove, parse_remote_character_spawn, parse_remote_guild_mark,
    parse_remote_guild_name, parse_remote_player_damage, parse_remove_tv, parse_scroll_result,
    parse_send_tv, parse_server_message, parse_set_avatar_megaphone, parse_set_back_effect,
    parse_set_traction, parse_show_combo, parse_show_foreign_effect, parse_skill_learn_item_result,
    parse_stop_clock, parse_summon_movement, parse_summon_remove, parse_summon_skill,
    parse_summon_spawn, parse_taming_mob_info, parse_whisper, pet_loot_packet, pickup_item,
    set_buddy_group_packet, spend_ability_point_packet, take_damage_packet, talk_to_npc,
    whisper_packet,
};
pub use guide::{GuideEvent, OPCODE_SPAWN_GUIDE, OPCODE_TALK_GUIDE, parse_guide_packet};
pub use guild::{
    GuildEmblem, GuildEvent, GuildInfo, GuildMember, OPCODE_DENY_GUILD_REQUEST,
    OPCODE_GUILD_OPERATION, OPCODE_GUILD_OPERATION_SEND, accept_guild_invite_packet,
    create_guild_packet, deny_guild_invite_packet, expel_guild_member_packet,
    guild_creation_confirmation_packet, guild_emblem_packet, guild_member_rank_packet,
    guild_notice_packet, guild_rank_titles_packet, invite_to_guild_packet, leave_guild_packet,
    parse_guild_operation,
};
pub use guild_bbs::{
    BbsReply, BbsThread, BbsThreadView, GuildBbsEvent, MAX_BBS_CONTENT_LENGTH,
    MAX_BBS_REPLY_LENGTH, MAX_BBS_TITLE_LENGTH, OPCODE_BBS_OPERATION, OPCODE_GUILD_BBS_PACKET,
    guild_bbs_delete_packet, guild_bbs_list_packet, guild_bbs_reply_packet, guild_bbs_view_packet,
    guild_bbs_write_packet, parse_guild_bbs_packet,
};
pub use inventory::{
    EquipInventoryData, InventoryCapacityUpdate, InventoryItem, InventoryModification,
    InventoryMovement, InventorySnapshot, InventoryType, InventoryUpdate, OPCODE_CANCEL_CHAIR,
    OPCODE_GATHER_ITEMS, OPCODE_GATHER_RESULT, OPCODE_INVENTORY_GROW, OPCODE_MODIFY_INVENTORY,
    OPCODE_MOVE_ITEM, OPCODE_PET_FOOD, OPCODE_SCROLL_EQUIP, OPCODE_SET_EXTRA_PENDANT_SLOT,
    OPCODE_SORT_ITEMS, OPCODE_SORT_RESULT, OPCODE_SPAWN_PET, OPCODE_STORAGE, OPCODE_STORAGE_ACTION,
    OPCODE_USE_CASH_ITEM, OPCODE_USE_CHAIR, OPCODE_USE_ITEM, PetInventoryData, StorageEvent,
    StorageSnapshot, cancel_chair_packet, gather_items_packet, move_item_packet,
    parse_cash_shop_withdrawal_item, parse_extra_pendant_slot, parse_inventory_capacity_update,
    parse_inventory_update, parse_storage_response, pet_food_packet, scroll_equip_packet,
    sort_items_packet, spawn_pet_packet, storage_close_packet, storage_meso_packet,
    storage_store_packet, storage_take_out_packet, use_avatar_megaphone_packet, use_chair_packet,
    use_item_packet, use_maple_tv_packet, use_megaphone_packet, use_pet_name_tag_packet,
};
pub use keymap::{
    KEYMAP_ENTRY_COUNT, KeyBinding, KeyMapUpdate, OPCODE_CHANGE_KEYMAP, OPCODE_KEYMAP,
    OPCODE_QUICKSLOT_INIT, QUICKSLOT_KEY_COUNT, QuickSlotInit, change_keymap_packet, parse_keymap,
    parse_quickslot_init,
};
pub use login::{
    AccountInfo, ChannelTransfer, CharacterInfo, CharacterList, CharacterLook, CharacterStats,
    DeleteCharacterResult, LoginResult, MapChange, OPCODE_ACCEPT_TOS, OPCODE_AFTER_LOGIN_ERROR,
    OPCODE_CHANNEL_TRANSFER, OPCODE_CHARACTER_NAME_REQUEST, OPCODE_CHARACTER_NAME_RESULT,
    OPCODE_CHARLIST_REQUEST, OPCODE_CHARLIST_RESULT, OPCODE_CHECK_PIC_RESULT,
    OPCODE_CREATE_CHARACTER, OPCODE_CREATE_CHARACTER_RESULT, OPCODE_DELETE_CHARACTER,
    OPCODE_DELETE_CHARACTER_RESULT, OPCODE_LAST_CONNECTED_WORLD, OPCODE_LOGIN, OPCODE_LOGIN_RESULT,
    OPCODE_LOGIN_START, OPCODE_PLAYER_LOGIN, OPCODE_PONG, OPCODE_RECOMMENDED_WORLDS,
    OPCODE_REGISTER_PIC, OPCODE_SELECT_CHARACTER, OPCODE_SELECT_CHARACTER_PIC, OPCODE_SERVER_IP,
    OPCODE_SERVERLIST_REQUEST, OPCODE_SERVERLIST_RESULT, OPCODE_SERVERSTATUS_REQUEST,
    OPCODE_SERVERSTATUS_RESULT, OPCODE_SET_FIELD, PicResult, RecommendedWorld, ServerListBatch,
    ServerRedirect, ServerStatus, SetField, WorldChannel, WorldInfo, accept_terms_of_service,
    create_character, delete_character, login_request, login_start, parse_after_login_error,
    parse_channel_transfer, parse_character_list, parse_character_name_result,
    parse_created_character, parse_delete_character_result, parse_last_connected_world,
    parse_login_result, parse_map_change, parse_pic_result, parse_recommended_worlds,
    parse_server_list, parse_server_redirect, parse_server_status, parse_set_field, player_login,
    pong, register_character_pic, request_character_name, request_characters,
    request_server_status, request_world_list, select_character, select_character_with_pic,
};
pub use maker::{MakerResult, OPCODE_MAKER_RESULT, parse_maker_result};
pub use maple::{MAX_ENCRYPTED_PACKET_LENGTH, MapleCipher, MapleFrameDecoder};
pub use messenger::{
    MessengerEvent, OPCODE_MESSENGER, OPCODE_MESSENGER_SEND, messenger_accept_invite_packet,
    messenger_chat_packet, messenger_decline_invite_packet, messenger_invite_packet,
    messenger_leave_packet, messenger_open_packet, parse_messenger,
};
pub use monster_book::{
    MonsterBookCardUpdate, MonsterBookCoverUpdate, MonsterBookSnapshot,
    OPCODE_MONSTER_BOOK_SET_CARD, OPCODE_MONSTER_BOOK_SET_COVER, parse_monster_book_set_card,
    parse_monster_book_set_cover,
};
pub use packet::{PacketError, PacketReader, PacketWriter};
pub use party_search::{
    ALL_PARTY_SEARCH_JOBS, MAX_PARTY_MEMBERS, MAX_PARTY_SEARCH_LEVEL_SPAN,
    OPCODE_PARTY_SEARCH_REGISTER, OPCODE_PARTY_SEARCH_START, PartySearchCriteria,
    party_search_register_packet, party_search_start_packet,
};
pub use quest::{
    CompletedQuest, OPCODE_QUEST_ACTION, OPCODE_QUEST_CLEAR, OPCODE_SHOW_STATUS_INFO,
    OPCODE_UPDATE_QUEST_INFO, QuestActionFailure, QuestEntry, QuestLog, QuestTimer,
    QuestTimerUpdate, QuestUpdate, SkillCooldown, complete_quest_packet, forfeit_quest_packet,
    parse_expired_quest, parse_quest_action_failure, parse_quest_clear, parse_quest_status_update,
    parse_quest_timer_update, restore_lost_quest_item_packet, start_quest_packet,
};
pub use rps::{
    OPCODE_RPS_ACTION, OPCODE_RPS_GAME, RpsGameEvent, parse_rps_game, rps_selection_packet,
};
pub use session::{SessionEvent, SessionTransport};
pub use skill_macros::{
    MAX_SKILL_MACRO_NAME_CHARS, OPCODE_SKILL_MACRO_MODIFIED, OPCODE_SKILL_MACROS,
    SKILL_MACRO_COUNT, SKILL_MACRO_SKILL_COUNT, SkillMacro, parse_skill_macros,
    save_skill_macros_packet,
};
pub use skills::{
    OPCODE_SPEND_SKILL_POINT, OPCODE_UPDATE_SKILL, OPCODE_USE_SKILL, SkillEntry, SkillUpdate,
    parse_skill_update, spend_skill_point_packet, use_skill_packet,
};
pub use social::{
    OPCODE_MARRIAGE_REQUEST, OPCODE_MARRIAGE_RESULT, OPCODE_NOTIFY_JOB_CHANGE,
    OPCODE_NOTIFY_LEVEL_UP, OPCODE_NOTIFY_MARRIAGE, OPCODE_NOTIFY_MARRIED_PARTNER,
    OPCODE_SPOUSE_CHAT, OPCODE_WEDDING_CEREMONY_END, OPCODE_WEDDING_GIFT_RESULT,
    OPCODE_WEDDING_PROGRESS, SocialNotification, parse_social_notification,
};
pub use stream::FrameDecoder;
pub use teleport_rock::{
    OPCODE_MAP_TRANSFER_RESULT, TeleportRockLocations, TeleportRockUpdate,
    parse_teleport_rock_update,
};
pub use trade::{
    HiredMerchantRemove, HiredMerchantRoom, HiredMerchantSpawn, HiredMerchantUpdate,
    MerchantChatMessage, MerchantStockItem, MerchantVisitor, OPCODE_DESTROY_HIRED_MERCHANT,
    OPCODE_ENTRUSTED_SHOP_CHECK_RESULT, OPCODE_HIRED_MERCHANT_REQUEST, OPCODE_OWL_ACTION,
    OPCODE_OWL_WARP, OPCODE_PLAYER_INTERACTION, OPCODE_PLAYER_INTERACTION_RESULT,
    OPCODE_SHOP_LINK_RESULT, OPCODE_SHOP_SCANNER_RESULT, OPCODE_SPAWN_HIRED_MERCHANT,
    OPCODE_UPDATE_HIRED_MERCHANT, OwlSearchEvent, OwlShopResult, PlayerInteractionRoomReply,
    TradeItemUpdate, TradeParticipant, TradeRoom, create_hired_merchant_shop_packet,
    create_personal_shop_packet, hired_merchant_request_packet, merchant_add_item_packet,
    merchant_buy_open_packet, merchant_buy_packet, merchant_chat_packet, merchant_close_packet,
    merchant_exit_packet, merchant_maintenance_off_packet, merchant_organize_packet,
    merchant_take_item_packet, merchant_take_meso_packet, mini_room_visit_packet,
    owl_search_packet, owl_top_items_packet, owl_warp_packet, parse_hired_merchant_check_result,
    parse_hired_merchant_remove, parse_hired_merchant_room_reply, parse_hired_merchant_spawn,
    parse_hired_merchant_update, parse_owl_link_result, parse_owl_search_event,
    parse_player_interaction_room_reply, personal_shop_remove_item_packet, trade_chat_packet,
    trade_confirm_packet, trade_create_packet, trade_decline_packet, trade_exit_packet,
    trade_invite_packet, trade_set_item_packet, trade_set_meso_packet, trade_visit_packet,
};
pub use ui::{
    OPCODE_DISABLE_UI, OPCODE_LOCK_UI, OPCODE_OPEN_UI, OpenUiRequest, parse_open_ui,
    parse_ui_disabled,
};

pub const HEADER_LENGTH: usize = 4;
pub const OPCODE_LENGTH: usize = 2;
pub const MIN_PACKET_BODY_LENGTH: usize = OPCODE_LENGTH;
pub const MAX_PACKET_LENGTH: usize = 131_072;

/// Decodes the legacy unencrypted length header and rejects lengths that cannot form
/// a v83 packet or exceed the receive-buffer limit.
pub fn decode_plain_header(header: [u8; HEADER_LENGTH]) -> Result<usize, PacketError> {
    let length = u32::from_le_bytes(header) as usize;
    if !(MIN_PACKET_BODY_LENGTH..=MAX_PACKET_LENGTH).contains(&length) {
        return Err(PacketError::InvalidFrameLength(length));
    }
    Ok(length)
}

pub fn encode_plain_header(length: usize) -> Result<[u8; HEADER_LENGTH], PacketError> {
    if !(MIN_PACKET_BODY_LENGTH..=MAX_PACKET_LENGTH).contains(&length) {
        return Err(PacketError::InvalidFrameLength(length));
    }
    Ok((length as u32).to_le_bytes())
}
