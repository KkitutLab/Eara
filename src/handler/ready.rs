use serenity::all::{
    ActivityData, CreateEmbed, CreateEmbedFooter, CreateMessage, OnlineStatus, Ready,
};
use serenity::prelude::Context;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::commands::slash::register_global_commands;
use crate::constants::CHANNELID_LOG_SECRET;
use crate::tasks::spawn_deploy_watch_task;

pub async fn handle_ready(ctx: &Context, ready: Ready, is_ready_flag: &Arc<AtomicBool>) {
    let is_ready = is_ready_flag.load(Ordering::Relaxed);

    let version_line = format!(
        "v{} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_DESCRIBE")
    );
    println!("Eara {version_line}");

    let embed_on;
    if is_ready {
        #[cfg(debug_assertions)]
        {
            println!("{} ?Reconnected", ready.user.name);
            embed_on = CreateEmbed::new()
                .title("?Reconnected")
                .footer(CreateEmbedFooter::new(version_line))
                .color(0x99657f);
        }
        #[cfg(not(debug_assertions))]
        {
            println!("{} !Reconnected", ready.user.name);
            embed_on = CreateEmbed::new()
                .title("!Reconnected")
                .footer(CreateEmbedFooter::new(version_line))
                .color(0xff91c9);
        }
    } else {
        is_ready_flag.store(true, Ordering::Relaxed);

        #[cfg(debug_assertions)]
        {
            println!("{} ?On", ready.user.name);
            embed_on = CreateEmbed::new()
                .title("?On")
                .footer(CreateEmbedFooter::new(version_line))
                .color(0xba0962);
        }
        #[cfg(not(debug_assertions))]
        {
            println!("{} !On", ready.user.name);
            embed_on = CreateEmbed::new()
                .title("!On")
                .footer(CreateEmbedFooter::new(version_line))
                .color(0xf7c8e0);
        }

        register_global_commands(&ctx.http).await;

        spawn_deploy_watch_task(ctx.http.clone());
    }
    ctx.set_presence(
        Some(ActivityData::listening("음성 로그")),
        OnlineStatus::Online,
    );
    if let Some(channel_id) = *CHANNELID_LOG_SECRET {
        let builderon = CreateMessage::new().embed(embed_on);
        if let Err(e) = channel_id.send_message(&ctx.http, builderon).await {
            eprintln!("[ERROR] ON send_message(): {:?}", e);
        }
    }
}
