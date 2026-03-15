use std::sync::Arc;

use crate::{ioc::interface::Interface, jira::api::JiraApi};

pub trait CurrentUserService: Interface {
    fn fetch_current_account_id(&self) -> Option<String>;
}

pub struct DefaultCurrentUserService {
    jira_api: Arc<dyn JiraApi>,
}

impl DefaultCurrentUserService {
    pub fn new(jira_api: Arc<dyn JiraApi>) -> Self {
        Self { jira_api }
    }
}

impl CurrentUserService for DefaultCurrentUserService {
    fn fetch_current_account_id(&self) -> Option<String> {
        let jql = "assignee+in+(currentUser())+OR+reporter+in+(currentUser())+OR+creator+in+(currentUser())";
        let response = self
            .jira_api
            .get_v3(&format!("search?maxResults=1&jql={jql}"))
            .ok()?;

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
}
