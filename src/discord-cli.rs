use std::env::{self, VarError};
use std::fmt::Display;
use std::num::ParseIntError;

use serenity::all::Ready;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;

mod steam;
use steam::router;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content.starts_with("steam-cli") {
            let response = get_steam_cli_response(&ctx, &msg).await;
            send_message(ctx, msg, response).await;
        }
    }

    async fn ready(&self, _ctx: Context, _ready: Ready) {}
}

async fn get_steam_cli_response<'a>(_ctx: &Context, msg: &Message) -> String {
    let args = msg
        .content
        .split(' ')
        .map(|s| s.to_owned())
        .collect::<Vec<_>>();
    let result = execute_steam_command(args).await;

    match result {
        Ok(result) => result,
        Err(result) => result.to_string(),
    }
}

async fn execute_steam_command<'a>(args: Vec<String>) -> Result<String, Error> {
    Ok(router::run_command(
        args.into_iter(),
        Some(env::var("USER_STEAM_ID")?.parse::<u64>()?),
    )
    .await?)
}

async fn send_message(ctx: Context, msg: Message, message: String) {
    if let Err(why) = msg
        .channel_id
        .say(&ctx.http, format!("```\n{message}\n```", message = message))
        .await
    {
        println!("Error sending message: {why:?}")
    }
}

#[tokio::main]
async fn main() {
    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot.
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Start listening for events by starting a single shard
    println!("starting the client");
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

enum Error {
    EnvVarMissing(VarError),
    Parse(ParseIntError),
    Execution(router::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Error::EnvVarMissing(err) => write!(f, "EnvVarMissing: {}", err),
            Error::Parse(err) => write!(f, "Parse: {}", err),
            Error::Execution(err) => write!(f, "{}", err),
        }
    }
}

impl From<VarError> for Error {
    fn from(value: VarError) -> Self {
        Error::EnvVarMissing(value)
    }
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Error::Parse(value)
    }
}

impl From<router::Error> for Error {
    fn from(value: router::Error) -> Self {
        Error::Execution(value)
    }
}
