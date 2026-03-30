use ratatui::{layout::Rect, Frame};

use crate::ui::components::{ui_boxed_title::UiBoxedTitle, ui_title::UiTitle, ui_widget::UiWidget};

/// Renders the sprint title bar and optional sprint-goal block.
///
/// Owns the sprint name and goal strings; has no interactive behaviour.
pub struct SprintGoalWidget {
    pub sprint_name: String,
    pub sprint_goal: String,
}

impl SprintGoalWidget {
    pub fn new(sprint_name: String, sprint_goal: String) -> Self {
        Self {
            sprint_name,
            sprint_goal,
        }
    }

    /// Height (in terminal rows) required for the goal block.
    /// Returns 0 when no goal is set (the slot collapses entirely).
    pub fn goal_height(&self) -> u16 {
        if self.sprint_goal.is_empty() {
            0
        } else {
            3
        }
    }

    /// Render the single-line title bar (sprint name).
    pub fn render_title(&self, frame: &mut Frame, area: Rect) {
        UiTitle::new(" Sprint: ", self.sprint_name.as_str()).render_widget(frame, area);
    }

    /// Render the bordered goal block. Does nothing when no goal is set.
    pub fn render_goal(&self, frame: &mut Frame, area: Rect) {
        if self.sprint_goal.is_empty() {
            return;
        }
        UiBoxedTitle::new(" Goal ", self.sprint_goal.as_str()).render_widget(frame, area);
    }
}
