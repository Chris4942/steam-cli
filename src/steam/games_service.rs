use super::client;
use futures::{future::join_all, join};
use std::collections::HashSet;

use super::models::Game;

pub async fn find_games_in_common(steam_ids: Vec<&u64>) -> Result<HashSet<Game>, Error> {
    let mut games_set = HashSet::<Game>::new();

    let query_results = join_all(
        steam_ids
            .into_iter()
            .map(|id| client::get_owned_games(client::GetOwnedGamesRequest { id: *id })),
    )
    .await;

    let mut first = true;
    for result in query_results {
        let games = result?;
        if first {
            for game in games {
                games_set.insert(game);
            }
            first = false;
        } else {
            let curr_games: HashSet<Game> = HashSet::from_iter(games.into_iter());
            games_set.retain(|game| curr_games.contains(game));
        }
    }
    Ok(games_set)
}

pub async fn games_missing_from_group(
    focus_steam_id: &u64,
    other_steam_ids: Vec<&u64>,
) -> Result<HashSet<Game>, Error> {
    println!("finding games missing from group");
    let result = join!(
        client::get_owned_games(client::GetOwnedGamesRequest {
            id: *focus_steam_id,
        }),
        find_games_in_common(other_steam_ids)
    );
    let mut games_in_common_minus_focus = result.1?;

    for game in result.0? {
        games_in_common_minus_focus.remove(&game);
    }
    Ok(games_in_common_minus_focus)
}

#[derive(Debug)]
pub enum Error {
    ClientError(client::Error),
}

impl From<client::Error> for Error {
    fn from(value: client::Error) -> Self {
        Error::ClientError(value)
    }
}
