use std::collections::HashSet;

// TODO: the arg_matcher, router and games_router files should all be moved into their own
// submodule
use clap::ArgMatches;

use crate::steam::service::games_missing_from_group;

use super::{
    logger::FilteringLogger,
    router::{compute_sorted_games_string, get_steam_ids, Error},
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
    let filtered_games = if arguments.get_flag("filter") {
        let filtered_games =
            filter_games(games.to_owned(), HashSet::from([27, 36, 38]), logger).await?;
        HashSet::from_iter(filtered_games.iter().cloned())
    } else {
        games
    };
    println!("accessed filter");
    Ok(compute_sorted_games_string(filtered_games))
}
