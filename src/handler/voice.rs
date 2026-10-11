use chrono::Utc;
use serenity::all::{Channel, ChannelId, ChannelType, CreateChannel};
use serenity::model::prelude::VoiceState;
use serenity::prelude::Context;
use sqlx::MySqlPool;

use crate::db::{LogConfig, LogMode, VoiceEventType, get_guild_voice_settings, get_voice_defaults};
use crate::handler::Handler;
use crate::macros::sr;
use crate::utils::{embed_error, embed_voice, get_bitrate_limit};

pub async fn handle_voice_state_update(
    handler: &Handler,
    ctx: &Context,
    old: &Option<VoiceState>,
    new: &VoiceState,
    pool: &MySqlPool,
) {
    let guild_id = sr!(new.guild_id);
    // t=time u=userid c=channelid o=oldchannelid
    let mut ctx_map = std::collections::HashMap::new();
    let user_id = new.user_id;
    let mut log_id: Option<u64> = None;

    let settings = match get_guild_voice_settings(pool, guild_id.get()).await {
        Ok(Some(s)) => s,
        Ok(None) => return,
        Err(e) => {
            eprintln!("[ERROR] get_guild_voice_settings: {:?}", e);
            return;
        }
    };

    if let (true, Some(id)) = (settings.log_enable, settings.log_channel) {
        log_id = Some(id);
        ctx_map.insert("t", Utc::now().timestamp().to_string());
        ctx_map.insert("u", user_id.get().to_string());
    }

    let mode_channel_list = settings.mode_channel_list;
    let add_on = settings.add_enable;
    let mode = settings.log_mode;

    let is_ignored = |channel_id: Option<ChannelId>| -> bool {
        match mode {
            LogMode::None => true,
            LogMode::BlackList => channel_id.is_some_and(|c| mode_channel_list.contains(&c.get())),
            LogMode::WhiteList => channel_id.is_none_or(|c| !mode_channel_list.contains(&c.get())),
        }
    };

    let old_channel = old.as_ref().and_then(|v| v.channel_id);
    let new_channel = new.channel_id;

    let mut is_old_blacklist = false;
    let mut is_new_blacklist = false;

    let mut event_type = match (old_channel, new_channel) {
        (None, Some(new_id)) => {
            ctx_map.insert("c", new_id.to_string());
            is_new_blacklist = is_ignored(new_channel);
            VoiceEventType::Join
        }
        (Some(old_id), None) => {
            ctx_map.insert("o", old_id.to_string());
            is_old_blacklist = is_ignored(old_channel);
            VoiceEventType::Leave
        }
        (Some(old_id), Some(new_id)) => {
            if old_id == new_id {
                return;
            }
            ctx_map.insert("c", new_id.to_string());
            ctx_map.insert("o", old_id.to_string());
            is_old_blacklist = is_ignored(old_channel);
            is_new_blacklist = is_ignored(new_channel);
            VoiceEventType::Move
        }
        _ => return,
    };

    let was_move = event_type == VoiceEventType::Move;
    if add_on {
        let add_channel = settings.add_channel;

        if let Some(add_channel) = add_channel {
            if matches!(new_channel, Some(ref ch) if ch.get() == add_channel) {
                event_type = VoiceEventType::Add;
            }

            if let Some(old_channel) = old_channel {
                if old_channel.get() == add_channel {
                    return;
                }

                for (tc_id, _, is_blacklist) in handler.get_voice_entries(guild_id.get()).await {
                    if tc_id != old_channel.get() {
                        continue;
                    }

                    let is_empty = match old_channel.to_channel(&ctx.http).await {
                        Ok(Channel::Guild(gc)) => {
                            gc.members(&ctx.cache).unwrap_or_default().is_empty()
                        }
                        _ => false,
                    };

                    if !is_empty {
                        continue;
                    }

                    if was_move {
                        if let Err(del_err) = old_channel.delete(&ctx.http).await {
                            eprintln!("[ERROR] add_channel.delete: {:?}", del_err);
                            if !is_old_blacklist {
                                let log_id = sr!(log_id);
                                let _ = embed_error(
                                    &ctx.http,
                                    &ChannelId::new(log_id),
                                    "유저가 없는 체널을 파괴하는 도중 오류가 발생했어요..",
                                )
                                .await;
                            }
                            return;
                        } else {
                            handler
                                .remove_voice_entry_by_channel(guild_id.get(), old_channel.get())
                                .await;
                            if is_blacklist {
                                is_old_blacklist = true;
                            } else if let Some(log_id) = log_id {
                                match VoiceEventType::Destroy
                                    .get_log_config(pool, guild_id.get())
                                    .await
                                {
                                    Ok(log_data) => {
                                        let default_cfg = LogConfig::default();
                                        let cfg = log_data.as_ref().unwrap_or(&default_cfg);
                                        if let Err(e) = embed_voice(
                                            &ctx.http,
                                            &ChannelId::new(log_id),
                                            VoiceEventType::Destroy,
                                            cfg,
                                            &ctx_map,
                                            &new.member,
                                        )
                                        .await
                                        {
                                            eprintln!("[ERROR] embed_voice: {:?}", e);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("[ERROR] log_type.get_log_config: {:?}", e);
                                        let _ = embed_error(
                                            &ctx.http,
                                            &ChannelId::new(log_id),
                                            "로그 작성을 위한 데이터베이스에 접근에 실패했어요..",
                                        )
                                        .await;
                                    }
                                }
                            }
                        }
                    } else {
                        event_type = VoiceEventType::Destroy;
                    }
                    break;
                }
            }
        }
    }

    match event_type {
        VoiceEventType::Add => {
            let mut channel_builder;
            match get_voice_defaults(pool, guild_id.get()).await {
                Ok(defaults) => {
                    channel_builder =
                        CreateChannel::new(defaults.name.unwrap_or("Temp".to_string()))
                            .kind(ChannelType::Voice)
                            .user_limit(defaults.user_limit.unwrap_or(0).into());
                    match guild_id.to_partial_guild(&ctx.http).await {
                        Ok(partial_guild) => {
                            let bitrate_limit = get_bitrate_limit(partial_guild.premium_tier);
                            let bitrate = defaults.bitrate.unwrap_or(96).min(bitrate_limit);
                            channel_builder = channel_builder.bitrate(bitrate as u32 * 1000);
                        }
                        Err(e) => {
                            eprintln!("[ERROR] guild_id.to_partial_guild(): {:?}", e);
                            channel_builder = channel_builder.bitrate(96000);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[ERROR] get_voice_defaults(): {:?}", e);
                    channel_builder = CreateChannel::new("Temp".to_string()).bitrate(96000);
                }
            }
            let new_channel = sr!(new_channel);
            match new_channel.to_channel(&ctx.http).await {
                Ok(Channel::Guild(gc)) => {
                    if let Some(parent_id) = gc.parent_id {
                        channel_builder = channel_builder.category(parent_id);
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("[ERROR] nc.to_channel(): {:?}", e);
                    if !is_new_blacklist {
                        if let Some(log_id) = log_id {
                            let _ = embed_error(
                                &ctx.http,
                                &ChannelId::new(log_id),
                                "체널 생성을 위한 탐색에 실패했어요..",
                            )
                            .await;
                        }
                        return;
                    }
                }
            }
            match guild_id.create_channel(&ctx.http, channel_builder).await {
                Ok(add_channel) => {
                    if let Err(e) = guild_id
                        .move_member(&ctx.http, new.user_id, add_channel.id)
                        .await
                    {
                        eprintln!("[ERROR] move_member(): {:?}", e);
                        if let (false, Some(log_id)) = (is_new_blacklist, log_id) {
                            let _ = embed_error(
                                &ctx.http,
                                &ChannelId::new(log_id),
                                "체널 생성 이후 유저 이동중 오류가 발생했어요..",
                            )
                            .await;
                        }

                        if let Err(del_err) = add_channel.delete(&ctx.http).await {
                            eprintln!("[ERROR] add_channel.delete: {:?}", del_err);
                            if !is_new_blacklist {
                                let log_id = sr!(log_id);
                                let _ = embed_error(
                                    &ctx.http,
                                    &ChannelId::new(log_id),
                                    "유저 이동 오류 이후 만든 체널을 파괴하는 도중 오류가 발생했어요..",
                                )
                                .await;
                            }
                        }
                        return;
                    }

                    handler
                        .add_voice_entry(
                            guild_id.get(),
                            add_channel.id.get(),
                            new.user_id.get(),
                            is_new_blacklist,
                        )
                        .await;
                }
                Err(e) => {
                    eprintln!("[ERROR] create_channel(): {:?}", e);
                    if !is_new_blacklist {
                        let log_id = sr!(log_id);
                        let _ = embed_error(
                            &ctx.http,
                            &ChannelId::new(log_id),
                            "체널 생성중 오류가 발생했어요..",
                        )
                        .await;
                    }
                }
            }

            if is_new_blacklist {
                if was_move {
                    event_type = VoiceEventType::Leave;
                } else {
                    return;
                }
            }
        }
        VoiceEventType::Destroy => {
            let old_channel = sr!(old_channel);
            if let Err(del_err) = old_channel.delete(&ctx.http).await {
                eprintln!("[ERROR] add_channel.delete: {:?}", del_err);
                if !is_old_blacklist {
                    let log_id = sr!(log_id);
                    let _ = embed_error(
                        &ctx.http,
                        &ChannelId::new(log_id),
                        "유저가 없는 체널을 파괴하는 도중 오류가 발생했어요..",
                    )
                    .await;
                }
                return;
            } else if let Some(true) = handler
                .remove_voice_entry_by_channel(guild_id.get(), old_channel.get())
                .await
            {
                return;
            }

            if is_old_blacklist {
                return;
            }
        }
        VoiceEventType::Join => {
            if is_new_blacklist {
                return;
            }

            let new_channel = sr!(new_channel);
            for (tc_id, _, is_blacklist) in handler.get_voice_entries(guild_id.get()).await {
                if new_channel.get() == tc_id && is_blacklist {
                    return;
                }
            }
        }
        VoiceEventType::Move => match (is_old_blacklist, is_new_blacklist) {
            (true, true) => return,
            (true, false) => event_type = VoiceEventType::Join,
            (false, true) => event_type = VoiceEventType::Leave,
            _ => {}
        },
        VoiceEventType::Leave => {
            if is_old_blacklist {
                return;
            }

            let old_channel = sr!(old_channel);
            for (tc_id, _, is_blacklist) in handler.get_voice_entries(guild_id.get()).await {
                if old_channel.get() == tc_id && is_blacklist {
                    return;
                }
            }
        }
    }

    let log_id = sr!(log_id);
    match event_type.get_log_config(pool, guild_id.get()).await {
        Ok(log_data) => {
            let default_cfg = LogConfig::default();
            let cfg = log_data.as_ref().unwrap_or(&default_cfg);
            if let Err(e) = embed_voice(
                &ctx.http,
                &ChannelId::new(log_id),
                event_type,
                cfg,
                &ctx_map,
                &new.member,
            )
            .await
            {
                eprintln!("[ERROR] embed_voice: {:?}", e);
            }
        }
        Err(e) => {
            eprintln!("[ERROR] log_type.get_log_config: {:?}", e);
            let _ = embed_error(
                &ctx.http,
                &ChannelId::new(log_id),
                "로그 작성을 위한 데이터베이스에 접근에 실패했어요..",
            )
            .await;
        }
    }
}
