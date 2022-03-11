use clap::{App, ArgMatches};

pub struct CommandHandler<'a> {
    commands: App<'a>,
}

impl CommandHandler<'_> {
    pub fn new() -> Self {
        Self {
            commands: App::new("commands")
                .setting(clap::AppSettings::NoBinaryName)
                .setting(clap::AppSettings::SubcommandRequired)
                .setting(clap::AppSettings::DisableHelpFlag)
                .setting(clap::AppSettings::DisableVersionFlag)
                .help_template("Command Help:\n{subcommands}")
                .subcommand(
                    App::new("shutdown")
                        .aliases(&["exit", "quit", "stop"])
                        .about("Exit the game"),
                )
                .subcommand(App::new("save").about("Save the game")),
        }
    }

    pub fn get_matches(&mut self, msg: &str) -> Result<ArgMatches, clap::Error> {
        self.commands
            .try_get_matches_from_mut(msg.split_whitespace())
    }
}

impl Default for CommandHandler<'_> {
    fn default() -> Self {
        Self::new()
    }
}
