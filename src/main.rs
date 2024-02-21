use std::collections::HashSet;

use clap::{command, value_parser, Arg, Command};
mod steam;
use steam::{games_service, models::Game};

fn main() {
    let matches = command!()
        .version("0.0")
        .author("Chris West")
        .about("Some utility functions for use with steam")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("games-in-common")
                .alias("gic")
                .arg(
                    Arg::new("steam_ids")
                        .num_args(1..)
                        .value_parser(value_parser!(u64)),
                )
                .arg_required_else_help(true),
        )
        .get_matches();
    match matches.subcommand() {
        Some(("games-in-common", arguments)) => {
            let steam_ids = arguments
                .get_many::<u64>("steam_ids")
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            match games_service::find_games_in_common(steam_ids) {
                Ok(games_in_common) => {
                    println!("{}", compute_sorted_games_string(&games_in_common));
                }
                Err(err) => {
                    println!("failed due to: {err:?}");
                }
            };
        }
        None => {
            println!("got nothing");
        }
        _ => unreachable!(),
    }
}

fn compute_sorted_games_string(games: &HashSet<Game>) -> String {
    let mut games: Vec<&Game> = games.iter().collect();
    games.sort_by(|a, b| a.name.cmp(&b.name));
    return format!(
        "{games}\n\tTotal: {total}\n",
        games = games
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join("\n"),
        total = games.len()
    );
}
