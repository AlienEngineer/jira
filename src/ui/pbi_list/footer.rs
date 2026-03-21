use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    Frame,
};

use crate::lua::init::get_keymap_collection;

pub struct Footer {
    pub status_msg: String,
}

impl Footer {
    pub fn new() -> Self {
        Self {
            status_msg: String::new(),
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = msg.into();
    }

    #[allow(dead_code)]
    pub fn clear_status(&mut self) {
        self.status_msg.clear();
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let status_span = if self.status_msg.is_empty() {
            Span::raw("")
        } else {
            Span::styled(
                format!("  [{}]", self.status_msg),
                Style::default().fg(Color::Green),
            )
        };

        let mut spans: Vec<Span> = vec![Span::raw(" ")];
        append_key_maps(&mut spans);
        spans.push(status_span);
        frame.render_widget(Line::from(spans), area);
    }
}

fn append_key_maps(spans: &mut Vec<Span<'_>>) {
    if let Some(collection) = get_keymap_collection() {
        let guard = collection.lock().expect("Failed to lock keymaps");
        let keymaps = guard.get_keymaps();
        let plugin_spans: Vec<Span> = keymaps
            .iter()
            .filter(|k| k.description.is_some())
            .flat_map(|k| {
                [
                    Span::styled(k.key.clone(), Style::default().fg(Color::Yellow).bold()),
                    Span::raw(format!(" {}  ", k.description.as_ref().unwrap())),
                ]
            })
            .collect();

        spans.extend(plugin_spans);
    }
}
