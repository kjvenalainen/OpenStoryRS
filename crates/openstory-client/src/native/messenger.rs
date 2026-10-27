//! Asset-backed Messenger window for the v83 UI.
//!
//! The caller owns protocol dispatch and pointer/keyboard focus. This module
//! keeps bounded view state and returns a BGRA panel plus global, 1024x768
//! logical action regions. The legacy status-bar command opens the window;
//! `Action::Open` is also emitted for the collapsed title strip.

use std::collections::VecDeque;

use fontdue::Font;
use openstory_assets::{NodeValue, NxArchive, NxNode};

pub const LOGICAL_WIDTH: f32 = 1024.0;
pub const LOGICAL_HEIGHT: f32 = 768.0;
pub const DEFAULT_X: f32 = 200.0;
pub const DEFAULT_Y: f32 = 20.0;
/// Four indices are retained for bounded protocol state; v83 displays slots 0..2.
pub const MEMBER_SLOTS: usize = 4;
pub const VISIBLE_MEMBER_SLOTS: usize = 3;
pub const MAX_NAME_BYTES: usize = 15;
pub const MAX_MESSAGE_BYTES: usize = 70;
pub const MAX_CHAT_LINES: usize = 50;
pub const VISIBLE_CHAT_LINES: usize = 8;

const FONT_SIZE: f32 = 14.0;
const CHAT_LINE_HEIGHT: f32 = 14.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitRect {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Reopen the window from its collapsed title strip (or status-bar icon).
    Open,
    /// Leave the current messenger group while keeping the client running.
    Leave,
    /// Start the invite-name prompt, or submit it when that prompt is active.
    Invite,
    Send,
    Accept,
    Decline,
    /// Focus the chat draft for text input.
    FocusChat,
    /// Focus the outgoing invite-name field for text input.
    FocusInviteName,
    /// Close the panel. Legacy close also leaves, so the caller should send a
    /// leave request when handling this action if preserving v83 behavior.
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionHit {
    pub bounds: HitRect,
    pub action: Action,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatLine {
    pub name: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InvitePrompt {
    EnterName { draft: String },
    Incoming { inviter: String },
}

#[derive(Clone, Debug)]
pub struct State {
    pub x: f32,
    pub y: f32,
    is_open: bool,
    members: [Option<String>; MEMBER_SLOTS],
    chat_history: VecDeque<ChatLine>,
    draft: String,
    invite_prompt: Option<InvitePrompt>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            x: DEFAULT_X,
            y: DEFAULT_Y,
            is_open: true,
            members: std::array::from_fn(|_| None),
            chat_history: VecDeque::with_capacity(MAX_CHAT_LINES),
            draft: String::new(),
            invite_prompt: None,
        }
    }
}

impl State {
    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn draft(&self) -> &str {
        &self.draft
    }

    pub fn invite_prompt(&self) -> Option<&InvitePrompt> {
        self.invite_prompt.as_ref()
    }

    /// Keeps the floating window within the game's fixed logical viewport.
    pub fn clamp_to_viewport(&mut self, panel_width: u32, panel_height: u32) {
        self.x = self
            .x
            .clamp(0.0, (LOGICAL_WIDTH - panel_width as f32).max(0.0));
        self.y = self
            .y
            .clamp(0.0, (LOGICAL_HEIGHT - panel_height as f32).max(0.0));
    }

    pub fn set_member(&mut self, slot: usize, name: impl AsRef<str>) {
        let Some(member) = self.members.get_mut(slot) else {
            return;
        };
        let name = bounded_text(name.as_ref(), MAX_NAME_BYTES);
        *member = (!name.is_empty()).then_some(name);
    }

    pub fn remove_member(&mut self, slot: usize) {
        if let Some(member) = self.members.get_mut(slot) {
            *member = None;
        }
    }

    pub fn add_chat(&mut self, name: impl AsRef<str>, message: impl AsRef<str>) {
        let line = ChatLine {
            name: bounded_text(name.as_ref(), MAX_NAME_BYTES),
            message: bounded_text(message.as_ref(), MAX_MESSAGE_BYTES),
        };
        self.chat_history.push_back(line);
        while self.chat_history.len() > MAX_CHAT_LINES {
            self.chat_history.pop_front();
        }
    }

    pub fn set_draft(&mut self, draft: impl AsRef<str>) {
        self.draft = bounded_text(draft.as_ref(), MAX_MESSAGE_BYTES);
    }

    pub fn push_draft_char(&mut self, character: char) {
        if character.is_control() || self.draft.len() + character.len_utf8() > MAX_MESSAGE_BYTES {
            return;
        }
        self.draft.push(character);
    }

    pub fn backspace_draft(&mut self) {
        self.draft.pop();
    }

    pub fn begin_invite(&mut self) {
        self.open();
        self.invite_prompt = Some(InvitePrompt::EnterName {
            draft: String::new(),
        });
    }

    pub fn prompt_incoming_invite(&mut self, inviter: impl AsRef<str>) {
        self.open();
        self.invite_prompt = Some(InvitePrompt::Incoming {
            inviter: bounded_text(inviter.as_ref(), MAX_NAME_BYTES),
        });
    }

    pub fn push_invite_char(&mut self, character: char) {
        let Some(InvitePrompt::EnterName { draft }) = &mut self.invite_prompt else {
            return;
        };
        if !character.is_control() && draft.len() + character.len_utf8() <= MAX_NAME_BYTES {
            draft.push(character);
        }
    }

    pub fn backspace_invite(&mut self) {
        if let Some(InvitePrompt::EnterName { draft }) = &mut self.invite_prompt {
            draft.pop();
        }
    }

    pub fn dismiss_invite_prompt(&mut self) {
        self.invite_prompt = None;
    }
}

pub struct Panel {
    pub width: u32,
    pub height: u32,
    /// BGRA composited pixels for `GpuRenderer::upload_bgra`.
    pub pixels: Vec<u8>,
    /// Hit regions use global 1024x768 logical coordinates, matching `State::x/y`.
    pub actions: Vec<ActionHit>,
}

#[derive(Clone)]
struct Sprite {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
    origin_x: f32,
    origin_y: f32,
}

pub fn render(ui_archive: &NxArchive, font: &Font, state: &State) -> Result<Panel, String> {
    let root = ui_archive.root();
    let messenger = resolve(&root, "UIWindow.img/Messenger")?;
    let background_node = resolve(&messenger, "backgrnd")?;
    let background = sprite(&background_node, "Messenger/backgrnd")?;
    let width = background.width;
    let height = background.height;
    if width == 0 || height == 0 || width > 2048 || height > 2048 {
        return Err(format!(
            "Messenger background has invalid dimensions {width}x{height}"
        ));
    }

    let mut pixels = vec![0; width * height * 4];
    let mut actions = Vec::with_capacity(8);
    if !state.is_open {
        // Keep a small, asset-backed title strip when collapsed so it has a
        // real, logical-area target from which the caller can reopen it.
        blit_region(
            &mut pixels,
            width,
            height,
            &background,
            0.0,
            0.0,
            height.min(20),
        );
        let open_bounds =
            clipped_region_bounds(&background, 0.0, 0.0, width, height, height.min(20));
        actions.push(ActionHit {
            bounds: HitRect {
                x: state.x + open_bounds.0,
                y: state.y + open_bounds.1,
                width: open_bounds.2,
                height: open_bounds.3,
            },
            action: Action::Open,
        });
        return Ok(Panel {
            width: width as u32,
            height: height as u32,
            pixels,
            actions,
        });
    }

    blit(&mut pixels, width, height, &background, 0.0, 0.0);
    if let Some(background2) = optional_sprite(&messenger, "backgrnd2", "Messenger/backgrnd2")? {
        blit(&mut pixels, width, height, &background2, 0.0, 0.0);
    }
    if let Some(background3) = optional_sprite(&messenger, "backgrnd3", "Messenger/backgrnd3")? {
        blit(&mut pixels, width, height, &background3, 14.0, 330.0);
    }

    let close = button_sprite(&root, "Basic.img/BtClose3", "Basic.img/BtClose3")?;
    let close_x = width.saturating_sub(19) as f32;
    blit(&mut pixels, width, height, &close, close_x, 6.0);
    actions.push(ActionHit {
        bounds: hit_rect(state, close_x, 6.0, &close),
        action: Action::Close,
    });

    let name_bars = resolve(&messenger, "NameBar")?;
    let empty_bar = sprite(&resolve(&name_bars, "0")?, "Messenger/NameBar/0")?;
    let occupied_bar = sprite(&resolve(&name_bars, "1")?, "Messenger/NameBar/1")?;
    // The v83 server assigns three room slots (`MAX_PLAYERS` in UIMessenger.cpp).
    // The fourth bounded state slot is retained for protocol-side compatibility.
    for (slot, member) in state.members.iter().take(VISIBLE_MEMBER_SLOTS).enumerate() {
        let slot_y = 30.0 + slot as f32 * 28.0;
        if let Some(name) = member {
            blit(&mut pixels, width, height, &occupied_bar, 10.0, slot_y);
            draw_text(
                &mut pixels,
                width,
                height,
                font,
                name,
                20.0,
                slot_y + 4.0,
                [255, 255, 255],
            );
        } else {
            blit(&mut pixels, width, height, &empty_bar, 10.0, slot_y);
        }
    }

    let chat_start = state.chat_history.len().saturating_sub(VISIBLE_CHAT_LINES);
    for (row, line) in state.chat_history.iter().skip(chat_start).enumerate() {
        let y = 120.0 + row as f32 * CHAT_LINE_HEIGHT;
        let display = if line.name.is_empty() {
            line.message.clone()
        } else {
            format!("{}: {}", line.name, line.message)
        };
        let display = truncate_to_width(font, &display, width.saturating_sub(18) as f32);
        draw_text(
            &mut pixels,
            width,
            height,
            font,
            &display,
            10.0,
            y,
            [24, 24, 24],
        );
    }

    let shown_draft = truncate_to_width(font, &state.draft, 174.0);
    draw_text(
        &mut pixels,
        width,
        height,
        font,
        &shown_draft,
        20.0,
        333.0,
        [24, 24, 24],
    );
    actions.push(ActionHit {
        bounds: global_rect(state, 20.0, 333.0, 174.0, 18.0),
        action: Action::FocusChat,
    });

    let enter = button_sprite(&messenger, "BtEnter", "Messenger/BtEnter")?;
    blit(&mut pixels, width, height, &enter, 205.0, 333.0);
    actions.push(ActionHit {
        bounds: hit_rect(state, 205.0, 333.0, &enter),
        action: if state.draft.is_empty() {
            Action::Invite
        } else {
            Action::Send
        },
    });

    // A separate Leave control makes the server-side leave operation explicit;
    // the original close button also leaves for compatibility with the C++ UI.
    let leave_bounds = HitRect {
        x: state.x + 12.0,
        y: state.y + 304.0,
        width: 62.0,
        height: 20.0,
    };
    draw_button(&mut pixels, width, height, font, 12, 304, 62, 20, "Leave");
    actions.push(ActionHit {
        bounds: leave_bounds,
        action: Action::Leave,
    });

    if let Some(prompt) = &state.invite_prompt {
        // A modal consumes input; retain only the window close target beneath it.
        actions.retain(|hit| hit.action == Action::Close);
        draw_modal(&mut pixels, width, height, font, prompt);
        let center_x = width as f32 / 2.0;
        let input_y = height as f32 / 2.0 - 13.0;
        let button_y = height as f32 / 2.0 + 20.0;
        if matches!(prompt, InvitePrompt::EnterName { .. }) {
            actions.push(ActionHit {
                bounds: global_rect(state, center_x - 85.0, input_y, 170.0, 22.0),
                action: Action::FocusInviteName,
            });
        }
        let accept = HitRect {
            x: state.x + center_x - 74.0,
            y: state.y + button_y,
            width: 66.0,
            height: 20.0,
        };
        let decline = HitRect {
            x: state.x + center_x + 8.0,
            y: state.y + button_y,
            width: 66.0,
            height: 20.0,
        };
        draw_button(
            &mut pixels,
            width,
            height,
            font,
            (center_x - 74.0) as i32,
            button_y as i32,
            66,
            20,
            match prompt {
                InvitePrompt::EnterName { .. } => "Invite",
                InvitePrompt::Incoming { .. } => "Accept",
            },
        );
        draw_button(
            &mut pixels,
            width,
            height,
            font,
            (center_x + 8.0) as i32,
            button_y as i32,
            66,
            20,
            "Decline",
        );
        actions.push(ActionHit {
            bounds: accept,
            action: match prompt {
                InvitePrompt::EnterName { .. } => Action::Invite,
                InvitePrompt::Incoming { .. } => Action::Accept,
            },
        });
        actions.push(ActionHit {
            bounds: decline,
            action: Action::Decline,
        });
    }

    Ok(Panel {
        width: width as u32,
        height: height as u32,
        pixels,
        actions,
    })
}

fn bounded_text(value: &str, maximum_bytes: usize) -> String {
    let mut bounded = String::with_capacity(value.len().min(maximum_bytes));
    for character in value.chars().filter(|character| !character.is_control()) {
        if bounded.len() + character.len_utf8() > maximum_bytes {
            break;
        }
        bounded.push(character);
    }
    bounded
}

fn clipped_region_bounds(
    image: &Sprite,
    anchor_x: f32,
    anchor_y: f32,
    canvas_width: usize,
    canvas_height: usize,
    source_height: usize,
) -> (f32, f32, f32, f32) {
    let drawn_left = (anchor_x - image.origin_x).round();
    let drawn_top = (anchor_y - image.origin_y).round();
    let left = drawn_left.max(0.0);
    let top = drawn_top.max(0.0);
    let right = (drawn_left + image.width as f32).min(canvas_width as f32);
    let bottom = (drawn_top + source_height.min(image.height) as f32).min(canvas_height as f32);
    (left, top, (right - left).max(0.0), (bottom - top).max(0.0))
}

fn global_rect(state: &State, x: f32, y: f32, width: f32, height: f32) -> HitRect {
    HitRect {
        x: state.x + x,
        y: state.y + y,
        width,
        height,
    }
}

fn hit_rect(state: &State, anchor_x: f32, anchor_y: f32, sprite: &Sprite) -> HitRect {
    HitRect {
        x: state.x + (anchor_x - sprite.origin_x).round(),
        y: state.y + (anchor_y - sprite.origin_y).round(),
        width: sprite.width as f32,
        height: sprite.height as f32,
    }
}

fn resolve(node: &NxNode, path: &str) -> Result<NxNode, String> {
    node.resolve(path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("NX node {path:?} was not found"))
}

fn optional_resolve(node: &NxNode, path: &str) -> Result<Option<NxNode>, String> {
    node.resolve(path).map_err(|error| error.to_string())
}

fn optional_sprite(node: &NxNode, path: &str, label: &str) -> Result<Option<Sprite>, String> {
    optional_resolve(node, path)?
        .map(|sprite_node| sprite(&sprite_node, label))
        .transpose()
}

fn button_sprite(node: &NxNode, path: &str, label: &str) -> Result<Sprite, String> {
    let button = resolve(node, path)?;
    for frame in ["normal/0", "0", "normal"] {
        if let Some(frame) = optional_resolve(&button, frame)?
            && frame.bitmap().map_err(|error| error.to_string())?.is_some()
        {
            return sprite(&frame, label);
        }
    }
    sprite(&button, label)
}

fn sprite(node: &NxNode, label: &str) -> Result<Sprite, String> {
    let bitmap = node
        .bitmap()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("NX node {label:?} is not a bitmap"))?;
    let width = usize::from(bitmap.width());
    let height = usize::from(bitmap.height());
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err(format!(
            "NX sprite {label:?} has invalid dimensions {width}x{height}"
        ));
    }
    let (origin_x, origin_y) = match node
        .child("origin")
        .map_err(|error| error.to_string())?
        .map(|origin| origin.value())
        .transpose()
        .map_err(|error| error.to_string())?
    {
        Some(NodeValue::Vector { x, y }) => (x as f32, y as f32),
        Some(_) => return Err(format!("NX sprite {label:?} has a malformed origin")),
        None => (0.0, 0.0),
    };
    let pixels = bitmap
        .decode_bgra_shared()
        .map_err(|error| error.to_string())?
        .to_vec();
    Ok(Sprite {
        pixels,
        width,
        height,
        origin_x,
        origin_y,
    })
}

fn blit(canvas: &mut [u8], width: usize, height: usize, image: &Sprite, x: f32, y: f32) {
    blit_region(canvas, width, height, image, x, y, image.height);
}

fn blit_region(
    canvas: &mut [u8],
    width: usize,
    height: usize,
    image: &Sprite,
    x: f32,
    y: f32,
    source_height: usize,
) {
    let left = (x - image.origin_x).round() as i32;
    let top = (y - image.origin_y).round() as i32;
    for source_y in 0..source_height.min(image.height) {
        let target_y = top + source_y as i32;
        if !(0..height as i32).contains(&target_y) {
            continue;
        }
        for source_x in 0..image.width {
            let target_x = left + source_x as i32;
            if !(0..width as i32).contains(&target_x) {
                continue;
            }
            let source = (source_y * image.width + source_x) * 4;
            let target = (target_y as usize * width + target_x as usize) * 4;
            blend_bgra(
                &mut canvas[target..target + 4],
                &image.pixels[source..source + 4],
            );
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
    baseline_y: f32,
    color: [u8; 3],
) {
    let mut cursor_x = x;
    for character in text.chars() {
        let (metrics, coverage) = font.rasterize(character, FONT_SIZE);
        let left = cursor_x.round() as i32 + metrics.xmin;
        let top = (baseline_y + FONT_SIZE).round() as i32 - metrics.height as i32 - metrics.ymin;
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                let target_x = left + column as i32;
                let target_y = top + row as i32;
                if !(0..width as i32).contains(&target_x) || !(0..height as i32).contains(&target_y)
                {
                    continue;
                }
                let alpha = coverage[row * metrics.width + column];
                if alpha == 0 {
                    continue;
                }
                let index = (target_y as usize * width + target_x as usize) * 4;
                blend_bgra(
                    &mut canvas[index..index + 4],
                    &[color[2], color[1], color[0], alpha],
                );
            }
        }
        cursor_x += metrics.advance_width;
    }
}

fn draw_button(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    font: &Font,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    label: &str,
) {
    fill_rect(
        pixels,
        canvas_width,
        canvas_height,
        x,
        y,
        width,
        height,
        [92, 65, 38, 245],
    );
    stroke_rect(
        pixels,
        canvas_width,
        canvas_height,
        x,
        y,
        width,
        height,
        [207, 167, 104, 255],
    );
    let label_width = measure_text(font, label);
    draw_text(
        pixels,
        canvas_width,
        canvas_height,
        font,
        label,
        x as f32 + ((width as f32 - label_width) / 2.0).max(2.0),
        y as f32 + (height as f32 - FONT_SIZE) / 2.0,
        [255, 245, 220],
    );
}

fn draw_modal(pixels: &mut [u8], width: usize, height: usize, font: &Font, prompt: &InvitePrompt) {
    fill_rect(
        pixels,
        width,
        height,
        0,
        0,
        width as i32,
        height as i32,
        [0, 0, 0, 132],
    );
    let modal_width = 190_i32.min(width as i32 - 12).max(1);
    let modal_height = 98_i32.min(height as i32 - 12).max(1);
    let x = (width as i32 - modal_width) / 2;
    let y = (height as i32 - modal_height) / 2;
    fill_rect(
        pixels,
        width,
        height,
        x,
        y,
        modal_width,
        modal_height,
        [38, 31, 24, 250],
    );
    stroke_rect(
        pixels,
        width,
        height,
        x,
        y,
        modal_width,
        modal_height,
        [209, 170, 111, 255],
    );
    let message = match prompt {
        InvitePrompt::EnterName { .. } => "Invite which player?",
        InvitePrompt::Incoming { .. } => "Messenger invitation",
    };
    draw_centered_text(
        pixels,
        width,
        height,
        font,
        message,
        width as f32 / 2.0,
        y as f32 + 10.0,
        [255, 239, 204],
    );
    let body = match prompt {
        InvitePrompt::EnterName { draft } => draft.as_str(),
        InvitePrompt::Incoming { inviter } => inviter.as_str(),
    };
    let input_y = y + 36;
    fill_rect(
        pixels,
        width,
        height,
        x + 10,
        input_y,
        modal_width - 20,
        22,
        [241, 231, 210, 255],
    );
    draw_text(
        pixels,
        width,
        height,
        font,
        body,
        (x + 15) as f32,
        (input_y + 3) as f32,
        [30, 25, 20],
    );
}

fn draw_centered_text(
    canvas: &mut [u8],
    width: usize,
    height: usize,
    font: &Font,
    text: &str,
    center_x: f32,
    baseline_y: f32,
    color: [u8; 3],
) {
    draw_text(
        canvas,
        width,
        height,
        font,
        text,
        center_x - measure_text(font, text) / 2.0,
        baseline_y,
        color,
    );
}

fn measure_text(font: &Font, text: &str) -> f32 {
    text.chars()
        .map(|character| font.metrics(character, FONT_SIZE).advance_width)
        .sum()
}

fn truncate_to_width(font: &Font, text: &str, maximum_width: f32) -> String {
    if measure_text(font, text) <= maximum_width {
        return text.to_owned();
    }
    let mut result = String::new();
    for character in text.chars() {
        let candidate = format!("{result}{character}…");
        if measure_text(font, &candidate) > maximum_width {
            break;
        }
        result.push(character);
    }
    result.push('…');
    result
}

fn fill_rect(
    canvas: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: [u8; 4],
) {
    for py in y.max(0)..(y + height).min(canvas_height as i32) {
        for px in x.max(0)..(x + width).min(canvas_width as i32) {
            let index = (py as usize * canvas_width + px as usize) * 4;
            blend_bgra(
                &mut canvas[index..index + 4],
                &[color[2], color[1], color[0], color[3]],
            );
        }
    }
}

fn stroke_rect(
    canvas: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: [u8; 4],
) {
    fill_rect(canvas, canvas_width, canvas_height, x, y, width, 1, color);
    fill_rect(
        canvas,
        canvas_width,
        canvas_height,
        x,
        y + height - 1,
        width,
        1,
        color,
    );
    fill_rect(canvas, canvas_width, canvas_height, x, y, 1, height, color);
    fill_rect(
        canvas,
        canvas_width,
        canvas_height,
        x + width - 1,
        y,
        1,
        height,
        color,
    );
}

fn blend_bgra(destination: &mut [u8], source: &[u8]) {
    let alpha = u16::from(source[3]);
    if alpha == 0 {
        return;
    }
    let inverse = 255 - alpha;
    for channel in 0..3 {
        destination[channel] = ((u16::from(source[channel]) * alpha
            + u16::from(destination[channel]) * inverse)
            / 255) as u8;
    }
    destination[3] = (alpha + u16::from(destination[3]) * inverse / 255).min(255) as u8;
}
