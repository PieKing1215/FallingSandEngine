use clap::{App, ArgMatches, SubCommand};

pub struct CommandHandler<'a> {
    commands: App<'a, 'a>,
}

impl CommandHandler<'_> {
    pub fn new() -> Self {
        Self {
            commands: App::new("commands")
                .setting(clap::AppSettings::NoBinaryName)
                .setting(clap::AppSettings::SubcommandRequired)
                .setting(clap::AppSettings::DisableHelpFlags)
                .setting(clap::AppSettings::DisableVersion)
                .setting(clap::AppSettings::VersionlessSubcommands)
                .template("Command Help:\n{subcommands}")
                .subcommand(
                    SubCommand::with_name("shutdown")
                        .aliases(&["exit", "quit", "stop"])
                        .about("Exit the game"),
                )
                .subcommand(SubCommand::with_name("save").about("Save the game")),
        }
    }

    pub fn get_matches(&mut self, msg: &str) -> Result<ArgMatches, clap::Error> {
        self.commands
            .get_matches_from_safe_borrow(msg.split_whitespace())
    }
}
