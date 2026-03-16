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
        self.get(endpoint, config::get_version().parse::<u8>().unwrap_or(3))
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
    ApiRequest {
        url: endpoint.to_string(),
        username: config::get_email(),
        password: config::get_token(),
        json: json_value,
        namespace: config::get_base_url(),
        version,
        auth_mode: config::get_auth_mode(),
    }
}
