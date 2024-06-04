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
            handle_steam_cli_request(&ctx, &msg).await;
        }
    }

    async fn ready(&self, _ctx: Context, _ready: Ready) {}
}

async fn handle_steam_cli_request(ctx: &Context, msg: &Message) {
    if let Err(err) = route_steam_cli_request(ctx, msg).await {
        eprintln!("{}", err);
    }
}

async fn route_steam_cli_request(ctx: &Context, msg: &Message) -> Result<(), Error> {
    let args = msg
        .content
        .split(' ')
        .map(|s| s.to_owned())
        .collect::<Vec<_>>();

    let send_message = |message: String| send_message(ctx, msg, message);

    // TODO: I should definitely be able to replace this `match` clause with a `?`, so I
    // should do that sometime
    match steam::router::route_arguments(
        args.into_iter(),
        Some(env::var("USER_STEAM_ID")?.parse::<u64>()?),
        send_message,
        send_message,
    )
    .await
    {
        Ok(()) => Ok(()),
        Err(e) => Err(Error::Execution(e)),
    }
}

async fn send_message(ctx: &Context, msg: &Message, message: String) {
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

#[derive(Debug)]
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
