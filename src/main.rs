use clap::{command, value_parser, Arg, Command};

mod games_service;

fn main() {
    let matches = command!()
        .version("0.0")
        .author("Chris West")
        .about("Some utility functions for use with steam")
        .subcommand(
            Command::new("games-in-common")
                .alias("gic")
                .arg(Arg::new("steam_ids")
                .num_args(1..)
                .value_parser(value_parser!(u32))
            )
        )
        .get_matches();
    match matches.subcommand() {
        Some(("games-in-common", arguments)) => {
            games_service::find_games_in_common(arguments);
        }
        None => {
            println!("got nothing");
        }
        _ => unreachable!(),
    }
}
