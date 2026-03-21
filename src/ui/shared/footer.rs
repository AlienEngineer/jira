use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    Frame,
};

use crate::{config::keymaps::Scope, ui::keycode_mapper::generate_keymaps};

pub struct Footer {
    pub status_msg: String,
    pub scopes: Vec<Scope>,
}

impl Footer {
    pub fn new(scopes: Vec<Scope>) -> Self {
        Self {
            status_msg: String::new(),
            scopes,
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
        generate_keymaps(&mut spans, self.scopes.clone());
        spans.push(status_span);
        frame.render_widget(Line::from(spans), area);
    }
}

impl Default for Footer {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
