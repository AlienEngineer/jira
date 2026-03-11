use clap::Arg;
use clap::{App, ArgMatches, SubCommand};

pub fn subcommand() -> App<'static, 'static> {
    SubCommand::with_name("plugin")
        .about("Manage Lua plugins.")
        .subcommand(
            SubCommand::with_name("new")
                .about("Generate a plugin template and open it in the default editor.")
                .arg(
                    Arg::with_name("NAME")
                        .help(
                            "Plugin name (without extension). The 'start_' prefix is added \
                             automatically if not already present.",
                        )
                        .required(true)
                        .index(1),
                ),
        )
}

pub fn handle(matches: &ArgMatches) {
    if let Some(new_matches) = matches.subcommand_matches("new") {
        let name = new_matches.value_of("NAME").unwrap();
        if let Err(e) = crate::plugins::lua_plugin::create_plugin(name) {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    } else {
        println!("Usage: jira plugin new <NAME>");
    }
}
