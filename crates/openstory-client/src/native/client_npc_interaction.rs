use super::*;

pub(super) const UNIMPLEMENTED_NPC_TEXT: &str = "Nothing to say...";

impl ClientApp {
    const NPC_CHAT_HORIZONTAL_RANGE: f64 = 200.0;
    const NPC_CHAT_VERTICAL_RANGE: f64 = 400.0;

    pub(super) fn request_npc_talk(&mut self, object_id: i32) {
        let now = Instant::now();
        if self
            .last_npc_talk_at
            .is_some_and(|last| now.saturating_duration_since(last) < Duration::from_millis(600))
            || self.session.is_none()
            || self.death_notice
        {
            return;
        }
        let Some(npc_id) = self
            .map_scene
            .as_ref()
            .and_then(|scene| scene.npcs.get(&object_id).map(|npc| npc.npc_id))
        else {
            return;
        };
        self.last_npc_talk_at = Some(now);
        let trial_npc_dialogue = self.map_scene.as_ref().and_then(|scene| {
            (scene.map_id == 1_020_000)
                .then(|| job_trial_npc_dialogue(npc_id))
                .flatten()
        });
        if let Some(text) = trial_npc_dialogue {
            self.show_local_quest_conversation(
                npc_id,
                LocalQuestConversation {
                    pages: vec![text.to_owned()],
                    page: 0,
                    request: None,
                    accepted_pages: Vec::new(),
                    questions: HashMap::new(),
                    feedback: None,
                },
            );
            return;
        }
        let choices = npc_quest_choices(
            npc_id,
            &self.quest_log,
            &self.quest_archive,
            &self.quest_start_npc_index,
            self.active_character.as_ref(),
            self.inventory.as_ref(),
        );
        if choices.is_empty() {
            self.send_npc_talk_request(object_id);
            return;
        }
        let (text, actions) = npc_quest_menu(&self.quest_archive, object_id, choices);
        self.show_npc_dialogue(openstory_protocol::NpcDialogue {
            npc_id,
            message_type: 4,
            speaker: 0,
            text,
            number_bounds: None,
            navigation: None,
        });
        if let Some(ui) = self.npc_dialogue.as_mut() {
            ui.local_menu = actions;
        }
    }

    pub(super) fn handle_local_npc_menu(&mut self, action: NpcDialogueAction) -> bool {
        let Some(ui) = self
            .npc_dialogue
            .as_ref()
            .filter(|ui| !ui.local_menu.is_empty())
        else {
            return false;
        };
        let npc_id = ui.state.npc_id;
        let selected = match action {
            NpcDialogueAction::Select(index) => usize::try_from(index)
                .ok()
                .and_then(|index| ui.local_menu.get(index))
                .copied(),
            _ => None,
        };
        self.npc_dialogue = None;
        self.npc_dialogue_scroll_dragging = None;
        match selected {
            Some(NpcMenuAction::Quest(choice)) => {
                // Re-evaluate after the menu: inventory or quest state can change while open.
                let current = npc_quest_choices(
                    npc_id,
                    &self.quest_log,
                    &self.quest_archive,
                    &self.quest_start_npc_index,
                    self.active_character.as_ref(),
                    self.inventory.as_ref(),
                );
                if let Some(choice) = current.into_iter().find(|current| {
                    current.quest_id == choice.quest_id && current.side == choice.side
                }) {
                    self.open_npc_quest_conversation(npc_id, choice, None);
                }
            }
            Some(NpcMenuAction::Talk(object_id)) => self.send_npc_talk_request(object_id),
            None => {}
        }
        true
    }

    fn send_npc_talk_request(&mut self, object_id: i32) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        match session.send_packet(openstory_protocol::talk_to_npc(object_id)) {
            Ok(()) => self.pending_npc_talk = Some((object_id, Instant::now())),
            Err(error) => {
                eprintln!("OpenStory: could not request NPC dialogue: {error}");
                self.channel_connected = false;
            }
        }
    }

    pub(super) fn expire_pending_npc_talk(&mut self, now: Instant) {
        let Some((object_id, sent)) = self.pending_npc_talk else {
            return;
        };
        if now.saturating_duration_since(sent) < Duration::from_millis(1500) {
            return;
        }
        self.pending_npc_talk = None;
        if self.npc_dialogue.is_some() || self.storage.is_some() || self.npc_shop.is_some() {
            return;
        }
        let Some(npc_id) = self
            .map_scene
            .as_ref()
            .and_then(|scene| scene.npcs.get(&object_id).map(|npc| npc.npc_id))
        else {
            return;
        };
        self.show_unimplemented_npc_dialogue(npc_id);
    }

    fn show_unimplemented_npc_dialogue(&mut self, npc_id: i32) {
        self.show_local_quest_conversation(
            npc_id,
            LocalQuestConversation {
                pages: vec![UNIMPLEMENTED_NPC_TEXT.to_owned()],
                page: 0,
                request: None,
                accepted_pages: Vec::new(),
                questions: HashMap::new(),
                feedback: None,
            },
        );
    }

    pub(super) fn open_npc_quest_conversation(
        &mut self,
        npc_id: i32,
        choice: NpcQuestChoice,
        reward_selection: Option<i16>,
    ) {
        let NpcQuestChoice {
            quest_id,
            side,
            ready,
        } = choice;
        let scripted = quest_requires_script(
            &self.quest_archive,
            quest_id,
            side,
            if side == 0 {
                "startscript"
            } else {
                "endscript"
            },
        );
        if scripted && ready {
            let packet = if side == 0 {
                openstory_protocol::start_quest_packet(quest_id, npc_id, true)
            } else {
                openstory_protocol::complete_quest_packet(quest_id, npc_id, reward_selection, true)
            };
            if let Some(session) = self.session.as_ref()
                && let Err(error) = session.send_packet(packet)
            {
                eprintln!("OpenStory: could not request quest dialogue: {error}");
                self.channel_connected = false;
            }
            return;
        }
        let path = format!("Say.img/{quest_id}/{side}");
        let pages = if ready {
            quest_dialogue_pages(&self.quest_archive, &path)
        } else if side == 0 {
            quest_dialogue_pages(&self.quest_archive, &format!("{path}/yes"))
        } else {
            ["quest", "mob", "item", "npc", "end"]
                .into_iter()
                .map(|reason| {
                    quest_dialogue_pages(&self.quest_archive, &format!("{path}/stop/{reason}"))
                })
                .find(|pages| !pages.is_empty())
                .unwrap_or_default()
        };
        let title = quest_info_text(&self.quest_archive, quest_id, "name")
            .unwrap_or_else(|| format!("Quest {quest_id}"));
        let pages = if pages.is_empty() {
            vec![if ready {
                format!("Complete #b{title}#k?")
            } else {
                quest_info_text(&self.quest_archive, quest_id, "1")
                    .unwrap_or_else(|| format!("#b{title}#k is still in progress."))
            }]
        } else {
            pages
        };
        let questions = if ready {
            quest_dialogue_questions(&self.quest_archive, &path, &pages)
        } else {
            HashMap::new()
        };
        let request = ready.then(|| {
            if side == 0 {
                openstory_protocol::start_quest_packet(quest_id, npc_id, false)
            } else {
                openstory_protocol::complete_quest_packet(quest_id, npc_id, reward_selection, false)
            }
        });
        let accepted_pages = if ready {
            quest_dialogue_pages(&self.quest_archive, &format!("{path}/yes"))
        } else {
            Vec::new()
        };
        self.show_local_quest_conversation(
            npc_id,
            LocalQuestConversation {
                pages,
                page: 0,
                request,
                accepted_pages,
                questions,
                feedback: None,
            },
        );
    }

    pub(super) fn show_local_quest_conversation(
        &mut self,
        npc_id: i32,
        conversation: LocalQuestConversation,
    ) {
        let text = conversation
            .feedback
            .clone()
            .unwrap_or_else(|| conversation.pages[conversation.page].clone());
        self.show_npc_dialogue(openstory_protocol::NpcDialogue {
            npc_id,
            message_type: conversation.message_type(),
            speaker: 0,
            text,
            number_bounds: None,
            navigation: None,
        });
        if let Some(ui) = self.npc_dialogue.as_mut() {
            ui.local_quest = Some(conversation);
        }
    }

    pub(super) fn is_npc_chat_key(&self, key: PhysicalKey) -> bool {
        gameplay_key_binding(self.server_keymap.as_deref(), key)
            .is_some_and(|binding| binding.kind == 5 && binding.action == 54)
    }

    pub(super) fn request_nearest_npc_talk(&mut self) {
        let object_id = self.map_scene.as_ref().and_then(|scene| {
            scene
                .npcs
                .iter()
                .filter_map(|(id, npc)| {
                    let dx = (npc.x - scene.player.x).abs();
                    let dy = (npc.y - scene.player.y).abs();
                    (dx <= Self::NPC_CHAT_HORIZONTAL_RANGE && dy <= Self::NPC_CHAT_VERTICAL_RANGE)
                        .then_some((*id, dx * dx + dy * dy))
                })
                .min_by(|left, right| {
                    left.1
                        .total_cmp(&right.1)
                        .then_with(|| left.0.cmp(&right.0))
                })
                .map(|(id, _)| id)
        });
        if let Some(object_id) = object_id {
            self.request_npc_talk(object_id);
        }
    }
}

fn job_trial_npc_dialogue(npc_id: i32) -> Option<&'static str> {
    match npc_id {
        10_200 => Some("Do you wish to learn the way of the Bowman?\n\nCome see me in Henesys."),
        10_201 => {
            Some("Those who wish to become a Magician, follow me.\n\nCome see me in Ellinia.")
        }
        10_202 => Some("Do you wish to know more about Warriors?\n\nCome see me in Perion."),
        10_203 => Some("Hey you! So you want to become a Thief?\n\nCome to Kerning City."),
        10_204 => Some("Not everyone can become a Pirate!\n\nCome see me in Nautilus Harbor."),
        _ => None,
    }
}

fn npc_quest_choices(
    npc_id: i32,
    log: &openstory_protocol::QuestLog,
    archive: &NxArchive,
    index: &HashMap<i32, Vec<i16>>,
    character: Option<&openstory_protocol::CharacterInfo>,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
) -> Vec<NpcQuestChoice> {
    let mut choices = Vec::new();
    for quest in &log.started {
        let start = archive
            .root()
            .resolve(&format!("Check.img/{}/0", quest.id))
            .ok()
            .flatten();
        let end = archive
            .root()
            .resolve(&format!("Check.img/{}/1", quest.id))
            .ok()
            .flatten();
        let start_npc = start
            .as_ref()
            .and_then(|node| quest_node_integer(node, "npc"));
        // v83 quests such as Pio's recycling omit the end NPC and return to the starter.
        let end_npc = end
            .as_ref()
            .and_then(|node| quest_node_integer(node, "npc"))
            .or(start_npc);
        if end_npc == Some(npc_id) || start_npc == Some(npc_id) {
            let side = if end_npc == Some(npc_id) { 1 } else { 0 };
            choices.push(NpcQuestChoice {
                quest_id: quest.id,
                side,
                ready: side == 1 && quest_is_completable(quest, archive, inventory, log),
            });
        }
    }
    choices.extend(
        available_quest_entries(log, archive, index, character, inventory)
            .into_iter()
            .filter(|(id, npc)| *npc == npc_id && quest_start_items_met(archive, *id, inventory))
            .map(|(quest_id, _)| NpcQuestChoice {
                quest_id,
                side: 0,
                ready: true,
            }),
    );
    choices.sort_by_key(|choice| {
        (
            if choice.side == 1 && choice.ready {
                0
            } else if choice.ready {
                1
            } else {
                2
            },
            choice.quest_id,
        )
    });
    choices
}

fn quest_dialogue_questions(
    archive: &NxArchive,
    path: &str,
    pages: &[String],
) -> HashMap<usize, LocalQuestQuestion> {
    pages
        .iter()
        .enumerate()
        .filter_map(|(page, text)| {
            let selections = parse_npc_selections(text).1;
            if selections.is_empty() {
                return None;
            }
            let stop = archive
                .root()
                .resolve(&format!("{path}/stop/{page}"))
                .ok()
                .flatten();
            // NX stores one-based answer positions; #L option IDs are zero-based.
            let answer = stop
                .as_ref()
                .and_then(|node| quest_node_integer(node, "answer"))
                .map(|answer| answer - 1)
                .unwrap_or(selections[0].0);
            let incorrect = selections
                .into_iter()
                .filter_map(|(id, _)| {
                    let node = stop.as_ref()?.child(&id.to_string()).ok().flatten()?;
                    match node.value().ok()? {
                        NodeValue::String(text) => Some((id, text)),
                        _ => None,
                    }
                })
                .collect();
            Some((page, LocalQuestQuestion { answer, incorrect }))
        })
        .collect()
}

#[cfg(test)]
fn npc_default_dialogue_pages(npcs: &NxArchive, strings: &NxArchive, npc_id: i32) -> Vec<String> {
    let path = format!("{:07}.img/info/speak", npc_id.max(0));
    let keys = npcs
        .root()
        .resolve(&path)
        .ok()
        .flatten()
        .and_then(|node| node.children().ok())
        .unwrap_or_default();
    let mut pages = keys
        .into_iter()
        .filter_map(|node| {
            let index = node.name().ok()?.parse::<usize>().ok()?;
            let NodeValue::String(key) = node.value().ok()? else {
                return None;
            };
            let text = nx_string_value(strings, &format!("Npc.img/{npc_id}/{key}"))?;
            Some((index, text))
        })
        .collect::<Vec<_>>();
    pages.sort_by_key(|(index, _)| *index);
    pages.into_iter().map(|(_, text)| text).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conversation(questions: HashMap<usize, LocalQuestQuestion>) -> LocalQuestConversation {
        LocalQuestConversation {
            pages: vec!["Question".into(), "Correct!".into()],
            page: 0,
            request: Some(vec![107, 0, 2]),
            accepted_pages: Vec::new(),
            questions,
            feedback: None,
        }
    }

    #[test]
    fn dialogue_input_filters_and_limits_text_and_numbers() {
        let mut text = String::new();
        append_npc_dialogue_input(&mut text, 2, &format!("{}\n", "é".repeat(60)));
        assert_eq!(text.chars().count(), 50);
        assert!(!text.contains('\n'));
        let mut number = String::new();
        append_npc_dialogue_input(&mut number, 3, "-1x-234567890123456789");
        assert_eq!(number, "-123456789012345");
        append_npc_dialogue_input(&mut number, 4, "9");
        assert_eq!(number.len(), 16);
    }

    #[test]
    fn dialogue_number_clamps_large_and_invalid_input() {
        let bounds = Some((5, -20, 100));
        assert_eq!(npc_dialogue_number("9999999999999999", bounds), 100);
        assert_eq!(npc_dialogue_number("-999999999999999", bounds), -20);
        assert_eq!(npc_dialogue_number("-", bounds), 5);
    }

    #[test]
    fn dialogue_close_responses_follow_message_type() {
        for message_type in [0, 1, 5, 6] {
            assert_eq!(npc_dialogue_cancel_response(message_type), -1);
        }
        for message_type in [2, 3, 4, 12] {
            assert_eq!(npc_dialogue_cancel_response(message_type), 0);
        }
    }

    #[test]
    fn quiz_wrong_answers_retry_without_granting_quest_progress() {
        let mut quiz = conversation(HashMap::from([(
            0,
            LocalQuestQuestion {
                answer: 1,
                incorrect: HashMap::from([(0, "Try again.".into())]),
            },
        )]));
        assert_eq!(quiz.message_type(), 4);
        assert_eq!(quiz.respond(1), LocalQuestStep::Show(None));
        assert_eq!(quiz.page, 0); // A quiz must be answered through a selection.
        assert_eq!(quiz.select(0), LocalQuestStep::Show(None));
        assert_eq!(quiz.feedback.as_deref(), Some("Try again."));
        assert_eq!(quiz.respond(1), LocalQuestStep::Show(None));
        assert_eq!(quiz.message_type(), 4);
        assert_eq!(quiz.select(1), LocalQuestStep::Show(None));
        assert_eq!(quiz.page, 1);
        assert_eq!(
            quiz.respond(1),
            LocalQuestStep::Close(Some(vec![107, 0, 2]))
        );
    }

    #[test]
    fn shipped_unterminated_quiz_options_still_parse() {
        let (_, choices) = parse_npc_selections("Choose: #L0#Warrior#l#L1#Magician#L2#Thief");
        assert_eq!(
            choices,
            vec![
                (0, "Warrior".into()),
                (1, "Magician".into()),
                (2, "Thief".into())
            ]
        );
    }

    #[test]
    #[ignore = "requires existing local NX assets; audits Maple Island quest conversations"]
    fn maple_island_quest_chains_and_quizzes() {
        use openstory_protocol::{CompletedQuest, QuestEntry, QuestLog};
        let directory = AssetDirectory::resolve_and_validate(None).unwrap();
        let quests = NxArchive::open(directory.root().join("Quest.nx")).unwrap();
        let npcs = NxArchive::open(directory.root().join("Npc.nx")).unwrap();
        let strings = NxArchive::open(directory.root().join("String.nx")).unwrap();
        let index = build_quest_start_npc_index(&quests);
        let mut character = ui_preview::preview_character();
        character.stats.level = 5;
        let empty = QuestLog::default();
        let choices = npc_quest_choices(2101, &empty, &quests, &index, Some(&character), None);
        assert!(
            choices
                .iter()
                .any(|choice| choice.quest_id == 1031 && choice.ready)
        );
        assert!(!choices.iter().any(|choice| choice.quest_id == 1000));
        let mut log = QuestLog {
            started: vec![QuestEntry {
                id: 1008,
                data: String::new(),
            }],
            completed: vec![],
        };
        let pio = npc_quest_choices(10000, &log, &quests, &index, Some(&character), None);
        assert!(
            pio.iter()
                .any(|choice| choice.quest_id == 1008 && choice.side == 1 && !choice.ready)
        );
        log.started = vec![QuestEntry {
            id: 1040,
            data: String::new(),
        }];
        assert!(!quest_is_completable(&log.started[0], &quests, None, &log));
        log.completed = (1041..=1044)
            .map(|id| CompletedQuest { id, timestamp: 0 })
            .collect();
        assert!(quest_is_completable(&log.started[0], &quests, None, &log));
        for (id, answers) in [
            (1009, vec![0]),
            (1010, vec![0]),
            (1011, vec![0]),
            (1012, vec![3]),
            (1013, vec![1]),
            (1014, vec![2]),
            (1015, vec![0]),
            (1036, vec![1, 1, 3]),
        ] {
            let path = format!("Say.img/{id}/1");
            let pages = quest_dialogue_pages(&quests, &path);
            let questions = quest_dialogue_questions(&quests, &path, &pages);
            assert_eq!(questions.len(), answers.len(), "quest {id}");
            for (page, expected) in answers.into_iter().enumerate() {
                let question = &questions[&page];
                assert_eq!(question.answer, expected, "quest {id}, page {page}");
                assert!(
                    parse_npc_selections(&pages[page])
                        .1
                        .iter()
                        .any(|(id, _)| *id == expected)
                );
            }
        }
        // Unscripted NPCs retain their authored greetings after their quests are finished.
        for id in [
            2000, 2001, 2002, 2004, 2005, 2100, 2101, 2102, 2103, 10000, 12000, 12100, 20001,
            20002, 20100, 9010000,
        ] {
            assert!(
                !npc_default_dialogue_pages(&npcs, &strings, id).is_empty(),
                "NPC {id}"
            );
        }
    }

    #[test]
    #[ignore = "requires existing local NX assets; checks Dalair title quest turn-ins"]
    fn dalair_title_quests_allow_valid_turn_ins() {
        use openstory_protocol::{QuestEntry, QuestLog};

        let directory = AssetDirectory::resolve_and_validate(None).unwrap();
        let quests = NxArchive::open(directory.root().join("Quest.nx")).unwrap();
        let empty_index = HashMap::new();
        for (quest_id, npc_id, scripted) in [
            (29900, 9000040, true),  // Beginner Adventurer uses an authored end script.
            (29922, 9000066, false), // Veteran Hunter has no end-side requirements.
            (29923, 9000066, false), // Legendary Hunter has no end-side requirements.
        ] {
            let log = QuestLog {
                started: vec![QuestEntry {
                    id: quest_id,
                    data: String::new(),
                }],
                completed: Vec::new(),
            };
            let choices = npc_quest_choices(npc_id, &log, &quests, &empty_index, None, None);
            let choice = choices
                .iter()
                .find(|choice| choice.quest_id == quest_id && choice.side == 1)
                .unwrap_or_else(|| panic!("missing Dalair turn-in choice for quest {quest_id}"));
            assert!(choice.ready, "quest {quest_id} should be ready to turn in");
            assert_eq!(
                quest_requires_script(&quests, quest_id, 1, "endscript"),
                scripted,
                "unexpected completion path for quest {quest_id}"
            );
        }

        for (quest_id, progress_quest_id, below, met) in [
            (29931, 7662, "9999", "10000"),
            (29932, 7760, "49999", "50000"),
        ] {
            let make_log = |progress: &str| QuestLog {
                started: vec![
                    QuestEntry {
                        id: quest_id,
                        data: String::new(),
                    },
                    QuestEntry {
                        id: progress_quest_id,
                        data: progress.to_owned(),
                    },
                ],
                completed: Vec::new(),
            };
            let below_log = make_log(below);
            assert!(!quest_is_completable(
                &below_log.started[0],
                &quests,
                None,
                &below_log
            ));
            let below_choice =
                npc_quest_choices(9000066, &below_log, &quests, &empty_index, None, None)
                    .into_iter()
                    .find(|choice| choice.quest_id == quest_id && choice.side == 1)
                    .unwrap();
            assert!(
                !below_choice.ready,
                "quest {quest_id} is below its title target"
            );

            let met_log = make_log(met);
            assert!(quest_is_completable(
                &met_log.started[0],
                &quests,
                None,
                &met_log
            ));
            let met_choice =
                npc_quest_choices(9000066, &met_log, &quests, &empty_index, None, None)
                    .into_iter()
                    .find(|choice| choice.quest_id == quest_id && choice.side == 1)
                    .unwrap();
            assert!(
                met_choice.ready,
                "quest {quest_id} has reached its title target"
            );
        }
    }
}

pub(super) fn npc_quest_menu(
    archive: &NxArchive,
    object_id: i32,
    choices: Vec<NpcQuestChoice>,
) -> (String, Vec<NpcMenuAction>) {
    let mut actions = Vec::new();
    let mut text = String::new();
    for choice in choices {
        let name = quest_info_text(archive, choice.quest_id, "name")
            .unwrap_or_else(|| format!("Quest {}", choice.quest_id));
        let status = if !choice.ready {
            "In progress"
        } else if choice.side == 0 {
            "Available"
        } else {
            "Complete"
        };
        text.push_str(&format!("#L{}#{name} ({status})#l\r\n", actions.len()));
        actions.push(NpcMenuAction::Quest(choice));
    }
    text.push_str(&format!("#L{}#Talk#l", actions.len()));
    actions.push(NpcMenuAction::Talk(object_id));
    (text, actions)
}

#[cfg(test)]
mod island_coverage_tests {
    use super::*;

    #[test]
    #[ignore = "requires existing local NX assets; enumerates all Maple Island NPCs"]
    fn every_island_npc_has_authored_dialogue_or_a_server_service() {
        let directory = AssetDirectory::resolve_and_validate(None).unwrap();
        let maps = NxArchive::open(directory.root().join("Map.nx")).unwrap();
        let npcs = NxArchive::open(directory.root().join("Npc.nx")).unwrap();
        let strings = NxArchive::open(directory.root().join("String.nx")).unwrap();
        let quests = NxArchive::open(directory.root().join("Quest.nx")).unwrap();
        let mut ids = HashSet::new();
        for map in maps
            .root()
            .resolve("Map/Map0")
            .unwrap()
            .unwrap()
            .children()
            .unwrap()
        {
            let name = map.name().unwrap();
            let Some(id) = name
                .strip_suffix(".img")
                .and_then(|id| id.parse::<i32>().ok())
            else {
                continue;
            };
            if id >= 3_000_000 {
                continue;
            }
            let Some(life) = map.child("life").unwrap() else {
                continue;
            };
            for npc in life.children().unwrap() {
                if string_property(&npc, "type").as_deref() != Some("n") {
                    continue;
                }
                if let Some(id) = string_property(&npc, "id").and_then(|id| id.parse::<i32>().ok())
                {
                    ids.insert(id);
                }
            }
        }
        assert_eq!(ids.len(), 32);
        // The server source/seed audit identifies these script and shop services.
        let services = [
            2100, 2101, 9101001, 2007, 2003, 9000000, 22000, 12101, 10200, 10201, 10202, 10203,
            10204, 11000, 11100, 21000,
        ];
        let mut rows = vec!["NPC ID\tName\tInteraction coverage".to_owned()];
        let mut ids = ids.into_iter().collect::<Vec<_>>();
        ids.sort_unstable();
        for id in ids {
            let pages = npc_default_dialogue_pages(&npcs, &strings, id);
            let service = services.contains(&id);
            assert!(
                service || !pages.is_empty(),
                "NPC {id} has neither service nor authored greeting"
            );
            let name = nx_string_value(&strings, &format!("Npc.img/{id}/name")).unwrap();
            rows.push(format!(
                "{id}\t{name}\t{}",
                if service {
                    "server script/shop"
                } else {
                    "NX quest/greeting"
                }
            ));
        }
        // Every active island quest side can build pages or an authored journal fallback.
        for id in 1000..=1046 {
            for side in 0..=1 {
                let check_path = format!("Check.img/{id}/{side}");
                if quests.root().resolve(&check_path).ok().flatten().is_none() {
                    continue;
                }
                let script = if side == 0 {
                    "startscript"
                } else {
                    "endscript"
                };
                if quest_requires_script(&quests, id, side, script) {
                    continue;
                }
                let pages = quest_dialogue_pages(&quests, &format!("Say.img/{id}/{side}"));
                assert!(
                    !pages.is_empty() || quest_info_text(&quests, id, "name").is_some(),
                    "quest {id}, side {side}"
                );
            }
        }
        let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target/npc-audit");
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(output.join("maple-island-coverage.tsv"), rows.join("\n")).unwrap();
    }
}
