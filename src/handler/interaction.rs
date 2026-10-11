use serenity::all::Interaction;
use serenity::prelude::Context;
use sqlx::MySqlPool;

use crate::commands::slash::dispatch_slash_command;
use crate::handler::Handler;

pub async fn handle_interaction(
    handler: &Handler,
    ctx: &Context,
    interaction: Interaction,
    pool: &MySqlPool,
) {
    if let Interaction::Command(command) = interaction {
        dispatch_slash_command(handler, ctx, &command, pool).await;
    }
}
