fn rps_game_button_layout(button: RpsGameButton) -> (f32, f32, f32, f32, &'static str) {
    match button {
        RpsGameButton::Start => (105.5, 252.0, 99.0, 32.0, "BtStart"),
        RpsGameButton::Rock => (42.0, 250.0, 33.0, 33.0, "BtRock"),
        RpsGameButton::Paper => (138.5, 250.0, 33.0, 33.0, "BtPaper"),
        RpsGameButton::Scissor => (235.0, 250.0, 33.0, 33.0, "BtScissor"),
        RpsGameButton::Continue => (5.0, 270.0, 99.0, 31.0, "BtContinue"),
        RpsGameButton::Retry => (105.5, 270.0, 99.0, 31.0, "BtRetry"),
        RpsGameButton::Exit => (238.0, 8.0, 59.0, 17.0, "BtExit"),
    }
}

fn rps_game_active_buttons(game: &RpsGameRuntime) -> Vec<RpsGameButton> {
    match game.phase {
        RpsGamePhase::Waiting => vec![RpsGameButton::Start, RpsGameButton::Exit],
        RpsGamePhase::Selecting => vec![
            RpsGameButton::Rock,
            RpsGameButton::Paper,
            RpsGameButton::Scissor,
        ],
        RpsGamePhase::Result => {
            let mut buttons = Vec::with_capacity(3);
            if game.result == Some(1) {
                buttons.push(RpsGameButton::Continue);
            }
            buttons.extend([RpsGameButton::Retry, RpsGameButton::Exit]);
            buttons
        }
    }
}

fn rps_game_button_at(game: &RpsGameRuntime, point: Point) -> Option<RpsGameButton> {
    let x = point.x - RPS_GAME_PANEL_X;
    let y = point.y - RPS_GAME_PANEL_Y;
    rps_game_active_buttons(game).into_iter().find(|button| {
        let (left, top, width, height, _) = rps_game_button_layout(*button);
        (left..left + width).contains(&x) && (top..top + height).contains(&y)
    })
}

fn composite_rps_game_buttons(
    archive: &NxArchive,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    game: &RpsGameRuntime,
) -> Result<(), String> {
    let offset_x = RPS_GAME_PANEL_X.round() as i32;
    let offset_y = RPS_GAME_PANEL_Y.round() as i32;
    for button in rps_game_active_buttons(game) {
        let (x, y, _, _, name) = rps_game_button_layout(button);
        let state = if game.hovered == Some(button) {
            "mouseOver"
        } else {
            "normal"
        };
        let path = format!("UIWindow.img/RpsGame/{name}/{state}/0");
        let path = if archive
            .root()
            .resolve(&path)
            .map_err(|error| error.to_string())?
            .is_some()
        {
            path
        } else {
            format!("UIWindow.img/RpsGame/{name}/normal/0")
        };
        composite_ui_bitmap(
            pixels,
            width,
            height,
            archive,
            &path,
            offset_x + x.round() as i32,
            offset_y + y.round() as i32,
        )?;
    }
    Ok(())
}

fn render_rps_game_panel(
    archive: &NxArchive,
    font: &fontdue::Font,
    game: &RpsGameRuntime,
) -> Result<Vec<u8>, String> {
    let width = 1024;
    let height = 768;
    let panel_width = RPS_GAME_PANEL_WIDTH as usize;
    let panel_x = RPS_GAME_PANEL_X.round() as i32;
    let panel_y = RPS_GAME_PANEL_Y.round() as i32;
    let mut pixels = vec![0_u8; width * height * 4];
    composite_ui_bitmap(
        &mut pixels,
        width,
        height,
        archive,
        "UIWindow.img/RpsGame/backgrnd",
        panel_x,
        panel_y,
    )?;
    composite_rps_game_buttons(archive, &mut pixels, width, height, game)?;

    if game.phase == RpsGamePhase::Result {
        if let Some(player) = game.player_choice.filter(|choice| (0..=2).contains(choice)) {
            let name = ["rock", "paper", "scissor"][player as usize];
            composite_ui_bitmap(
                &mut pixels,
                width,
                height,
                archive,
                &format!("UIWindow.img/RpsGame/{name}"),
                panel_x + 50,
                panel_y + 80,
            )?;
        }
        if let Some(opponent) = game.opponent_choice.filter(|choice| (0..=2).contains(choice)) {
            let name = ["Frock", "Fpaper", "Fscissor"][opponent as usize];
            composite_ui_bitmap(
                &mut pixels,
                width,
                height,
                archive,
                &format!("UIWindow.img/RpsGame/{name}"),
                panel_x + panel_width as i32 - 110,
                panel_y + 80,
            )?;
        }

        let (overlay, character, message, color) = match game.result {
            Some(0) => (
                Some("lose"),
                Some("charLose"),
                "You lose!",
                [255, 255, 0],
            ),
            Some(1) => (
                Some("win"),
                Some("charWin"),
                "You win!",
                [255, 255, 0],
            ),
            Some(2) => (Some("draw"), None, "Draw!", [255, 255, 0]),
            _ => (None, None, "", [255, 255, 0]),
        };
        if let Some(overlay) = overlay {
            composite_ui_bitmap(
                &mut pixels,
                width,
                height,
                archive,
                &format!("UIWindow.img/RpsGame/{overlay}"),
                panel_x + panel_width as i32 / 2,
                panel_y + 50,
            )?;
        }
        if let Some(character) = character {
            composite_ui_bitmap(
                &mut pixels,
                width,
                height,
                archive,
                &format!("UIWindow.img/RpsGame/{character}"),
                panel_x + 50,
                panel_y + 40,
            )?;
        }
        let text_width = dialogue_text_width(font, message, 12.0).ceil() as usize;
        draw_dialogue_text(
            font,
            &mut pixels,
            width,
            height,
            message,
            panel_x + ((panel_width.saturating_sub(text_width)) / 2) as i32,
            panel_y + 300,
            12.0,
            panel_width,
            18,
            color,
        );
    }

    let score = format!("Wins: {}", game.wins);
    let score_width = dialogue_text_width(font, &score, 12.0).ceil() as usize;
    draw_dialogue_text(
        font,
        &mut pixels,
        width,
        height,
        &score,
        panel_x + ((panel_width.saturating_sub(score_width)) / 2) as i32,
        panel_y + 328,
        12.0,
        panel_width,
        18,
        [255, 255, 255],
    );

    Ok(pixels)
}

impl ClientApp {
    pub(super) fn open_rps_game(&mut self) {
        self.rps_game = Some(RpsGameRuntime::new());
        self.refresh_rps_game_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn apply_rps_game_event(&mut self, event: openstory_protocol::RpsGameEvent) {
        use openstory_protocol::RpsGameEvent;

        match event {
            RpsGameEvent::Open { .. } => self.open_rps_game(),
            RpsGameEvent::SelectionResult { player, opponent } => {
                if let Some(game) = self.rps_game.as_mut() {
                    let result = if player == opponent {
                        2
                    } else if matches!((player, opponent), (0, 2) | (1, 0) | (2, 1)) {
                        0
                    } else {
                        1
                    };
                    game.player_choice = Some(player);
                    game.opponent_choice = Some(opponent);
                    game.result = Some(result);
                    game.phase = RpsGamePhase::Result;
                    if result == 1 {
                        game.wins = game.wins.wrapping_add(1);
                    }
                    game.hovered = self.cursor.and_then(|point| rps_game_button_at(game, point));
                    self.refresh_rps_game_texture();
                }
            }
            RpsGameEvent::NotEnoughMesos => self.show_status_message(
                "Not enough mesos for Rock-Paper-Scissors.".to_owned(),
                [255, 0, 0],
            ),
            RpsGameEvent::Close => self.rps_game = None,
            RpsGameEvent::Unknown(mode) => {
                eprintln!("OpenStoryRS: ignored RPS_GAME mode {mode}");
            }
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn refresh_rps_game_texture(&mut self) {
        let Some(game) = self.rps_game.as_ref() else {
            return;
        };
        let pixels = match render_rps_game_panel(&self.ui_archive, &self.font, game) {
            Ok(pixels) => pixels,
            Err(error) => {
                eprintln!("OpenStoryRS: could not render v83 RPS game UI: {error}");
                return;
            }
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let result = if let Some(texture) = self
            .rps_game
            .as_ref()
            .and_then(|game| game.texture.as_ref())
        {
            renderer.update_bgra(texture, &pixels).map(|()| None)
        } else {
            renderer
                .upload_bgra(1024, 768, &pixels)
                .map(Some)
        };
        match result {
            Ok(Some(texture)) => {
                if let Some(game) = self.rps_game.as_mut() {
                    game.texture = Some(texture);
                }
            }
            Ok(None) => {}
            Err(error) => eprintln!("OpenStoryRS: could not upload v83 RPS game UI: {error}"),
        }
    }

    pub(super) fn update_rps_game_hover(&mut self) -> bool {
        let Some(game) = self.rps_game.as_ref() else {
            return false;
        };
        let hovered = self.cursor.and_then(|point| rps_game_button_at(game, point));
        if game.hovered == hovered {
            return false;
        }
        if let Some(game) = self.rps_game.as_mut() {
            game.hovered = hovered;
        }
        self.refresh_rps_game_texture();
        if hovered.is_some() {
            self.audio
                .play_ui_sound(&self.sound_archive, "UI.img/BtMouseOver");
        }
        true
    }

    pub(super) fn click_rps_game_button(&mut self, point: Point) {
        let button = self
            .rps_game
            .as_ref()
            .and_then(|game| rps_game_button_at(game, point));
        let Some(button) = button else {
            return;
        };
        self.audio
            .play_ui_sound(&self.sound_archive, "UI.img/BtMouseClick");
        match button {
            RpsGameButton::Start | RpsGameButton::Continue => {
                if let Some(game) = self.rps_game.as_mut() {
                    game.phase = RpsGamePhase::Selecting;
                    game.result = None;
                }
            }
            RpsGameButton::Rock => self.send_rps_selection(0),
            RpsGameButton::Paper => self.send_rps_selection(1),
            RpsGameButton::Scissor => self.send_rps_selection(2),
            RpsGameButton::Retry => {
                if let Some(game) = self.rps_game.as_mut() {
                    game.wins = 0;
                    game.result = None;
                    game.phase = RpsGamePhase::Selecting;
                }
            }
            RpsGameButton::Exit => {
                self.rps_game = None;
                return;
            }
        }
        if let Some(game) = self.rps_game.as_mut() {
            game.hovered = self.cursor.and_then(|point| rps_game_button_at(game, point));
        }
        self.refresh_rps_game_texture();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}
