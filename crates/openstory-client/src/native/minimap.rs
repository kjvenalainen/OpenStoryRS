//! Asset-backed, 1024x768-logical in-game minimap presentation.
//!
//! The caller owns the live map entities and GPU texture. `MiniMapData::load`
//! decodes the map's static minimap once; `render` composes that bitmap, legacy
//! frame art, portals, and caller-provided actor markers into a compact panel.

use std::sync::Arc;

use fontdue::Font;
use openstory_assets::{Bitmap, NodeValue, NxArchive, NxNode};
use openstory_core::viewport::LogicalSize;

const DEFAULT_X: f32 = 0.0;
const DEFAULT_Y: f32 = 0.0;
const DEFAULT_MAP_ZOOM: f32 = 1.5;
const MAX_MAP_ID: i32 = 999_999_999;
const MAX_MAP_BITMAP_SIDE: u32 = 4096;
const MAX_UI_BITMAP_SIDE: u32 = 2048;
const MAX_DYNAMIC_MARKERS: usize = 1024;
const MAX_CUSTOM_MARKER_SIDE: u32 = 512;
const MAX_PORTALS: usize = 512;
const MAX_MAP_LABEL_CHARS: usize = 80;
const MAP_LABEL_SIZE: f32 = 13.0;
const MAP_LABEL_OUTLINE: [u8; 3] = [20, 24, 32];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum MarkerKind {
    Player,
    Npc,
    Mob,
    OtherPlayer,
}

#[derive(Clone, Copy)]
pub(super) struct MarkerSprite<'a> {
    /// BGRA pixels for a single standing frame or other compact marker image.
    pub pixels: &'a [u8],
    pub width: u32,
    pub height: u32,
    /// Sprite origin measured from the top-left; map position anchors at origin.
    pub origin_x: f32,
    pub origin_y: f32,
}

#[derive(Clone, Copy)]
pub(super) struct Marker<'a> {
    pub kind: MarkerKind,
    /// Position in map world coordinates, before the NX minimap transform.
    pub x: f32,
    pub y: f32,
    pub facing_left: bool,
    /// Optional caller-owned actor frame. When present, the legacy minimap actor
    /// scale is applied; otherwise a small `MapHelper.img/minimap` icon is used.
    pub sprite: Option<MarkerSprite<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Presentation {
    Collapsed,
    Normal,
    Expanded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HitAction {
    Expand,
    Collapse,
    OpenWorldMap,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct HitRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl HitRect {
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct HitRegion {
    pub action: HitAction,
    /// Absolute logical-screen coordinates, compatible with the shared viewport.
    pub bounds: HitRect,
}

pub(super) struct Panel {
    pub x: f32,
    pub y: f32,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub hit_regions: Vec<HitRegion>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WorldViewport {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone)]
struct SpriteImage {
    pixels: Arc<[u8]>,
    width: u32,
    height: u32,
    origin_x: f32,
    origin_y: f32,
}

#[derive(Clone, Copy)]
struct MapPortal {
    x: f32,
    y: f32,
}

/// Static minimap content and metadata decoded from one `Map/<map>.img` node.
pub(super) struct MiniMapData {
    pub map_id: i32,
    pub center_x: f32,
    pub center_y: f32,
    pub magnification: u8,
    pub hidden: bool,
    has_map: bool,
    map_mark: Option<SpriteImage>,
    canvas: SpriteImage,
    portals: Vec<MapPortal>,
    marker_icons: Vec<(MarkerKind, SpriteImage)>,
    portal_icon: SpriteImage,
}

#[derive(Clone)]
struct CollapsedFrameArt {
    west: SpriteImage,
    center: SpriteImage,
    east: SpriteImage,
}

#[derive(Clone)]
struct MapFrameArt {
    west: SpriteImage,
    east: SpriteImage,
    north: SpriteImage,
    northwest: SpriteImage,
    northeast: SpriteImage,
    south: SpriteImage,
    southwest: SpriteImage,
    southeast: SpriteImage,
}

/// UI art shared by all map minimaps. Decode NX bitmaps once, then reuse their
/// BGRA buffers whenever moving actors force the panel to refresh.
pub(super) struct UiAssets {
    collapsed: CollapsedFrameArt,
    title: SpriteImage,
    normal: MapFrameArt,
    expanded: MapFrameArt,
    min_normal: SpriteImage,
    min_disabled: SpriteImage,
    max_normal: SpriteImage,
    max_disabled: SpriteImage,
    map: SpriteImage,
}

impl UiAssets {
    pub fn load(archive: &NxArchive) -> Result<Self, String> {
        let ui_root = archive.root();
        // UI.nx is the v83 archive, where the authored minimap pieces live
        // directly under UIWindow.img rather than the newer UIWindow2 tree.
        let minimap_path = "UIWindow.img/MiniMap";
        let minimap_ui = required(&ui_root, minimap_path)?;
        let collapsed_root = required(&minimap_ui, "Min")?;
        let collapsed = CollapsedFrameArt {
            west: load_ui_image(archive, &collapsed_root, minimap_path, "w", "Min/w")?,
            center: load_ui_image(archive, &collapsed_root, minimap_path, "c", "Min/c")?,
            east: load_ui_image(archive, &collapsed_root, minimap_path, "e", "Min/e")?,
        };
        let normal = load_map_frame(archive, &minimap_ui, minimap_path, "MinMap")?;
        let expanded = load_map_frame(archive, &minimap_ui, minimap_path, "MaxMap")?;
        let map_root = required(&minimap_ui, "BtMap")?;
        let min_normal = load_minimap_button(archive, minimap_path, "BtMin/normal/0", false)?;
        let min_disabled = load_minimap_button(archive, minimap_path, "BtMin/disabled/0", true)?;
        let max_normal = load_minimap_button(archive, minimap_path, "BtMax/normal/0", false)?;
        let max_disabled = load_minimap_button(archive, minimap_path, "BtMax/disabled/0", true)?;
        let map = load_ui_image(
            archive,
            &required(&map_root, "normal")?,
            minimap_path,
            "0",
            "BtMap/normal/0",
        )?;
        Ok(Self {
            title: load_ui_image(archive, &minimap_ui, minimap_path, "title", "title")?,
            collapsed,
            normal,
            expanded,
            min_normal,
            min_disabled,
            max_normal,
            max_disabled,
            map,
        })
    }
}

impl MiniMapData {
    /// Loads a map's `miniMap` canvas, transform, portal positions, and minimap
    /// marker art. Maps without a canvas still get the collapsed map-name bar.
    pub fn load(map_archive: &NxArchive, map_id: i32) -> Result<Option<Self>, String> {
        if !(0..=MAX_MAP_ID).contains(&map_id) {
            return Err(format!("minimap map id {map_id} is outside the v83 range"));
        }
        let map_path = format!("Map/Map{}/{map_id:09}.img", map_id / 100_000_000);
        let map = required(&map_archive.root(), &map_path)?;
        let info = required(&map, "info")?;
        let hidden = integer_child(&info, "hideMinimap").unwrap_or(0) != 0;
        let Some(mini_map) = optional_child(&map, "miniMap")? else {
            let empty_image = SpriteImage {
                pixels: Arc::from(vec![0_u8; 4]),
                width: 1,
                height: 1,
                origin_x: 0.0,
                origin_y: 0.0,
            };
            return Ok(Some(Self {
                map_id,
                center_x: 0.0,
                center_y: 0.0,
                magnification: 0,
                hidden,
                has_map: false,
                map_mark: None,
                canvas: empty_image.clone(),
                portals: Vec::new(),
                marker_icons: Vec::new(),
                portal_icon: empty_image,
            }));
        };
        let _width = integer_child(&mini_map, "width")
            .filter(|value| *value > 0)
            .ok_or_else(|| format!("{map_path}/miniMap has no positive width"))?;
        let _height = integer_child(&mini_map, "height")
            .filter(|value| *value > 0)
            .ok_or_else(|| format!("{map_path}/miniMap has no positive height"))?;
        let center_x = integer_child(&mini_map, "centerX")
            .ok_or_else(|| format!("{map_path}/miniMap has no centerX"))?;
        let center_y = integer_child(&mini_map, "centerY")
            .ok_or_else(|| format!("{map_path}/miniMap has no centerY"))?;
        let magnification = integer_child(&mini_map, "mag")
            .filter(|value| (0..=12).contains(value))
            .ok_or_else(|| format!("{map_path}/miniMap has an invalid magnification"))?
            as u8;
        let canvas_node = required(&mini_map, "canvas")?;
        let canvas = sprite_image(
            map_archive,
            &canvas_node,
            &format!("{map_path}/miniMap/canvas"),
            MAX_MAP_BITMAP_SIDE,
        )?;
        if canvas.width == 0 || canvas.height == 0 {
            return Err(format!("{map_path}/miniMap/canvas has empty dimensions"));
        }

        let portals = load_portals(&map, &map_path)?;
        let marker_root = required(&map_archive.root(), "MapHelper.img/minimap")?;
        let marker_icons = [
            (MarkerKind::Player, "user"),
            (MarkerKind::Npc, "npc"),
            // C++ renders current mob animation frames. `match` is the compact
            // NX fallback when a caller does not supply a mob frame.
            (MarkerKind::Mob, "match"),
            (MarkerKind::OtherPlayer, "another"),
        ]
        .into_iter()
        .map(|(kind, name)| {
            let node = required(&marker_root, name)?;
            Ok((
                kind,
                sprite_image(
                    map_archive,
                    &node,
                    &format!("MapHelper.img/minimap/{name}"),
                    MAX_UI_BITMAP_SIDE,
                )?,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
        let portal_node = required(&marker_root, "portal")?;
        let portal_icon = sprite_image(
            map_archive,
            &portal_node,
            "MapHelper.img/minimap/portal",
            MAX_UI_BITMAP_SIDE,
        )?;
        let map_mark = string_child(&info, "mapMark")
            .map(|name| {
                map_archive
                    .root()
                    .resolve(&format!("MapHelper.img/mark/{name}"))
                    .map_err(|error| error.to_string())?
                    .map(|node| {
                        sprite_image(
                            map_archive,
                            &node,
                            &format!("MapHelper.img/mark/{name}"),
                            MAX_UI_BITMAP_SIDE,
                        )
                    })
                    .transpose()
            })
            .transpose()?
            .flatten();
        Ok(Some(Self {
            map_id,
            center_x: center_x as f32,
            center_y: center_y as f32,
            magnification,
            hidden,
            has_map: true,
            map_mark,
            canvas,
            portals,
            marker_icons,
            portal_icon,
        }))
    }

    pub fn has_map(&self) -> bool {
        self.has_map
    }

    fn map_units_per_pixel(&self) -> f32 {
        2.0_f32.powi(i32::from(self.magnification))
    }

    fn project(&self, x: f32, y: f32, zoom: f32) -> (f32, f32) {
        let units = self.map_units_per_pixel();
        (
            (x + self.center_x) / units * zoom,
            (y + self.center_y) / units * zoom,
        )
    }

    fn icon(&self, kind: MarkerKind) -> &SpriteImage {
        &self
            .marker_icons
            .iter()
            .find(|(candidate, _)| *candidate == kind)
            .expect("all minimap marker kinds are loaded")
            .1
    }
}

/// Renders the minimap using its static NX canvas and live marker inputs.
/// The result is a compact BGRA texture positioned in logical game coordinates;
/// the caller can upload it once per map/UI update and draw it at `(x, y)`.
pub(super) fn render(
    data: &MiniMapData,
    ui_assets: &UiAssets,
    viewport: LogicalSize,
    map_name: &str,
    street_name: &str,
    presentation: Presentation,
    world_viewport: Option<WorldViewport>,
    markers: &[Marker<'_>],
    font: &Font,
) -> Result<Panel, String> {
    if !viewport.width.is_finite()
        || !viewport.height.is_finite()
        || viewport.width < 160.0
        || viewport.height < 120.0
        || viewport.width > 4096.0
        || viewport.height > 4096.0
    {
        return Err(format!(
            "minimap viewport has invalid logical dimensions {}x{}",
            viewport.width, viewport.height
        ));
    }
    if markers.len() > MAX_DYNAMIC_MARKERS {
        return Err(format!(
            "minimap received {} markers; maximum is {MAX_DYNAMIC_MARKERS}",
            markers.len()
        ));
    }
    if data.hidden {
        return Err(format!("minimap is hidden on map {}", data.map_id));
    }
    let presentation = if data.has_map {
        presentation
    } else {
        Presentation::Collapsed
    };

    let label: String = map_name.chars().take(MAX_MAP_LABEL_CHARS).collect();
    let street_label: String = street_name.chars().take(MAX_MAP_LABEL_CHARS).collect();
    let label_font = super::ui_foundation::bold_font().unwrap_or(font);
    let name_width = measure_text(label_font, &label, MAP_LABEL_SIZE).max(measure_text(
        label_font,
        &street_label,
        MAP_LABEL_SIZE,
    ));
    let (button_min, button_max, min_action, max_action) = match presentation {
        Presentation::Collapsed => (
            &ui_assets.min_disabled,
            if data.has_map {
                &ui_assets.max_normal
            } else {
                &ui_assets.max_disabled
            },
            None,
            data.has_map.then_some(HitAction::Expand),
        ),
        Presentation::Normal => (
            &ui_assets.min_normal,
            &ui_assets.max_normal,
            Some(HitAction::Collapse),
            Some(HitAction::Expand),
        ),
        Presentation::Expanded => (
            &ui_assets.min_normal,
            &ui_assets.max_disabled,
            Some(HitAction::Collapse),
            None,
        ),
    };
    let button_map = &ui_assets.map;

    let x = DEFAULT_X.min((viewport.width - 8.0).max(0.0));
    let y = DEFAULT_Y.min((viewport.height - 8.0).max(0.0));
    let mut hit_regions = Vec::new();
    let (panel_width, panel_height, pixels) = match presentation {
        Presentation::Collapsed => {
            let collapsed_name_width = measure_text(label_font, &label, 11.0);
            let desired_width = if data.has_map {
                149.0
            } else {
                (collapsed_name_width
                    + button_min.width as f32
                    + button_max.width as f32
                    + button_map.width as f32
                    + 20.0)
                    .max(149.0)
            };
            let width = desired_width.min(viewport.width - x).max(1.0);
            let mut canvas = Canvas::new(width as u32, 20)?;
            draw_collapsed_frame(&mut canvas, &ui_assets.collapsed, width);
            if data.has_map {
                canvas.draw_image(
                    &ui_assets.title,
                    9.0,
                    8.0,
                    ui_assets.title.width as f32,
                    ui_assets.title.height as f32,
                    false,
                );
            } else {
                canvas.draw_text_outlined(
                    label_font,
                    &label,
                    9.0,
                    3.0,
                    11.0,
                    [255, 255, 255],
                    MAP_LABEL_OUTLINE,
                    width
                        - button_min.width as f32
                        - button_max.width as f32
                        - button_map.width as f32
                        - 25.0,
                );
            }
            let mut button_x = width
                - button_min.width as f32
                - button_max.width as f32
                - button_map.width as f32
                - 9.0;
            let _ = draw_button(&mut canvas, button_min, button_x, 4.0, x, y, None);
            button_x += button_min.width as f32 + 1.0;
            if let Some(hit) = draw_button(&mut canvas, button_max, button_x, 4.0, x, y, max_action)
            {
                hit_regions.push(hit);
            }
            button_x += button_max.width as f32 + 1.0;
            if let Some(hit) = draw_button(
                &mut canvas,
                button_map,
                button_x,
                4.0,
                x,
                y,
                Some(HitAction::OpenWorldMap),
            ) {
                hit_regions.push(hit);
            }
            (width as u32, 20, canvas.into_pixels())
        }
        Presentation::Normal | Presentation::Expanded => {
            if !data.has_map {
                return Err(format!("minimap map {} has no canvas", data.map_id));
            }
            let expanded = presentation == Presentation::Expanded;
            let canvas_y = if expanded { 60.0 } else { 29.0 };
            let map_height = data.canvas.height as f32;
            let map_width = data.canvas.width as f32;
            let available_width = (viewport.width - x - 16.0).max(64.0);
            let available_height = (viewport.height - y - canvas_y - 7.0).max(64.0);
            let base_zoom = if expanded { DEFAULT_MAP_ZOOM } else { 1.0 };
            let zoom = base_zoom
                .min(available_width / map_width.max(1.0))
                .min(available_height / map_height.max(1.0))
                .max(0.1);
            let zoomed_width = (map_width * zoom).ceil().max(1.0);
            let zoomed_height = (map_height * zoom).ceil().max(1.0);
            // Keep the normal panel sized to the map. Expanded mode needs a
            // small title area beside the map mark, while controls remain
            // positioned in the frame's existing right-hand control zone.
            let text_width = if expanded { name_width + 78.0 } else { 0.0 };
            let width = (zoomed_width + 20.0)
                .max(text_width)
                .max(178.0)
                .min(viewport.width - x)
                .max(1.0);
            let height = (zoomed_height + canvas_y + 14.0)
                .min(viewport.height - y)
                .max(1.0);
            let mut canvas = Canvas::new(width as u32, height as u32)?;
            for row in canvas_y as usize..height as usize - 14 {
                for column in 6..width as usize - 6 {
                    let index = (row * width as usize + column) * 4;
                    canvas.pixels[index..index + 4].copy_from_slice(&[75, 65, 65, 255]);
                }
            }
            draw_map_frame(
                &mut canvas,
                if expanded {
                    &ui_assets.expanded
                } else {
                    &ui_assets.normal
                },
                width,
                zoomed_height,
                canvas_y,
            );
            if expanded && let Some(map_mark) = data.map_mark.as_ref() {
                canvas.draw_image(
                    map_mark,
                    7.0,
                    21.0,
                    map_mark.width as f32,
                    map_mark.height as f32,
                    false,
                );
            }
            canvas.draw_image(
                &data.canvas,
                ((width - zoomed_width) * 0.5).round(),
                canvas_y,
                zoomed_width,
                zoomed_height,
                false,
            );
            if let Some(world_viewport) = world_viewport {
                let (left, top) = data.project(world_viewport.left, world_viewport.top, zoom);
                let (right, bottom) = data.project(
                    world_viewport.left + world_viewport.width,
                    world_viewport.top + world_viewport.height,
                    zoom,
                );
                canvas.draw_outline_clipped(
                    (width - zoomed_width) * 0.5 + left,
                    canvas_y + top,
                    right - left,
                    bottom - top,
                    (width - zoomed_width) * 0.5,
                    canvas_y,
                    (width - zoomed_width) * 0.5 + zoomed_width,
                    canvas_y + zoomed_height,
                    [0, 255, 255, 220],
                );
            }
            for portal in &data.portals {
                let (map_x, map_y) = data.project(portal.x, portal.y, zoom);
                if map_x < 0.0 || map_y < 0.0 || map_x >= zoomed_width || map_y >= zoomed_height {
                    continue;
                }
                canvas.draw_image(
                    &data.portal_icon,
                    (width - zoomed_width) * 0.5 + map_x - data.portal_icon.width as f32 * 0.5,
                    // World positions anchor at the feet/base, as they do for
                    // actor markers and the full-size portal animation.
                    canvas_y + map_y - data.portal_icon.height as f32,
                    data.portal_icon.width as f32,
                    data.portal_icon.height as f32,
                    false,
                );
            }
            for marker in markers {
                if !marker.x.is_finite() || !marker.y.is_finite() {
                    continue;
                }
                let (map_x, map_y) = data.project(marker.x, marker.y, zoom);
                if map_x < 0.0 || map_y < 0.0 || map_x >= zoomed_width || map_y >= zoomed_height {
                    continue;
                }
                let anchor_x = (width - zoomed_width) * 0.5 + map_x;
                let anchor_y = canvas_y + map_y;
                if let Some(sprite) = marker.sprite {
                    validate_marker_sprite(sprite)?;
                    let marker_scale = match marker.kind {
                        MarkerKind::Player | MarkerKind::OtherPlayer => 0.16 * zoom,
                        MarkerKind::Npc => 0.14 * zoom,
                        MarkerKind::Mob => 0.12 * zoom,
                    };
                    canvas.draw_pixels(
                        sprite.pixels,
                        sprite.width,
                        sprite.height,
                        anchor_x - sprite.origin_x * marker_scale,
                        anchor_y - sprite.origin_y * marker_scale,
                        sprite.width as f32 * marker_scale,
                        sprite.height as f32 * marker_scale,
                        marker.facing_left,
                    );
                } else {
                    let icon = data.icon(marker.kind);
                    canvas.draw_image(
                        icon,
                        anchor_x - icon.width as f32 * 0.5,
                        anchor_y - icon.height as f32,
                        icon.width as f32,
                        icon.height as f32,
                        false,
                    );
                }
            }
            if expanded {
                canvas.draw_text_outlined(
                    label_font,
                    &label,
                    48.0,
                    27.0,
                    MAP_LABEL_SIZE,
                    [255, 255, 255],
                    MAP_LABEL_OUTLINE,
                    width - 130.0,
                );
                canvas.draw_text_outlined(
                    label_font,
                    &street_label,
                    48.0,
                    41.0,
                    MAP_LABEL_SIZE,
                    [255, 255, 255],
                    MAP_LABEL_OUTLINE,
                    width - 130.0,
                );
            }
            canvas.draw_image(
                &ui_assets.title,
                9.0,
                8.0,
                ui_assets.title.width as f32,
                ui_assets.title.height as f32,
                false,
            );
            let collapse_x = width
                - button_min.width as f32
                - button_max.width as f32
                - button_map.width as f32
                - 8.0;
            if let Some(hit) =
                draw_button(&mut canvas, button_min, collapse_x, 4.0, x, y, min_action)
            {
                hit_regions.push(hit);
            }
            let expand_x = width - button_max.width as f32 - button_map.width as f32 - 5.0;
            if let Some(hit) = draw_button(&mut canvas, button_max, expand_x, 4.0, x, y, max_action)
            {
                hit_regions.push(hit);
            }
            if let Some(hit) = draw_button(
                &mut canvas,
                button_map,
                expand_x + button_max.width as f32 + 1.0,
                4.0,
                x,
                y,
                Some(HitAction::OpenWorldMap),
            ) {
                hit_regions.push(hit);
            }
            (width as u32, height as u32, canvas.into_pixels())
        }
    };

    Ok(Panel {
        x,
        y,
        width: panel_width,
        height: panel_height,
        pixels,
        hit_regions,
    })
}

fn load_portals(map: &NxNode, map_path: &str) -> Result<Vec<MapPortal>, String> {
    let Some(portal_node) = optional_child(map, "portal")? else {
        return Ok(Vec::new());
    };
    let mut portals = Vec::new();
    for portal in portal_node.children().map_err(|error| error.to_string())? {
        if portals.len() >= MAX_PORTALS {
            return Err(format!(
                "{map_path}/portal exceeds the {MAX_PORTALS}-entry minimap limit"
            ));
        }
        if integer_child(&portal, "pt") != Some(2) {
            continue;
        }
        let (Some(x), Some(y)) = (integer_child(&portal, "x"), integer_child(&portal, "y")) else {
            continue;
        };
        portals.push(MapPortal {
            x: x as f32,
            y: y as f32,
        });
    }
    Ok(portals)
}

fn load_ui_image(
    archive: &NxArchive,
    root: &NxNode,
    minimap_path: &str,
    child_path: &str,
    label: &str,
) -> Result<SpriteImage, String> {
    sprite_image(
        archive,
        &required(root, child_path)?,
        &format!("{minimap_path}/{label}"),
        MAX_UI_BITMAP_SIDE,
    )
}

/// The v83 minimap sheet does not contain the newer zoom buttons. Reuse the
/// matching v83 Basic controls so the retained zoom interactions stay NX-backed.
fn load_minimap_button(
    archive: &NxArchive,
    minimap_path: &str,
    path: &str,
    disabled: bool,
) -> Result<SpriteImage, String> {
    let full_path = format!("{minimap_path}/{path}");
    if let Some(node) = super::resolve_ui_path(&archive.root(), &full_path)? {
        return sprite_image(archive, &node, &full_path, MAX_UI_BITMAP_SIDE);
    }
    let button = if path.starts_with("BtMax/") {
        "BtMax"
    } else {
        "BtMin"
    };
    let state = if disabled { "disabled" } else { "normal" };
    let fallback_path = format!("Basic.img/{button}/{state}/0");
    let node = super::resolve_ui_path(&archive.root(), &fallback_path)?.ok_or_else(|| {
        format!("missing v83 minimap control {full_path:?} and {fallback_path:?}")
    })?;
    sprite_image(archive, &node, &fallback_path, MAX_UI_BITMAP_SIDE)
}

fn load_map_frame(
    archive: &NxArchive,
    minimap_ui: &NxNode,
    minimap_path: &str,
    frame_name: &str,
) -> Result<MapFrameArt, String> {
    let frame = required(minimap_ui, frame_name)?;
    let prefix = frame_name.to_owned();
    let load = |child: &str| {
        load_ui_image(
            archive,
            &frame,
            minimap_path,
            child,
            &format!("{prefix}/{child}"),
        )
    };
    Ok(MapFrameArt {
        west: load("w")?,
        east: load("e")?,
        north: load("n")?,
        northwest: load("nw")?,
        northeast: load("ne")?,
        south: load("s")?,
        southwest: load("sw")?,
        southeast: load("se")?,
    })
}

fn draw_collapsed_frame(canvas: &mut Canvas, frame: &CollapsedFrameArt, width: f32) {
    let left = &frame.west;
    let center = &frame.center;
    let right = &frame.east;
    canvas.draw_image(left, 0.0, 0.0, left.width as f32, left.height as f32, false);
    canvas.tile_image(
        center,
        left.width as f32,
        0.0,
        (width - left.width as f32 - right.width as f32).max(0.0),
        center.height as f32,
    );
    canvas.draw_image(
        right,
        width - right.width as f32,
        0.0,
        right.width as f32,
        right.height as f32,
        false,
    );
}

fn draw_map_frame(
    canvas: &mut Canvas,
    frame: &MapFrameArt,
    width: f32,
    map_height: f32,
    canvas_y: f32,
) {
    let left = frame.northwest.width as f32;
    let right = frame.northeast.width as f32;
    let inner_width = (width - left - right).max(0.0);
    // These are six-pixel v83 corners, not the 64-pixel title zones used by
    // later archives. Tile the one-pixel edges at their native thickness.
    canvas.draw_image(
        &frame.northwest,
        0.0,
        0.0,
        left,
        frame.northwest.height as f32,
        false,
    );
    canvas.tile_image(
        &frame.north,
        left,
        0.0,
        inner_width,
        frame.north.height as f32,
    );
    canvas.draw_image(
        &frame.northeast,
        width - right,
        0.0,
        right,
        frame.northeast.height as f32,
        false,
    );
    canvas.tile_image(
        &frame.west,
        0.0,
        frame.northwest.height as f32,
        frame.west.width as f32,
        (canvas_y + map_height - frame.northwest.height as f32).max(0.0),
    );
    canvas.tile_image(
        &frame.east,
        width - frame.east.width as f32,
        frame.northeast.height as f32,
        frame.east.width as f32,
        (canvas_y + map_height - frame.northeast.height as f32).max(0.0),
    );
    let bottom_y = canvas_y + map_height;
    canvas.draw_image(
        &frame.southwest,
        0.0,
        bottom_y,
        frame.southwest.width as f32,
        frame.southwest.height as f32,
        false,
    );
    canvas.tile_image(
        &frame.south,
        frame.southwest.width as f32,
        bottom_y,
        width - frame.southwest.width as f32 - frame.southeast.width as f32,
        frame.south.height as f32,
    );
    canvas.draw_image(
        &frame.southeast,
        width - frame.southeast.width as f32,
        bottom_y,
        frame.southeast.width as f32,
        frame.southeast.height as f32,
        false,
    );
}

fn draw_button(
    canvas: &mut Canvas,
    image: &SpriteImage,
    x: f32,
    y: f32,
    panel_x: f32,
    panel_y: f32,
    action: Option<HitAction>,
) -> Option<HitRegion> {
    canvas.draw_image(
        image,
        x - image.origin_x,
        y - image.origin_y,
        image.width as f32,
        image.height as f32,
        false,
    );
    action.map(|action| HitRegion {
        action,
        bounds: HitRect {
            x: panel_x + x - image.origin_x,
            y: panel_y + y - image.origin_y,
            width: image.width as f32,
            height: image.height as f32,
        },
    })
}

fn sprite_image(
    archive: &NxArchive,
    node: &NxNode,
    path: &str,
    max_side: u32,
) -> Result<SpriteImage, String> {
    let (bitmap, bitmap_node) = resolve_bitmap(archive, node, path)?;
    let width = u32::from(bitmap.width());
    let height = u32::from(bitmap.height());
    if width == 0 || height == 0 || width > max_side || height > max_side {
        return Err(format!(
            "NX sprite {path:?} has invalid dimensions {width}x{height}"
        ));
    }
    let expected = width as usize * height as usize * 4;
    let pixels = bitmap
        .decode_bgra_shared()
        .map_err(|error| format!("could not decode NX sprite {path:?}: {error}"))?;
    if pixels.len() != expected {
        return Err(format!(
            "NX sprite {path:?} decoded to an invalid pixel length"
        ));
    }
    let origin = vector_child(node, "origin").or_else(|| vector_child(&bitmap_node, "origin"));
    let (origin_x, origin_y) = origin.unwrap_or((0, 0));
    Ok(SpriteImage {
        pixels,
        width,
        height,
        origin_x: origin_x as f32,
        origin_y: origin_y as f32,
    })
}

fn resolve_bitmap(
    archive: &NxArchive,
    node: &NxNode,
    path: &str,
) -> Result<(Bitmap, NxNode), String> {
    if let Some(bitmap) = node.bitmap().map_err(|error| error.to_string())? {
        return Ok((bitmap, node.clone()));
    }
    let source = node
        .child("source")
        .map_err(|error| error.to_string())?
        .and_then(|node| string_value(&node))
        .ok_or_else(|| format!("NX node {path:?} is not a bitmap and has no source"))?;
    let source = source
        .split_once('/')
        .map_or(source.as_str(), |(_, path)| path);
    let source_node = super::resolve_ui_path(&archive.root(), source)?
        .ok_or_else(|| format!("NX bitmap source {source:?} for {path:?} is missing"))?;
    let bitmap = source_node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("NX bitmap source {source:?} for {path:?} is not a bitmap"))?;
    Ok((bitmap, source_node))
}

fn required(root: &NxNode, path: &str) -> Result<NxNode, String> {
    super::resolve_ui_path(root, path)?.ok_or_else(|| format!("NX node {path:?} was not found"))
}

fn optional_child(root: &NxNode, path: &str) -> Result<Option<NxNode>, String> {
    root.child(path).map_err(|error| error.to_string())
}

fn integer_child(root: &NxNode, path: &str) -> Option<i32> {
    match root.child(path).ok().flatten()?.value().ok()? {
        NodeValue::Integer(value) => i32::try_from(value).ok(),
        _ => None,
    }
}

fn string_child(root: &NxNode, path: &str) -> Option<String> {
    string_value(&root.child(path).ok().flatten()?)
}

fn string_value(node: &NxNode) -> Option<String> {
    match node.value().ok()? {
        NodeValue::String(value) if !value.is_empty() => Some(value),
        _ => None,
    }
}

fn vector_child(root: &NxNode, path: &str) -> Option<(i32, i32)> {
    match root.child(path).ok().flatten()?.value().ok()? {
        NodeValue::Vector { x, y } => Some((x, y)),
        _ => None,
    }
}

fn validate_marker_sprite(sprite: MarkerSprite<'_>) -> Result<(), String> {
    let expected = (sprite.width as usize)
        .checked_mul(sprite.height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "minimap marker sprite dimensions overflow".to_owned())?;
    if sprite.width == 0
        || sprite.height == 0
        || sprite.width > MAX_CUSTOM_MARKER_SIDE
        || sprite.height > MAX_CUSTOM_MARKER_SIDE
        || expected != sprite.pixels.len()
        || !sprite.origin_x.is_finite()
        || !sprite.origin_y.is_finite()
    {
        return Err("minimap marker sprite has invalid dimensions or pixel data".to_owned());
    }
    Ok(())
}

fn measure_text(font: &Font, text: &str, size: f32) -> f32 {
    let mut width = 0.0;
    let mut previous = None;
    for character in text.chars().take(MAX_MAP_LABEL_CHARS) {
        if let Some(previous) = previous {
            width += font
                .horizontal_kern(previous, character, size)
                .unwrap_or(0.0);
        }
        width += super::rasterize_ui_glyph(font, character, size)
            .0
            .advance_width;
        previous = Some(character);
    }
    width
}

struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl Canvas {
    fn new(width: u32, height: u32) -> Result<Self, String> {
        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err(format!(
                "minimap panel dimensions {width}x{height} are invalid"
            ));
        }
        let length = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|length| *length <= 64 * 1024 * 1024)
            .ok_or_else(|| "minimap panel exceeds the 64 MiB render bound".to_owned())?;
        Ok(Self {
            width: width as usize,
            height: height as usize,
            pixels: vec![0; length],
        })
    }

    fn into_pixels(self) -> Vec<u8> {
        self.pixels
    }

    fn draw_image(
        &mut self,
        image: &SpriteImage,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        flip_x: bool,
    ) {
        self.draw_pixels(
            image.pixels.as_ref(),
            image.width,
            image.height,
            x,
            y,
            width,
            height,
            flip_x,
        );
    }

    fn tile_image(&mut self, image: &SpriteImage, x: f32, y: f32, width: f32, height: f32) {
        let area = super::ui_foundation::UiRect::new(x, y, width, height);
        let tile_width = image.width as f32;
        let tile_height = image.height as f32;
        super::ui_dialog::tile_frame_area(area, tile_width, tile_height, |x, y| {
            self.draw_image(image, x, y, tile_width, tile_height, false);
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_outline_clipped(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        clip_left: f32,
        clip_top: f32,
        clip_right: f32,
        clip_bottom: f32,
        color: [u8; 4],
    ) {
        if ![
            x,
            y,
            width,
            height,
            clip_left,
            clip_top,
            clip_right,
            clip_bottom,
        ]
        .into_iter()
        .all(f32::is_finite)
            || width <= 0.0
            || height <= 0.0
            || clip_right <= clip_left
            || clip_bottom <= clip_top
        {
            return;
        }

        let left = x.round() as i32;
        let right = (x + width).round() as i32;
        let top = y.round() as i32;
        let bottom = (y + height).round() as i32;
        let min_x = (clip_left.ceil() as i32).max(0);
        let max_x = (clip_right.floor() as i32).min(self.width as i32);
        let min_y = (clip_top.ceil() as i32).max(0);
        let max_y = (clip_bottom.floor() as i32).min(self.height as i32);

        let mut blend_at = |pixel_x: i32, pixel_y: i32| {
            if pixel_x < min_x || pixel_x >= max_x || pixel_y < min_y || pixel_y >= max_y {
                return;
            }
            let index = (pixel_y as usize * self.width + pixel_x as usize) * 4;
            blend_bgra(&mut self.pixels[index..index + 4], &color);
        };

        if top >= min_y && top < max_y {
            for pixel_x in left.max(min_x)..right.min(max_x) {
                blend_at(pixel_x, top);
            }
        }
        if bottom >= min_y && bottom < max_y {
            for pixel_x in left.max(min_x)..right.min(max_x) {
                blend_at(pixel_x, bottom);
            }
        }
        if left >= min_x && left < max_x {
            for pixel_y in top.max(min_y)..bottom.min(max_y) {
                blend_at(left, pixel_y);
            }
        }
        if right >= min_x && right < max_x {
            for pixel_y in top.max(min_y)..bottom.min(max_y) {
                blend_at(right, pixel_y);
            }
        }
    }

    fn draw_pixels(
        &mut self,
        source_pixels: &[u8],
        source_width: u32,
        source_height: u32,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        flip_x: bool,
    ) {
        if !x.is_finite()
            || !y.is_finite()
            || !width.is_finite()
            || !height.is_finite()
            || source_width == 0
            || source_height == 0
            || source_pixels.len() != source_width as usize * source_height as usize * 4
            || width <= 0.0
            || height <= 0.0
        {
            return;
        }
        let left = x.floor() as i32;
        let top = y.floor() as i32;
        let right = (x + width).ceil() as i32;
        let bottom = (y + height).ceil() as i32;
        let source_width = source_width as usize;
        let source_height = source_height as usize;
        for dest_y in top.max(0)..bottom.min(self.height as i32) {
            let v = ((dest_y as f32 + 0.5 - y) / height).clamp(0.0, 0.999_999);
            let source_y = (v * source_height as f32) as usize;
            for dest_x in left.max(0)..right.min(self.width as i32) {
                let u = ((dest_x as f32 + 0.5 - x) / width).clamp(0.0, 0.999_999);
                let source_x = ((if flip_x { 1.0 - u } else { u }) * source_width as f32)
                    .min(source_width.saturating_sub(1) as f32)
                    as usize;
                let source = (source_y * source_width + source_x) * 4;
                let destination = (dest_y as usize * self.width + dest_x as usize) * 4;
                blend_bgra(
                    &mut self.pixels[destination..destination + 4],
                    &source_pixels[source..source + 4],
                );
            }
        }
    }

    fn draw_text(
        &mut self,
        font: &Font,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: [u8; 3],
        max_width: f32,
    ) {
        let mut pen_x = x;
        let baseline = y.round() as i32 + size.ceil() as i32;
        let mut previous = None;
        for character in text.chars().take(MAX_MAP_LABEL_CHARS) {
            if let Some(previous) = previous {
                pen_x += font
                    .horizontal_kern(previous, character, size)
                    .unwrap_or(0.0);
            }
            let (metrics, coverage) = super::rasterize_ui_glyph(font, character, size);
            let left = pen_x.round() as i32 + metrics.xmin;
            let top = baseline - metrics.ymin - metrics.height as i32;
            for row in 0..metrics.height {
                for column in 0..metrics.width {
                    let pixel_x = left + column as i32;
                    let pixel_y = top + row as i32;
                    if pixel_x < 0
                        || pixel_y < 0
                        || pixel_x >= self.width as i32
                        || pixel_y >= self.height as i32
                    {
                        continue;
                    }
                    let alpha = coverage[row * metrics.width + column];
                    let source = [color[2], color[1], color[0], alpha];
                    let destination = (pixel_y as usize * self.width + pixel_x as usize) * 4;
                    blend_bgra(&mut self.pixels[destination..destination + 4], &source);
                }
            }
            pen_x += metrics.advance_width;
            if pen_x - x > max_width {
                break;
            }
            previous = Some(character);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_text_outlined(
        &mut self,
        font: &Font,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: [u8; 3],
        outline: [u8; 3],
        max_width: f32,
    ) {
        for (offset_x, offset_y) in [
            (-1.0, -1.0),
            (0.0, -1.0),
            (1.0, -1.0),
            (-1.0, 0.0),
            (1.0, 0.0),
            (-1.0, 1.0),
            (0.0, 1.0),
            (1.0, 1.0),
        ] {
            self.draw_text(
                font,
                text,
                x + offset_x,
                y + offset_y,
                size,
                outline,
                max_width,
            );
        }
        self.draw_text(font, text, x, y, size, color, max_width);
    }
}

fn blend_bgra(destination: &mut [u8], source: &[u8]) {
    let alpha = u32::from(source[3]);
    if alpha == 0 {
        return;
    }
    let destination_alpha = u32::from(destination[3]);
    let inverse = 255 - alpha;
    let output_alpha = alpha + (destination_alpha * inverse + 127) / 255;
    for channel in 0..3 {
        let premultiplied = u32::from(source[channel]) * alpha
            + (u32::from(destination[channel]) * destination_alpha * inverse + 127) / 255;
        destination[channel] = if output_alpha == 0 {
            0
        } else {
            ((premultiplied + output_alpha / 2) / output_alpha).min(255) as u8
        };
    }
    destination[3] = output_alpha.min(255) as u8;
}
