use std::collections::HashSet;

use clap::{command, value_parser, Arg, Command};
mod steam;
use steam::{games_service, models::Game};
use tokio::runtime;

fn main() {
    let matches = command!()
        .version("0.0")
        .author("Chris West")
        .about("Some utility functions to run against steam")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("games-in-common")
                .about("find the intersection of games owned by provided steam accounts")
                .alias("gic")
                .arg(
                    Arg::new("steam_ids")
                        .help("id(s) assoicated with steam account(s), e.g., for accounts 42 and 7: steam-cli gic 7 42")
                        .num_args(1..)
                        .value_parser(value_parser!(u64)),
                )
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("games-missing-from-group")
            .about("find the games owned by everyone in the group except for the focused steam account")
            .alias("gmig")
            .arg(
                Arg::new("focus_steam_id")
                    .help("id associated with the focus steam account")
                    .value_parser(value_parser!(u64))
            )
            .arg(
                Arg::new("steam_ids")
                    .help("id(s) assoicated with steam account(s), e.g., for accounts 42 and 7: steam-cli gic 7 42")
                    .num_args(1..)
                    .value_parser(value_parser!(u64)),
            )
            .arg_required_else_help(true)
        )
        .get_matches();
    match matches.subcommand() {
        Some(("games-in-common", arguments)) => {
            let steam_ids = arguments
                .get_many::<u64>("steam_ids")
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            match get_blocking_runtime().block_on(games_service::find_games_in_common(steam_ids)) {
                Ok(games_in_common) => {
                    println!("{}", compute_sorted_games_string(&games_in_common));
                }
                Err(err) => {
                    println!("failed due to: {err:?}");
                }
            };
        }
        Some(("games-missing-from-group", arguments)) => {
            let focus_steam_id = arguments
                .get_one::<u64>("focus_steam_id")
                .expect("1 arg required");
            let steam_ids = arguments
                .get_many::<u64>("steam_ids")
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            match get_blocking_runtime().block_on(games_service::games_missing_from_group(
                focus_steam_id,
                steam_ids,
            )) {
                Ok(games) => println!("{}", compute_sorted_games_string(&games)),
                Err(err) => {
                    println!("failed due to: {err:?}");
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
