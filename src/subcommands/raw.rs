use clap::{App, Arg, SubCommand};

pub fn subcommand() -> App<'static, 'static> {
    SubCommand::with_name("raw")
        .about("Fetch a PBI and output the raw JSON response from the Jira API.")
        .arg(
            Arg::with_name("TICKET")
                .help("Ticket ID (e.g. PROJ-123).")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("pretty")
                .short("p")
                .long("pretty")
                .help("Pretty-print the JSON output (default: compact)."),
        )
}
