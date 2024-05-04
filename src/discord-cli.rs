use std::env;

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
            let args = msg
                .content
                .split(' ')
                .map(|s| s.to_owned())
                .collect::<Vec<_>>();
            match router::run_command(
                args.into_iter(),
                Some(
                    env::var("USER_STEAM_ID")
                        .expect("TODO, this shouldn't be hardcoded like this here")
                        .parse::<u64>()
                        .expect("TODO this shouldn't be hardcoded like this"),
                ),
            )
            .await
            {
                Ok(result) => send_message(ctx, msg, result).await,
                Err(result) => send_message(ctx, msg, result.to_string()).await,
            }
        }
    }

    async fn ready(&self, _ctx: Context, _ready: Ready) {}
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
