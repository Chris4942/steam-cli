use std::env::{self, VarError};
use std::fmt::Display;
use std::num::ParseIntError;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;

use serenity::all::Ready;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;

mod steam;
use steam::logger::Logger;
use steam::router;
mod util;
use util::{async_help::get_blocking_runtime, string_parser};

struct Handler {
    tx: Sender<DiscordMessage>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let msg = Arc::new(msg);
        let logger = DiscordLogger {
            ctx: Arc::new(ctx),
            msg: msg.clone(),
            tx: self.tx.clone(),
        };
        if msg.as_ref().content.starts_with("steam-cli") {
            handle_steam_cli_request(&msg, logger).await;
        }
    }

    async fn ready(&self, _ctx: Context, _ready: Ready) {}
}

async fn handle_steam_cli_request(msg: &Message, logger: DiscordLogger) {
    if let Err(err) = route_steam_cli_request(msg, logger).await {
        eprintln!("{}", err);
    }
}

async fn route_steam_cli_request<'a>(msg: &Message, logger: DiscordLogger) -> Result<(), Error> {
    let args = msg
        .content
        .split(' ')
        .map(|s| s.to_owned())
        .collect::<Vec<_>>();

    steam::router::route_arguments(
        args,
        Some(env::var("USER_STEAM_ID")?.parse::<u64>()?),
        &logger,
    )
    .await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let (tx, rx) = channel::<DiscordMessage>();
    thread::spawn(move || {
        eprintln!("logging thread started");
        let rt = get_blocking_runtime();
        let mut errors_since_success = 0;
        loop {
            eprintln!("awaiting message...");
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
            if let Err(err) = rt.block_on(send_message(log)) {
                eprintln!("failed to send message due to {:?}", err);
            }
        }
    });

    // Create a new instance of the Client, logging in as a bot.
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler { tx })
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

async fn send_message<'a>(discord_message: DiscordMessage) -> Result<(), DiscordSendError> {
    // NOTE: I don't know what the actual size limit is on discord messages. There webiste says
    // 4000 chars; however, it doesn't end up working for me at that size, but 2000 - 8 generally
    // seems to work. The 8 comes from the 4 markdown characters that are used for formatting
    let batches = string_parser::batch_string(&discord_message.message, 2000 - 8, '\n')?;
    for batch in batches {
        discord_message
            .msg
            .channel_id
            .say(
                &discord_message.ctx.http,
                format!("```\n{message}\n```", message = batch),
            )
            .await?;
    }
    Ok(())
}

#[derive(Debug)]
enum DiscordSendError {
    Batch(string_parser::Error),
    // NOTE: the value of Send(value) is never used directly, but it is used in logging
    #[allow(dead_code)]
    Send(serenity::Error),
}

impl From<string_parser::Error> for DiscordSendError {
    fn from(value: string_parser::Error) -> Self {
        DiscordSendError::Batch(value)
    }
}

impl From<serenity::Error> for DiscordSendError {
    fn from(value: serenity::Error) -> Self {
        DiscordSendError::Send(value)
    }
}

///msg and ctx are Arcs (Async Reference Counted) so that they can be safely passed between threads
///without deep copying
///
///https://doc.rust-lang.org/std/sync/struct.Arc.html
struct DiscordLogger {
    /// Arc so that it can be passed between threads
    msg: Arc<Message>,
    /// Arc so that it can be passed between threads
    ctx: Arc<Context>,
    tx: Sender<DiscordMessage>,
}

impl Logger for DiscordLogger {
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

impl DiscordLogger {
    fn create_discord_message(&self, message: String) -> DiscordMessage {
        DiscordMessage {
            // NOTE: these calls to clone do not do a deep copy. Since these are Arcs, they just
            // copy a pointer to the data instead and increment a counter. See the docs for more
            // info
            // https://doc.rust-lang.org/std/sync/struct.Arc.html
            msg: self.msg.clone(),
            ctx: self.ctx.clone(),
            message,
        }
    }
}

struct DiscordMessage {
    msg: Arc<Message>,
    ctx: Arc<Context>,
    message: String,
}
