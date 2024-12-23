use std::collections::HashSet;

// TODO: the arg_matcher, router and games_router files should all be moved into their own
// submodule
use clap::ArgMatches;
use futures::future::join_all;

use crate::steam::{client::GameInfo, models::Game, service::games_missing_from_group};

use super::{
    client::{self, GetGameInfoResponse},
    logger::FilteringLogger,
    router::{get_steam_ids, Error},
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

pub fn compute_sorted_games_string(games: impl IntoIterator<Item = Game>) -> String {
    let mut games: Vec<Game> = games.into_iter().collect();
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

pub fn compute_game_info_string(games: impl IntoIterator<Item = GetGameInfoResponse>) -> String {
    let games: Vec<GameInfo> = games
        .into_iter()
        .flat_map(|response| response.games.into_values())
        .collect();
    format!(
        "{games}\n\tTotal: {total}\n",
        games = games
            .iter()
            .map(|game_info| match &game_info.data {
                None => "No info".to_string(),
                Some(data) => format!(
                    "{name},{id},{requirements}",
                    name = data.name,
                    id = data.steam_appid,
                    requirements = match &data.pc_requirements {
                        None => "No requirements".to_string(),
                        Some(req) => match &req.recommended {
                            None => "No recommendations".to_string(),
                            Some(req) => req.clone(),
                        },
                    }
                )
                .to_string(),
            })
            .collect::<Vec<String>>()
            .join("\n"),
        total = games.len()
    )
}
