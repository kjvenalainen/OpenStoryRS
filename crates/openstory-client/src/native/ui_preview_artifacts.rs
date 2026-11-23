// Reference crops and comparison output belong to the offline harness only.

/// The source rectangle is in the logical composition. Reference dimensions
/// belong to comparison output, never to live layout or pointer coordinates.
fn reference_crop(
    name: &str,
    scene: Option<&LoginSceneAssets>,
) -> Option<(u32, u32, u32, u32, u32, u32)> {
    let rect = match name {
        "hud-bottom-bar" => (0, 697, 804, 71, 804, 71),
        "hud-gain-feed" => (480, 560, 544, 135, 544, 135),
        "hud-chat" => (
            CHAT_X as u32,
            CHAT_Y as u32,
            CHAT_WIDTH,
            142,
            575,
            142,
        ),
        "hud-quickslot" | "hud-quickslot-cooldown" => (865, 615, 152, 83, 152, 83),
        "hud-quickslot-item-tooltip" | "hud-quickslot-skill-tooltip" => {
            let tooltip = scene?.sprites.iter().find(|sprite| {
                sprite.texture_key.ends_with("quickslot-preview")
            })?;
            let left = tooltip.rect.x.min(865.0).floor().max(0.0) as u32;
            let top = tooltip.rect.y.min(615.0).floor().max(0.0) as u32;
            let right = (tooltip.rect.x + tooltip.rect.width).max(1016.0).ceil() as u32;
            let bottom = (tooltip.rect.y + tooltip.rect.height).max(695.0).ceil() as u32;
            let width = right.saturating_sub(left);
            let height = bottom.saturating_sub(top);
            (left, top, width, height, width, height)
        }
        "hud-minimap-collapsed" | "hud-minimap-normal" | "hud-minimap-expanded" => {
            let sprite = scene?.sprites.last()?;
            let (w, h) = match name {
                "hud-minimap-collapsed" => (149, 26),
                "hud-minimap-normal" => (151, 138),
                _ => (222, 198),
            };
            (sprite.rect.x as u32, sprite.rect.y as u32, w, h, w, h)
        }
        "panel-item-inventory" | "panel-item-inventory-pending-move" => (
            INVENTORY_PANEL_X as u32,
            INVENTORY_PANEL_Y as u32,
            INVENTORY_PANEL_WIDTH,
            INVENTORY_PANEL_HEIGHT,
            176,
            291,
        ),
        "panel-item-inventory-tabs" => (0, 20, 896, 291, 635, 211),
        name if equipment_preview_tab(name).is_some() => (
            EQUIPMENT_INVENTORY_PANEL_X as u32,
            EQUIPMENT_INVENTORY_PANEL_Y as u32,
            EQUIPMENT_INVENTORY_PANEL_WIDTH,
            EQUIPMENT_INVENTORY_PANEL_HEIGHT,
            EQUIPMENT_INVENTORY_PANEL_WIDTH,
            EQUIPMENT_INVENTORY_PANEL_HEIGHT,
        ),
        name if name.starts_with("panel-item-tooltip") => {
            let sprite = scene?.sprites.last()?;
            (
                sprite.rect.x as u32,
                sprite.rect.y as u32,
                sprite.texture_width,
                sprite.texture_height,
                sprite.texture_width,
                sprite.texture_height,
            )
        }
        "panel-skill-inventory"
        | "panel-skill-inventory-disabled"
        | "panel-skill-inventory-beginner" => (
            SKILL_BOOK_PANEL_X as u32,
            SKILL_BOOK_PANEL_Y as u32,
            SKILL_BOOK_PANEL_WIDTH,
            SKILL_BOOK_PANEL_HEIGHT,
            177,
            292,
        ),
        "panel-stats" | "panel-stats-auto-assign" | "panel-stats-detail" => {
            let width = STATUS_DETAIL_OFFSET_X as u32 + STATUS_DETAIL_WIDTH;
            (
                STATUS_PANEL_X as u32,
                STATUS_PANEL_Y as u32,
                width,
                350,
                width,
                350,
            )
        }
        "panel-npc-chat" => (
            NpcDialogueLayout::PANEL_X as u32,
            NpcDialogueLayout::PANEL_Y as u32,
            NpcDialogueLayout::WIDTH as u32,
            NpcDialogueLayout::HEIGHT as u32,
            530,
            325,
        ),
        "panel-npc-storage" => (182, 105, 463, 318, 206, 355),
        name if name.starts_with("panel-keyboard-settings") => (
            KEY_CONFIG_PANEL_X as u32,
            KEY_CONFIG_PANEL_Y as u32,
            633,
            343,
            633,
            343,
        ),
        "panel-system-options" => (
            SETTINGS_PANEL_X as u32,
            SETTINGS_PANEL_Y as u32,
            SETTINGS_PANEL_WIDTH,
            SETTINGS_PANEL_HEIGHT,
            SETTINGS_PANEL_WIDTH,
            SETTINGS_PANEL_HEIGHT,
        ),
        "panel-revive-character" => (369, 311, 285, 139, 285, 139),
        "panel-quest-helper" => (
            QUEST_HELPER_PANEL_X as u32,
            QUEST_HELPER_PANEL_Y as u32,
            QUEST_HELPER_PANEL_WIDTH,
            252,
            QUEST_HELPER_PANEL_WIDTH,
            252,
        ),
        "panel-quests" => (
            QUEST_LOG_PANEL_X as u32 - 14,
            QUEST_LOG_PANEL_Y as u32 - 15,
            617,
            447,
            617,
            447,
        ),
        "panel-monster-book" => (276, 20, 564, 420, 564, 420),
        "page-cash-shop" => (112, 84, 800, 600, 576, 437),
        _ => return None,
    };
    Some(rect)
}

fn write_reference_artifacts(
    output: &Path,
    name: &str,
    base: &[u8],
    scene: Option<&LoginSceneAssets>,
) -> Result<(), String> {
    let Some((x, y, width, height, target_width, target_height)) = reference_crop(name, scene)
    else {
        return Ok(());
    };
    let mut crop = vec![0; target_width as usize * target_height as usize * 4];
    for row in 0..target_height {
        for column in 0..target_width {
            let sx = x + column * width / target_width;
            let sy = y + row * height / target_height;
            if sx < CANVAS_WIDTH && sy < CANVAS_HEIGHT {
                let src = ((sy * CANVAS_WIDTH + sx) * 4) as usize;
                let dst = ((row * target_width + column) * 4) as usize;
                crop[dst..dst + 4].copy_from_slice(&base[src..src + 4]);
            }
        }
    }
    write_png(
        &output.join(format!("{name}.png")),
        target_width,
        target_height,
        &crop,
    )
    .map_err(|e| e.to_string())?;
    // The source capture predates the current system-options and tooltip layouts.
    if name == "panel-system-options" || name.starts_with("panel-item-tooltip") {
        return Ok(());
    }
    let reference_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-images")
        .join(format!("{name}.png"));
    let file = match fs::File::open(&reference_path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("{}: {error}", reference_path.display())),
    };
    let mut decoder = png::Decoder::new(file);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut decoded = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut decoded).map_err(|e| e.to_string())?;
    if info.width > target_width || info.height > target_height {
        return Err(format!(
            "reference {name} is {}x{}, larger than its {target_width}x{target_height} crop",
            info.width, info.height
        ));
    }
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return Err(format!("unsupported reference color for {name}")),
    };
    let mut comparison = vec![255; target_width as usize * 2 * target_height as usize * 4];
    let mut difference = vec![0; crop.len()];
    for row in 0..target_height as usize {
        for col in 0..target_width as usize {
            let index = row * target_width as usize + col;
            let left = (row * target_width as usize * 2 + col) * 4;
            let right = left + target_width as usize * 4;
            comparison[right..right + 4].copy_from_slice(&crop[index * 4..index * 4 + 4]);
            if row >= info.height as usize || col >= info.width as usize {
                continue;
            }
            let offset = (row * info.width as usize + col) * channels;
            let source = &decoded[offset..offset + channels];
            let reference = [source[2], source[1], source[0], 255];
            comparison[left..left + 4].copy_from_slice(&reference);
            for channel in 0..3 {
                difference[index * 4 + channel] =
                    reference[channel].abs_diff(crop[index * 4 + channel]);
            }
            difference[index * 4 + 3] = 255;
        }
    }
    write_png(
        &output.join(format!("{name}-comparison.png")),
        target_width * 2,
        target_height,
        &comparison,
    )
    .map_err(|e| e.to_string())?;
    write_png(
        &output.join(format!("{name}-difference.png")),
        target_width,
        target_height,
        &difference,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn write_asset_catalog(archive: &NxArchive, output: &Path) -> Result<(), String> {
    fn visit(
        node: openstory_assets::NxNode,
        path: String,
        rows: &mut String,
    ) -> Result<(), String> {
        if let Some(bitmap) = node.bitmap().map_err(|e| e.to_string())? {
            let origin = node
                .child("origin")
                .map_err(|e| e.to_string())?
                .and_then(|n| n.value().ok());
            let delay = node
                .child("delay")
                .map_err(|e| e.to_string())?
                .and_then(|n| n.value().ok());
            rows.push_str(&format!(
                "{path}\t{}\t{}\t{origin:?}\t{delay:?}\n",
                bitmap.width(),
                bitmap.height()
            ));
        }
        for child in node.children().map_err(|e| e.to_string())? {
            let name = child.name().map_err(|e| e.to_string())?;
            visit(child, format!("{path}/{name}"), rows)?;
        }
        Ok(())
    }
    let mut rows = "path\twidth\theight\torigin\tdelay\n".to_owned();
    for path in [
        "StatusBar.img",
        "MapHelper.img",
        "UIWindow.img/Item",
        "UIWindow.img/Equip",
        "UIWindow.img/Skill",
        "UIWindow.img/Stat",
        "UIWindow.img/StatDetail",
        "UIWindow.img/UtilDlgEx",
        "UIWindow.img/Trunk",
        "UIWindow.img/Shop",
        "UIWindow.img/KeyConfig",
        "UIWindow.img/SysOpt",
        "UIWindow.img/GameOpt",
        "UIWindow.img/Quest",
        "UIWindow.img/QuestAlarm",
        "UIWindow.img/itemSearch",
        "UIWindow.img/EventWindow",
        "UIWindow.img/Messenger",
        "UIWindow.img/UserList",
        "UIToolTip.img",
        "UIWindow2.img/MonsterBook",
        "CashShop.img",
        "CashShopGL.img",
        "Basic.img",
    ] {
        if let Some(node) = resolve_ui_path(&archive.root(), path)? {
            visit(node, path.to_owned(), &mut rows)?;
        } else {
            rows.push_str(&format!("{path}\tMISSING\n"));
        }
    }
    for path in [
        "StatusBar.img/base/backgrnd",
        "StatusBar.img/base/backgrnd2",
        "StatusBar.img/gauge/bar",
        "StatusBar.img/gauge/graduation",
        "StatusBar.img/number/3",
        "Basic.img/LevelNo/8",
        "Basic.img/LevelNo/9",
        "Basic.img/LevelNo/blank",
        "Basic.img/Tab/fill0",
        "Basic.img/Tab/fill1",
        "Basic.img/Tab/left0",
        "Basic.img/Tab/left1",
        "Basic.img/Tab/middle0",
        "Basic.img/Tab/middle1",
        "Basic.img/Tab/middle2",
        "Basic.img/Tab/right0",
        "Basic.img/Tab/right1",
        "Basic.img/Tab2/fill0",
        "Basic.img/Tab2/fill1",
        "Basic.img/Tab2/left0",
        "Basic.img/Tab2/left1",
        "Basic.img/Tab2/middle0",
        "Basic.img/Tab2/middle1",
        "Basic.img/Tab2/middle2",
        "Basic.img/Tab2/right0",
        "Basic.img/Tab2/right1",
        "Basic.img/Notice/backgrnd",
        "Basic.img/Notice2/t",
        "Basic.img/Notice2/c",
        "Basic.img/Notice2/s",
        "Basic.img/Notice3/t",
        "Basic.img/Notice3/c",
        "Basic.img/Notice3/s",
        "Basic.img/Notice4/t",
        "Basic.img/Notice4/c",
        "Basic.img/Notice4/s",
        "Basic.img/Notice5/t",
        "Basic.img/Notice5/c",
        "Basic.img/Notice5/s_",
        "Basic.img/VScr4/enabled/base",
        "Basic.img/VScr4/enabled/prev0",
        "Basic.img/VScr4/enabled/next0",
        "Basic.img/VScr4/enabled/thumb0",
        "UIWindow.img/Skill/backgrnd",
        "UIWindow.img/Item/backgrnd",
        "UIWindow.img/Item/Tab/enabled/0",
        "UIWindow.img/Item/Tab/disabled/0",
        "UIWindow.img/KeyConfig/backgrnd",
        "UIWindow.img/KeyConfig/key/2",
        "UIWindow.img/KeyConfig/key/3",
        "UIWindow.img/KeyConfig/icon/52",
        "UIWindow.img/KeyConfig/icon/100",
        "UIWindow.img/Equip/backgrnd",
        "UIWindow.img/UtilDlgEx/backgrnd",
        "UIWindow.img/SysOpt/backgrnd",
        "UIWindow.img/Quest/backgrnd",
        "UIWindow.img/Quest/backgrnd2",
        "UIWindow.img/Quest/backgrnd3",
        "UIWindow.img/Quest/backgrnd4",
        "UIWindow.img/Quest/backgrnd5",
        "UIWindow.img/Quest/Tab/enabled/0",
        "UIWindow.img/Quest/Tab/enabled/1",
        "UIWindow.img/Quest/Tab/enabled/2",
        "UIWindow.img/Quest/Tab/disabled/0",
        "UIWindow.img/Quest/Tab/disabled/1",
        "UIWindow.img/Quest/Tab/disabled/2",
        "UIWindow.img/Quest/BtAlert/normal/0",
        "UIWindow.img/Quest/BtDetail/normal/0",
        "UIWindow.img/Quest/BtGiveup/normal/0",
        "UIWindow.img/Quest/BtGiveup/disabled/0",
        "UIWindow.img/Quest/BtMarkNpc/normal/0",
        "UIWindow.img/Quest/BtMarkNpc/disabled/0",
        "UIWindow.img/Quest/BtOK/normal/0",
        "UIWindow.img/Quest/icon0",
        "UIWindow.img/Quest/icon1",
        "UIWindow.img/Quest/icon2/0",
        "UIWindow.img/Quest/icon3/0",
        "UIWindow.img/Quest/icon4",
        "UIWindow.img/Quest/icon5/0",
        "UIWindow.img/Quest/icon6/0",
        "UIWindow.img/QuestAlarm/backgrndmin",
        "UIWindow.img/QuestAlarm/backgrndmax",
        "UIWindow.img/QuestAlarm/backgrndcenter",
        "UIWindow.img/QuestAlarm/backgrndbottom",
        "UIWindow.img/QuestAlarm/BtQ/normal/0",
        "UIWindow.img/QuestAlarm/BtAuto/normal/0",
        "UIWindow.img/BtUIClose/normal/0",
        "UIWindow.img/BtUIClose/disabled/0",
        "Basic.img/BtClose/normal/0",
        "Basic.img/BtClose2/normal/0",
        "CashShop.img/Base/backgrnd",
        "CashShop.img/CSList/Base",
        "CashShop.img/Base/Preview/0",
    ] {
        if let Some(node) = resolve_ui_path(&archive.root(), path)?
            && let Some(bitmap) = node.bitmap().map_err(|e| e.to_string())?
        {
            let pixels = bitmap.decode_bgra_shared().map_err(|e| e.to_string())?;
            let name = format!("nx-{}.png", path.replace(['/', '.'], "-"));
            write_png(
                &output.join(name),
                u32::from(bitmap.width()),
                u32::from(bitmap.height()),
                &pixels,
            )
            .map_err(|e| e.to_string())?;
        }
    }
    fs::write(output.join("nx-assets.tsv"), rows).map_err(|e| e.to_string())
}

fn write_typography_preview(font: &fontdue::Font, output: &Path) -> Result<(), String> {
    const WIDTH: usize = 720;
    const HEIGHT: usize = 180;
    let mut pixels = vec![248; WIDTH * HEIGHT * 4];
    for pixel in pixels.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    for (index, role) in [
        ui_foundation::UiTextRole::Title,
        ui_foundation::UiTextRole::Label,
        ui_foundation::UiTextRole::Body,
        ui_foundation::UiTextRole::Small,
        ui_foundation::UiTextRole::Value,
        ui_foundation::UiTextRole::Emphasis,
    ]
    .into_iter()
    .enumerate()
    {
        ui_foundation::draw_text_role(
            font,
            &mut pixels,
            WIDTH,
            HEIGHT,
            "MapleStory 0123456789 | Sword Mastery | 1,765,592 | Agjpq Il1 MW",
            12,
            8 + index as i32 * 25,
            role,
            WIDTH - 24,
            24,
            None,
        );
    }
    write_png(
        &output.join("typography.png"),
        WIDTH as u32,
        HEIGHT as u32,
        &pixels,
    )
    .map_err(|error| error.to_string())?;
    fs::write(output.join("typography.txt"), format!("Regular: {}\nBold: {}\nRasterizer: fontdue, grayscale coverage; logical-pixel snapping\n",
        font.name().unwrap_or("unnamed custom font"), ui_foundation::bold_font().and_then(|font| font.name()).unwrap_or("regular fallback")))
        .map_err(|error| error.to_string())
}

fn write_keyboard_art_catalog(
    ui: &NxArchive,
    skills: &NxArchive,
    items: &NxArchive,
    characters: &NxArchive,
    font: &fontdue::Font,
    output: &Path,
) -> Result<(), String> {
    let mut entries = (0..42)
        .map(|id| {
            (
                id,
                decode_nx_icon(ui, &format!("UIWindow.img/KeyConfig/icon/{id}")),
            )
        })
        .collect::<Vec<_>>();
    for class in [200, 210, 211] {
        if let Some(root) = skills
            .root()
            .resolve(&format!("{class}.img/skill"))
            .map_err(|e| e.to_string())?
        {
            for node in root.children().map_err(|e| e.to_string())? {
                if let Ok(id) = node.name().map_err(|e| e.to_string())?.parse::<i32>() {
                    entries.push((
                        id,
                        decode_nx_icon(skills, &format!("{class}.img/skill/{id}/icon")),
                    ));
                }
            }
        }
    }
    for id in 2_000_000..=2_000_006 {
        entries.push((
            id,
            inventory_item_icon(id, items, characters, &mut HashMap::new()),
        ));
    }
    let width = 640;
    let height = entries.len().div_ceil(10) * 60;
    let mut pixels = vec![245; width * height * 4];
    for pixel in pixels.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    for (index, (id, icon)) in entries.into_iter().enumerate() {
        let x = index % 10 * 64;
        let y = index / 10 * 60;
        if let Some((w, h, image)) = icon {
            write_png(&output.join(format!("keyboard-art-{id}.png")), w, h, &image)
                .map_err(|e| e.to_string())?;
            composite_bgra_pixels(
                &mut pixels,
                width,
                height,
                &image,
                w as usize,
                h as usize,
                x as i32 + 16,
                y as i32,
            );
        }
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            &id.to_string(),
            x as i32 + 2,
            y as i32 + 35,
            9.0,
            62,
            14,
            [20, 20, 20],
        );
    }
    write_png(
        &output.join("keyboard-art-catalog.png"),
        width as u32,
        height as u32,
        &pixels,
    )
    .map_err(|e| e.to_string())
}
