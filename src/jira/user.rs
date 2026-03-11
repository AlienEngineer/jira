use crate::jira::api;

/// Fetch the account ID of the currently authenticated user.
///
/// Runs a single JQL query combining assignee, reporter, and creator with OR
/// clauses, limiting to 1 result. Extracts the user key from `["key"]`
/// (Jira Server/Data Center) or `["accountId"]` (Jira Cloud).
///
/// Returns `None` if the account ID cannot be determined.
pub fn fetch_current_account_id() -> Option<String> {
    let jql = "assignee+in+(currentUser())+OR+reporter+in+(currentUser())+OR+creator+in+(currentUser())";
    let response = api::get_call_v3(format!("search?maxResults=1&jql={jql}")).ok()?;

    let fields = &response["issues"][0]["fields"];
    for field in &["assignee", "reporter", "creator"] {
        let user = &fields[*field];
        let id = user["key"].as_str().or_else(|| user["accountId"].as_str());
        if let Some(id) = id {
            return Some(id.to_string());
        }
    }

    None
}
