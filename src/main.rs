mod commands;
mod constants;
mod db;
mod handler;
mod macros;
mod memory;
mod utils;

#[cfg(test)]
mod tests;

use dotenvy::dotenv;
use serenity::all::GatewayIntents;
use serenity::prelude::Client;
use std::env;

use crate::db::{DATABASE_URL, TOKEN, get_pool};
use crate::handler::Handler;

#[tokio::main]
async fn main() {
    dotenv().ok();

    if let Err(e) =
        TOKEN.set(env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set in .env file"))
    {
        eprintln!("[ERROR] TOKEN set(): {:?}", e);
        return;
    }

    if let Err(e) =
        DATABASE_URL.set(env::var("DATABASE_URL").expect("DATABASE_URL not set in .env file"))
    {
        eprintln!("[ERROR] DATABASE_URL set(): {:?}", e);
        return;
    }

    let intents = GatewayIntents::GUILD_VOICE_STATES | GatewayIntents::GUILDS;

    let _pool = get_pool().await.expect("[ERROR] Database connect()");

    let mut client = Client::builder(TOKEN.get().unwrap(), intents)
        .event_handler(Handler::new())
        .await
        .expect("[ERROR] Client builder()");

    if let Err(why) = client.start().await {
        eprintln!("[ERROR] Client start(): {why:?}");
    }
}
