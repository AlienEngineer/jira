use crate::jira::api;

/// Fetch the raw Jira API response for `ticket` and print it to stdout.
pub fn print_raw(ticket: String, pretty: bool) {
    match api::get_call_v2(format!("issue/{ticket}?expand=changelog,renderedFields")) {
        Ok(value) => {
            if pretty {
                println!("{}", json::stringify_pretty(value, 2));
            } else {
                println!("{}", json::stringify(value));
            }
        }
        Err(e) => {
            eprintln!("Error fetching {ticket}: {e}");
            std::process::exit(1);
        }
    }
}
