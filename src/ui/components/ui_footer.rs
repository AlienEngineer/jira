use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
    Frame,
};

use crate::{
    config::keymaps::Scope,
    ui::{components::ui_widget::UiWidget, keycode_mapper::generate_keymaps},
};

#[derive(Clone)]
pub struct UiFooter {
    pub status_msg: String,
    pub scopes: Vec<Scope>,
}

impl UiFooter {
    pub fn new(scopes: Vec<Scope>) -> Self {
        Self {
            status_msg: String::new(),
            scopes,
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = msg.into();
    }

    pub fn clear_status(&mut self) {
        self.status_msg.clear();
    }
}

impl UiWidget for UiFooter {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
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

        Line::from(spans).render(area, buf);
    }

    fn get_constraint(&self) -> Constraint {
        Constraint::Length(1)
    }

    fn skip(&self) -> bool {
        false
    }

    fn render_widget(&mut self, frame: &mut Frame, area: Rect) {
        self.render(area, frame.buffer_mut());
    }
}
