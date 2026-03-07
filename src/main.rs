//! # JIRA Application
//!
//! This is a command line application that can be used as a personal productivity tool for
//! interacting with JIRA.
//!
//! # Installing
//! You can install via Homebrew:
//! ```
//! brew install alienengineer/jira/jira
//! ```
//!
//! # Usage
//! ### First Run
//! You can open the jira cli for the first time by just entering
//! ```
//! jira
//! ```
//! Upon first run, it will ask you with the namespace, email and token.
//! If your JIRA Dashboard starts with format https://example.atlassian.net, your namespace is
//! example.
//! Similarly, you can create a token from [https://id.atlassian.com/manage-profile/security/api-tokens](https://id.atlassian.com/manage-profile/security/api-tokens)
//!
//! Configuration is stored in XDG-compliant locations:
//! - Linux: $XDG_CONFIG_HOME/jira/configuration.json (default: ~/.config/jira/configuration.json)
//! - macOS: ~/Library/Application Support/jira/configuration.json
//!
//!
#[macro_use]
extern crate clap;
extern crate rpassword;
use clap::App;

pub mod api;
pub mod config;
pub mod jira;
pub mod prelude;
pub mod subcommands;

fn main() -> prelude::Result<()> {
    config::ensure_config()?;
    let app = App::new("JIRA")
        .version(crate_version!())
        .author("alienengineer")
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
        .subcommand(subcommands::logout::subcommand());
    subcommands::handle_matches(app);
    Ok(())
}
