mod footer;
mod progress_block;
mod sprint_goal;
mod sprint_table;

use footer::Footer;
use progress_block::{ProgressBlock, SprintProgressData};
use sprint_goal::SprintGoalWidget;
use sprint_table::{SprintTable, TableAction};

use crate::jira::sprint::{self, Pbi, Sprint};
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use std::time::Duration;

/// Top-level coordinator that owns all sprint UI components and drives the
/// event loop.
///
/// `SprintApp` is responsible for:
/// - Composing the terminal layout
/// - Routing key events to `SprintTable` and dispatching the returned
///   [`TableAction`]s to the appropriate component
/// - Cross-cutting concerns: persisting the sprint cache (needs data from
///   multiple components)
pub struct SprintApp {
    goal: SprintGoalWidget,
    table: SprintTable,
    progress: ProgressBlock,
    footer: Footer,
    board_id: String,
    sprint_end_date: String,
    exit: bool,
}

impl SprintApp {
    pub fn new(
        sprint_name: String,
        sprint_goal: String,
        sprint_end_date: String,
        board_id: String,
        pbis: Vec<Pbi>,
    ) -> Self {
        Self {
            goal: SprintGoalWidget::new(sprint_name, sprint_goal),
            table: SprintTable::new(board_id.clone(), pbis),
            progress: ProgressBlock::new(),
            footer: Footer::new(),
            board_id,
            sprint_end_date,
            exit: false,
        }
    }

    pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> crate::prelude::Result<()> {
        while !self.exit {
            self.process_background_messages();
            terminal.draw(|frame| self.draw(frame))?;

            // Short timeout keeps the UI refreshing during background loads.
            if event::poll(Duration::from_millis(50))? {
                self.handle_events()?;
            }
        }
        Ok(())
    }

    // ── Background messages ───────────────────────────────────────────────────

    fn process_background_messages(&mut self) {
        if let Some(update) = self.table.process_messages() {
            self.footer.set_status(update.status);
            self.save_cache();
        }
    }

    // ── Cache persistence ─────────────────────────────────────────────────────

    fn save_cache(&self) {
        sprint::save_sprint_cache(
            &self.board_id,
            &Sprint {
                name: self.goal.sprint_name.to_string(),
                goal: self.goal.sprint_goal.to_string(),
                end_date: self.sprint_end_date.to_string(),
                pbis: self.table.pbis().to_vec(),
            },
        );
    }

    // ── Layout & rendering ────────────────────────────────────────────────────

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let goal_height = self.goal.goal_height();

        let layout = Layout::vertical([
            Constraint::Length(1),           // title bar
            Constraint::Length(goal_height), // sprint goal (collapses when empty)
            Constraint::Min(0),              // PBI table
            Constraint::Length(3),           // progress block
            Constraint::Length(1),           // footer / key hints
        ])
        .split(area);

        self.goal.render_title(frame, layout[0]);
        self.goal.render_goal(frame, layout[1]);
        self.table.render(frame, layout[2]);

        let progress_data =
            SprintProgressData::from_sprint(self.table.pbis(), &self.sprint_end_date);
        self.progress.render(frame, layout[3], &progress_data);

        self.footer.render(frame, layout[4]);
    }

    // ── Event handling ────────────────────────────────────────────────────────

    fn handle_events(&mut self) -> crate::prelude::Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                for action in self.table.handle_key(key.code) {
                    self.dispatch(action);
                }
            }
        }
        Ok(())
    }

    fn dispatch(&mut self, action: TableAction) {
        match action {
            TableAction::Exit => self.exit = true,
            TableAction::SetStatus(msg) => self.footer.set_status(msg),
            TableAction::ClearStatus => self.footer.clear_status(),
            TableAction::SaveCache => self.save_cache(),
        }
    }
}
