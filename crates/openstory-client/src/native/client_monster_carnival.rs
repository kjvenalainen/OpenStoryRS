use super::*;

impl ClientApp {
    pub(super) fn apply_monster_carnival_event(
        &mut self,
        event: openstory_protocol::MonsterCarnivalEvent,
    ) {
        match event {
            openstory_protocol::MonsterCarnivalEvent::Start {
                team,
                cp,
                total_cp,
                enemy_cp,
                enemy_total_cp,
            } => {
                let mut state = monster_carnival::State::default();
                state.team = Some(team);
                state.my_cp = cp;
                state.my_total_cp = total_cp;
                state.enemy_cp = enemy_cp;
                state.enemy_total_cp = enemy_total_cp;
                self.monster_carnival = Some(MonsterCarnivalUi {
                    state,
                    texture: None,
                    close: monster_carnival::HitRect {
                        x: 0.0,
                        y: 0.0,
                        width: 0.0,
                        height: 0.0,
                    },
                    side: monster_carnival::HitRect {
                        x: 0.0,
                        y: 0.0,
                        width: 0.0,
                        height: 0.0,
                    },
                });
            }
            openstory_protocol::MonsterCarnivalEvent::Cp { cp, total_cp } => {
                if let Some(carnival) = self.monster_carnival.as_mut() {
                    carnival.state.my_cp = cp;
                    carnival.state.my_total_cp = total_cp;
                }
            }
            openstory_protocol::MonsterCarnivalEvent::PartyCp { team, cp, total_cp } => {
                if let Some(carnival) = self.monster_carnival.as_mut() {
                    if team == 0 {
                        carnival.state.my_cp = cp;
                        carnival.state.my_total_cp = total_cp;
                    } else {
                        carnival.state.enemy_cp = cp;
                        carnival.state.enemy_total_cp = total_cp;
                    }
                }
            }
            openstory_protocol::MonsterCarnivalEvent::Summon {
                category,
                count,
                name,
            } => {
                if (0..4).contains(&category)
                    && let Some(carnival) = self.monster_carnival.as_mut()
                {
                    carnival.state.summon_counts[category as usize] = count;
                }
                self.push_chat_line(
                    format!("[Carnival] {name} summoned a monster!"),
                    [255, 255, 0],
                );
            }
            openstory_protocol::MonsterCarnivalEvent::Message { code } => {
                let message = match code {
                    1 => "You don't have enough CP!".to_owned(),
                    2 => "The summon limit has been reached!".to_owned(),
                    3 => "You cannot summon during this phase!".to_owned(),
                    4 => "The carnival is about to end!".to_owned(),
                    _ => format!("Carnival message: {code}"),
                };
                self.push_chat_line(format!("[Carnival] {message}"), [255, 255, 0]);
            }
            openstory_protocol::MonsterCarnivalEvent::Died {
                team: _,
                name,
                lost_cp,
            } => self.push_chat_line(
                format!("[Carnival] {name} was eliminated! (-{lost_cp} CP)"),
                [255, 80, 80],
            ),
        }
        self.refresh_monster_carnival_texture();
    }

    pub(super) fn refresh_monster_carnival_texture(&mut self) {
        let Some(state) = self
            .monster_carnival
            .as_ref()
            .map(|carnival| carnival.state.clone())
        else {
            return;
        };
        let panel =
            match monster_carnival::render_state(&self.legacy_ui_archive, &self.font, &state) {
                Ok(panel) => panel,
                Err(error) => {
                    eprintln!("OpenStoryRS: could not render Monster Carnival panel: {error}");
                    return;
                }
            };
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        match renderer.upload_bgra(panel.width, panel.height, &panel.pixels) {
            Ok(texture) => {
                if let Some(carnival) = self.monster_carnival.as_mut() {
                    carnival.texture = Some(texture);
                    carnival.close = panel.close;
                    carnival.side = panel.side;
                }
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            Err(error) => {
                eprintln!("OpenStoryRS: could not upload Monster Carnival panel: {error}")
            }
        }
    }
}
