use super::*;

impl ClientApp {
    pub(super) fn show_avatar_megaphone(&mut self, update: openstory_protocol::AvatarMegaphone) {
        let animation = match self.load_avatar_megaphone_animation(update.item_id) {
            Ok(animation) => animation,
            Err(error) => {
                eprintln!("OpenStoryRS: could not load avatar-megaphone art: {error}");
                return;
            }
        };
        let text_pixels = match avatar_megaphone::render_text(
            &self.map_archive,
            &self.font,
            &update.sender,
            &update.lines,
        ) {
            Ok(pixels) => pixels,
            Err(error) => {
                eprintln!("OpenStoryRS: could not render avatar-megaphone text: {error}");
                return;
            }
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let text_texture = match renderer.upload_bgra(
            avatar_megaphone::PANEL_WIDTH,
            avatar_megaphone::PANEL_HEIGHT,
            &text_pixels,
        ) {
            Ok(texture) => texture,
            Err(error) => {
                eprintln!("OpenStoryRS: could not upload avatar-megaphone text: {error}");
                return;
            }
        };

        let mut avatar_sprites = Vec::new();
        let mut art_cache = MapArtCache::default();
        if let Err(error) = append_character_preview(
            &mut avatar_sprites,
            &self.character_archive,
            &update.sender_look,
            1024.0 - avatar_megaphone::PANEL_WIDTH as f32 + 4.0 + 70.0,
            4.0 + 92.0,
            1.0,
            false,
            &mut art_cache,
        ) {
            eprintln!("OpenStoryRS: could not render avatar-megaphone sender: {error}");
        }
        let avatar_draws = match self.upload_dynamic_sprites(avatar_sprites) {
            Ok(draws) => draws,
            Err(error) => {
                eprintln!("OpenStoryRS: could not upload avatar-megaphone sender: {error}");
                Vec::new()
            }
        };

        self.avatar_megaphone = Some(AvatarMegaphoneUi {
            animation,
            text_texture,
            avatar_draws,
            expires_at: Instant::now() + Duration::from_secs(10),
        });
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn clear_avatar_megaphone(&mut self) {
        self.avatar_megaphone = None;
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(super) fn advance_avatar_megaphone(&mut self, elapsed: Duration, now: Instant) {
        let expired = self
            .avatar_megaphone
            .as_ref()
            .is_some_and(|banner| now >= banner.expires_at);
        if expired {
            self.avatar_megaphone = None;
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        } else if let Some(banner) = self.avatar_megaphone.as_mut() {
            banner.animation.advance_loop(elapsed);
        }
    }

    fn load_avatar_megaphone_animation(&self, item_id: i32) -> Result<MapleTvAnimation, String> {
        const MAX_FRAMES: usize = 128;
        let variant = match item_id {
            5_390_001 => "Bright",
            5_390_002 => "Heart",
            5_390_005 => "Tiger1",
            5_390_006 => "Tiger2",
            _ => "Burning",
        };
        let path = format!("MapHelper.img/AvatarMegaphone/{variant}");
        let source = self
            .map_archive
            .root()
            .resolve(&path)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("avatar-megaphone animation {path:?} was not found"))?;
        let frame_nodes = numeric_children(&source)?;
        if frame_nodes.len() > MAX_FRAMES {
            return Err(format!(
                "avatar-megaphone animation {path:?} has {} frames (limit {MAX_FRAMES})",
                frame_nodes.len()
            ));
        }
        let mut frames = Vec::with_capacity(frame_nodes.len().max(1));
        let frame_nodes = if frame_nodes.is_empty() {
            vec![source]
        } else {
            frame_nodes
        };
        for frame in frame_nodes {
            let frame_path = if frame.name().ok().as_deref() == Some(variant) {
                path.clone()
            } else {
                format!(
                    "{path}/{}",
                    frame.name().map_err(|error| error.to_string())?
                )
            };
            let (width, height, pixels, origin) = read_ui_bitmap(&self.map_archive, &frame_path)?;
            let renderer = self
                .renderer
                .as_ref()
                .ok_or_else(|| "graphics are not ready for avatar-megaphone art".to_owned())?;
            let texture = renderer
                .upload_bgra(width, height, &pixels)
                .map_err(|error| error.to_string())?;
            let delay = if frame.name().ok().as_deref() == Some(variant) {
                100
            } else {
                integer_property(&frame, "delay")
                    .and_then(|delay| u64::try_from(delay).ok())
                    .filter(|delay| *delay > 0)
                    .unwrap_or(100)
            };
            frames.push(MapleTvAnimationFrame {
                texture,
                delay: Duration::from_millis(delay),
                origin,
            });
        }
        if frames.is_empty() {
            return Err(format!("avatar-megaphone animation {path:?} has no frames"));
        }
        Ok(MapleTvAnimation {
            frames,
            frame_index: 0,
            elapsed: Duration::ZERO,
        })
    }
}
