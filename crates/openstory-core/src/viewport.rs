//! Logical game coordinates mapped into a resizable physical framebuffer.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalSize {
    pub width: f32,
    pub height: f32,
}

impl LogicalSize {
    pub const LEGACY_1024X768: Self = Self {
        width: 1024.0,
        height: 768.0,
    };

    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// Left edge in framebuffer pixels.
    pub x: f32,
    /// Top edge in framebuffer pixels.
    pub y: f32,
    /// Width in framebuffer pixels.
    pub width: f32,
    /// Height in framebuffer pixels.
    pub height: f32,
    /// Uniform logical-to-physical scale.
    pub scale: f32,
    logical: LogicalSize,
}

impl Viewport {
    /// Fits the logical canvas into the framebuffer while preserving aspect ratio.
    /// The caller should clear the full framebuffer so the unused bars are stable.
    pub fn fit(logical: LogicalSize, physical: PhysicalSize) -> Option<Self> {
        if !logical.width.is_finite()
            || !logical.height.is_finite()
            || logical.width <= 0.0
            || logical.height <= 0.0
            || physical.width == 0
            || physical.height == 0
        {
            return None;
        }

        let scale =
            (physical.width as f32 / logical.width).min(physical.height as f32 / logical.height);
        let width = logical.width * scale;
        let height = logical.height * scale;

        Some(Self {
            x: (physical.width as f32 - width) * 0.5,
            y: (physical.height as f32 - height) * 0.5,
            width,
            height,
            scale,
            logical,
        })
    }

    /// Converts framebuffer coordinates into logical game coordinates.
    /// Returns `None` for clicks in the letterbox bars.
    pub fn to_logical(self, point: Point) -> Option<Point> {
        if point.x < self.x
            || point.y < self.y
            || point.x >= self.x + self.width
            || point.y >= self.y + self.height
        {
            return None;
        }

        Some(Point {
            x: (point.x - self.x) / self.scale,
            y: (point.y - self.y) / self.scale,
        })
    }

    pub fn logical_size(self) -> LogicalSize {
        self.logical
    }
}
