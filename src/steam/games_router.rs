use std::collections::HashSet;

// TODO: the arg_matcher, router and games_router files should all be moved into their own
// submodule
use clap::ArgMatches;
use futures::future::join_all;

use crate::steam::service::games_missing_from_group;

use super::{
    client,
    logger::FilteringLogger,
    router::{compute_game_info_string, compute_sorted_games_string, get_steam_ids, Error},
    service::{filter_games, find_games_in_common},
};

pub async fn run_games_command<'a>(
    arguments: &ArgMatches,
    user_steam_id: Option<u64>,
    logger: &'a FilteringLogger<'a>,
) -> Result<String, Error> {
    let games = match arguments.subcommand() {
        Some(("in-common", arguments)) => {
            let steam_ids = get_steam_ids(arguments, user_steam_id, "steam_ids", logger).await?;
            find_games_in_common(steam_ids, logger).await?
        }
        Some(("missing-from-group", arguments)) => {
            let focus_steam_id = get_steam_ids(arguments, user_steam_id, "focus_steam_id", logger)
                .await?
                .first()
                .ok_or(Error::Argument("could not find focus_steam_id".to_string()))?
                .to_owned();
            let other_steam_ids =
                get_steam_ids(arguments, user_steam_id, "steam_ids", logger).await?;

            games_missing_from_group(focus_steam_id, other_steam_ids, logger).await?
        }
        _ => {
            panic!("no subcommand matched")
        }
    };
    let filtered_games = match arguments.get_one::<String>("filter") {
        None => games,
        Some(filter) => {
            let filter_numbers = HashSet::from_iter(
                match filter.as_str() {
                    "multiplayer" => [27, 36, 38].iter(),
                    "controller" => [28].iter(),
                    _ => panic!(),
                }
                .cloned(),
            );
            let filtered_games = filter_games(games.to_owned(), filter_numbers, logger).await?;
            HashSet::from_iter(filtered_games.iter().cloned())
        }
    };
    if !arguments.get_flag("info") {
        Ok(compute_sorted_games_string(filtered_games))
    } else {
        // TODO: this get_game_info call should be able to take all of them at once
        let game_infos: Vec<Result<client::GetGameInfoResponse, client::Error>> = join_all(
            filtered_games
                .iter()
                .map(|game| client::get_game_info(&game.appid, logger)),
        )
        .await
        .into_iter()
        .collect::<Vec<_>>();
        let game_infos: Result<Vec<client::GetGameInfoResponse>, client::Error> =
            game_infos.into_iter().collect();
        Ok(compute_game_info_string(game_infos?))
    }
}
