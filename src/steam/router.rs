use std::{collections::HashSet, fmt::Display, num::ParseIntError, vec};

use clap::ArgMatches;

use crate::steam::{
    client::{GetUserDetailsRequest, GetUserSummariesRequest},
    logger::Logger,
    models::Game,
};

use super::{
    arg_matcher::{self, get_matches},
    client,
    logger::FilteringLogger,
    service,
};

const FUZZY_THRESHOLD: u32 = 50;

pub async fn route_arguments(
    args: vec::IntoIter<String>,
    user_id: Option<u64>,
    logger: &dyn Logger,
) -> Result<(), Error> {
    match run_command(args, user_id, logger).await {
        Ok(str) => logger.stdout(str).await,
        Err(err) => {
            logger.stderr(err.to_string()).await;
            return Err(err);
        }
    };
    Ok(())
}

pub async fn run_command(
    args: vec::IntoIter<String>,
    user_id: Option<u64>,
    logger: &dyn Logger,
) -> Result<String, Error> {
    let matches = get_matches(args).await?;

    run_subcommand(
        matches,
        user_id,
        &FilteringLogger {
            logger,
            verbose: true,
        },
    )
    .await
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
    logger: &'a FilteringLogger<'a>,
) -> Result<String, Error> {
    match matches.subcommand() {
        Some(("games-in-common", arguments)) => {
            let steam_ids = get_steam_ids(arguments, user_steam_id, "steam_ids", logger).await?;
            Ok(compute_sorted_games_string(
                &service::find_games_in_common(steam_ids, logger).await?,
            ))
        }
        Some(("games-missing-from-group", arguments)) => {
            let focus_steam_id = get_steam_ids(arguments, user_steam_id, "focus_steam_id", logger)
                .await?
                .first()
                .ok_or(Error::Argument("could not find focus_steam_id".to_string()))?
                .to_owned();
            let other_steam_ids =
                get_steam_ids(arguments, user_steam_id, "steam_ids", logger).await?;
            let games =
                service::games_missing_from_group(focus_steam_id, other_steam_ids, logger).await?;
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
            let friends =
                client::get_user_friends_list(GetUserDetailsRequest { id }, logger).await?;

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
            let steamids = get_steam_ids(arguments, user_steam_id, "steam_ids", logger).await?;
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

            let friends_list =
                service::find_friends_who_own_game(gameid, user_steam_id, logger).await?;

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

#[derive(Debug)]
pub enum Error {
    Argument(String),
    Parse(String),
    Execution(String),
    CommandNotFound(arg_matcher::Error),
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

impl From<arg_matcher::Error> for Error {
    fn from(value: arg_matcher::Error) -> Self {
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
    logger: &'a FilteringLogger<'a>,
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
            service::resolve_usernames_strictly(steam_id_strings, user_steam_id, logger).await?
        } else {
            service::resolve_usernames_fuzzily(
                steam_id_strings,
                user_steam_id,
                FUZZY_THRESHOLD,
                logger,
            )
            .await?
        }
    };
    Ok(steam_ids)
}
