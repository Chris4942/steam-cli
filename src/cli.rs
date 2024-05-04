mod steam;
use std::env;

use steam::router;
use tokio::runtime;

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
    match rt.block_on(router::run_command(args.into_iter(), user_steam_id)) {
        Ok(s) => println!("{}", s),
        Err(s) => eprintln!("{}", s),
    }
}

fn get_blocking_runtime() -> runtime::Runtime {
    runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("tokio is borked")
}
