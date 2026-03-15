#[macro_use]
extern crate clap;
extern crate rpassword;
use clap::App;

pub mod api;
pub mod config;
pub mod ioc;
pub mod jira;
pub mod plugins;
pub mod prelude;
pub mod subcommands;
pub mod ui;

fn main() -> prelude::Result<()> {
    init_ioc_container();

    config::ensure_config()?;
    let app = App::new("JIRA")
        .version(crate_version!())
        .author("Alien Engineer <aimirim.software@gmail.com>")
        .about("This is a command line application that can be used as a personal productivity tool for interacting with JIRA")
        .subcommand(subcommands::transition::subcommand())
        .subcommand(subcommands::list::subcommand())
        .subcommand(subcommands::detail::subcommand())
        .subcommand(subcommands::alias::subcommand())
        .subcommand(subcommands::fields::subcommand())
        .subcommand(subcommands::assign::subcommand())
        .subcommand(subcommands::comments::subcommand())
        .subcommand(subcommands::update::subcommand())
        .subcommand(subcommands::autocompletion::subcommand())
        .subcommand(subcommands::new_subcommand::subcommand())
        .subcommand(subcommands::logout::subcommand())
        .subcommand(subcommands::config::subcommand())
        .subcommand(subcommands::sprint::subcommand())
        .subcommand(subcommands::raw::subcommand())
        .subcommand(subcommands::plugin::subcommand());
    subcommands::handle_matches(app);
    Ok(())
}

fn init_ioc_container() {
    let mut ioc = ioc::Ioc::new();
    register_service!(ioc, jira::api::JiraApi, jira::api::ConfigJiraApi);
    register_service!(ioc, jira::user::CurrentUserService, {
        jira::user::DefaultCurrentUserService::new(get_instance!(jira::api::JiraApi))
    });
    register_service!(ioc, jira::utils::MetadataService, {
        jira::utils::DefaultMetadataService::new(get_instance!(jira::api::JiraApi))
    });
    register_service!(ioc, jira::assign::AssignService, {
        jira::assign::DefaultAssignService::new(
            get_instance!(jira::api::JiraApi),
            get_instance!(jira::utils::MetadataService),
        )
    });
    register_service!(ioc, jira::fields::FieldsService, {
        jira::fields::DefaultFieldsService::new(get_instance!(jira::api::JiraApi))
    });
    register_service!(ioc, jira::lists::ListService, {
        jira::lists::DefaultListService::new(get_instance!(jira::api::JiraApi))
    });
    register_service!(ioc, jira::raw::RawService, {
        jira::raw::DefaultRawService::new(get_instance!(jira::api::JiraApi))
    });
    register_service!(ioc, jira::comments::CommentsService, {
        jira::comments::DefaultCommentsService::new(
            get_instance!(jira::api::JiraApi),
            get_instance!(jira::utils::MetadataService),
        )
    });
    register_service!(ioc, jira::details::DetailService, {
        jira::details::DefaultDetailService::new(
            get_instance!(jira::api::JiraApi),
            get_instance!(jira::comments::CommentsService),
        )
    });
    register_service!(ioc, jira::new_issue::IssueCreationService, {
        jira::new_issue::DefaultIssueCreationService::new(
            get_instance!(jira::api::JiraApi),
            get_instance!(jira::utils::MetadataService),
        )
    });
    register_service!(ioc, jira::update::UpdateService, {
        jira::update::DefaultUpdateService::new(get_instance!(jira::api::JiraApi))
    });
    register_service!(ioc, jira::transitions::TransitionService, {
        jira::transitions::DefaultTransitionService::new(get_instance!(jira::api::JiraApi))
    });
    register_service!(ioc, jira::sprint::SprintService, {
        jira::sprint::DefaultSprintService::new(get_instance!(jira::api::JiraApi))
    });
    register_service!(
        ioc,
        jira::logout::LogoutService,
        jira::logout::DefaultLogoutService
    );
    if ioc::set_global(ioc).is_err() {
        panic!("global IoC container should only be initialized once");
    }
}
