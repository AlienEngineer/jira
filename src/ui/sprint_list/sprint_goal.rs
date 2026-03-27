use ratatui::{layout::Rect, Frame};

use crate::ui::components::{Component, UiComponent};

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
        frame.render_widget(
            UiComponent::labeled_text(" Sprint: ", self.sprint_name.as_str()),
            area,
        );
    }

    /// Render the bordered goal block. Does nothing when no goal is set.
    pub fn render_goal(&self, frame: &mut Frame, area: Rect) {
        if self.sprint_goal.is_empty() {
            return;
        }
        frame.render_widget(
            UiComponent::blocked_title(" Goal ", self.sprint_goal.as_str()),
            area,
        );
    }
}
