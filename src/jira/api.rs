use crate::api;
use crate::api::request::ApiRequest;
use crate::config;
use crate::ioc::interface::Interface;
use std::error::Error;

pub trait JiraApi: Interface {
    fn get(&self, endpoint: &str, version: u8) -> Result<json::JsonValue, Box<dyn Error>>;
    fn get_v2(&self, endpoint: &str) -> Result<json::JsonValue, Box<dyn Error>> {
        self.get(endpoint, 2)
    }
    fn get_v3(&self, endpoint: &str) -> Result<json::JsonValue, Box<dyn Error>> {
        self.get(
            endpoint,
            config::get_config("version".to_string())
                .parse::<u8>()
                .unwrap_or(3),
        )
    }
    fn post(
        &self,
        endpoint: &str,
        json_value: json::JsonValue,
        version: u8,
    ) -> Result<String, Box<dyn Error>>;
    fn put(
        &self,
        endpoint: &str,
        json_value: json::JsonValue,
        version: u8,
    ) -> Result<String, Box<dyn Error>>;
    fn get_agile(&self, endpoint: &str) -> Result<json::JsonValue, Box<dyn Error>>;
}

#[derive(Default)]
pub struct ConfigJiraApi;

impl JiraApi for ConfigJiraApi {
    fn get(&self, endpoint: &str, version: u8) -> Result<json::JsonValue, Box<dyn Error>> {
        let api_request = build_api_request(endpoint, json::object! {}, version);
        api::get(api_request)
    }

    fn post(
        &self,
        endpoint: &str,
        json_value: json::JsonValue,
        version: u8,
    ) -> Result<String, Box<dyn Error>> {
        let api_request = build_api_request(endpoint, json_value, version);
        api::post(api_request)
    }

    fn put(
        &self,
        endpoint: &str,
        json_value: json::JsonValue,
        version: u8,
    ) -> Result<String, Box<dyn Error>> {
        let api_request = build_api_request(endpoint, json_value, version);
        api::put(api_request)
    }

    fn get_agile(&self, endpoint: &str) -> Result<json::JsonValue, Box<dyn Error>> {
        let api_request = build_api_request(endpoint, json::object! {}, 1);
        api::get_agile(api_request)
    }
}

fn build_api_request(endpoint: &str, json_value: json::JsonValue, version: u8) -> ApiRequest {
    let auth_mode = config::get_config("auth_mode".to_string());
    ApiRequest {
        url: endpoint.to_string(),
        username: config::get_config("email".to_string()),
        password: config::get_config("token".to_string()),
        json: json_value,
        namespace: config::get_config("namespace".to_string()),
        version,
        auth_mode: if auth_mode.is_empty() {
            "Basic".to_string()
        } else {
            auth_mode
        },
    }
}
