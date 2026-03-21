use crate::jira::lists::{ListFilter, FILTER_FIELDS};
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

// ── Actions ───────────────────────────────────────────────────────────────────

pub enum FilterEditorAction {
    Apply(Box<ListFilter>),
    Close,
}

// ── Internal mode ─────────────────────────────────────────────────────────────

enum EditorMode {
    PickField,
    TypeValue { field_idx: usize },
}

// ── FilterEditor ──────────────────────────────────────────────────────────────

pub struct FilterEditor {
    filter: ListFilter,
    mode: EditorMode,
    list_state: ListState,
    input: String,
}

impl FilterEditor {
    pub fn new(filter: ListFilter) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            filter,
            mode: EditorMode::PickField,
            list_state,
            input: String::new(),
        }
    }

    // ── Key handling ──────────────────────────────────────────────────────────

    pub fn handle_key(&mut self, key: KeyCode) -> Option<FilterEditorAction> {
        match &self.mode {
            EditorMode::PickField => self.handle_pick_field(key),
            EditorMode::TypeValue { field_idx } => {
                let idx = *field_idx;
                self.handle_type_value(key, idx)
            }
        }
    }

    fn handle_pick_field(&mut self, key: KeyCode) -> Option<FilterEditorAction> {
        let total = FILTER_FIELDS.len();
        match key {
            KeyCode::Down | KeyCode::Char('j') => {
                let next =
                    self.list_state.selected().map_or(
                        0,
                        |i| {
                            if i + 1 >= total {
                                0
                            } else {
                                i + 1
                            }
                        },
                    );
                self.list_state.select(Some(next));
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let prev =
                    self.list_state.selected().map_or(
                        0,
                        |i| {
                            if i == 0 {
                                total - 1
                            } else {
                                i - 1
                            }
                        },
                    );
                self.list_state.select(Some(prev));
                None
            }
            KeyCode::Enter => {
                if let Some(idx) = self.list_state.selected() {
                    let key_name = FILTER_FIELDS[idx].0;
                    self.input = self.filter.get_display(key_name);
                    self.mode = EditorMode::TypeValue { field_idx: idx };
                }
                None
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                if let Some(idx) = self.list_state.selected() {
                    let key_name = FILTER_FIELDS[idx].0;
                    self.filter.clear_field(key_name);
                }
                None
            }
            KeyCode::Char('m') => {
                self.filter.me = !self.filter.me;
                None
            }
            KeyCode::Char('F') => Some(FilterEditorAction::Apply(Box::new(self.filter.clone()))),
            KeyCode::Esc => Some(FilterEditorAction::Close),
            _ => None,
        }
    }

    fn handle_type_value(&mut self, key: KeyCode, field_idx: usize) -> Option<FilterEditorAction> {
        match key {
            KeyCode::Char(c) => {
                self.input.push(c);
                None
            }
            KeyCode::Backspace => {
                self.input.pop();
                None
            }
            KeyCode::Enter => {
                let key_name = FILTER_FIELDS[field_idx].0;
                self.filter.set_from_str(key_name, &self.input);
                self.input.clear();
                self.mode = EditorMode::PickField;
                None
            }
            KeyCode::Esc => {
                self.input.clear();
                self.mode = EditorMode::PickField;
                None
            }
            _ => None,
        }
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let popup_area = centered_rect(70, 80, area);

        frame.render_widget(Clear, popup_area);

        let title = match &self.mode {
            EditorMode::PickField => " Filters ",
            EditorMode::TypeValue { field_idx } => FILTER_FIELDS[*field_idx].1,
        };

        let block = Block::bordered()
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        match &self.mode {
            EditorMode::PickField => self.render_pick_field(frame, inner),
            EditorMode::TypeValue { field_idx } => {
                let idx = *field_idx;
                self.render_type_value(frame, inner, idx);
            }
        }
    }

    fn render_pick_field(&mut self, frame: &mut Frame, area: Rect) {
        // Field list
        let items: Vec<ListItem> = FILTER_FIELDS
            .iter()
            .map(|(key, label)| {
                let value = self.filter.get_display(key);
                let value_span = if value.is_empty() {
                    Span::styled("  —", Style::default().fg(Color::DarkGray))
                } else {
                    Span::styled(format!("  {value}"), Style::default().fg(Color::Cyan))
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{label:<14}"), Style::default().fg(Color::White)),
                    value_span,
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("❯ ");

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn render_type_value(&self, frame: &mut Frame, area: Rect, field_idx: usize) {
        let (key, label) = FILTER_FIELDS[field_idx];
        let layout = Layout::vertical([
            Constraint::Length(1), // label
            Constraint::Length(1), // spacer
            Constraint::Length(3), // input box
            Constraint::Min(0),    // remaining space
        ])
        .split(area);

        // Label
        let is_multi = !matches!(key, "text" | "jql");
        let hint_text = if is_multi {
            "Separate multiple values with commas"
        } else {
            "Single value"
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("Field: ", Style::default().fg(Color::DarkGray)),
                Span::styled(label, Style::default().fg(Color::Yellow).bold()),
                Span::styled(
                    format!("  ({hint_text})"),
                    Style::default().fg(Color::DarkGray),
                ),
            ])),
            layout[0],
        );

        // Input box
        let input_text = format!("{}_", self.input);
        frame.render_widget(
            Paragraph::new(input_text.clone())
                .block(Block::bordered().border_style(Style::default().fg(Color::Cyan)))
                .alignment(Alignment::Left),
            layout[2],
        );
    }
}

// ── Layout helpers ────────────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .split(area);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0]);
    horizontal[0]
}
