use std::env::{self, VarError};
use std::fmt::Display;
use std::num::ParseIntError;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::Duration;

use serenity::all::Ready;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;

mod steam;
use steam::logger::Logger;
use steam::router;
mod util;
use util::async_help::get_blocking_runtime;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let logger = build_logger(&ctx, &msg);
        if msg.content.starts_with("steam-cli") {
            handle_steam_cli_request(&ctx, &msg, logger).await;
        }
    }

    async fn ready(&self, _ctx: Context, _ready: Ready) {}
}

async fn handle_steam_cli_request<'a>(ctx: &Context, msg: &Message, logger: DiscordLogger<'a>) {
    if let Err(err) = route_steam_cli_request(ctx, msg, logger).await {
        eprintln!("{}", err);
    }
}

async fn route_steam_cli_request<'a>(
    _: &Context,
    msg: &Message,
    logger: DiscordLogger<'a>,
) -> Result<(), Error> {
    let args = msg
        .content
        .split(' ')
        .map(|s| s.to_owned())
        .collect::<Vec<_>>();

    // TODO: I should definitely be able to replace this `match` clause with a `?`, so I
    // should do that sometime
    match steam::router::route_arguments(
        args.into_iter(),
        Some(env::var("USER_STEAM_ID")?.parse::<u64>()?),
        &logger,
    )
    .await
    {
        Ok(()) => Ok(()),
        Err(e) => Err(Error::Execution(e)),
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

fn build_logger<'a>(ctx: &'a Context, msg: &'a Message) -> DiscordLogger<'a> {
    let (tx, rx) = channel::<DiscordMessage>();
    let logger = DiscordLogger { ctx, msg, tx };
    {
        thread::spawn(move || {
            let rt = get_blocking_runtime();
            let mut errors_since_success = 0;
            loop {
                let log = match rx.recv() {
                    Ok(log) => log,
                    Err(err) => {
                        errors_since_success += 1;
                        eprintln!(
                            "error received: {}\n\tErrors received since last success: {}",
                            err, errors_since_success
                        );
                        if errors_since_success > 3 {
                            eprintln!("error received: {}\nNot sure what's going on, so the thread looping thread is exiting", err);
                            return;
                        } else {
                            continue;
                        }
                    }
                };
                rt.block_on(send_message(log));
            }
        });
    }
    logger
}

async fn send_message<'a>(discord_message: DiscordMessage) {
    if let Err(why) = discord_message
        .msg
        .channel_id
        .say(
            &discord_message.ctx.http,
            format!("```\n{message}\n```", message = discord_message.message),
        )
        .await
    {
        println!("Error sending message: {why:?}")
    }
}

struct DiscordLogger<'a> {
    msg: &'a Message,
    ctx: &'a Context,
    tx: Sender<DiscordMessage>,
}

#[async_trait]
impl<'a> Logger for DiscordLogger<'a> {
    fn stdout(&self, str: String) {
        if let Err(err) = self.tx.send(self.create_discord_message(str.clone())) {
            eprintln!("Failed to send message {} to channel due to {}", str, err);
        }
    }

    fn stderr(&self, str: String) {
        if let Err(err) = self.tx.send(self.create_discord_message(str.clone())) {
            eprintln!("Failed to send message {} to channel due to {}", str, err);
        }
    }
}

impl<'a> DiscordLogger<'a> {
    fn create_discord_message(&self, message: String) -> DiscordMessage {
        // TODO: these clones might be unnecessary, but I'm not sure how to avoid them
        DiscordMessage {
            msg: self.msg.clone(),
            ctx: self.ctx.clone(),
            message,
        }
    }
}

struct DiscordMessage {
    msg: Message,
    ctx: Context,
    message: String,
}
