use super::client::{self, GetUserSummariesRequest};
use futures::{future::join_all, join};
use std::{collections::HashSet, env};

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

pub async fn resolve_usernames(usernames: impl Iterator<Item = &str>) -> Result<Vec<u64>, Error> {
    let my_steamid = env::var("USER_STEAM_ID")
        .expect("env var USER_STEAM_ID must be set in order to resolve usernames directly")
        .parse::<u64>()
        .expect("USER_STEAM_ID needs to be a valid u64");
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
    return Ok(steamids);
}

pub async fn find_friends_who_own_game(gameid: &u64) -> Result<Vec<client::UserSummary>, Error> {
    let my_steamid = env::var("USER_STEAM_ID")
        .expect("env var USER_STEAM_ID must be set in order to resolve usernames directly")
        .parse::<u64>()
        .expect("USER_STEAM_ID needs to be a valid u64");
    let friends =
        client::get_user_friends_list(client::GetUserDetailsRequest { id: my_steamid }).await?;

    eprint!("friends list{:?}", friends);

    let mut friends_with_game_ids: Vec<u64> = vec![];

    for friend in friends
        .iter()
        .map(|f| f.steamid.parse::<u64>().unwrap())
        .chain(std::iter::once(my_steamid))
    {
        eprintln!("checking against friend: {:?}", friend);
        let games_own_by_player =
            client::get_owned_games(client::GetUserDetailsRequest { id: friend }).await?;
        if games_own_by_player.iter().any(|game| &game.appid == gameid) {
            friends_with_game_ids.push(friend);
        }
    }

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
