use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, Clone)] // Derive Deserialize and Serialize traits for Game
pub struct Game {
    pub name: String,
    appid: i128,
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Game {{ appid: {}, name: \"{}\" }}",
            self.appid, self.name
        )
    }
}
