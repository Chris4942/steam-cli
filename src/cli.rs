mod steam;
use std::env;

use steam::router;
mod util;
use util::async_help::get_blocking_runtime;

fn main() {
    let args = env::args_os()
        .map(|s| s.into_string().unwrap())
        .collect::<Vec<_>>();
    let user_steam_id: Option<u64> = match env::var("USER_STEAM_ID") {
        Ok(value) => Some(
            value
                .parse::<u64>()
                .expect("USER_STEAM_ID needs to be a u64"),
        ),
        Err(_) => None,
    };
    let rt = get_blocking_runtime();
    let _ = rt.block_on(router::route_arguments(args, user_steam_id, &StdLogger {}));
}

struct StdLogger {}

impl steam::logger::Logger for StdLogger {
    fn stdout(&self, str: String) {
        println!("{}", str)
    }

    fn stderr(&self, str: String) {
        eprintln!("{}", str)
    }
}
