use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
    Frame,
};

use crate::ui::components::ui_widget::UiWidget;

pub struct UiTitle {
    pub label: String,
    pub description: String,
}

impl UiTitle {
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: description.into(),
        }
    }
}

impl UiWidget for UiTitle {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        Line::from(vec![
            Span::raw(self.label.as_str()),
            Span::styled(
                self.description.as_str(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
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

    use crate::ui::components::{ui_title::UiTitle, ui_widget::UiWidget};

    #[test]
    fn rendering_title_renders_label_and_description() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 32, 1));

        UiTitle::new(" Sprint: ", "This is my description.").render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![" Sprint: This is my description."]);
        expected.set_style(Rect::new(0, 0, 9, 1), Style::new());
        expected.set_style(
            Rect::new(9, 0, 32 - 9, 1),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

        assert_eq!(buf, expected);
    }
}
