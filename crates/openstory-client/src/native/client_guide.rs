use super::*;

impl ClientApp {
    pub(super) fn apply_guide_event(&mut self, event: openstory_protocol::GuideEvent) {
        match event {
            openstory_protocol::GuideEvent::Spawn { visible: true } => self.push_chat_line(
                "[Guide] Maple Guide has appeared! Press the guide key for help.".to_owned(),
                [255, 255, 0],
            ),
            openstory_protocol::GuideEvent::Spawn { visible: false } => {}
            openstory_protocol::GuideEvent::Text(text) => {
                self.push_chat_line(format!("[Guide] {text}"), [255, 255, 0]);
            }
            openstory_protocol::GuideEvent::Hint { id, duration_ms: _ } => {
                let text = self
                    .guide_hint_text(id)
                    .filter(|text| !text.trim().is_empty())
                    .unwrap_or_else(|| format!("Hint {id}"));
                self.push_chat_line(format!("[Guide] {text}"), [255, 255, 0]);
            }
        }
    }

    fn guide_hint_text(&self, id: i32) -> Option<String> {
        let root = self.ui_archive.root();
        let hint = root
            .resolve(&format!("tutorial.img/{id}"))
            .ok()
            .flatten()
            .or_else(|| {
                root.resolve(&format!("UIWindow.img/tutorial/{id}"))
                    .ok()
                    .flatten()
            })?;
        let text = hint.child("text").ok().flatten()?;
        match text.value().ok()? {
            NodeValue::String(text) => Some(text),
            _ => None,
        }
    }
}
