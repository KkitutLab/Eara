use chrono::Utc;
use serenity::all::CommandInteraction;
use serenity::prelude::Context;

use crate::utils::{interaction_embed_say, lng_trs};

pub async fn handle_ping(ctx: &Context, command: &CommandInteraction, lng: bool) {
    let ts = command.id.created_at();
    let elapsed_ms = Utc::now().timestamp_millis() - ts.timestamp_millis();
    let _ = interaction_embed_say(
        &ctx.http,
        command,
        lng_trs(lng, "📡 Delay", "📡 지연"),
        &format!("{} ms", elapsed_ms),
        0xed95c1,
        false,
    )
    .await;
}
