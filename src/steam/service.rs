use super::{
    client::{self, GetUserSummariesRequest, UserSummary},
    logger::FilteringLogger,
};
use futures::{future::join_all, join};
use std::{collections::HashSet, fmt::Display, num::ParseIntError};

use super::models::Game;

pub async fn find_games_in_common<'a>(
    steam_ids: Vec<u64>,
    logger: &'a FilteringLogger<'a>,
) -> Result<HashSet<Game>, Error> {
    let mut games_set = HashSet::<Game>::new();

    let query_results = join_all(
        steam_ids
            .into_iter()
            .map(|id| client::get_owned_games(client::GetUserDetailsRequest { id }, logger)),
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

pub async fn games_missing_from_group<'a>(
    focus_steam_id: u64,
    other_steam_ids: Vec<u64>,
    logger: &'a FilteringLogger<'a>,
) -> Result<HashSet<Game>, Error> {
    println!("finding games missing from group");
    let result = join!(
        client::get_owned_games(client::GetUserDetailsRequest { id: focus_steam_id }, logger),
        find_games_in_common(other_steam_ids, logger)
    );
    let mut games_in_common_minus_focus = result.1?;

    for game in result.0? {
        games_in_common_minus_focus.remove(&game);
    }
    Ok(games_in_common_minus_focus)
}

pub async fn resolve_usernames_strictly<'a>(
    usernames: impl Iterator<Item = &str>,
    my_steamid: u64,
    logger: &'a FilteringLogger<'a>,
) -> Result<Vec<u64>, Error> {
    resolve_username_with_mapping_function(
        usernames,
        my_steamid,
        |username, user_summaries| {
            user_summaries
                .iter()
                .find(|user| {
                    user.personaname.to_ascii_lowercase() == *username.to_ascii_lowercase()
                })
                .ok_or(Error::User("supplied user not in list".to_string()))
        },
        logger,
    )
    .await
}

pub async fn resolve_usernames_fuzzily<'a>(
    usernames: impl Iterator<Item = &str>,
    my_steamid: u64,
    threshold: u32,
    logger: &'a FilteringLogger<'a>,
) -> Result<Vec<u64>, Error> {
    // TODO: this involves a lot of unnecessaries recomputations around user_summaries that should
    // be removed
    resolve_username_with_mapping_function(
        usernames,
        my_steamid,
        |username, user_summaries| {
            let usernames = user_summaries
                .iter()
                .map(|summary| &summary.personaname)
                .collect::<Vec<_>>();
            let matches = nucleo_matcher::pattern::Pattern::parse(
                username,
                nucleo_matcher::pattern::CaseMatching::Ignore,
                nucleo_matcher::pattern::Normalization::Smart,
            )
            .match_list_with_index(
                usernames.clone(),
                &mut nucleo_matcher::Matcher::new(nucleo_matcher::Config::DEFAULT),
            );

            if !matches.is_empty() {
                let (_, score, index) = matches[0];

                if score > threshold {
                    return Ok(&user_summaries[index]);
                }
            }

            let realnames_indexed = user_summaries
                .iter()
                .enumerate()
                .filter_map(|(index, summary)| {
                    summary.realname.as_ref().map(|realname| (index, realname))
                })
                .collect::<Vec<_>>();

            let realnames = realnames_indexed
                .iter()
                .map(|(_, realname)| realname)
                .collect::<Vec<_>>();

            let matches = nucleo_matcher::pattern::Pattern::parse(
                username,
                nucleo_matcher::pattern::CaseMatching::Ignore,
                nucleo_matcher::pattern::Normalization::Smart,
            )
            .match_list_with_index(
                realnames,
                &mut nucleo_matcher::Matcher::new(nucleo_matcher::Config::DEFAULT),
            );

            if !matches.is_empty() {
                let (_, score, index) = matches[0];

                if score > threshold {
                    return Ok(&user_summaries[realnames_indexed[index].0]);
                }
            }

            Err(Error::User(format!("Could not match {username}")))
        },
        logger,
    )
    .await
}

pub async fn resolve_username_with_mapping_function<'b, F>(
    usernames: impl Iterator<Item = &str>,
    my_steamid: u64,
    mapping_function: F,
    logger: &'b FilteringLogger<'b>,
) -> Result<Vec<u64>, Error>
where
    F: for<'a> Fn(&str, &'a Vec<client::UserSummary>) -> Result<&'a UserSummary, Error>,
{
    let friends =
        client::get_user_friends_list(client::GetUserDetailsRequest { id: my_steamid }, logger)
            .await?;
    println!("got friends");
    let mut ids: Vec<u64> = friends
        .iter()
        .map(|friend| friend.steamid.parse::<u64>())
        .collect::<Result<Vec<u64>, ParseIntError>>()?;
    ids.push(my_steamid);
    let user_summaries =
        client::get_user_summaries(client::GetUserSummariesRequest { ids }).await?;
    let steamids: Vec<u64> = usernames
        .map(|username| mapping_function(username, &user_summaries))
        .collect::<Result<Vec<_>, Error>>()?
        .iter()
        .map(|user| user.steamid.parse::<u64>())
        .collect::<Result<Vec<u64>, ParseIntError>>()?;
    Ok(steamids)
}

pub async fn find_friends_who_own_game<'a>(
    appid: &u64,
    my_steamid: u64,
    logger: &'a FilteringLogger<'a>,
) -> Result<Vec<client::UserSummary>, Error> {
    let friends =
        client::get_user_friends_list(client::GetUserDetailsRequest { id: my_steamid }, logger)
            .await?;

    let steamids_iterator = friends
        .iter()
        .map(|friend| friend.steamid.parse::<u64>().unwrap())
        .chain(std::iter::once(my_steamid));

    let player_owned_games = join_all(
        steamids_iterator
            .clone() // We need to use this iterator again later so we can't move it here
            .map(|id| (client::get_owned_games(client::GetUserDetailsRequest { id }, logger)))
            .collect::<Vec<_>>(),
    )
    .await;

    let mut errors = vec![];

    let friends_with_game_ids = player_owned_games
        .into_iter()
        .zip(steamids_iterator)
        .filter_map(|(result, steam_id)| match result {
            Ok(v) => Some((v, steam_id)),
            Err(err) => {
                errors.push(err);
                None
            }
        })
        .filter(|(games, _)| games.iter().any(|game| &game.appid == appid))
        .map(|(_, steamid)| steamid)
        .collect::<Vec<u64>>();

    let user_summaries = client::get_user_summaries(GetUserSummariesRequest {
        ids: friends_with_game_ids,
    })
    .await?;

    if !errors.is_empty() {
        logger
            .trace(format!("filtered out some results due to {:?}", errors))
            .await;
    }

    Ok(user_summaries)
}

#[derive(Debug)]
pub enum Error {
    Client(client::Error),
    Parse(ParseIntError),
    User(String),
}

impl From<client::Error> for Error {
    fn from(value: client::Error) -> Self {
        Error::Client(value)
    }
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Error::Parse(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Client(value) => write!(f, "ClientError: {}", value),
            Error::Parse(value) => write!(f, "ParseError: {}", value),
            Error::User(value) => write!(f, "UserError: {}", value),
        }
    }
}
