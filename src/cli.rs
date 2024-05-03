mod steam;
use steam::router;

fn main() {
    match router::run_command() {
        Ok(s) => println!("{}", s),
        Err(s) => eprintln!("{}", s),
    }
}
