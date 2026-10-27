use super::*;

impl ClientApp {
    pub(super) fn refresh_party_texture(&mut self) {
        let local_id = self.active_character.as_ref().map(|character| character.id);
        let pixels = render_party_panel(
            &self.font,
            self.party_state.as_ref(),
            self.pending_party_invite.as_ref(),
            local_id,
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.party_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(PARTY_PANEL_WIDTH, PARTY_PANEL_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.party_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update party panel: {error}"),
        }
    }

    pub(super) fn refresh_guild_texture(&mut self) {
        let (background_preview, mark_preview) = self.guild_emblem_preview_bitmaps();
        let pixels = render_guild_panel(
            &self.font,
            self.guild_info.as_ref(),
            self.pending_guild_invite.as_ref(),
            self.pending_guild_creation_invite.as_ref(),
            self.guild_scroll,
            &self.guild_panel_mode,
            background_preview.as_ref(),
            mark_preview.as_ref(),
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.guild_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(GUILD_PANEL_WIDTH, GUILD_PANEL_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.guild_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update guild panel: {error}"),
        }
    }

    pub(super) fn refresh_family_texture(&mut self) {
        let pixels = render_family_panel(&self.font, &self.ui_archive, &self.family_ui);
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.family_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(FAMILY_PANEL_WIDTH, FAMILY_PANEL_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.family_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update family panel: {error}"),
        }
    }

    pub(super) fn refresh_family_tree_texture(&mut self) {
        let pixels = render_family_tree_panel(&self.font, &self.ui_archive, &self.family_ui);
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.family_tree_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(FAMILY_TREE_PANEL_WIDTH, FAMILY_TREE_PANEL_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.family_tree_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update family tree: {error}"),
        }
    }

    pub(super) fn toggle_family_panel(&mut self) {
        self.show_family_panel = !self.show_family_panel;
        if self.show_family_panel {
            self.show_party_panel = false;
            self.show_buddy_panel = false;
            self.show_guild_panel = false;
            self.show_alliance_panel = false;
            self.show_family_tree = false;
            if let Some(session) = self.session.as_ref()
                && let Err(error) = session.send_packet(openstory_protocol::open_family_packet())
            {
                self.family_ui.notice = Some(format!("Could not request family data: {error}"));
            }
        } else {
            self.show_family_tree = false;
        }
        self.refresh_family_texture();
        if self.show_family_tree {
            self.refresh_family_tree_texture();
        }
    }

    pub(super) fn open_family_tree(&mut self) {
        let Some(name) = self
            .active_character
            .as_ref()
            .map(|character| character.stats.name.clone())
        else {
            self.family_ui.notice = Some("Character data is not available yet.".to_owned());
            self.refresh_family_texture();
            return;
        };
        let packet = match openstory_protocol::family_pedigree_request_packet(&name) {
            Ok(packet) => packet,
            Err(error) => {
                self.family_ui.notice = Some(format!("Could not request family tree: {error}"));
                self.refresh_family_texture();
                return;
            }
        };
        if let Some(session) = self.session.as_ref()
            && let Err(error) = session.send_packet(packet)
        {
            self.family_ui.notice = Some(format!("Could not request family tree: {error}"));
        } else {
            self.show_family_tree = true;
            self.family_ui.notice = None;
        }
        self.refresh_family_texture();
        self.refresh_family_tree_texture();
    }

    pub(super) fn apply_family_event(&mut self, event: openstory_protocol::FamilyEvent) {
        use openstory_protocol::FamilyEvent;
        match event {
            FamilyEvent::Info(info) => {
                self.family_ui.info = Some(info);
                self.family_ui.notice = None;
            }
            FamilyEvent::Chart(chart) => {
                self.family_ui.selected_member = Some(chart.viewed_character_id);
                self.family_ui.chart = Some(chart);
                self.show_family_tree = true;
            }
            FamilyEvent::Result { result_code, mesos } => {
                let message = match result_code {
                    65 => "Player not found.".to_owned(),
                    69 => "Target is on a different map or hidden.".to_owned(),
                    72 => "Level difference is too great (maximum 20).".to_owned(),
                    73 => "Target already has a pending family invitation.".to_owned(),
                    74 => "Target is already being summoned by another family.".to_owned(),
                    75 => "You cannot teleport to that location right now.".to_owned(),
                    77 => "Target is too low level to be a junior.".to_owned(),
                    _ => format!("Family result {result_code} (mesos: {mesos})."),
                };
                self.family_ui.notice = Some(message.clone());
                self.push_chat_line(format!("[Family] {message}"), [255, 222, 112]);
            }
            FamilyEvent::JoinRequest { character_id, name } => {
                self.pending_family_join = Some((character_id, name.clone()));
                self.push_chat_line(
                    format!("[Family] {name} wants to join. Press Y to accept or N to decline."),
                    [255, 222, 112],
                );
            }
            FamilyEvent::JoinRequestResult { accepted, name } => {
                let message = if accepted {
                    format!("{name} accepted your family invitation.")
                } else {
                    format!("{name} declined your family invitation.")
                };
                self.push_chat_line(format!("[Family] {message}"), [255, 222, 112]);
            }
            FamilyEvent::Joined { name, .. } => {
                self.push_chat_line(
                    format!("[Family] {name} joined the family."),
                    [255, 222, 112],
                );
                if self.show_family_panel {
                    self.open_family_panel_refresh();
                }
            }
            FamilyEvent::Privileges(privileges) => {
                self.family_ui.privileges = privileges;
            }
            FamilyEvent::ReputationGained { amount, from } => {
                self.push_chat_line(
                    format!("[Family] +{amount} reputation from {from}."),
                    [255, 222, 112],
                );
            }
            FamilyEvent::MemberPresenceChanged { logged_in, name } => {
                let action = if logged_in { "logged in" } else { "logged out" };
                self.push_chat_line(format!("[Family] {name} {action}."), [255, 222, 112]);
            }
            FamilyEvent::SummonRequest { from, family_name } => {
                self.pending_family_summon = Some((from.clone(), family_name.clone()));
                self.push_chat_line(
                    format!("[Family] {from} is summoning you. Press Y to accept or N to decline."),
                    [255, 222, 112],
                );
            }
        }
        if self.show_family_panel {
            self.refresh_family_texture();
        }
        if self.show_family_tree {
            self.refresh_family_tree_texture();
        }
    }

    pub(super) fn open_family_panel_refresh(&mut self) {
        if let Some(session) = self.session.as_ref()
            && let Err(error) = session.send_packet(openstory_protocol::open_family_packet())
        {
            self.family_ui.notice = Some(format!("Could not refresh family data: {error}"));
        }
        self.refresh_family_texture();
    }

    pub(super) fn answer_family_request(&mut self, accept: bool) {
        if let Some((character_id, name)) = self.pending_family_join.take() {
            if accept {
                match openstory_protocol::family_accept_request_packet(character_id) {
                    Ok(packet) => {
                        if let Some(session) = self.session.as_ref()
                            && let Err(error) = session.send_packet(packet)
                        {
                            self.push_chat_line(
                                format!("[Family] Could not accept {name}: {error}"),
                                [255, 105, 105],
                            );
                        }
                    }
                    Err(error) => self.push_chat_line(
                        format!("[Family] Could not accept {name}: {error}"),
                        [255, 105, 105],
                    ),
                }
            } else {
                self.push_chat_line(
                    format!("[Family] Declined {name}'s request."),
                    [255, 222, 112],
                );
            }
            return;
        }
        if let Some((from, family_name)) = self.pending_family_summon.take() {
            match openstory_protocol::family_summon_response_packet(&family_name, accept) {
                Ok(packet) => {
                    if let Some(session) = self.session.as_ref()
                        && let Err(error) = session.send_packet(packet)
                    {
                        self.push_chat_line(
                            format!("[Family] Could not respond to {from}: {error}"),
                            [255, 105, 105],
                        );
                    }
                }
                Err(error) => self.push_chat_line(
                    format!("[Family] Could not respond to {from}: {error}"),
                    [255, 105, 105],
                ),
            }
        }
    }

    pub(super) fn click_family_panel(&mut self, cursor: Point) {
        let x = cursor.x - 375.0;
        let y = cursor.y - 165.0;
        if self.family_ui.input.is_some() {
            if (53.0..132.0).contains(&x) && (253.0..281.0).contains(&y) {
                self.submit_family_input();
            } else if (143.0..222.0).contains(&x) && (253.0..281.0).contains(&y) {
                self.family_ui.input = None;
            }
            self.refresh_family_texture();
            return;
        }
        if self.family_ui.confirm_leave {
            if (53.0..132.0).contains(&x) && (253.0..281.0).contains(&y) {
                self.resolve_family_leave(true);
            } else if (143.0..222.0).contains(&x) && (253.0..281.0).contains(&y) {
                self.family_ui.confirm_leave = false;
            }
            self.refresh_family_texture();
            return;
        }
        if (242.0..270.0).contains(&x) && (4.0..32.0).contains(&y) {
            self.show_family_panel = false;
            self.show_family_tree = false;
        } else if (12.0..72.0).contains(&x) && (390.0..428.0).contains(&y) {
            self.open_family_tree();
        } else if (76.0..126.0).contains(&x) && (390.0..428.0).contains(&y) {
            self.use_family_ability();
        } else if (130.0..188.0).contains(&x) && (390.0..428.0).contains(&y) {
            self.family_ui.input = Some(FamilyInput {
                kind: FamilyInputKind::InviteJunior,
                text: String::new(),
                key_text_for_ime_dedup: None,
            });
            self.family_ui.notice = None;
        } else if (192.0..260.0).contains(&x) && (390.0..428.0).contains(&y) {
            self.family_ui.confirm_leave = true;
        } else if (236.0..266.0).contains(&x) && (300.0..332.0).contains(&y) {
            self.family_ui.selected_ability = (self.family_ui.selected_ability + 1) % 5;
        } else if (196.0..226.0).contains(&x) && (300.0..332.0).contains(&y) {
            self.family_ui.selected_ability = self.family_ui.selected_ability.saturating_sub(1);
        } else if (14.0..260.0).contains(&x) && (185.0..237.0).contains(&y) {
            self.family_ui.input = Some(FamilyInput {
                kind: FamilyInputKind::EditPrecepts,
                text: self
                    .family_ui
                    .info
                    .as_ref()
                    .map(|info| info.message.clone())
                    .unwrap_or_default(),
                key_text_for_ime_dedup: None,
            });
            self.family_ui.notice = None;
        }
        self.refresh_family_texture();
    }

    pub(super) fn edit_family_input(&mut self, text: &str) {
        let Some(input) = self.family_ui.input.as_mut() else {
            return;
        };
        let max_utf16 = match input.kind {
            FamilyInputKind::InviteJunior => 13,
            FamilyInputKind::EditPrecepts => 200,
        };
        for character in text.chars().filter(|character| !character.is_control()) {
            if input.text.encode_utf16().count() + character.len_utf16() > max_utf16 {
                break;
            }
            input.text.push(character);
        }
        self.family_ui.notice = None;
        self.refresh_family_texture();
    }

    pub(super) fn backspace_family_input(&mut self) {
        if let Some(input) = self.family_ui.input.as_mut() {
            input.text.pop();
            self.family_ui.notice = None;
            self.refresh_family_texture();
        }
    }

    pub(super) fn submit_family_input(&mut self) {
        let Some(input) = self.family_ui.input.take() else {
            return;
        };
        let value = match input.kind {
            FamilyInputKind::InviteJunior => input.text.trim(),
            FamilyInputKind::EditPrecepts => input.text.as_str(),
        };
        if value.trim().is_empty() {
            self.family_ui.notice = Some(match input.kind {
                FamilyInputKind::InviteJunior => "Enter a character name first.".to_owned(),
                FamilyInputKind::EditPrecepts => "The family motto cannot be empty.".to_owned(),
            });
            return;
        }
        let packet = match input.kind {
            FamilyInputKind::InviteJunior => openstory_protocol::family_add_junior_packet(value),
            FamilyInputKind::EditPrecepts => openstory_protocol::family_precepts_packet(value),
        };
        match packet {
            Ok(packet) => {
                let Some(session) = self.session.as_ref() else {
                    self.family_ui.notice = Some("No server connection.".to_owned());
                    self.refresh_family_texture();
                    return;
                };
                if let Err(error) = session.send_packet(packet) {
                    self.family_ui.notice = Some(format!("Could not send family request: {error}"));
                } else {
                    self.family_ui.notice = Some(match input.kind {
                        FamilyInputKind::InviteJunior => format!("Invitation sent to {value}."),
                        FamilyInputKind::EditPrecepts => {
                            "Family motto update requested.".to_owned()
                        }
                    });
                    self.open_family_panel_refresh();
                }
            }
            Err(error) => {
                self.family_ui.notice = Some(format!("Invalid family request: {error}"));
            }
        }
        self.refresh_family_texture();
    }

    pub(super) fn resolve_family_leave(&mut self, leave: bool) {
        self.family_ui.confirm_leave = false;
        if !leave {
            self.refresh_family_texture();
            return;
        }
        let Some(entry_id) = self.active_character.as_ref().map(|character| character.id) else {
            self.family_ui.notice = Some("Character data is not available yet.".to_owned());
            self.refresh_family_texture();
            return;
        };
        let Some(session) = self.session.as_ref() else {
            self.family_ui.notice = Some("No server connection.".to_owned());
            self.refresh_family_texture();
            return;
        };
        if let Err(error) = session.send_packet(openstory_protocol::family_leave_packet(entry_id)) {
            self.family_ui.notice = Some(format!("Could not leave the family: {error}"));
        } else {
            self.family_ui.info = None;
            self.family_ui.chart = None;
            self.family_ui.notice = Some("Family leave requested.".to_owned());
            self.open_family_panel_refresh();
        }
        self.refresh_family_texture();
    }

    pub(super) fn click_family_tree(&mut self, cursor: Point) {
        let x = cursor.x - 223.0;
        let y = cursor.y - 191.0;
        if (542.0..576.0).contains(&x) && (4.0..36.0).contains(&y) {
            self.show_family_tree = false;
        } else if (12.0..566.0).contains(&x)
            && (44.0..365.0).contains(&y)
            && let Some(chart) = self.family_ui.chart.as_ref()
        {
            let row = ((y - 44.0) / 30.0) as usize;
            if let Some(entry) = chart.entries.get(row).cloned() {
                self.family_ui.selected_member = Some(entry.character_id);
                if let Ok(packet) = openstory_protocol::family_pedigree_request_packet(&entry.name)
                    && let Some(session) = self.session.as_ref()
                    && let Err(error) = session.send_packet(packet)
                {
                    self.family_ui.notice = Some(format!("Could not view {}: {error}", entry.name));
                }
            }
        }
        if self.show_family_tree {
            self.refresh_family_tree_texture();
        }
    }

    pub(super) fn use_family_ability(&mut self) {
        let ability = self.family_ui.selected_ability.min(4) as i32;
        let target = if ability <= 1 {
            self.family_ui
                .chart
                .as_ref()
                .and_then(|chart| {
                    self.family_ui
                        .selected_member
                        .and_then(|id| chart.entries.iter().find(|entry| entry.character_id == id))
                })
                .map(|entry| entry.name.as_str())
        } else {
            None
        };
        if ability <= 1 && target.is_none() {
            self.family_ui.notice =
                Some("Open the family tree and select a member first.".to_owned());
            self.refresh_family_texture();
            return;
        }
        match openstory_protocol::family_use_packet(ability, target) {
            Ok(packet) => {
                if let Some(session) = self.session.as_ref()
                    && let Err(error) = session.send_packet(packet)
                {
                    self.family_ui.notice = Some(format!("Could not use family ability: {error}"));
                } else {
                    self.family_ui.notice = Some("Family ability requested.".to_owned());
                }
            }
            Err(error) => self.family_ui.notice = Some(format!("Invalid family ability: {error}")),
        }
        self.refresh_family_texture();
    }

    pub(super) fn edit_guild_panel_text(&mut self, text: &str) {
        match &mut self.guild_panel_mode {
            GuildPanelMode::Create { name, notice } => {
                for character in text.chars() {
                    if character.is_ascii_alphabetic() && name.len() < 12 {
                        name.push(character);
                    }
                }
                *notice = None;
            }
            GuildPanelMode::RankTitles {
                titles,
                selected,
                notice,
            } => {
                let title = &mut titles[*selected];
                for character in text.chars() {
                    if !character.is_control()
                        && title.encode_utf16().count() + character.len_utf16() <= 13
                    {
                        title.push(character);
                    }
                }
                *notice = None;
            }
            GuildPanelMode::Overview | GuildPanelMode::Emblem { .. } => {}
        }
        self.refresh_guild_texture();
    }

    pub(super) fn backspace_guild_panel_text(&mut self) {
        match &mut self.guild_panel_mode {
            GuildPanelMode::Create { name, notice } => {
                name.pop();
                *notice = None;
            }
            GuildPanelMode::RankTitles {
                titles,
                selected,
                notice,
            } => {
                titles[*selected].pop();
                *notice = None;
            }
            GuildPanelMode::Overview | GuildPanelMode::Emblem { .. } => {}
        }
        self.refresh_guild_texture();
    }

    pub(super) fn handle_guild_panel_key(&mut self, key: PhysicalKey, repeat: bool) {
        match key {
            PhysicalKey::Code(KeyCode::Escape) if !repeat => {
                self.guild_panel_mode = GuildPanelMode::Overview;
            }
            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) if !repeat => {
                self.submit_guild_panel_editor();
            }
            PhysicalKey::Code(KeyCode::Backspace) => self.backspace_guild_panel_text(),
            PhysicalKey::Code(KeyCode::Tab) if !repeat => match &mut self.guild_panel_mode {
                GuildPanelMode::RankTitles { selected, .. } => {
                    *selected = (*selected + 1) % 5;
                }
                GuildPanelMode::Emblem { selected, .. } => {
                    *selected = (*selected + 1) % 4;
                }
                GuildPanelMode::Create { .. } | GuildPanelMode::Overview => {}
            },
            PhysicalKey::Code(KeyCode::ArrowUp | KeyCode::ArrowDown) if !repeat => {
                if let GuildPanelMode::RankTitles { selected, .. } = &mut self.guild_panel_mode {
                    *selected = if key == PhysicalKey::Code(KeyCode::ArrowUp) {
                        selected.saturating_sub(1)
                    } else {
                        (*selected + 1).min(4)
                    };
                }
            }
            PhysicalKey::Code(KeyCode::ArrowLeft | KeyCode::ArrowRight) => {
                self.adjust_guild_emblem(if key == PhysicalKey::Code(KeyCode::ArrowLeft) {
                    -1
                } else {
                    1
                });
            }
            _ => {}
        }
        self.refresh_guild_texture();
    }

    pub(super) fn adjust_guild_emblem(&mut self, delta: i16) {
        let GuildPanelMode::Emblem {
            emblem,
            selected,
            backgrounds,
            marks,
        } = &mut self.guild_panel_mode
        else {
            return;
        };
        match *selected {
            0 => {
                emblem.background = step_guild_mark_id(backgrounds, emblem.background, delta);
                emblem.background_color = nearest_guild_mark_color(
                    backgrounds,
                    emblem.background,
                    emblem.background_color,
                );
            }
            1 => {
                emblem.background_color = step_guild_mark_color(
                    backgrounds,
                    emblem.background,
                    emblem.background_color,
                    delta,
                );
            }
            2 => {
                emblem.logo = step_guild_mark_id(marks, emblem.logo, delta);
                emblem.logo_color = nearest_guild_mark_color(marks, emblem.logo, emblem.logo_color);
            }
            3 => {
                emblem.logo_color =
                    step_guild_mark_color(marks, emblem.logo, emblem.logo_color, delta);
            }
            _ => {}
        }
    }

    pub(super) fn guild_emblem_preview_bitmaps(&self) -> (Option<Bitmap>, Option<Bitmap>) {
        let GuildPanelMode::Emblem {
            emblem,
            backgrounds,
            marks,
            ..
        } = &self.guild_panel_mode
        else {
            return (None, None);
        };
        let root = self.ui_archive.root();
        let background = (emblem.background > 0)
            .then(|| {
                root.resolve(&format!(
                    "GuildMark.img/BackGround/{:08}/{}",
                    emblem.background, emblem.background_color
                ))
                .ok()
                .flatten()?
                .bitmap()
                .ok()
                .flatten()
            })
            .flatten();
        let mark = marks
            .iter()
            .find(|choice| choice.id == emblem.logo && choice.id > 0)
            .and_then(|choice| {
                root.resolve(&format!(
                    "GuildMark.img/Mark/{}/{:08}/{}",
                    choice.category, emblem.logo, emblem.logo_color
                ))
                .ok()
                .flatten()?
                .bitmap()
                .ok()
                .flatten()
            });
        let background = background.filter(|_| {
            backgrounds
                .iter()
                .any(|choice| choice.id == emblem.background)
        });
        (background, mark)
    }

    pub(super) fn submit_guild_panel_editor(&mut self) {
        let command = match &self.guild_panel_mode {
            GuildPanelMode::Create { name, .. } => {
                if !(3..=12).contains(&name.len()) {
                    if let GuildPanelMode::Create { notice, .. } = &mut self.guild_panel_mode {
                        *notice = Some("Use 3–12 English letters for the guild name.".into());
                    }
                    return;
                }
                GuildCommand::Create(name.clone())
            }
            GuildPanelMode::RankTitles { titles, .. } => {
                if titles
                    .iter()
                    .any(|title| title.is_empty() || title.encode_utf16().count() > 13)
                {
                    if let GuildPanelMode::RankTitles { notice, .. } = &mut self.guild_panel_mode {
                        *notice = Some("Each rank title needs 1–13 characters.".into());
                    }
                    return;
                }
                GuildCommand::RankTitles(titles.clone())
            }
            GuildPanelMode::Emblem { emblem, .. } => GuildCommand::Emblem {
                background: emblem.background,
                background_color: emblem.background_color,
                logo: emblem.logo,
                logo_color: emblem.logo_color,
            },
            GuildPanelMode::Overview => return,
        };
        self.guild_panel_mode = GuildPanelMode::Overview;
        self.execute_guild_command(command);
        self.refresh_guild_texture();
    }

    pub(super) fn click_guild_panel(&mut self, cursor: Point) {
        let x = cursor.x - 680.0;
        let y = cursor.y - 174.0;
        match &mut self.guild_panel_mode {
            GuildPanelMode::Overview => {
                if self.guild_info.is_some() {
                    if (128.0..208.0).contains(&x) && (5.0..26.0).contains(&y) {
                        if let Some(guild) = self.guild_info.as_ref() {
                            self.guild_panel_mode = GuildPanelMode::RankTitles {
                                titles: guild.rank_titles.clone(),
                                selected: 0,
                                notice: None,
                            };
                        }
                    } else if (216.0..324.0).contains(&x) && (5.0..26.0).contains(&y) {
                        if let Some(guild) = self.guild_info.as_ref() {
                            let emblem = guild.emblem;
                            match load_guild_mark_catalog(&self.ui_archive) {
                                Ok((backgrounds, marks)) => {
                                    self.guild_panel_mode = GuildPanelMode::Emblem {
                                        emblem,
                                        selected: 0,
                                        backgrounds,
                                        marks,
                                    };
                                }
                                Err(error) => self.push_chat_line(
                                    format!("[Guild] Could not load emblem choices: {error}"),
                                    [255, 105, 105],
                                ),
                            }
                        }
                    }
                } else if (90.0..246.0).contains(&x) && (104.0..138.0).contains(&y) {
                    self.guild_panel_mode = GuildPanelMode::Create {
                        name: String::new(),
                        notice: None,
                    };
                }
            }
            GuildPanelMode::Create { .. } => {
                if (34.0..146.0).contains(&x) && (344.0..378.0).contains(&y) {
                    self.submit_guild_panel_editor();
                } else if (190.0..302.0).contains(&x) && (344.0..378.0).contains(&y) {
                    self.guild_panel_mode = GuildPanelMode::Overview;
                }
            }
            GuildPanelMode::RankTitles { selected, .. } => {
                let row = (0..5).find(|index| {
                    (88.0 + *index as f32 * 46.0..124.0 + *index as f32 * 46.0).contains(&y)
                        && (14.0..322.0).contains(&x)
                });
                if let Some(row) = row {
                    *selected = row;
                } else if (34.0..146.0).contains(&x) && (350.0..384.0).contains(&y) {
                    self.submit_guild_panel_editor();
                } else if (190.0..302.0).contains(&x) && (350.0..384.0).contains(&y) {
                    self.guild_panel_mode = GuildPanelMode::Overview;
                }
            }
            GuildPanelMode::Emblem { selected, .. } => {
                for index in 0..4 {
                    let row_y = 176.0 + index as f32 * 38.0;
                    if (row_y..row_y + 30.0).contains(&y) && (14.0..322.0).contains(&x) {
                        *selected = index;
                        if (220.0..248.0).contains(&x) {
                            self.adjust_guild_emblem(-1);
                        } else if (290.0..318.0).contains(&x) {
                            self.adjust_guild_emblem(1);
                        }
                        break;
                    }
                }
                if (34.0..146.0).contains(&x) && (350.0..384.0).contains(&y) {
                    self.submit_guild_panel_editor();
                } else if (190.0..302.0).contains(&x) && (350.0..384.0).contains(&y) {
                    self.guild_panel_mode = GuildPanelMode::Overview;
                }
            }
        }
        self.refresh_guild_texture();
    }

    pub(super) fn refresh_alliance_texture(&mut self) {
        let pixels = render_alliance_panel(
            &self.font,
            self.guild_info.as_ref(),
            self.alliance_info.as_ref(),
            self.pending_alliance_invite.as_ref(),
        );
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.alliance_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(ALLIANCE_PANEL_WIDTH, ALLIANCE_PANEL_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.alliance_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update alliance panel: {error}"),
        }
    }

    pub(super) fn execute_guild_command(&mut self, command: GuildCommand) {
        if matches!(&command, GuildCommand::List) {
            self.show_guild_panel = !self.show_guild_panel;
            if !self.show_guild_panel {
                self.guild_panel_mode = GuildPanelMode::Overview;
            }
            if self.show_guild_panel {
                self.show_party_panel = false;
                self.show_buddy_panel = false;
                self.show_alliance_panel = false;
            }
            self.refresh_guild_texture();
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
            return;
        }
        if let GuildCommand::Create(name) = &command {
            if self.guild_info.is_some() {
                self.push_chat_line(
                    "[Guild] Leave your current guild before creating another.".to_owned(),
                    [255, 222, 112],
                );
                return;
            }
            if self.party_state.is_some() {
                self.push_chat_line(
                    "[Guild] Leave your party before starting guild creation.".to_owned(),
                    [255, 222, 112],
                );
                return;
            }
            match openstory_protocol::create_guild_packet(name) {
                Ok(packet) => {
                    self.send_guild_packet(
                        packet,
                        "Creation request sent; eligible nearby players will be asked to confirm.",
                    );
                }
                Err(error) => self.push_chat_line(
                    format!("[Guild] Could not build creation request: {error}"),
                    [255, 105, 105],
                ),
            }
            return;
        }
        let Some(guild) = self.guild_info.as_ref() else {
            self.push_chat_line(
                "[Guild] You must be in a guild to use that action.".to_owned(),
                [255, 222, 112],
            );
            return;
        };
        let request = match command {
            GuildCommand::Create(_) => unreachable!(),
            GuildCommand::Invite(name) => match openstory_protocol::invite_to_guild_packet(&name) {
                Ok(packet) => Some((packet, format!("Invitation sent to {name}."))),
                Err(error) => {
                    self.push_chat_line(
                        format!("[Guild] Could not build invitation: {error}"),
                        [255, 105, 105],
                    );
                    return;
                }
            },
            GuildCommand::Leave => {
                let Some(character) = self.active_character.as_ref() else {
                    self.push_chat_line(
                        "[Guild] Character information is unavailable.".to_owned(),
                        [255, 105, 105],
                    );
                    return;
                };
                match openstory_protocol::leave_guild_packet(character.id, &character.stats.name) {
                    Ok(packet) => Some((packet, "Leave request sent.".to_owned())),
                    Err(error) => {
                        self.push_chat_line(
                            format!("[Guild] Could not build leave request: {error}"),
                            [255, 105, 105],
                        );
                        return;
                    }
                }
            }
            GuildCommand::Expel(target) => {
                let Some(member) = guild
                    .members
                    .iter()
                    .find(|member| member.name.eq_ignore_ascii_case(&target))
                else {
                    self.push_chat_line(
                        format!("[Guild] No guild member named {target} was found."),
                        [255, 222, 112],
                    );
                    return;
                };
                if member.character_id
                    == self
                        .active_character
                        .as_ref()
                        .map_or(0, |character| character.id)
                {
                    self.push_chat_line(
                        "[Guild] Use /guild leave to leave your own guild.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                }
                match openstory_protocol::expel_guild_member_packet(
                    member.character_id,
                    &member.name,
                ) {
                    Ok(packet) => Some((packet, format!("Expel request sent for {target}."))),
                    Err(error) => {
                        self.push_chat_line(
                            format!("[Guild] Could not build expel request: {error}"),
                            [255, 105, 105],
                        );
                        return;
                    }
                }
            }
            GuildCommand::MemberRank { target, rank } => {
                let Some(member) = guild
                    .members
                    .iter()
                    .find(|member| member.name.eq_ignore_ascii_case(&target))
                else {
                    self.push_chat_line(
                        format!("[Guild] No guild member named {target} was found."),
                        [255, 222, 112],
                    );
                    return;
                };
                Some((
                    openstory_protocol::guild_member_rank_packet(member.character_id, rank),
                    format!("Rank change requested for {target}."),
                ))
            }
            GuildCommand::RankTitles(titles) => {
                match openstory_protocol::guild_rank_titles_packet(&titles) {
                    Ok(packet) => Some((packet, "Rank-title update requested.".to_owned())),
                    Err(error) => {
                        self.push_chat_line(
                            format!("[Guild] Could not build rank titles: {error}"),
                            [255, 105, 105],
                        );
                        return;
                    }
                }
            }
            GuildCommand::Emblem {
                background,
                background_color,
                logo,
                logo_color,
            } => Some((
                openstory_protocol::guild_emblem_packet(
                    background,
                    background_color,
                    logo,
                    logo_color,
                ),
                "Emblem change requested.".to_owned(),
            )),
            GuildCommand::Notice(notice) => {
                let length = notice.encode_utf16().count();
                if length > 100 {
                    self.push_chat_line(
                        format!(
                            "[Guild] Notice is {length} UTF-16 code units; the server limit is 100."
                        ),
                        [255, 222, 112],
                    );
                    return;
                }
                match openstory_protocol::guild_notice_packet(&notice) {
                    Ok(packet) => Some((packet, "Notice update requested.".to_owned())),
                    Err(error) => {
                        self.push_chat_line(
                            format!("[Guild] Could not build notice: {error}"),
                            [255, 105, 105],
                        );
                        return;
                    }
                }
            }
            GuildCommand::List => unreachable!(),
        };
        let Some((packet, success)) = request else {
            return;
        };
        self.send_guild_packet(packet, &success);
    }

    pub(super) fn send_guild_packet(&mut self, packet: Vec<u8>, success: &str) -> bool {
        if !self.channel_connected {
            self.push_chat_line(
                "[Guild] Not connected to a game channel.".to_owned(),
                [255, 222, 112],
            );
            return false;
        }
        match self
            .session
            .as_ref()
            .map(|session| session.send_packet(packet))
        {
            Some(Ok(())) => {
                self.push_chat_line(format!("[Guild] {success}"), [190, 225, 255]);
                true
            }
            Some(Err(error)) => {
                self.channel_connected = false;
                self.push_chat_line(
                    format!("[Guild] Could not send request: {error}"),
                    [255, 105, 105],
                );
                false
            }
            None => {
                self.push_chat_line(
                    "[Guild] Not connected to a game channel.".to_owned(),
                    [255, 222, 112],
                );
                false
            }
        }
    }

    pub(super) fn resolve_guild_creation_invite(&mut self, accepted: bool) {
        let Some((leader, guild_name)) = self.pending_guild_creation_invite.as_ref() else {
            return;
        };
        let success = if accepted {
            format!("Agreed to help create {guild_name}.")
        } else {
            format!("Declined {leader}'s guild-creation request.")
        };
        if self.send_guild_packet(
            openstory_protocol::guild_creation_confirmation_packet(accepted),
            &success,
        ) {
            self.pending_guild_creation_invite = None;
            self.refresh_guild_texture();
        }
    }

    pub(super) fn open_alliance_panel(&mut self) {
        self.show_alliance_panel = true;
        self.show_party_panel = false;
        self.show_buddy_panel = false;
        self.show_guild_panel = false;
        self.show_inventory = false;
        self.show_skill_book = false;
        self.show_quest_log = false;
        self.show_pet_equipment = false;
        self.inventory_drag_source = None;
        self.pending_scroll_slot = None;
        self.inventory_hover_slot = None;
        if self.channel_connected
            && self
                .guild_info
                .as_ref()
                .is_some_and(|guild| guild.alliance_id > 0)
        {
            self.send_alliance_packet(openstory_protocol::alliance_info_request_packet());
        }
        self.refresh_alliance_texture();
    }

    pub(super) fn apply_alliance_event(&mut self, event: openstory_protocol::AllianceEvent) {
        use openstory_protocol::{AllianceEvent, AllianceInfo};
        match event {
            AllianceEvent::Invite {
                alliance_id,
                inviter,
            } => {
                self.pending_alliance_invite = Some((alliance_id, inviter.clone()));
                self.open_alliance_panel();
                self.push_chat_line(
                    format!("[Alliance] {inviter} invited your guild. Press Y to accept or N to decline."),
                    [190, 225, 255],
                );
            }
            AllianceEvent::ShowInfo {
                alliance_id,
                character_id,
            } => {
                if self
                    .active_character
                    .as_ref()
                    .is_some_and(|character| character.id == character_id)
                    && self
                        .alliance_info
                        .as_ref()
                        .is_none_or(|info| info.id != alliance_id)
                {
                    self.alliance_info = Some(AllianceInfo {
                        id: alliance_id,
                        name: format!("Alliance {alliance_id}"),
                        rank_titles: std::array::from_fn(|_| String::new()),
                        capacity: 0,
                        notice: String::new(),
                        guilds: Vec::new(),
                    });
                }
            }
            AllianceEvent::Snapshot(info) if info.id == 0 => {
                self.alliance_info = None;
                if let Some(guild) = self.guild_info.as_mut() {
                    guild.alliance_id = 0;
                }
            }
            AllianceEvent::Snapshot(info) => {
                let belongs_to_current_alliance = self
                    .guild_info
                    .as_ref()
                    .is_none_or(|guild| guild.alliance_id == 0 || guild.alliance_id == info.id);
                if belongs_to_current_alliance {
                    if let Some(existing) = self
                        .alliance_info
                        .as_ref()
                        .filter(|existing| existing.id == info.id)
                    {
                        let mut info = info;
                        if info.notice.is_empty() {
                            info.notice.clone_from(&existing.notice);
                        }
                        for guild in &mut info.guilds {
                            if guild.name.is_empty()
                                && let Some(previous) = existing
                                    .guilds
                                    .iter()
                                    .find(|previous| previous.id == guild.id)
                            {
                                guild.name.clone_from(&previous.name);
                            }
                        }
                        self.alliance_info = Some(info);
                    } else {
                        self.alliance_info = Some(info);
                    }
                }
            }
            AllianceEvent::GuildList(guilds) => {
                let alliance_id = self.alliance_info.as_ref().map(|info| info.id).or_else(|| {
                    self.guild_info
                        .as_ref()
                        .filter(|guild| guild.alliance_id > 0)
                        .map(|guild| guild.alliance_id)
                });
                if let Some(alliance_id) = alliance_id {
                    let mut info = self.alliance_info.take().unwrap_or_else(|| AllianceInfo {
                        id: alliance_id,
                        name: format!("Alliance {alliance_id}"),
                        rank_titles: std::array::from_fn(|_| String::new()),
                        capacity: 0,
                        notice: String::new(),
                        guilds: Vec::new(),
                    });
                    info.guilds = guilds;
                    if let Some(local_guild) = self.guild_info.as_ref() {
                        if local_guild.alliance_id == alliance_id
                            && let Some(entry) = info
                                .guilds
                                .iter_mut()
                                .find(|entry| entry.id == local_guild.id)
                        {
                            entry.name.clone_from(&local_guild.name);
                        }
                    }
                    self.alliance_info = Some(info);
                }
            }
            AllianceEvent::MemberOnline {
                alliance_id,
                guild_id,
                character_id,
                online,
            } => {
                if let Some(member) = self
                    .guild_info
                    .as_mut()
                    .filter(|guild| guild.id == guild_id && guild.alliance_id == alliance_id)
                    .and_then(|guild| {
                        guild
                            .members
                            .iter_mut()
                            .find(|member| member.character_id == character_id)
                    })
                {
                    member.online = online;
                }
                if self
                    .active_character
                    .as_ref()
                    .is_none_or(|character| character.id != character_id)
                {
                    self.push_chat_line(
                        format!(
                            "[Alliance] A member has {}.",
                            if online { "logged in" } else { "logged out" }
                        ),
                        [255, 222, 112],
                    );
                }
            }
            AllianceEvent::MemberStatsChanged {
                alliance_id,
                guild_id,
                character_id,
                level,
                job,
            } => {
                if let Some(member) = self
                    .guild_info
                    .as_mut()
                    .filter(|guild| guild.id == guild_id && guild.alliance_id == alliance_id)
                    .and_then(|guild| {
                        guild
                            .members
                            .iter_mut()
                            .find(|member| member.character_id == character_id)
                    })
                {
                    member.level = level;
                    member.job = job;
                }
            }
            AllianceEvent::RankTitlesChanged {
                alliance_id,
                titles,
            } => {
                if self
                    .alliance_info
                    .as_ref()
                    .is_none_or(|info| info.id != alliance_id)
                    && self
                        .guild_info
                        .as_ref()
                        .is_some_and(|guild| guild.alliance_id == alliance_id)
                {
                    self.alliance_info = Some(AllianceInfo {
                        id: alliance_id,
                        name: format!("Alliance {alliance_id}"),
                        rank_titles: std::array::from_fn(|_| String::new()),
                        capacity: 0,
                        notice: String::new(),
                        guilds: Vec::new(),
                    });
                }
                if let Some(info) = self
                    .alliance_info
                    .as_mut()
                    .filter(|info| info.id == alliance_id)
                {
                    info.rank_titles = titles;
                }
            }
            AllianceEvent::NoticeChanged {
                alliance_id,
                notice,
            } => {
                if self
                    .alliance_info
                    .as_ref()
                    .is_none_or(|info| info.id != alliance_id)
                    && self
                        .guild_info
                        .as_ref()
                        .is_some_and(|guild| guild.alliance_id == alliance_id)
                {
                    self.alliance_info = Some(AllianceInfo {
                        id: alliance_id,
                        name: format!("Alliance {alliance_id}"),
                        rank_titles: std::array::from_fn(|_| String::new()),
                        capacity: 0,
                        notice: String::new(),
                        guilds: Vec::new(),
                    });
                }
                if let Some(info) = self
                    .alliance_info
                    .as_mut()
                    .filter(|info| info.id == alliance_id)
                {
                    info.notice = notice;
                }
            }
            AllianceEvent::Disbanded { alliance_id } => {
                if self
                    .alliance_info
                    .as_ref()
                    .is_some_and(|info| info.id == alliance_id)
                {
                    self.alliance_info = None;
                }
                if let Some(guild) = self
                    .guild_info
                    .as_mut()
                    .filter(|guild| guild.alliance_id == alliance_id)
                {
                    guild.alliance_id = 0;
                }
                self.push_chat_line(
                    "[Alliance] The alliance was disbanded.".to_owned(),
                    [255, 222, 112],
                );
            }
        }
        if self.show_alliance_panel {
            self.refresh_alliance_texture();
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn send_alliance_packet(&mut self, packet: Vec<u8>) -> bool {
        if !self.channel_connected {
            self.push_chat_line(
                "[Alliance] Not connected to a game channel.".to_owned(),
                [255, 222, 112],
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
                    format!("[Alliance] Could not send request: {error}"),
                    [255, 105, 105],
                );
                false
            }
            None => {
                self.push_chat_line(
                    "[Alliance] Not connected to a game channel.".to_owned(),
                    [255, 222, 112],
                );
                false
            }
        }
    }

    pub(super) fn execute_alliance_command(&mut self, command: AllianceCommand) {
        if matches!(&command, AllianceCommand::List) {
            if self.show_alliance_panel {
                self.show_alliance_panel = false;
            } else {
                self.open_alliance_panel();
            }
            self.refresh_alliance_texture();
            return;
        }
        let is_response = matches!(&command, AllianceCommand::Accept | AllianceCommand::Deny);
        let accepted = matches!(&command, AllianceCommand::Accept);
        let request = match command {
            AllianceCommand::Invite(guild_name) => {
                let Some(guild) = self.guild_info.as_ref() else {
                    self.push_chat_line(
                        "[Alliance] Join a guild before inviting another guild.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                };
                if guild.alliance_id <= 0 {
                    self.push_chat_line(
                        "[Alliance] Your guild is not in an alliance.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                }
                match openstory_protocol::alliance_invite_packet(&guild_name) {
                    Ok(packet) => Some((packet, format!("Invitation sent to {guild_name}."))),
                    Err(error) => {
                        self.push_chat_line(
                            format!("[Alliance] Could not build invitation: {error}"),
                            [255, 105, 105],
                        );
                        return;
                    }
                }
            }
            AllianceCommand::Expel(target) => {
                let Some(guild) = self.guild_info.as_ref() else {
                    self.push_chat_line(
                        "[Alliance] Join a guild before managing an alliance.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                };
                if guild.alliance_id <= 0 {
                    self.push_chat_line(
                        "[Alliance] Your guild is not in an alliance.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                }
                let guild_id = target.parse::<i32>().ok().or_else(|| {
                    self.alliance_info
                        .as_ref()
                        .filter(|info| info.id == guild.alliance_id)
                        .and_then(|info| {
                            info.guilds
                                .iter()
                                .find(|entry| entry.name.eq_ignore_ascii_case(&target))
                                .map(|entry| entry.id)
                        })
                });
                let Some(guild_id) = guild_id else {
                    self.push_chat_line(
                        format!("[Alliance] No alliance guild named {target} was found."),
                        [255, 222, 112],
                    );
                    return;
                };
                if guild_id == guild.id {
                    self.push_chat_line(
                        "[Alliance] Use /alliance leave to remove your own guild.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                }
                Some((
                    openstory_protocol::alliance_expel_guild_packet(guild_id, guild.alliance_id),
                    format!("Removal requested for guild {target}."),
                ))
            }
            AllianceCommand::Leave => {
                if self
                    .guild_info
                    .as_ref()
                    .is_none_or(|guild| guild.alliance_id <= 0)
                {
                    self.push_chat_line(
                        "[Alliance] Your guild is not in an alliance.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                }
                Some((
                    openstory_protocol::alliance_leave_packet(),
                    "Leave request sent.".to_owned(),
                ))
            }
            AllianceCommand::Notice(notice) => {
                if self
                    .guild_info
                    .as_ref()
                    .is_none_or(|guild| guild.alliance_id <= 0)
                {
                    self.push_chat_line(
                        "[Alliance] Your guild is not in an alliance.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                }
                match openstory_protocol::alliance_notice_packet(&notice) {
                    Ok(packet) => Some((packet, "Notice update requested.".to_owned())),
                    Err(error) => {
                        self.push_chat_line(
                            format!("[Alliance] Could not build notice: {error}"),
                            [255, 105, 105],
                        );
                        return;
                    }
                }
            }
            AllianceCommand::RankTitles(titles) => {
                if self
                    .guild_info
                    .as_ref()
                    .is_none_or(|guild| guild.alliance_id <= 0)
                {
                    self.push_chat_line(
                        "[Alliance] Your guild is not in an alliance.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                }
                match openstory_protocol::alliance_rank_titles_packet(&titles) {
                    Ok(packet) => Some((packet, "Rank-title update requested.".to_owned())),
                    Err(error) => {
                        self.push_chat_line(
                            format!("[Alliance] Could not build rank-title update: {error}"),
                            [255, 105, 105],
                        );
                        return;
                    }
                }
            }
            AllianceCommand::Accept | AllianceCommand::Deny => {
                let Some((alliance_id, inviter)) = self.pending_alliance_invite.as_ref() else {
                    self.push_chat_line(
                        "[Alliance] There is no pending invitation.".to_owned(),
                        [255, 222, 112],
                    );
                    return;
                };
                let packet = if accepted {
                    Ok(openstory_protocol::alliance_accept_invite_packet(
                        *alliance_id,
                    ))
                } else {
                    let Some(guild) = self.guild_info.as_ref() else {
                        self.push_chat_line(
                            "[Alliance] Your guild must be online to decline this invitation."
                                .to_owned(),
                            [255, 222, 112],
                        );
                        return;
                    };
                    openstory_protocol::deny_alliance_invite_packet(inviter, &guild.name)
                };
                match packet {
                    Ok(packet) => Some((
                        packet,
                        if accepted {
                            format!("Joining {inviter}'s alliance...")
                        } else {
                            "Invitation declined.".to_owned()
                        },
                    )),
                    Err(error) => {
                        self.push_chat_line(
                            format!("[Alliance] Could not build response: {error}"),
                            [255, 105, 105],
                        );
                        return;
                    }
                }
            }
            AllianceCommand::List => unreachable!(),
        };
        let Some((packet, success)) = request else {
            return;
        };
        if self.send_alliance_packet(packet) {
            if is_response {
                self.pending_alliance_invite = None;
            }
            self.push_chat_line(format!("[Alliance] {success}"), [190, 225, 255]);
            self.refresh_alliance_texture();
        }
    }

    pub(super) fn resolve_alliance_invite(&mut self, accepted: bool) {
        self.execute_alliance_command(if accepted {
            AllianceCommand::Accept
        } else {
            AllianceCommand::Deny
        });
    }

    pub(super) fn apply_guild_event(&mut self, event: openstory_protocol::GuildEvent) {
        use openstory_protocol::GuildEvent;
        match event {
            GuildEvent::Snapshot(info) => {
                let alliance_id = info.as_ref().map_or(0, |guild| guild.alliance_id);
                self.guild_info = info;
                self.guild_scroll = 0;
                if alliance_id == 0 {
                    self.alliance_info = None;
                } else if self
                    .alliance_info
                    .as_ref()
                    .is_none_or(|alliance| alliance.id != alliance_id)
                {
                    self.alliance_info = Some(openstory_protocol::AllianceInfo {
                        id: alliance_id,
                        name: format!("Alliance {alliance_id}"),
                        rank_titles: std::array::from_fn(|_| String::new()),
                        capacity: 0,
                        notice: String::new(),
                        guilds: Vec::new(),
                    });
                }
            }
            GuildEvent::Invite { guild_id, inviter } => {
                self.pending_guild_invite = Some((guild_id, inviter.clone()));
                self.show_guild_panel = true;
                self.show_party_panel = false;
                self.show_buddy_panel = false;
                self.show_alliance_panel = false;
                self.push_chat_line(
                    format!("[Guild] {inviter} invited you. Press Y to accept or N to decline."),
                    [190, 225, 255],
                );
            }
            GuildEvent::CreationInvite { leader, guild_name } => {
                self.pending_guild_creation_invite = Some((leader.clone(), guild_name.clone()));
                self.show_guild_panel = true;
                self.show_party_panel = false;
                self.show_buddy_panel = false;
                self.show_alliance_panel = false;
                self.push_chat_line(
                    format!("[Guild] {leader} asks you to help create {guild_name}. Press Y to agree or N to decline."),
                    [255, 222, 112],
                );
            }
            GuildEvent::MemberJoined { guild_id, member } => {
                if let Some(guild) = self
                    .guild_info
                    .as_mut()
                    .filter(|guild| guild.id == guild_id)
                {
                    guild
                        .members
                        .retain(|existing| existing.character_id != member.character_id);
                    let name = member.name.clone();
                    guild.members.push(member);
                    guild.members.sort_by_key(|entry| entry.rank);
                    self.push_chat_line(
                        format!("[Guild] {name} joined the guild."),
                        [190, 225, 255],
                    );
                }
            }
            GuildEvent::MemberLeft {
                guild_id,
                character_id,
                name,
                expelled,
            } => {
                if self
                    .active_character
                    .as_ref()
                    .is_some_and(|character| character.id == character_id)
                {
                    self.guild_info = None;
                    self.alliance_info = None;
                    self.show_alliance_panel = false;
                } else if let Some(guild) = self
                    .guild_info
                    .as_mut()
                    .filter(|guild| guild.id == guild_id)
                {
                    guild
                        .members
                        .retain(|member| member.character_id != character_id);
                }
                let verb = if expelled {
                    "was expelled from"
                } else {
                    "left"
                };
                self.push_chat_line(format!("[Guild] {name} {verb} the guild."), [190, 225, 255]);
            }
            GuildEvent::GuildDisbanded { .. } => {
                self.guild_info = None;
                self.alliance_info = None;
                self.show_guild_panel = false;
                self.show_alliance_panel = false;
                self.guild_scroll = 0;
                self.push_chat_line(
                    "[Guild] The guild has been disbanded.".to_owned(),
                    [255, 255, 255],
                );
            }
            GuildEvent::CapacityChanged { guild_id, capacity } => {
                if let Some(guild) = self
                    .guild_info
                    .as_mut()
                    .filter(|guild| guild.id == guild_id)
                {
                    guild.capacity = capacity;
                }
            }
            GuildEvent::MemberStatsChanged {
                guild_id,
                character_id,
                level,
                job,
            } => {
                if let Some(member) = self
                    .guild_info
                    .as_mut()
                    .filter(|guild| guild.id == guild_id)
                    .and_then(|guild| {
                        guild
                            .members
                            .iter_mut()
                            .find(|member| member.character_id == character_id)
                    })
                {
                    member.level = level;
                    member.job = job;
                }
            }
            GuildEvent::MemberOnlineChanged {
                guild_id,
                character_id,
                online,
            } => {
                if let Some(member) = self
                    .guild_info
                    .as_mut()
                    .filter(|guild| guild.id == guild_id)
                    .and_then(|guild| {
                        guild
                            .members
                            .iter_mut()
                            .find(|member| member.character_id == character_id)
                    })
                {
                    member.online = online;
                    let name = member.name.clone();
                    if self
                        .active_character
                        .as_ref()
                        .is_none_or(|character| character.id != character_id)
                    {
                        self.push_chat_line(
                            format!(
                                "[Guild] {name} has {}.",
                                if online { "logged in" } else { "logged out" }
                            ),
                            [255, 222, 112],
                        );
                    }
                }
            }
            GuildEvent::RankTitlesChanged { guild_id, titles } => {
                if let Some(guild) = self
                    .guild_info
                    .as_mut()
                    .filter(|guild| guild.id == guild_id)
                {
                    guild.rank_titles = titles;
                }
            }
            GuildEvent::MemberRankChanged {
                guild_id,
                character_id,
                rank,
            } => {
                if let Some(member) = self
                    .guild_info
                    .as_mut()
                    .filter(|guild| guild.id == guild_id)
                    .and_then(|guild| {
                        guild
                            .members
                            .iter_mut()
                            .find(|member| member.character_id == character_id)
                    })
                {
                    member.rank = rank;
                }
            }
            GuildEvent::EmblemChanged { guild_id, emblem } => {
                if let Some(guild) = self
                    .guild_info
                    .as_mut()
                    .filter(|guild| guild.id == guild_id)
                {
                    guild.emblem = emblem;
                }
            }
            GuildEvent::NoticeChanged { guild_id, notice } => {
                if let Some(guild) = self
                    .guild_info
                    .as_mut()
                    .filter(|guild| guild.id == guild_id)
                {
                    guild.notice = notice;
                }
            }
            GuildEvent::GuildPointsChanged { guild_id, points } => {
                if let Some(guild) = self
                    .guild_info
                    .as_mut()
                    .filter(|guild| guild.id == guild_id)
                {
                    guild.gp = points;
                }
            }
            GuildEvent::InvitationDenied { name } => self.push_chat_line(
                format!("[Guild] {name} declined the invitation."),
                [255, 222, 112],
            ),
            GuildEvent::InvitationFailed { name } => self.push_chat_line(
                format!("[Guild] {name} is already handling another invitation."),
                [255, 105, 105],
            ),
            GuildEvent::CreationDismissed => {
                self.pending_guild_creation_invite = None;
                self.push_chat_line(
                    "[Guild] The guild-creation request was cancelled.".to_owned(),
                    [255, 222, 112],
                );
            }
            GuildEvent::CreationFailed => {
                self.pending_guild_creation_invite = None;
                self.push_chat_line(
                    "[Guild] The server could not create that guild.".to_owned(),
                    [255, 105, 105],
                );
            }
        }
        let maximum = self
            .guild_info
            .as_ref()
            .map_or(0, |guild| guild.members.len().saturating_sub(12));
        self.guild_scroll = self.guild_scroll.min(maximum);
        if self.show_guild_panel {
            self.refresh_guild_texture();
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn resolve_guild_invite(&mut self, accepted: bool) {
        let Some((guild_id, inviter)) = self.pending_guild_invite.take() else {
            return;
        };
        let Some(character_id) = self.active_character.as_ref().map(|character| character.id)
        else {
            return;
        };
        let packet = if accepted {
            Ok(openstory_protocol::accept_guild_invite_packet(
                guild_id,
                character_id,
            ))
        } else {
            openstory_protocol::deny_guild_invite_packet(&inviter)
        };
        let Some(session) = self.session.as_ref() else {
            self.push_chat_line(
                "[Guild] Not connected to a game channel.".to_owned(),
                [255, 105, 105],
            );
            return;
        };
        let result = match packet {
            Ok(packet) => session.send_packet(packet),
            Err(error) => Err(error.to_string()),
        };
        match result {
            Ok(()) => self.push_chat_line(
                if accepted {
                    format!("[Guild] Joining {inviter}'s guild...")
                } else {
                    "[Guild] Invitation declined.".to_owned()
                },
                [190, 225, 255],
            ),
            Err(error) => {
                self.channel_connected = false;
                self.push_chat_line(
                    format!("[Guild] Could not respond to invitation: {error}"),
                    [255, 105, 105],
                );
            }
        }
        self.refresh_guild_texture();
    }

    pub(super) fn refresh_guild_bbs_texture(&mut self) {
        let Some(bbs) = self.guild_bbs.as_ref() else {
            return;
        };
        let pixels = render_guild_bbs_panel(&self.font, bbs, self.guild_info.as_ref());
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self.guild_bbs_texture.as_ref() {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(GUILD_BBS_PANEL_WIDTH, GUILD_BBS_PANEL_HEIGHT, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => self.guild_bbs_texture = Some(texture),
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not update guild BBS: {error}"),
        }
    }

    pub(super) fn open_guild_bbs(&mut self) {
        if self.guild_info.is_none() {
            self.push_chat_line(
                "[Guild] Join a guild to open its bulletin board.".to_owned(),
                [255, 222, 112],
            );
            return;
        }
        self.guild_bbs = Some(GuildBbsUi {
            page: 0,
            total_threads: 0,
            notice: None,
            threads: Vec::new(),
            thread: None,
            draft: None,
            reply_input: String::new(),
            reply_focused: false,
            confirm_delete: false,
            feedback: None,
        });
        self.show_guild_panel = false;
        self.show_alliance_panel = false;
        self.show_party_panel = false;
        self.show_buddy_panel = false;
        self.show_inventory = false;
        self.show_skill_book = false;
        self.show_quest_log = false;
        self.chat_open = false;
        self.request_guild_bbs_page(0);
        self.refresh_guild_bbs_texture();
    }

    pub(super) fn request_guild_bbs_page(&mut self, page: usize) {
        if self.guild_info.is_none() {
            return;
        }
        let maximum = self
            .guild_bbs
            .as_ref()
            .map_or(0, |bbs| bbs.total_threads.saturating_sub(1) / 10);
        let page = page.min(maximum);
        if let Some(bbs) = self.guild_bbs.as_mut() {
            bbs.page = page;
            bbs.thread = None;
            bbs.reply_input.clear();
            bbs.reply_focused = false;
            bbs.confirm_delete = false;
        }
        self.refresh_guild_bbs_texture();
        let Some(session) = self.session.as_ref() else {
            return;
        };
        if let Err(error) =
            session.send_packet(openstory_protocol::guild_bbs_list_packet(page as i32))
        {
            eprintln!("OpenStoryRS: could not request guild BBS page: {error}");
            self.channel_connected = false;
        }
    }

    pub(super) fn request_guild_bbs_thread(&mut self, thread_id: i32) {
        if let Some(bbs) = self.guild_bbs.as_mut() {
            bbs.reply_input.clear();
            bbs.reply_focused = false;
            bbs.confirm_delete = false;
            bbs.feedback = None;
        }
        let Some(session) = self.session.as_ref() else {
            return;
        };
        if let Err(error) =
            session.send_packet(openstory_protocol::guild_bbs_view_packet(thread_id))
        {
            eprintln!("OpenStoryRS: could not open guild BBS thread: {error}");
            self.channel_connected = false;
        }
    }

    pub(super) fn apply_guild_bbs_event(&mut self, event: openstory_protocol::GuildBbsEvent) {
        use openstory_protocol::GuildBbsEvent;
        if self.guild_bbs.is_none() {
            return;
        }
        let bbs = self.guild_bbs.get_or_insert(GuildBbsUi {
            page: 0,
            total_threads: 0,
            notice: None,
            threads: Vec::new(),
            thread: None,
            draft: None,
            reply_input: String::new(),
            reply_focused: false,
            confirm_delete: false,
            feedback: None,
        });
        match event {
            GuildBbsEvent::ThreadList {
                notice,
                total,
                threads,
            } => {
                bbs.notice = notice;
                bbs.total_threads = total.max(0) as usize;
                bbs.threads = threads;
            }
            GuildBbsEvent::ThreadView(thread) => bbs.thread = Some(thread),
        }
        self.refresh_guild_bbs_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn click_guild_bbs(&mut self, cursor: Point) {
        const PANEL_X: f32 = 100.0;
        const PANEL_Y: f32 = 100.0;
        let local_x = cursor.x - PANEL_X;
        let local_y = cursor.y - PANEL_Y;

        if self
            .guild_bbs
            .as_ref()
            .is_some_and(|bbs| bbs.draft.is_some())
        {
            if (510.0..600.0).contains(&local_x) && (422.0..458.0).contains(&local_y) {
                self.submit_guild_bbs_post();
            } else if (410.0..500.0).contains(&local_x) && (422.0..458.0).contains(&local_y) {
                if let Some(bbs) = self.guild_bbs.as_mut() {
                    bbs.draft = None;
                    bbs.feedback = None;
                }
            } else if (224.0..600.0).contains(&local_x) && (192.0..222.0).contains(&local_y) {
                if let Some(draft) = self.guild_bbs.as_mut().and_then(|bbs| bbs.draft.as_mut()) {
                    draft.focus = GuildBbsDraftFocus::Title;
                }
            } else if (224.0..600.0).contains(&local_x) && (240.0..408.0).contains(&local_y) {
                if let Some(draft) = self.guild_bbs.as_mut().and_then(|bbs| bbs.draft.as_mut()) {
                    draft.focus = GuildBbsDraftFocus::Content;
                }
            }
            return;
        }

        if self
            .guild_bbs
            .as_ref()
            .is_some_and(|bbs| bbs.confirm_delete)
        {
            if (420.0..500.0).contains(&local_x) && (300.0..334.0).contains(&local_y) {
                self.delete_guild_bbs_thread();
            } else if (320.0..400.0).contains(&local_x) && (300.0..334.0).contains(&local_y) {
                if let Some(bbs) = self.guild_bbs.as_mut() {
                    bbs.confirm_delete = false;
                }
            }
            return;
        }

        let Some(bbs) = self.guild_bbs.as_ref() else {
            return;
        };
        let page = bbs.page;
        let has_thread = bbs.thread.is_some();
        if (774.0..814.0).contains(&local_x) && (46.0..76.0).contains(&local_y) {
            self.guild_bbs = None;
            return;
        }
        if (604.0..658.0).contains(&local_x) && (46.0..76.0).contains(&local_y) {
            self.request_guild_bbs_page(page);
            return;
        }
        if (660.0..704.0).contains(&local_x) && (46.0..76.0).contains(&local_y) {
            self.request_guild_bbs_page(page.saturating_sub(1));
            return;
        }
        if (706.0..750.0).contains(&local_x) && (46.0..76.0).contains(&local_y) {
            self.request_guild_bbs_page(page.saturating_add(1));
            return;
        }

        if (14.0..90.0).contains(&local_x) && (462.0..498.0).contains(&local_y) {
            self.begin_guild_bbs_write(false);
            return;
        }
        if (98.0..184.0).contains(&local_x) && (462.0..498.0).contains(&local_y) {
            self.begin_guild_bbs_write(true);
            return;
        }
        if (192.0..278.0).contains(&local_x) && (462.0..498.0).contains(&local_y) && has_thread {
            if let Some(bbs) = self.guild_bbs.as_mut() {
                bbs.confirm_delete = true;
                bbs.reply_focused = false;
            }
            return;
        }
        if (744.0..812.0).contains(&local_x) && (450.0..498.0).contains(&local_y) && has_thread {
            self.send_guild_bbs_reply();
            return;
        }
        if (388.0..740.0).contains(&local_x) && (450.0..498.0).contains(&local_y) && has_thread {
            if let Some(bbs) = self.guild_bbs.as_mut() {
                bbs.reply_focused = true;
                bbs.feedback = None;
            }
            return;
        }
        let thread_id = if (14.0..362.0).contains(&local_x) && (108.0..438.0).contains(&local_y) {
            let row = ((local_y - 108.0) / 30.0) as usize;
            if bbs.notice.is_some() {
                if row == 0 {
                    bbs.notice.as_ref().map(|thread| thread.id)
                } else {
                    bbs.threads.get(row - 1).map(|thread| thread.id)
                }
            } else {
                bbs.threads.get(row).map(|thread| thread.id)
            }
        } else {
            None
        };
        if let Some(bbs) = self.guild_bbs.as_mut() {
            bbs.reply_focused = false;
        }
        if let Some(thread_id) = thread_id {
            self.request_guild_bbs_thread(thread_id);
        }
    }

    pub(super) fn begin_guild_bbs_write(&mut self, is_notice: bool) {
        if let Some(bbs) = self.guild_bbs.as_mut() {
            bbs.reply_focused = false;
            bbs.confirm_delete = false;
            bbs.feedback = None;
            bbs.draft = Some(GuildBbsDraft {
                title: String::new(),
                content: String::new(),
                is_notice,
                focus: GuildBbsDraftFocus::Title,
            });
        }
    }

    pub(super) fn submit_guild_bbs_post(&mut self) {
        let Some((title, content, is_notice)) = self
            .guild_bbs
            .as_ref()
            .and_then(|bbs| bbs.draft.as_ref())
            .map(|draft| (draft.title.clone(), draft.content.clone(), draft.is_notice))
        else {
            return;
        };
        if title.trim().is_empty() || content.trim().is_empty() {
            if let Some(bbs) = self.guild_bbs.as_mut() {
                bbs.feedback = Some("Enter both a subject and post content.".to_owned());
            }
            return;
        }
        if title.encode_utf16().count() > GUILD_BBS_TITLE_LIMIT
            || content.encode_utf16().count() > GUILD_BBS_CONTENT_LIMIT
        {
            if let Some(bbs) = self.guild_bbs.as_mut() {
                bbs.feedback = Some("Subject limit: 25 characters; post limit: 600.".to_owned());
            }
            return;
        }
        let packet = match openstory_protocol::guild_bbs_write_packet(is_notice, &title, &content) {
            Ok(packet) => packet,
            Err(error) => {
                if let Some(bbs) = self.guild_bbs.as_mut() {
                    bbs.feedback = Some(format!("Could not encode post: {error}"));
                }
                return;
            }
        };
        let Some(session) = self.session.as_ref() else {
            if let Some(bbs) = self.guild_bbs.as_mut() {
                bbs.feedback = Some("Not connected to a game channel.".to_owned());
            }
            return;
        };
        if let Err(error) = session.send_packet(packet) {
            if let Some(bbs) = self.guild_bbs.as_mut() {
                bbs.feedback = Some(format!("Could not send post: {error}"));
            }
            self.channel_connected = false;
            return;
        }
        if let Some(bbs) = self.guild_bbs.as_mut() {
            bbs.draft = None;
            bbs.feedback = Some("Post submitted.".to_owned());
        }
        self.request_guild_bbs_page(0);
    }

    pub(super) fn send_guild_bbs_reply(&mut self) {
        let Some((thread_id, content)) = self
            .guild_bbs
            .as_ref()
            .and_then(|bbs| Some((bbs.thread.as_ref()?.id, bbs.reply_input.trim().to_owned())))
        else {
            return;
        };
        if content.is_empty() {
            if let Some(bbs) = self.guild_bbs.as_mut() {
                bbs.feedback = Some("Enter a reply first.".to_owned());
            }
            return;
        }
        if content.encode_utf16().count() > GUILD_BBS_REPLY_LIMIT {
            if let Some(bbs) = self.guild_bbs.as_mut() {
                bbs.feedback = Some("Replies are limited to 25 characters.".to_owned());
            }
            return;
        }
        let packet = match openstory_protocol::guild_bbs_reply_packet(thread_id, &content) {
            Ok(packet) => packet,
            Err(error) => {
                if let Some(bbs) = self.guild_bbs.as_mut() {
                    bbs.feedback = Some(format!("Could not encode reply: {error}"));
                }
                return;
            }
        };
        let Some(session) = self.session.as_ref() else {
            if let Some(bbs) = self.guild_bbs.as_mut() {
                bbs.feedback = Some("Not connected to a game channel.".to_owned());
            }
            return;
        };
        if let Err(error) = session.send_packet(packet) {
            if let Some(bbs) = self.guild_bbs.as_mut() {
                bbs.feedback = Some(format!("Could not send reply: {error}"));
            }
            self.channel_connected = false;
            return;
        }
        if let Some(bbs) = self.guild_bbs.as_mut() {
            bbs.reply_input.clear();
            bbs.reply_focused = false;
            bbs.feedback = Some("Reply submitted.".to_owned());
        }
    }

    pub(super) fn delete_guild_bbs_thread(&mut self) {
        let Some(thread_id) = self
            .guild_bbs
            .as_ref()
            .and_then(|bbs| bbs.thread.as_ref().map(|thread| thread.id))
        else {
            return;
        };
        let Some(session) = self.session.as_ref() else {
            if let Some(bbs) = self.guild_bbs.as_mut() {
                bbs.confirm_delete = false;
                bbs.feedback = Some("Not connected to a game channel.".to_owned());
            }
            return;
        };
        if let Err(error) =
            session.send_packet(openstory_protocol::guild_bbs_delete_packet(thread_id))
        {
            if let Some(bbs) = self.guild_bbs.as_mut() {
                bbs.confirm_delete = false;
                bbs.feedback = Some(format!("Could not delete post: {error}"));
            }
            self.channel_connected = false;
            return;
        }
        if let Some(bbs) = self.guild_bbs.as_mut() {
            bbs.confirm_delete = false;
            bbs.thread = None;
            bbs.feedback = Some("Delete request sent.".to_owned());
        }
        self.request_guild_bbs_page(0);
    }
}
