use std::{
    borrow::Borrow,
    env::{self, VarError},
    fmt::Display,
};

use reqwest;
use serde::{Deserialize, Serialize};

use super::{logger::FilteringLogger, models::Game};
use backoff::ExponentialBackoff;

const BASE_URL: &str = "http://api.steampowered.com";

pub async fn get_owned_games<'a>(
    request: GetUserDetailsRequest,
    logger: &'a FilteringLogger<'a>,
) -> Result<Vec<Game>, Error> {
    let url = format!(
        "{base}/IPlayerService/GetOwnedGames/v0001/",
        base = BASE_URL
    );
    let url_slice = &url[..];

    let params = [
        ("key", env::var("STEAM_API_KEY")?),
        ("steamId", request.id.to_string()),
        ("format", "json".to_string()),
        ("include_appinfo", "true".to_string()),
        ("include_played_free_games", "false".to_string()),
        ("appids_filter", "false".to_string()),
        ("language", "EN".to_string()),
        ("inclde_extended_app_info", "false".to_string()),
    ];

    let client = reqwest::Client::new();
    let response = backoff::future::retry(ExponentialBackoff::default(), || async {
        let response = match client.get(url_slice).query(&params).send().await {
            Ok(res) => res,
            Err(_) => {
                return Err(backoff::Error::Transient {
                    err: 0,
                    retry_after: None,
                })
            }
        };
        if response.status().is_success() {
            return Ok(response);
        }
        if response.status().as_u16() == 429 {
            logger.trace(format!("retyring for {} due to 429", request.id));
            return Err(backoff::Error::Transient {
                err: 429,
                retry_after: None,
            });
        }
        Err(backoff::Error::Permanent(response.status().as_u16()))
    })
    .await?;

    if response.status().is_success() {
        let body = response.text().await?;
        let parse_body: serde_json::Value = serde_json::from_str(&body)?;
        if !parse_body["response"]
            .as_object()
            .unwrap()
            .contains_key("games")
        {
            return Ok(vec![]);
        }
        if let Some(games_array) = parse_body["response"]["games"].as_array() {
            return Ok(serde_json::from_value(serde_json::Value::Array(
                games_array.to_owned(),
            ))?);
        }
        return Err(Error::JsonMissingValue);
    }
    Err(Error::HttpStatus(response.status().as_u16()))
}

pub async fn get_available_endpoints() -> Result<GetAvailableEndpointsResponse, Error> {
    let params = [("key", env::var("STEAM_API_KEY")?)];

    let url = format!(
        "{base}/ISteamWebAPIUtil/GetSupportedAPIList/v0001/",
        base = BASE_URL
    );

    let client = reqwest::Client::new();

    let response = client.get(url).query(&params).send().await?;

    if response.status().is_success() {
        let body = response.text().await?;
        let parse_body: GetAvailableEndpointsResponse = serde_json::from_str(&body)?;
        return Ok(parse_body);
    }

    Err(Error::HttpStatus(response.status().as_u16()))
}

#[derive(Serialize, Deserialize)]
pub struct GetAvailableEndpointsResponse {
    pub apilist: ApiList,
}

pub async fn get_user_friends_list(request: GetUserDetailsRequest) -> Result<Vec<Friend>, Error> {
    let user = request.id;
    eprintln!("getting user friends for user: {user}");

    let params = [
        ("key", env::var("STEAM_API_KEY")?),
        ("steamid", user.to_string()),
    ];

    let url = format!("{base}/ISteamUser/GetFriendList/v0001/", base = BASE_URL);

    let client = reqwest::Client::new();
    let response = client.get(url).query(&params).send().await?;

    if response.status().is_success() {
        let body = response.text().await?;
        let parse_body: serde_json::Value = serde_json::from_str(&body)?;
        if let Some(friends) = parse_body["friendslist"]["friends"].as_array() {
            return Ok(serde_json::from_value(serde_json::Value::Array(
                friends.to_owned(),
            ))?);
        }
        return Err(Error::JsonMissingValue);
    }

    Err(Error::HttpStatus(response.status().into()))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Friend {
    pub steamid: String,
}

pub async fn get_user_summaries(
    request: GetUserSummariesRequest,
) -> Result<Vec<UserSummary>, Error> {
    let users = request.ids;
    eprintln!("getting player summary for users: {:?}", users);

    let params = [
        ("key", env::var("STEAM_API_KEY")?),
        (
            "steamids",
            users.iter().fold(String::new(), |aggregate, id| {
                aggregate + "," + id.to_string().borrow()
            }),
        ),
    ];

    let url = format!(
        "{base}/ISteamUser/GetPlayerSummaries/v0002/",
        base = BASE_URL
    );

    let client = reqwest::Client::new();
    let response = client.get(url).query(&params).send().await?;

    if response.status().is_success() {
        let body = response.text().await?;
        let parse_body: serde_json::Value = serde_json::from_str(&body)?;
        if let Some(players) = parse_body["response"]["players"].as_array() {
            return Ok(serde_json::from_value(serde_json::Value::Array(
                players.to_owned(),
            ))?);
        }
        return Err(Error::JsonMissingValue);
    }

    Err(Error::HttpStatus(response.status().into()))
}

#[derive(Debug)]
pub struct GetUserSummariesRequest {
    pub ids: Vec<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserSummary {
    pub steamid: String,
    pub personaname: String,
    pub realname: Option<String>,
}

#[derive(Debug)]
pub struct GetUserDetailsRequest {
    pub id: u64,
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
    Json(serde_json::Error),
    JsonMissingValue,
    Http(reqwest::Error),
    HttpStatus(u16),
    MissingApiKey(VarError),
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::Http(value)
    }
}

impl From<VarError> for Error {
    fn from(value: VarError) -> Self {
        Self::MissingApiKey(value)
    }
}

impl From<u16> for Error {
    fn from(value: u16) -> Self {
        Error::HttpStatus(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Json(err) => write!(f, "JsonError({})", err),
            Error::JsonMissingValue => write!(f, "JsonMissingValueError"),
            Error::Http(err) => write!(f, "HttpError({})", err),
            Error::HttpStatus(err) => write!(f, "HttpStatusError({})", err),
            Error::MissingApiKey(err) => write!(f, "MissingApiKey({})", err),
        }
    }
}
