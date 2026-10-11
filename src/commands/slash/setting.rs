use serenity::all::{Channel, CommandInteraction, EditChannel, GuildId};
use serenity::prelude::Context;

use crate::handler::Handler;
use crate::utils::{
    extract_voice_options, followup_embed_error, followup_embed_say, get_bitrate_limit, lng_trs,
};

pub async fn handle_setting(
    handler: &Handler,
    ctx: &Context,
    command: &CommandInteraction,
    guild_id: GuildId,
    lng: bool,
) {
    let (name, user_limit, bitrate) = extract_voice_options(&command.data.options);

    if name.is_none() && user_limit.is_none() && bitrate.is_none() {
        let _ = followup_embed_say(
            &ctx.http,
            command,
            lng_trs(lng, "❕ Settings missing..", "❕ 설정어디갔어.."),
            lng_trs(lng, "No value was set!!", "아무런 값도 설정되지 않았어요!!"),
            0x91275c,
        )
        .await;
        return;
    }

    let current_channel_id = ctx.cache.guild(guild_id).and_then(|guild| {
        guild
            .voice_states
            .get(&command.user.id)
            .and_then(|voice_state| voice_state.channel_id)
    });

    let Some(current_channel_id) = current_channel_id else {
        let _ = followup_embed_say(
            &ctx.http,
            command,
            lng_trs(lng, "❕ Can't hear voices", "❕ 목소리가 안 들려요"),
            lng_trs(
                lng,
                "You're not in a voice channel!!",
                "음성 채팅에 속해있지 않아요!!",
            ),
            0x91275c,
        )
        .await;
        return;
    };

    let entries = handler.get_voice_entries(guild_id.get()).await;
    let matching_channel = entries
        .iter()
        .find(|(tc_id, _, _)| *tc_id == current_channel_id.get());

    let Some(&(_tc_id, user_id, _)) = matching_channel else {
        let _ = followup_embed_say(
            &ctx.http,
            command,
            lng_trs(lng, "❕ There's no one here..", "❕ 아무도 없잖아.."),
            lng_trs(
                lng,
                "No one belongs in the temporary voice chat!!",
                "그 누구도 임시 음성 채팅에 속해있지 않아요!!",
            ),
            0x91275c,
        )
        .await;
        return;
    };

    if user_id != command.user.id.get() {
        let _ = followup_embed_say(
            &ctx.http,
            command,
            lng_trs(lng, "❕ No key for you", "❕ 열쇠가 없대요"),
            lng_trs(
                lng,
                "You don't have permission to change channel settings!!",
                "체널 설정을 바꿀 권한이 없어요!!",
            ),
            0x91275c,
        )
        .await;
        return;
    }

    let voice_channel = current_channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|channel| {
            if let Channel::Guild(vc) = channel {
                Some(vc)
            } else {
                None
            }
        });

    let Some(mut voice_channel) = voice_channel else {
        let _ = followup_embed_say(
            &ctx.http,
            command,
            lng_trs(lng, "❕ Where My room", "❕ 내방 어디갔어"),
            lng_trs(lng, "aah?? Channel Not Found!!", "어라?? 체널이 없어요!!"),
            0x91275c,
        )
        .await;
        return;
    };

    let mut edits = EditChannel::new();

    if let Some(ref name) = name {
        edits = edits.name(name);
    }
    if let Some(user_limit) = user_limit {
        edits = edits.user_limit(user_limit as u32);
    }
    if let Some(bitrate) = bitrate {
        match guild_id.to_partial_guild(&ctx.http).await {
            Ok(partial_guild) => {
                let bitrate_limit = get_bitrate_limit(partial_guild.premium_tier);
                let new_bitrate = (bitrate as u16).min(bitrate_limit);
                edits = edits.bitrate(new_bitrate as u32 * 1000);
            }
            Err(e) => {
                eprintln!("[ERROR] guild_id.to_partial_guild(): {:?}", e);
                edits = edits.bitrate(96000);
            }
        }
    }

    if let Err(e) = voice_channel.edit(&ctx.http, edits).await {
        eprintln!("[ERROR] voice_channel.edit(): {:?}", e);
        let _ = followup_embed_error(
            &ctx.http,
            command,
            lng,
            lng_trs(
                lng,
                "An error occurred during setting..",
                "설정중에 오류가 났어요..",
            ),
        )
        .await;
    } else {
        let _ = followup_embed_say(
            &ctx.http,
            command,
            lng_trs(lng, "☑️ OK", "☑️ 완료"),
            lng_trs(lng, "set normally!", "정상적으로 설정하였어요!"),
            0xed95c1,
        )
        .await;
    }
}
