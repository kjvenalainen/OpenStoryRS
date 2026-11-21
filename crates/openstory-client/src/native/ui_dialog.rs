//! Draws NX dialog chrome and controls around panel-owned content.
use super::ui_foundation::UiRect;
use super::{
    NxArchive, composite_bgra_pixels, composite_nx_ui_bitmap, composite_nx_ui_bitmap_centered,
    composite_nx_ui_bitmap_scaled_to, composite_nx_ui_nine_slice_frame,
    composite_nx_ui_vertical_frame, nx_dialogue_button_geometry, resolve_ui_path,
};
use std::{ops::Range, sync::Arc};

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
