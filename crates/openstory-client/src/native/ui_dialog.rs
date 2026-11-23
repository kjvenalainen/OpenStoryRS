//! Draws NX dialog chrome and controls around panel-owned content.
use super::ui_foundation::{
    UiRect, UiTextRole, draw_control_label, draw_tab_label, draw_text_role,
};
use super::{
    NxArchive, composite_bgra_pixels, composite_nx_ui_bitmap, composite_nx_ui_bitmap_centered,
    composite_nx_ui_bitmap_scaled_to, composite_nx_ui_nine_slice_frame, composite_nx_ui_tab,
    composite_nx_ui_vertical_frame, nx_dialogue_button_geometry, resolve_ui_path,
};
use std::{ops::Range, sync::Arc};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ButtonState {
    Normal,
    Hovered,
    Disabled,
}

#[derive(Clone, Copy)]
pub(super) struct NxButton<'a> {
    art: &'a str,
    state: ButtonState,
}

impl<'a> NxButton<'a> {
    pub(super) const fn new(art: &'a str, state: ButtonState) -> Self {
        Self { art, state }
    }

    fn path(self) -> String {
        let state = match self.state {
            ButtonState::Normal => "normal",
            ButtonState::Hovered => "mouseOver",
            ButtonState::Disabled => "disabled",
        };
        format!("{}/{state}/0", self.art)
    }

    pub(super) fn anchored_bounds(self, archive: &NxArchive, x: f32, y: f32) -> Option<UiRect> {
        let (width, height, ox, oy) = nx_dialogue_button_geometry(archive, &self.path())?;
        Some(UiRect::new(x - ox as f32, y - oy as f32, width, height))
    }
}

pub(super) struct NxDialog<'a> {
    pixels: &'a mut [u8],
    width: usize,
    height: usize,
    archive: &'a NxArchive,
}

impl<'a> NxDialog<'a> {
    pub(super) fn new(
        pixels: &'a mut [u8],
        width: usize,
        height: usize,
        archive: &'a NxArchive,
    ) -> Self {
        Self {
            pixels,
            width,
            height,
            archive,
        }
    }

    /// Use an authored frame when it also includes a panel's static contents.
    pub(super) fn frame(&mut self, path: &str, anchor_x: i32, anchor_y: i32) -> Result<(), String> {
        composite_nx_ui_bitmap(
            self.pixels,
            self.width,
            self.height,
            self.archive,
            path,
            anchor_x,
            anchor_y,
        )
    }

    /// Keeps the original offsets of multipart backgrounds and anchored controls.
    pub(super) fn layers(&mut self, layers: &[(&str, i32, i32)]) -> Result<(), String> {
        for &(path, x, y) in layers {
            self.frame(path, x, y)?;
        }
        Ok(())
    }

    /// An authored window keeps its title and content art and uses the common close control.
    pub(super) fn authored_window(
        &mut self,
        layers: &[(&str, i32, i32)],
        close: UiRect,
    ) -> Result<(), String> {
        let frame = self.layers(layers);
        frame.and(self.close(close))
    }

    pub(super) fn close(&mut self, bounds: UiRect) -> Result<(), String> {
        self.buttons(&[(CLOSE_BUTTON_ART, bounds)])
    }

    /// Builds a window for client-laid-out content from the native equipment
    /// frame. Its blank footer is the same treatment as the item window's.
    /// The outer bevels stay at 1:1 and only undecorated pixels are repeated.
    pub(super) fn inventory_window(
        &mut self,
        bounds: UiRect,
        font: &fontdue::Font,
        title: &str,
        close: UiRect,
    ) -> Result<(), String> {
        if bounds.height < 60.0 {
            return Err("inventory window is too short for its chrome".to_owned());
        }
        self.window_chrome(
            bounds,
            font,
            title,
            UiRect::new(
                bounds.x + 8.0,
                bounds.y + 5.0,
                close.x - 4.0 - (bounds.x + 8.0),
                12.0,
            ),
        )?;
        self.close(close)
    }

    /// Builds the shared resizable window frame and title without choosing a
    /// control, so compact windows can place their own buttons in the header.
    pub(super) fn window_chrome(
        &mut self,
        bounds: UiRect,
        font: &fontdue::Font,
        title: &str,
        title_bounds: UiRect,
    ) -> Result<(), String> {
        const HEADER: usize = 30;
        const FOOTER: usize = 30;
        let height = bounds.height as usize;
        if height < HEADER {
            return Err("window is too short for its header".to_owned());
        }
        self.repeat_art(
            "UIWindow.img/Equip/backgrnd",
            bounds,
            (175, 304),
            6,
            124..126,
            |y| match y {
                y if y < HEADER => y,
                y if y >= height - FOOTER => 304 - (height - y),
                y => 266 + y % 2,
            },
        )?;
        self.replace_title(font, title, title_bounds);
        Ok(())
    }

    pub(super) fn title(&mut self, font: &fontdue::Font, title: &str, bounds: UiRect) {
        draw_text_role(
            font,
            self.pixels,
            self.width,
            self.height,
            &title.to_ascii_uppercase(),
            bounds.x as i32,
            bounds.y as i32,
            UiTextRole::Small,
            bounds.width as usize,
            bounds.height as usize,
            Some([29, 35, 43]),
        );
    }

    /// Replaces baked title lettering within the flat area of an NX title bar.
    /// The last column of the supplied bounds must lie on blank title art.
    pub(super) fn replace_title(&mut self, font: &fontdue::Font, title: &str, bounds: UiRect) {
        let left = bounds.x as usize;
        let right = left.saturating_add(bounds.width as usize);
        if left >= right || right > self.width {
            return;
        }
        for row in self
            .pixels
            .chunks_exact_mut(self.width * 4)
            .skip(bounds.y as usize)
            .take(bounds.height as usize)
        {
            let blank = [
                row[right * 4 - 4],
                row[right * 4 - 3],
                row[right * 4 - 2],
                row[right * 4 - 1],
            ];
            for pixel in row[left * 4..right * 4].chunks_exact_mut(4) {
                pixel.copy_from_slice(&blank);
            }
        }
        self.title(font, title, bounds);
    }

    pub(super) fn button(&mut self, bounds: UiRect, button: NxButton<'_>) -> Result<(), String> {
        self.buttons(&[(&button.path(), bounds)])
    }

    /// Repeats the blank strip between the bevel and the stock button label.
    pub(super) fn labeled_button(
        &mut self,
        bounds: UiRect,
        button: NxButton<'_>,
        font: &fontdue::Font,
        label: &str,
    ) -> Result<(), String> {
        let path = button.path();
        let (width, height, _) = self
            .frame_bitmap(&path)
            .ok_or_else(|| format!("missing NX dialog button {path:?}"))?;
        self.repeat_art(&path, bounds, (width, height), 3, 3..5, |y| y)?;
        let enabled = button.state != ButtonState::Disabled;
        draw_control_label(
            font,
            self.pixels,
            self.width,
            self.height,
            &label.to_ascii_uppercase(),
            bounds,
            if enabled {
                [255, 255, 255]
            } else {
                [236, 236, 236]
            },
            if enabled {
                [70, 63, 55]
            } else {
                [111, 108, 105]
            },
        );
        Ok(())
    }

    /// Copies the native left and right caps and tiles a blank strip between them.
    fn repeat_art(
        &mut self,
        path: &str,
        bounds: UiRect,
        source_size: (usize, usize),
        cap: usize,
        repeat: Range<usize>,
        source_row: impl Fn(usize) -> usize,
    ) -> Result<(), String> {
        let (source_width, _, source) = self
            .frame_bitmap(path)
            .filter(|(width, height, _)| (*width, *height) == source_size)
            .ok_or_else(|| format!("missing native-size NX dialog artwork {path:?}"))?;
        let width = bounds.width as usize;
        let height = bounds.height as usize;
        if width < cap * 2 {
            return Err(format!(
                "NX dialog artwork {path:?} is too narrow for both caps"
            ));
        }
        let mut frame = vec![0; width * height * 4];
        let right = (width - cap) * 4;
        for (y, row) in frame.chunks_exact_mut(width * 4).enumerate() {
            let start = source_row(y) * source_width * 4;
            let source = source
                .get(start..start + source_width * 4)
                .filter(|row| {
                    repeat.start < repeat.end && repeat.end <= row.len() / 4 && cap <= source_width
                })
                .ok_or_else(|| format!("NX dialog artwork {path:?} is missing a frame strip"))?;
            row[..cap * 4].copy_from_slice(&source[..cap * 4]);
            row[right..].copy_from_slice(&source[(source_width - cap) * 4..]);
            let strip = &source[repeat.start * 4..repeat.end * 4];
            for tile in row[cap * 4..right].chunks_mut(strip.len()) {
                tile.copy_from_slice(&strip[..tile.len()]);
            }
        }
        composite_bgra_pixels(
            self.pixels,
            self.width,
            self.height,
            &frame,
            width,
            height,
            bounds.x as i32,
            bounds.y as i32,
        );
        Ok(())
    }

    pub(super) fn tab(&mut self, bounds: UiRect, selected: bool) -> Result<(), String> {
        composite_nx_ui_tab(
            self.pixels,
            self.width,
            self.height,
            self.archive,
            bounds,
            selected,
        )
    }

    pub(super) fn labeled_tab(
        &mut self,
        bounds: UiRect,
        selected: bool,
        font: &fontdue::Font,
        label: &str,
    ) -> Result<(), String> {
        let result = self.tab(bounds, selected);
        draw_tab_label(font, self.pixels, self.width, self.height, label, bounds);
        result
    }

    /// Paints each available control while retaining its authored NX anchor.
    pub(super) fn anchored_buttons(&mut self, buttons: &[(&str, i32, i32)]) {
        for &(path, x, y) in buttons {
            let _ = self.frame(path, x, y);
        }
    }

    pub(super) fn scaled_frame(
        &mut self,
        path: &str,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    ) -> Result<(), String> {
        composite_nx_ui_bitmap_scaled_to(
            self.pixels,
            self.width,
            self.height,
            self.archive,
            path,
            x,
            y,
            width,
            height,
        )
    }

    pub(super) fn vertical_frame(
        &mut self,
        [top, center, bottom]: [&str; 3],
        footer_y: i32,
    ) -> Result<(), String> {
        composite_nx_ui_vertical_frame(
            self.pixels,
            self.width,
            self.height,
            self.archive,
            top,
            center,
            bottom,
            0,
            0,
            footer_y,
        )
    }

    /// Use the tiled window frame for panels that lay out their own contents.
    pub(super) fn resizable_frame(&mut self) -> Result<(), String> {
        composite_nx_ui_nine_slice_frame(
            self.pixels,
            self.width,
            self.height,
            self.archive,
            "UIWindow.img/EventWindow/backgrnd",
            0,
            0,
            self.width,
            self.height,
        )
    }

    pub(super) fn frame_bitmap(&self, path: &str) -> Option<(usize, usize, Arc<[u8]>)> {
        let node = resolve_ui_path(&self.archive.root(), path).ok().flatten()?;
        let bitmap = node.bitmap().ok().flatten()?;
        Some((
            usize::from(bitmap.width()),
            usize::from(bitmap.height()),
            bitmap.decode_bgra_shared().ok()?,
        ))
    }

    /// Joins full-width bands at native size when a panel omits baked-in rows.
    pub(super) fn frame_rows(
        &mut self,
        source: &[u8],
        source_width: usize,
        left: i32,
        bands: &[(Range<usize>, i32)],
    ) -> Result<(), String> {
        let stride = source_width * 4;
        for (rows, top) in bands {
            let band = source
                .get(rows.start * stride..rows.end * stride)
                .ok_or_else(|| format!("NX dialog frame is missing rows {rows:?}"))?;
            composite_bgra_pixels(
                self.pixels,
                self.width,
                self.height,
                band,
                source_width,
                rows.len(),
                left,
                *top,
            );
        }
        Ok(())
    }

    /// Pass the same panel-local rectangles to rendering and input handling.
    /// The compositor applies each button's NX origin.
    pub(super) fn buttons(&mut self, buttons: &[(&str, UiRect)]) -> Result<(), String> {
        let mut result = Ok(());
        for &(path, rect) in buttons {
            let painted = composite_nx_ui_bitmap_centered(
                self.pixels,
                self.width,
                self.height,
                self.archive,
                path,
                rect,
            );
            result = result.and(painted);
        }
        result
    }

    /// Draws a control at its NX anchor and returns its visible, panel-local bounds.
    pub(super) fn anchored_button(&mut self, path: &str, x: i32, y: i32) -> Result<UiRect, String> {
        let (width, height, origin_x, origin_y) =
            nx_dialogue_button_geometry(self.archive, path)
                .ok_or_else(|| format!("missing NX dialog button {path:?}"))?;
        self.frame(path, x, y)?;
        Ok(UiRect::new(
            (x - origin_x) as f32,
            (y - origin_y) as f32,
            width,
            height,
        ))
    }
}

/// Tiles cached frame pieces for windows that use their own image compositor.
pub(super) fn tile_frame_area(
    area: UiRect,
    tile_width: f32,
    tile_height: f32,
    mut draw: impl FnMut(f32, f32),
) {
    if tile_width <= 0.0 || tile_height <= 0.0 {
        return;
    }
    let mut y = area.y;
    while y < area.y + area.height {
        let mut x = area.x;
        while x < area.x + area.width {
            draw(x, y);
            x += tile_width;
        }
        y += tile_height;
    }
}

pub(super) const CLOSE_BUTTON_ART: &str = "Basic.img/BtClose/normal/0";

pub(super) const fn close_button(x: f32, y: f32) -> UiRect {
    UiRect::new(x, y, 12.0, 12.0)
}

pub(super) const fn inventory_close(right: f32) -> UiRect {
    close_button(right - 19.0, 6.0)
}

/// Places an action row from left to right in the blue footer.
pub(super) const fn right_aligned_footer<const N: usize>(
    right: f32,
    bottom: f32,
    widths: [f32; N],
) -> [UiRect; N] {
    const BUTTON_HEIGHT: f32 = 18.0;
    const BUTTON_GAP: f32 = 6.0;
    const BOTTOM_INSET: f32 = 6.0;
    let mut buttons = [UiRect::new(0.0, 0.0, 0.0, 0.0); N];
    let top = bottom - BOTTOM_INSET - BUTTON_HEIGHT;
    let mut next_right = right;
    let mut index = N;
    while index > 0 {
        index -= 1;
        let left = next_right - widths[index];
        buttons[index] = UiRect::new(left, top, widths[index], BUTTON_HEIGHT);
        next_right = left - BUTTON_GAP;
    }
    buttons
}
