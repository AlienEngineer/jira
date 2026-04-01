use chrono::{Local, NaiveDate};
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
    pub title: String,
    pub total: usize,
    pub resolved: usize,
    end_date: String,
}

static mut TEST_NOW: Option<NaiveDate> = None;

fn get_now() -> NaiveDate {
    match unsafe { TEST_NOW } {
        Some(now) => now,
        _ => Local::now().date().naive_local(),
    }
}

impl UiSprintProgress {
    pub fn new(
        title: impl Into<String>,
        total: usize,
        resolved: usize,
        end_date: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            total,
            resolved,
            end_date: end_date.into(),
        }
    }

    fn calculate_percentage(&self) -> usize {
        match self.total > 0 {
            true => self.resolved * 100 / self.total,
            false => 0,
        }
    }

    fn count_working_days(end: chrono::NaiveDate, today: chrono::NaiveDate) -> Option<i64> {
        use chrono::{Datelike, Weekday};
        let mut count = 0i64;
        let mut d = today;
        while d <= end {
            match d.weekday() {
                Weekday::Sat | Weekday::Sun => {}
                _ => count += 1,
            }
            d = d.succ_opt()?;
        }
        Some(count)
    }

    fn compute_working_days(&self) -> Option<i64> {
        if self.end_date.is_empty() {
            return None;
        }
        let end =
            NaiveDate::parse_from_str(&self.end_date[..10.min(self.end_date.len())], "%Y-%m-%d")
                .ok()?;
        let today = get_now();
        if today > end {
            return Some(0);
        }
        Self::count_working_days(end, today)
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

    fn get_label(&mut self) -> String {
        let working_days_label = match self.compute_working_days() {
            Some(0) => "Sprint ends today!".to_string(),
            Some(d) => format!("{} working day{} left", d, if d == 1 { "" } else { "s" }),
            None => String::new(),
        };
        working_days_label
    }
}

impl UiWidget for UiSprintProgress {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(Line::from(vec![
            Span::styled(self.get_bars(), Style::default().fg(self.get_color())),
            Span::styled(
                format!(
                    " {}% ({}/{} resolved) ",
                    self.calculate_percentage(),
                    self.resolved,
                    self.total
                ),
                Style::default().fg(Color::White),
            ),
            Span::styled(self.get_label(), Style::default().fg(Color::Cyan)),
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
        Constraint::Length(3)
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

    use chrono::NaiveDate;
    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Modifier, Style},
    };

    use crate::ui::components::{
        ui_sprint_progress::{UiSprintProgress, TEST_NOW},
        ui_widget::UiWidget,
    };

    #[test]
    fn rendering_sprint_progress_with_low_progress_renders_it_with_red_bar() {
        unsafe { TEST_NOW = Some(NaiveDate::from_ymd_opt(2026, 3, 15).unwrap()) };
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 3));

        UiSprintProgress::new(" Sprint Progress ", 10, 1, "2026-03-20").render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "┌ Sprint Progress ─────────────────────────────────────────────────────────────┐",
            "│[█░░░░░░░░░] 10% (1/10 resolved) 5 working days left                          │",
            "└──────────────────────────────────────────────────────────────────────────────┘",
        ]);

        // Row 0: border (full row), then title overlay
        expected.set_style(Rect::new(0, 0, 80, 1), Style::default().fg(Color::DarkGray));
        expected.set_style(Rect::new(1, 0, 17, 1), get_title_style());

        // Row 1: left border, bar, stats, label, padding, right border
        expected.set_style(Rect::new(0, 1, 1, 1), Style::default().fg(Color::DarkGray));
        expected.set_style(Rect::new(1, 1, 12, 1), Style::default().fg(Color::Red));
        expected.set_style(Rect::new(13, 1, 21, 1), Style::default().fg(Color::White));
        expected.set_style(Rect::new(34, 1, 19, 1), Style::default().fg(Color::Cyan));
        expected.set_style(Rect::new(79, 1, 1, 1), Style::default().fg(Color::DarkGray));

        // Row 2: border (full row)
        expected.set_style(Rect::new(0, 2, 80, 1), Style::default().fg(Color::DarkGray));

        assert_eq!(buf, expected);
    }

    #[test]
    fn rendering_sprint_progress_at_50_percent_renders_it_with_yellow_bar() {
        unsafe { TEST_NOW = Some(NaiveDate::from_ymd_opt(2026, 3, 15).unwrap()) };
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 3));

        UiSprintProgress::new(" Sprint Progress ", 10, 5, "2026-03-15").render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "┌ Sprint Progress ─────────────────────────────────────────────────────────────┐",
            "│[█████░░░░░] 50% (5/10 resolved) Sprint ends today!                           │",
            "└──────────────────────────────────────────────────────────────────────────────┘",
        ]);

        // Row 0: border (full row), then title overlay
        expected.set_style(Rect::new(0, 0, 80, 1), Style::default().fg(Color::DarkGray));
        expected.set_style(Rect::new(1, 0, 17, 1), get_title_style());

        // Row 1: left border, bar, stats, label, padding, right border
        expected.set_style(Rect::new(0, 1, 1, 1), Style::default().fg(Color::DarkGray));
        expected.set_style(Rect::new(1, 1, 12, 1), Style::default().fg(Color::Yellow));
        expected.set_style(Rect::new(13, 1, 21, 1), Style::default().fg(Color::White));
        expected.set_style(Rect::new(34, 1, 18, 1), Style::default().fg(Color::Cyan));
        expected.set_style(Rect::new(79, 1, 1, 1), Style::default().fg(Color::DarkGray));

        // Row 2: border (full row)
        expected.set_style(Rect::new(0, 2, 80, 1), Style::default().fg(Color::DarkGray));

        assert_eq!(buf, expected);
    }

    #[test]
    fn rendering_sprint_progress_at_100_percent_renders_it_with_green_bar() {
        unsafe { TEST_NOW = Some(NaiveDate::from_ymd_opt(2026, 3, 15).unwrap()) };
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 3));

        UiSprintProgress::new(" Sprint Progress ", 10, 10, "2026-04-05").render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "┌ Sprint Progress ─────────────────────────────────────────────────────────────┐",
            "│[██████████] 100% (10/10 resolved) 15 working days left                       │",
            "└──────────────────────────────────────────────────────────────────────────────┘",
        ]);

        // Row 0: border (full row), then title overlay
        expected.set_style(Rect::new(0, 0, 80, 1), Style::default().fg(Color::DarkGray));
        expected.set_style(Rect::new(1, 0, 17, 1), get_title_style());

        // Row 1: left border, bar, stats, label, padding, right border
        expected.set_style(Rect::new(0, 1, 1, 1), Style::default().fg(Color::DarkGray));
        expected.set_style(Rect::new(1, 1, 12, 1), Style::default().fg(Color::Green));
        expected.set_style(Rect::new(13, 1, 23, 1), Style::default().fg(Color::White));
        expected.set_style(Rect::new(36, 1, 20, 1), Style::default().fg(Color::Cyan));
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
