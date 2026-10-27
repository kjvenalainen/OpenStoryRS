use fontdue::Font;
use openstory_assets::{NodeValue, NxArchive, NxNode};
use openstory_render::GpuTexture;
use std::collections::HashMap;

const VIEW_WIDTH: f32 = 1024.0;
const PANEL_TOP: f32 = 20.0;
const MAX_BITMAP_SIDE: u32 = 4096;
const MAX_SEARCH_INDEX_ENTRIES: usize = 10_000;
pub(super) const MAX_VISIBLE_SEARCH_RESULTS: usize = 12;
const SEARCH_RESULT_TOP: f32 = 54.0;
const SEARCH_RESULT_HEIGHT: f32 = 34.0;

pub(super) struct SearchEntry {
    pub map_id: i32,
    pub title: String,
    pub world_map: String,
    title_key: String,
    description_key: String,
    map_id_key: String,
}

impl SearchEntry {
    fn match_rank(&self, query: &str) -> Option<usize> {
        if self.title_key == query {
            Some(0)
        } else if self.title_key.starts_with(query) {
            Some(1)
        } else if self.title_key.contains(query) {
            Some(2)
        } else if self.map_id_key.contains(query) {
            Some(3)
        } else if self.description_key.contains(query) {
            Some(4)
        } else {
            None
        }
    }
}

#[derive(Default)]
pub(super) struct SearchCatalog {
    entries: Vec<SearchEntry>,
}

#[derive(Clone)]
pub(super) struct SearchHit {
    pub bounds: HitRect,
    pub map_id: i32,
    pub world_map: String,
}

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Debug)]
pub(super) struct LinkHit {
    pub bounds: HitRect,
    pub target: String,
}

#[derive(Clone)]
pub(super) struct MarkerHit {
    pub bounds: HitRect,
    pub map_id: i32,
    pub map_ids: Vec<i32>,
    pub title: String,
    pub description: String,
    path: Option<NxNode>,
}

pub(super) struct Panel {
    pub width: u32,
    pub height: u32,
    pub x: f32,
    pub y: f32,
    pub pixels: Vec<u8>,
    pub close: HitRect,
    pub search_toggle: Option<HitRect>,
    pub search_close: Option<HitRect>,
    pub search_input: Option<HitRect>,
    pub search_submit: Option<HitRect>,
    pub search_hits: Vec<SearchHit>,
    pub search_result_count: usize,
    pub links: Vec<LinkHit>,
    pub markers: Vec<MarkerHit>,
    pub map_path: String,
    pub parent_map: Option<String>,
}

pub(super) struct State {
    pub open: bool,
    pub search_open: bool,
    pub texture: Option<GpuTexture>,
    pub map_id: Option<i32>,
    pub map_path: Option<String>,
    pub parent_map: Option<String>,
    pub close: Option<HitRect>,
    pub search_toggle: Option<HitRect>,
    pub search_close: Option<HitRect>,
    pub search_input: Option<HitRect>,
    pub search_submit: Option<HitRect>,
    pub search_hits: Vec<SearchHit>,
    pub search_result_count: usize,
    pub search_catalog: Option<SearchCatalog>,
    pub search_query: String,
    pub search_scroll: usize,
    pub search_focused: bool,
    pub selected_route_map_id: Option<i32>,
    pub links: Vec<LinkHit>,
    pub markers: Vec<MarkerHit>,
    pub hovered_link: Option<usize>,
    pub hovered_marker: Option<usize>,
    pub x: f32,
    pub y: f32,
    pub width: u32,
    pub height: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            open: false,
            search_open: true,
            texture: None,
            map_id: None,
            map_path: None,
            parent_map: None,
            close: None,
            search_toggle: None,
            search_close: None,
            search_input: None,
            search_submit: None,
            search_hits: Vec::new(),
            search_result_count: 0,
            search_catalog: None,
            search_query: String::new(),
            search_scroll: 0,
            search_focused: false,
            selected_route_map_id: None,
            links: Vec::new(),
            markers: Vec::new(),
            hovered_link: None,
            hovered_marker: None,
            x: 0.0,
            y: 0.0,
            width: 0,
            height: 0,
        }
    }
}

impl State {
    pub fn set_layout(&mut self, panel: Panel) {
        self.x = panel.x;
        self.y = panel.y;
        self.width = panel.width;
        self.height = panel.height;
        self.close = Some(panel.close);
        self.search_toggle = panel.search_toggle;
        self.search_close = panel.search_close;
        self.search_input = panel.search_input;
        self.search_submit = panel.search_submit;
        self.search_hits = panel.search_hits;
        self.search_result_count = panel.search_result_count;
        self.links = panel.links;
        self.markers = panel.markers;
        if self
            .hovered_link
            .is_some_and(|index| index >= self.links.len())
        {
            self.hovered_link = None;
        }
        if self
            .hovered_marker
            .is_some_and(|index| index >= self.markers.len())
        {
            self.hovered_marker = None;
        }
        self.map_path = Some(panel.map_path);
        self.parent_map = panel.parent_map;
    }
}

pub(super) fn load_search_catalog(
    map_archive: &NxArchive,
    strings: &NxArchive,
) -> Result<SearchCatalog, String> {
    let map_folder = resolve(&map_archive.root(), "WorldMap")?;
    let mut entries = HashMap::new();
    for region in children(&map_folder)? {
        let Some(region_file) = region.name().ok() else {
            continue;
        };
        if !region_file.starts_with("WorldMap") || region_file == "WorldMap.img" {
            continue;
        }
        let Some(world_map) = region_file.strip_suffix(".img") else {
            continue;
        };
        let Some(map_list) = optional_child(&region, "MapList")? else {
            continue;
        };
        for marker in children(&map_list)? {
            let map_ids = integer_children(&marker, "mapNo");
            if map_ids.is_empty() {
                continue;
            }
            let direct_title = string_child(&marker, "title");
            let direct_description = string_child(&marker, "desc");
            for map_id in map_ids {
                if entries.len() >= MAX_SEARCH_INDEX_ENTRIES && !entries.contains_key(&map_id) {
                    continue;
                }
                let category = super::map_string_category(map_id);
                let title = direct_title
                    .clone()
                    .or_else(|| super::map_string_name(strings, map_id))
                    .unwrap_or_else(|| format!("Map {map_id}"));
                let description = direct_description
                    .clone()
                    .or_else(|| {
                        super::nx_string_value(
                            strings,
                            &format!("Map.img/{category}/{map_id}/mapDesc"),
                        )
                    })
                    .unwrap_or_default();
                let title_key = title.to_lowercase();
                let description_key = description.to_lowercase();
                entries.entry(map_id).or_insert_with(|| SearchEntry {
                    map_id,
                    title,
                    world_map: world_map.to_owned(),
                    title_key,
                    description_key,
                    map_id_key: map_id.to_string(),
                });
            }
        }
    }
    let mut entries = entries.into_values().collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        left.title
            .to_lowercase()
            .cmp(&right.title.to_lowercase())
            .then_with(|| left.map_id.cmp(&right.map_id))
    });
    Ok(SearchCatalog { entries })
}

#[derive(Clone)]
struct Sprite {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
    origin_x: f32,
    origin_y: f32,
}

pub(super) fn render(
    map_archive: &NxArchive,
    ui_archive: &NxArchive,
    map_id: i32,
    requested_map: Option<&str>,
    search_open: bool,
    search_query: &str,
    search_focused: bool,
    search_catalog: Option<&SearchCatalog>,
    search_scroll: usize,
    hovered_link: Option<usize>,
    hovered_marker: Option<usize>,
    selected_route_map_id: Option<i32>,
    strings: &NxArchive,
    font: &Font,
) -> Result<Panel, String> {
    let ui_root = ui_archive.root();
    let map_root = map_archive.root();
    let world_map_ui = resolve(&ui_root, "UIWindow2.img/WorldMap")?;
    let border = resolve(&world_map_ui, "Border/0")?;
    let border_sprite = sprite(&border)?;
    let border_width = border_sprite.width;
    let border_height = border_sprite.height;

    let search_background = optional_resolve(&world_map_ui, "WorldMapSearch/backgrnd")?
        .and_then(|node| sprite(&node).ok());
    let search_notice = optional_resolve(&world_map_ui, "WorldMapSearch/notice")?
        .and_then(|node| sprite(&node).ok());
    let search_width = if search_open {
        search_background.as_ref().map_or(0, |image| image.width)
    } else {
        0
    };
    let width = border_width.saturating_add(search_width).min(1024);
    let height = border_height
        .max(if search_open {
            search_background.as_ref().map_or(0, |image| image.height)
        } else {
            0
        })
        .min(768);
    if width == 0
        || height == 0
        || width > MAX_BITMAP_SIDE as usize
        || height > MAX_BITMAP_SIDE as usize
    {
        return Err(format!(
            "world map panel has invalid dimensions {width}x{height}"
        ));
    }

    let map_folder = resolve(&map_root, "WorldMap")?;
    let selected_path = requested_map
        .map(str::to_owned)
        .unwrap_or_else(|| format!("WorldMap0{}", map_id / 10_000_000));
    let (map_path, map_data) = resolve_world_map(&map_folder, &selected_path)?;
    let parent_map = string_child(&map_data, "info/parentMap");
    let base_node = optional_resolve(&map_data, "BaseImg/0")?
        .or(optional_resolve(&map_data, "BaseImg")?)
        .ok_or_else(|| format!("world map {map_path:?} has no BaseImg"))?;
    let base_sprite = sprite(&base_node)?;
    let map_helper = resolve(&map_root, "MapHelper.img/worldMap")?;

    let x = ((VIEW_WIDTH - width as f32) / 2.0).max(0.0);
    let y = PANEL_TOP;
    let mut pixels = vec![0; width * height * 4];
    blit(&mut pixels, width, height, &border_sprite, 0.0, 0.0, 1.0);
    if search_open {
        if let Some(image) = search_background.as_ref() {
            blit(
                &mut pixels,
                width,
                height,
                image,
                border_width as f32,
                0.0,
                1.0,
            );
        }
        if let Some(image) = search_notice.as_ref() {
            blit(
                &mut pixels,
                width,
                height,
                image,
                border_width as f32,
                0.0,
                1.0,
            );
        }
    }

    let content_anchor_x = border_width as f32 / 2.0;
    let content_anchor_y = border_height as f32 / 2.0 + 15.0;
    let available_width = border_width.max(1) as f32;
    let available_height = border_height.saturating_sub(30).max(1) as f32;
    let map_scale = (available_width / base_sprite.width as f32)
        .min(available_height / base_sprite.height as f32)
        .min(1.0)
        .max(0.1);
    blit(
        &mut pixels,
        width,
        height,
        &base_sprite,
        content_anchor_x,
        content_anchor_y,
        map_scale,
    );

    let mut links = Vec::new();
    if let Some(map_links) = optional_resolve(&map_data, "MapLink")? {
        for entry in children(&map_links)? {
            let Some(link) = optional_child(&entry, "link")? else {
                continue;
            };
            let (Some(link_map), Some(link_image_node)) = (
                string_child(&link, "linkMap"),
                optional_child(&link, "linkImg")?,
            ) else {
                continue;
            };
            let Ok(image) = sprite(&link_image_node) else {
                continue;
            };
            let left = x + content_anchor_x + (0.0 - image.origin_x) * map_scale;
            let top = y + content_anchor_y + (0.0 - image.origin_y) * map_scale;
            if hovered_link == Some(links.len()) {
                blit(
                    &mut pixels,
                    width,
                    height,
                    &image,
                    content_anchor_x,
                    content_anchor_y,
                    map_scale,
                );
            }
            links.push(LinkHit {
                bounds: HitRect {
                    x: left,
                    y: top,
                    width: image.width as f32 * map_scale,
                    height: image.height as f32 * map_scale,
                },
                target: link_map,
            });
        }
    }

    let normalized_query = search_query.trim().to_lowercase();
    let mut search_result_buckets = [0; 5];
    if !normalized_query.is_empty()
        && let Some(catalog) = search_catalog
    {
        for entry in &catalog.entries {
            if let Some(rank) = entry.match_rank(&normalized_query) {
                search_result_buckets[rank] += 1;
            }
        }
    }
    let search_result_count: usize = search_result_buckets.iter().sum();
    let first_result =
        search_scroll.min(search_result_count.saturating_sub(MAX_VISIBLE_SEARCH_RESULTS));
    let mut search_hits = Vec::new();
    let mut markers = Vec::new();
    let mut marker_visuals = Vec::new();
    if let Some(map_list) = optional_resolve(&map_data, "MapList")? {
        let map_image = optional_child(&map_helper, "mapImage")?;
        for entry in children(&map_list)? {
            let Some((spot_x, spot_y)) = vector_child(&entry, "spot") else {
                continue;
            };
            let map_ids = integer_children(&entry, "mapNo");
            let title = string_child(&entry, "title");
            let description = string_child(&entry, "desc")
                .or_else(|| {
                    map_ids.first().and_then(|id| {
                        let category = super::map_string_category(*id);
                        super::nx_string_value(strings, &format!("Map.img/{category}/{id}/mapDesc"))
                    })
                })
                .unwrap_or_default();
            if !normalized_query.is_empty() {
                let map_id_matches = map_ids
                    .iter()
                    .any(|id| id.to_string().contains(&normalized_query));
                let title_matches = title
                    .as_deref()
                    .is_some_and(|name| name.to_lowercase().contains(&normalized_query));
                let localized_name_matches = map_ids.iter().any(|id| {
                    super::map_string_name(strings, *id)
                        .is_some_and(|name| name.to_lowercase().contains(&normalized_query))
                });
                let description_matches = description.to_lowercase().contains(&normalized_query);
                if !map_id_matches
                    && !title_matches
                    && !localized_name_matches
                    && !description_matches
                {
                    continue;
                }
            }
            let spot_type = integer_child(&entry, "type").unwrap_or(0).clamp(0, 3) as usize;
            let marker = map_image
                .as_ref()
                .and_then(|node| optional_child(node, &spot_type.to_string()).ok().flatten())
                .and_then(|node| sprite(&node).ok());
            let (marker_origin_x, marker_origin_y, marker_width, marker_height) =
                marker.as_ref().map_or((6.0, 6.0, 12.0, 12.0), |image| {
                    (
                        image.origin_x,
                        image.origin_y,
                        image.width as f32,
                        image.height as f32,
                    )
                });
            let map_title = title
                .clone()
                .or_else(|| {
                    map_ids
                        .first()
                        .and_then(|id| super::map_string_name(strings, *id))
                })
                .unwrap_or_else(|| {
                    map_ids
                        .first()
                        .map_or_else(|| "Map".to_owned(), |id| format!("Map {id}"))
                });
            let path_node = optional_child(&entry, "path")?;
            markers.push(MarkerHit {
                bounds: HitRect {
                    x: x + content_anchor_x + (spot_x as f32 - marker_origin_x) * map_scale,
                    y: y + content_anchor_y + (spot_y as f32 - marker_origin_y) * map_scale,
                    width: marker_width * map_scale,
                    height: marker_height * map_scale,
                },
                map_id: map_ids.first().copied().unwrap_or_default(),
                map_ids: map_ids.clone(),
                title: map_title,
                description,
                path: path_node,
            });
            marker_visuals.push((
                marker,
                spot_x,
                spot_y,
                spot_type,
                map_ids.contains(&map_id),
                selected_route_map_id.is_some_and(|target| map_ids.contains(&target)),
            ));
        }
    }

    let selected_marker = selected_route_map_id.and_then(|map_id| {
        markers
            .iter()
            .find(|marker| marker.map_ids.contains(&map_id))
    });
    if let Some(path) = hovered_marker
        .and_then(|index| markers.get(index))
        .and_then(|marker| marker.path.as_ref())
        .or_else(|| selected_marker.and_then(|marker| marker.path.as_ref()))
        .and_then(|node| sprite(node).ok())
    {
        blit(
            &mut pixels,
            width,
            height,
            &path,
            content_anchor_x,
            content_anchor_y,
            map_scale,
        );
    }
    for (marker, spot_x, spot_y, spot_type, is_current_map, is_route_target) in marker_visuals {
        let anchor_x = content_anchor_x + spot_x as f32 * map_scale;
        let anchor_y = content_anchor_y + spot_y as f32 * map_scale;
        if let Some(marker) = marker {
            blit(
                &mut pixels,
                width,
                height,
                &marker,
                anchor_x,
                anchor_y,
                map_scale,
            );
        }
        if is_current_map || is_route_target {
            if let Ok(Some(npc_marker)) =
                optional_resolve(&map_helper, &format!("npcPos{spot_type}"))
            {
                if is_current_map
                    && let Some(frame) =
                        first_frame(&npc_marker).and_then(|node| sprite(&node).ok())
                {
                    blit(
                        &mut pixels,
                        width,
                        height,
                        &frame,
                        anchor_x,
                        anchor_y,
                        map_scale,
                    );
                }
            }
            if let Ok(Some(cursor)) = optional_child(&map_helper, "curPos") {
                if let Some(frame) = first_frame(&cursor).and_then(|node| sprite(&node).ok()) {
                    blit(
                        &mut pixels,
                        width,
                        height,
                        &frame,
                        anchor_x,
                        anchor_y,
                        map_scale,
                    );
                }
            }
        }
    }

    if let Some(marker) = hovered_marker
        .and_then(|index| markers.get(index))
        .or(selected_marker)
    {
        let tooltip_in_search_panel = search_open && normalized_query.is_empty();
        let tooltip_width = if tooltip_in_search_panel {
            search_width.saturating_sub(18)
        } else {
            220
        };
        let tooltip_x = if tooltip_in_search_panel {
            border_width as f32 + 9.0
        } else {
            (marker.bounds.x - x + 10.0)
                .clamp(8.0, border_width.saturating_sub(tooltip_width + 8) as f32)
        };
        let tooltip_y = if tooltip_in_search_panel {
            54.0
        } else {
            (marker.bounds.y - y + 12.0).clamp(36.0, border_height.saturating_sub(70) as f32)
        };
        let tooltip_width = tooltip_width.max(70);
        fill_rect(
            &mut pixels,
            width,
            height,
            tooltip_x as usize,
            tooltip_y as usize,
            tooltip_width,
            62,
            [244, 236, 213, 255],
        );
        let description = marker.description.chars().collect::<Vec<_>>();
        let description_line_one = description.iter().take(18).collect::<String>();
        let description_line_two = description.iter().skip(18).take(18).collect::<String>();
        let selected_map_id =
            selected_route_map_id.filter(|map_id| marker.map_ids.contains(map_id));
        let tooltip_line_one = if selected_map_id.is_some() {
            "Route selected"
        } else {
            description_line_one.as_str()
        };
        let tooltip_line_two = if selected_map_id.is_some() {
            description_line_one.as_str()
        } else {
            description_line_two.as_str()
        };
        draw_text_color(
            &mut pixels,
            width,
            height,
            font,
            &marker.title.chars().take(22).collect::<String>(),
            tooltip_x + 5.0,
            tooltip_y + 3.0,
            [35, 27, 18],
        );
        draw_text_color(
            &mut pixels,
            width,
            height,
            font,
            &format!("Map {}", selected_map_id.unwrap_or(marker.map_id)),
            tooltip_x + 5.0,
            tooltip_y + 19.0,
            [83, 68, 48],
        );
        draw_text_color(
            &mut pixels,
            width,
            height,
            font,
            tooltip_line_one,
            tooltip_x + 5.0,
            tooltip_y + 34.0,
            [83, 68, 48],
        );
        draw_text_color(
            &mut pixels,
            width,
            height,
            font,
            tooltip_line_two,
            tooltip_x + 5.0,
            tooltip_y + 48.0,
            [83, 68, 48],
        );
    }

    let title = optional_child(&world_map_ui, "title")?.and_then(|node| sprite(&node).ok());
    if let Some(title) = title {
        blit(
            &mut pixels,
            width,
            height,
            &title,
            (border_width as f32 - title.width as f32) / 2.0,
            12.0,
            1.0,
        );
    } else {
        draw_text(&mut pixels, width, height, font, "WORLD MAP", 22.0, 12.0);
    }

    let close_node = optional_resolve(&ui_root, "Basic.img/BtClose3")?;
    let close_sprite = close_node
        .as_ref()
        .and_then(|node| first_frame(node))
        .and_then(|node| sprite(&node).ok());
    let close_anchor_x = border_width.saturating_sub(22) as f32;
    let close_anchor_y = 4.0;
    let close = if let Some(image) = close_sprite.as_ref() {
        blit(
            &mut pixels,
            width,
            height,
            image,
            close_anchor_x,
            close_anchor_y,
            1.0,
        );
        HitRect {
            x: x + close_anchor_x - image.origin_x,
            y: y + close_anchor_y - image.origin_y,
            width: image.width as f32,
            height: image.height as f32,
        }
    } else {
        HitRect {
            x: x + border_width as f32 - 30.0,
            y: y + 4.0,
            width: 20.0,
            height: 20.0,
        }
    };
    if close_sprite.is_none() {
        fill_rect(
            &mut pixels,
            width,
            height,
            border_width.saturating_sub(30),
            4,
            20,
            20,
            [44, 44, 44, 255],
        );
        draw_text(
            &mut pixels,
            width,
            height,
            font,
            "X",
            border_width.saturating_sub(26) as f32,
            2.0,
        );
    }

    let search_toggle = draw_button(
        &mut pixels,
        width,
        height,
        &world_map_ui,
        "BtSearch",
        0.0,
        0.0,
        x,
        y,
    )?;
    let mut search_submit = None;
    if search_open {
        let _ = draw_button(
            &mut pixels,
            width,
            height,
            &world_map_ui,
            "BtAutoFly_1",
            0.0,
            0.0,
            x,
            y,
        )?;
        let _ = draw_button(
            &mut pixels,
            width,
            height,
            &world_map_ui,
            "BtNaviRegister",
            0.0,
            0.0,
            x,
            y,
        )?;
        search_submit = draw_button(
            &mut pixels,
            width,
            height,
            &world_map_ui,
            "WorldMapSearch/BtAllsearch",
            border_width as f32,
            0.0,
            x,
            y,
        )?;
    }
    let search_input = search_open.then(|| HitRect {
        x: x + border_width as f32 + 14.0,
        y: y + 19.0,
        width: 83.0,
        height: 19.0,
    });
    if search_input.is_some() {
        let visible_query = if search_query.is_empty() {
            "Search maps"
        } else {
            search_query
        };
        draw_text_color(
            &mut pixels,
            width,
            height,
            font,
            visible_query,
            border_width as f32 + 16.0,
            20.0,
            if search_query.is_empty() {
                [118, 105, 88]
            } else {
                [30, 24, 18]
            },
        );
        if search_focused {
            let caret_x = border_width as f32 + 16.0 + (search_query.chars().count() as f32 * 7.0);
            fill_rect(
                &mut pixels,
                width,
                height,
                caret_x.max(0.0) as usize,
                22,
                1,
                13,
                [35, 26, 18, 255],
            );
        }
    }
    if search_open && !normalized_query.is_empty() {
        let results_label = if search_result_count == 0 {
            "No maps found".to_owned()
        } else {
            format!("{} matches", search_result_count)
        };
        draw_text_color(
            &mut pixels,
            width,
            height,
            font,
            &results_label,
            border_width as f32 + 10.0,
            40.0,
            [77, 62, 45],
        );
        let list_x = border_width + 8;
        let list_width = search_width.saturating_sub(16);
        let mut visible_index = 0;
        let mut remaining_skip = first_result;
        if let Some(catalog) = search_catalog {
            for rank in 0..search_result_buckets.len() {
                let bucket_count = search_result_buckets[rank];
                if remaining_skip >= bucket_count {
                    remaining_skip -= bucket_count;
                    continue;
                }
                let bucket_skip = remaining_skip;
                remaining_skip = 0;
                for result in catalog
                    .entries
                    .iter()
                    .filter(|entry| entry.match_rank(&normalized_query) == Some(rank))
                    .skip(bucket_skip)
                    .take(MAX_VISIBLE_SEARCH_RESULTS.saturating_sub(visible_index))
                {
                    let row_y = SEARCH_RESULT_TOP + visible_index as f32 * SEARCH_RESULT_HEIGHT;
                    fill_rect(
                        &mut pixels,
                        width,
                        height,
                        list_x,
                        row_y as usize,
                        list_width,
                        SEARCH_RESULT_HEIGHT as usize - 2,
                        [242, 233, 212, 255],
                    );
                    draw_text_color(
                        &mut pixels,
                        width,
                        height,
                        font,
                        &result.title.chars().take(17).collect::<String>(),
                        list_x as f32 + 4.0,
                        row_y + 1.0,
                        [42, 31, 20],
                    );
                    draw_text_color(
                        &mut pixels,
                        width,
                        height,
                        font,
                        &format!("Map {}", result.map_id),
                        list_x as f32 + 4.0,
                        row_y + 17.0,
                        [91, 74, 52],
                    );
                    search_hits.push(SearchHit {
                        bounds: HitRect {
                            x: x + list_x as f32,
                            y: y + row_y,
                            width: list_width as f32,
                            height: SEARCH_RESULT_HEIGHT - 2.0,
                        },
                        map_id: result.map_id,
                        world_map: result.world_map.clone(),
                    });
                    visible_index += 1;
                }
                if visible_index >= MAX_VISIBLE_SEARCH_RESULTS {
                    break;
                }
            }
        }
    }
    let search_close = if search_open {
        close_sprite.as_ref().map(|image| {
            let anchor_x = search_width as f32 + close_anchor_x;
            blit(
                &mut pixels,
                width,
                height,
                image,
                anchor_x,
                close_anchor_y,
                1.0,
            );
            HitRect {
                x: x + anchor_x - image.origin_x,
                y: y + close_anchor_y - image.origin_y,
                width: image.width as f32,
                height: image.height as f32,
            }
        })
    } else {
        None
    };

    Ok(Panel {
        width: width as u32,
        height: height as u32,
        x,
        y,
        pixels,
        close,
        search_toggle,
        search_close,
        search_input,
        search_submit,
        search_hits,
        search_result_count,
        links,
        markers,
        map_path,
        parent_map,
    })
}

fn draw_button(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    button_root: &NxNode,
    path: &str,
    anchor_x: f32,
    anchor_y: f32,
    screen_x: f32,
    screen_y: f32,
) -> Result<Option<HitRect>, String> {
    let Some(button) = optional_resolve(button_root, path)? else {
        return Ok(None);
    };
    let Some(normal) =
        optional_resolve(&button, "normal/0")?.or(optional_resolve(&button, "normal")?)
    else {
        return Ok(None);
    };
    let image = sprite(&normal)?;
    blit(pixels, width, height, &image, anchor_x, anchor_y, 1.0);
    Ok(Some(HitRect {
        x: screen_x + anchor_x - image.origin_x,
        y: screen_y + anchor_y - image.origin_y,
        width: image.width as f32,
        height: image.height as f32,
    }))
}

fn resolve_world_map(folder: &NxNode, desired: &str) -> Result<(String, NxNode), String> {
    let path = format!("{desired}.img");
    if let Some(map) = optional_child(folder, &path)? {
        return Ok((desired.to_owned(), map));
    }
    let fallback = optional_child(folder, "WorldMap.img")?
        .ok_or_else(|| format!("WorldMap/{path} and WorldMap/WorldMap.img are missing"))?;
    Ok(("WorldMap".to_owned(), fallback))
}

fn resolve(node: &NxNode, path: &str) -> Result<NxNode, String> {
    node.resolve(path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("NX node {path:?} was not found"))
}

fn optional_resolve(node: &NxNode, path: &str) -> Result<Option<NxNode>, String> {
    node.resolve(path).map_err(|error| error.to_string())
}

fn optional_child(node: &NxNode, path: &str) -> Result<Option<NxNode>, String> {
    node.resolve(path).map_err(|error| error.to_string())
}

fn children(node: &NxNode) -> Result<Vec<NxNode>, String> {
    node.children().map_err(|error| error.to_string())
}

fn string_child(node: &NxNode, path: &str) -> Option<String> {
    let node = optional_child(node, path).ok()??;
    match node.value().ok()? {
        NodeValue::String(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

fn integer_child(node: &NxNode, path: &str) -> Option<i32> {
    let node = optional_child(node, path).ok()??;
    match node.value().ok()? {
        NodeValue::Integer(value) => i32::try_from(value).ok(),
        _ => None,
    }
}

fn integer_children(node: &NxNode, path: &str) -> Vec<i32> {
    let Ok(Some(list)) = optional_child(node, path) else {
        return Vec::new();
    };
    if let Ok(NodeValue::Integer(value)) = list.value()
        && let Ok(value) = i32::try_from(value)
    {
        return vec![value];
    }
    children(&list)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| match item.value().ok()? {
            NodeValue::Integer(value) => i32::try_from(value).ok(),
            _ => None,
        })
        .collect()
}

fn vector_child(node: &NxNode, path: &str) -> Option<(i32, i32)> {
    let node = optional_child(node, path).ok()??;
    match node.value().ok()? {
        NodeValue::Vector { x, y } => Some((x, y)),
        _ => None,
    }
}

fn first_frame(node: &NxNode) -> Option<NxNode> {
    if node.bitmap().ok()?.is_some() {
        return Some(node.clone());
    }
    let mut frames = children(node).ok()?;
    frames.sort_by_key(|frame| frame.name().ok().and_then(|name| name.parse::<u32>().ok()));
    frames
        .into_iter()
        .find(|frame| frame.bitmap().ok().flatten().is_some())
}

fn sprite(node: &NxNode) -> Result<Sprite, String> {
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!(
                "NX node {:?} is not a bitmap",
                node.name().unwrap_or_default()
            )
        })?;
    let (origin_x, origin_y) = optional_child(node, "origin")?
        .and_then(|origin| match origin.value().ok()? {
            NodeValue::Vector { x, y } => Some((x as f32, y as f32)),
            _ => None,
        })
        .unwrap_or((0.0, 0.0));
    Ok(Sprite {
        pixels: bitmap
            .decode_bgra_shared()
            .map_err(|error| error.to_string())?
            .to_vec(),
        width: usize::from(bitmap.width()),
        height: usize::from(bitmap.height()),
        origin_x,
        origin_y,
    })
}

fn blit(
    canvas: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    image: &Sprite,
    anchor_x: f32,
    anchor_y: f32,
    scale: f32,
) {
    if image.width == 0 || image.height == 0 || scale <= 0.0 {
        return;
    }
    let draw_width = (image.width as f32 * scale).round().max(1.0) as usize;
    let draw_height = (image.height as f32 * scale).round().max(1.0) as usize;
    let left = (anchor_x - image.origin_x * scale).round() as i32;
    let top = (anchor_y - image.origin_y * scale).round() as i32;
    for y in 0..draw_height {
        let target_y = top + y as i32;
        if !(0..canvas_height as i32).contains(&target_y) {
            continue;
        }
        let source_y = (y * image.height / draw_height).min(image.height - 1);
        for x in 0..draw_width {
            let target_x = left + x as i32;
            if !(0..canvas_width as i32).contains(&target_x) {
                continue;
            }
            let source_x = (x * image.width / draw_width).min(image.width - 1);
            let source_index = (source_y * image.width + source_x) * 4;
            let target_index = (target_y as usize * canvas_width + target_x as usize) * 4;
            let alpha = u16::from(image.pixels[source_index + 3]);
            if alpha == 0 {
                continue;
            }
            let inverse = 255 - alpha;
            for channel in 0..3 {
                canvas[target_index + channel] = ((u16::from(image.pixels[source_index + channel])
                    * alpha
                    + u16::from(canvas[target_index + channel]) * inverse)
                    / 255) as u8;
            }
            canvas[target_index + 3] =
                (alpha + u16::from(canvas[target_index + 3]) * inverse / 255).min(255) as u8;
        }
    }
}

fn fill_rect(
    canvas: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: [u8; 4],
) {
    for target_y in y..y.saturating_add(height).min(canvas_height) {
        for target_x in x..x.saturating_add(width).min(canvas_width) {
            let index = (target_y * canvas_width + target_x) * 4;
            canvas[index..index + 4].copy_from_slice(&color);
        }
    }
}

fn draw_text(
    canvas: &mut [u8],
    width: usize,
    height: usize,
    font: &Font,
    text: &str,
    x: f32,
    y: f32,
) {
    draw_text_color(canvas, width, height, font, text, x, y, [230, 230, 220]);
}

fn draw_text_color(
    canvas: &mut [u8],
    width: usize,
    height: usize,
    font: &Font,
    text: &str,
    x: f32,
    y: f32,
    color: [u8; 3],
) {
    let mut cursor_x = x;
    for character in text.chars() {
        let (metrics, coverage) = font.rasterize(character, 14.0);
        let left = cursor_x.round() as i32 + metrics.xmin;
        let top = (y + 14.0).round() as i32 - metrics.height as i32 - metrics.ymin;
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                let target_x = left + column as i32;
                let target_y = top + row as i32;
                if !(0..width as i32).contains(&target_x) || !(0..height as i32).contains(&target_y)
                {
                    continue;
                }
                let alpha = coverage[row * metrics.width + column];
                let index = (target_y as usize * width + target_x as usize) * 4;
                let inverse = 255 - u16::from(alpha);
                for channel in 0..3 {
                    canvas[index + channel] = ((u16::from(color[channel]) * u16::from(alpha)
                        + u16::from(canvas[index + channel]) * inverse)
                        / 255) as u8;
                }
                canvas[index + 3] = (u16::from(alpha)
                    + u16::from(canvas[index + 3]) * inverse / 255)
                    .min(255) as u8;
            }
        }
        cursor_x += metrics.advance_width;
    }
}
