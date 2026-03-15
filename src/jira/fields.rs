use std::sync::Arc;

use crate::{ioc::interface::Interface, jira::api::JiraApi};

pub trait FieldsService: Interface {
    fn display_all_fields(&self, ticket: String);
}

pub struct DefaultFieldsService {
    jira_api: Arc<dyn JiraApi>,
}

impl DefaultFieldsService {
    pub fn new(jira_api: Arc<dyn JiraApi>) -> Self {
        Self { jira_api }
    }
}

impl FieldsService for DefaultFieldsService {
    fn display_all_fields(&self, ticket: String) {
        let fields_response = self.jira_api.get_v2(&format!("issue/{ticket}/editmeta"));
        if fields_response.is_err() {
            eprintln!("Error occurred in API Call: {fields_response:?}");
            std::process::exit(1);
        }
        let fields = &fields_response.unwrap()["fields"];
        if fields.is_null() {
            eprintln!("Cannot fetch fields");
            std::process::exit(1);
        }
        println!("{:35}: Field Header", "Key");
        println!("{:-<65}", "-");
        for (field, value) in fields.entries() {
            println!("{:35}: {}", field, value["name"]);
        }

        println!("{:=<65}", "=");
        println!(
            "\n\nNote: If you want to use custom fields as alias, you can add an alias as
jira alias --add \"customfield_XXXXX\" new_field

After that, you can pass new_field as options for field in details.
        "
        )
    }
}
