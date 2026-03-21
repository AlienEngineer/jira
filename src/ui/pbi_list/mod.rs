mod filter_editor;
mod footer;
mod table;

use filter_editor::{FilterEditor, FilterEditorAction};
use footer::Footer;
use table::IssueTable;

use crate::jira::lists::{ListFilter, ListService};
use crate::jira::pbi::Pbi;
use crate::lua::init::{take_command_receiver, JiraCommand};
use crate::prelude::Result;
use crate::ui::pbi_detail::PbiDetailView;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    DefaultTerminal, Frame,
};

use std::env;
use std::fs;
use std::process::Command;
use std::sync::mpsc::Receiver;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

// ── Background fetch message ──────────────────────────────────────────────────

enum BgMsg {
    Loaded(Vec<Pbi>),
    Error(String),
}

// ── Active view ───────────────────────────────────────────────────────────────

enum ActiveView {
    List,
    Detail(Box<PbiDetailView>),
    FilterEditor(Box<FilterEditor>),
}

// ── PbiListApp ────────────────────────────────────────────────────────────────

pub struct PbiListApp {
    issues: Vec<Pbi>,
    filter: ListFilter,
    list_service: Arc<dyn ListService>,
    table: IssueTable,
    footer: Footer,
    active_view: ActiveView,
    exit: bool,
    bg_rx: Option<mpsc::Receiver<BgMsg>>,
    selected_pbi: Option<Pbi>,
    command_rx: Option<Receiver<JiraCommand>>,
}

impl PbiListApp {
    pub fn new(issues: Vec<Pbi>, filter: ListFilter, list_service: Arc<dyn ListService>) -> Self {
        let table = IssueTable::new(issues.len());
        Self {
            table,
            issues,
            filter,
            selected_pbi: None,
            list_service,
            footer: Footer::new(),
            active_view: ActiveView::List,
            exit: false,
            bg_rx: None,
            command_rx: take_command_receiver(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            self.process_bg_messages();
            self.process_lua_commands();

            if let Some(pbi) = self.selected_pbi.take() {
                ratatui::restore();
                self.open_raw_in_editor(&pbi);
                *terminal = ratatui::init();
            }

            terminal.draw(|frame| self.draw(frame))?;
            if event::poll(Duration::from_millis(50))? {
                self.handle_events()?;
            }
        }
        Ok(())
    }

    /// Process any commands received from Lua keybindings
    fn process_lua_commands(&mut self) {
        let commands: Vec<JiraCommand> = {
            let Some(rx) = &self.command_rx else { return };
            rx.try_iter().collect()
        };

        for cmd in commands {
            match &mut self.active_view {
                ActiveView::List => self.process_list_command(cmd),
                ActiveView::Detail(detail) => match cmd {
                    JiraCommand::GoLeft => {
                        self.active_view = ActiveView::List;
                    }
                    JiraCommand::GoUp => {
                        detail.scroll_up();
                    }
                    JiraCommand::GoDown => {
                        detail.scroll_down();
                    }
                    JiraCommand::OpenRawPbiJson => {
                        self.selected_pbi = Some(detail.pbi.clone());
                    }
                    _ => {}
                },
                ActiveView::FilterEditor(_) => {
                    // Filter editor handles its own keys directly, not via commands
                }
            }
        }
    }

    fn process_list_command(&mut self, cmd: JiraCommand) {
        match cmd {
            JiraCommand::GoUp => {
                self.table.navigate_up(self.issues.len());
            }
            JiraCommand::GoDown => {
                self.table.navigate_down(self.issues.len());
            }
            JiraCommand::GoLeft => {
                // no-op or back
            }
            JiraCommand::GoRight | JiraCommand::OpenPbiDetails => {
                if let Some(pbi) = self.get_selected_pbi() {
                    self.active_view = ActiveView::Detail(Box::new(PbiDetailView::new(pbi)));
                }
            }
            JiraCommand::Quit => {
                self.exit = true;
            }
            JiraCommand::Refresh | JiraCommand::RefreshAll => {
                let filter = self.filter.clone();
                self.start_fetch(filter);
            }
            JiraCommand::OpenInBrowser => {
                self.open_selected_in_browser();
            }
            JiraCommand::OpenPluginList => {
                // Not implemented for pbi_list
            }
            JiraCommand::OpenRawPbiJson => {
                if let Some(pbi) = self.get_selected_pbi() {
                    self.selected_pbi = Some(pbi);
                }
            }
            JiraCommand::OpenFilter => {
                let editor = FilterEditor::new(self.filter.clone());
                self.active_view = ActiveView::FilterEditor(Box::new(editor));
            }
            _ => {}
        }
    }

    fn get_selected_pbi(&self) -> Option<Pbi> {
        self.table
            .table_state
            .selected()
            .and_then(|i| self.issues.get(i).cloned())
    }

    fn open_selected_in_browser(&mut self) {
        use crate::config::JiraConfig;

        if let Some(pbi) = self.get_selected_pbi() {
            let config = JiraConfig::load().unwrap_or_default();
            let url = format!("{}/browse/{}", config.namespace, pbi.key);

            #[cfg(target_os = "macos")]
            let result = Command::new("open").arg(&url).status();
            #[cfg(target_os = "linux")]
            let result = Command::new("xdg-open").arg(&url).status();
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            let result: std::result::Result<std::process::ExitStatus, std::io::Error> = Err(
                std::io::Error::new(std::io::ErrorKind::Unsupported, "unsupported platform"),
            );

            match result {
                Ok(_) => self.footer.set_status(format!("Opened {url}")),
                Err(e) => self
                    .footer
                    .set_status(format!("Failed to open browser: {e}")),
            }
        }
    }

    // TODO: duplicated with sprint_list, maybe move to util?
    fn open_raw_in_editor(&self, pbi: &Pbi) {
        let json = pbi.raw.clone();
        let key = pbi.key.as_str();
        let tmp_path = env::temp_dir().join(format!("jira_raw_{key}.json"));
        if let Err(e) = fs::write(&tmp_path, &json) {
            eprintln!("Failed to write temp file: {e}");
            return;
        }

        let editor = env::var("VISUAL")
            .or_else(|_| env::var("EDITOR"))
            .unwrap_or_else(|_| "vi".to_string());

        let _ = Command::new(&editor)
            .arg(&tmp_path)
            .status()
            .map_err(|e| eprintln!("Failed to open editor '{editor}': {e}"));

        let _ = fs::remove_file(&tmp_path);
    }

    // ── Background fetch ──────────────────────────────────────────────────────

    fn start_fetch(&mut self, filter: ListFilter) {
        let svc = Arc::clone(&self.list_service);
        let (tx, rx) = mpsc::channel();
        self.bg_rx = Some(rx);
        thread::spawn(move || {
            let msg = match svc.fetch_with_filter(&filter) {
                Ok(issues) => BgMsg::Loaded(issues),
                Err(e) => BgMsg::Error(e),
            };
            let _ = tx.send(msg);
        });
        self.footer.set_status("Fetching from Jira…");
    }

    fn process_bg_messages(&mut self) {
        let msg = {
            let Some(rx) = self.bg_rx.as_ref() else {
                return;
            };
            match rx.try_recv() {
                Ok(m) => m,
                Err(_) => return,
            }
        };
        self.bg_rx = None;
        match msg {
            BgMsg::Loaded(issues) => {
                let count = issues.len();
                self.issues = issues;
                self.table.reset_selection(count);
                self.footer.set_status(format!("{count} issues loaded"));
            }
            BgMsg::Error(e) => {
                self.footer.set_status(format!("Error: {e}"));
            }
        }
    }

    // ── Layout & draw ─────────────────────────────────────────────────────────

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        match &mut self.active_view {
            ActiveView::Detail(detail) => {
                detail.render(frame, area);
            }
            _ => {
                // Shared base: title bar, active filters, table, footer
                let layout = Layout::vertical([
                    Constraint::Length(1), // title bar
                    Constraint::Length(1), // active filters
                    Constraint::Min(0),    // table
                    Constraint::Length(1), // footer
                ])
                .split(area);

                self.render_title(frame, layout[0]);
                self.render_active_filters(frame, layout[1]);
                self.table.render(frame, layout[2], &self.issues);
                self.footer.render(frame, layout[3]);

                // Filter editor overlaid on top
                if let ActiveView::FilterEditor(editor) = &mut self.active_view {
                    editor.render(frame, area);
                }
            }
        }
    }

    fn render_title(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        frame.render_widget(
            Line::from(vec![
                Span::styled(
                    " JIRA Issues",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  ·  {} results", self.issues.len()),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            area,
        );
    }

    fn render_active_filters(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let jql = self.filter.to_jql();
        let content = if jql.is_empty() {
            Span::styled("  No filters active", Style::default().fg(Color::DarkGray))
        } else {
            Span::styled(format!("  Filter: {jql}"), Style::default().fg(Color::Cyan))
        };
        frame.render_widget(Line::from(vec![content]), area);
    }

    // ── Event handling ────────────────────────────────────────────────────────

    fn handle_events(&mut self) -> Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match &mut self.active_view {
                    ActiveView::Detail(_) => self.handle_detail_key(key.code),
                    ActiveView::FilterEditor(_) => self.handle_filter_editor_key(key.code),
                    ActiveView::List => self.handle_list_key(key.code),
                }
            }
        }
        Ok(())
    }

    fn handle_list_key(&mut self, key: crossterm::event::KeyCode) {
        self.table.handle_key(key);
    }

    fn handle_detail_key(&mut self, key: KeyCode) {
        let ActiveView::Detail(ref mut detail) = self.active_view else {
            return;
        };
        detail.handle_key(key);
    }

    fn handle_filter_editor_key(&mut self, key: KeyCode) {
        let ActiveView::FilterEditor(ref mut editor) = self.active_view else {
            return;
        };
        match editor.handle_key(key) {
            Some(FilterEditorAction::Apply(new_filter)) => {
                self.filter = *new_filter.clone();
                self.active_view = ActiveView::List;
                self.start_fetch(*new_filter);
            }
            Some(FilterEditorAction::Close) => {
                self.active_view = ActiveView::List;
            }
            None => {}
        }
    }
}

// ── Non-TUI display (--json path) ─────────────────────────────────────────────

/// Column definition used by the tabular and JSON non-TUI output.
struct ColumnDef {
    title: &'static str,
    width: usize,
}

fn column_def(name: &str) -> Option<ColumnDef> {
    match name {
        "key" => Some(ColumnDef {
            title: "Key",
            width: 10,
        }),
        "resolution" => Some(ColumnDef {
            title: "Resolution",
            width: 10,
        }),
        "priority" => Some(ColumnDef {
            title: "Priority",
            width: 10,
        }),
        "assignee" => Some(ColumnDef {
            title: "Assignee",
            width: 20,
        }),
        "status" => Some(ColumnDef {
            title: "Status",
            width: 15,
        }),
        "components" => Some(ColumnDef {
            title: "Components",
            width: 30,
        }),
        "creator" => Some(ColumnDef {
            title: "Creator",
            width: 15,
        }),
        "reporter" => Some(ColumnDef {
            title: "Reporter",
            width: 15,
        }),
        "issuetype" => Some(ColumnDef {
            title: "Issue Type",
            width: 10,
        }),
        "project" => Some(ColumnDef {
            title: "Project",
            width: 15,
        }),
        "summary" => Some(ColumnDef {
            title: "Summary",
            width: 100,
        }),
        _ => None,
    }
}

fn field_value(pbi: &Pbi, column: &str) -> String {
    match column {
        "key" => pbi.key.clone(),
        "summary" => pbi.summary.clone(),
        "status" => pbi.status.clone(),
        "assignee" => pbi.assignee.clone(),
        "resolution" => pbi.resolution.clone().unwrap_or_else(|| "-".to_string()),
        "priority" => pbi.priority.clone().unwrap_or_else(|| "-".to_string()),
        "components" => pbi.components.join(", "),
        "creator" => pbi.creator.clone(),
        "reporter" => pbi.reporter.clone(),
        "issuetype" => pbi.issue_type.clone(),
        "project" => pbi.project.clone(),
        _ => "-".to_string(),
    }
}

pub fn display_issues(issues: &[Pbi], columns: &[&str], show_json: bool) {
    if issues.is_empty() {
        println!("No issues found for the filter.");
        return;
    }
    if show_json {
        display_json(issues, columns);
    } else {
        display_table(issues, columns);
    }
}

fn display_json(issues: &[Pbi], columns: &[&str]) {
    let mut response = json::JsonValue::new_array();
    for pbi in issues {
        let mut data = json::JsonValue::new_object();
        for &col in columns {
            if col == "components" {
                let mut arr = json::JsonValue::new_array();
                for c in &pbi.components {
                    let _ = arr.push(c.as_str());
                }
                data[col] = arr;
            } else {
                data[col] = field_value(pbi, col).into();
            }
        }
        let _ = response.push(data);
    }
    println!("{}", response.pretty(4));
}

fn display_table(issues: &[Pbi], columns: &[&str]) {
    let defs: Vec<(&str, ColumnDef)> = columns
        .iter()
        .map(|&col| {
            let def = column_def(col).unwrap_or_else(|| {
                eprintln!("Unknown display option '{col}' passed.");
                std::process::exit(1);
            });
            (col, def)
        })
        .collect();

    let mut total_width = 0;
    for (_, def) in &defs {
        print!("{title:width$}|", title = def.title, width = def.width);
        total_width += def.width + 1;
    }
    println!();
    println!("{:->width$}", "", width = total_width);

    for pbi in issues {
        for (col, def) in &defs {
            let mut value = field_value(pbi, col);
            value.truncate(def.width);
            print!("{value:width$}|", width = def.width);
        }
        println!();
    }
}
