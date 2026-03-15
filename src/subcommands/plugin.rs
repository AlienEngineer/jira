use crate::plugins::lua_plugin::{get_plugins_path, install_bundled_plugins};
use clap::{App, ArgMatches, SubCommand};
use std::process;

pub fn subcommand() -> App<'static, 'static> {
    SubCommand::with_name("plugin")
        .about("Manage Lua plugins.")
        .subcommand(
            SubCommand::with_name("generate")
                .alias("new")
                .about("Install the bundled Lua plugins into your local plugins directory."),
        )
}

pub fn handle(matches: &ArgMatches) {
    if matches.subcommand_matches("generate").is_some() {
        if let Err(e) = install_bundled_plugins().map(|summary| {
            let plugins_path = get_plugins_path();
            println!(
                "Bundled plugins synced to {plugins_path} (copied: {}, skipped: {}).",
                summary.copied, summary.skipped
            );
        }) {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    } else {
        println!("Usage: jira plugin generate");
    }
}

#[cfg(test)]
mod tests {
    use super::subcommand;

    #[test]
    fn parses_generate_subcommand() {
        let matches = subcommand()
            .get_matches_from_safe(vec!["plugin", "generate"])
            .expect("generate subcommand should parse");

        matches
            .subcommand_matches("generate")
            .expect("generate subcommand should be present");
    }

    #[test]
    fn parses_new_alias() {
        let matches = subcommand()
            .get_matches_from_safe(vec!["plugin", "new"])
            .expect("new alias should parse");

        matches
            .subcommand_matches("generate")
            .expect("generate subcommand should match new alias");
    }
}
