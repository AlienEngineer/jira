use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    Frame,
};

use crate::ui::components::ui_widget::UiWidget;

pub struct UiLayout {
    widgets: Vec<Box<dyn UiWidget>>,
}

impl UiWidget for UiLayout {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.render(area, buf);
    }

    fn get_constraint(&self) -> ratatui::layout::Constraint {
        Constraint::Min(0)
    }

    fn skip(&self) -> bool {
        self.widgets.is_empty()
    }

    fn render_widget(&mut self, frame: &mut Frame, area: Rect) {
        let constraints: Vec<Constraint> = self
            .widgets
            .iter()
            .filter(|w| !w.skip())
            .map(|w| w.get_constraint())
            .collect();

        let layout = Layout::vertical(constraints).split(area);

        let mut idx = 0;
        for widget in self.widgets.iter_mut() {
            if !widget.skip() {
                widget.render_widget(frame, layout[idx]);
                idx += 1;
            }
        }
    }
}

impl UiLayout {
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
        }
    }

    pub fn add_widget(&mut self, widget: Box<dyn UiWidget>) {
        self.widgets.push(widget);
    }

    fn visible_widgets(&self) -> impl Iterator<Item = &dyn UiWidget> {
        self.widgets
            .iter()
            .map(|w| w.as_ref())
            .filter(|w| !w.skip())
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let layout =
            Layout::vertical(self.visible_widgets().map(|w| w.get_constraint())).split(area);

        let mut idx = 0;
        for widget in self.widgets.iter_mut() {
            if !widget.skip() {
                widget.render(layout[idx], buf);
                idx += 1;
            }
        }
    }
}

impl Default for UiLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {

    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Modifier, Style},
    };

    use crate::ui::components::{ui_layout::UiLayout, ui_title::UiTitle};

    #[test]
    fn rendering_title_in_layout_renders_label_and_description() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 32, 1));

        let mut layout = UiLayout::new();
        layout.add_widget(Box::new(UiTitle::new(
            " Sprint: ",
            "This is my description.",
        )));
        layout.render(buf.area, &mut buf);

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
