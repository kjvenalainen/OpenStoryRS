fn quest_ids_for_tab(
    log: &openstory_protocol::QuestLog,
    available: &[(i16, i32)],
    tab: QuestLogTab,
    archive: &NxArchive,
    font: &fontdue::Font,
) -> Vec<i16> {
    let ids: Vec<i16> = match tab {
        QuestLogTab::Active => log.started.iter().map(|quest| quest.id).collect(),
        QuestLogTab::Available => available.iter().map(|(id, _)| *id).collect(),
        QuestLogTab::Completed => log.completed.iter().map(|quest| quest.id).collect(),
    };
    ids.into_iter()
        .filter(|id| quest_is_displayable(archive, *id, font))
        .collect()
}

fn quest_is_displayable(archive: &NxArchive, quest_id: i16, font: &fontdue::Font) -> bool {
    if quest_id <= 0 {
        return false;
    }
    let Ok(Some(quest_info)) = archive.root().resolve(&format!("QuestInfo.img/{quest_id}")) else {
        return false;
    };
    if quest_property_truthy(&quest_info, "blocked") || quest_property_truthy(&quest_info, "hidden")
    {
        return false;
    }
    quest_info_text(archive, quest_id, "name")
        .is_some_and(|name| quest_name_is_displayable(font, &name))
}

fn quest_name_is_displayable(font: &fontdue::Font, name: &str) -> bool {
    name.chars().any(char::is_alphanumeric)
        && name.chars().all(|character| {
            !matches!(character, '\u{fffd}' | '\u{25a0}' | '\u{25a1}')
                && (character.is_whitespace()
                    || (!character.is_control() && font.has_glyph(character)))
        })
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

fn quest_description_is_displayable(description: &str) -> bool {
    // Some NX quest descriptions contain embedded control bytes instead of prose.
    !description.trim().is_empty()
        && description
            .chars()
            .all(|character| !character.is_control() || matches!(character, '\r' | '\n' | '\t'))
}

fn clean_quest_detail_text(text: &str) -> String {
    let normalized = text
        .replace("\\r\\n", "\n")
        .replace("\\n", "\n")
        .replace("\\r", "\n")
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let prose = normalized
        .lines()
        .filter(|line| !is_quest_item_progress_markup_row(line))
        .collect::<Vec<_>>()
        .join("\n");
    clean_npc_markup(&prose).trim().to_owned()
}

fn is_quest_item_progress_markup_row(line: &str) -> bool {
    let line = line.trim();
    let Some((markup, count)) = line.rsplit_once('/') else {
        return false;
    };
    markup.contains("#t")
        && markup.contains("#c")
        && !count.is_empty()
        && count.chars().all(|character| character.is_ascii_digit())
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

fn quest_dialogue_pages(archive: &NxArchive, path: &str) -> Vec<String> {
    let Some(node) = archive.root().resolve(path).ok().flatten() else {
        return Vec::new();
    };
    let mut pages = node
        .children()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|node| {
            let index = node.name().ok()?.parse::<usize>().ok()?;
            match node.value().ok()? {
                NodeValue::String(text) if !text.trim().is_empty() => Some((index, text)),
                _ => None,
            }
        })
        .collect::<Vec<_>>();
    pages.sort_by_key(|(index, _)| *index);
    pages.into_iter().map(|(_, text)| text).collect()
}

fn resolve_quest_dialogue_references(text: &str, string_archive: &NxArchive) -> String {
    let characters = text.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if character == 'M'
            && characters.get(index + 1) == Some(&'a')
            && characters.get(index + 2) == Some(&'p')
            && characters.get(index + 3) == Some(&' ')
        {
            let id_start = index + 4;
            let id_end = id_start
                + characters[id_start..]
                    .iter()
                    .take_while(|value| value.is_ascii_digit())
                    .count();
            let token_ends = id_end > id_start
                && characters
                    .get(id_end)
                    .is_none_or(|next| !next.is_ascii_alphanumeric() && *next != '_');
            if token_ends
                && let Ok(map_id) = characters[id_start..id_end]
                    .iter()
                    .collect::<String>()
                    .parse::<i32>()
                && let Some(name) = map_string_name(string_archive, map_id)
            {
                output.push_str(&name);
                index = id_end;
                continue;
            }
        }
        if character == '#'
            && let Some(code @ ('m' | 'p')) = characters.get(index + 1).copied()
        {
            let id_start = index + 2;
            let Some(id_tail) = characters.get(id_start..) else {
                output.push(character);
                index += 1;
                continue;
            };
            let Some(relative_end) = id_tail.iter().position(|value| *value == '#') else {
                output.push(character);
                index += 1;
                continue;
            };
            let id_end = id_start + relative_end;
            let id = characters[id_start..id_end].iter().collect::<String>();
            if !id.is_empty() && id.chars().all(|value| value.is_ascii_digit()) {
                let name = if code == 'm' {
                    id.parse::<i32>()
                        .ok()
                        .and_then(|map_id| map_string_name(string_archive, map_id))
                        .unwrap_or_else(|| format!("Map {id}"))
                } else {
                    nx_string_value(string_archive, &format!("Npc.img/{id}/name"))
                        .unwrap_or_else(|| format!("NPC {id}"))
                };
                output.push_str(&name);
                index = id_end + 1;
                continue;
            }
        }
        output.push(character);
        index += 1;
    }
    output
}

fn quest_start_items_met(
    archive: &NxArchive,
    quest_id: i16,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
) -> bool {
    quest_requirement_pairs(archive, quest_id, 0, "item")
        .into_iter()
        .all(|(item_id, required)| {
            let have = inventory.map_or(0, |inventory| inventory.item_count(item_id));
            if required == 0 {
                have == 0
            } else {
                have >= required
            }
        })
}

#[derive(Clone, Debug)]
struct QuestRewardChoice {
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
        let item_id = quest_node_integer(&item, "id").unwrap_or(0);
        let count = quest_node_integer(&item, "count").unwrap_or(1).max(1);
        choices.push(QuestRewardChoice {
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
    log: &openstory_protocol::QuestLog,
) -> bool {
    // Some v83 quests omit Check.img/{id}/1 entirely. Cosmic treats that as
    // an empty completion-requirement set, so an active quest with no end
    // check is still eligible to turn in.
    let end_check = archive
        .root()
        .resolve(&format!("Check.img/{}/1", quest.id))
        .ok()
        .flatten();
    if let Some(end_check) = &end_check
        && (!quest_prerequisites_met(end_check, log)
            || !quest_info_progress_met(end_check, quest, log))
    {
        return false;
    }
    for (item_id, required) in quest_requirement_pairs(archive, quest.id, 1, "item") {
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
        if quest_progress_counter(&quest.data, mob_index) < required {
            return false;
        }
    }
    let required_meso = end_check
        .as_ref()
        .and_then(|end_check| quest_node_integer(end_check, "money"))
        .unwrap_or(0);
    if required_meso > 0 {
        if inventory.map_or(0, |snapshot| snapshot.meso) < i64::from(required_meso) {
            return false;
        }
    }
    true
}

fn quest_info_progress_met(
    end_check: &openstory_assets::NxNode,
    quest: &openstory_protocol::QuestEntry,
    log: &openstory_protocol::QuestLog,
) -> bool {
    let Ok(Some(requirements)) = end_check.child("infoex") else {
        return true;
    };
    let Ok(entries) = requirements.children() else {
        return false;
    };
    let mut expected = Vec::with_capacity(entries.len());
    for requirement in entries {
        let Ok(name) = requirement.name() else {
            return false;
        };
        let Ok(index) = name.parse::<usize>() else {
            return false;
        };
        let Ok(Some(value)) = requirement.child("value") else {
            return false;
        };
        let Ok(NodeValue::String(value)) = value.value() else {
            return false;
        };
        expected.push((index, value));
    }
    if expected.is_empty() {
        return true;
    }
    expected.sort_by_key(|(index, _)| *index);

    // Cosmic stores InfoEx progress under the end-check's infoNumber quest,
    // and sends that quest's progress string in the ordinary quest update.
    let progress_quest_id = quest_node_integer(end_check, "infoNumber")
        .filter(|id| *id > 0)
        .and_then(|id| i16::try_from(id).ok())
        .unwrap_or(quest.id);
    let progress = if progress_quest_id == quest.id {
        Some(quest.data.as_str())
    } else {
        log.started
            .iter()
            .find(|entry| entry.id == progress_quest_id)
            .map(|entry| entry.data.as_str())
    };
    let expected = expected
        .into_iter()
        .map(|(_, value)| value)
        .collect::<String>();
    progress == Some(expected.as_str())
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
                0 => {
                    !log.started.iter().any(|quest| i32::from(quest.id) == id)
                        && !log.completed.iter().any(|quest| i32::from(quest.id) == id)
                }
                _ => false,
            }
        })
}

fn quest_is_available_for_player(
    quest_id: i16,
    npc_id: i32,
    log: &openstory_protocol::QuestLog,
    quest_archive: &NxArchive,
    character: &openstory_protocol::CharacterInfo,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
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
    if start
        .child("fieldEnter")
        .ok()
        .flatten()
        .is_some_and(|field| quest_node_integer(&field, "0") != Some(character.stats.map_id))
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
        || !quest_prerequisites_met(&start, log)
        || !quest_start_items_met(quest_archive, quest_id, inventory))
}

fn available_quest_entries(
    log: &openstory_protocol::QuestLog,
    quest_archive: &NxArchive,
    start_npc_index: &HashMap<i32, Vec<i16>>,
    character: Option<&openstory_protocol::CharacterInfo>,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
) -> Vec<(i16, i32)> {
    let Some(character) = character else {
        return Vec::new();
    };
    let mut available = start_npc_index
        .iter()
        .flat_map(|(&npc_id, quest_ids)| {
            quest_ids.iter().copied().filter_map(move |quest_id| {
                quest_is_available_for_player(
                    quest_id,
                    npc_id,
                    log,
                    quest_archive,
                    character,
                    inventory,
                )
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
            .unwrap_or(start_npc);
        for npc_id in [start_npc, end_npc] {
            if npc_id <= 0 || !npc_ids.contains(&npc_id) || marks.contains_key(&npc_id) {
                continue;
            }
            let mark = if npc_id == end_npc
                && quest_is_completable(quest, quest_archive, inventory, log)
            {
                NpcQuestMark::Complete
            } else {
                NpcQuestMark::InProgress
            };
            marks.insert(npc_id, mark);
        }
    }

    for (_, npc_id) in available_quest_entries(
        log,
        quest_archive,
        start_npc_index,
        Some(character),
        inventory,
    ) {
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

#[derive(Clone, Debug)]
struct QuestHelperDisplayEntry {
    name: String,
    objectives: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct QuestHelperProgressSnapshot {
    data: String,
    item_counts: Vec<(i32, u32)>,
}

fn quest_helper_progress_snapshot(
    quest: &openstory_protocol::QuestEntry,
    archive: &NxArchive,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
) -> QuestHelperProgressSnapshot {
    let item_counts = quest_requirement_pairs(archive, quest.id, 1, "item")
        .into_iter()
        .filter(|(_, required)| *required > 0)
        .map(|(item_id, _)| {
            (
                item_id,
                inventory.map_or(0, |snapshot| snapshot.item_count(item_id)),
            )
        })
        .collect();
    QuestHelperProgressSnapshot {
        data: quest.data.clone(),
        item_counts,
    }
}

fn sync_quest_helper_order(
    log: &openstory_protocol::QuestLog,
    archive: &NxArchive,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
    order: &mut Vec<i16>,
    previous_progress: &mut HashMap<i16, QuestHelperProgressSnapshot>,
    initialized: &mut bool,
) {
    let current_progress: HashMap<i16, QuestHelperProgressSnapshot> = log
        .started
        .iter()
        .map(|quest| {
            (
                quest.id,
                quest_helper_progress_snapshot(quest, archive, inventory),
            )
        })
        .collect();

    if !*initialized {
        *order = log.started.iter().map(|quest| quest.id).collect();
        *previous_progress = current_progress;
        *initialized = true;
        return;
    }

    let current_ids: HashSet<i16> = current_progress.keys().copied().collect();
    let newly_started: HashSet<i16> = current_ids
        .iter()
        .filter(|quest_id| !previous_progress.contains_key(*quest_id))
        .copied()
        .collect();
    let progressed: HashSet<i16> = current_progress
        .iter()
        .filter_map(|(quest_id, current)| {
            previous_progress
                .get(quest_id)
                .is_some_and(|previous| {
                    current.data != previous.data
                        || current.item_counts.iter().any(|(item_id, count)| {
                            previous
                                .item_counts
                                .iter()
                                .find(|(previous_item_id, _)| previous_item_id == item_id)
                                .is_some_and(|(_, previous_count)| count > previous_count)
                        })
                })
                .then_some(*quest_id)
        })
        .collect();

    order.retain(|quest_id| current_ids.contains(quest_id));
    let ordered_new: Vec<i16> = log
        .started
        .iter()
        .filter(|quest| newly_started.contains(&quest.id))
        .map(|quest| quest.id)
        .collect();
    let ordered_progressed: Vec<i16> = order
        .iter()
        .filter(|quest_id| progressed.contains(quest_id))
        .copied()
        .collect();
    let mut next_order = ordered_new;
    next_order.extend(ordered_progressed);
    next_order.extend(
        order
            .iter()
            .filter(|quest_id| !progressed.contains(quest_id))
            .copied(),
    );
    *order = next_order;

    *previous_progress = current_progress;
}

fn quest_helper_entries(
    log: &openstory_protocol::QuestLog,
    order: &[i16],
    archive: &NxArchive,
    string_archive: &NxArchive,
    font: &fontdue::Font,
    inventory: Option<&openstory_protocol::InventorySnapshot>,
    item_name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<QuestHelperDisplayEntry> {
    let mut started_by_id: HashMap<i16, &openstory_protocol::QuestEntry> =
        log.started.iter().map(|quest| (quest.id, quest)).collect();
    order
        .iter()
        .filter_map(|quest_id| started_by_id.remove(quest_id))
        .filter(|quest| quest_is_displayable(archive, quest.id, font))
        .take(5)
        .map(|quest| {
            let mut objectives = Vec::new();
            for (item_id, required) in quest_requirement_pairs(archive, quest.id, 1, "item") {
                if required == 0 {
                    continue;
                }
                let have = inventory.map_or(0, |snapshot| snapshot.item_count(item_id));
                let name = item_name(item_id, string_archive, item_name_cache);
                objectives.push(format!("{have} / {required} {name}"));
            }
            let mut mob_index = 0;
            for (mob_id, required) in quest_requirement_pairs(archive, quest.id, 1, "mob") {
                let progress_index = mob_index;
                mob_index += 1;
                if required == 0 {
                    continue;
                }
                let killed = quest_progress_counter(&quest.data, progress_index);
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
                objectives.push(format!("{killed} / {required} {name}"));
            }
            if objectives.is_empty() {
                objectives.push("In progress...".to_owned());
            }
            QuestHelperDisplayEntry {
                name: quest_info_text(archive, quest.id, "name")
                    .unwrap_or_else(|| format!("Quest {}", quest.id)),
                objectives,
            }
        })
        .collect()
}

fn render_quest_helper_panel(
    font: &fontdue::Font,
    entries: &[QuestHelperDisplayEntry],
    ui_archive: &NxArchive,
) -> (u32, u32, Vec<u8>) {
    const WIDTH: usize = QUEST_HELPER_PANEL_WIDTH as usize;
    const HEADER_HEIGHT: usize = 25;
    const CONTENT_TOP: usize = 27;
    const ENTRY_NAME_HEIGHT: usize = 21;
    const OBJECTIVE_HEIGHT: usize = 18;
    const ENTRY_GAP: usize = 7;
    const BOTTOM_PADDING: usize = 5;
    const HEADER_CLOSE_BUTTON_X: usize = WIDTH - 14;
    const HEADER_AUTO_BUTTON_X: usize = HEADER_CLOSE_BUTTON_X - 22;
    const HEADER_QUEST_BUTTON_X: usize = HEADER_AUTO_BUTTON_X - 13;

    let content_height = entries.iter().fold(0_usize, |height, entry| {
        height
            .saturating_add(ENTRY_NAME_HEIGHT)
            .saturating_add(entry.objectives.len().saturating_mul(OBJECTIVE_HEIGHT))
            .saturating_add(ENTRY_GAP)
    });
    let height = if entries.is_empty() {
        61
    } else {
        CONTENT_TOP
            .saturating_add(content_height)
            .saturating_add(BOTTOM_PADDING)
    };
    let mut pixels = vec![0_u8; WIDTH * height * 4];

    let _ = composite_nx_ui_bitmap_scaled_to(
        &mut pixels,
        WIDTH,
        height,
        ui_archive,
        "UIWindow.img/QuestAlarm/backgrndmax",
        0,
        0,
        WIDTH,
        HEADER_HEIGHT,
    );
    let _ = composite_nx_ui_bitmap_scaled_to(
        &mut pixels,
        WIDTH,
        height,
        ui_archive,
        "UIWindow.img/QuestAlarm/backgrndcenter",
        0,
        HEADER_HEIGHT,
        WIDTH,
        height.saturating_sub(HEADER_HEIGHT + 5),
    );
    let _ = composite_nx_ui_bitmap_scaled_to(
        &mut pixels,
        WIDTH,
        height,
        ui_archive,
        "UIWindow.img/QuestAlarm/backgrndbottom",
        0,
        height.saturating_sub(5),
        WIDTH,
        5,
    );
    const BACKGROUND_ALPHA_LIFT_PERCENT: u32 = 45;
    // Keep the NX panel colors, but make the composed chrome obscure more of the map.
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);
        pixel[3] = (alpha + (255 - alpha) * BACKGROUND_ALPHA_LIFT_PERCENT / 100) as u8;
    }

    let title = format!("Quest Helper ({}/{})", entries.len(), 5);
    ui_foundation::draw_text_role_at_size(
        font,
        &mut pixels,
        WIDTH,
        height,
        &title,
        5,
        4,
        ui_foundation::UiTextRole::Small,
        10.0,
        WIDTH - 76,
        15,
        Some([27, 27, 23]),
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        height,
        ui_archive,
        "UIWindow.img/QuestAlarm/BtQ/normal/0",
        HEADER_QUEST_BUTTON_X as i32,
        5,
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        height,
        ui_archive,
        "UIWindow.img/QuestAlarm/BtAuto/normal/0",
        HEADER_AUTO_BUTTON_X as i32,
        5,
    );
    let _ = composite_nx_ui_bitmap(
        &mut pixels,
        WIDTH,
        height,
        ui_archive,
        "Basic.img/BtClose/normal/0",
        HEADER_CLOSE_BUTTON_X as i32,
        5,
    );

    let mut y = CONTENT_TOP as i32;
    if entries.is_empty() {
        ui_foundation::draw_text_role_at_size(
            font,
            &mut pixels,
            WIDTH,
            height,
            "No quest tracked",
            15,
            y,
            ui_foundation::UiTextRole::Small,
            12.0,
            WIDTH - 28,
            16,
            Some([75, 72, 65]),
        );
    }
    for entry in entries {
        ui_foundation::draw_text_role_at_size(
            font,
            &mut pixels,
            WIDTH,
            height,
            &entry.name,
            8,
            y,
            ui_foundation::UiTextRole::Label,
            13.0,
            WIDTH - 34,
            ENTRY_NAME_HEIGHT,
            Some([28, 27, 23]),
        );
        let _ = composite_nx_ui_bitmap_scaled_to(
            &mut pixels,
            WIDTH,
            height,
            ui_archive,
            "Basic.img/BtClose2/normal/0",
            WIDTH - 13,
            y as usize + 2,
            10,
            10,
        );
        y += ENTRY_NAME_HEIGHT as i32;
        for objective in &entry.objectives {
            ui_foundation::draw_text_role_at_size(
                font,
                &mut pixels,
                WIDTH,
                height,
                objective,
                11,
                y,
                ui_foundation::UiTextRole::Value,
                12.0,
                WIDTH - 24,
                OBJECTIVE_HEIGHT,
                Some([39, 37, 32]),
            );
            y += OBJECTIVE_HEIGHT as i32;
        }
        y += ENTRY_GAP as i32;
    }

    (WIDTH as u32, height as u32, pixels)
}

const QUEST_SCROLLBAR_WIDTH: usize = 15;
const QUEST_SCROLLBAR_ARROW_HEIGHT: usize = 13;
const QUEST_SCROLLBAR_THUMB_HEIGHT: usize = 25;
const QUEST_DETAIL_BODY_TOP: usize = 129;
const QUEST_DETAIL_TELEPORT_GAP: usize = 4;
const QUEST_DETAIL_BODY_BOTTOM: usize =
    ui_foundation::QUEST_TELEPORT_BUTTON.y as usize - QUEST_DETAIL_TELEPORT_GAP;
const QUEST_DETAIL_ACTIVE_BODY_BOTTOM: usize = 294;
const QUEST_DETAIL_SCROLLBAR_INSET: usize = 5;

fn quest_detail_body_bottom(tab: QuestLogTab, has_reward_choices: bool) -> usize {
    if tab == QuestLogTab::Active && has_reward_choices {
        QUEST_DETAIL_ACTIVE_BODY_BOTTOM
    } else {
        QUEST_DETAIL_BODY_BOTTOM
    }
}

fn draw_quest_panel_scrollbar(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    ui_archive: &NxArchive,
    x: usize,
    y: usize,
    track_height: usize,
    thumb_offset: usize,
) {
    let arrow_height = QUEST_SCROLLBAR_ARROW_HEIGHT;
    let fixed_height = 2 * arrow_height + QUEST_SCROLLBAR_THUMB_HEIGHT;
    if track_height < fixed_height {
        return;
    }
    let _ = composite_nx_ui_bitmap_scaled_to(
        pixels,
        width,
        height,
        ui_archive,
        "Basic.img/VScr4/enabled/base",
        x,
        y + arrow_height,
        QUEST_SCROLLBAR_WIDTH,
        track_height - arrow_height * 2,
    );
    let _ = composite_nx_ui_bitmap(
        pixels,
        width,
        height,
        ui_archive,
        "Basic.img/VScr4/enabled/prev0",
        x as i32,
        y as i32,
    );
    let _ = composite_nx_ui_bitmap(
        pixels,
        width,
        height,
        ui_archive,
        "Basic.img/VScr4/enabled/next0",
        x as i32,
        (y + track_height - arrow_height) as i32,
    );
    let thumb_y = y + arrow_height + thumb_offset.min(track_height - fixed_height);
    let _ = composite_nx_ui_bitmap(
        pixels,
        width,
        height,
        ui_archive,
        "Basic.img/VScr4/enabled/thumb0",
        x as i32,
        thumb_y as i32,
    );
}

fn draw_quest_teleport_button(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    font: &fontdue::Font,
    enabled: bool,
) {
    let button = ui_foundation::QUEST_TELEPORT_BUTTON;
    let x = button.x.round() as usize;
    let y = button.y.round() as usize;
    let button_width = button.width.round() as usize;
    let button_height = button.height.round() as usize;
    let (edge, highlight, face, shadow, text) = if enabled {
        (
            [33, 76, 37],
            [151, 207, 137],
            [76, 157, 70],
            [37, 104, 45],
            [255, 255, 255],
        )
    } else {
        (
            [86, 83, 77],
            [179, 175, 166],
            [139, 135, 126],
            [100, 96, 88],
            [201, 197, 188],
        )
    };
    for dy in 0..button_height {
        for dx in 0..button_width {
            let px = x + dx;
            let py = y + dy;
            if px >= width || py >= height {
                continue;
            }
            let color = if dx == 0 || dx + 1 == button_width || dy == 0 || dy + 1 == button_height {
                edge
            } else if dy == 1 || dx == 1 {
                highlight
            } else if dy + 2 == button_height {
                shadow
            } else {
                face
            };
            let offset = (py * width + px) * 4;
            pixels[offset..offset + 4].copy_from_slice(&[color[2], color[1], color[0], 255]);
        }
    }
    draw_dialogue_text(
        font,
        pixels,
        width,
        height,
        "TELEPORT",
        x as i32 + 4,
        y as i32 + 3,
        8.0,
        button_width.saturating_sub(8),
        button_height.saturating_sub(5),
        text,
    );
}

fn fit_quest_portrait(
    node: &openstory_assets::NxNode,
    max_width: usize,
    max_height: usize,
) -> Option<(usize, usize, Vec<u8>)> {
    let bitmap = node.bitmap().ok().flatten()?;
    let source_width = usize::from(bitmap.width());
    let source_height = usize::from(bitmap.height());
    if source_width == 0 || source_height == 0 || max_width == 0 || max_height == 0 {
        return None;
    }
    let source = bitmap.decode_bgra_shared().ok()?;
    if source.len() < source_width.saturating_mul(source_height).saturating_mul(4) {
        return None;
    }

    let mut min_x = source_width;
    let mut min_y = source_height;
    let mut max_x = 0;
    let mut max_y = 0;
    for y in 0..source_height {
        for x in 0..source_width {
            if source[(y * source_width + x) * 4 + 3] != 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if min_x > max_x || min_y > max_y {
        return None;
    }

    let visible_width = max_x - min_x + 1;
    let visible_height = max_y - min_y + 1;
    let scale = (max_width as f32 / visible_width as f32)
        .min(max_height as f32 / visible_height as f32)
        .min(1.0);
    let target_width = ((visible_width as f32 * scale).round() as usize).clamp(1, max_width);
    let target_height = ((visible_height as f32 * scale).round() as usize).clamp(1, max_height);
    let mut target = vec![0; target_width * target_height * 4];
    for y in 0..target_height {
        let source_y = min_y + y * visible_height / target_height;
        for x in 0..target_width {
            let source_x = min_x + x * visible_width / target_width;
            let source_index = (source_y * source_width + source_x) * 4;
            let target_index = (y * target_width + x) * 4;
            target[target_index..target_index + 4]
                .copy_from_slice(&source[source_index..source_index + 4]);
        }
    }
    Some((target_width, target_height, target))
}

const QUEST_FORFEIT_DIALOG_WIDTH: f32 = 316.0;
const QUEST_FORFEIT_DIALOG_HEIGHT: f32 = 140.0;

fn quest_forfeit_dialog_bounds() -> ui_foundation::UiRect {
    ui_foundation::UiRect::new(
        (QUEST_LOG_PANEL_WIDTH as f32 - QUEST_FORFEIT_DIALOG_WIDTH) * 0.5,
        (QUEST_LOG_PANEL_HEIGHT as f32 - QUEST_FORFEIT_DIALOG_HEIGHT) * 0.5,
        QUEST_FORFEIT_DIALOG_WIDTH,
        QUEST_FORFEIT_DIALOG_HEIGHT,
    )
}

fn quest_forfeit_close_button() -> ui_foundation::UiRect {
    let dialog = quest_forfeit_dialog_bounds();
    let close = ui_dialog::inventory_close(QUEST_FORFEIT_DIALOG_WIDTH);
    ui_foundation::UiRect::new(
        dialog.x + close.x,
        dialog.y + close.y,
        close.width,
        close.height,
    )
}

fn quest_forfeit_action_buttons() -> [ui_foundation::UiRect; 2] {
    let dialog = quest_forfeit_dialog_bounds();
    let button_y = dialog.y + QUEST_FORFEIT_DIALOG_HEIGHT - 24.0;
    let first_x = dialog.x + (QUEST_FORFEIT_DIALOG_WIDTH - 47.0 * 2.0 - 6.0) * 0.5;
    [
        ui_foundation::UiRect::new(first_x, button_y, 47.0, 18.0),
        ui_foundation::UiRect::new(first_x + 53.0, button_y, 47.0, 18.0),
    ]
}

fn render_quest_forfeit_dialog(
    pixels: &mut [u8],
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    quest_name: &str,
) {
    let bounds = quest_forfeit_dialog_bounds();
    let close = quest_forfeit_close_button();
    let mut dialog = NxDialog::new(
        pixels,
        QUEST_LOG_PANEL_WIDTH as usize,
        QUEST_LOG_PANEL_HEIGHT as usize,
        ui_archive,
    );
    let _ = dialog.inventory_window(bounds, font, "Forfeit Quest", close);
    let [accept, cancel] = quest_forfeit_action_buttons();
    let _ = dialog.button(
        accept,
        ui_dialog::NxButton::new("Basic.img/BtYes2", ui_dialog::ButtonState::Normal),
    );
    let _ = dialog.button(
        cancel,
        ui_dialog::NxButton::new("Basic.img/BtCancel2", ui_dialog::ButtonState::Normal),
    );
    drop(dialog);

    draw_dialogue_text(
        font,
        pixels,
        QUEST_LOG_PANEL_WIDTH as usize,
        QUEST_LOG_PANEL_HEIGHT as usize,
        &format!("Forfeit quest {quest_name}?"),
        bounds.x as i32 + 20,
        bounds.y as i32 + 48,
        12.0,
        bounds.width as usize - 40,
        48,
        [55, 50, 42],
    );
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
    detail_open: bool,
    scroll: usize,
    detail_scroll: usize,
    selected_reward: usize,
    character: Option<&openstory_protocol::CharacterInfo>,
    confirmation: Option<i16>,
    ui_archive: &NxArchive,
    npc_archive: &NxArchive,
) -> (Vec<u8>, usize) {
    const WIDTH: usize = QUEST_LOG_PANEL_WIDTH as usize;
    const HEIGHT: usize = QUEST_LOG_PANEL_HEIGHT as usize;
    const LIST_WIDTH: usize = QUEST_LOG_LIST_WIDTH as usize;
    const LIST_TEXT_WIDTH: usize = LIST_WIDTH.saturating_sub(18) - 31;
    const DETAIL_X: usize = QUEST_LOG_DETAIL_X;
    const VISIBLE_ROWS: usize = QUEST_LOG_VISIBLE_ROWS;
    const SCROLLBAR_INSET: usize = 3;
    const LIST_SCROLLBAR_Y: usize = 50;
    const LIST_SCROLLBAR_HEIGHT: usize = 319;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    let ids = quest_ids_for_tab(log, available_quests, tab, archive, font);
    let selected_quest = detail_open.then(|| ids.get(selected).copied()).flatten();
    let mut dialog = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui_archive);
    if selected_quest.is_some() {
        let _ = dialog.authored_window(
            &[("UIWindow.img/Quest/backgrnd2", DETAIL_X as i32, 0)],
            ui_foundation::QUEST_DETAIL_CLOSE_BUTTON,
        );
        dialog.title(
            font,
            "Quest Details",
            ui_foundation::UiRect::new(DETAIL_X as f32 + 10.0, 4.0, 200.0, 12.0),
        );
    }
    // Paint the list after the detail backdrop so its right bevel stays in front.
    let _ = dialog.authored_window(
        &[("UIWindow.img/Quest/backgrnd", 0, 0)],
        ui_foundation::QUEST_LIST_CLOSE_BUTTON,
    );
    dialog.replace_title(
        font,
        "Quest Log",
        ui_foundation::UiRect::new(7.0, 5.0, 183.0, 12.0),
    );
    let active = tab == QuestLogTab::Active;
    let available = tab == QuestLogTab::Available;
    let completed = tab == QuestLogTab::Completed;
    for (index, (selected, label)) in [
        (available, "Available"),
        (active, "In Progress"),
        (completed, "Complete"),
    ]
    .into_iter()
    .enumerate()
    {
        let _ = dialog.labeled_tab(ui_foundation::quest_tab(index), selected, font, label);
    }

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
            let y = ui_foundation::QUEST_GRID.cell(visible).y as usize;
            let icon_path = match tab {
                QuestLogTab::Available => "UIWindow.img/Quest/icon0",
                QuestLogTab::Active => "UIWindow.img/Quest/icon2/0",
                QuestLogTab::Completed => "UIWindow.img/Quest/icon4",
            };
            let _ = composite_nx_ui_bitmap(
                &mut pixels,
                WIDTH,
                HEIGHT,
                ui_archive,
                icon_path,
                11,
                y as i32 + 4,
            );
            let name =
                quest_info_text(archive, id, "name").unwrap_or_else(|| "Unknown quest".to_owned());
            let name = ui_foundation::ellipsize(font, &name, 11.0, LIST_TEXT_WIDTH as f32);
            let name_font = if index == selected {
                ui_foundation::bold_font().unwrap_or(font)
            } else {
                font
            };
            draw_dialogue_text(
                name_font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &name,
                31,
                y as i32 + 1,
                11.0,
                LIST_TEXT_WIDTH,
                18,
                if index == selected {
                    [34, 72, 112]
                } else {
                    [28, 28, 28]
                },
            );
        }
    }
    if ids.len() > VISIBLE_ROWS {
        let max_scroll = ids.len().saturating_sub(VISIBLE_ROWS);
        let max_thumb_offset = LIST_SCROLLBAR_HEIGHT
            - (QUEST_SCROLLBAR_ARROW_HEIGHT * 2 + QUEST_SCROLLBAR_THUMB_HEIGHT);
        let thumb_offset = if max_scroll == 0 {
            0
        } else {
            (max_thumb_offset as f32 * scroll.min(max_scroll) as f32 / max_scroll as f32).round()
                as usize
        };
        draw_quest_panel_scrollbar(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ui_archive,
            LIST_WIDTH - QUEST_SCROLLBAR_WIDTH - SCROLLBAR_INSET,
            LIST_SCROLLBAR_Y,
            LIST_SCROLLBAR_HEIGHT,
            thumb_offset,
        );
    }

    let mut detail_scroll_max = 0;
    if let Some(id) = selected_quest {
        let text_x = DETAIL_X as i32 + 18;
        let scrollbar_x = DETAIL_X + QUEST_LOG_DETAIL_WIDTH as usize
            - QUEST_SCROLLBAR_WIDTH
            - QUEST_DETAIL_SCROLLBAR_INSET;
        let text_width = scrollbar_x.saturating_sub(text_x as usize + 7);
        let name =
            quest_info_text(archive, id, "name").unwrap_or_else(|| "Unknown quest".to_owned());
        let name = ui_foundation::ellipsize(font, &name, 11.0, 150.0);
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &name,
            DETAIL_X as i32 + 39,
            41,
            11.0,
            150,
            26,
            [255, 255, 255],
        );
        let state = match tab {
            QuestLogTab::Available => "Available",
            QuestLogTab::Active => "In progress",
            QuestLogTab::Completed => "Completed",
        };
        draw_dialogue_text(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            state,
            DETAIL_X as i32 + 39,
            63,
            10.0,
            150,
            15,
            [225, 235, 244],
        );
        if let Some(level) = archive
            .root()
            .resolve(&format!("Check.img/{id}/0"))
            .ok()
            .flatten()
            .and_then(|node| quest_node_integer(&node, "lvmin"))
            .filter(|level| *level > 0)
        {
            draw_dialogue_text(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                &format!("Level {level}+"),
                DETAIL_X as i32 + 39,
                78,
                10.0,
                150,
                15,
                [225, 235, 244],
            );
        }

        let preferred_dialogue_side = if available { 0 } else { 1 };
        let dialogue = [preferred_dialogue_side, 1 - preferred_dialogue_side]
            .into_iter()
            .find_map(|side| {
                quest_dialogue_pages(archive, &format!("Say.img/{id}/{side}"))
                    .into_iter()
                    .next()
                    .map(|line| {
                        (
                            clean_npc_markup(&resolve_quest_dialogue_references(
                                &line,
                                string_archive,
                            )),
                            side,
                        )
                    })
            })
            .filter(|(line, _)| !line.trim().is_empty());
        let npc_side = dialogue
            .as_ref()
            .map_or(preferred_dialogue_side, |(_, side)| *side);
        let npc_id = [npc_side, 1 - npc_side].into_iter().find_map(|side| {
            archive
                .root()
                .resolve(&format!("Check.img/{id}/{side}"))
                .ok()
                .flatten()
                .and_then(|node| quest_node_integer(&node, "npc"))
                .filter(|npc_id| *npc_id > 0)
        });
        if let Some(npc_id) = npc_id {
            const PORTRAIT_BOX: usize = 60;
            let path = format!("{npc_id:07}.img/stand/0");
            if let Ok(Some(npc)) = npc_archive.root().resolve(&path) {
                if let Some((portrait_width, portrait_height, portrait)) =
                    fit_quest_portrait(&npc, PORTRAIT_BOX, PORTRAIT_BOX)
                {
                    let x = DETAIL_X + 200 + (PORTRAIT_BOX - portrait_width) / 2;
                    let y = 32 + (PORTRAIT_BOX - portrait_height) / 2;
                    composite_bgra_pixels(
                        &mut pixels,
                        WIDTH,
                        HEIGHT,
                        &portrait,
                        portrait_width,
                        portrait_height,
                        x as i32,
                        y as i32,
                    );
                }
            }
            if let Some(name) = string_archive
                .root()
                .resolve(&format!("Npc.img/{npc_id}/name"))
                .ok()
                .flatten()
                .and_then(|node| match node.value().ok()? {
                    NodeValue::String(name) if !name.trim().is_empty() => {
                        Some(clean_npc_markup(&name))
                    }
                    _ => None,
                })
            {
                let name = ui_foundation::ellipsize(font, &name, 9.0, 110.0);
                let portrait_center_x = DETAIL_X as f32 + 200.0 + PORTRAIT_BOX as f32 / 2.0;
                let label_x = (portrait_center_x
                    - ui_foundation::measure_text(font, &name, 9.0) / 2.0)
                    .round() as i32;
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    &name,
                    label_x,
                    94,
                    9.0,
                    110,
                    13,
                    [225, 196, 122],
                );
            }
        }
        let description = if active || completed {
            quest_info_text(archive, id, "1")
                .filter(|description| quest_description_is_displayable(description))
                .or_else(|| {
                    quest_info_text(archive, id, "0")
                        .filter(|description| quest_description_is_displayable(description))
                })
        } else {
            quest_info_text(archive, id, "0")
                .filter(|description| quest_description_is_displayable(description))
        };
        let description = description.map(|description| clean_quest_detail_text(&description));
        let reward_choices = if active {
            character.map_or_else(Vec::new, |character| {
                quest_reward_choices(id, archive, string_archive, character, item_name_cache)
            })
        } else {
            Vec::new()
        };
        let body_bottom = quest_detail_body_bottom(tab, !reward_choices.is_empty());
        let body_height = body_bottom.saturating_sub(QUEST_DETAIL_BODY_TOP);
        let dialogue_text = dialogue.map(|(line, _)| format!("“{line}”"));
        let dialogue_height = dialogue_text.as_deref().map_or(0, |text| {
            dialogue_text_content_height(font, text, 10.0, text_width, &EMPTY_NPC_INLINE_ICONS)
        });
        let dialogue_bottom = QUEST_DETAIL_BODY_TOP.saturating_add(dialogue_height);
        let description_y = if dialogue_text.is_some() {
            dialogue_bottom.saturating_add(8)
        } else {
            QUEST_DETAIL_BODY_TOP
        };
        let description_height = description.as_deref().map_or(0, |text| {
            dialogue_text_content_height(font, text, 10.0, text_width, &EMPTY_NPC_INLINE_ICONS)
        });
        let description_bottom = description_y.saturating_add(description_height);
        let intro_bottom = description_bottom.max(dialogue_bottom);
        let note = if completed {
            quest_info_text(archive, id, "2")
                .filter(|note| quest_description_is_displayable(note))
                .map(|note| clean_quest_detail_text(&note))
        } else {
            None
        };
        let note_y = (intro_bottom + 8).max(220) as i32;
        let note_height = note.as_deref().map_or(0, |text| {
            dialogue_text_content_height(font, text, 10.0, text_width, &EMPTY_NPC_INLINE_ICONS)
        });
        let objective_heading_y = (intro_bottom + 8).max(177) as i32;
        let objectives = if completed {
            Vec::new()
        } else {
            quest_objective_lines(
                log,
                id,
                tab,
                archive,
                string_archive,
                inventory,
                item_name_cache,
            )
        };
        let mut objective_rows = Vec::new();
        let mut content_bottom = intro_bottom;
        if completed {
            content_bottom = content_bottom.max(note_y as usize + note_height);
        } else {
            content_bottom = content_bottom.max(objective_heading_y as usize + 15);
            let mut row_y = objective_heading_y + 20;
            if objectives.is_empty() {
                let empty_label = if available {
                    "No listed requirements.".to_owned()
                } else {
                    "No tracked objectives.".to_owned()
                };
                let row_height = dialogue_text_content_height(
                    font,
                    &empty_label,
                    10.0,
                    text_width,
                    &EMPTY_NPC_INLINE_ICONS,
                )
                .max(16);
                content_bottom = content_bottom.max(row_y as usize + row_height);
                objective_rows.push((empty_label, row_y, row_height, [79, 74, 66]));
            } else {
                for (objective, done) in objectives.iter().take(5) {
                    let prefix = if available {
                        "· "
                    } else if *done {
                        "✓ "
                    } else {
                        "○ "
                    };
                    let text = format!("{prefix}{objective}");
                    let row_height = dialogue_text_content_height(
                        font,
                        &text,
                        9.0,
                        text_width,
                        &EMPTY_NPC_INLINE_ICONS,
                    )
                    .max(13);
                    let color = if *done { [62, 112, 57] } else { [57, 53, 45] };
                    content_bottom = content_bottom.max(row_y as usize + row_height);
                    objective_rows.push((text, row_y, row_height, color));
                    row_y += row_height as i32 + 2;
                }
            }
        }
        let content_height = content_bottom.saturating_sub(QUEST_DETAIL_BODY_TOP);
        detail_scroll_max = content_height.saturating_sub(body_height);
        let detail_scroll = detail_scroll.min(detail_scroll_max);
        if let Some(dialogue) = dialogue_text.as_deref() {
            draw_dialogue_text_with_icons_scrolled_in_viewport(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                dialogue,
                text_x,
                QUEST_DETAIL_BODY_TOP as i32,
                QUEST_DETAIL_BODY_TOP as i32,
                10.0,
                text_width,
                body_height,
                [49, 75, 93],
                &EMPTY_NPC_INLINE_ICONS,
                detail_scroll,
            );
        }
        if let Some(description) = description.as_deref() {
            draw_dialogue_text_with_icons_scrolled_in_viewport(
                font,
                &mut pixels,
                WIDTH,
                HEIGHT,
                description,
                text_x,
                description_y as i32,
                QUEST_DETAIL_BODY_TOP as i32,
                10.0,
                text_width,
                body_height,
                [73, 69, 61],
                &EMPTY_NPC_INLINE_ICONS,
                detail_scroll,
            );
        }
        if completed {
            if let Some(note) = note.as_deref() {
                draw_dialogue_text_with_icons_scrolled_in_viewport(
                    font,
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    note,
                    text_x,
                    note_y,
                    QUEST_DETAIL_BODY_TOP as i32,
                    10.0,
                    text_width,
                    body_height,
                    [73, 69, 61],
                    &EMPTY_NPC_INLINE_ICONS,
                    detail_scroll,
                );
            }
        } else {
            draw_dialogue_text_with_icons_scrolled_in_viewport(
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
                objective_heading_y,
                QUEST_DETAIL_BODY_TOP as i32,
                10.0,
                text_width,
                body_height,
                [49, 75, 93],
                &EMPTY_NPC_INLINE_ICONS,
                detail_scroll,
            );
            for (text, y, _row_height, color) in objective_rows {
                draw_dialogue_text_with_icons_scrolled_in_viewport(
                    font,
                    &mut pixels,
                    WIDTH,
                    HEIGHT,
                    &text,
                    text_x,
                    y,
                    QUEST_DETAIL_BODY_TOP as i32,
                    9.0,
                    text_width,
                    body_height,
                    color,
                    &EMPTY_NPC_INLINE_ICONS,
                    detail_scroll,
                );
            }
        }
        if detail_scroll_max > 0 {
            let max_thumb_offset = body_height
                .saturating_sub(QUEST_SCROLLBAR_ARROW_HEIGHT * 2 + QUEST_SCROLLBAR_THUMB_HEIGHT);
            let thumb_offset = (max_thumb_offset as f32 * detail_scroll as f32
                / detail_scroll_max as f32)
                .round() as usize;
            draw_quest_panel_scrollbar(
                &mut pixels,
                WIDTH,
                HEIGHT,
                ui_archive,
                scrollbar_x,
                QUEST_DETAIL_BODY_TOP,
                body_height,
                thumb_offset,
            );
        }
        if active {
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
        let teleport_enabled = active
            && log
                .started
                .iter()
                .find(|quest| quest.id == id)
                .is_some_and(|quest| quest_is_completable(quest, archive, inventory, log));
        draw_quest_teleport_button(&mut pixels, WIDTH, HEIGHT, font, teleport_enabled);
        let forfeit_path = if active {
            "UIWindow.img/Quest/BtGiveup/normal/0"
        } else {
            "UIWindow.img/Quest/BtGiveup/disabled/0"
        };
        let buttons = [
            (
                "UIWindow.img/Quest/BtAlert/normal/0",
                ui_foundation::QUEST_HELPER_BUTTON,
            ),
            (forfeit_path, ui_foundation::QUEST_FORFEIT_BUTTON),
        ];
        let _ = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui_archive).buttons(&buttons);
    }
    if let Some(id) = confirmation {
        let quest_name =
            quest_info_text(archive, id, "name").unwrap_or_else(|| "this quest".to_owned());
        render_quest_forfeit_dialog(&mut pixels, font, ui_archive, &quest_name);
    }
    (pixels, detail_scroll_max)
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

fn status_auto_assign_plan(
    character: &openstory_protocol::CharacterInfo,
) -> [(openstory_protocol::AbilityStat, u16); 4] {
    use openstory_protocol::AbilityStat as Stat;

    let points = character.stats.ability_points;
    let level = u16::from(character.stats.level);
    match character.stats.job / 100 {
        5 if character.stats.dexterity > character.stats.strength => {
            let strength_points = points.min(level.saturating_sub(character.stats.strength));
            [
                (Stat::Strength, strength_points),
                (Stat::Dexterity, points - strength_points),
                (Stat::Intelligence, 0),
                (Stat::Luck, 0),
            ]
        }
        5 => {
            let dexterity_points = points.min(level.saturating_sub(character.stats.dexterity));
            [
                (Stat::Dexterity, dexterity_points),
                (Stat::Strength, points - dexterity_points),
                (Stat::Intelligence, 0),
                (Stat::Luck, 0),
            ]
        }
        4 | 14 => {
            let target_dexterity = level.saturating_mul(2);
            let dexterity_points =
                points.min(target_dexterity.saturating_sub(character.stats.dexterity));
            [
                (Stat::Dexterity, dexterity_points),
                (Stat::Luck, points - dexterity_points),
                (Stat::Strength, 0),
                (Stat::Intelligence, 0),
            ]
        }
        2 | 12 | 22 => {
            let luck_points =
                points.min(level.saturating_add(3).saturating_sub(character.stats.luck));
            [
                (Stat::Luck, luck_points),
                (Stat::Intelligence, points - luck_points),
                (Stat::Strength, 0),
                (Stat::Dexterity, 0),
            ]
        }
        1 | 11 | 21 => {
            let target_dexterity = level.saturating_mul(2).min(80);
            let dexterity_points =
                points.min(target_dexterity.saturating_sub(character.stats.dexterity));
            [
                (Stat::Dexterity, dexterity_points),
                (Stat::Strength, points - dexterity_points),
                (Stat::Intelligence, 0),
                (Stat::Luck, 0),
            ]
        }
        3 | 13 => {
            let strength_points = points.min(
                level
                    .saturating_add(5)
                    .saturating_sub(character.stats.strength),
            );
            [
                (Stat::Strength, strength_points),
                (Stat::Dexterity, points - strength_points),
                (Stat::Intelligence, 0),
                (Stat::Luck, 0),
            ]
        }
        _ => {
            let mut allocations = [
                (Stat::Strength, 0),
                (Stat::Dexterity, 0),
                (Stat::Intelligence, 0),
                (Stat::Luck, 0),
            ];
            match primary_ability_stat(character) {
                Stat::Strength => allocations[0].1 = points,
                Stat::Dexterity => allocations[1].1 = points,
                Stat::Intelligence => allocations[2].1 = points,
                Stat::Luck => allocations[3].1 = points,
                Stat::Hp | Stat::Mp => {}
            }
            allocations
        }
    }
}

fn status_auto_assign_additions(character: &openstory_protocol::CharacterInfo) -> [u16; 4] {
    let mut additions = [0; 4];
    for (stat, amount) in status_auto_assign_plan(character) {
        let index = match stat {
            openstory_protocol::AbilityStat::Strength => Some(0),
            openstory_protocol::AbilityStat::Dexterity => Some(1),
            openstory_protocol::AbilityStat::Intelligence => Some(2),
            openstory_protocol::AbilityStat::Luck => Some(3),
            openstory_protocol::AbilityStat::Hp | openstory_protocol::AbilityStat::Mp => None,
        };
        if let Some(index) = index {
            additions[index] = amount;
        }
    }
    additions
}

fn character_job_name(job: u16) -> String {
    // Use the actual advancement names for known jobs. The branch name plus
    // numeric ID fallback below obscures second, third, and fourth jobs.
    let advancement_name = match job {
        100 => "Swordsman",
        110 => "Fighter",
        111 => "Crusader",
        112 => "Hero",
        120 => "Page",
        121 => "White Knight",
        122 => "Paladin",
        130 => "Spearman",
        131 => "Dragon Knight",
        132 => "Dark Knight",
        200 => "Magician",
        210 => "Wizard (F/P)",
        211 => "Mage (F/P)",
        212 => "Archmage (F/P)",
        220 => "Wizard (I/L)",
        221 => "Mage (I/L)",
        222 => "Archmage (I/L)",
        230 => "Cleric",
        231 => "Priest",
        232 => "Bishop",
        300 => "Archer",
        310 => "Hunter",
        311 => "Ranger",
        312 => "Bowmaster",
        320 => "Crossbowman",
        321 => "Sniper",
        322 => "Marksman",
        400 => "Rogue",
        410 => "Assassin",
        411 => "Hermit",
        412 => "Night Lord",
        420 => "Bandit",
        421 => "Chief Bandit",
        422 => "Shadower",
        500 => "Pirate",
        510 => "Brawler",
        511 => "Marauder",
        512 => "Buccaneer",
        520 => "Gunslinger",
        521 => "Outlaw",
        522 => "Corsair",
        600 => "Sage",
        610 => "Elementalist",
        611 => "Arcanum",
        612 => "Archsage",
        700 => "Necromancer",
        710 => "Dark Acolyte",
        711 => "Soul Reaper",
        712 => "Lich King",
        800 => "Summoner",
        810 => "Beastmaster",
        811 => "Warcaller",
        812 => "Primal Alpha",
        910 => "SuperGM",
        1100 | 1110 | 1111 | 1112 => "Dawn Warrior",
        1200 | 1210 | 1211 | 1212 => "Blaze Wizard",
        1300 | 1310 | 1311 | 1312 => "Wind Archer",
        1400 | 1410 | 1411 | 1412 => "Night Walker",
        1500 | 1510 | 1511 | 1512 => "Thunder Breaker",
        2100 | 2110 | 2111 | 2112 => "Aran",
        2001 | 2200..=2218 | 3000 => "Evan",
        _ => "",
    };
    if !advancement_name.is_empty() {
        return advancement_name.to_owned();
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
    if job == 0 || job == 1000 || job == 2000 || job == 900 {
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
) -> Vec<u8> {
    const WIDTH: usize = STATUS_PANEL_WIDTH as usize;
    const HEIGHT: usize = STATUS_PANEL_HEIGHT as usize;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    use ui_dialog::{ButtonState, NxButton};

    let mut dialog = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui_archive);
    let _ = dialog.authored_window(
        &[("UIWindow.img/Stat/backgrnd", 0, 0)],
        ui_foundation::STATUS_CLOSE_BUTTON,
    );
    dialog.replace_title(
        font,
        "Character Stats",
        ui_foundation::UiRect::new(7.0, 5.0, 142.0, 12.0),
    );
    let auto_state = if character.stats.ability_points > 0 && !waiting_for_stats {
        ButtonState::Normal
    } else {
        ButtonState::Disabled
    };
    let _ = dialog.button(
        ui_foundation::STATUS_AUTO_BUTTON,
        NxButton::new("UIWindow.img/Stat/BtAuto", auto_state),
    );
    let _ = dialog.button(
        ui_foundation::STATUS_DETAIL_BUTTON,
        NxButton::new("UIWindow.img/Stat/BtDetail", ButtonState::Normal),
    );

    let stats = &character.stats;
    let color = [56, 42, 31];
    let values = [
        (stats.name.clone(), 59, 31),
        (character_job_name(stats.job), 59, 48),
        (stats.level.to_string(), 59, 77),
        ("-".to_owned(), 59, 95),
        (format!("{} / {}", stats.hp, stats.max_hp), 59, 112),
        (format!("{} / {}", stats.mp, stats.max_mp), 59, 130),
        (stats.experience.to_string(), 59, 149),
        (stats.fame.to_string(), 59, 167),
        (stats.strength.to_string(), 59, 244),
        (stats.dexterity.to_string(), 59, 262),
        (stats.intelligence.to_string(), 59, 280),
        (stats.luck.to_string(), 59, 298),
    ];
    for (value, x, y) in values {
        let has_ap_button =
            stats.ability_points > 0 && matches!(y, 112 | 130 | 244 | 262 | 280 | 298);
        let text_width = if has_ap_button { 92 } else { 111 };
        let value = ui_foundation::ellipsize(
            font,
            &value,
            ui_foundation::UiTextRole::Value.style().size,
            text_width as f32,
        );
        ui_foundation::draw_text_role(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &value,
            x,
            y,
            ui_foundation::UiTextRole::Value,
            text_width,
            16,
            Some(color),
        );
    }
    if stats.ability_points > 0 {
        let state = if waiting_for_stats {
            ButtonState::Disabled
        } else {
            ButtonState::Normal
        };
        let button = NxButton::new("UIWindow.img/Stat/BtApUp", state);
        let mut dialog = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui_archive);
        for (_, rect) in ui_foundation::status_ap_buttons() {
            let _ = dialog.button(rect, button);
        }
    }
    let ability_points = stats.ability_points.to_string();
    let emphasis_style = ui_foundation::UiTextRole::Emphasis.style();
    let ability_points_width =
        dialogue_text_width(font, &ability_points, emphasis_style.size).ceil() as i32 + 1;
    ui_foundation::draw_text_role(
        font,
        &mut pixels,
        WIDTH,
        HEIGHT,
        &ability_points,
        61 + (29 - ability_points_width) / 2,
        214,
        ui_foundation::UiTextRole::Emphasis,
        29,
        15,
        Some(color),
    );

    pixels
}

const STATUS_AUTO_ASSIGN_DIALOG_WIDTH: usize = 190;
const STATUS_AUTO_ASSIGN_DIALOG_HEIGHT: usize = 154;
const STATUS_AUTO_ASSIGN_CLOSE: ui_foundation::UiRect =
    ui_dialog::inventory_close(STATUS_AUTO_ASSIGN_DIALOG_WIDTH as f32);

fn status_auto_assign_buttons() -> [ui_foundation::UiRect; 2] {
    ui_dialog::right_aligned_footer(
        STATUS_AUTO_ASSIGN_DIALOG_WIDTH as f32 - 8.0,
        STATUS_AUTO_ASSIGN_DIALOG_HEIGHT as f32,
        [47.0, 47.0],
    )
}

fn render_status_auto_assign_dialog(
    font: &fontdue::Font,
    ui_archive: &NxArchive,
    character: &openstory_protocol::CharacterInfo,
) -> Result<Vec<u8>, String> {
    use ui_dialog::{ButtonState, NxButton};
    use ui_foundation::{UiRect, UiTextRole, draw_text_role};

    let width = STATUS_AUTO_ASSIGN_DIALOG_WIDTH;
    let height = STATUS_AUTO_ASSIGN_DIALOG_HEIGHT;
    let mut pixels = vec![0; width * height * 4];
    let mut dialog = NxDialog::new(&mut pixels, width, height, ui_archive);
    dialog.inventory_window(
        UiRect::new(0.0, 0.0, width as f32, height as f32),
        font,
        "Auto-Assign Ability Points",
        STATUS_AUTO_ASSIGN_CLOSE,
    )?;
    let [accept, cancel] = status_auto_assign_buttons();
    dialog.button(
        accept,
        NxButton::new("Basic.img/BtOK2", ButtonState::Normal),
    )?;
    dialog.button(
        cancel,
        NxButton::new("Basic.img/BtCancel2", ButtonState::Normal),
    )?;

    let additions = status_auto_assign_additions(character);
    for (index, (label, amount)) in ["STR", "DEX", "INT", "LUK"]
        .into_iter()
        .zip(additions)
        .enumerate()
    {
        draw_text_role(
            font,
            &mut pixels,
            width,
            height,
            label,
            18,
            36 + index as i32 * 19,
            UiTextRole::Label,
            60,
            16,
            None,
        );
        draw_text_role(
            font,
            &mut pixels,
            width,
            height,
            &format!("+{amount}"),
            96,
            36 + index as i32 * 19,
            UiTextRole::Value,
            70,
            16,
            None,
        );
    }
    Ok(pixels)
}

fn status_auto_assign_dialog_rect() -> SpriteRect {
    SpriteRect {
        x: (ui_foundation::LOGICAL_WIDTH - STATUS_AUTO_ASSIGN_DIALOG_WIDTH as f32) * 0.5,
        y: (ui_foundation::LOGICAL_HEIGHT - STATUS_AUTO_ASSIGN_DIALOG_HEIGHT as f32) * 0.5,
        width: STATUS_AUTO_ASSIGN_DIALOG_WIDTH as f32,
        height: STATUS_AUTO_ASSIGN_DIALOG_HEIGHT as f32,
        blend_mode: BlendMode::Alpha,
    }
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
    active_buffs: &CharacterBuffs,
    forced_stats: &ForcedEquipmentStats,
) -> Vec<u8> {
    const WIDTH: usize = STATUS_DETAIL_WIDTH as usize;
    const HEIGHT: usize = STATUS_DETAIL_HEIGHT as usize;
    let mut pixels = vec![0_u8; WIDTH * HEIGHT * 4];
    let _ = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui_archive).frame(
        "UIWindow.img/Stat/backgrnd2",
        0,
        0,
    );
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
            9,
        ),
        (weapon_defense.to_string(), 27),
        (combat.magic.to_string(), 45),
        (magic_defense.to_string(), 63),
        (combat.accuracy.to_string(), 81),
        (avoidability.to_string(), 99),
        (hands.to_string(), 117),
        (format!("{}%", combat.speed), 135),
        (format!("{}%", combat.jump), 153),
    ];
    for (value, y) in rows {
        ui_foundation::draw_text_role(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            &value,
            84,
            y,
            ui_foundation::UiTextRole::Value,
            WIDTH - 85,
            16,
            Some([48, 41, 33]),
        );
    }
    pixels
}

const KEY_CONFIG_ACTIONS: [(&str, u8, i32, bool); 45] = [
    ("Equipment", 4, 0, true),
    ("Items", 4, 1, true),
    ("Stats", 4, 2, true),
    ("Skills", 4, 3, true),
    ("Friends", 4, 4, true),
    ("World map", 4, 5, true),
    ("Messenger", 4, 6, true),
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
    ("Events", 4, 32, true),
    ("Family (extended)", 4, 42, true),
    ("Guild BBS", 4, 43, true),
    ("Alliance", 4, 44, true),
    ("Pet equipment", 4, 45, true),
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
                    let label = format!("Macro #{macro_index} (unavailable)");
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
    for (index, name) in skill_macros::NAMES.iter().enumerate() {
        let action = index as i32;
        macro_actions.insert(
            action,
            KeyConfigAction {
                label: format!("Macro: {name}"),
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
    keyboard_settings::defaults()
}

#[cfg(test)]
mod npc_quest_asset_tests {
    use super::*;

    #[test]
    #[ignore = "requires the existing local NX data directory"]
    fn npc_quest_tutorial_assets_offer_current_quests_and_reminders() {
        let directory = AssetDirectory::resolve_and_validate(None).unwrap();
        let quests = NxArchive::open(directory.root().join("Quest.nx")).unwrap();
        // The old GM-clothes tutorials must not mask the normal starter quests.
        assert!(!quest_start_items_met(&quests, 1000, None));
        assert!(!quest_start_items_met(&quests, 1018, None));
        assert!(quest_start_items_met(&quests, 1031, None));
        assert!(quest_start_items_met(&quests, 1035, None));
        let intro = quest_dialogue_pages(&quests, "Say.img/1031/0");
        assert!(!intro.is_empty());
        let todd = quest_dialogue_pages(&quests, "Say.img/1035/0");
        assert_eq!(todd.len(), 2);
        assert!(todd[0].contains("Todd"));
        assert!(todd[1].contains("Ctrl"));
        assert!(!quest_dialogue_pages(&quests, "Say.img/1035/0/yes").is_empty());
        assert!(!quest_dialogue_pages(&quests, "Say.img/1035/1/stop/mob").is_empty());
        assert!(quest_dialogue_pages(&quests, "Say.img/missing/0").is_empty());
        let quest = openstory_protocol::QuestEntry {
            id: 1031,
            data: String::new(),
        };
        assert!(quest_is_completable(
            &quest,
            &quests,
            None,
            &openstory_protocol::QuestLog::default()
        ));
    }
}
