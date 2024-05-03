use std::env;

use serde_json;
use serenity::all::{GuildId, Ready};
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
            match router::run_command(args.into_iter()) {
                Ok(result) => {
                    if let Err(why) = msg.channel_id.say(&ctx.http, result).await {
                        println!("Error sending message: {why:?}")
                    }
                }
                Err(result) => {
                    if let Err(why) = msg.channel_id.say(&ctx.http, result).await {
                        println!("Error sending message: {why:?}")
                    }
                }
            }
        }

        if msg.content == "!ping" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message: {why:?}");
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        println!("{}", serde_json::to_string_pretty(&ready).unwrap());

        let guild_id = GuildId::new(1127646508905406527);
        // let commands = GuildId::set_application_command(&guild_id, &ctx.http, |command| {});
        println!("{}", guild_id);
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
