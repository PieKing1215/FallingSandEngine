use clap::{ArgMatches, Command};

pub struct CommandHandler {
    commands: Command,
}

impl CommandHandler {
    pub fn new() -> Self {
        Self {
            commands: Command::new("commands")
                .no_binary_name(true)
                .subcommand_required(true)
                .disable_help_flag(true)
                .disable_version_flag(true)
                .help_template("Command Help:\n{subcommands}")
                .subcommand(
                    Command::new("shutdown")
                        .aliases(["exit", "quit", "stop"])
                        .about("Exit the game"),
                )
                .subcommand(Command::new("save").about("Save the game")),
        }
    }

    pub fn get_matches(&mut self, msg: &str) -> Result<ArgMatches, clap::Error> {
        self.commands
            .try_get_matches_from_mut(msg.split_whitespace())
    }
}

impl Default for CommandHandler {
    fn default() -> Self {
        Self::new()
    }
}
