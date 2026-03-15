use std::path::Path;

use crate::{config::get_config_file_name, ioc::interface::Interface};

pub trait LogoutService: Interface {
    fn delete_configuration(&self);
}

pub struct DefaultLogoutService;

impl LogoutService for DefaultLogoutService {
    fn delete_configuration(&self) {
        let conf_fn = get_config_file_name();
        let path = Path::new(&conf_fn);
        std::fs::remove_file(path).unwrap(); // user will be prompted for login
        println!("You've logged out successfully, please keep your jira token handy for next time")
    }
}
