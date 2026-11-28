use std::collections::VecDeque;

#[derive(Default)]
pub(super) struct ChatHistory {
    entries: VecDeque<String>,
    position: Option<usize>,
    draft: String,
}

impl ChatHistory {
    pub(super) fn record(&mut self, message: &str) {
        const LIMIT: usize = 100;
        if self.entries.back().is_none_or(|last| last != message) {
            if self.entries.len() == LIMIT {
                self.entries.pop_front();
            }
            self.entries.push_back(message.to_owned());
        }
        self.position = None;
        self.draft.clear();
    }

    pub(super) fn older(&mut self, input: &mut String) {
        if self.entries.is_empty() {
            return;
        }
        let position = match self.position {
            Some(position) => position.saturating_sub(1),
            None => {
                self.draft = input.clone();
                self.entries.len() - 1
            }
        };
        self.position = Some(position);
        *input = self.entries[position].clone();
    }

    pub(super) fn newer(&mut self, input: &mut String) {
        let Some(position) = self.position else {
            return;
        };
        if position + 1 < self.entries.len() {
            self.position = Some(position + 1);
            *input = self.entries[position + 1].clone();
        } else {
            self.position = None;
            *input = std::mem::take(&mut self.draft);
        }
    }
}

pub(super) fn append_chat_text(input: &mut String, text: &str, max_chars: usize) {
    let remaining = max_chars.saturating_sub(input.chars().count());
    input.extend(
        text.chars()
            .filter(|character| !character.is_control())
            .take(remaining),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_navigates_and_restores_draft() {
        let mut history = ChatHistory::default();
        history.record("hello");
        history.record("/party test");
        let mut input = "draft".to_owned();
        history.older(&mut input);
        assert_eq!(input, "/party test");
        history.older(&mut input);
        history.older(&mut input);
        assert_eq!(input, "hello");
        history.newer(&mut input);
        assert_eq!(input, "/party test");
        history.newer(&mut input);
        assert_eq!(input, "draft");
    }

    #[test]
    fn pasted_text_respects_character_limit_and_filters_controls() {
        let mut input = "a".to_owned();
        append_chat_text(&mut input, "é\n🙂bc", 3);
        assert_eq!(input, "aé🙂");
    }
}
