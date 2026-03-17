use std::sync::Arc;

use crate::config;
use crate::ioc::interface::Interface;
use crate::jira::api::JiraApi;
use crate::jira::utils::MetadataService;

pub trait AssignService: Interface {
    fn assign_task(&self, ticket: String, user: String, silent: bool);
}

pub struct DefaultAssignService {
    jira_api: Arc<dyn JiraApi>,
    metadata_service: Arc<dyn MetadataService>,
}

impl DefaultAssignService {
    pub fn new(jira_api: Arc<dyn JiraApi>, metadata_service: Arc<dyn MetadataService>) -> Self {
        Self {
            jira_api,
            metadata_service,
        }
    }
}

impl AssignService for DefaultAssignService {
    fn assign_task(&self, ticket: String, user: String, silent: bool) {
        let aliased_query = config::get_alias_or(user);
        let account_id = self.metadata_service.get_account_id(aliased_query);
        let payload = json::object! {
            "accountId": account_id
        };
        let update_response = self.jira_api.put(
            &format!("issue/{ticket}/assignee"),
            payload,
            config::get_version().parse::<u8>().unwrap_or(3),
        );
        if update_response.is_err() {
            if !silent {
                eprintln!("Error occurred While assigning the ticket.");
            }
            std::process::exit(1);
        }
        let response = update_response.unwrap();
        if !silent {
            println!("Successfully Assigned {response}");
        }
    }
}
