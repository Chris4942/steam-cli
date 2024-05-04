use super::client::{self, GetUserSummariesRequest};
use futures::{future::join_all, join};
use std::{collections::HashSet, fmt::Display};

use super::models::Game;

pub async fn find_games_in_common(steam_ids: Vec<u64>) -> Result<HashSet<Game>, Error> {
    let mut games_set = HashSet::<Game>::new();

    let query_results = join_all(
        steam_ids
            .into_iter()
            .map(|id| client::get_owned_games(client::GetUserDetailsRequest { id })),
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
    focus_steam_id: u64,
    other_steam_ids: Vec<u64>,
) -> Result<HashSet<Game>, Error> {
    println!("finding games missing from group");
    let result = join!(
        client::get_owned_games(client::GetUserDetailsRequest { id: focus_steam_id }),
        find_games_in_common(other_steam_ids)
    );
    let mut games_in_common_minus_focus = result.1?;

    for game in result.0? {
        games_in_common_minus_focus.remove(&game);
    }
    Ok(games_in_common_minus_focus)
}

pub async fn resolve_usernames(
    usernames: impl Iterator<Item = &str>,
    my_steamid: u64,
) -> Result<Vec<u64>, Error> {
    let friends =
        client::get_user_friends_list(client::GetUserDetailsRequest { id: my_steamid }).await?;
    println!("got friends");
    let mut ids: Vec<u64> = friends
        .iter()
        .map(|friend| {
            friend
                .steamid
                .parse::<u64>()
                .expect("The Steam api returned bad data")
        })
        .collect();
    ids.push(my_steamid);
    let user_summaries =
        client::get_user_summaries(client::GetUserSummariesRequest { ids }).await?;
    let steamids = usernames
        .map(|username| {
            user_summaries
                .iter()
                .find(|user| {
                    user.personaname.to_ascii_lowercase() == *username.to_ascii_lowercase()
                })
                .expect("supplied user was not in list")
                .steamid
                .parse::<u64>()
                .expect("logic error occured in steamapi or I had bad assumptions")
        })
        .collect::<Vec<_>>();
    Ok(steamids)
}

pub async fn find_friends_who_own_game(
    appid: &u64,
    my_steamid: u64,
) -> Result<Vec<client::UserSummary>, Error> {
    let friends =
        client::get_user_friends_list(client::GetUserDetailsRequest { id: my_steamid }).await?;

    let steamids_iterator = friends
        .iter()
        .map(|friend| friend.steamid.parse::<u64>().unwrap())
        .chain(std::iter::once(my_steamid));

    let player_owned_games = join_all(
        steamids_iterator
            .clone() // We need to use this iterator again later so we can't move it here
            .map(|id| (client::get_owned_games(client::GetUserDetailsRequest { id })))
            .collect::<Vec<_>>(),
    )
    .await;

    let friends_with_game_ids = player_owned_games
        .into_iter()
        .zip(steamids_iterator)
        .filter(|(result, _)| match result {
            Ok(_) => true,
            Err(err) => {
                eprintln!("filtering out result due to {:?}", err);
                false
            }
        })
        .map(|(ok_result, steamid)| (ok_result.expect("we just filtered by is_ok"), steamid))
        .filter(|(games, _)| games.iter().any(|game| &game.appid == appid))
        .map(|(_, steamid)| steamid)
        .collect::<Vec<u64>>();

    let user_summaries = client::get_user_summaries(GetUserSummariesRequest {
        ids: friends_with_game_ids,
    })
    .await?;

    Ok(user_summaries)
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

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ClientError(value) => write!(f, "ClientError: {}", value.to_string()),
        }
    }
}
