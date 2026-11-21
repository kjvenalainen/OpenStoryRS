// Shared quantity window for inventory and shop actions. Coordinates inside
// the frame are the same for painting and pointer input.
const QUANTITY_WINDOW_WIDTH: usize = 299;
const QUANTITY_WINDOW_HEIGHT: usize = 118;
const QUANTITY_TRACK_LEFT: f32 = 73.0;
const QUANTITY_TRACK_RIGHT: f32 = 205.0;
const QUANTITY_TRACK_Y: usize = 51;
const QUANTITY_FIELD: ui_foundation::UiRect = ui_foundation::UiRect::new(228.0, 41.0, 56.0, 20.0);
const QUANTITY_SLIDER: ui_foundation::UiRect = ui_foundation::UiRect::new(62.0, 41.0, 155.0, 20.0);
const QUANTITY_CLOSE: ui_foundation::UiRect = ui_dialog::close_button(280.0, 6.0);

enum QuantityIcon<'a> {
    Item(&'a (u32, u32, Arc<[u8]>)),
    Mesos,
}

struct QuantityInput {
    text: String,
    selected: bool,
    dragging: bool,
    key_text: Option<String>,
}

impl QuantityInput {
    fn new(value: Option<u16>) -> Self {
        Self {
            text: value.map_or_else(String::new, |value| value.to_string()),
            selected: true,
            dragging: false,
            key_text: None,
        }
    }

    fn value(&self) -> Option<u16> {
        self.text.parse().ok()
    }

    fn clamp_to_max(&mut self, max: u16) {
        if self
            .text
            .parse::<u32>()
            .is_ok_and(|value| value > u32::from(max))
        {
            self.text = max.to_string();
        }
    }

    fn press(&mut self, x: f32, y: f32, min: u16, max: u16) -> bool {
        if QUANTITY_FIELD.contains(x, y) {
            self.selected = true;
        } else if min < max && QUANTITY_SLIDER.contains(x, y) {
            self.dragging = true;
            self.set_from_slider(x, min, max);
        } else {
            return false;
        }
        true
    }

    fn set_from_slider(&mut self, local_x: f32, min: u16, max: u16) {
        let fraction = ((local_x - QUANTITY_TRACK_LEFT)
            / (QUANTITY_TRACK_RIGHT - QUANTITY_TRACK_LEFT))
            .clamp(0.0, 1.0);
        self.text =
            (min + (fraction * f32::from(max.saturating_sub(min))).round() as u16).to_string();
        self.selected = true;
    }

    fn edit_key(&mut self, event: &winit::event::KeyEvent, max: u16) -> bool {
        self.key_text = None;
        if matches!(
            event.physical_key,
            PhysicalKey::Code(KeyCode::Backspace | KeyCode::Delete)
        ) {
            if self.selected {
                self.text.clear();
            } else {
                self.text.pop();
            }
            self.selected = false;
            true
        } else if let Some(text) = event.text.as_deref() {
            let changed = self.insert_digits(text, max);
            if changed {
                self.key_text = Some(text.to_owned());
            }
            changed
        } else {
            false
        }
    }

    fn commit_text(&mut self, text: &str, max: u16) -> bool {
        self.key_text.take().as_deref() != Some(text) && self.insert_digits(text, max)
    }

    fn insert_digits(&mut self, text: &str, max: u16) -> bool {
        let mut digits = text.chars().filter(char::is_ascii_digit).peekable();
        if digits.peek().is_none() {
            return false;
        }
        if self.selected {
            self.text.clear();
            self.selected = false;
        }
        let remaining = max.to_string().len().saturating_sub(self.text.len());
        self.text.extend(digits.take(remaining));
        remaining > 0
    }
}

fn quantity_window_origin(canvas_width: usize, canvas_height: usize) -> (i32, i32) {
    (
        ((canvas_width - QUANTITY_WINDOW_WIDTH) / 2) as i32,
        ((canvas_height - QUANTITY_WINDOW_HEIGHT) / 2) as i32,
    )
}

fn quantity_window_local(cursor: Point, left: f32, top: f32, width: usize, height: usize) -> Point {
    let (x, y) = quantity_window_origin(width, height);
    Point {
        x: cursor.x - left - x as f32,
        y: cursor.y - top - y as f32,
    }
}

impl ClientApp {
    fn drag_quantity_slider(&mut self, cursor: Point) -> bool {
        if let Some(prompt) = self.inventory_drop_prompt.as_mut()
            && prompt.quantity.dragging
        {
            let local = quantity_window_local(cursor, 0.0, 0.0, 1024, 768);
            let min = prompt.minimum();
            if prompt.max_quantity >= min {
                prompt
                    .quantity
                    .set_from_slider(local.x, min, prompt.max_quantity);
                prompt.notice = None;
                self.refresh_inventory_drop_prompt_texture();
            }
            return true;
        }
        if let Some(quantity) = self
            .npc_shop
            .as_mut()
            .and_then(|shop| shop.quantity.as_mut())
            && quantity.input.dragging
        {
            let local = quantity_window_local(
                cursor,
                NPC_SHOP_X,
                NPC_SHOP_Y,
                NPC_SHOP_WIDTH,
                NPC_SHOP_HEIGHT,
            );
            quantity
                .input
                .set_from_slider(local.x, 1, quantity.maximum as u16);
            quantity.invalid = false;
            self.refresh_npc_shop_texture();
            return true;
        }
        false
    }

    fn finish_quantity_drag(&mut self) -> bool {
        if let Some(prompt) = self.inventory_drop_prompt.as_mut()
            && std::mem::take(&mut prompt.quantity.dragging)
        {
            self.refresh_inventory_drop_prompt_texture();
            return true;
        }
        if let Some(quantity) = self
            .npc_shop
            .as_mut()
            .and_then(|shop| shop.quantity.as_mut())
            && std::mem::take(&mut quantity.input.dragging)
        {
            self.refresh_npc_shop_texture();
            return true;
        }
        false
    }
}

fn quantity_window_buttons(single: bool) -> (ui_foundation::UiRect, ui_foundation::UiRect) {
    let ok_x = if single { 126.0 } else { 182.0 };
    (
        ui_foundation::UiRect::new(ok_x, 95.0, 47.0, 18.0),
        ui_foundation::UiRect::new(237.0, 95.0, 51.0, 18.0),
    )
}

fn render_quantity_window(
    ui: &NxArchive,
    font: &fontdue::Font,
    title: &str,
    message: &str,
    icon: Option<QuantityIcon<'_>>,
    controls: Option<(&QuantityInput, u16, u16)>,
    notice: Option<&str>,
    step: InventoryDropStep,
) -> Result<Vec<u8>, String> {
    const W: usize = QUANTITY_WINDOW_WIDTH;
    const H: usize = QUANTITY_WINDOW_HEIGHT;
    const CHROME: &str = "UIWindow.img/SysOpt/backgrnd";
    const HEADER: usize = 25;
    const FOOTER: usize = 28;
    const INK: [u8; 3] = [40, 51, 61];
    let mut pixels = vec![0; W * H * 4];
    let mut dialog = NxDialog::new(&mut pixels, W, H, ui);
    let (source_width, source_height, source) = dialog
        .frame_bitmap(CHROME)
        .filter(|(width, height, _)| *width == W && *height >= HEADER + FOOTER + 1)
        .ok_or_else(|| format!("missing quantity window frame {CHROME}"))?;
    dialog.frame_rows(
        &source,
        source_width,
        0,
        &[
            (0..HEADER, 0),
            (source_height - FOOTER..source_height, (H - FOOTER) as i32),
        ],
    )?;
    for y in HEADER..H - FOOTER {
        dialog.frame_rows(&source, source_width, 0, &[(HEADER..HEADER + 1, y as i32)])?;
    }

    // The panel art includes the settings title and controls. Keep only its
    // silver frame and replace the title interior and body.
    fill_dialogue_rect(&mut pixels, W, H, 7, 6, W - 14, 12, [255, 255, 255, 255]);
    fill_dialogue_rect(
        &mut pixels,
        W,
        H,
        5,
        HEADER,
        W - 10,
        H - HEADER - FOOTER,
        [244, 245, 243, 255],
    );
    let (ok, cancel) = quantity_window_buttons(step == InventoryDropStep::Blocked);
    let ok_art = match step {
        InventoryDropStep::ConfirmUntradable => "Basic.img/BtYes2/normal/0",
        _ => "Basic.img/BtOK2/normal/0",
    };
    let mut dialog = NxDialog::new(&mut pixels, W, H, ui);
    dialog.buttons(&[(ui_dialog::CLOSE_BUTTON_ART, QUANTITY_CLOSE), (ok_art, ok)])?;
    if step != InventoryDropStep::Blocked {
        dialog.buttons(&[("UIWindow.img/ItemMove/BtCancel/normal/0", cancel)])?;
    }

    draw_dialogue_text(font, &mut pixels, W, H, title, 9, 6, 10.0, W - 37, 13, INK);
    let has_icon = icon.is_some();
    if let Some(icon) = icon {
        const X: usize = 14;
        const Y: usize = 36;
        const CELL: usize = 36;
        fill_dialogue_rect(&mut pixels, W, H, X, Y, CELL, CELL, [188, 195, 199, 255]);
        fill_dialogue_rect(
            &mut pixels,
            W,
            H,
            X + 1,
            Y + 1,
            CELL - 2,
            CELL - 2,
            [222, 227, 229, 255],
        );
        match icon {
            QuantityIcon::Item((w, h, pixels_bgra)) => {
                let (w, h) = (*w as usize, *h as usize);
                if w <= 32 && h <= 32 {
                    composite_bgra_pixels(
                        &mut pixels,
                        W,
                        H,
                        pixels_bgra,
                        w,
                        h,
                        (X + (CELL - w) / 2) as i32,
                        (Y + (CELL - h) / 2) as i32,
                    );
                } else {
                    composite_inventory_icon(
                        &mut pixels,
                        W,
                        H,
                        pixels_bgra,
                        w,
                        h,
                        X + 2,
                        Y + 2,
                        32,
                    );
                }
            }
            QuantityIcon::Mesos => {
                composite_nx_ui_bitmap_centered(
                    &mut pixels,
                    W,
                    H,
                    ui,
                    "UIWindow.img/Shop/meso",
                    ui_foundation::UiRect::new(X as f32, Y as f32, CELL as f32, CELL as f32),
                )?;
            }
        }
    }
    if !message.is_empty() {
        let (x, y, max_width) = if controls.is_some() {
            (13, 26, W - 26)
        } else if has_icon {
            (62, 38, W - 75)
        } else {
            (13, 38, W - 26)
        };
        let size = if controls.is_some() { 10.0 } else { 12.0 };
        draw_dialogue_text(
            font,
            &mut pixels,
            W,
            H,
            message,
            x,
            y,
            size,
            max_width,
            35,
            INK,
        );
    }
    if let Some((input, min, max)) = controls {
        let track_left = QUANTITY_TRACK_LEFT as usize;
        let track_width = QUANTITY_TRACK_RIGHT as usize - track_left;
        let thumb_offset = if max > min {
            usize::from(input.value().unwrap_or(min).clamp(min, max) - min) * track_width
                / usize::from(max - min)
        } else {
            0
        };
        let thumb = if max <= min {
            "thumbDisabled"
        } else if input.dragging {
            "thumbPressed"
        } else {
            "thumbNormal"
        };
        draw_nx_slider(
            &mut pixels,
            W,
            H,
            ui,
            (track_left, QUANTITY_TRACK_Y, track_width),
            thumb_offset,
            thumb,
        )?;
        let x = QUANTITY_FIELD.x as usize;
        let y = QUANTITY_FIELD.y as usize;
        let w = QUANTITY_FIELD.width as usize;
        let h = QUANTITY_FIELD.height as usize;
        fill_dialogue_rect(&mut pixels, W, H, x, y, w, h, [145, 120, 89, 255]);
        fill_dialogue_rect(
            &mut pixels,
            W,
            H,
            x + 1,
            y + 1,
            w - 2,
            h - 2,
            [255, 255, 255, 255],
        );
        let text_width = dialogue_text_width(font, &input.text, 12.0).ceil() as usize;
        if input.selected && text_width > 0 {
            fill_dialogue_rect(
                &mut pixels,
                W,
                H,
                x + 4,
                y + 3,
                text_width + 2,
                h - 6,
                [246, 217, 187, 255],
            );
        } else {
            fill_dialogue_rect(
                &mut pixels,
                W,
                H,
                x + 5 + text_width,
                y + 4,
                1,
                h - 8,
                [112, 84, 54, 255],
            );
        }
        draw_dialogue_text(
            font,
            &mut pixels,
            W,
            H,
            &input.text,
            x as i32 + 5,
            y as i32 + 3,
            12.0,
            w - 10,
            h - 4,
            INK,
        );
        let min_label = min.to_string();
        let max_label = max.to_string();
        let max_x =
            QUANTITY_TRACK_RIGHT as i32 - dialogue_text_width(font, &max_label, 10.0).ceil() as i32;
        draw_dialogue_text(
            font,
            &mut pixels,
            W,
            H,
            &min_label,
            track_left as i32,
            63,
            10.0,
            50,
            12,
            INK,
        );
        if min != max {
            draw_dialogue_text(
                font,
                &mut pixels,
                W,
                H,
                &max_label,
                max_x,
                63,
                10.0,
                50,
                12,
                INK,
            );
        }
    }
    if let Some(notice) = notice {
        draw_dialogue_text(
            font,
            &mut pixels,
            W,
            H,
            notice,
            13,
            76,
            10.0,
            W - 26,
            14,
            [144, 43, 36],
        );
    }
    Ok(pixels)
}
