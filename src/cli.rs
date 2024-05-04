mod steam;
use std::env;

use steam::router;
use tokio::runtime;

fn main() {
    let args = env::args_os()
        .map(|s| s.into_string().unwrap())
        .collect::<Vec<_>>();
    let rt = get_blocking_runtime();
    match rt.block_on(router::run_command(args.into_iter())) {
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
