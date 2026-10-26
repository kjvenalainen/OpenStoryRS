#![forbid(unsafe_code)]

//! Safe, bounded MapleStory packet primitives and v83 framing/cipher support.

mod alliance;
mod cash_shop;
mod gameplay;
mod guild;
mod guild_bbs;
mod inventory;
mod login;
mod maple;
mod packet;
mod quest;
mod session;
mod skills;
mod stream;
mod trade;

pub use alliance::{
    AllianceEvent, AllianceGuild, AllianceInfo, OPCODE_ALLIANCE_OPERATION,
    OPCODE_ALLIANCE_OPERATION_SEND, OPCODE_DENY_ALLIANCE_REQUEST, alliance_accept_invite_packet,
    alliance_expel_guild_packet, alliance_info_request_packet, alliance_invite_packet,
    alliance_leave_packet, alliance_notice_packet, alliance_rank_titles_packet,
    deny_alliance_invite_packet, parse_alliance_operation,
};
pub use cash_shop::{
    CashShopItem, CashShopOperation, OPCODE_BUY_CASH_ITEM, OPCODE_CASH_SHOP_OPERATION,
    OPCODE_ENTER_CASH_SHOP, OPCODE_QUERY_CASH_RESULT, OPCODE_SET_CASH_SHOP, buy_cash_item_packet,
    enter_cash_shop_packet, exit_cash_shop_packet, parse_cash_balances, parse_cash_shop_open,
    parse_cash_shop_operation, withdraw_cash_item_packet,
};
pub use gameplay::{
    AbilityStat, AttackDamageTarget, AttackType, BuddyEntry, BuddyEvent, BuffMaskGroup,
    BuffStatKey, CharacterBuffCancel, CharacterBuffEntry, CharacterBuffUpdate, CharacterInfoPet,
    CharacterInfoReply, CharacterMedalInfo, CharacterMiniRoom, CharacterMonsterBookInfo,
    CharacterMountInfo, CharacterStatChange, CharacterStatUpdate, ChatMessage, FieldObstacleToggle,
    ForeignCharacterEffect, ForeignSkillEffect, MapChairCancel, MapChairShow, MapDropRemove,
    MapDropSpawn, MapMistSpawn, MapMobMovement, MapMobSpawn, MapNpcSpawn, MapPetMovement,
    MapPetSpawn, MapReactorSpawn, MapReactorState, MapSummonMovement, MapSummonRemove,
    MapSummonSpawn, MobControllerUpdate, MobDamage, MobHpUpdate, MobMovementResponse,
    MobStatusCancel, MobStatusEntry, MobStatusUpdate, MovementFragment, MovementKind,
    MultiChatMessage, MultiChatType, NpcAction, NpcControllerUpdate, NpcDialogue, NpcShop,
    NpcShopItem, OPCODE_APPLY_MOB_STATUS, OPCODE_ATTACKED_CLOSE, OPCODE_ATTACKED_MAGIC,
    OPCODE_ATTACKED_RANGED, OPCODE_BUDDY_LIST, OPCODE_BUDDY_LIST_MODIFY, OPCODE_CANCEL_BUFF,
    OPCODE_CANCEL_CHAIR_BROADCAST, OPCODE_CANCEL_MOB_STATUS, OPCODE_CANCEL_SKILL_EFFECT,
    OPCODE_CHANGE_MAP, OPCODE_CHARACTER_INFO, OPCODE_CHARACTER_INFO_REQUEST,
    OPCODE_CHARACTER_MOVED, OPCODE_CHARACTER_STAT_UPDATE, OPCODE_CHAT_RECEIVED,
    OPCODE_CLOSE_ATTACK, OPCODE_CONFIRM_NPC_SHOP_TRANSACTION, OPCODE_DAMAGE_MOB,
    OPCODE_DAMAGE_PLAYER, OPCODE_DAMAGE_REACTOR, OPCODE_DAMAGE_SUMMON, OPCODE_DENY_PARTY_INVITE,
    OPCODE_DROP_LOOT, OPCODE_FACE_EXPRESSION, OPCODE_FACIAL_EXPRESSION, OPCODE_FIELD_OBSTACLE_ONE,
    OPCODE_FIELD_OBSTACLE_ONOFF, OPCODE_GENERAL_CHAT, OPCODE_GIVE_BUFF, OPCODE_HEAL_OVER_TIME,
    OPCODE_HIT_REACTOR, OPCODE_KILL_MOB, OPCODE_MAGIC_ATTACK, OPCODE_MOB_MOVED, OPCODE_MOVE_MOB,
    OPCODE_MOVE_MOB_RESPONSE, OPCODE_MOVE_PET, OPCODE_MOVE_PLAYER, OPCODE_MOVE_SUMMON,
    OPCODE_MULTI_CHAT_RECEIVED, OPCODE_MULTI_CHAT_SEND, OPCODE_NPC_ACTION, OPCODE_NPC_DIALOGUE,
    OPCODE_NPC_SHOP_ACTION, OPCODE_NPC_TALK_MORE, OPCODE_OPEN_NPC_SHOP, OPCODE_PARTY_OPERATION,
    OPCODE_PARTY_OPERATION_SEND, OPCODE_PET_CHAT_RECEIVED, OPCODE_PET_COMMAND_RECEIVED,
    OPCODE_PET_LOOT, OPCODE_PICKUP_ITEM, OPCODE_RANGED_ATTACK, OPCODE_REMOVE_CHARACTER,
    OPCODE_REMOVE_LOOT, OPCODE_REMOVE_MIST, OPCODE_REMOVE_NPC, OPCODE_REMOVE_REACTOR,
    OPCODE_REMOVE_SUMMON, OPCODE_SET_NPC_SCRIPTABLE, OPCODE_SHOW_CHAIR, OPCODE_SHOW_FOREIGN_EFFECT,
    OPCODE_SHOW_MOB_HP, OPCODE_SKILL_EFFECT, OPCODE_SPAWN_CHARACTER, OPCODE_SPAWN_MIST,
    OPCODE_SPAWN_MOB, OPCODE_SPAWN_MOB_CONTROLLER, OPCODE_SPAWN_NPC, OPCODE_SPAWN_NPC_CONTROLLER,
    OPCODE_SPAWN_PET_RECEIVED, OPCODE_SPAWN_REACTOR, OPCODE_SPAWN_SUMMON,
    OPCODE_SPEND_ABILITY_POINT, OPCODE_SUMMON_ATTACK, OPCODE_SUMMON_SKILL, OPCODE_TAKE_DAMAGE,
    OPCODE_TALK_TO_NPC, OPCODE_UPDATE_CHARACTER_BOX, OPCODE_UPDATE_CHARACTER_LOOK,
    OPCODE_UPDATE_PARTY_MEMBER_HP, OPCODE_WHISPER_RECEIVED, OPCODE_WHISPER_SEND, PartyEvent,
    PartyMember, PartyMemberHp, PartySnapshot, PetChatUpdate, PetCommandUpdate, PetSpawnEvent,
    PlayerAttackPacket, PlayerMovement, RemoteAttackDamage, RemoteAttackTarget, RemoteAttackType,
    RemoteCharacterAttack, RemoteCharacterLookUpdate, RemoteCharacterMiniRoomUpdate,
    RemoteCharacterMovement, RemoteCharacterSpawn, RemoteFacialExpression, RemotePlayerDamage,
    SummonSkillUpdate, WhisperMessage, accept_buddy_packet, add_buddy_packet,
    build_player_attack_packet, change_map_packet, change_party_leader_packet,
    character_info_request_packet, create_party_packet, damage_reactor_packet, delete_buddy_packet,
    deny_party_invite_packet, expel_party_member_packet, face_expression, find_player_packet,
    general_chat_packet, heal_over_time_packet, invite_to_party_packet, join_party_packet,
    leave_party_packet, move_mob, move_mob_action, move_player, multi_chat_packet,
    npc_shop_item_action, npc_shop_leave, npc_shop_recharge, npc_talk_more, npc_talk_number,
    npc_talk_selection, npc_talk_text, parse_buddy_list, parse_chair_cancel, parse_chair_show,
    parse_character_buff_cancel, parse_character_buff_update, parse_character_info,
    parse_character_stat_update, parse_chat_received, parse_drop_remove, parse_drop_spawn,
    parse_facial_expression, parse_field_obstacle_list, parse_field_obstacle_one,
    parse_foreign_skill_effect, parse_mist_remove, parse_mist_spawn, parse_mob_controller,
    parse_mob_damage, parse_mob_hp, parse_mob_kill, parse_mob_movement,
    parse_mob_movement_response, parse_mob_spawn, parse_mob_status, parse_mob_status_cancel,
    parse_movements, parse_multi_chat, parse_npc_action, parse_npc_controller, parse_npc_dialogue,
    parse_npc_remove, parse_npc_scriptable, parse_npc_shop, parse_npc_shop_transaction_result,
    parse_npc_spawn, parse_party_member_hp, parse_party_operation, parse_pet_chat,
    parse_pet_command, parse_pet_movement, parse_pet_spawn, parse_reactor_hit,
    parse_reactor_remove, parse_reactor_spawn, parse_remote_character_attack,
    parse_remote_character_look, parse_remote_character_mini_room, parse_remote_character_movement,
    parse_remote_character_remove, parse_remote_character_spawn, parse_remote_player_damage,
    parse_show_foreign_effect, parse_summon_movement, parse_summon_remove, parse_summon_skill,
    parse_summon_spawn, parse_whisper, pet_loot_packet, pickup_item, set_buddy_group_packet,
    spend_ability_point_packet, take_damage_packet, talk_to_npc, whisper_packet,
};
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
    EquipInventoryData, InventoryItem, InventoryModification, InventoryMovement, InventorySnapshot,
    InventoryType, InventoryUpdate, OPCODE_CANCEL_CHAIR, OPCODE_GATHER_ITEMS, OPCODE_GATHER_RESULT,
    OPCODE_MODIFY_INVENTORY, OPCODE_MOVE_ITEM, OPCODE_PET_FOOD, OPCODE_SCROLL_EQUIP,
    OPCODE_SORT_ITEMS, OPCODE_SORT_RESULT, OPCODE_SPAWN_PET, OPCODE_STORAGE, OPCODE_STORAGE_ACTION,
    OPCODE_USE_CASH_ITEM, OPCODE_USE_CHAIR, OPCODE_USE_ITEM, PetInventoryData, StorageEvent,
    StorageSnapshot, cancel_chair_packet, gather_items_packet, move_item_packet,
    parse_cash_shop_withdrawal_item, parse_inventory_update, parse_storage_response,
    pet_food_packet, scroll_equip_packet, sort_items_packet, spawn_pet_packet,
    storage_close_packet, storage_meso_packet, storage_store_packet, storage_take_out_packet,
    use_avatar_megaphone_packet, use_chair_packet, use_item_packet, use_maple_tv_packet,
    use_megaphone_packet, use_pet_name_tag_packet,
};
pub use login::{
    AccountInfo, CharacterInfo, CharacterList, CharacterLook, CharacterStats,
    DeleteCharacterResult, LoginResult, MapChange, OPCODE_CHARACTER_NAME_REQUEST,
    OPCODE_CHARACTER_NAME_RESULT, OPCODE_CHARLIST_REQUEST, OPCODE_CHARLIST_RESULT,
    OPCODE_CHECK_PIC_RESULT, OPCODE_CREATE_CHARACTER, OPCODE_CREATE_CHARACTER_RESULT,
    OPCODE_DELETE_CHARACTER, OPCODE_DELETE_CHARACTER_RESULT, OPCODE_LAST_CONNECTED_WORLD,
    OPCODE_LOGIN, OPCODE_LOGIN_RESULT, OPCODE_LOGIN_START, OPCODE_PLAYER_LOGIN, OPCODE_PONG,
    OPCODE_RECOMMENDED_WORLDS, OPCODE_REGISTER_PIC, OPCODE_SELECT_CHARACTER,
    OPCODE_SELECT_CHARACTER_PIC, OPCODE_SERVER_IP, OPCODE_SERVERLIST_REQUEST,
    OPCODE_SERVERLIST_RESULT, OPCODE_SERVERSTATUS_REQUEST, OPCODE_SERVERSTATUS_RESULT,
    OPCODE_SET_FIELD, PicResult, RecommendedWorld, ServerListBatch, ServerRedirect, ServerStatus,
    SetField, WorldChannel, WorldInfo, create_character, delete_character, login_request,
    login_start, parse_character_list, parse_character_name_result, parse_created_character,
    parse_delete_character_result, parse_last_connected_world, parse_login_result,
    parse_map_change, parse_pic_result, parse_recommended_worlds, parse_server_list,
    parse_server_redirect, parse_server_status, parse_set_field, player_login, pong,
    register_character_pic, request_character_name, request_characters, request_server_status,
    request_world_list, select_character, select_character_with_pic,
};
pub use maple::{MAX_ENCRYPTED_PACKET_LENGTH, MapleCipher, MapleFrameDecoder};
pub use packet::{PacketError, PacketReader, PacketWriter};
pub use quest::{
    CompletedQuest, OPCODE_QUEST_ACTION, OPCODE_QUEST_CLEAR, OPCODE_SHOW_STATUS_INFO,
    OPCODE_UPDATE_QUEST_INFO, QuestEntry, QuestLog, QuestUpdate, complete_quest_packet,
    forfeit_quest_packet, parse_expired_quest, parse_quest_clear, parse_quest_status_update,
    restore_lost_quest_item_packet, start_quest_packet,
};
pub use session::{SessionEvent, SessionTransport};
pub use skills::{
    OPCODE_SPEND_SKILL_POINT, OPCODE_UPDATE_SKILL, OPCODE_USE_SKILL, SkillEntry, SkillUpdate,
    parse_skill_update, spend_skill_point_packet, use_skill_packet,
};
pub use stream::FrameDecoder;
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
