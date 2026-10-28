fn quest_ids_for_tab(
    log: &openstory_protocol::QuestLog,
    available: &[(i16, i32)],
    tab: QuestLogTab,
) -> Vec<i16> {
    match tab {
        QuestLogTab::Active => log.started.iter().map(|quest| quest.id).collect(),
        QuestLogTab::Available => available.iter().map(|(id, _)| *id).collect(),
        QuestLogTab::Completed => log.completed.iter().map(|quest| quest.id).collect(),
    }
}

fn quest_info_text(archive: &NxArchive, quest_id: i16, field: &str) -> Option<String> {
    let node = archive
        .root()
        .resolve(&format!("QuestInfo.img/{quest_id}/{field}"))
        .ok()??;
    match node.value().ok()? {
        NodeValue::String(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

fn quest_node_integer(node: &openstory_assets::NxNode, field: &str) -> Option<i32> {
    let node = node.child(field).ok()??;
    match node.value().ok()? {
        NodeValue::Integer(value) => i32::try_from(value).ok(),
        _ => None,
    }
}

fn quest_requires_script(archive: &NxArchive, quest_id: i16, side: u8, field: &str) -> bool {
    archive
        .root()
        .resolve(&format!("Check.img/{quest_id}/{side}/{field}"))
        .ok()
        .flatten()
        .and_then(|node| match node.value().ok()? {
            NodeValue::String(script) => Some(!script.trim().is_empty()),
            _ => None,
        })
        .unwrap_or(false)
}

#[derive(Clone, Debug)]
struct QuestRewardChoice {
    selection: i16,
    count: i32,
    name: String,
}

fn quest_reward_job_categories(encoded: i32) -> Vec<i32> {
    let masks: &[(i32, &[i32])] = &[
        (0x0000_0001, &[0]),
        (0x0000_0002, &[100]),
        (0x0000_0004, &[200]),
        (0x0000_0008, &[300]),
        (0x0000_0010, &[400]),
        (0x0000_0020, &[500]),
        (0x0000_0400, &[1000]),
        (0x0000_0800, &[1100]),
        (0x0000_1000, &[1200]),
        (0x0000_2000, &[1300]),
        (0x0000_4000, &[1400]),
        (0x0000_8000, &[1500]),
        (0x0002_0000, &[2001, 2200]),
        (0x0010_0000, &[2000, 2001]),
        (0x0020_0000, &[2100]),
        (0x0040_0000, &[2001, 2200]),
        (0x4000_0000, &[3000, 3200, 3300, 3500]),
    ];
    masks
        .iter()
        .filter(|(mask, _)| encoded & mask != 0)
        .flat_map(|(_, categories)| categories.iter().copied())
        .collect()
}

fn quest_reward_choices(
    quest_id: i16,
    archive: &NxArchive,
    string_archive: &NxArchive,
    character: &openstory_protocol::CharacterInfo,
    item_name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<QuestRewardChoice> {
    // Action 5 starts the server-side end script; its choices are sent as
    // NPC_TALK_MORE and cannot be supplied in the quest-action packet.
    if quest_requires_script(archive, quest_id, 1, "endscript") {
        return Vec::new();
    }
    let reward_node = archive
        .root()
        .resolve(&format!("Act.img/{quest_id}/1"))
        .ok()
        .flatten()
        .or_else(|| {
            archive
                .root()
                .resolve(&format!("Act.img/{quest_id}/0"))
                .ok()
                .flatten()
        });
    let Some(item_nodes) = reward_node.and_then(|node| node.child("item").ok().flatten()) else {
        return Vec::new();
    };
    let mut items = item_nodes.children().unwrap_or_default();
    items.sort_by_key(|item| {
        item.name()
            .ok()
            .and_then(|name| name.parse::<i32>().ok())
            .unwrap_or(i32::MAX)
    });

    let player_job = i32::from(character.stats.job);
    let player_gender = i32::from(character.stats.female);
    let mut selection = 0_i16;
    let mut choices = Vec::new();
    for item in items {
        if quest_node_integer(&item, "prop") != Some(-1) {
            continue;
        }
        let gender = quest_node_integer(&item, "gender").unwrap_or(2);
        if gender != 2 && gender != player_gender {
            continue;
        }
        let job = quest_node_integer(&item, "job").unwrap_or(-1);
        if job > 0
            && !quest_reward_job_categories(job)
                .iter()
                .any(|category| category / 100 == player_job / 100)
        {
            continue;
        }
        let current_selection = selection;
        selection = selection.saturating_add(1);
        let item_id = quest_node_integer(&item, "id").unwrap_or(0);
        let count = quest_node_integer(&item, "count").unwrap_or(1).max(1);
        choices.push(QuestRewardChoice {
            selection: current_selection,
            count,
            name: if item_id > 0 {
                item_name(item_id, string_archive, item_name_cache)
            } else {
                "Unknown reward".to_owned()
            },
        });
    }
    choices
}

fn quest_requirement_pairs(
    archive: &NxArchive,
    quest_id: i16,
    side: u8,
    kind: &str,
) -> Vec<(i32, u32)> {
    let path = format!("Check.img/{quest_id}/{side}/{kind}");
    let Ok(Some(requirements)) = archive.root().resolve(&path) else {
        return Vec::new();
    };
    requirements
        .children()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|requirement| {
            let id = quest_node_integer(&requirement, "id")?;
            // Cosmic's Quest parser defaults an omitted item count to zero;
            // for item requirements that means the player must not carry it.
            let count = quest_node_integer(&requirement, "count").unwrap_or(0);
            (id > 0 && count >= 0).then(|| (id, count as u32))
        })
        .collect()
}

fn quest_progress_counter(data: &str, index: usize) -> u32 {
    let start = index.saturating_mul(3);
    let Some(counter) = data.as_bytes().get(start..start.saturating_add(3)) else {
        return 0;
    };
    if !counter.iter().all(u8::is_ascii_digit) {
        return 0;
    }
    std::str::from_utf8(counter)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn build_quest_start_npc_index(archive: &NxArchive) -> HashMap<i32, Vec<i16>> {
    let Ok(Some(quest_info)) = archive.root().resolve("QuestInfo.img") else {
        return HashMap::new();
    };
    let mut index: HashMap<i32, Vec<i16>> = HashMap::new();
    for quest_node in quest_info.children().unwrap_or_default() {
        let Ok(name) = quest_node.name() else {
            continue;
        };
        let Ok(quest_id) = name.parse::<i16>() else {
            continue;
        };
        if quest_id <= 0 {
            continue;
        }
        let Ok(Some(start)) = archive.root().resolve(&format!("Check.img/{quest_id}/0")) else {
            continue;
        };
        let npc_id = quest_node_integer(&start, "npc").unwrap_or(0);
        if npc_id > 0 {
            index.entry(npc_id).or_default().push(quest_id);
        }
    }
    for quests in index.values_mut() {
        quests.sort_unstable();
        quests.dedup();
    }
    index
}

fn quest_property_truthy(node: &openstory_assets::NxNode, field: &str) -> bool {
    let Some(value) = node
        .child(field)
        .ok()
        .flatten()
        .and_then(|value| value.value().ok())
    else {
        return false;
    };
    match value {
        NodeValue::Integer(value) => value != 0,
        NodeValue::Real(value) => value != 0.0,
        NodeValue::String(value) => {
            !value.trim().is_empty() && !matches!(value.trim(), "0" | "false" | "False")
        }
        _ => false,
    }
}

fn quest_is_completable(
    quest: &openstory_protocol::QuestEntry,
    archive: &NxArchive,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
) -> bool {
    let Ok(Some(end_check)) = archive.root().resolve(&format!("Check.img/{}/1", quest.id)) else {
        return false;
    };
    let mut has_objective = false;
    for (item_id, required) in quest_requirement_pairs(archive, quest.id, 1, "item") {
        has_objective = true;
        let Some(inventory) = inventory else {
            return false;
        };
        let have = inventory.item_count(item_id);
        if (required == 0 && have != 0) || (required > 0 && have < required) {
            return false;
        }
    }
    for (mob_index, (_, required)) in quest_requirement_pairs(archive, quest.id, 1, "mob")
        .into_iter()
        .enumerate()
    {
        if required == 0 {
            continue;
        }
        has_objective = true;
        if quest_progress_counter(&quest.data, mob_index) < required {
            return false;
        }
    }
    let required_meso = quest_node_integer(&end_check, "money").unwrap_or(0);
    if required_meso > 0 {
        has_objective = true;
        if inventory.map_or(0, |snapshot| snapshot.meso) < i64::from(required_meso) {
            return false;
        }
    }
    has_objective
}

fn quest_job_requirements_met(start_check: &openstory_assets::NxNode, player_job: u16) -> bool {
    let Ok(Some(jobs)) = start_check.child("job") else {
        return true;
    };
    let children = jobs.children().unwrap_or_default();
    if children.is_empty() {
        return true;
    }
    let player_job = i32::from(player_job);
    children.into_iter().any(|entry| {
        let Ok(NodeValue::Integer(required)) = entry.value() else {
            return false;
        };
        let Ok(required) = i32::try_from(required) else {
            return false;
        };
        required == 0
            || required == player_job
            || (required > 0 && player_job / 100 == required / 100 && player_job >= required)
    })
}

fn quest_prerequisites_met(
    start_check: &openstory_assets::NxNode,
    log: &openstory_protocol::QuestLog,
) -> bool {
    let Ok(Some(prerequisites)) = start_check.child("quest") else {
        return true;
    };
    prerequisites
        .children()
        .unwrap_or_default()
        .into_iter()
        .all(|prerequisite| {
            let id = quest_node_integer(&prerequisite, "id").unwrap_or(0);
            let state = quest_node_integer(&prerequisite, "state").unwrap_or(0);
            if id <= 0 {
                return true;
            }
            match state {
                1 => log.started.iter().any(|quest| i32::from(quest.id) == id),
                2 => log.completed.iter().any(|quest| i32::from(quest.id) == id),
                _ => true,
            }
        })
}

fn quest_is_available_for_player(
    quest_id: i16,
    npc_id: i32,
    log: &openstory_protocol::QuestLog,
    quest_archive: &NxArchive,
    character: &openstory_protocol::CharacterInfo,
) -> bool {
    if quest_id <= 0
        || log.started.iter().any(|quest| quest.id == quest_id)
        || log.completed.iter().any(|quest| quest.id == quest_id)
    {
        return false;
    }
    let Ok(Some(quest_info)) = quest_archive
        .root()
        .resolve(&format!("QuestInfo.img/{quest_id}"))
    else {
        return false;
    };
    if quest_property_truthy(&quest_info, "blocked")
        || quest_property_truthy(&quest_info, "autoStart")
        || quest_property_truthy(&quest_info, "autoPreComplete")
        || quest_info_text(quest_archive, quest_id, "name").is_none()
    {
        return false;
    }
    let Ok(Some(start)) = quest_archive
        .root()
        .resolve(&format!("Check.img/{quest_id}/0"))
    else {
        return false;
    };
    if quest_archive
        .root()
        .resolve(&format!("Check.img/{quest_id}/1"))
        .ok()
        .flatten()
        .is_none()
        || quest_node_integer(&start, "npc") != Some(npc_id)
        || quest_property_truthy(&start, "normalAutoStart")
        || quest_property_truthy(&start, "autoStart")
        || quest_property_truthy(&start, "autoPreComplete")
    {
        return false;
    }
    let level = i32::from(character.stats.level);
    let level_min = quest_node_integer(&start, "lvmin").unwrap_or(0);
    let level_max = quest_node_integer(&start, "lvmax").unwrap_or(0);
    !((level_min > 0 && (level < level_min || level - level_min > 10))
        || (level_max > 0 && level > level_max)
        || string_property(&start, "end").is_some_and(|date| !date.is_empty())
        || !quest_job_requirements_met(&start, character.stats.job)
        || !quest_prerequisites_met(&start, log))
}

fn available_quest_entries(
    log: &openstory_protocol::QuestLog,
    quest_archive: &NxArchive,
    start_npc_index: &HashMap<i32, Vec<i16>>,
    character: Option<&openstory_protocol::CharacterInfo>,
) -> Vec<(i16, i32)> {
    let Some(character) = character else {
        return Vec::new();
    };
    let mut available = start_npc_index
        .iter()
        .flat_map(|(&npc_id, quest_ids)| {
            quest_ids.iter().copied().filter_map(move |quest_id| {
                quest_is_available_for_player(quest_id, npc_id, log, quest_archive, character)
                    .then_some((quest_id, npc_id))
            })
        })
        .collect::<Vec<_>>();
    available.sort_unstable();
    available.dedup_by_key(|(quest_id, _)| *quest_id);
    available
}

fn npc_quest_marks(
    npc_ids: &HashSet<i32>,
    log: &openstory_protocol::QuestLog,
    quest_archive: &NxArchive,
    start_npc_index: &HashMap<i32, Vec<i16>>,
    character: &openstory_protocol::CharacterInfo,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
) -> HashMap<i32, NpcQuestMark> {
    let mut marks = HashMap::new();
    for quest in &log.started {
        let check_path = format!("Check.img/{}/", quest.id);
        let Ok(Some(check)) = quest_archive.root().resolve(&check_path) else {
            continue;
        };
        let start = check.child("0").ok().flatten();
        let end = check.child("1").ok().flatten();
        let start_npc = start
            .as_ref()
            .and_then(|node| quest_node_integer(node, "npc"))
            .unwrap_or(0);
        let end_npc = end
            .as_ref()
            .and_then(|node| quest_node_integer(node, "npc"))
            .unwrap_or(0);
        for npc_id in [start_npc, end_npc] {
            if npc_id <= 0 || !npc_ids.contains(&npc_id) || marks.contains_key(&npc_id) {
                continue;
            }
            let mark = if npc_id == end_npc && quest_is_completable(quest, quest_archive, inventory)
            {
                NpcQuestMark::Complete
            } else {
                NpcQuestMark::InProgress
            };
            marks.insert(npc_id, mark);
        }
    }

    for (_, npc_id) in available_quest_entries(log, quest_archive, start_npc_index, Some(character))
    {
        if npc_ids.contains(&npc_id) {
            marks.entry(npc_id).or_insert(NpcQuestMark::Available);
        }
    }
    marks
}

fn quest_objective_lines(
    log: &openstory_protocol::QuestLog,
    quest_id: i16,
    tab: QuestLogTab,
    archive: &NxArchive,
    string_archive: &NxArchive,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
    item_name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<(String, bool)> {
    let active = tab == QuestLogTab::Active;
    let available = tab == QuestLogTab::Available;
    let completed = tab == QuestLogTab::Completed;
    let check_side = if available { 0 } else { 1 };
    let data = if active {
        log.started
            .iter()
            .find(|quest| quest.id == quest_id)
            .map_or("", |quest| quest.data.as_str())
    } else {
        ""
    };
    let mut lines = Vec::new();

    for (item_id, required) in quest_requirement_pairs(archive, quest_id, check_side, "item") {
        if required == 0 {
            continue;
        }
        let have = if active {
            inventory.map_or(0, |snapshot| snapshot.item_count(item_id))
        } else if completed {
            required
        } else {
            0
        };
        let name = item_name(item_id, string_archive, item_name_cache);
        let line = if available {
            format!("{name} × {required}")
        } else {
            format!("{name}  {have}/{required}")
        };
        lines.push((line, completed || (!available && have >= required)));
    }

    let mut mob_index = 0;
    for (mob_id, required) in quest_requirement_pairs(archive, quest_id, check_side, "mob") {
        let progress_index = mob_index;
        mob_index += 1;
        if required == 0 {
            continue;
        }
        let killed = if active {
            quest_progress_counter(data, progress_index)
        } else if completed {
            required
        } else {
            0
        };
        let name = string_archive
            .root()
            .resolve(&format!("Mob.img/{mob_id}/name"))
            .ok()
            .flatten()
            .and_then(|node| match node.value().ok()? {
                NodeValue::String(name) if !name.trim().is_empty() => Some(name),
                _ => None,
            })
            .unwrap_or_else(|| format!("Monster #{mob_id}"));
        let line = if available {
            format!("{name} × {required}")
        } else {
            format!("{name}  {killed}/{required}")
        };
        lines.push((line, completed || (!available && killed >= required)));
    }

    if let Ok(Some(end_check)) = archive
        .root()
        .resolve(&format!("Check.img/{quest_id}/{check_side}"))
    {
        if let Some(npc_id) = quest_node_integer(&end_check, "npc") {
            if npc_id > 0 {
                let name = string_archive
                    .root()
                    .resolve(&format!("Npc.img/{npc_id}/name"))
                    .ok()
                    .flatten()
                    .and_then(|node| match node.value().ok()? {
                        NodeValue::String(name) if !name.trim().is_empty() => Some(name),
                        _ => None,
                    })
                    .unwrap_or_else(|| format!("NPC #{npc_id}"));
                lines.push((format!("Talk to {name}"), completed));
            }
        }
    }
    lines
}

fn render_quest_log_panel(
    font: &fontdue::Font,
    log: &openstory_protocol::QuestLog,
    available_quests: &[(i16, i32)],
    archive: &NxArchive,
    string_archive: &NxArchive,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
    item_name_cache: &mut HashMap<i32, Option<String>>,
    tab: QuestLogTab,
    selected: usize,
    scroll: usize,
    selected_reward: usize,
    character: Option<&openstory_protocol::CharacterInfo>,
    confirmation: Option<i16>,
    ui_archive: &NxArchive,
) -> Vec<u8> {
    const WIDTH: usize = QUEST_LOG_PANEL_WIDTH as usize;
    const HEIGHT: usize = QUEST_LOG_PANEL_HEIGHT as usize;
    const LIST_WIDTH: usize = 245;
    const DETAIL_X: usize = QUEST_LOG_DETAIL_X;
    const VISIBLE_ROWS: usize = QUEST_LOG_VISIBLE_ROWS;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    let ids = quest_ids_for_tab(log, available_quests, tab);
    let selected_quest = ids.get(selected).copied();
    let _ = composite_nx_ui_layers(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        &[("UIWindow.img/Quest/backgrnd", 0, 0)],
    );
    if selected_quest.is_some() {
        // The installed v83 UI.nx has no UIWindow2.img/Quest/quest_info
        // subtree. Its backgrnd3/4 assets are blue-card variants and backgrnd5
        // is a modal paper texture, so use the authored white Quest window
        // frame for the detail pane as well.
        let _ = composite_nx_ui_layers(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            &[
                ("UIWindow.img/Quest/backgrnd", DETAIL_X as i32, 0),
                (
                    "UIWindow.img/BtUIClose/normal/0",
                    (DETAIL_X + QUEST_LOG_DETAIL_WIDTH as usize - 35) as i32,
                    5,
                ),
            ],
        );
    }
    let active = tab == QuestLogTab::Active;
    let available = tab == QuestLogTab::Available;
    let completed = tab == QuestLogTab::Completed;
    for (index, selected_tab) in [available, active, completed].into_iter().enumerate() {
        let state = if selected_tab { "enabled" } else { "disabled" };
        let path = format!("UIWindow.img/Quest/Tab/{state}/{index}");
        let x = 17 + index as i32 * 70;
        let _ = composite_nx_ui_bitmap(&mut pixels, WIDTH, HEIGHT, ui_archive, &path, x, 23);
    }
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "UIWindow.img/BtUIClose/normal/0",
        (LIST_WIDTH - 35) as i32,
        5,
    );

    if ids.is_empty() {
        let notice_index = if available {
            0
        } else if active {
            1
        } else {
            2
        };
        let notice_width = [143, 150, 170][notice_index];
        let notice_path = format!("UIWindow.img/Quest/notice{notice_index}");
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            &notice_path,
            ((LIST_WIDTH - notice_width) / 2) as i32,
            164,
        );
    } else {
        for visible in 0..VISIBLE_ROWS {
            let index = scroll.saturating_add(visible);
            let Some(id) = ids.get(index).copied() else {
                break;
            };
            let y = 54 + visible * 25;
            if index == selected {
                let _ = composite_nx_ui_bitmap(
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    ui_archive,
                    "UIWindow.img/Quest/icon1",
                    11,
                    y as i32 + 4,
                );
            } else {
                let _ = composite_nx_ui_bitmap(
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    ui_archive,
                    "UIWindow.img/Quest/icon0",
                    11,
                    y as i32 + 4,
                );
            }
            let name =
                quest_info_text(archive, id, "name").unwrap_or_else(|| format!("Quest #{id}"));
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &name,
                31,
                y as i32 + 1,
                11.0,
                LIST_WIDTH - 42,
                23,
                if index == selected {
                    [35, 104, 163]
                } else {
                    [41, 39, 33]
                },
            );
        }
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &format!("{} quest(s)", ids.len()),
        14,
        270,
        9.0,
        170,
        14,
        [74, 70, 61],
    );

    if let Some(id) = selected_quest {
        let text_x = DETAIL_X as i32 + 18;
        let text_width = QUEST_LOG_DETAIL_WIDTH as usize - 36;
        let name = quest_info_text(archive, id, "name").unwrap_or_else(|| format!("Quest #{id}"));
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &name,
            text_x,
            27,
            11.0,
            text_width,
            24,
            [37, 57, 72],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!(
                "Quest ID: {id}  ·  {}",
                match tab {
                    QuestLogTab::Available => "Available",
                    QuestLogTab::Active => "In progress",
                    QuestLogTab::Completed => "Completed",
                }
            ),
            text_x,
            48,
            9.0,
            text_width,
            14,
            [89, 83, 71],
        );
        if let Some(summary) = quest_info_text(archive, id, "0") {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &summary,
                text_x,
                68,
                11.0,
                text_width,
                34,
                [53, 50, 44],
            );
        }
        if let Some(description) = quest_info_text(archive, id, "1") {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &description,
                text_x,
                103,
                10.0,
                text_width,
                48,
                [73, 69, 61],
            );
        }
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            if available {
                "Requirements"
            } else {
                "Objectives"
            },
            text_x,
            155,
            10.0,
            text_width,
            15,
            [49, 75, 93],
        );
        let objectives = quest_objective_lines(
            log,
            id,
            tab,
            archive,
            string_archive,
            inventory,
            item_name_cache,
        );
        if objectives.is_empty() {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                if available {
                    "No listed requirements."
                } else {
                    "No tracked objectives."
                },
                text_x,
                175,
                10.0,
                text_width,
                16,
                [79, 74, 66],
            );
        } else {
            for (index, (objective, done)) in objectives.iter().take(5).enumerate() {
                let prefix = if available {
                    "· "
                } else if *done {
                    "✓ "
                } else {
                    "○ "
                };
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    &format!("{prefix}{objective}"),
                    text_x,
                    174 + index as i32 * 18,
                    9.0,
                    text_width,
                    17,
                    if *done { [62, 112, 57] } else { [57, 53, 45] },
                );
            }
        }
        if active {
            let reward_choices = character.map_or_else(Vec::new, |character| {
                quest_reward_choices(id, archive, string_archive, character, item_name_cache)
            });
            if !reward_choices.is_empty() {
                let choice_index = selected_reward.min(reward_choices.len() - 1);
                let choice = &reward_choices[choice_index];
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    &format!(
                        "Reward: {} × {} ({}/{})",
                        choice.name,
                        choice.count,
                        choice_index + 1,
                        reward_choices.len()
                    ),
                    text_x,
                    304,
                    9.0,
                    text_width,
                    28,
                    [91, 74, 42],
                );
            }
        }
        if available {
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                WIDTH,
                HEIGHT,
                ui_archive,
                "UIWindow.img/Quest/BtOK/normal/0",
                DETAIL_X as i32 + 42,
                367,
            );
        } else if active {
            let can_complete = log
                .started
                .iter()
                .find(|quest| quest.id == id)
                .is_some_and(|quest| quest_is_completable(quest, archive, inventory));
            if can_complete {
                let _ = composite_nx_ui_bitmap(
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    ui_archive,
                    "UIWindow.img/Quest/BtOK/normal/0",
                    DETAIL_X as i32 + 42,
                    367,
                );
            }
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                WIDTH,
                HEIGHT,
                ui_archive,
                "UIWindow.img/Quest/BtGiveup/normal/0",
                DETAIL_X as i32 + 126,
                367,
            );
        }
    }
    if let Some(id) = confirmation {
        let overlay_x = ((WIDTH - 266) / 2) as i32;
        let overlay_y = 140;
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            "UIWindow.img/Quest/backgrnd5",
            overlay_x,
            overlay_y,
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!("Forfeit quest #{id}?"),
            overlay_x + 24,
            overlay_y + 28,
            12.0,
            218,
            30,
            [55, 50, 42],
        );
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            "UIWindow.img/Quest/BtOK/normal/0",
            overlay_x + 44,
            overlay_y + 79,
        );
        let _ = composite_nx_ui_bitmap(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            "UIWindow.img/Quest/BtNo/normal/0",
            overlay_x + 147,
            overlay_y + 79,
        );
    }
    pixels
}

fn primary_ability_stat(
    character: &openstory_protocol::CharacterInfo,
) -> openstory_protocol::AbilityStat {
    use openstory_protocol::AbilityStat as Stat;
    let job = character.stats.job;
    match job / 100 {
        2 | 6 | 7 | 8 | 12 | 22 => Stat::Intelligence,
        3 | 13 => Stat::Dexterity,
        4 | 14 => Stat::Luck,
        5 if character.look.weapon_id / 10_000 == 149 => Stat::Dexterity,
        _ => Stat::Strength,
    }
}

fn character_job_name(job: u16) -> String {
    // v83 uses Rogue for job 400 in its character-selection stat sheet.
    if job == 400 {
        return "Rogue".to_owned();
    }
    let branch = match job / 100 {
        0 => "Beginner",
        1 => "Warrior",
        2 => "Magician",
        3 => "Bowman",
        4 => "Thief",
        5 => "Pirate",
        10 => "Noblesse",
        11 => "Dawn Warrior",
        12 => "Blaze Wizard",
        13 => "Wind Archer",
        14 => "Night Walker",
        15 => "Thunder Breaker",
        20 => "Legend",
        21 => "Aran",
        30 => "Evan",
        90 => "GM",
        _ => return format!("Job {job}"),
    };
    if job == 0 || job == 1000 || job == 2000 || job == 3000 || job == 900 {
        branch.to_owned()
    } else {
        format!("{branch} ({job})")
    }
}

fn render_status_panel(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    character: &openstory_protocol::CharacterInfo,
    waiting_for_stats: bool,
    auto_confirmation: bool,
) -> Vec<u8> {
    const WIDTH: usize = STATUS_PANEL_WIDTH as usize;
    const HEIGHT: usize = STATUS_PANEL_HEIGHT as usize;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    let _ = composite_nx_ui_layers(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        &[("UIWindow.img/Stat/backgrnd", 0, 0)],
    );
    let auto_state = if character.stats.ability_points > 0 && !waiting_for_stats {
        "normal"
    } else {
        "disabled"
    };
    let auto_path = format!("UIWindow.img/Stat/BtAuto/{auto_state}/0");
    let _ = composite_nx_ui_layers(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        &[
            (auto_path.as_str(), 98, 203),
            ("UIWindow.img/Stat/BtDetail/normal/0", 124, 323),
            ("Basic.img/BtClose/normal/0", 155, 6),
        ],
    );

    let stats = &character.stats;
    let color = [56, 42, 31];
    let values = [
        (stats.name.clone(), 59, 30),
        (character_job_name(stats.job), 59, 48),
        (stats.level.to_string(), 59, 66),
        ("-".to_owned(), 59, 84),
        (format!("{} / {}", stats.hp, stats.max_hp), 59, 102),
        (format!("{} / {}", stats.mp, stats.max_mp), 59, 120),
        (stats.experience.to_string(), 59, 138),
        (stats.fame.to_string(), 59, 156),
        (stats.strength.to_string(), 59, 244),
        (stats.dexterity.to_string(), 59, 262),
        (stats.intelligence.to_string(), 59, 280),
        (stats.luck.to_string(), 59, 298),
    ];
    for (value, x, y) in values {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &value,
            x,
            y,
            11.0,
            111,
            16,
            color,
        );
    }
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        "ABILITY POINT",
        13,
        196,
        11.0,
        WIDTH - 26,
        16,
        color,
    );
    let ability_points = stats.ability_points.to_string();
    let ability_points_width = dialogue_text_width(font, &ability_points, 11.0).ceil() as i32;
    draw_dialogue_text(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &ability_points,
        61 + (29 - ability_points_width) / 2,
        214,
        11.0,
        29,
        15,
        color,
    );

    if auto_confirmation {
        let _ = composite_nx_ui_layers(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            &[("UIWindow.img/Quest/backgrnd5", -45, 104)],
        );
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "Auto-allocate AP?",
            22,
            121,
            12.0,
            132,
            18,
            color,
        );
        let primary = match primary_ability_stat(character) {
            openstory_protocol::AbilityStat::Hp => "HP",
            openstory_protocol::AbilityStat::Mp => "MP",
            openstory_protocol::AbilityStat::Strength => "STR",
            openstory_protocol::AbilityStat::Dexterity => "DEX",
            openstory_protocol::AbilityStat::Intelligence => "INT",
            openstory_protocol::AbilityStat::Luck => "LUK",
        };
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &format!("{} points to {primary}.", stats.ability_points),
            22,
            148,
            10.0,
            132,
            20,
            color,
        );
        let _ = composite_nx_ui_layers(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            &[
                ("UIWindow.img/Quest/BtOK/normal/0", 20, 185),
                ("UIWindow.img/Quest/BtNo/normal/0", 93, 185),
            ],
        );
    }
    pixels
}

fn render_status_detail_panel(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    character: &openstory_protocol::CharacterInfo,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
    character_archive: &NxArchive,
    item_archive: &NxArchive,
    skill_archive: &NxArchive,
    skills: &HashMap<i32, openstory_protocol::SkillEntry>,
    active_buffs: &HashMap<openstory_protocol::BuffStatKey, ActiveCharacterBuff>,
    forced_stats: &ForcedEquipmentStats,
) -> Vec<u8> {
    const WIDTH: usize = STATUS_DETAIL_WIDTH as usize;
    const HEIGHT: usize = STATUS_DETAIL_HEIGHT as usize;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        HEIGHT,
        ui_archive,
        "UIWindow.img/Stat/backgrnd2",
        0,
        0,
    );
    recolor_v83_detail_labels(&mut pixels, WIDTH, HEIGHT);

    let combat = status_detail_combat_values(
        character,
        inventory,
        character_archive,
        item_archive,
        skill_archive,
        skills,
        active_buffs,
        forced_stats,
    );
    let equipment_stat = |index: usize| -> i32 {
        inventory.map_or(0, |inventory| {
            inventory
                .equipped()
                .filter_map(|item| item.equip.as_ref())
                .map(|equip| i32::from(equip.stats[index]))
                .sum()
        })
    };
    let weapon_defense = forced_stats
        .weapon_defense
        .map_or_else(|| equipment_stat(8), i32::from);
    let magic_defense = forced_stats
        .magic_defense
        .map_or_else(|| equipment_stat(9), i32::from);
    let avoidability = forced_stats.avoidability.map_or_else(
        || {
            equipment_stat(11)
                .saturating_add(i32::from(character.stats.luck) / 2)
                .saturating_add(i32::from(character.stats.dexterity) / 4)
        },
        i32::from,
    );
    let hands = equipment_stat(12);
    let rows = [
        (
            combat.attack_range.map_or_else(
                || "—".to_owned(),
                |(minimum, maximum)| format!("{minimum} ~ {maximum}"),
            ),
            7,
        ),
        (weapon_defense.to_string(), 25),
        (combat.magic.to_string(), 43),
        (magic_defense.to_string(), 61),
        (combat.accuracy.to_string(), 79),
        (avoidability.to_string(), 97),
        (hands.to_string(), 115),
        (format!("{}%", combat.speed), 133),
        (format!("{}%", combat.jump), 151),
    ];
    for (value, y) in rows {
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &value,
            86,
            y,
            10.0,
            82,
            16,
            [48, 41, 33],
        );
    }

    pixels
}

fn recolor_v83_detail_labels(pixels: &mut [u8], width: usize, height: usize) {
    for y in 6..171.min(height) {
        for x in 12..76.min(width) {
            let index = (y * width + x) * 4;
            let blue = pixels[index];
            let green = pixels[index + 1];
            let red = pixels[index + 2];
            if pixels[index + 3] == 0 || blue < red.saturating_add(22) {
                continue;
            }
            let gray = ((u16::from(blue) + u16::from(green) + u16::from(red)) / 3) as u8;
            pixels[index] = gray;
            pixels[index + 1] = gray;
            pixels[index + 2] = gray;
        }
    }
}

const KEY_CONFIG_ACTIONS: [(&str, u8, i32, bool); 41] = [
    ("Equipment", 4, 0, true),
    ("Items", 4, 1, true),
    ("Stats", 4, 2, true),
    ("Skills", 4, 3, true),
    ("Friends", 4, 4, true),
    ("World map", 4, 5, true),
    ("Public chat", 4, 6, true),
    ("Mini map", 4, 7, true),
    ("Quest log", 4, 8, true),
    ("Key bindings", 4, 9, true),
    ("Say", 4, 10, true),
    ("Whisper", 4, 11, true),
    ("Party chat", 4, 12, true),
    ("Friends chat", 4, 13, true),
    ("Status menu", 4, 14, true),
    ("Quick slots", 4, 15, true),
    ("Toggle chat", 4, 16, true),
    ("Guild", 4, 17, true),
    ("Guild chat", 4, 18, true),
    ("Party", 4, 19, true),
    ("Character info", 4, 21, true),
    ("Monster Book", 4, 22, true),
    ("Cash Shop", 4, 23, true),
    ("Alliance chat", 4, 24, true),
    ("Change channel", 4, 25, true),
    // v83's action 26 is the Family shortcut. Do not reuse it for newer UI.
    ("Family", 4, 26, true),
    ("Boss party (unsupported)", 4, 27, false),
    ("Settings", 4, 28, true),
    ("Family (extended)", 4, 42, true),
    ("Pick up", 5, 50, true),
    ("Sit", 5, 51, true),
    ("Attack", 5, 52, true),
    ("Jump", 5, 53, true),
    ("NPC interaction", 5, 54, true),
    ("Face 1", 6, 100, true),
    ("Face 2", 6, 101, true),
    ("Face 3", 6, 102, true),
    ("Face 4", 6, 103, true),
    ("Face 5", 6, 104, true),
    ("Face 6", 6, 105, true),
    ("Face 7", 6, 106, true),
];

fn static_key_config_actions() -> Vec<KeyConfigAction> {
    KEY_CONFIG_ACTIONS
        .iter()
        .map(|(label, kind, action, assignable)| KeyConfigAction {
            label: (*label).to_owned(),
            kind: *kind,
            action: *action,
            assignable: *assignable,
        })
        .collect()
}

fn key_config_skill_name(skill_id: i32, string_archive: &NxArchive) -> String {
    let path = format!("Skill.img/{skill_id:07}");
    string_archive
        .root()
        .resolve(&path)
        .ok()
        .flatten()
        .and_then(|node| string_property(&node, "name"))
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| format!("Skill #{skill_id}"))
}

fn build_key_config_actions(
    inventory: Option<&openstory_protocol::InventorySnapshot>,
    skills: &HashMap<i32, openstory_protocol::SkillEntry>,
    server_keymap: Option<&[openstory_protocol::KeyBinding]>,
    macros: &[openstory_protocol::SkillMacro],
    string_archive: &NxArchive,
    item_name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<KeyConfigAction> {
    let mut actions = static_key_config_actions();
    let mut skill_actions = BTreeMap::<i32, KeyConfigAction>::new();
    let mut item_actions = BTreeMap::<(u8, i32), KeyConfigAction>::new();
    let mut macro_actions = BTreeMap::<i32, KeyConfigAction>::new();

    if let Some(bindings) = server_keymap {
        for binding in bindings {
            match (binding.kind, binding.action) {
                (4, action)
                    if !actions
                        .iter()
                        .any(|entry| entry.kind == binding.kind && entry.action == action) =>
                {
                    let label = legacy_menu_action_label(action).unwrap_or("Unknown menu action");
                    actions.push(KeyConfigAction {
                        label: format!("{label} (unsupported)"),
                        kind: binding.kind,
                        action,
                        assignable: false,
                    });
                }
                (1, skill_id) if skill_id > 0 => {
                    let name = key_config_skill_name(skill_id, string_archive);
                    skill_actions.entry(skill_id).or_insert(KeyConfigAction {
                        label: format!("Skill: {name} (unavailable)"),
                        kind: 1,
                        action: skill_id,
                        assignable: false,
                    });
                }
                (kind @ (2 | 3), item_id) if item_id > 0 => {
                    let name = item_name(item_id, string_archive, item_name_cache);
                    let category = if kind == 3 { "Cash" } else { "Item" };
                    item_actions
                        .entry((kind, item_id))
                        .or_insert(KeyConfigAction {
                            label: format!("{category}: {name} (not owned)"),
                            kind,
                            action: item_id,
                            assignable: false,
                        });
                }
                (8, macro_index) => {
                    let label = usize::try_from(macro_index)
                        .ok()
                        .and_then(|index| macros.get(index).map(|macro_entry| (index, macro_entry)))
                        .map_or_else(
                            || format!("Macro #{macro_index} (unavailable)"),
                            |(index, macro_entry)| {
                                if macro_entry.name.is_empty() {
                                    format!("Macro {}", index + 1)
                                } else {
                                    format!("Macro: {}", macro_entry.name)
                                }
                            },
                        );
                    macro_actions.entry(macro_index).or_insert(KeyConfigAction {
                        label,
                        kind: 8,
                        action: macro_index,
                        assignable: usize::try_from(macro_index)
                            .is_ok_and(|index| index < openstory_protocol::SKILL_MACRO_COUNT),
                    });
                }
                _ => {}
            }
        }
    }

    for skill in skills.values().filter(|skill| skill.id > 0) {
        let passive = (skill.id % 10_000) / 1_000 == 0;
        if skill.level <= 0 || passive {
            continue;
        }
        let name = key_config_skill_name(skill.id, string_archive);
        skill_actions.insert(
            skill.id,
            KeyConfigAction {
                label: format!("Skill: {name}"),
                kind: 1,
                action: skill.id,
                assignable: true,
            },
        );
    }

    if let Some(inventory) = inventory {
        for item in inventory.items.iter().filter(|item| item.count > 0) {
            let (kind, category) = match item.inventory_type {
                openstory_protocol::InventoryType::Use => (2, "Use"),
                openstory_protocol::InventoryType::Setup => (2, "Setup"),
                openstory_protocol::InventoryType::Cash => (3, "Cash"),
                _ => continue,
            };
            let name = item_name(item.item_id, string_archive, item_name_cache);
            item_actions.insert(
                (kind, item.item_id),
                KeyConfigAction {
                    label: format!("{category}: {name}"),
                    kind,
                    action: item.item_id,
                    assignable: true,
                },
            );
        }
    }

    actions.extend(skill_actions.into_values());
    actions.extend(item_actions.into_values());
    for (index, macro_entry) in macros.iter().enumerate() {
        let action = index as i32;
        macro_actions.insert(
            action,
            KeyConfigAction {
                label: if macro_entry.name.is_empty() {
                    format!("Macro {}", index + 1)
                } else {
                    format!("Macro: {}", macro_entry.name)
                },
                kind: 8,
                action,
                assignable: true,
            },
        );
    }
    actions.extend(macro_actions.into_values());
    actions
}

fn legacy_menu_action_label(action: i32) -> Option<&'static str> {
    Some(match action {
        14 => "Menu",
        20 => "Notifier",
        26 => "Family",
        27 => "Boss party",
        29 => "Screenshot",
        30 => "Profession",
        31 => "Item pot",
        32 => "Event",
        33 => "Silent Crusade",
        34 => "Legion",
        35 => "Battle analysis",
        36 => "Bits",
        37 => "Guide",
        38 => "Enhance equipment",
        39 => "Monster collection",
        40 => "Soul weapon",
        41 => "Maple news",
        _ => return None,
    })
}

fn default_keymap_bindings() -> Vec<openstory_protocol::KeyBinding> {
    let mut bindings = vec![openstory_protocol::KeyBinding::default(); 90];
    for (key, action) in [(29, 52), (56, 53), (45, 50), (46, 51), (57, 54)] {
        bindings[key] = openstory_protocol::KeyBinding { kind: 5, action };
    }
    bindings
}
