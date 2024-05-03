mod steam;
use std::env;

use steam::router;

fn main() {
    let args = env::args_os()
        .map(|s| s.into_string().unwrap())
        .collect::<Vec<_>>();
    match router::run_command(args.into_iter()) {
        Ok(s) => println!("{}", s),
        Err(s) => eprintln!("{}", s),
    }
}
