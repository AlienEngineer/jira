use std::sync::Arc;

use crate::config;
use crate::ioc::interface::Interface;
use crate::jira::api::JiraApi;

pub trait TransitionService: Interface {
    fn print_transition_lists(&self, ticket: String);
    fn move_ticket_status(&self, ticket: String, status: String);
}

pub struct DefaultTransitionService {
    jira_api: Arc<dyn JiraApi>,
}

impl DefaultTransitionService {
    pub fn new(jira_api: Arc<dyn JiraApi>) -> Self {
        Self { jira_api }
    }

    fn get_project_code(&self, ticket: &str) -> String {
        String::from(ticket.split('-').next().unwrap())
    }

    fn get_transitions(&self, ticket: &str) -> Option<json::JsonValue> {
        let transitions_response = self.jira_api.get_v3(&format!("issue/{ticket}/transitions"));
        if transitions_response.is_err() {
            return None;
        }
        let transitions = &transitions_response.unwrap()["transitions"];
        if !transitions.is_array() {
            return None;
        }
        let mut transition_object = json::object! {};
        for transition in transitions.members() {
            let name = String::from(transition["name"].as_str().unwrap()).to_lowercase();
            let id: u16 = transition["id"].as_str().unwrap().parse().unwrap();
            transition_object[name] = id.into();
        }
        let project_code = self.get_project_code(ticket);
        config::set_transitions(project_code, transition_object);
        Some(transitions.clone())
    }

    fn get_transition_code(&self, ticket: &str, transition_name: &str) -> Option<u16> {
        let project_code = self.get_project_code(ticket);
        let aliased_name = config::get_alias_or(transition_name.to_lowercase()).to_lowercase();
        if !config::transition_exists(project_code.clone(), aliased_name.clone()) {
            self.get_transitions(ticket);
        }
        let transitioned_object = &config::get_transitions(project_code)[aliased_name];
        if (!transitioned_object.is_null()) && transitioned_object.is_number() {
            return transitioned_object.as_u16();
        }
        None
    }
}

impl TransitionService for DefaultTransitionService {
    fn print_transition_lists(&self, ticket: String) {
        let transition_object_response = self.get_transitions(&ticket);
        if transition_object_response.is_none() {
            eprintln!("Cannot find transitions for {ticket}");
            std::process::exit(1);
        }
        let transitions = transition_object_response.unwrap();
        println!("Allowed transitions for {ticket} are as below: ");
        for transition in transitions.members() {
            let name = String::from(transition["name"].as_str().unwrap());
            println!("- {name}");
        }
    }

    fn move_ticket_status(&self, ticket: String, status: String) {
        let transition_options = self.get_transition_code(&ticket, &status);
        if transition_options.is_none() {
            eprintln!("Invalid status...");
            std::process::exit(1);
        }
        let transition_code = transition_options.unwrap();
        let json_object = json::object! {
            "transition": {
                "id": transition_code
            }
        };
        let transitions_response =
            self.jira_api
                .post(&format!("issue/{ticket}/transitions"), json_object, 3);
        if transitions_response.is_err() {
            eprintln!("Unable to perform transition.");
            std::process::exit(1);
        }
        let response = transitions_response.unwrap();
        println!("Successfully Completed {response}");
    }
}
