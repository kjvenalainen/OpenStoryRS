use super::*;

impl ClientApp {
    pub(super) fn handle_server_packet(&mut self, packet: &[u8]) {
        if packet.len() < 2 {
            eprintln!("OpenStory: ignored packet without an opcode");
            return;
        }
        let opcode = u16::from_le_bytes([packet[0], packet[1]]);
        if matches!(
            opcode,
            openstory_protocol::OPCODE_FAMILY_CHART_RESULT
                | openstory_protocol::OPCODE_FAMILY_INFO
                | openstory_protocol::OPCODE_FAMILY_RESULT
                | openstory_protocol::OPCODE_FAMILY_JOIN_REQUEST
                | openstory_protocol::OPCODE_FAMILY_JOIN_REQUEST_RESULT
                | openstory_protocol::OPCODE_FAMILY_JOIN_ACCEPTED
                | openstory_protocol::OPCODE_FAMILY_PRIVILEGE_LIST
                | openstory_protocol::OPCODE_FAMILY_REP_GAIN
                | openstory_protocol::OPCODE_FAMILY_NOTIFY_LOGIN
                | openstory_protocol::OPCODE_FAMILY_SUMMON_REQUEST
        ) {
            match openstory_protocol::parse_family_packet(packet) {
                Ok(Some(event)) => self.apply_family_event(event),
                Ok(None) => {}
                Err(error) => eprintln!("OpenStoryRS: invalid Family packet: {error}"),
            }
            return;
        }
        match opcode {
            openstory_protocol::OPCODE_MESSENGER => {
                match openstory_protocol::parse_messenger(packet) {
                    Ok(event) => self.apply_messenger_event(event),
                    Err(error) => eprintln!("OpenStoryRS: invalid Messenger packet: {error}"),
                }
            }
            openstory_protocol::OPCODE_OPEN_UI => match openstory_protocol::parse_open_ui(packet) {
                Ok(request) => self.handle_open_ui_request(request),
                Err(error) => eprintln!("OpenStoryRS: invalid OPEN_UI packet: {error}"),
            },
            openstory_protocol::OPCODE_KEYMAP => match openstory_protocol::parse_keymap(packet) {
                Ok(keymap) => {
                    self.server_keymap = Some(keymap.bindings);
                    self.key_config_actions_dirty = true;
                    self.refresh_key_config_texture();
                    self.refresh_quickslots_texture();
                }
                Err(error) => eprintln!("OpenStoryRS: invalid KEYMAP packet: {error}"),
            },
            openstory_protocol::OPCODE_SKILL_MACROS => {
                match openstory_protocol::parse_skill_macros(packet) {
                    Ok(server_macros) => {
                        let mut macros = vec![
                            openstory_protocol::SkillMacro::default();
                            openstory_protocol::SKILL_MACRO_COUNT
                        ];
                        for (index, macro_entry) in server_macros.into_iter().enumerate() {
                            macros[index] = macro_entry;
                        }
                        self.skill_macros = macros;
                        self.key_config_actions_dirty = true;
                        self.refresh_key_config_texture();
                        self.refresh_quickslots_texture();
                    }
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid skill-macro packet: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_QUICKSLOT_INIT => {
                match openstory_protocol::parse_quickslot_init(packet) {
                    Ok(init) => {
                        if let Some(keys) = init.keys
                            && !self.quickslot_keys_customized
                        {
                            self.quickslot_keys = keys;
                            self.refresh_key_config_texture();
                        }
                        self.refresh_key_config_texture();
                        self.refresh_quickslots_texture();
                    }
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid QUICKSLOT_INIT packet: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_SEND_TV => match openstory_protocol::parse_send_tv(packet) {
                Ok(broadcast) => self.start_maple_tv_broadcast(broadcast),
                Err(error) => eprintln!("OpenStoryRS: invalid SEND_TV packet: {error}"),
            },
            openstory_protocol::OPCODE_REMOVE_TV => {
                match openstory_protocol::parse_remove_tv(packet) {
                    Ok(()) => self.stop_maple_tv_broadcast(),
                    Err(error) => eprintln!("OpenStoryRS: invalid REMOVE_TV packet: {error}"),
                }
            }
            openstory_protocol::OPCODE_ENABLE_TV => {
                if let Err(error) = openstory_protocol::parse_enable_tv(packet) {
                    eprintln!("OpenStoryRS: invalid ENABLE_TV packet: {error}");
                }
            }
            openstory_protocol::OPCODE_GATHER_RESULT | openstory_protocol::OPCODE_SORT_RESULT => {}
            openstory_protocol::OPCODE_CHAT_RECEIVED => {
                match openstory_protocol::parse_chat_received(packet) {
                    Ok(chat) => {
                        let name = self
                            .active_character
                            .as_ref()
                            .filter(|character| character.id == chat.character_id)
                            .map(|character| character.stats.name.clone())
                            .or_else(|| {
                                self.map_scene
                                    .as_ref()?
                                    .remote_players
                                    .get(&chat.character_id)
                                    .map(|character| character.name.clone())
                            })
                            .unwrap_or_else(|| format!("Character {}", chat.character_id));
                        self.show_character_speech(chat.character_id, &chat.message);
                        self.push_chat_line(
                            format!("{name}: {}", chat.message),
                            public_chat_color(chat.chat_type, chat.gm),
                        );
                    }
                    Err(error) => eprintln!("OpenStoryRS: invalid public chat packet: {error}"),
                }
            }
            openstory_protocol::OPCODE_MULTI_CHAT_RECEIVED => {
                match openstory_protocol::parse_multi_chat(packet) {
                    Ok(chat) => {
                        let (prefix, color) = match chat.chat_type {
                            0 => ("[Buddy]", [255, 180, 90]),
                            1 => ("[Party]", [255, 160, 215]),
                            2 => ("[Guild]", [135, 230, 150]),
                            3 => ("[Alliance]", [185, 240, 170]),
                            _ => ("[Chat]", [238, 232, 214]),
                        };
                        self.push_chat_line(
                            format!("{prefix} {}: {}", chat.from, chat.message),
                            color,
                        );
                    }
                    Err(error) => eprintln!("OpenStoryRS: invalid group chat packet: {error}"),
                }
            }
            openstory_protocol::OPCODE_WHISPER_RECEIVED => {
                match openstory_protocol::parse_whisper(packet) {
                    Ok(Some(openstory_protocol::WhisperMessage::Incoming {
                        from,
                        channel: _,
                        from_admin,
                        message,
                    })) => {
                        let color = if from_admin {
                            [255, 255, 255]
                        } else {
                            [255, 160, 215]
                        };
                        self.push_chat_line(format!("{from} <Whisper>: {message}"), color);
                        self.append_whisper_line(&from, format!("{from}: {message}"), color);
                    }
                    Ok(Some(openstory_protocol::WhisperMessage::FindResult { name, result })) => {
                        let line = match result {
                            -1 => format!("{name} is not online."),
                            0 => format!("{name} is in this channel but on a different map."),
                            channel => format!("{name} is on Channel {channel}."),
                        };
                        self.push_chat_line(line, [255, 222, 112]);
                    }
                    Ok(Some(openstory_protocol::WhisperMessage::DeliveryResult {
                        name,
                        result: 0,
                    })) => {
                        let notice = format!("The user {name} is not logged in.");
                        self.push_chat_line(notice.clone(), [255, 105, 105]);
                        self.append_whisper_line(
                            &name,
                            format!("[Notice] {notice}"),
                            [255, 105, 105],
                        );
                    }
                    Ok(Some(openstory_protocol::WhisperMessage::DeliveryResult { .. }))
                    | Ok(None) => {}
                    Err(error) => eprintln!("OpenStoryRS: invalid whisper packet: {error}"),
                }
            }
            openstory_protocol::OPCODE_PARTY_OPERATION => {
                match openstory_protocol::parse_party_operation(packet) {
                    Ok(Some(event)) => self.apply_party_event(event),
                    Ok(None) => {}
                    Err(error) => eprintln!("OpenStoryRS: invalid party operation: {error}"),
                }
            }
            openstory_protocol::OPCODE_BUDDY_LIST => {
                match openstory_protocol::parse_buddy_list(packet) {
                    Ok(Some(event)) => self.apply_buddy_event(event),
                    Ok(None) => {}
                    Err(error) => eprintln!("OpenStoryRS: invalid buddy-list packet: {error}"),
                }
            }
            openstory_protocol::OPCODE_GUILD_OPERATION => {
                match openstory_protocol::parse_guild_operation(packet) {
                    Ok(Some(event)) => self.apply_guild_event(event),
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid guild operation packet: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_ALLIANCE_OPERATION => {
                match openstory_protocol::parse_alliance_operation(packet) {
                    Ok(Some(event)) => self.apply_alliance_event(event),
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid alliance operation packet: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_GUILD_BBS_PACKET => {
                match openstory_protocol::parse_guild_bbs_packet(packet) {
                    Ok(event) => self.apply_guild_bbs_event(event),
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid guild BBS packet: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_UPDATE_PARTY_MEMBER_HP => {
                match openstory_protocol::parse_party_member_hp(packet) {
                    Ok(update) => {
                        if let Some(party) = self.party_state.as_mut()
                            && let Some(member) = party
                                .members
                                .iter_mut()
                                .find(|member| member.character_id == update.character_id)
                        {
                            member.hp = Some((update.hp.max(0), update.max_hp.max(0)));
                            self.refresh_party_texture();
                        }
                    }
                    Err(error) => eprintln!("OpenStoryRS: invalid party HP update: {error}"),
                }
            }
            openstory_protocol::OPCODE_SHOW_STATUS_INFO => {
                match openstory_protocol::parse_quest_status_update(packet) {
                    Ok(Some(update)) => self.apply_quest_update(update),
                    Ok(None) => {}
                    Err(error) => eprintln!("OpenStory: invalid quest status update: {error}"),
                }
            }
            openstory_protocol::OPCODE_QUEST_CLEAR => {
                match openstory_protocol::parse_quest_clear(packet) {
                    Ok(quest_id) => {
                        if let Some(name) = quest_info_text(&self.quest_archive, quest_id, "name") {
                            eprintln!("OpenStoryRS: quest completed: {name}");
                        }
                        self.show_quest_clear_effect();
                        if self.show_quest_log {
                            self.refresh_quest_log_texture();
                        }
                    }
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid quest completion notification: {error}");
                    }
                }
            }
            openstory_protocol::OPCODE_UPDATE_QUEST_INFO => {
                match openstory_protocol::parse_quest_timer_update(packet) {
                    Ok(Some(update)) => self.handle_quest_timer_update(update),
                    Ok(None) => match openstory_protocol::parse_quest_action_failure(packet) {
                        Ok(Some(failure)) => {
                            let message = match failure {
                                openstory_protocol::QuestActionFailure::RequirementsNotMet {
                                    quest_id,
                                } => quest_info_text(&self.quest_archive, quest_id, "name")
                                    .map_or_else(
                                        || {
                                            "[Quest] You don't meet the requirements for this quest."
                                                .to_owned()
                                        },
                                        |name| {
                                            format!(
                                                "[Quest] You don't meet the requirements for '{name}'."
                                            )
                                        },
                                    ),
                                openstory_protocol::QuestActionFailure::NotEnoughMesos => {
                                    "[Quest] You don't have enough mesos.".to_owned()
                                }
                                openstory_protocol::QuestActionFailure::ItemEquipped => {
                                    "[Quest] Item is currently worn by character.".to_owned()
                                }
                                openstory_protocol::QuestActionFailure::MissingRequiredItem => {
                                    "[Quest] You don't have the required item.".to_owned()
                                }
                            };
                            self.push_chat_line(message, [255, 105, 105]);
                        }
                        Ok(None) => match openstory_protocol::parse_expired_quest(packet) {
                            Ok(Some(id)) => {
                                self.apply_quest_update(
                                    openstory_protocol::QuestUpdate::Forfeited { id },
                                );
                            }
                            Ok(None) => {}
                            Err(error) => {
                                eprintln!("OpenStory: invalid quest info update: {error}")
                            }
                        },
                        Err(error) => {
                            eprintln!("OpenStory: invalid quest info update: {error}")
                        }
                    },
                    Err(error) => eprintln!("OpenStory: invalid quest info update: {error}"),
                }
            }
            openstory_protocol::OPCODE_SET_CASH_SHOP => self.handle_cash_shop_packet(packet),
            openstory_protocol::OPCODE_CASH_SHOP_OPERATION => {
                self.handle_cash_shop_operation(packet)
            }
            openstory_protocol::OPCODE_QUERY_CASH_RESULT => self.handle_cash_balances(packet),
            openstory_protocol::OPCODE_SHOP_SCANNER_RESULT => self.handle_owl_search_packet(packet),
            openstory_protocol::OPCODE_SHOP_LINK_RESULT => self.handle_owl_link_packet(packet),
            openstory_protocol::OPCODE_ENTRUSTED_SHOP_CHECK_RESULT => {
                self.handle_hired_merchant_check_packet(packet)
            }
            openstory_protocol::OPCODE_PLAYER_INTERACTION_RESULT => {
                self.handle_hired_merchant_room_packet(packet)
            }
            openstory_protocol::OPCODE_DELETE_CHARACTER_RESULT => {
                match openstory_protocol::parse_delete_character_result(packet) {
                    Ok(result) if result.state == 0 => {
                        if let Some(characters) = self.character_list.as_mut() {
                            characters
                                .characters
                                .retain(|character| character.id != result.character_id);
                        }
                        self.refresh_character_scene();
                    }
                    Ok(result) if result.state == 20 => {
                        self.begin_pic_entry_for(
                            result.character_id,
                            PicEntryStep::Verify,
                            PicPurpose::DeleteCharacter,
                        );
                        if let Some(entry) = self.pic_entry.as_mut() {
                            entry.notice = Some("Incorrect PIC; try again".to_owned());
                        }
                        self.refresh_pic_dialog_texture();
                    }
                    Ok(result) => {
                        let message = match result.state {
                            6 => "Trouble logging into the game; try again later.".to_owned(),
                            9 => "The server could not delete this character.".to_owned(),
                            10 => {
                                "Too many requests; wait a moment before trying again.".to_owned()
                            }
                            18 => "Birthday verification failed.".to_owned(),
                            22 => "A guild master cannot be deleted.".to_owned(),
                            24 => {
                                "A character with a pending wedding cannot be deleted.".to_owned()
                            }
                            26 => "A character with a pending world transfer cannot be deleted."
                                .to_owned(),
                            29 => "A character who still belongs to a family cannot be deleted."
                                .to_owned(),
                            state => format!("Deletion failed (server state {state})."),
                        };
                        self.show_delete_notice(result.character_id, message);
                    }
                    Err(error) => {
                        eprintln!("OpenStory: invalid character deletion response: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_CHECK_PIC_RESULT => {
                match openstory_protocol::parse_pic_result(packet) {
                    Ok(result) if result.reason == 0 => {
                        if let Some(character_id) =
                            self.active_character.as_ref().map(|character| character.id)
                        {
                            self.begin_pic_entry(character_id, PicEntryStep::Verify);
                            if let Some(entry) = self.pic_entry.as_mut() {
                                entry.notice = Some("Incorrect PIC; try again".to_owned());
                            }
                            self.refresh_pic_dialog_texture();
                        } else {
                            eprintln!("OpenStory: server rejected PIC without an active character");
                        }
                    }
                    Ok(result) => eprintln!("OpenStory: PIC response reason {}", result.reason),
                    Err(error) => eprintln!("OpenStory: invalid PIC response: {error}"),
                }
            }
            openstory_protocol::OPCODE_LOGIN_RESULT => {
                match openstory_protocol::parse_login_result(packet) {
                    Ok(openstory_protocol::LoginResult::Accepted(account)) => {
                        eprintln!("OpenStory: account login accepted");
                        if account.gender != 10 {
                            self.worlds.clear();
                            if let Some(session) = self.session.as_ref() {
                                if let Err(error) =
                                    session.send_packet(openstory_protocol::request_world_list())
                                {
                                    eprintln!("OpenStory: could not request worlds: {error}");
                                }
                            }
                        }
                    }
                    Ok(openstory_protocol::LoginResult::Rejected { reason }) => {
                        eprintln!("OpenStory: login rejected (reason {reason})");
                    }
                    Err(error) => eprintln!("OpenStory: invalid login response: {error}"),
                }
            }
            openstory_protocol::OPCODE_SERVERLIST_RESULT => {
                match openstory_protocol::parse_server_list(packet) {
                    Ok(batch) => {
                        for world in &batch.worlds {
                            eprintln!(
                                "OpenStory: world {} [{}] has {} channels",
                                world.name,
                                world.id,
                                world.channels.len()
                            );
                        }
                        let complete = batch.complete;
                        self.worlds.extend(batch.worlds);
                        if complete {
                            let worlds = std::mem::take(&mut self.worlds);
                            self.show_world_selection(worlds);
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid world list: {error}"),
                }
            }
            openstory_protocol::OPCODE_LAST_CONNECTED_WORLD => {
                match openstory_protocol::parse_last_connected_world(packet) {
                    Ok(world_id) if (0..=i32::from(u8::MAX)).contains(&world_id) => {
                        let world_id = world_id as u8;
                        self.preferred_world = Some(world_id);
                        if self
                            .worlds
                            .iter()
                            .any(|world| world.id == world_id && !world.channels.is_empty())
                        {
                            self.select_world(world_id);
                        }
                    }
                    Ok(_) => self.preferred_world = None,
                    Err(error) => {
                        eprintln!("OpenStory: invalid last-connected-world packet: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_RECOMMENDED_WORLDS => {
                if let Err(error) = openstory_protocol::parse_recommended_worlds(packet) {
                    eprintln!("OpenStory: invalid recommended-world packet: {error}");
                }
            }
            openstory_protocol::OPCODE_CHARACTER_INFO => {
                match openstory_protocol::parse_character_info(packet) {
                    Ok(info) => self.handle_character_info(info),
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid character-info response: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_FAME_RESPONSE => {
                match openstory_protocol::parse_fame_response(packet) {
                    Ok(response) => self.handle_fame_response(response),
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid fame response: {error}");
                        if let Some(ui) = self.character_info.as_mut() {
                            ui.fame_pending = false;
                        }
                        self.push_chat_line(
                            "[Fame] The server returned an invalid response.".to_owned(),
                            [255, 105, 105],
                        );
                        if self.character_info.is_some() {
                            self.refresh_character_info_texture();
                        }
                    }
                }
            }
            openstory_protocol::OPCODE_SKILL_LEARN_ITEM_RESULT => {
                match openstory_protocol::parse_skill_learn_item_result(packet) {
                    Ok(result) => self.handle_skill_learn_item_result(result),
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid skill-book result: {error}");
                        self.push_chat_line(
                            "[Skill Book] The server returned an invalid result.".to_owned(),
                            [255, 105, 105],
                        );
                    }
                }
            }
            openstory_protocol::OPCODE_MONSTER_BOOK_SET_CARD => {
                match openstory_protocol::parse_monster_book_set_card(packet) {
                    Ok(update) => {
                        self.monster_book
                            .add_card(update.card_item_id, update.card_level as i8);
                        if self.monster_book.open {
                            self.refresh_monster_book_texture();
                            if let Some(window) = self.window.as_ref() {
                                window.request_redraw();
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid Monster Book card update: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_MONSTER_BOOK_SET_COVER => {
                match openstory_protocol::parse_monster_book_set_cover(packet) {
                    Ok(update) => {
                        self.monster_book.set_cover(update.card_item_id);
                        if self.monster_book.open {
                            self.refresh_monster_book_texture();
                            if let Some(window) = self.window.as_ref() {
                                window.request_redraw();
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid Monster Book cover update: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_CHARLIST_RESULT => {
                match openstory_protocol::parse_character_list(packet) {
                    Ok(characters) => {
                        eprintln!(
                            "OpenStory: received {} character(s) for channel {} ({} slots, PIC state {})",
                            characters.characters.len(),
                            characters.channel_id,
                            characters.slots,
                            characters.pic_state
                        );
                        for character in &characters.characters {
                            eprintln!(
                                "  {} (id {}, level {}, job {}, map {})",
                                character.stats.name,
                                character.id,
                                character.stats.level,
                                character.stats.job,
                                character.stats.map_id
                            );
                        }
                        self.show_character_selection(characters);
                    }
                    Err(error) => eprintln!("OpenStory: invalid character list: {error}"),
                }
            }
            openstory_protocol::OPCODE_CHARACTER_NAME_RESULT => {
                self.handle_character_name_result(packet);
            }
            openstory_protocol::OPCODE_CREATE_CHARACTER_RESULT => {
                self.handle_created_character(packet);
            }
            openstory_protocol::OPCODE_SERVER_IP => {
                match openstory_protocol::parse_server_redirect(packet) {
                    Ok(redirect) => {
                        let address = format!("{}:{}", redirect.address, redirect.port);
                        let login_packet = openstory_protocol::player_login(redirect.character_id);
                        if let Some(session) = self.session.as_ref() {
                            if let Err(error) = session.reconnect(address, login_packet) {
                                eprintln!("OpenStory: could not reconnect to channel: {error}");
                                self.show_character_selection_failure(
                                    "Could not connect to the selected channel. Click Start to retry.",
                                );
                            }
                        } else {
                            self.show_character_selection_failure(
                                "The login connection closed before channel entry. Click Start to retry.",
                            );
                        }
                    }
                    Err(error) => {
                        eprintln!("OpenStory: invalid channel redirect: {error}");
                        self.show_character_selection_failure(
                            "The server returned an invalid channel address. Click Start to retry.",
                        );
                    }
                }
            }
            openstory_protocol::OPCODE_CHANNEL_TRANSFER => {
                match openstory_protocol::parse_channel_transfer(packet) {
                    Ok(transfer) => {
                        let Some(character_id) =
                            self.active_character.as_ref().map(|character| character.id)
                        else {
                            eprintln!(
                                "OpenStoryRS: received a channel transfer without an active character"
                            );
                            return;
                        };
                        let address = format!("{}:{}", transfer.address, transfer.port);
                        let login_packet = openstory_protocol::player_login(character_id);
                        if let Some(session) = self.session.as_ref() {
                            match session.reconnect(address, login_packet) {
                                Ok(()) => {
                                    self.channel_connected = false;
                                    eprintln!("OpenStoryRS: reconnecting to the selected channel");
                                }
                                Err(error) => {
                                    self.channel_connected = false;
                                    eprintln!(
                                        "OpenStoryRS: could not reconnect to selected channel: {error}"
                                    );
                                }
                            }
                        } else {
                            self.channel_connected = false;
                            eprintln!("OpenStoryRS: channel transfer arrived without a session");
                        }
                    }
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid channel transfer packet: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_SERVERSTATUS_RESULT => {
                match openstory_protocol::parse_server_status(packet) {
                    Ok(status) => eprintln!("OpenStory: server status {status:?}"),
                    Err(error) => eprintln!("OpenStory: invalid server status: {error}"),
                }
            }
            openstory_protocol::OPCODE_CHARACTER_STAT_UPDATE => {
                let current_job = self
                    .active_character
                    .as_ref()
                    .map_or(0, |character| character.stats.job);
                match openstory_protocol::parse_character_stat_update(packet, current_job) {
                    Ok(update) => self.apply_character_stat_update(update),
                    Err(error) => {
                        eprintln!("OpenStory: invalid character stat update: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_FORCED_STAT_SET => {
                match openstory_protocol::parse_forced_stat_set(packet) {
                    Ok(update) => self.apply_forced_stat_set(update),
                    Err(error) => eprintln!("OpenStoryRS: invalid forced-stat packet: {error}"),
                }
            }
            openstory_protocol::OPCODE_RECALCULATE_STATS => {
                self.recalculate_forced_stats();
            }
            openstory_protocol::OPCODE_GIVE_BUFF => {
                match openstory_protocol::parse_character_buff_update(packet) {
                    Ok(update) => self.apply_character_buffs(update),
                    Err(error) => {
                        eprintln!("OpenStory: invalid character buff update: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_CANCEL_BUFF => {
                match openstory_protocol::parse_character_buff_cancel(packet) {
                    Ok(update) => self.cancel_character_buffs(update),
                    Err(error) => {
                        eprintln!("OpenStory: invalid character buff cancellation: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_GIVE_FOREIGN_BUFF => {
                match openstory_protocol::parse_foreign_mob_disease_effect(packet) {
                    Ok(Some(effect)) => self.show_mob_disease_effect(
                        Some(effect.character_id),
                        effect.skill_type,
                        effect.skill_level,
                    ),
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid foreign mob disease effect: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_UPDATE_SKILL => {
                match openstory_protocol::parse_skill_update(packet) {
                    Ok(update) => {
                        for skill in update.skills {
                            self.skills.insert(skill.id, skill);
                        }
                        self.key_config_actions_dirty = true;
                        self.refresh_key_config_texture();
                        self.refresh_skill_book_entries();
                        self.refresh_skill_book_texture();
                    }
                    Err(error) => eprintln!("OpenStory: invalid skill update: {error}"),
                }
            }
            openstory_protocol::OPCODE_MODIFY_INVENTORY => {
                match openstory_protocol::parse_inventory_update(packet) {
                    Ok(update) => {
                        self.key_config_actions_dirty = true;
                        if let Some(inventory) = self.inventory.as_mut() {
                            self.inventory_drag_source = None;
                            inventory.apply(update);
                            self.resolve_pending_storage_restores();
                            self.sync_equipped_look();
                            self.refresh_inventory_texture();
                            self.refresh_storage_texture();
                            self.refresh_inventory_drop_prompt_texture();
                            self.refresh_pet_equipment_texture();
                            self.refresh_key_config_texture();
                            self.refresh_npc_shop();
                            if self.show_quest_log {
                                self.refresh_quest_log_texture();
                            }
                            self.refresh_npc_quest_marks();
                        } else {
                            eprintln!("OpenStory: received inventory update before field entry");
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid inventory update: {error}"),
                }
            }
            openstory_protocol::OPCODE_STORAGE => {
                match openstory_protocol::parse_storage_response(packet) {
                    Ok(event) => self.apply_storage_event(event),
                    Err(error) => eprintln!("OpenStoryRS: invalid storage packet: {error}"),
                }
            }
            openstory_protocol::OPCODE_SET_FIELD => {
                match openstory_protocol::parse_set_field(packet) {
                    Ok(openstory_protocol::SetField::MapChange(change)) => {
                        self.scriptable_npc_ids.clear();
                        self.storage = None;
                        self.storage_texture = None;
                        self.show_map_scene(change.map_id, change.portal)
                    }
                    Ok(openstory_protocol::SetField::CharacterEntry {
                        character_id,
                        mut stats,
                        inventory,
                        skills,
                        quest_log,
                    }) => {
                        self.scriptable_npc_ids.clear();
                        self.storage = None;
                        self.storage_texture = None;
                        self.character_selection_status = None;
                        if let Some(initial_inventory) = inventory.as_ref() {
                            stats.meso = i32::try_from(initial_inventory.meso).ok();
                        }
                        self.inventory = inventory;
                        self.key_config_actions_dirty = true;
                        self.inventory_texture = None;
                        self.skills = skills.into_iter().map(|skill| (skill.id, skill)).collect();
                        self.quest_log = quest_log.unwrap_or_default();
                        self.quest_log_tab = QuestLogTab::Active;
                        self.quest_log_selected = 0;
                        self.quest_log_scroll = 0;
                        self.quest_reward_selected = 0;
                        self.quest_forfeit_confirmation = None;
                        self.show_status = false;
                        self.status_auto_confirmation = false;
                        self.status_waiting_for_stats = false;
                        self.skill_book_entries.clear();
                        self.skill_book_texture = None;
                        self.active_character = self.character_list.as_ref().and_then(|list| {
                            list.characters
                                .iter()
                                .find(|character| character.id == character_id)
                                .cloned()
                                .map(|mut character| {
                                    character.stats = stats.clone();
                                    character
                                })
                        });
                        eprintln!(
                            "OpenStory: entered field as character {character_id} at map {} portal {}",
                            stats.map_id, stats.portal
                        );
                        self.show_map_scene(stats.map_id, stats.portal);
                        self.sync_equipped_look();
                        self.refresh_inventory_texture();
                        self.refresh_pet_equipment_texture();
                        self.refresh_skill_book_entries();
                        self.refresh_skill_book_texture();
                    }
                    Err(error) => eprintln!("OpenStory: invalid SET_FIELD packet: {error}"),
                }
            }
            openstory_protocol::OPCODE_SPAWN_CHARACTER => {
                match openstory_protocol::parse_remote_character_spawn(packet) {
                    Ok(character) => self.add_remote_character(character),
                    Err(error) => eprintln!("OpenStory: invalid character spawn: {error}"),
                }
            }
            openstory_protocol::OPCODE_REMOVE_CHARACTER => {
                match openstory_protocol::parse_remote_character_remove(packet) {
                    Ok(id) => {
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.remote_players.remove(&id);
                            scene.pets.retain(|(owner_id, _), _| *owner_id != id);
                        }
                        self.pet_speech_textures
                            .retain(|(owner_id, _), _| *owner_id != id);
                    }
                    Err(error) => eprintln!("OpenStory: invalid character removal: {error}"),
                }
            }
            openstory_protocol::OPCODE_SPAWN_PET_RECEIVED => {
                match openstory_protocol::parse_pet_spawn(packet) {
                    Ok(openstory_protocol::PetSpawnEvent::Spawn(spawn)) => {
                        self.spawn_pet(spawn);
                    }
                    Ok(openstory_protocol::PetSpawnEvent::Remove {
                        character_id, slot, ..
                    }) => self.remove_pet(character_id, slot),
                    Err(error) => eprintln!("OpenStoryRS: invalid pet spawn update: {error}"),
                }
            }
            openstory_protocol::OPCODE_MOVE_PET => {
                match openstory_protocol::parse_pet_movement(packet) {
                    Ok(movement) => self.move_pet(movement),
                    Err(error) => eprintln!("OpenStoryRS: invalid pet movement: {error}"),
                }
            }
            openstory_protocol::OPCODE_PET_CHAT_RECEIVED => {
                match openstory_protocol::parse_pet_chat(packet) {
                    Ok(update) => self.show_pet_speech(update),
                    Err(error) => eprintln!("OpenStoryRS: invalid pet chat update: {error}"),
                }
            }
            openstory_protocol::OPCODE_PET_COMMAND_RECEIVED => {
                match openstory_protocol::parse_pet_command(packet) {
                    Ok(update) => self.handle_pet_command(update),
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid pet command update: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_CHARACTER_MOVED => {
                match openstory_protocol::parse_remote_character_movement(packet) {
                    Ok(movement) => {
                        let id = movement.id;
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.apply_remote_movement(movement);
                            if std::env::var("OPENSTORY_DEBUG_REMOTE").as_deref() == Ok("1") {
                                if let Some(summary) = scene.remote_summary(id) {
                                    eprintln!("OpenStory: moved remote character {summary}");
                                }
                            }
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid character movement: {error}"),
                }
            }
            openstory_protocol::OPCODE_ATTACKED_CLOSE
            | openstory_protocol::OPCODE_ATTACKED_RANGED
            | openstory_protocol::OPCODE_ATTACKED_MAGIC => {
                match openstory_protocol::parse_remote_character_attack(packet) {
                    Ok(attack) => {
                        let local_id = self.active_character.as_ref().map(|character| character.id);
                        let (character_level, two_handed) = self
                            .map_scene
                            .as_ref()
                            .and_then(|scene| {
                                scene.remote_players.get(&attack.attacker_id).map(|player| {
                                    (
                                        player.level,
                                        character_weapon_id(&player.look)
                                            .is_some_and(is_two_handed_weapon),
                                    )
                                })
                            })
                            .unwrap_or((0, false));
                        let skill_hit_effects = if attack.skill_id > 0 {
                            match self.load_skill_hit_effects(
                                attack.skill_id,
                                attack.skill_level,
                                character_level,
                                two_handed,
                            ) {
                                Ok(animations) => animations,
                                Err(error) => {
                                    eprintln!(
                                        "OpenStory: could not load remote skill {} hit effect: {error}",
                                        attack.skill_id
                                    );
                                    Vec::new()
                                }
                            }
                        } else {
                            Vec::new()
                        };
                        if Some(attack.attacker_id) != local_id
                            && let Some(position) = self
                                .map_scene
                                .as_ref()
                                .and_then(|scene| scene.remote_players.get(&attack.attacker_id))
                                .map(|player| (rounded_i16(player.x), rounded_i16(player.y)))
                        {
                            self.audio.play_skill_use(
                                &self.sound_archive,
                                attack.skill_id,
                                position,
                            );
                        }
                        let projectile_frames = if Some(attack.attacker_id) != local_id {
                            let skill_ball = if attack.skill_id > 0 {
                                match self.load_skill_projectile_frames(
                                    attack.skill_id,
                                    attack.skill_level,
                                ) {
                                    Ok(frames) => frames,
                                    Err(error) => {
                                        eprintln!(
                                            "OpenStory: could not load remote skill {} projectile: {error}",
                                            attack.skill_id
                                        );
                                        None
                                    }
                                }
                            } else {
                                None
                            };
                            match skill_ball {
                                Some(frames) => Some(frames),
                                None if attack.bullet_id > 0 => {
                                    match self.load_projectile_frames(attack.bullet_id) {
                                        Ok(frames) => Some(frames),
                                        Err(error) => {
                                            eprintln!(
                                                "OpenStory: could not load remote projectile {}: {error}",
                                                attack.bullet_id
                                            );
                                            None
                                        }
                                    }
                                }
                                None => None,
                            }
                        } else {
                            None
                        };
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.apply_remote_attack(attack, projectile_frames, skill_hit_effects);
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid remote attack: {error}"),
                }
            }
            openstory_protocol::OPCODE_DAMAGE_PLAYER => {
                match openstory_protocol::parse_remote_player_damage(packet) {
                    Ok(damage) => {
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.show_remote_player_damage(damage);
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid remote player damage: {error}"),
                }
            }
            openstory_protocol::OPCODE_FACIAL_EXPRESSION => {
                match openstory_protocol::parse_facial_expression(packet) {
                    Ok(expression) => self.show_remote_face_expression(expression),
                    Err(error) => {
                        eprintln!("OpenStory: invalid facial expression: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_UPDATE_CHARACTER_LOOK => {
                match openstory_protocol::parse_remote_character_look(packet) {
                    Ok(update) => {
                        let id = update.id;
                        self.update_remote_character_look(update);
                        if let Some(scene) = self.map_scene.as_mut() {
                            if std::env::var("OPENSTORY_DEBUG_REMOTE").as_deref() == Ok("1") {
                                if let Some(summary) = scene.remote_summary(id) {
                                    eprintln!("OpenStory: updated remote look {summary}");
                                }
                            }
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid character look: {error}"),
                }
            }
            openstory_protocol::OPCODE_SHOW_ITEM_EFFECT => {
                match openstory_protocol::parse_remote_character_item_effect(packet) {
                    Ok(effect) => {
                        self.set_character_item_effect(effect.character_id, effect.item_id);
                    }
                    Err(error) => {
                        eprintln!("OpenStory: invalid character item effect: {error}");
                    }
                }
            }
            openstory_protocol::OPCODE_UPDATE_CHARACTER_BOX => {
                match openstory_protocol::parse_remote_character_mini_room(packet) {
                    Ok(update) => self.update_remote_mini_room(update),
                    Err(error) => {
                        eprintln!("OpenStory: invalid character mini-room update: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_SKILL_EFFECT => {
                match openstory_protocol::parse_foreign_skill_effect(packet) {
                    Ok(effect)
                        if self
                            .active_character
                            .as_ref()
                            .is_none_or(|character| character.id != effect.character_id) =>
                    {
                        self.show_remote_skill_effect(
                            effect.character_id,
                            effect.skill_id,
                            effect.level,
                            effect.speed,
                            effect.direction,
                        );
                    }
                    Ok(_) => {}
                    Err(error) => eprintln!("OpenStory: invalid foreign skill effect: {error}"),
                }
            }
            openstory_protocol::OPCODE_SHOW_FOREIGN_EFFECT => {
                match openstory_protocol::parse_show_foreign_effect(packet) {
                    Ok(Some(effect)) => {
                        if let Some(skill_id) = effect.skill_id {
                            if self
                                .active_character
                                .as_ref()
                                .is_none_or(|character| character.id != effect.character_id)
                            {
                                self.show_remote_skill_effect(
                                    effect.character_id,
                                    skill_id,
                                    effect.level,
                                    0,
                                    effect.direction,
                                );
                            }
                        } else {
                            self.show_foreign_character_effect(effect);
                        }
                    }
                    Ok(_) => {}
                    Err(error) => {
                        eprintln!("OpenStory: invalid foreign character effect: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_SHOW_ITEM_GAIN_INCHAT => {
                match openstory_protocol::parse_item_gain_in_chat(packet) {
                    Ok(Some(openstory_protocol::ItemGainInChat::Item { item_id, quantity })) => {
                        let name =
                            item_name(item_id, &self.string_archive, &mut self.item_name_cache);
                        let sign = if quantity < 0 { "-" } else { "+" };
                        self.push_chat_line(
                            format!("Gained an item: {name} ({sign}{})", quantity.unsigned_abs()),
                            [128, 190, 255],
                        );
                    }
                    Ok(Some(openstory_protocol::ItemGainInChat::MonsterBookCard)) => {
                        if let Some(character_id) =
                            self.active_character.as_ref().map(|character| character.id)
                        {
                            self.show_foreign_character_effect(
                                openstory_protocol::ForeignCharacterEffect {
                                    character_id,
                                    effect_type: 13,
                                    variant: 0,
                                    heal_amount: None,
                                    skill_id: None,
                                    level: 0,
                                    direction: -1,
                                },
                            );
                        }
                    }
                    Ok(Some(openstory_protocol::ItemGainInChat::SkillUse { skill_id })) => {
                        self.show_local_skill_buff_effect(skill_id);
                    }
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("OpenStory: invalid item-gain notification: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_SHOW_CHAIR => {
                match openstory_protocol::parse_chair_show(packet) {
                    Ok(update) => {
                        if self
                            .active_character
                            .as_ref()
                            .is_none_or(|character| character.id != update.character_id)
                            && let Some(scene) = self.map_scene.as_mut()
                        {
                            scene.set_remote_chair(update.character_id, update.item_id);
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid chair state: {error}"),
                }
            }
            openstory_protocol::OPCODE_CANCEL_CHAIR_BROADCAST => {
                match openstory_protocol::parse_chair_cancel(packet) {
                    Ok(update) => {
                        if self
                            .active_character
                            .as_ref()
                            .is_none_or(|character| character.id != update.character_id)
                            && let Some(scene) = self.map_scene.as_mut()
                        {
                            scene.set_remote_chair(update.character_id, 0);
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid chair cancel: {error}"),
                }
            }
            openstory_protocol::OPCODE_CANCEL_SKILL_EFFECT => {}
            openstory_protocol::OPCODE_NPC_DIALOGUE => {
                match openstory_protocol::parse_npc_dialogue(packet) {
                    Ok(dialogue) => self.show_npc_dialogue(dialogue),
                    Err(error) => eprintln!("OpenStory: invalid NPC dialogue: {error}"),
                }
            }
            openstory_protocol::OPCODE_OPEN_NPC_SHOP => {
                match openstory_protocol::parse_npc_shop(packet) {
                    Ok(shop) => self.show_npc_shop(shop),
                    Err(error) => eprintln!("OpenStory: invalid NPC shop: {error}"),
                }
            }
            openstory_protocol::OPCODE_CONFIRM_NPC_SHOP_TRANSACTION => {
                match openstory_protocol::parse_npc_shop_transaction_result(packet) {
                    Ok(code) => self.handle_npc_shop_transaction_result(code),
                    Err(error) => eprintln!("OpenStory: invalid NPC shop result: {error}"),
                }
            }
            openstory_protocol::OPCODE_NPC_ACTION => {
                match openstory_protocol::parse_npc_action(packet) {
                    Ok(action) => {
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.apply_npc_action(action);
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid NPC action: {error}"),
                }
            }
            openstory_protocol::OPCODE_SET_NPC_SCRIPTABLE => {
                match openstory_protocol::parse_npc_scriptable(packet) {
                    Ok(npc_ids) => self.mark_npcs_scriptable(&npc_ids),
                    Err(error) => eprintln!("OpenStory: invalid scriptable NPC list: {error}"),
                }
            }
            openstory_protocol::OPCODE_FIELD_EFFECT => {
                match openstory_protocol::parse_field_effect(packet) {
                    Ok(Some(effect)) => self.apply_field_effect(effect),
                    Ok(None) => {}
                    Err(error) => eprintln!("OpenStory: invalid field effect: {error}"),
                }
            }
            openstory_protocol::OPCODE_BLOW_WEATHER => {
                match openstory_protocol::parse_blow_weather(packet) {
                    Ok(update) => self.apply_blow_weather(update),
                    Err(error) => eprintln!("OpenStoryRS: invalid weather packet: {error}"),
                }
            }
            openstory_protocol::OPCODE_FIELD_OBSTACLE_ONE => {
                match openstory_protocol::parse_field_obstacle_one(packet) {
                    Ok(toggle) => {
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.toggle_environment(&toggle.name, toggle.mode);
                        }
                    }
                    Err(error) => {
                        eprintln!("OpenStory: invalid field obstacle toggle: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_FIELD_OBSTACLE_ONOFF => {
                match openstory_protocol::parse_field_obstacle_list(packet) {
                    Ok(toggles) => {
                        if let Some(scene) = self.map_scene.as_mut() {
                            for toggle in toggles {
                                scene.toggle_environment(&toggle.name, toggle.mode);
                            }
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid field obstacle list: {error}"),
                }
            }
            openstory_protocol::OPCODE_DROP_LOOT => {
                match openstory_protocol::parse_drop_spawn(packet) {
                    Ok(spawn) => self.spawn_drop(spawn),
                    Err(error) => eprintln!("OpenStory: invalid drop spawn: {error}"),
                }
            }
            openstory_protocol::OPCODE_REMOVE_LOOT => {
                match openstory_protocol::parse_drop_remove(packet) {
                    Ok(update) => self.remove_drop(update),
                    Err(error) => eprintln!("OpenStory: invalid drop removal: {error}"),
                }
            }
            openstory_protocol::OPCODE_SPAWN_REACTOR => {
                match openstory_protocol::parse_reactor_spawn(packet) {
                    Ok(spawn) => self.spawn_reactor(spawn),
                    Err(error) => eprintln!("OpenStory: invalid reactor spawn: {error}"),
                }
            }
            openstory_protocol::OPCODE_HIT_REACTOR => {
                match openstory_protocol::parse_reactor_hit(packet) {
                    Ok(update) => {
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.set_reactor_state(update.object_id, update.state);
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid reactor hit: {error}"),
                }
            }
            openstory_protocol::OPCODE_REMOVE_REACTOR => {
                match openstory_protocol::parse_reactor_remove(packet) {
                    Ok(update) => {
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.remove_reactor(update.object_id, update.state);
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid reactor removal: {error}"),
                }
            }
            openstory_protocol::OPCODE_SPAWN_NPC => {
                match openstory_protocol::parse_npc_spawn(packet) {
                    Ok(spawn) => self.spawn_npc(spawn),
                    Err(error) => eprintln!("OpenStory: invalid NPC spawn: {error}"),
                }
            }
            openstory_protocol::OPCODE_SPAWN_HIRED_MERCHANT => {
                match openstory_protocol::parse_hired_merchant_spawn(packet) {
                    Ok(spawn) => self.spawn_hired_merchant(spawn),
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid hired merchant spawn: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_DESTROY_HIRED_MERCHANT => {
                match openstory_protocol::parse_hired_merchant_remove(packet) {
                    Ok(update) => self.remove_hired_merchant(update.owner_id),
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid hired merchant removal: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_UPDATE_HIRED_MERCHANT => {
                match openstory_protocol::parse_hired_merchant_update(packet) {
                    Ok(update) => self.update_hired_merchant(update),
                    Err(error) => {
                        eprintln!("OpenStoryRS: invalid hired merchant update: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_REMOVE_NPC => {
                match openstory_protocol::parse_npc_remove(packet) {
                    Ok(object_id) => self.remove_npc(object_id),
                    Err(error) => eprintln!("OpenStory: invalid NPC removal: {error}"),
                }
            }
            openstory_protocol::OPCODE_SPAWN_NPC_CONTROLLER => {
                match openstory_protocol::parse_npc_controller(packet) {
                    Ok(openstory_protocol::NpcControllerUpdate::Spawn(spawn)) => {
                        self.spawn_npc(spawn)
                    }
                    Ok(openstory_protocol::NpcControllerUpdate::Remove { object_id }) => {
                        self.remove_npc(object_id)
                    }
                    Err(error) => {
                        eprintln!("OpenStory: invalid NPC controller update: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_SPAWN_MIST => {
                match openstory_protocol::parse_mist_spawn(packet) {
                    Ok(mist) => {
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.spawn_mist(mist);
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid mist spawn: {error}"),
                }
            }
            openstory_protocol::OPCODE_REMOVE_MIST => {
                match openstory_protocol::parse_mist_remove(packet) {
                    Ok(object_id) => {
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.remove_mist(object_id);
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid mist removal: {error}"),
                }
            }
            openstory_protocol::OPCODE_SPAWN_MOB => {
                match openstory_protocol::parse_mob_spawn(packet) {
                    Ok(spawn) => self.spawn_mob(spawn),
                    Err(error) => eprintln!("OpenStory: invalid mob spawn: {error}"),
                }
            }
            openstory_protocol::OPCODE_KILL_MOB => {
                match openstory_protocol::parse_mob_kill(packet) {
                    Ok((object_id, animation)) => {
                        let mob_sound = self.map_scene.as_ref().and_then(|scene| {
                            scene.mobs.get(&object_id).and_then(|mob| {
                                (animation == 1 && !mob.dying).then_some((
                                    mob.mob_id,
                                    rounded_i16(mob.x),
                                    rounded_i16(mob.y),
                                ))
                            })
                        });
                        if let Some((mob_id, x, y)) = mob_sound {
                            let path = format!("Mob.img/{mob_id:07}.img/Die");
                            self.audio
                                .play_world_sound(&self.sound_archive, &path, (x, y));
                        }
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.kill_mob(object_id, animation);
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid mob kill: {error}"),
                }
            }
            openstory_protocol::OPCODE_SPAWN_MOB_CONTROLLER => {
                match openstory_protocol::parse_mob_controller(packet) {
                    Ok(update) => self.handle_mob_controller(update),
                    Err(error) => {
                        eprintln!("OpenStory: invalid mob controller update: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_MOB_MOVED => {
                match openstory_protocol::parse_mob_movement(packet) {
                    Ok(movement) => {
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.apply_mob_movement(movement);
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid mob movement: {error}"),
                }
            }
            openstory_protocol::OPCODE_MOVE_MOB_RESPONSE => {
                match openstory_protocol::parse_mob_movement_response(packet) {
                    Ok(response) => {
                        let skill_packet = self
                            .map_scene
                            .as_mut()
                            .and_then(|scene| scene.apply_mob_movement_response(response));
                        if let Some(packet) = skill_packet
                            && self.channel_connected
                            && let Some(session) = self.session.as_ref()
                            && let Err(error) = session.send_packet(packet)
                        {
                            eprintln!("OpenStory: could not send mob skill action: {error}");
                            self.channel_connected = false;
                        }
                    }
                    Err(error) => {
                        eprintln!("OpenStory: invalid mob movement response: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_SHOW_MOB_HP => {
                match openstory_protocol::parse_mob_hp(packet) {
                    Ok(update) => {
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.show_mob_hp(update);
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid mob HP update: {error}"),
                }
            }
            openstory_protocol::OPCODE_DAMAGE_MOB => {
                match openstory_protocol::parse_mob_damage(packet) {
                    Ok(damage) => {
                        let mob_sound = self.map_scene.as_ref().and_then(|scene| {
                            scene
                                .mobs
                                .get(&damage.object_id)
                                .map(|mob| (mob.mob_id, rounded_i16(mob.x), rounded_i16(mob.y)))
                        });
                        if let Some((mob_id, x, y)) = mob_sound {
                            let path = format!("Mob.img/{mob_id:07}.img/Damage");
                            self.audio
                                .play_world_sound(&self.sound_archive, &path, (x, y));
                        }
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.show_mob_damage(damage);
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid mob damage packet: {error}"),
                }
            }
            openstory_protocol::OPCODE_APPLY_MOB_STATUS => {
                match openstory_protocol::parse_mob_status(packet) {
                    Ok(update) => {
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.apply_mob_status(update);
                        }
                    }
                    Err(error) => eprintln!("OpenStory: invalid mob status update: {error}"),
                }
            }
            openstory_protocol::OPCODE_CANCEL_MOB_STATUS => {
                match openstory_protocol::parse_mob_status_cancel(packet) {
                    Ok(update) => {
                        if let Some(scene) = self.map_scene.as_mut() {
                            scene.cancel_mob_status(update);
                        }
                    }
                    Err(error) => {
                        eprintln!("OpenStory: invalid mob status cancellation: {error}")
                    }
                }
            }
            openstory_protocol::OPCODE_SPAWN_SUMMON => {
                match openstory_protocol::parse_summon_spawn(packet) {
                    Ok(spawn) => self.spawn_summon(spawn),
                    Err(error) => eprintln!("OpenStory: invalid summon spawn: {error}"),
                }
            }
            openstory_protocol::OPCODE_REMOVE_SUMMON => {
                match openstory_protocol::parse_summon_remove(packet) {
                    Ok(update) => self.remove_summon(update),
                    Err(error) => eprintln!("OpenStory: invalid summon removal: {error}"),
                }
            }
            openstory_protocol::OPCODE_MOVE_SUMMON => {
                match openstory_protocol::parse_summon_movement(packet) {
                    Ok(movement) => self.move_summon(movement),
                    Err(error) => eprintln!("OpenStory: invalid summon movement: {error}"),
                }
            }
            openstory_protocol::OPCODE_SUMMON_SKILL => {
                match openstory_protocol::parse_summon_skill(packet) {
                    Ok(update) => self.update_summon_skill(update),
                    Err(error) => eprintln!("OpenStory: invalid summon skill update: {error}"),
                }
            }
            openstory_protocol::OPCODE_SUMMON_ATTACK | openstory_protocol::OPCODE_DAMAGE_SUMMON => {
            }
            17 => {
                if let Some(session) = self.session.as_ref() {
                    if let Err(error) = session.send_packet(openstory_protocol::pong()) {
                        eprintln!("OpenStory: could not answer ping: {error}");
                    }
                }
            }
            _ => eprintln!(
                "OpenStory: received packet opcode {opcode} ({} bytes)",
                packet.len()
            ),
        }
    }
}
