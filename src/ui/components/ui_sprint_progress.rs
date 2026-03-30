use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
    Frame,
};

use crate::ui::components::ui_widget::UiWidget;

pub struct UiSprintProgress {
    pub label: String,
    pub title: String,
    pub total: usize,
    pub resolved: usize,
}

impl UiSprintProgress {
    pub fn new(
        label: impl Into<String>,
        title: impl Into<String>,
        total: usize,
        resolved: usize,
    ) -> Self {
        Self {
            label: label.into(),
            title: title.into(),
            total,
            resolved,
        }
    }

    fn calculate_percentage(&self) -> usize {
        match self.total > 0 {
            true => self.resolved * 100 / self.total,
            false => 0,
        }
    }

    fn get_bars(&self) -> String {
        format!(
            "[{}{}]",
            "█".repeat(self.resolved),
            "░".repeat(self.total - self.resolved)
        )
    }

    fn get_color(&self) -> Color {
        let pct = self.calculate_percentage();
        if pct >= 80 {
            Color::Green
        } else if pct >= 40 {
            Color::Yellow
        } else {
            Color::Red
        }
    }
}

impl UiWidget for UiSprintProgress {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let bar_color = self.get_color();
        Paragraph::new(Line::from(vec![
            Span::styled(self.get_bars(), Style::default().fg(bar_color)),
            Span::styled(
                format!(
                    " {}% ({}/{} resolved) ",
                    self.calculate_percentage(),
                    self.resolved,
                    self.total
                ),
                Style::default().fg(Color::White),
            ),
            Span::styled(self.label.as_str(), Style::default().fg(Color::Cyan)),
        ]))
        .block(
            Block::bordered()
                .title(Span::styled(
                    self.title.as_str(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .render(area, buf);
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

#[cfg(test)]
mod test {

    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Modifier, Style},
    };

    use crate::ui::components::{ui_sprint_progress::UiSprintProgress, ui_widget::UiWidget};

    #[test]
    fn rendering_sprint_progress_with_low_progress_renders_it_with_red_bar() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 3));

        UiSprintProgress::new("This is my label", " Sprint Progress ", 10, 1)
            .render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "┌ Sprint Progress ─────────────────────────────────────────────────────────────┐",
            "│[█░░░░░░░░░] 10% (1/10 resolved) This is my label                             │",
            "└──────────────────────────────────────────────────────────────────────────────┘",
        ]);

        // Row 0: border (full row), then title overlay
        expected.set_style(Rect::new(0, 0, 80, 1), Style::default().fg(Color::DarkGray));
        expected.set_style(Rect::new(1, 0, 17, 1), get_title_style());

        // Row 1: left border, bar, stats, label, padding, right border
        expected.set_style(Rect::new(0, 1, 1, 1), Style::default().fg(Color::DarkGray));
        expected.set_style(Rect::new(1, 1, 12, 1), Style::default().fg(Color::Red));
        expected.set_style(Rect::new(13, 1, 21, 1), Style::default().fg(Color::White));
        expected.set_style(Rect::new(34, 1, 16, 1), Style::default().fg(Color::Cyan));
        expected.set_style(Rect::new(79, 1, 1, 1), Style::default().fg(Color::DarkGray));

        // Row 2: border (full row)
        expected.set_style(Rect::new(0, 2, 80, 1), Style::default().fg(Color::DarkGray));

        assert_eq!(buf, expected);
    }

    #[test]
    fn rendering_sprint_progress_at_50_percent_renders_it_with_yellow_bar() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 3));

        UiSprintProgress::new("This is my label", " Sprint Progress ", 10, 5)
            .render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "┌ Sprint Progress ─────────────────────────────────────────────────────────────┐",
            "│[█████░░░░░] 50% (5/10 resolved) This is my label                             │",
            "└──────────────────────────────────────────────────────────────────────────────┘",
        ]);

        // Row 0: border (full row), then title overlay
        expected.set_style(Rect::new(0, 0, 80, 1), Style::default().fg(Color::DarkGray));
        expected.set_style(Rect::new(1, 0, 17, 1), get_title_style());

        // Row 1: left border, bar, stats, label, padding, right border
        expected.set_style(Rect::new(0, 1, 1, 1), Style::default().fg(Color::DarkGray));
        expected.set_style(Rect::new(1, 1, 12, 1), Style::default().fg(Color::Yellow));
        expected.set_style(Rect::new(13, 1, 21, 1), Style::default().fg(Color::White));
        expected.set_style(Rect::new(34, 1, 16, 1), Style::default().fg(Color::Cyan));
        expected.set_style(Rect::new(79, 1, 1, 1), Style::default().fg(Color::DarkGray));

        // Row 2: border (full row)
        expected.set_style(Rect::new(0, 2, 80, 1), Style::default().fg(Color::DarkGray));

        assert_eq!(buf, expected);
    }

    #[test]
    fn rendering_sprint_progress_at_100_percent_renders_it_with_green_bar() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 3));

        UiSprintProgress::new("This is my label", " Sprint Progress ", 10, 10)
            .render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "┌ Sprint Progress ─────────────────────────────────────────────────────────────┐",
            "│[██████████] 100% (10/10 resolved) This is my label                           │",
            "└──────────────────────────────────────────────────────────────────────────────┘",
        ]);

        // Row 0: border (full row), then title overlay
        expected.set_style(Rect::new(0, 0, 80, 1), Style::default().fg(Color::DarkGray));
        expected.set_style(Rect::new(1, 0, 17, 1), get_title_style());

        // Row 1: left border, bar, stats, label, padding, right border
        expected.set_style(Rect::new(0, 1, 1, 1), Style::default().fg(Color::DarkGray));
        expected.set_style(Rect::new(1, 1, 12, 1), Style::default().fg(Color::Green));
        expected.set_style(Rect::new(13, 1, 23, 1), Style::default().fg(Color::White));
        expected.set_style(Rect::new(36, 1, 16, 1), Style::default().fg(Color::Cyan));
        expected.set_style(Rect::new(79, 1, 1, 1), Style::default().fg(Color::DarkGray));

        // Row 2: border (full row)
        expected.set_style(Rect::new(0, 2, 80, 1), Style::default().fg(Color::DarkGray));

        assert_eq!(buf, expected);
    }

    fn get_title_style() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }
}
