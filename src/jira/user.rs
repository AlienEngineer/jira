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

    fn get_user_id_by(&self, field: &str) -> Option<String> {
        let f = field.to_string();
        let jql = f.to_string() + "+in+(currentUser())";
        let response = self
            .jira_api
            .get_v3(&format!("search?maxResults=1&jql={jql}"))
            .ok()?;

        let fields = &response["issues"][0]["fields"];
        let user = &fields[f];
        let id = user["key"].as_str();
        if let Some(id) = id {
            return Some(id.to_string());
        }

        None
    }
}

impl CurrentUserService for DefaultCurrentUserService {
    fn fetch_current_account_id(&self) -> Option<String> {
        self.get_user_id_by("assignee")
            .or_else(|| self.get_user_id_by("reporter"))
            .or_else(|| self.get_user_id_by("creator"))
    }
}
