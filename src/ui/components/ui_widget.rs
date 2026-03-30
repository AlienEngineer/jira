use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    Frame,
};

pub trait UiWidget {
    fn render_widget(&mut self, frame: &mut Frame, area: Rect);
    fn render(&mut self, area: Rect, buf: &mut Buffer);
    fn get_constraint(&self) -> Constraint;
    fn skip(&self) -> bool;
}
