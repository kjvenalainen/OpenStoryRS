const EVENT_LIST_PANEL_WIDTH: usize = 480;
const EVENT_LIST_PANEL_HEIGHT: usize = 440;

struct EventWindowTile {
    width: usize,
    height: usize,
    origin_x: i32,
    origin_y: i32,
    pixels: Arc<[u8]>,
}

struct EventListPanel {
    pixels: Vec<u8>,
}

fn load_event_window_tile(archive: &NxArchive, path: &str) -> Result<EventWindowTile, String> {
    let node = archive
        .root()
        .resolve(path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("v83 UI archive is missing {path:?}"))?;
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("v83 UI asset {path:?} is not a bitmap"))?;
    let origin = node
        .child("origin")
        .map_err(|error| error.to_string())?
        .map(|origin| origin.value())
        .transpose()
        .map_err(|error| error.to_string())?;
    let (origin_x, origin_y) = match origin {
        Some(NodeValue::Vector { x, y }) => (x, y),
        Some(_) => return Err(format!("v83 UI asset {path:?} has an invalid origin")),
        None => (0, 0),
    };
    Ok(EventWindowTile {
        width: usize::from(bitmap.width()),
        height: usize::from(bitmap.height()),
        origin_x,
        origin_y,
        pixels: bitmap
            .decode_bgra_shared()
            .map_err(|error| error.to_string())?,
    })
}

fn composite_event_window_tile(
    destination: &mut [u8],
    tile: &EventWindowTile,
    x: i32,
    y: i32,
) {
    composite_bgra_pixels(
        destination,
        EVENT_LIST_PANEL_WIDTH,
        EVENT_LIST_PANEL_HEIGHT,
        &tile.pixels,
        tile.width,
        tile.height,
        x - tile.origin_x,
        y - tile.origin_y,
    );
}

fn format_event_time(seconds: i32) -> String {
    let seconds = seconds.max(0);
    if seconds >= 3600 {
        format!("{}:{:02}:{:02}", seconds / 3600, (seconds % 3600) / 60, seconds % 60)
    } else {
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    }
}

fn build_event_list_panel(
    archive: &NxArchive,
    font: &fontdue::Font,
    string_archive: &NxArchive,
    item_name_cache: &mut HashMap<i32, Option<String>>,
    events: &[EventListEntry],
    offset: usize,
    selected: Option<usize>,
    received: bool,
) -> Result<EventListPanel, String> {
    let mut pixels = vec![0_u8; EVENT_LIST_PANEL_WIDTH * EVENT_LIST_PANEL_HEIGHT * 4];
    let rows: [[EventWindowTile; 3]; 3] = [
        [
            load_event_window_tile(archive, "UIWindow.img/EventWindow/backgrnd/top/left")?,
            load_event_window_tile(archive, "UIWindow.img/EventWindow/backgrnd/top/center")?,
            load_event_window_tile(archive, "UIWindow.img/EventWindow/backgrnd/top/right")?,
        ],
        [
            load_event_window_tile(archive, "UIWindow.img/EventWindow/backgrnd/center/left")?,
            load_event_window_tile(archive, "UIWindow.img/EventWindow/backgrnd/center/center")?,
            load_event_window_tile(archive, "UIWindow.img/EventWindow/backgrnd/center/right")?,
        ],
        [
            load_event_window_tile(archive, "UIWindow.img/EventWindow/backgrnd/bottom/left")?,
            load_event_window_tile(archive, "UIWindow.img/EventWindow/backgrnd/bottom/center")?,
            load_event_window_tile(archive, "UIWindow.img/EventWindow/backgrnd/bottom/right")?,
        ],
    ];
    let tile_width = rows[1][1].width.max(1);
    let tile_height = rows[1][1].height.max(1);
    let top_height = rows[0][1].height;
    let bottom_y = EVENT_LIST_PANEL_HEIGHT.saturating_sub(rows[2][1].height);
    for x in (0..EVENT_LIST_PANEL_WIDTH).step_by(tile_width) {
        let column = if x == 0 {
            0
        } else if x + tile_width >= EVENT_LIST_PANEL_WIDTH {
            2
        } else {
            1
        };
        composite_event_window_tile(&mut pixels, &rows[0][column], x as i32, 0);
        composite_event_window_tile(&mut pixels, &rows[2][column], x as i32, bottom_y as i32);
    }
    for y in (top_height..bottom_y).step_by(tile_height) {
        for x in (0..EVENT_LIST_PANEL_WIDTH).step_by(tile_width) {
            let column = if x == 0 {
                0
            } else if x + tile_width >= EVENT_LIST_PANEL_WIDTH {
                2
            } else {
                1
            };
            composite_event_window_tile(&mut pixels, &rows[1][column], x as i32, y as i32);
        }
    }
    fill_dialogue_rect(
        &mut pixels,
        EVENT_LIST_PANEL_WIDTH,
        EVENT_LIST_PANEL_HEIGHT,
        6,
        top_height,
        EVENT_LIST_PANEL_WIDTH - 12,
        EVENT_LIST_PANEL_HEIGHT - top_height - rows[2][1].height,
        [232, 231, 226, 255],
    );

    let close = load_event_window_tile(
        archive,
        "UIWindow.img/EventWindow/backgrnd/BtClose/normal/0",
    )?;
    composite_event_window_tile(
        &mut pixels,
        &close,
        EVENT_LIST_PANEL_WIDTH as i32 - 28,
        7,
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        EVENT_LIST_PANEL_WIDTH,
        EVENT_LIST_PANEL_HEIGHT,
        "Event List",
        18,
        7,
        15.0,
        EVENT_LIST_PANEL_WIDTH - 56,
        20,
        [35, 33, 29],
    );
    draw_dialogue_text(
        font,
        &mut pixels,
        EVENT_LIST_PANEL_WIDTH,
        EVENT_LIST_PANEL_HEIGHT,
        "Current server events and rewards",
        18,
        25,
        10.0,
        EVENT_LIST_PANEL_WIDTH - 36,
        14,
        [104, 101, 94],
    );

    let row_top = 48_i32;
    let row_step = 91_i32;
    let row_height = 84_i32;
    let visible_count = 4;
    if events.is_empty() {
        let message = if received {
            "No events running right now"
        } else {
            "Requesting events..."
        };
        draw_dialogue_text(
            font,
            &mut pixels,
            EVENT_LIST_PANEL_WIDTH,
            EVENT_LIST_PANEL_HEIGHT,
            message,
            22,
            72,
            13.0,
            EVENT_LIST_PANEL_WIDTH - 44,
            26,
            [92, 89, 82],
        );
    } else {
        for row in 0..visible_count {
            let index = offset.saturating_add(row);
            let Some(entry) = events.get(index) else {
                break;
            };
            let y = row_top + row as i32 * row_step;
            let selected_row = selected == Some(index);
            let fill = if selected_row {
                [224, 235, 242, 255]
            } else {
                [241, 240, 236, 255]
            };
            fill_dialogue_rect(
                &mut pixels,
                EVENT_LIST_PANEL_WIDTH,
                EVENT_LIST_PANEL_HEIGHT,
                14,
                y as usize,
                EVENT_LIST_PANEL_WIDTH - 28,
                row_height as usize,
                fill,
            );
            let event = &entry.event;
            let mut title = event.name.chars().take(32).collect::<String>();
            if event.multiplier > 100 {
                title.push_str(&format!(
                    "  ({}.{:01}x)",
                    event.multiplier / 100,
                    (event.multiplier % 100) / 10
                ));
            }
            draw_dialogue_text(
                font,
                &mut pixels,
                EVENT_LIST_PANEL_WIDTH,
                EVENT_LIST_PANEL_HEIGHT,
                &title,
                24,
                y + 7,
                12.0,
                EVENT_LIST_PANEL_WIDTH - 170,
                17,
                [37, 37, 37],
            );
            let (status, status_color) = if event.seconds_remaining > 0 {
                (format!("Running  {}", format_event_time(event.seconds_remaining)), [47, 116, 64])
            } else if event.seconds_remaining < 0 {
                ("Upcoming".to_owned(), [142, 107, 45])
            } else {
                ("Ended".to_owned(), [112, 108, 102])
            };
            draw_dialogue_text(
                font,
                &mut pixels,
                EVENT_LIST_PANEL_WIDTH,
                EVENT_LIST_PANEL_HEIGHT,
                &status,
                EVENT_LIST_PANEL_WIDTH as i32 - 160,
                y + 7,
                10.0,
                130,
                16,
                status_color,
            );
            draw_dialogue_text(
                font,
                &mut pixels,
                EVENT_LIST_PANEL_WIDTH,
                EVENT_LIST_PANEL_HEIGHT,
                &event.description,
                24,
                y + 28,
                10.0,
                EVENT_LIST_PANEL_WIDTH - 52,
                24,
                [89, 87, 83],
            );
            if event.has_item_rewards && !event.rewards.is_empty() {
                let rewards = event
                    .rewards
                    .iter()
                    .take(5)
                    .map(|reward| {
                        let name = item_name(reward.item_id, string_archive, item_name_cache);
                        format!("{name} x{}", reward.quantity)
                    })
                    .collect::<Vec<_>>()
                    .join("  ·  ");
                draw_dialogue_text(
                    font,
                    &mut pixels,
                    EVENT_LIST_PANEL_WIDTH,
                    EVENT_LIST_PANEL_HEIGHT,
                    &rewards,
                    24,
                    y + 56,
                    9.0,
                    EVENT_LIST_PANEL_WIDTH - 52,
                    19,
                    [74, 91, 101],
                );
            } else if event.seconds_remaining > 0 && entry.peak_duration > 0 {
                let gauge_left = 24_i32;
                let gauge_top = y + 63;
                let gauge_width = EVENT_LIST_PANEL_WIDTH as i32 - 52;
                fill_dialogue_rect(
                    &mut pixels,
                    EVENT_LIST_PANEL_WIDTH,
                    EVENT_LIST_PANEL_HEIGHT,
                    gauge_left as usize,
                    gauge_top as usize,
                    gauge_width as usize,
                    6,
                    [204, 205, 201, 255],
                );
                let fill_width = ((gauge_width as i64
                    * i64::from(event.seconds_remaining.max(0)))
                    / i64::from(entry.peak_duration.max(1)))
                .clamp(0, i64::from(gauge_width)) as i32;
                fill_dialogue_rect(
                    &mut pixels,
                    EVENT_LIST_PANEL_WIDTH,
                    EVENT_LIST_PANEL_HEIGHT,
                    gauge_left as usize,
                    gauge_top as usize,
                    fill_width as usize,
                    6,
                    [119, 177, 139, 255],
                );
            }
        }
        if events.len() > visible_count {
            draw_dialogue_text(
                font,
                &mut pixels,
                EVENT_LIST_PANEL_WIDTH,
                EVENT_LIST_PANEL_HEIGHT,
                &format!(
                    "{}–{} of {}  •  Scroll to browse",
                    offset + 1,
                    (offset + visible_count).min(events.len()),
                    events.len()
                ),
                18,
                EVENT_LIST_PANEL_HEIGHT as i32 - 23,
                9.0,
                EVENT_LIST_PANEL_WIDTH - 36,
                14,
                [92, 89, 82],
            );
        }
    }

    Ok(EventListPanel { pixels })
}
