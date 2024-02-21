use super::client;
use futures::future::join_all;
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

#[derive(Debug)]
pub enum Error {
    ClientError(client::Error),
}

impl From<client::Error> for Error {
    fn from(value: client::Error) -> Self {
        Error::ClientError(value)
    }
}
