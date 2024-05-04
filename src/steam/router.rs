use std::{collections::HashSet, fmt::Display, iter, vec};

use clap::{command, value_parser, Arg, ArgMatches, Command, Error as ClapError};

use crate::steam::{
    client::{GetUserDetailsRequest, GetUserSummariesRequest},
    models::Game,
};

use super::{client, service};

pub async fn run_command<'a>(
    args: vec::IntoIter<String>,
    user_id: Option<u64>,
) -> Result<String, Error<'a>> {
    let by_name_flag = Arg::new("by-name")
        .help("if present, then steam ids will be interpretted as persona names and resolves against your steam account and your friends steam accounts. This will not work if your friends list contains duplicate persona names")
        .long("by-name")
        .short('b')
        .alias("b")
        .action(clap::ArgAction::SetTrue);
    let self_flag = Arg::new("self")
        .help("if present, then the calling user will be included as a steam id. In the discord implemenation, then this currently is hard coded to my steam_id")
        .long("self")
        .short('s')
        .alias("s")
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
        .version("0.1.5")
        .author("Chris West")
        .about("Some utility functions to run against steam")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("games-in-common")
                .about("find the intersection of games owned by provided steam accounts")
                .alias("gic")
                .arg(by_name_flag.clone())
                .arg(steam_ids_arg.clone())
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("games-missing-from-group")
            .about("find the games owned by everyone in the group except for the focused steam account")
            .alias("gmig")
            .arg(by_name_flag.clone())
            .arg(
                Arg::new("focus_steam_id")
                    .help("id associated with the focus steam account")
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
            .about("get user summary data")
            .long_about("get user summary data. Much more data is provided by the steam api than what is exposed by this command. Feel free to submit a PR to update this is you want more")
            .arg(steam_id_arg.clone())
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
) -> Result<String, Error<'a>> {
    match matches.subcommand() {
        Some(("games-in-common", arguments)) => {
            let partially_ingested_steam_ids = arguments
                .get_many::<String>("steam_ids")
                .into_iter()
                .flatten();
            let steam_ids = if arguments.get_flag("by-name") {
                match user_steam_id {
                    Some(user_steam_id) => {
                        let steam_id_strings = partially_ingested_steam_ids.map(|s| s.trim());
                        service::resolve_usernames(steam_id_strings, user_steam_id)
                            .await
                            .expect(
                                "if this fails then we need to add some logic here to handle it",
                            )
                    }
                    None => return Err(Error::Argument("user_steam_id is required in order to resolve user_steam_ids by persona name")),
                }
            } else {
                partially_ingested_steam_ids
                    .map(|id| id.parse::<u64>().expect("ids should be valid steam ids"))
                    .collect::<Vec<_>>()
            };

            match service::find_games_in_common(steam_ids).await {
                Ok(games_in_common) => Ok(compute_sorted_games_string(&games_in_common)),
                Err(err) => Ok(format!("failed due to: {err:?}")),
            }
        }
        Some(("games-missing-from-group", arguments)) => {
            let focus_steam_id = arguments
                .get_one::<String>("focus_steam_id")
                .expect("1 arg required");
            let partially_ingested_steam_ids = arguments
                .get_many::<String>("steam_ids")
                .into_iter()
                .flatten();
            let (focus_steam_id, other_steam_ids) = if arguments.get_flag("by-name") {
                let persona_names = partially_ingested_steam_ids
                    .map(|s| s.trim())
                    .chain(iter::once(focus_steam_id.trim()));
                match user_steam_id {
                    Some(user_steam_id) => {
                        let resolved_steam_ids =
                            service::resolve_usernames(persona_names, user_steam_id)
                                .await
                                .expect("failed to resolve focus or other steam ids");
                        (
                            resolved_steam_ids
                                .last()
                                .expect("resolved steam ids list empty. This was probably user error.")
                                .to_owned(),
                            resolved_steam_ids[..resolved_steam_ids.len() - 1].to_vec(),
                        )
                    }
                    None => return Err(Error::Argument("user_steam_id is required in order to resolve user_steam_ids by persona name")),
                }
            } else {
                (
                    focus_steam_id
                        .parse::<u64>()
                        .expect("focus steam id should be a valid u64"),
                    partially_ingested_steam_ids
                        .map(|id| id.parse::<u64>().expect("ids should be valid steam ids"))
                        .collect::<Vec<_>>(),
                )
            };
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
                match user_steam_id {
                    Some(user_steam_id) => user_steam_id,
                    None => return Err(Error::Parse("user_steam_id is required in order to resolve user_steam_ids by persona name".to_owned())),
                }
            } else {
                arguments
                    .get_one::<u64>("steamid")
                    .ok_or(Error::Argument("1 arg required"))?
                    .to_owned()
            };
            let friends = client::get_user_friends_list(GetUserDetailsRequest { id })
                .await
                .expect("this better have worked");

            let summaries = client::get_user_summaries(GetUserSummariesRequest {
                ids: friends
                    .iter()
                    .map(|friend| friend.steamid.parse::<u64>().expect("parsing u64 failed"))
                    .collect::<Vec<u64>>(),
            })
            .await
            .expect("failed to get summaries");
            Ok(format!(
                "friend summaries: {}",
                serde_json::to_string_pretty(&summaries).expect("failed to pretty jsonify"),
            ))
        }
        Some(("get-player-summary", arguments)) => {
            let steam_ids_iter = arguments
                .get_many::<u64>("steamid")
                .into_iter()
                .flatten()
                .map(|i| i.to_owned());
            let steamids = if arguments.get_flag("self") {
                match user_steam_id {
                    Some(user_steam_id) => steam_ids_iter
                        .chain(iter::once(user_steam_id))
                        .collect::<Vec<_>>(),
                    None => {
                        return Err(Error::Argument(
                            "user_steam_id is required in order use self flag",
                        ))
                    }
                }
            } else {
                steam_ids_iter.collect::<Vec<_>>()
            };
            let friends_list =
                client::get_user_summaries(GetUserSummariesRequest { ids: steamids }).await?;
            Ok(serde_json::to_string_pretty(&friends_list).expect("failed to unwrap values"))
        }
        Some(("friends-who-own-game", arguments)) => {
            let gameid = arguments
                .get_one::<u64>("gameid")
                .ok_or(Error::Argument("gameid must be a valid u64"))?;

            let user_steam_id = user_steam_id.ok_or(Error::Argument(
                "user_steam_id must be set to run this command",
            ))?;

            let friends_list = service::find_friends_who_own_game(gameid, user_steam_id).await?;

            Ok(format!(
                "{}\nTotal: {}",
                serde_json::to_string_pretty(&friends_list).unwrap(),
                friends_list.len()
            ))
        }
        None => return Err(Error::Argument("thing")),
        _ => unreachable!(),
    }
}

pub enum Error<'a> {
    Argument(&'a str),
    Parse(String),
    Execution(String),
}

impl<'a> From<client::Error> for Error<'a> {
    fn from(value: client::Error) -> Self {
        Error::Execution(value.to_string())
    }
}

impl<'a> From<service::Error> for Error<'a> {
    fn from(value: service::Error) -> Self {
        Error::Execution(value.to_string())
    }
}

impl<'a> From<serde_json::Error> for Error<'a> {
    fn from(value: serde_json::Error) -> Self {
        Error::Execution(value.to_string())
    }
}

impl<'a> From<ClapError> for Error<'a> {
    fn from(value: ClapError) -> Self {
        Error::Execution(value.to_string())
    }
}

impl<'a> Display for Error<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Argument(str) => write!(f, "ArgumentError: {}", str),
            Error::Parse(str) => write!(f, "ParseError: {}", str),
            Error::Execution(str) => write!(f, "ExecutionError: {}", str),
        }
    }
}
