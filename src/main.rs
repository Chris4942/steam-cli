use std::{collections::HashSet, iter};

use clap::{command, value_parser, Arg, Command};
mod steam;
use steam::{
    client::{GetUserDetailsRequest, GetUserSummariesRequest},
    games_service,
    models::Game,
};
use tokio::runtime;

fn main() {
    let by_name_flag =
                    Arg::new("by-name")
                        .help("if present, then steam ids will be interpretted as persona names and resolves against your steam account and your friends steam accounts. This will not work if your friends list contains duplicate persona names")
                        .long("by-name")
                        .short('b')
                        .alias("b")
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
        .version("0.0")
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
        )
        .subcommand(
            Command::new("get-player-summary")
            .about("get user summary data")
            .long_about("get user summary data. Much more data is provided by the steam api than what is exposed by this command. Feel free to submit a PR to update this is you want more")
            .arg(steam_id_arg.clone())
            .arg_required_else_help(true)
        )
        .get_matches();
    match matches.subcommand() {
        Some(("games-in-common", arguments)) => {
            let rt = get_blocking_runtime();

            let partially_ingested_steam_ids = arguments
                .get_many::<String>("steam_ids")
                .into_iter()
                .flatten();
            let steam_ids = if arguments.get_flag("by-name") {
                let steam_id_strings = partially_ingested_steam_ids
                    .map(|s| s.trim());
                rt.block_on(games_service::resolve_usernames(steam_id_strings))
                    .expect("if this fails then we need to add some logic here to handle it")
            } else {
                partially_ingested_steam_ids
                    .map(|id| id.parse::<u64>().expect("ids should be valid steam ids"))
                    .collect::<Vec<_>>()
            };

            match rt.block_on(games_service::find_games_in_common(steam_ids)) {
                Ok(games_in_common) => {
                    println!("{}", compute_sorted_games_string(&games_in_common));
                }
                Err(err) => {
                    println!("failed due to: {err:?}");
                }
            };
        }
        Some(("games-missing-from-group", arguments)) => {
            let rt = get_blocking_runtime();

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
                let resolved_steam_ids = rt
                    .block_on(games_service::resolve_usernames(persona_names))
                    .expect("failed to resolve focus or other steam ids");
                (
                    resolved_steam_ids
                        .last()
                        .expect("resolved steam ids list empty. This was probably user error.")
                        .to_owned(),
                    resolved_steam_ids[..resolved_steam_ids.len() - 1].to_vec(),
                )
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
            match rt.block_on(games_service::games_missing_from_group(
                focus_steam_id,
                other_steam_ids,
            )) {
                Ok(games) => println!("{}", compute_sorted_games_string(&games)),
                Err(err) => {
                    eprintln!("failed due to: {err:?}");
                }
            }
        }
        Some(("get-available-endpoints", _)) => {
            match get_blocking_runtime().block_on(steam::client::get_available_endpoints()) {
                Ok(available_endpoints) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&available_endpoints)
                            .expect("failed to unwrap values")
                    );
                }
                Err(err) => {
                    eprintln!("failed due to: {err:?}");
                }
            }
        }
        Some(("get-user-friends-list", arguments)) => {
            let steamid = arguments.get_one::<u64>("steamid").expect("1 arg required");
            match get_blocking_runtime().block_on(steam::client::get_user_friends_list(
                GetUserDetailsRequest {
                    id: steamid.to_owned(),
                },
            )) {
                Ok(friends_list) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&friends_list)
                            .expect("failed to unwrap values")
                    );
                    println!("Total: {}", friends_list.len());
                }
                Err(err) => {
                    eprintln!("failed due to: {err:?}");
                }
            }
        }
        Some(("get-player-summary", arguments)) => {
            let steamids = arguments
                .get_many::<u64>("steamid")
                .into_iter()
                .flatten()
                .map(|i| i.to_owned())
                .collect::<Vec<_>>();
            match get_blocking_runtime().block_on(steam::client::get_user_summaries(
                GetUserSummariesRequest { ids: steamids },
            )) {
                Ok(friends_list) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&friends_list)
                            .expect("failed to unwrap values")
                    )
                }
                Err(err) => {
                    eprintln!("failed due to: {err:?}");
                }
            }
        }
        None => {
            println!("got nothing");
        }
        _ => unreachable!(),
    }
}

fn get_blocking_runtime() -> runtime::Runtime {
    runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("tokio is borked")
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
