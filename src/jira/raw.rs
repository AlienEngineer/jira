use std::error::Error;
use std::sync::Arc;

use crate::ioc::interface::Interface;
use crate::jira::api::JiraApi;

pub trait RawService: Interface {
    fn print_raw(&self, ticket: String, pretty: bool);
    fn fetch_raw_issue(&self, key: &str) -> Result<json::JsonValue, Box<dyn Error>>;
}

pub struct DefaultRawService {
    jira_api: Arc<dyn JiraApi>,
}

impl DefaultRawService {
    pub fn new(jira_api: Arc<dyn JiraApi>) -> Self {
        Self { jira_api }
    }
}

impl RawService for DefaultRawService {
    fn print_raw(&self, ticket: String, pretty: bool) {
        match self.fetch_raw_issue(&ticket) {
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

    fn fetch_raw_issue(&self, key: &str) -> Result<json::JsonValue, Box<dyn Error>> {
        self.jira_api
            .get_v2(&format!("issue/{key}?expand=changelog,renderedFields"))
    }
}
