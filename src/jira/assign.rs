use crate::config;
use crate::ioc::interface::Interface;
use crate::jira::api::JiraApi;
use crate::prelude::Result;
use std::sync::Arc;

pub trait AssignService: Interface {
    fn assign_ticket_to_account(&self, ticket_id: String, account_id: String) -> Result<()>;
    fn assign_task(&self, ticket: String, account_id: String, silent: bool);
}

pub struct DefaultAssignService {
    jira_api: Arc<dyn JiraApi>,
}

impl DefaultAssignService {
    pub fn new(jira_api: Arc<dyn JiraApi>) -> Self {
        Self { jira_api }
    }
}

impl AssignService for DefaultAssignService {
    fn assign_task(&self, ticket: String, account_id: String, silent: bool) {
        let update_response = self.assign_ticket_to_account(ticket.clone(), account_id);
        if update_response.is_err() {
            if !silent {
                eprintln!("Error occurred While assigning the ticket.");
            }
            std::process::exit(1);
        }
        if !silent {
            println!("Successfully Assigned");
        }
    }

    fn assign_ticket_to_account(&self, ticket_id: String, account_id: String) -> Result<()> {
        self.jira_api
            .put(
                &format!("issue/{ticket_id}/assignee"),
                json::object! {
                    "name": account_id
                },
                config::get_version().parse::<u8>().unwrap_or(3),
            )
            .map(|_| ())
    }
}
