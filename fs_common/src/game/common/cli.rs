use std::path::PathBuf;

use clap::{
    builder::{StringValueParser, TypedValueParser, ValueParserFactory},
    error::ErrorKind,
    Parser, Subcommand,
};

#[derive(Parser, Debug)]
#[command(version = clap::crate_version!())]
#[command(author = clap::crate_authors!())]
pub struct CLArgs {
    #[arg(long, short, action, help = "Enable debugging features")]
    pub debug: bool,

    #[arg(long = "no-tick", action, help = "Turn off simulation by default")]
    pub no_tick: bool,

    #[arg(
        long,
        short,
        action,
        value_name = "IP:PORT",
        help = "Connect to a server automatically"
    )]
    pub connect: Option<IPPort>,

    #[arg(
        long = "game-dir",
        value_name = "PATH",
        action,
        default_value = "./gamedir/",
        help = "Set the game directory"
    )]
    pub game_dir: PathBuf,

    #[arg(
        long = "assets-dir",
        value_name = "PATH",
        action,
        default_value = "./gamedir/assets/",
        help = "Set the assets directory"
    )]
    pub assets_dir: PathBuf,

    #[arg(
        long = "assets-dir",
        value_name = "PATH",
        action,
        default_value = "./gamedir/asset_packs/",
        help = "Set the asset packs directory"
    )]
    pub asset_packs_dir: PathBuf,

    #[command(subcommand)]
    pub subcommand: Option<CLSubcommand>,
}

#[derive(Debug, Subcommand)]
pub enum CLSubcommand {
    #[command()]
    Server {
        #[arg(
            short,
            action,
            default_value = "6673",
            help = "The port to run the server on"
        )]
        port: u16,
    },
}

impl CLArgs {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

#[derive(Debug, Clone)]
pub struct IPPort {
    pub ip: String,
    pub port: u16,
}

impl std::fmt::Display for IPPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{}:{}", self.ip, self.port).as_str())
    }
}

impl ValueParserFactory for IPPort {
    type Parser = IPPortParser;

    fn value_parser() -> Self::Parser {
        IPPortParser
    }
}

impl TryFrom<String> for IPPort {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let Some((ip, port)) = value.split_once(':') else {
            Err(())?
        };

        Ok(Self { ip: ip.into(), port: port.parse().map_err(|_| ())? })
    }
}

#[derive(Clone, Debug)]
pub struct IPPortParser;
impl TypedValueParser for IPPortParser {
    type Value = IPPort;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let inner = StringValueParser::new();
        let val = inner.parse_ref(cmd, arg, value)?;
        IPPort::try_from(val).map_err(|_| {
            clap::Error::raw(
                ErrorKind::ValueValidation,
                "Connection address must be in form IP:PORT\n",
            )
        })
    }
}
