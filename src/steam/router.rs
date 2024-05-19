use std::{collections::HashSet, fmt::Display, num::ParseIntError, vec};

use clap::{command, value_parser, Arg, ArgMatches, Command, Error as ClapError};

use crate::steam::{
    client::{GetUserDetailsRequest, GetUserSummariesRequest},
    models::Game,
};

use super::{client, service};

const FUZZY_THRESHOLD: u32 = 50;

pub async fn run_command<'a>(
    args: vec::IntoIter<String>,
    user_id: Option<u64>,
) -> Result<String, Error> {
    let self_flag = Arg::new("self")
        .help("if present, then the calling user will be included as a steam id. In the discord implemenation, then this currently is hard coded to my steam_id")
        .long("self")
        .short('s')
        .alias("s")
        .action(clap::ArgAction::SetTrue);
    let strict_matching_flag = Arg::new("strict")
        .help("Use strict string matching against personaname or realname")
        .long("strict")
        .short('s')
        .action(clap::ArgAction::SetTrue);
    let use_ids_flag = Arg::new("use-ids")
        .help("Use steamids directly instead of having them looked up dynamically")
        .long("use-ids")
        .short('i')
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
    run_subcommand(matches, user_id).await
}

fn compute_sorted_games_string(games: &HashSet<Game>) -> String {
    let mut games: Vec<&Game> = games.iter().collect();
    games.sort_by(|a, b| a.name.cmp(&b.name));
    format!(
        "{games}\n\tTotal: {total}\n",
        games = games
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join("\n"),
        total = games.len()
    )
}

async fn run_subcommand<'a>(
    matches: ArgMatches,
    user_steam_id: Option<u64>,
) -> Result<String, Error> {
    match matches.subcommand() {
        Some(("games-in-common", arguments)) => {
            let steam_ids = get_steam_ids(arguments, user_steam_id, "steam_ids").await?;
            Ok(compute_sorted_games_string(
                &service::find_games_in_common(steam_ids).await?,
            ))
        }
        Some(("games-missing-from-group", arguments)) => {
            let focus_steam_id = get_steam_ids(arguments, user_steam_id, "focus_steam_id")
                .await?
                .first()
                .ok_or(Error::Argument("could not find focus_steam_id".to_string()))?
                .to_owned();
            let other_steam_ids = get_steam_ids(arguments, user_steam_id, "steam_ids").await?;
            let games = service::games_missing_from_group(focus_steam_id, other_steam_ids).await?;
            Ok(compute_sorted_games_string(&games))
        }
        Some(("get-available-endpoints", _)) => {
            let available_endpoints = client::get_available_endpoints().await?;
            let pretty_string = serde_json::to_string_pretty(&available_endpoints)?;
            Ok(pretty_string)
        }
        Some(("get-user-friends-list", arguments)) => {
            let id = if arguments.get_flag("self") {
                user_steam_id.ok_or(Error::Argument(
                    "user_steam_id is required in order to resolve user_steam_ids by persona name"
                        .to_string(),
                ))?
            } else {
                arguments
                    .get_one::<u64>("steamid")
                    .ok_or(Error::Argument("1 arg required".to_string()))?
                    .to_owned()
            };
            let friends = client::get_user_friends_list(GetUserDetailsRequest { id }).await?;

            let summaries = client::get_user_summaries(GetUserSummariesRequest {
                ids: friends
                    .iter()
                    .map(|friend| friend.steamid.parse::<u64>())
                    .collect::<Result<Vec<u64>, ParseIntError>>()?,
            })
            .await?;
            Ok(format!(
                "friend summaries: {}",
                serde_json::to_string_pretty(&summaries)?,
            ))
        }
        Some(("get-player-summary", arguments)) => {
            let steamids = get_steam_ids(arguments, user_steam_id, "steam_ids").await?;
            let friends_list =
                client::get_user_summaries(GetUserSummariesRequest { ids: steamids }).await?;
            Ok(serde_json::to_string_pretty(&friends_list)?)
        }
        Some(("friends-who-own-game", arguments)) => {
            let gameid = arguments
                .get_one::<u64>("gameid")
                .ok_or(Error::Argument("gameid must be a valid u64".to_string()))?;

            let user_steam_id = user_steam_id.ok_or(Error::Argument(
                "user_steam_id must be set to run this command".to_string(),
            ))?;

            let friends_list = service::find_friends_who_own_game(gameid, user_steam_id).await?;

            Ok(format!(
                "{}\nTotal: {}",
                serde_json::to_string_pretty(&friends_list)?,
                friends_list.len()
            ))
        }
        None => Err(Error::Argument("should be unreachable".to_string())),
        _ => unreachable!(),
    }
}

pub enum Error {
    Argument(String),
    Parse(String),
    Execution(String),
    CommandNotFound(ClapError),
}

impl From<client::Error> for Error {
    fn from(value: client::Error) -> Self {
        Error::Execution(value.to_string())
    }
}

impl From<service::Error> for Error {
    fn from(value: service::Error) -> Self {
        Error::Execution(value.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Execution(value.to_string())
    }
}

impl From<ClapError> for Error {
    fn from(value: ClapError) -> Self {
        Error::CommandNotFound(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Argument(str) => write!(f, "ArgumentError: {}", str),
            Error::Parse(str) => write!(f, "ParseError: {}", str),
            Error::Execution(str) => write!(f, "ExecutionError: {}", str),
            Error::CommandNotFound(str) => write!(f, "{}", str),
        }
    }
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Error::Parse(value.to_string())
    }
}

async fn get_steam_ids<'a>(
    arguments: &ArgMatches,
    user_steam_id: Option<u64>,
    steam_ids_key: &str,
) -> Result<Vec<u64>, Error> {
    let partially_ingested_steam_ids = arguments
        .get_many::<String>(steam_ids_key)
        .into_iter()
        .flatten();
    let steam_ids = if arguments.get_flag("use-ids") {
        partially_ingested_steam_ids
            .map(|id| id.parse::<u64>())
            .collect::<Result<Vec<_>, ParseIntError>>()?
    } else {
        let user_steam_id = user_steam_id.ok_or(Error::Argument(
            "user_steam_id is required in order to resolve user_steam_ids by persona name"
                .to_string(),
        ))?;
        let steam_id_strings = partially_ingested_steam_ids.map(|s| s.trim());
        if arguments.get_flag("strict") {
            service::resolve_usernames_strictly(steam_id_strings, user_steam_id).await?
        } else {
            service::resolve_usernames_fuzzily(steam_id_strings, user_steam_id, FUZZY_THRESHOLD)
                .await?
        }
    };
    Ok(steam_ids)
}
