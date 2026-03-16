pub mod api;
pub mod assign;
pub mod comments;
pub mod details;
pub mod fields;
pub mod lists;
pub mod logout;
pub mod new_issue;
pub mod pbi;
pub mod raw;
pub mod sprint;
pub mod transitions;
pub mod update;
pub mod user;
pub mod utils;

extern crate clap;
use clap::ArgMatches;
use std::sync::Arc;

fn service<T>() -> Arc<T>
where
    T: ?Sized + crate::ioc::interface::Interface + 'static,
{
    crate::ioc::global()
        .get::<T>()
        .expect("service not registered in IoC container")
}

pub fn handle_transition_matches(matches: &ArgMatches) {
    let ticket = matches.value_of("transition_ticket").unwrap();
    let transition_service = service::<dyn transitions::TransitionService>();
    if matches.is_present("transition_list") {
        transition_service.print_transition_lists(ticket.to_string());
    } else {
        let status = matches.value_of("STATUS").unwrap();
        transition_service.move_ticket_status(ticket.to_string(), status.to_string());
    }
}

pub fn handle_logout(_matches: &ArgMatches) {
    service::<dyn logout::LogoutService>().delete_configuration();
}

pub fn handle_fields_matches(matches: &ArgMatches) {
    let ticket = String::from(matches.value_of("TICKET").unwrap());
    service::<dyn fields::FieldsService>().display_all_fields(ticket);
}

pub fn handle_list_matches(matches: &ArgMatches) {
    use crate::ui::pbi_list;

    // --json: skip the TUI and print structured output
    if matches.is_present("json") {
        let issues = service::<dyn lists::ListService>().list_issues(matches);
        let display = matches
            .value_of("display")
            .unwrap_or("key,summary,status,assignee");
        let columns: Vec<&str> = display.trim().split(',').collect();
        pbi_list::display_issues(&issues, &columns, true);
        return;
    }

    let filter = lists::ListFilter::from_matches(matches);

    if matches.is_present("alias") {
        let alias_name = matches.value_of("alias").unwrap();
        crate::config::set_alias(alias_name.to_string(), filter.to_jql());
        println!("Current filter is now set with value {alias_name}");
        println!("You can use jira list --jql \"{alias_name}\" to reuse this filter.");
    }

    let list_service = service::<dyn lists::ListService>();
    let issues = list_service.fetch_with_filter(&filter).unwrap_or_else(|e| {
        eprintln!("Error fetching issues: {e}");
        std::process::exit(1);
    });

    let mut terminal = ratatui::init();
    let result = pbi_list::PbiListApp::new(issues, filter, list_service).run(&mut terminal);
    ratatui::restore();
    if let Err(e) = result {
        eprintln!("TUI error: {e}");
        std::process::exit(1);
    }
}

pub fn handle_detail_matches(matches: &ArgMatches) {
    let ticket = String::from(matches.value_of("TICKET").unwrap());
    let fields = String::from(
        matches
            .value_of("fields")
            .unwrap_or("key,summary,description"),
    );
    service::<dyn details::DetailService>().show_details(ticket, fields);
}

pub fn handle_update_matches(matches: &ArgMatches) {
    let ticket = String::from(matches.value_of("TICKET").unwrap());
    let field = String::from(matches.value_of("field").unwrap());
    let value = String::from(matches.value_of("value").unwrap());
    service::<dyn update::UpdateService>().update_jira_ticket(ticket, field, value);
}

pub fn handle_new_matches(matches: &ArgMatches) {
    service::<dyn new_issue::IssueCreationService>().handle_issue_creation(matches);
}

pub fn handle_assign_matches(matches: &ArgMatches) {
    let ticket = String::from(matches.value_of("ticket").unwrap());
    let user = String::from(matches.value_of("user").unwrap());
    service::<dyn assign::AssignService>().assign_task(ticket, user);
}

pub fn handle_comments_matches(matches: &ArgMatches) {
    let ticket = String::from(matches.value_of("ticket").unwrap());
    let comments_service = service::<dyn comments::CommentsService>();
    if matches.is_present("list") {
        comments_service.get_all_comments(ticket);
        return;
    }
    comments_service.add_new_comment(ticket, matches);
}

pub fn handle_sprint(_matches: &ArgMatches) {
    use crate::config;
    use crate::ui::sprint_list::SprintApp;

    let board_id = config::get_config("board-id".to_string());
    if board_id.is_empty() {
        eprintln!(
            "No board_id found in configuration.\n\
             Please set it first with:\n\
             \n\
             jira config board-id <YOUR_BOARD_ID>"
        );
        std::process::exit(1);
    }

    config::ensure_account_id();
    let sprint_service = service::<dyn sprint::SprintService>();

    // Try the on-disk cache first; fall back to a fresh API fetch
    let sprint = if let Some(cached) = sprint::load_sprint_cache(&board_id) {
        println!("Loaded sprint from cache. Press F to refresh all items.");
        cached
    } else {
        println!("Fetching active sprint for board {board_id}...");
        match sprint_service.fetch_active_sprint_issues(&board_id) {
            Err(e) => {
                eprintln!("Error fetching sprint: {e}");
                std::process::exit(1);
            }
            Ok(sprint) => {
                sprint::save_sprint_cache(&sprint);
                sprint
            }
        }
    };

    let mut terminal = ratatui::init();
    let result = SprintApp::new(sprint, sprint_service).run(&mut terminal);
    ratatui::restore();
    if let Err(e) = result {
        eprintln!("TUI error: {e}");
        std::process::exit(1);
    }
}

pub fn handle_raw(matches: &ArgMatches) {
    let ticket = String::from(matches.value_of("TICKET").unwrap());
    let pretty = matches.is_present("pretty");
    service::<dyn raw::RawService>().print_raw(ticket, pretty);
}
