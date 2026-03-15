use std::sync::Arc;

use crate::{ioc::interface::Interface, jira::api::JiraApi};

pub trait MetadataService: Interface {
    fn get_account_id(&self, query: String) -> String;
    fn get_issuetype_id(&self, project: String, entry: Option<String>) -> Option<String>;
}

pub struct DefaultMetadataService {
    jira_api: Arc<dyn JiraApi>,
}

impl DefaultMetadataService {
    pub fn new(jira_api: Arc<dyn JiraApi>) -> Self {
        Self { jira_api }
    }
}

impl MetadataService for DefaultMetadataService {
    fn get_account_id(&self, query: String) -> String {
        let url = format!("user/search?query={query}");
        let api_response = self.jira_api.get_v3(&url);
        if api_response.is_err() {
            eprintln!("Cannot search for provided assignee user. {api_response:?}");
            return String::new();
        }
        let account_response = &api_response.unwrap()[0];
        if account_response.is_null() {
            eprintln!("Cannot search for provided assignee user. ");
            return String::new();
        }
        println!("Selecting user {}", account_response["displayName"]);
        String::from(account_response["accountId"].as_str().unwrap())
    }

    fn get_issuetype_id(&self, project: String, entry: Option<String>) -> Option<String> {
        let name = entry.as_ref()?;
        let url = format!("issue/createmeta?projectKeys={project}");
        let api_response = self.jira_api.get_v3(&url);
        if api_response.is_err() {
            eprintln!("Error while verifying issue type: {api_response:?}");
            return None;
        }
        let project_list = &api_response.unwrap()["projects"];
        for project in project_list.members() {
            let issuetypes = &project["issuetypes"];
            for issuetype in issuetypes.members() {
                if issuetype["name"].as_str().unwrap_or("").to_lowercase() == name.to_lowercase() {
                    return Some(issuetype["id"].as_str().unwrap_or("").to_string());
                }
            }
        }
        None
    }
}
