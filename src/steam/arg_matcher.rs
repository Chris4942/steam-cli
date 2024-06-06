use clap::{command, value_parser, Arg, ArgMatches, Command, Error as ClapError};
use std::{fmt::Display, vec};

pub async fn get_matches(args: vec::IntoIter<String>) -> Result<ArgMatches, Error> {
    let self_flag = Arg::new("self")
        .help("if present, then the calling user will be included as a steam id. In the discord implemenation, then this currently is hard coded to my steam_id")
        .long("self")
        .short('s')
        .alias("s")
        .action(clap::ArgAction::SetTrue);
    let strict_matching_flag = Arg::new("strict")
        .help("Use strict string matching against personaname")
        .long("strict")
        .short('s')
        .action(clap::ArgAction::SetTrue);
    let use_ids_flag = Arg::new("use-ids")
        .help("Use steamids directly instead of having them looked up dynamically")
        .long("use-ids")
        .short('i')
        .action(clap::ArgAction::SetTrue);
    let verbose_flag = Arg::new("verbose")
        .long("verbose")
        .short('v')
        .action(clap::ArgAction::SetTrue);

    let steam_ids_arg = Arg::new("steam_ids")
        .help("id(s) assoicated with steam account(s), e.g., for accounts 42 and 7: steam-cli gic 7 42")
        .num_args(1..)
        .value_parser(value_parser!(String));

    let steam_id_arg = Arg::new("steamid")
        .help("id associated with the steam account")
        .num_args(1)
        .value_parser(value_parser!(u64));

    let matches = command!()
        .version(env!("CARGO_PKG_VERSION"))
        .author("Chris West")
        .about("Some utility functions to run against steam")
        .arg_required_else_help(true)
        .arg(verbose_flag.clone())
        .subcommand(
            Command::new("games-in-common")
                .about("find the intersection of games owned by provided steam accounts")
                .alias("gic")
                .arg(strict_matching_flag.clone())
                .arg(use_ids_flag.clone())
                .arg(steam_ids_arg.clone())
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("games-missing-from-group")
                .about("find the games owned by everyone in the group except for the focused steam account")
                .alias("gmig")
                .arg(strict_matching_flag.clone())
                .arg(use_ids_flag.clone())
                .arg(
                    Arg::new("focus_steam_id")
                        .help("id associated with the focus steam account")
                        .num_args(1)
                        .value_parser(value_parser!(String))
                )
                .arg(steam_ids_arg.clone())
                .arg_required_else_help(true)
        )
        .subcommand(
            Command::new("get-available-endpoints")
                .about("print out all of the available endpoints. You'll probably want to pipe these into another file that you can search through")
        )
        .subcommand(
            Command::new("get-user-friends-list")
                .alias("friends")
                .about("get the friends list of the user")
                .arg(steam_id_arg.clone())
                .arg(self_flag.clone())
        )
        .subcommand(
            Command::new("get-player-summary")
                .about("get user summary data.")
                .long_about("get user summary data. Much more data is provided by the steam api than what is exposed by this command. Feel free to submit a PR to update this is you want more")
                .arg(strict_matching_flag.clone())
                .arg(use_ids_flag.clone())
                .arg(steam_ids_arg.clone())
                .arg(self_flag.clone())
                .arg_required_else_help(true)
        )
        .subcommand(
            Command::new("friends-who-own-game")
                .arg(
                    Arg::new("gameid")
                    .value_parser(value_parser!(u64))
                )
                .arg_required_else_help(true)
        )
        .try_get_matches_from(args)?;
    Ok(matches)
}

#[derive(Debug)]
pub enum Error {
    Matcher(ClapError),
}

impl From<ClapError> for Error {
    fn from(value: ClapError) -> Self {
        Error::Matcher(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Matcher(str) => write!(f, "MatcherError: {}", str),
        }
    }
}
