use super::*;

impl ClientApp {
    pub(super) fn send_trade_action(&mut self, packet: Vec<u8>, action: &str) -> bool {
        if !self.channel_connected {
            self.push_chat_line(
                "[Trade] Not connected to a game channel.".to_owned(),
                [255, 105, 105],
            );
            return false;
        }
        match self
            .session
            .as_ref()
            .map(|session| session.send_packet(packet))
        {
            Some(Ok(())) => true,
            Some(Err(error)) => {
                self.channel_connected = false;
                self.push_chat_line(
                    format!("[Trade] Could not {action}: {error}"),
                    [255, 105, 105],
                );
                false
            }
            None => false,
        }
    }

    pub(super) fn execute_trade_command(&mut self, command: TradeCommand) {
        match command {
            TradeCommand::Invite(name) => {
                if self.trade_room.is_some() || self.trade_request_pending {
                    self.push_chat_line(
                        "[Trade] Finish the current trade request or room first.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                }
                let target = self.map_scene.as_ref().and_then(|scene| {
                    scene
                        .remote_players
                        .iter()
                        .find(|(_, player)| player.name.eq_ignore_ascii_case(&name))
                        .map(|(&id, player)| (id, player.name.clone()))
                });
                let Some((character_id, target_name)) = target else {
                    self.push_chat_line(
                        format!("[Trade] {name} is not visible in this map."),
                        [255, 222, 112],
                    );
                    return;
                };
                let Some(invite_packet) = openstory_protocol::trade_invite_packet(character_id)
                else {
                    return;
                };
                if self.send_trade_action(
                    openstory_protocol::trade_create_packet(),
                    "create a trade room",
                ) {
                    self.trade_request_pending = true;
                    if self.send_trade_action(invite_packet, "invite the player") {
                        self.push_chat_line(
                            format!("[Trade] Invited {target_name} to trade."),
                            [230, 210, 160],
                        );
                    } else {
                        self.send_trade_action(
                            openstory_protocol::trade_exit_packet(),
                            "close the unused trade room",
                        );
                        self.trade_request_pending = false;
                    }
                }
            }
            TradeCommand::Accept => {
                let Some(invite) = self.pending_trade_invite.as_ref() else {
                    self.push_chat_line(
                        "[Trade] There is no pending trade invitation.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                };
                let Some(packet) = openstory_protocol::trade_visit_packet(invite.room_id) else {
                    self.push_chat_line(
                        "[Trade] The invitation did not include a valid room.".to_owned(),
                        [255, 105, 105],
                    );
                    return;
                };
                let inviter = invite.inviter.clone();
                if self.send_trade_action(packet, "accept the invitation") {
                    self.push_chat_line(
                        format!("[Trade] Joining {inviter}'s trade room."),
                        [230, 210, 160],
                    );
                }
            }
            TradeCommand::Decline => {
                if self.pending_trade_invite.is_some() {
                    if self.send_trade_action(
                        openstory_protocol::trade_decline_packet(),
                        "decline the invitation",
                    ) {
                        self.pending_trade_invite = None;
                    }
                } else {
                    self.push_chat_line(
                        "[Trade] There is no pending trade invitation.".to_owned(),
                        [255, 222, 112],
                    );
                }
            }
        }
    }

    pub(super) fn send_party_packet(&mut self, packet: Vec<u8>, success: &str) -> bool {
        if !self.channel_connected {
            self.push_chat_line(
                "[Party] Not connected to a game channel.".to_owned(),
                [255, 222, 112],
            );
            return false;
        }
        let result = self
            .session
            .as_ref()
            .map(|session| session.send_packet(packet));
        match result {
            Some(Ok(())) => {
                self.push_chat_line(format!("[Party] {success}"), [190, 225, 255]);
                true
            }
            Some(Err(error)) => {
                self.channel_connected = false;
                self.push_chat_line(
                    format!("[Party] Could not send request: {error}"),
                    [255, 105, 105],
                );
                false
            }
            None => {
                self.push_chat_line(
                    "[Party] Not connected to a game channel.".to_owned(),
                    [255, 222, 112],
                );
                false
            }
        }
    }

    pub(super) fn party_target_id(&self, target: &str) -> Option<i32> {
        if let Ok(character_id) = target.parse::<i32>() {
            return Some(character_id);
        }
        self.party_state
            .as_ref()?
            .members
            .iter()
            .find_map(|member| {
                member
                    .name
                    .eq_ignore_ascii_case(target)
                    .then_some(member.character_id)
            })
    }

    pub(super) fn execute_party_command(&mut self, command: PartyCommand) {
        if matches!(&command, PartyCommand::List) {
            self.show_party_panel = !self.show_party_panel;
            if self.show_party_panel {
                self.show_buddy_panel = false;
                self.show_guild_panel = false;
                self.show_alliance_panel = false;
            }
            self.refresh_party_texture();
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
            return;
        }
        let clears_pending_invite = matches!(&command, PartyCommand::Accept | PartyCommand::Deny);
        let (packet, success, invited_name) = match command {
            PartyCommand::Create => (
                Ok(openstory_protocol::create_party_packet()),
                "Party creation requested.",
                None,
            ),
            PartyCommand::Leave => (
                Ok(openstory_protocol::leave_party_packet()),
                "Leave request sent.",
                None,
            ),
            PartyCommand::Join(party_id) => (
                Ok(openstory_protocol::join_party_packet(party_id)),
                "Join request sent.",
                None,
            ),
            PartyCommand::Invite(name) => (
                openstory_protocol::invite_to_party_packet(&name),
                "Invitation sent.",
                Some(name),
            ),
            PartyCommand::Expel(target) => match self.party_target_id(&target) {
                Some(character_id) => (
                    Ok(openstory_protocol::expel_party_member_packet(character_id)),
                    "Expel request sent.",
                    None,
                ),
                None => {
                    self.push_chat_line(
                        format!("[Party] No party member named {target} was found."),
                        [255, 222, 112],
                    );
                    return;
                }
            },
            PartyCommand::Leader(target) => match self.party_target_id(&target) {
                Some(character_id) => (
                    Ok(openstory_protocol::change_party_leader_packet(character_id)),
                    "Leader change requested.",
                    None,
                ),
                None => {
                    self.push_chat_line(
                        format!("[Party] No party member named {target} was found."),
                        [255, 222, 112],
                    );
                    return;
                }
            },
            PartyCommand::Accept => {
                let Some((party_id, _)) = self.pending_party_invite.as_ref() else {
                    self.push_chat_line(
                        "[Party] There is no pending party invitation.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                };
                (
                    Ok(openstory_protocol::join_party_packet(*party_id)),
                    "Join request sent.",
                    None,
                )
            }
            PartyCommand::Deny => {
                let Some((_, inviter)) = self.pending_party_invite.as_ref() else {
                    self.push_chat_line(
                        "[Party] There is no pending party invitation.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                };
                (
                    openstory_protocol::deny_party_invite_packet(inviter),
                    "Invitation declined.",
                    None,
                )
            }
            PartyCommand::List => unreachable!(),
        };
        let packet = match packet {
            Ok(packet) => packet,
            Err(error) => {
                self.push_chat_line(
                    format!("[Party] Could not build request: {error}"),
                    [255, 105, 105],
                );
                return;
            }
        };
        if self.send_party_packet(packet, success) {
            if let Some(name) = invited_name {
                self.last_party_invite_target = name;
            }
            if clears_pending_invite {
                self.pending_party_invite = None;
                self.refresh_party_texture();
            }
        }
    }

    pub(super) fn apply_party_event(&mut self, event: openstory_protocol::PartyEvent) {
        use openstory_protocol::PartyEvent;
        match event {
            PartyEvent::Invite { party_id, from } => {
                let display_name = from.strip_prefix("PS: ").unwrap_or(&from).to_owned();
                self.pending_party_invite = Some((party_id, from));
                self.show_party_panel = true;
                self.show_buddy_panel = false;
                self.show_guild_panel = false;
                self.show_alliance_panel = false;
                self.push_chat_line(
                    format!(
                        "[Party] {display_name} invited you. Use /party accept or /party deny."
                    ),
                    [255, 222, 112],
                );
            }
            PartyEvent::Created { party_id } => {
                self.party_state = Some(openstory_protocol::PartySnapshot {
                    party_id,
                    leader_id: self
                        .active_character
                        .as_ref()
                        .map_or(0, |character| character.id),
                    members: Vec::new(),
                });
                self.show_party_panel = true;
                self.show_guild_panel = false;
                self.show_alliance_panel = false;
                self.push_chat_line("[Party] You created a party.".to_owned(), [190, 225, 255]);
            }
            PartyEvent::Snapshot(snapshot) => self.party_state = Some(snapshot),
            PartyEvent::Removed {
                party_id,
                character_id,
                disbanded,
                expelled,
                name,
                snapshot,
            } => {
                let local_id = self.active_character.as_ref().map(|character| character.id);
                if disbanded || local_id == Some(character_id) {
                    self.party_state = None;
                    self.push_chat_line(
                        if disbanded {
                            "[Party] The party was disbanded.".to_owned()
                        } else if expelled {
                            "[Party] You were expelled from the party.".to_owned()
                        } else {
                            "[Party] You left the party.".to_owned()
                        },
                        if expelled {
                            [255, 105, 105]
                        } else {
                            [190, 225, 255]
                        },
                    );
                } else if let Some(snapshot) = snapshot {
                    self.party_state = Some(snapshot);
                    self.push_chat_line(
                        format!(
                            "[Party] {} {}.",
                            name,
                            if expelled {
                                "was expelled"
                            } else {
                                "left the party"
                            }
                        ),
                        [190, 225, 255],
                    );
                } else if let Some(party) = self.party_state.as_mut() {
                    if party.party_id == party_id {
                        party
                            .members
                            .retain(|member| member.character_id != character_id);
                    }
                }
            }
            PartyEvent::MemberJoined { name, snapshot, .. } => {
                if let Some(snapshot) = snapshot {
                    self.party_state = Some(snapshot);
                }
                self.push_chat_line(format!("[Party] {name} joined the party."), [190, 225, 255]);
            }
            PartyEvent::LeaderChanged { character_id } => {
                if let Some(party) = self.party_state.as_mut() {
                    party.leader_id = character_id;
                }
                self.push_chat_line("[Party] The leader changed.".to_owned(), [190, 225, 255]);
            }
            PartyEvent::Status { code, name } => {
                let target = name.unwrap_or_else(|| self.last_party_invite_target.clone());
                let (message, color) = match code {
                    10 => (
                        "A beginner cannot create a party.".to_owned(),
                        [255, 105, 105],
                    ),
                    13 => ("You are not in a party.".to_owned(), [255, 105, 105]),
                    16 => (
                        format!(
                            "{} is already in a party.",
                            if target.is_empty() {
                                "That player"
                            } else {
                                &target
                            }
                        ),
                        [255, 105, 105],
                    ),
                    17 => ("The party is full.".to_owned(), [255, 105, 105]),
                    19 => (
                        "That player is not in this channel.".to_owned(),
                        [255, 105, 105],
                    ),
                    21 => (
                        "That player is blocking party invitations.".to_owned(),
                        [255, 105, 105],
                    ),
                    22 => (
                        format!("{} is already handling an invitation.", target),
                        [255, 105, 105],
                    ),
                    23 => (
                        format!("{} denied the invitation.", target),
                        [255, 222, 112],
                    ),
                    25 => (
                        "You cannot expel a member in this map.".to_owned(),
                        [255, 105, 105],
                    ),
                    28 | 29 => (
                        "Leader changes require a nearby party member.".to_owned(),
                        [255, 105, 105],
                    ),
                    30 => (
                        "Leader changes require the same channel.".to_owned(),
                        [255, 105, 105],
                    ),
                    _ => (
                        format!("Party request failed (code {code})."),
                        [255, 105, 105],
                    ),
                };
                self.push_chat_line(format!("[Party] {message}"), color);
            }
            PartyEvent::DoorUpdated => {}
        }
        self.refresh_party_texture();
    }

    pub(super) fn buddy_display_entries(&self) -> Vec<openstory_protocol::BuddyEntry> {
        self.buddy_entries
            .iter()
            .filter(|buddy| {
                self.buddy_filter
                    .as_ref()
                    .is_none_or(|group| buddy.group.eq_ignore_ascii_case(group))
            })
            .cloned()
            .collect()
    }

    pub(super) fn buddy_display_count(&self) -> usize {
        self.buddy_entries
            .iter()
            .filter(|buddy| {
                self.buddy_filter
                    .as_ref()
                    .is_none_or(|group| buddy.group.eq_ignore_ascii_case(group))
            })
            .count()
    }

    pub(super) fn refresh_buddy_texture(&mut self) {
        let buddies = self.buddy_display_entries();
        let pixels = render_buddy_panel(
            &self.font,
            &buddies,
            self.buddy_capacity,
            self.buddy_entries.len(),
            self.buddy_filter.as_deref(),
            self.pending_buddy_invite.as_ref(),
            self.buddy_scroll,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.buddy_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(BUDDY_PANEL_WIDTH, BUDDY_PANEL_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.buddy_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update buddy panel: {error}"),
        }
    }

    pub(super) fn buddy_target_id(&self, target: &str) -> Option<i32> {
        if let Ok(character_id) = target.parse::<i32>() {
            return Some(character_id);
        }
        self.buddy_entries
            .iter()
            .find(|buddy| buddy.name.eq_ignore_ascii_case(target))
            .map(|buddy| buddy.character_id)
    }

    pub(super) fn execute_buddy_command(&mut self, command: BuddyCommand) {
        if matches!(&command, BuddyCommand::List) {
            self.show_buddy_panel = !self.show_buddy_panel;
            if self.show_buddy_panel {
                self.show_party_panel = false;
                self.show_guild_panel = false;
                self.show_alliance_panel = false;
            }
            self.refresh_buddy_texture();
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
            return;
        }
        if let BuddyCommand::Filter(group) = command {
            if let Some(group) = group.as_ref()
                && !self
                    .buddy_entries
                    .iter()
                    .any(|buddy| buddy.group.eq_ignore_ascii_case(group))
            {
                self.push_chat_line(
                    format!("[Buddy] No buddies use the group {group}."),
                    [255, 222, 112],
                );
                return;
            }
            self.buddy_filter = group;
            self.buddy_scroll = 0;
            self.show_buddy_panel = true;
            self.show_party_panel = false;
            self.show_guild_panel = false;
            self.show_alliance_panel = false;
            self.refresh_buddy_texture();
            return;
        }
        if matches!(&command, BuddyCommand::Decline) {
            if self.pending_buddy_invite.take().is_some() {
                self.push_chat_line("[Buddy] Invitation dismissed.".to_owned(), [205, 190, 160]);
            } else {
                self.push_chat_line(
                    "[Buddy] There is no pending invitation.".to_owned(),
                    [255, 222, 112],
                );
            }
            self.refresh_buddy_texture();
            return;
        }
        let is_accept = matches!(&command, BuddyCommand::Accept);
        let packet = match command {
            BuddyCommand::Add(name) => {
                if !(4..=13).contains(&name.len()) {
                    self.push_chat_line(
                        "[Buddy] Character names must be 4–13 characters.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                }
                if self
                    .buddy_entries
                    .iter()
                    .any(|buddy| buddy.name.eq_ignore_ascii_case(&name))
                {
                    self.push_chat_line(
                        format!(
                            "[Buddy] {name} is already on your list; use /buddy group to move them."
                        ),
                        [255, 222, 112],
                    );
                    return;
                }
                match openstory_protocol::add_buddy_packet(&name) {
                    Ok(packet) => Ok((packet, format!("Buddy request sent to {name}."))),
                    Err(error) => Err(error.to_string()),
                }
            }
            BuddyCommand::ChangeGroup { target, group } => {
                let Some(name) = self
                    .buddy_entries
                    .iter()
                    .find(|buddy| buddy.name.eq_ignore_ascii_case(&target))
                    .map(|buddy| buddy.name.clone())
                else {
                    self.push_chat_line(
                        format!("[Buddy] No buddy named {target} was found."),
                        [255, 222, 112],
                    );
                    return;
                };
                match openstory_protocol::set_buddy_group_packet(&name, &group) {
                    Ok(packet) => Ok((packet, format!("Moved {name} to buddy group {group}."))),
                    Err(error) => Err(error.to_string()),
                }
            }
            BuddyCommand::Remove(target) => {
                let Some(character_id) = self.buddy_target_id(&target) else {
                    self.push_chat_line(
                        format!("[Buddy] No buddy named {target} was found."),
                        [255, 222, 112],
                    );
                    return;
                };
                Ok((
                    openstory_protocol::delete_buddy_packet(character_id),
                    format!("Removed {target} from your buddy list."),
                ))
            }
            BuddyCommand::Accept => {
                let Some((character_id, name)) = self.pending_buddy_invite.as_ref() else {
                    self.push_chat_line(
                        "[Buddy] There is no pending invitation.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                };
                Ok((
                    openstory_protocol::accept_buddy_packet(*character_id),
                    format!("Accepted {name}'s buddy request."),
                ))
            }
            BuddyCommand::Decline | BuddyCommand::Filter(_) | BuddyCommand::List => {
                unreachable!()
            }
        };
        let (packet, success) = match packet {
            Ok(request) => request,
            Err(error) => {
                self.push_chat_line(
                    format!("[Buddy] Could not build request: {error}"),
                    [255, 105, 105],
                );
                return;
            }
        };
        if !self.channel_connected {
            self.push_chat_line(
                "[Buddy] Not connected to a game channel.".to_owned(),
                [255, 222, 112],
            );
            return;
        }
        let result = self
            .session
            .as_ref()
            .map(|session| session.send_packet(packet));
        match result {
            Some(Ok(())) => {
                if is_accept {
                    self.pending_buddy_invite = None;
                    self.refresh_buddy_texture();
                }
                self.push_chat_line(format!("[Buddy] {success}"), [190, 225, 255]);
            }
            Some(Err(error)) => {
                self.channel_connected = false;
                self.push_chat_line(
                    format!("[Buddy] Could not send request: {error}"),
                    [255, 105, 105],
                );
            }
            None => self.push_chat_line(
                "[Buddy] Not connected to a game channel.".to_owned(),
                [255, 222, 112],
            ),
        }
    }

    pub(super) fn apply_buddy_event(&mut self, event: openstory_protocol::BuddyEvent) {
        use openstory_protocol::BuddyEvent;
        match event {
            BuddyEvent::FullList(mut entries) => {
                entries.sort_by(|left, right| {
                    right
                        .channel
                        .cmp(&left.channel)
                        .then_with(|| left.name.cmp(&right.name))
                });
                self.buddy_entries = entries;
                self.buddy_scroll =
                    self.buddy_scroll
                        .min(self.buddy_display_count().saturating_sub(
                            if self.pending_buddy_invite.is_some() {
                                5
                            } else {
                                7
                            },
                        ));
            }
            BuddyEvent::Invite { character_id, name } => {
                self.pending_buddy_invite = Some((character_id, name.clone()));
                self.show_buddy_panel = true;
                self.show_party_panel = false;
                self.show_guild_panel = false;
                self.show_alliance_panel = false;
                self.buddy_scroll = self
                    .buddy_scroll
                    .min(self.buddy_entries.len().saturating_sub(5));
                self.push_chat_line(
                    format!("[Buddy] {name} wants to be your buddy. Use /buddy accept or /buddy dismiss."),
                    [255, 222, 112],
                );
            }
            BuddyEvent::Channel {
                character_id,
                channel,
            } => {
                let changed = self
                    .buddy_entries
                    .iter_mut()
                    .find(|buddy| buddy.character_id == character_id)
                    .and_then(|buddy| {
                        let was_online = buddy.channel >= 0;
                        buddy.channel = channel;
                        (was_online != (channel >= 0)).then(|| {
                            format!(
                                "[Buddy] {} has logged {}.",
                                buddy.name,
                                if channel >= 0 { "in" } else { "out" }
                            )
                        })
                    });
                if let Some(message) = changed {
                    self.push_chat_line(message, [255, 222, 112]);
                }
            }
            BuddyEvent::Capacity(capacity) => self.buddy_capacity = capacity,
            BuddyEvent::Error(code) => {
                let message = match code {
                    0x0B => "Your buddy list is full.",
                    0x0C => "That player is already on your buddy list.",
                    0x0D => "That player's buddy list is full.",
                    0x0F => "That character does not exist.",
                    _ => "Buddy request failed.",
                };
                self.push_chat_line(format!("[Buddy] {message}"), [255, 105, 105]);
            }
        }
        self.refresh_buddy_texture();
    }

    pub(super) fn show_character_speech(&mut self, character_id: i32, text: &str) {
        let is_local = self
            .active_character
            .as_ref()
            .is_some_and(|character| character.id == character_id);
        let Some(scene) = self.map_scene.as_mut() else {
            return;
        };
        if is_local {
            scene.local_speech_bubble_remaining = Some(SPEECH_BUBBLE_DURATION);
        } else if let Some(player) = scene.remote_players.get_mut(&character_id) {
            player.speech_bubble_remaining = Some(SPEECH_BUBBLE_DURATION);
        } else {
            return;
        }

        let Some(skin) = self.chat_balloon_skin.as_ref() else {
            return;
        };
        let pixels = render_speech_bubble(&self.font, skin, text);
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if is_local {
            if let Some(texture) = self.local_speech_texture.as_ref() {
                renderer.update_bgra(texture, &pixels).map(|()| None)
            } else {
                renderer
                    .upload_bgra(SPEECH_BUBBLE_WIDTH, SPEECH_BUBBLE_HEIGHT, &pixels)
                    .map(Some)
            }
        } else if let Some(texture) = self.remote_speech_textures.get(&character_id) {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(SPEECH_BUBBLE_WIDTH, SPEECH_BUBBLE_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) if is_local => self.local_speech_texture = Some(texture),
            Ok(Some(texture)) => {
                self.remote_speech_textures.insert(character_id, texture);
            }
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update speech balloon: {error}"),
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn show_pet_speech(&mut self, update: openstory_protocol::PetChatUpdate) {
        let key = (update.character_id, update.slot);
        let Some(pet) = self
            .map_scene
            .as_mut()
            .and_then(|scene| scene.pets.get_mut(&key))
        else {
            return;
        };
        pet.speech_bubble_remaining = Some(SPEECH_BUBBLE_DURATION);

        let Some(skin) = self.chat_balloon_skin.as_ref() else {
            return;
        };
        let pixels = render_speech_bubble(&self.font, skin, &update.text);
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.pet_speech_textures.get(&key) {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(SPEECH_BUBBLE_WIDTH, SPEECH_BUBBLE_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => {
                self.pet_speech_textures.insert(key, texture);
            }
            Ok(None) => {}
            Err(error) => {
                eprintln!("OpenStoryRS: could not update pet speech balloon: {error}")
            }
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn submit_chat_message(&mut self) {
        if let Some(compose) = self.cash_item_compose.clone() {
            self.submit_cash_item_message(compose);
            return;
        }
        let message = self.chat_input.trim().to_owned();
        if message.is_empty() {
            self.chat_open = false;
            self.refresh_chat_texture();
            return;
        }
        if message.chars().count() > MAX_CHAT_MESSAGE_CHARS {
            self.push_chat_line(
                format!(
                    "[Notice] Chat messages are limited to {MAX_CHAT_MESSAGE_CHARS} characters."
                ),
                [255, 222, 112],
            );
            return;
        }
        let command = match parse_chat_command(&message) {
            Ok(command) => command,
            Err(notice) => {
                self.push_chat_line(notice.to_owned(), [255, 222, 112]);
                return;
            }
        };
        if let Some(ChatCommand::Trade(command)) = command {
            self.execute_trade_command(command);
            self.chat_input.clear();
            self.chat_open = false;
            self.refresh_chat_texture();
            return;
        }
        if let Some(ChatCommand::Party(command)) = command {
            self.execute_party_command(command);
            self.chat_input.clear();
            self.chat_open = false;
            self.refresh_chat_texture();
            return;
        }
        if let Some(ChatCommand::Buddy(command)) = command {
            self.execute_buddy_command(command);
            self.chat_input.clear();
            self.chat_open = false;
            self.refresh_chat_texture();
            return;
        }
        if let Some(ChatCommand::Alliance(command)) = command {
            self.execute_alliance_command(command);
            self.chat_input.clear();
            self.chat_open = false;
            self.refresh_chat_texture();
            return;
        }
        if let Some(ChatCommand::Guild(command)) = command {
            self.execute_guild_command(command);
            self.chat_input.clear();
            self.chat_open = false;
            self.refresh_chat_texture();
            return;
        }
        if let Some(ChatCommand::OpenWhisper(target)) = command {
            self.chat_input.clear();
            self.chat_open = false;
            self.refresh_chat_texture();
            self.open_whisper_window(&target, true);
            return;
        }
        if let Some(ChatCommand::Whisper { target, message }) = command {
            self.chat_input.clear();
            self.chat_open = false;
            self.refresh_chat_texture();
            self.send_whisper_message(&target, &message, false);
            return;
        }
        if !self.channel_connected {
            self.push_chat_line(
                "[Notice] Not connected to a game channel.".to_owned(),
                [255, 222, 112],
            );
            return;
        }
        let Some(session) = self.session.as_ref() else {
            self.push_chat_line(
                "[Notice] Not connected to a game channel.".to_owned(),
                [255, 222, 112],
            );
            return;
        };
        let (packet, echo) = match command {
            Some(ChatCommand::Whisper { .. } | ChatCommand::OpenWhisper(_)) => {
                unreachable!("whisper commands are handled above")
            }
            Some(ChatCommand::Find { target }) => {
                (openstory_protocol::find_player_packet(&target), None)
            }
            Some(ChatCommand::PartyMessage(message)) => (
                openstory_protocol::multi_chat_packet(
                    openstory_protocol::MultiChatType::Party,
                    &[],
                    &message,
                ),
                Some(format!("[To Party] {message}")),
            ),
            Some(ChatCommand::Party(_)) => unreachable!("party commands are handled above"),
            Some(ChatCommand::Trade(_)) => unreachable!("trade commands are handled above"),
            Some(ChatCommand::Buddy(_)) => unreachable!("buddy commands are handled above"),
            Some(ChatCommand::Alliance(_)) => {
                unreachable!("alliance commands are handled above")
            }
            Some(ChatCommand::Guild(_)) => unreachable!("guild commands are handled above"),
            None => match self.chat_target.multi_chat_type() {
                Some(chat_type) => (
                    openstory_protocol::multi_chat_packet(chat_type, &[], &message),
                    Some(format!("[To {}] {message}", self.chat_target.label())),
                ),
                None => (
                    openstory_protocol::general_chat_packet(&message, true),
                    None,
                ),
            },
        };
        let packet = match packet {
            Ok(packet) => packet,
            Err(error) => {
                self.push_chat_line(
                    format!("[Notice] Could not send chat: {error}"),
                    [255, 105, 105],
                );
                return;
            }
        };
        if let Err(error) = session.send_packet(packet) {
            self.channel_connected = false;
            self.push_chat_line(
                format!("[Notice] Could not send chat: {error}"),
                [255, 105, 105],
            );
            return;
        }
        self.chat_input.clear();
        self.refresh_chat_texture();
        if let Some(echo) = echo {
            self.push_chat_line(echo, [255, 255, 255]);
        }
    }

    pub(super) fn open_cash_item_composer(
        &mut self,
        slot: i16,
        item_id: i32,
        kind: CashItemComposeKind,
    ) {
        self.cash_item_compose = Some(CashItemCompose {
            slot,
            item_id,
            kind,
            lines: Vec::new(),
            recipient: String::new(),
            recipient_prompt_pending: kind.has_recipient_field(item_id),
            whisper: false,
            attached_item: None,
            awaiting_attachment: false,
        });
        self.chat_input.clear();
        self.chat_target = ChatTarget::Public;
        self.chat_open = true;
        let prompt = if kind.has_recipient_field(item_id) {
            "Enter an optional target name (blank Enter skips), then message lines; Escape cancels."
        } else if kind.uses_item_attachment(item_id) {
            "Enter a message; Enter then click an inventory item to attach it, or Enter blank to send without one. Tab toggles the whisper icon."
        } else {
            match kind {
                CashItemComposeKind::PetNameTag => {
                    "Enter the new name; Enter sends and Escape cancels."
                }
                CashItemComposeKind::PersonalShop => {
                    "Enter a shop name; Enter creates it and Escape cancels."
                }
                CashItemComposeKind::HiredMerchant => {
                    "Enter a merchant name; Enter creates it and Escape cancels."
                }
                _ if kind.max_lines(item_id) > 1 => {
                    "Enter a message; Enter adds a line, blank Enter sends, Escape cancels."
                }
                _ => "Enter a message; Enter sends and Escape cancels.",
            }
        };
        let prompt = if kind.has_whisper_toggle(item_id) && !kind.uses_item_attachment(item_id) {
            format!("{prompt} Tab toggles the whisper icon.")
        } else {
            prompt.to_owned()
        };
        self.push_chat_line(
            format!("[{}] {prompt}", kind.label(item_id)),
            [255, 222, 112],
        );
        self.refresh_chat_texture();
    }

    pub(super) fn submit_cash_item_message(&mut self, mut compose: CashItemCompose) {
        let message = self.chat_input.trim().to_owned();
        if compose.recipient_prompt_pending {
            if message.chars().count() > 12 || message.chars().any(char::is_control) {
                self.push_chat_line(
                    "[Notice] Maple TV recipient names are limited to 12 characters.".to_owned(),
                    [255, 222, 112],
                );
                return;
            }
            compose.recipient = message;
            compose.recipient_prompt_pending = false;
            self.cash_item_compose = Some(compose);
            self.chat_input.clear();
            self.push_chat_line(
                "[Maple TV] Enter up to five message lines; blank Enter sends the lines entered."
                    .to_owned(),
                [255, 222, 112],
            );
            self.refresh_chat_texture();
            return;
        }
        if compose.awaiting_attachment && !message.is_empty() {
            self.push_chat_line(
                "[Item Megaphone] The message is ready. Click an inventory item to attach it, or press Enter on a blank line to send."
                    .to_owned(),
                [255, 222, 112],
            );
            return;
        }
        let finish_now = message.is_empty();
        let mut lines = compose.lines.clone();
        if !message.is_empty() {
            if message.chars().count() > compose.kind.max_chars() {
                self.push_chat_line(
                    format!(
                        "[Notice] {} text is limited to {} characters per line.",
                        compose.kind.label(compose.item_id),
                        compose.kind.max_chars()
                    ),
                    [255, 222, 112],
                );
                return;
            }
            lines.push(message);
        }
        if lines.is_empty() {
            return;
        }
        let still_owned = self.inventory.as_ref().is_some_and(|inventory| {
            inventory.items.iter().any(|item| {
                item.inventory_type == openstory_protocol::InventoryType::Cash
                    && item.slot == compose.slot as u16
                    && item.item_id == compose.item_id
            })
        });
        if !still_owned {
            self.cash_item_compose = None;
            self.chat_open = false;
            self.chat_input.clear();
            self.refresh_chat_texture();
            self.push_chat_line(
                "[Notice] The selected cash item is no longer in your Cash inventory.".to_owned(),
                [255, 222, 112],
            );
            return;
        }
        if !self.channel_connected {
            self.push_chat_line(
                "[Notice] Cash item use requires a field connection.".to_owned(),
                [255, 222, 112],
            );
            return;
        }
        if compose.kind.uses_item_attachment(compose.item_id) && !compose.awaiting_attachment {
            compose.lines = lines;
            compose.awaiting_attachment = true;
            self.cash_item_compose = Some(compose);
            self.chat_input.clear();
            self.push_chat_line(
                "[Item Megaphone] Click an inventory item to attach it; click it again to clear. Press Enter to send, or Enter on blank to send without an attachment. Press I to reopen the inventory."
                    .to_owned(),
                [255, 222, 112],
            );
            self.refresh_chat_texture();
            return;
        }
        let maximum_lines = compose.kind.max_lines(compose.item_id);
        if lines.len() < maximum_lines && !finish_now {
            let next_line = lines.len() + 1;
            self.cash_item_compose = Some(CashItemCompose {
                lines,
                ..compose.clone()
            });
            self.chat_input.clear();
            self.push_chat_line(
                format!(
                    "[{}] Line saved. Enter line {next_line}, or press Enter on a blank line to send.",
                    compose.kind.label(compose.item_id)
                ),
                [255, 222, 112],
            );
            self.refresh_chat_texture();
            return;
        }
        let message = lines.first().map(String::as_str).unwrap_or_default();
        let attached_item = compose
            .attached_item
            .and_then(|(inventory_type, slot, item_id)| {
                self.inventory.as_ref().and_then(|inventory| {
                    inventory
                        .items
                        .iter()
                        .any(|item| {
                            item.inventory_type == inventory_type
                                && item.slot == slot
                                && item.item_id == item_id
                        })
                        .then_some((inventory_type, slot))
                })
            });
        if compose.attached_item.is_some() && attached_item.is_none() {
            self.push_chat_line(
                "[Item Megaphone] The selected attachment is no longer in that inventory slot; sending without it."
                    .to_owned(),
                [255, 222, 112],
            );
        }
        let packet = match compose.kind {
            CashItemComposeKind::Megaphone => openstory_protocol::use_megaphone_packet(
                compose.slot,
                compose.item_id,
                &lines,
                compose.whisper,
                attached_item,
            ),
            CashItemComposeKind::MapleTv => openstory_protocol::use_maple_tv_packet(
                compose.slot,
                compose.item_id,
                &compose.recipient,
                &lines,
                matches!(compose.item_id, 5_075_003..=5_075_005),
            ),
            CashItemComposeKind::AvatarMegaphone => {
                openstory_protocol::use_avatar_megaphone_packet(
                    compose.slot,
                    compose.item_id,
                    &lines,
                    false,
                )
            }
            CashItemComposeKind::PetNameTag => {
                openstory_protocol::use_pet_name_tag_packet(compose.slot, compose.item_id, message)
            }
            CashItemComposeKind::PersonalShop => {
                openstory_protocol::create_personal_shop_packet(message, compose.item_id)
            }
            CashItemComposeKind::HiredMerchant => {
                openstory_protocol::create_hired_merchant_shop_packet(message, compose.item_id)
            }
        };
        let Some(packet) = packet else {
            self.push_chat_line(
                format!(
                    "[Notice] {} item payload is invalid.",
                    compose.kind.label(compose.item_id)
                ),
                [255, 222, 112],
            );
            return;
        };
        let Some(session) = self.session.as_ref() else {
            self.push_chat_line(
                "[Notice] Not connected to a game channel.".to_owned(),
                [255, 222, 112],
            );
            return;
        };
        if let Err(error) = session.send_packet(packet) {
            self.channel_connected = false;
            self.push_chat_line(
                format!(
                    "[Notice] Could not use {}: {error}",
                    compose.kind.label(compose.item_id)
                ),
                [255, 105, 105],
            );
            return;
        }
        self.chat_input.clear();
        self.chat_open = false;
        self.cash_item_compose = None;
        self.refresh_chat_texture();
    }
}
