const EVENT_LIST_PANEL_WIDTH: usize = 480;
const EVENT_LIST_PANEL_HEIGHT: usize = 440;

struct EventListPanel {
    pixels: Vec<u8>,
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
    let mut dialog = NxDialog::new(
        &mut pixels,
        EVENT_LIST_PANEL_WIDTH,
        EVENT_LIST_PANEL_HEIGHT,
        archive,
    );
    dialog.resizable_frame()?;
    dialog.buttons(&[(
        "UIWindow.img/EventWindow/backgrnd/BtClose/normal/0",
        ui_dialog::close_button(EVENT_LIST_PANEL_WIDTH as f32 - 28.0, 7.0),
    )])?;
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
                if selected_row {
                    [33, 76, 125]
                } else {
                    [37, 37, 37]
                },
            );
            let (mut status, status_color) = if event.seconds_remaining > 0 {
                (format!("Running  {}", format_event_time(event.seconds_remaining)), [47, 116, 64])
            } else if event.seconds_remaining < 0 {
                ("Upcoming".to_owned(), [142, 107, 45])
            } else {
                ("Ended".to_owned(), [112, 108, 102])
            };
            if event.seconds_remaining > 0 && entry.peak_duration > 0 {
                let progress = (100_i64
                    * i64::from(event.seconds_remaining.max(0))
                    / i64::from(entry.peak_duration.max(1)))
                .clamp(0, 100);
                status.push_str(&format!("  {progress}%"));
            }
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
