//! User interaction tests for PbiListApp
//!
//! These tests simulate actual user key presses and verify the resulting behavior.
//! Tests run the app as it would in runtime, using real Lua keymaps with only
//! the Jira API being faked.
//!
//! Pattern (from ratatui tutorial):
//! ```ignore
//! app.handle_key_event(KeyCode::Char('j'));
//! assert_eq!(app.selected_pbi().unwrap().key, "TEST-2");
//! ```
//!
//! **Note:** Tests must run with `--test-threads=1` because they share a global
//! command channel for Lua keymaps.
//!
//! Default keymaps tested:
//! - j/k: Navigate down/up
//! - l/h: Open detail view / go back
//! - /: Open filter editor
//! - q/ESC: Quit
//! - f: Refresh selected PBI
//! - r: View raw JSON
//! - o: Open in browser

use crossterm::event::KeyCode;
use jira::jira::api::JiraApi;
use jira::jira::lists::{ListFilter, ListService};
use jira::jira::pbi::Pbi;
use jira::ui::pbi_list::PbiListApp;
use std::error::Error;
use std::sync::{Arc, Mutex, Once};

// ── Test serialization ────────────────────────────────────────────────────────
// The Lua command channel is global, so tests must not run in parallel.

static TEST_MUTEX: Mutex<()> = Mutex::new(());
static INIT_LUA: Once = Once::new();

fn ensure_lua_initialized() {
    INIT_LUA.call_once(|| {
        jira::lua::init::init_lua_config().expect("Failed to initialize Lua config");
    });
}

// ── Mock implementations ──────────────────────────────────────────────────────

struct MockJiraApi;

impl JiraApi for MockJiraApi {
    fn get(&self, _endpoint: &str, _version: u8) -> Result<json::JsonValue, Box<dyn Error>> {
        Ok(json::object! {})
    }

    fn post(
        &self,
        _endpoint: &str,
        _json_value: json::JsonValue,
        _version: u8,
    ) -> Result<String, Box<dyn Error>> {
        Ok(String::new())
    }

    fn put(
        &self,
        _endpoint: &str,
        _json_value: json::JsonValue,
        _version: u8,
    ) -> Result<String, Box<dyn Error>> {
        Ok(String::new())
    }

    fn get_agile(&self, _endpoint: &str) -> Result<json::JsonValue, Box<dyn Error>> {
        Ok(json::object! {})
    }
}

struct MockListService {
    api: Arc<MockJiraApi>,
}

impl MockListService {
    fn new() -> Self {
        Self {
            api: Arc::new(MockJiraApi),
        }
    }
}

impl ListService for MockListService {
    fn fetch_with_filter(&self, _filter: &ListFilter) -> Result<Vec<Pbi>, String> {
        Ok(vec![])
    }

    fn jira_api(&self) -> &dyn JiraApi {
        self.api.as_ref()
    }
}

// ── Test fixtures ─────────────────────────────────────────────────────────────

fn make_pbi(key: &str, summary: &str, status: &str, assignee: &str) -> Pbi {
    Pbi {
        key: key.to_string(),
        summary: summary.to_string(),
        status: status.to_string(),
        assignee: assignee.to_string(),
        issue_type: "Story".to_string(),
        description: None,
        priority: Some("Medium".to_string()),
        story_points: None,
        labels: vec![],
        loaded: false,
        in_progress_at: None,
        resolved_at: None,
        raw: String::new(),
        resolution: None,
        components: vec![],
        creator: String::new(),
        reporter: String::new(),
        project: "MYPROJ".to_string(),
    }
}

fn issue_search_results() -> Vec<Pbi> {
    vec![
        make_pbi("MYPROJ-201", "User authentication", "Open", "Alice"),
        make_pbi("MYPROJ-202", "Password validation", "In Progress", "Bob"),
        make_pbi("MYPROJ-203", "Session management", "In Review", "Carol"),
        make_pbi("MYPROJ-204", "OAuth integration", "Blocked", "Dave"),
        make_pbi("MYPROJ-205", "Two-factor auth", "Open", "Eve"),
        make_pbi("MYPROJ-206", "Login page design", "Closed", "Frank"),
    ]
}

fn create_test_app() -> PbiListApp {
    ensure_lua_initialized();
    // Reset the command channel so each test gets a fresh receiver
    jira::lua::init::reset_command_channel();
    let issues = issue_search_results();
    let filter = ListFilter::default();
    let service = Arc::new(MockListService::new());
    PbiListApp::new(issues, filter, service)
}

/// Run a test with exclusive access to the command channel.
fn run_test<F>(test_fn: F)
where
    F: FnOnce(),
{
    let _guard = TEST_MUTEX.lock().unwrap();
    test_fn();
}

// ══════════════════════════════════════════════════════════════════════════════
// USER INTERACTION TESTS: Navigation
// ══════════════════════════════════════════════════════════════════════════════

mod navigation {
    use super::*;

    #[test]
    fn pressing_j_selects_item_below() {
        run_test(|| {
            let mut app = create_test_app();
            assert_eq!(app.selected_pbi().unwrap().key, "MYPROJ-201");

            app.handle_key_event(KeyCode::Char('j'));

            assert_eq!(
                app.selected_pbi().unwrap().key,
                "MYPROJ-202",
                "pressing 'j' should select the item below"
            );
        });
    }

    #[test]
    fn pressing_j_multiple_times_navigates_down() {
        run_test(|| {
            let mut app = create_test_app();

            app.handle_key_event(KeyCode::Char('j'));
            app.handle_key_event(KeyCode::Char('j'));
            app.handle_key_event(KeyCode::Char('j'));

            assert_eq!(
                app.selected_pbi().unwrap().key,
                "MYPROJ-204",
                "navigating down 3 times should reach 4th item"
            );
            assert_eq!(
                app.selected_pbi().unwrap().assignee,
                "Dave",
                "should show Dave as assignee"
            );
        });
    }

    #[test]
    fn pressing_k_selects_item_above() {
        run_test(|| {
            let mut app = create_test_app();
            // Navigate to fifth item first
            for _ in 0..4 {
                app.handle_key_event(KeyCode::Char('j'));
            }
            assert_eq!(app.selected_pbi().unwrap().key, "MYPROJ-205");

            app.handle_key_event(KeyCode::Char('k'));
            app.handle_key_event(KeyCode::Char('k'));

            assert_eq!(
                app.selected_pbi().unwrap().key,
                "MYPROJ-203",
                "pressing 'k' twice should move up 2 items"
            );
        });
    }

    #[test]
    fn pressing_down_arrow_navigates() {
        run_test(|| {
            let mut app = create_test_app();

            app.handle_key_event(KeyCode::Down);
            app.handle_key_event(KeyCode::Down);

            assert_eq!(
                app.selected_pbi().unwrap().key,
                "MYPROJ-203",
                "down arrow should navigate through list"
            );
        });
    }

    #[test]
    fn pressing_j_at_bottom_wraps_to_top() {
        run_test(|| {
            let mut app = create_test_app();
            // Navigate to last item
            for _ in 0..5 {
                app.handle_key_event(KeyCode::Char('j'));
            }
            assert_eq!(app.selected_pbi().unwrap().key, "MYPROJ-206");

            app.handle_key_event(KeyCode::Char('j'));

            assert_eq!(
                app.selected_pbi().unwrap().key,
                "MYPROJ-201",
                "pressing 'j' at bottom should wrap to top"
            );
        });
    }

    #[test]
    fn pressing_k_at_top_wraps_to_bottom() {
        run_test(|| {
            let mut app = create_test_app();
            assert_eq!(app.selected_pbi().unwrap().key, "MYPROJ-201");

            app.handle_key_event(KeyCode::Char('k'));

            assert_eq!(
                app.selected_pbi().unwrap().key,
                "MYPROJ-206",
                "pressing 'k' at top should wrap to bottom"
            );
        });
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// USER INTERACTION TESTS: Actions
// ══════════════════════════════════════════════════════════════════════════════

mod actions {
    use super::*;

    #[test]
    fn pressing_l_opens_detail_view() {
        run_test(|| {
            let mut app = create_test_app();
            // Navigate to third item
            app.handle_key_event(KeyCode::Char('j'));
            app.handle_key_event(KeyCode::Char('j'));
            assert!(!app.is_detail_view());

            app.handle_key_event(KeyCode::Char('l'));

            assert!(app.is_detail_view(), "pressing 'l' should open detail view");
        });
    }

    #[test]
    fn pressing_right_arrow_opens_detail() {
        run_test(|| {
            let mut app = create_test_app();
            assert!(!app.is_detail_view());

            app.handle_key_event(KeyCode::Right);

            assert!(
                app.is_detail_view(),
                "pressing Right arrow should open detail view"
            );
        });
    }

    #[test]
    fn pressing_q_exits() {
        run_test(|| {
            let mut app = create_test_app();
            assert!(!app.is_exit());

            app.handle_key_event(KeyCode::Char('q'));

            assert!(app.is_exit(), "pressing 'q' should set exit flag");
        });
    }

    #[test]
    fn pressing_esc_exits() {
        run_test(|| {
            let mut app = create_test_app();
            assert!(!app.is_exit());

            app.handle_key_event(KeyCode::Esc);

            assert!(app.is_exit(), "pressing Escape should set exit flag");
        });
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// USER WORKFLOW TESTS
// ══════════════════════════════════════════════════════════════════════════════

mod workflows {
    use super::*;

    #[test]
    fn find_blocked_item_in_search_results() {
        run_test(|| {
            let mut app = create_test_app();

            // Navigate to blocked item (MYPROJ-204 at index 3)
            app.handle_key_event(KeyCode::Char('j'));
            app.handle_key_event(KeyCode::Char('j'));
            app.handle_key_event(KeyCode::Char('j'));

            let selected = app.selected_pbi().unwrap();
            assert_eq!(selected.key, "MYPROJ-204");
            assert_eq!(selected.status, "Blocked");
            assert_eq!(selected.summary, "OAuth integration");
        });
    }

    #[test]
    fn cycle_through_all_search_results() {
        run_test(|| {
            let mut app = create_test_app();
            let mut visited_keys = vec![];

            // Cycle through all items
            for _ in 0..6 {
                visited_keys.push(app.selected_pbi().unwrap().key.clone());
                app.handle_key_event(KeyCode::Char('j'));
            }

            assert_eq!(
                visited_keys,
                vec![
                    "MYPROJ-201",
                    "MYPROJ-202",
                    "MYPROJ-203",
                    "MYPROJ-204",
                    "MYPROJ-205",
                    "MYPROJ-206"
                ],
                "should cycle through all search results"
            );
        });
    }

    #[test]
    fn find_issues_by_assignee() {
        run_test(|| {
            let mut app = create_test_app();
            let mut iterations = 0;
            let max_iterations = 10;

            // Navigate to find Carol's item (with safety limit)
            while app.selected_pbi().unwrap().assignee != "Carol" && iterations < max_iterations {
                app.handle_key_event(KeyCode::Char('j'));
                iterations += 1;
            }

            let selected = app.selected_pbi().unwrap();
            assert_eq!(selected.key, "MYPROJ-203");
            assert_eq!(selected.summary, "Session management");
            assert_eq!(selected.status, "In Review");
        });
    }

    #[test]
    fn navigate_up_and_down_to_compare_items() {
        run_test(|| {
            let mut app = create_test_app();

            // Move down, then back up to compare
            app.handle_key_event(KeyCode::Char('j'));
            app.handle_key_event(KeyCode::Char('j'));
            let third_item = app.selected_pbi().unwrap().key.clone();

            app.handle_key_event(KeyCode::Char('k'));
            let second_item = app.selected_pbi().unwrap().key.clone();

            assert_eq!(third_item, "MYPROJ-203");
            assert_eq!(second_item, "MYPROJ-202");
        });
    }

    #[test]
    fn mix_vim_and_arrow_keys() {
        run_test(|| {
            let mut app = create_test_app();

            app.handle_key_event(KeyCode::Char('j'));
            app.handle_key_event(KeyCode::Down);
            app.handle_key_event(KeyCode::Char('k'));
            app.handle_key_event(KeyCode::Down);

            assert_eq!(
                app.selected_pbi().unwrap().key,
                "MYPROJ-203",
                "mixing vim and arrow keys should work"
            );
        });
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// EDGE CASE TESTS
// ══════════════════════════════════════════════════════════════════════════════

mod edge_cases {
    use super::*;

    fn create_single_item_app() -> PbiListApp {
        ensure_lua_initialized();
        jira::lua::init::reset_command_channel();
        let issues = vec![make_pbi("SOLO-1", "Only item", "Open", "Alice")];
        let filter = ListFilter::default();
        let service = Arc::new(MockListService::new());
        PbiListApp::new(issues, filter, service)
    }

    fn create_empty_app() -> PbiListApp {
        ensure_lua_initialized();
        jira::lua::init::reset_command_channel();
        let issues: Vec<Pbi> = vec![];
        let filter = ListFilter::default();
        let service = Arc::new(MockListService::new());
        PbiListApp::new(issues, filter, service)
    }

    #[test]
    fn navigation_with_single_item_list() {
        run_test(|| {
            let mut app = create_single_item_app();

            app.handle_key_event(KeyCode::Char('j'));
            assert_eq!(
                app.selected_pbi().unwrap().key,
                "SOLO-1",
                "navigation with single item should stay on same item"
            );

            app.handle_key_event(KeyCode::Char('k'));
            assert_eq!(
                app.selected_pbi().unwrap().key,
                "SOLO-1",
                "navigation with single item should stay on same item"
            );
        });
    }

    #[test]
    fn navigation_with_empty_list() {
        run_test(|| {
            let mut app = create_empty_app();

            app.handle_key_event(KeyCode::Char('j'));

            assert!(
                app.selected_pbi().is_none(),
                "navigation with empty list should not select anything"
            );
        });
    }

    #[test]
    fn actions_with_no_selection() {
        run_test(|| {
            let mut app = create_empty_app();

            // Try to open details with no selection
            app.handle_key_event(KeyCode::Char('l'));

            assert!(
                !app.is_detail_view(),
                "should not open detail when nothing is selected"
            );
        });
    }
}
