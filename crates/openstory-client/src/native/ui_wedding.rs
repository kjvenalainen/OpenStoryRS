const WEDDING_PANEL_WIDTH: usize = 234;
const WEDDING_PANEL_HEIGHT: usize = 250;

struct WeddingPanel {
    pixels: Vec<u8>,
}

fn build_wedding_panel(
    archive: &NxArchive,
    font: &fontdue::Font,
    countdown_seconds: i32,
) -> Result<WeddingPanel, String> {
    let mut pixels = vec![0_u8; WEDDING_PANEL_WIDTH * WEDDING_PANEL_HEIGHT * 4];
    composite_ui_bitmap(
        &mut pixels,
        WEDDING_PANEL_WIDTH,
        WEDDING_PANEL_HEIGHT,
        archive,
        "UIWindow.img/Wedding/Invitation/Cathedral",
        0,
        0,
    )?;

    if countdown_seconds > 0 {
        composite_ui_bitmap(
            &mut pixels,
            WEDDING_PANEL_WIDTH,
            WEDDING_PANEL_HEIGHT,
            archive,
            "UIWindow.img/Wedding/Counter/StopWatch",
            117,
            135,
        )?;
        let countdown = format!(
            "{}:{:02}",
            countdown_seconds / 60,
            countdown_seconds % 60
        );
        let label_width = measured_text_width(font, &countdown, 12.0).ceil() as u32;
        let label = rasterize_label_color(
            font,
            &countdown,
            12.0,
            label_width.max(1),
            20,
            [0, 255, 255],
        );
        composite_bgra_pixels(
            &mut pixels,
            WEDDING_PANEL_WIDTH,
            WEDDING_PANEL_HEIGHT,
            &label,
            label_width.max(1) as usize,
            20,
            117 - label_width as i32 / 2,
            155,
        );
    }

    composite_ui_bitmap(
        &mut pixels,
        WEDDING_PANEL_WIDTH,
        WEDDING_PANEL_HEIGHT,
        archive,
        "UIWindow.img/Wedding/Invitation/BtOK/normal/0",
        88,
        220,
    )?;

    Ok(WeddingPanel { pixels })
}
