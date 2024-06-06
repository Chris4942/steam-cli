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
    rt.block_on(router::route_arguments(
        args.into_iter(),
        user_steam_id,
        println_async,
        eprintln_async,
    ))
    .unwrap(); // If the command fails when running in cli, just blow up; it's fine
}

fn get_blocking_runtime() -> runtime::Runtime {
    runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("tokio is borked")
}

async fn println_async(str: String) {
    println!("{}", str);
}

async fn eprintln_async(str: String) {
    eprintln!("{}", str);
}
