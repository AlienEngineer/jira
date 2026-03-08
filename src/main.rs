#[macro_use]
extern crate clap;
extern crate rpassword;
use clap::App;

pub mod api;
pub mod config;
pub mod jira;
pub mod prelude;
pub mod subcommands;
pub mod ui;

fn main() -> prelude::Result<()> {
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
        .subcommand(subcommands::sprint::subcommand());
    subcommands::handle_matches(app);
    Ok(())
}
