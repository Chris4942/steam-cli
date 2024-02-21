use std::env;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::models::Game;



const BASE_URL: &str = "http://api.steampowered.com";
#[derive(Debug)]
pub struct GetOwnedGamesRequest {
    pub id: u64,
}

pub async fn get_owned_games(request: GetOwnedGamesRequest) -> Result<Vec<Game>, Error> {
    println!("running get_owned_games_async for {:?}", request);

    let url = format!(
        "{base}/IPlayerService/GetOwnedGames/v0001/",
        base = BASE_URL
    );

    let params = [
        (
            "key",
            env::var("STEAM_API_KEY").expect("STEAM_API_KEY not set"),
        ),
        ("steamId", request.id.to_string()),
        ("format", "json".to_string()),
        ("include_appinfo", "true".to_string()),
        ("include_played_free_games", "false".to_string()),
        ("appids_filter", "false".to_string()),
        ("language", "EN".to_string()),
        ("inclde_extended_app_info", "false".to_string()),
    ];

    let client = Client::new();

    let response = client.get(url).query(&params).send().await?;

    if response.status().is_success() {
        let body = response.text().await.expect("failed to parse body");
        let parse_body: serde_json::Value = serde_json::from_str(&body)?;
        if let Some(games_array) = parse_body["response"]["games"].as_array() {
            return Ok(serde_json::from_value(serde_json::Value::Array(
                games_array.to_owned(),
            ))?);
        }
        return Err(Error::JsonMissingValueError);
    }
    println!("failed with status code: {}", response.status());
    return Err(Error::HttpStatusError(response.status().as_u16()));
}

#[derive(Serialize, Deserialize)]
pub struct GetAvailableEndpointsResponse {
    pub apilist: ApiList,
}

#[derive(Serialize, Deserialize)]
pub struct ApiList {
    pub interfaces: Vec<SteamEndpoint>,
}

#[derive(Serialize, Deserialize)]
pub struct SteamEndpoint {
    pub name: String,
    methods: Vec<SteamMethod>,
}

#[derive(Serialize, Deserialize)]
pub struct SteamMethod {
    name: String,
    version: i32,
    httpmethod: String,
    parameters: Vec<SteamMethodParameter>,
}

#[derive(Serialize, Deserialize)]
pub struct SteamMethodParameter {
    name: String,
    #[serde(rename = "type")]
    parameter_type: String,
    optional: bool,
}

#[derive(Debug)]
pub enum Error {
    JsonError(serde_json::Error),
    JsonMissingValueError,
    HttpError(reqwest::Error),
    HttpStatusError(u16),
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::JsonError(value)
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::HttpError(value)
    }
}
