use once_cell::sync::Lazy;
use serenity::all::{
    Channel, ChannelId, CommandDataOption, CommandInteraction, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
    CreateMessage, GuildChannel, Http, Member, Permissions, PremiumTier,
};

use crate::db::{LogConfig, VoiceEventType};

static PLACEHOLDER_RE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"\{([a-z])\}").unwrap());

pub fn lng_trs<T>(is_korean: bool, en: T, ko: T) -> T {
    if is_korean { ko } else { en }
}

pub fn get_bitrate_limit(tier: PremiumTier) -> u16 {
    match tier {
        PremiumTier::Tier1 => 128,
        PremiumTier::Tier2 => 256,
        PremiumTier::Tier3 => 384,
        _ => 96,
    }
}

pub fn is_channel_manager(member: &Member) -> bool {
    member
        .permissions
        .is_some_and(|p| p.contains(Permissions::MANAGE_CHANNELS))
}

pub async fn defer(http: &Http, command: &CommandInteraction, ephemeral: bool) -> bool {
    let result = if ephemeral {
        command.defer_ephemeral(http).await
    } else {
        command.defer(http).await
    };

    match result {
        Ok(_) => true,
        Err(e) => {
            eprintln!("[ERROR] Command defer: {:?}", e);
            false
        }
    }
}

pub async fn interaction_embed_say(
    http: &Http,
    command: &CommandInteraction,
    title: &str,
    description: &str,
    color: u32,
    ephemeral: bool,
) -> bool {
    let embed = CreateEmbed::new()
        .title(title)
        .description(description)
        .color(color);

    if let Err(e) = command
        .create_response(
            http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(ephemeral),
            ),
        )
        .await
    {
        eprintln!("[ERROR] Command create_response(): {:?}", e);
        return false;
    }
    true
}

/*
pub async fn interaction_embed_error(
    http: &Http,
    command: &CommandInteraction,
    lng: bool,
    description: &str,
) -> bool {
    let embed = CreateEmbed::new()
        .title(lng_trs(lng, "❌ Error", "❌ 오류"))
        .description(description)
        .color(0xd93636);

    if let Err(e) = command
        .create_response(
            &http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(true),
            ),
        )
        .await
    {
        eprintln!("[ERROR] Command create_response(): {:?}", e);
        return false;
    }
    true
}
*/

pub async fn followup_embed_say(
    http: &Http,
    command: &CommandInteraction,
    title: &str,
    description: &str,
    color: u32,
) -> bool {
    let embed = CreateEmbed::new()
        .title(title)
        .description(description)
        .color(color);

    match command
        .create_followup(http, CreateInteractionResponseFollowup::new().embed(embed))
        .await
    {
        Ok(_) => true,
        Err(e) => {
            eprintln!("[ERROR] create_followup(): {:?}", e);
            false
        }
    }
}

pub async fn followup_embed_error(
    http: &Http,
    command: &CommandInteraction,
    lng: bool,
    description: &str,
) -> bool {
    let embed = CreateEmbed::new()
        .title(lng_trs(lng, "❌ Error", "❌ 오류"))
        .description(description)
        .color(0xd93636);

    match command
        .create_followup(http, CreateInteractionResponseFollowup::new().embed(embed))
        .await
    {
        Ok(_) => true,
        Err(e) => {
            eprintln!("[ERROR] create_followup(): {:?}", e);
            false
        }
    }
}

pub async fn followup_db_error(http: &Http, command: &CommandInteraction, lng: bool) -> bool {
    followup_embed_error(
        http,
        command,
        lng,
        lng_trs(
            lng,
            "There was a problem connecting to the database..",
            "데이터베이스 연결중 문제가 발생했어요..",
        ),
    )
    .await
}

/*
pub async fn embed_say(

    http: &Http,
    channel_id: &ChannelId,
    title: &str,
    description: &str,
    color: u32,
) -> bool {
    let embed = CreateEmbed::new()
        .title(title)
        .description(description)
        .color(color);

    match channel_id
        .send_message(&http, CreateMessage::new().embed(embed))
        .await
    {
        Ok(_) => true,
        Err(e) => {
            eprintln!("[ERROR] send_message(): {:?}", e);
            false
        }
    }
}
*/

pub async fn embed_error(http: &Http, channel_id: &ChannelId, description: &str) -> bool {
    let embed = CreateEmbed::new()
        .title("❌ 오류")
        .description(description)
        .color(0xd93636);

    match channel_id
        .send_message(http, CreateMessage::new().embed(embed))
        .await
    {
        Ok(_) => true,
        Err(e) => {
            eprintln!("[ERROR] send_message(): {:?}", e);
            false
        }
    }
}

pub async fn check_permission(
    http: &Http,
    member: &Member,
    command: &CommandInteraction,
    lng: bool,
) -> bool {
    if is_channel_manager(member) {
        true
    } else {
        let _ = followup_embed_error(
            http,
            command,
            lng,
            lng_trs(
                lng,
                "You don't have enough permission!",
                "당신의 권한이 부족해요!",
            ),
        )
        .await;
        false
    }
}

pub async fn validate_guild_channel(
    http: &Http,
    command: &CommandInteraction,
    channel_id: ChannelId,
    lng: bool,
) -> Option<GuildChannel> {
    match channel_id.to_channel(http).await {
        Ok(Channel::Guild(guild_channel)) => Some(guild_channel),
        _ => {
            let _ = followup_embed_error(
                http,
                command,
                lng,
                lng_trs(
                    lng,
                    "Invalid channel!\nDo I have permission to `View channel` to check its validity?",
                    "유효하지 않은 채널이에요!\n제가 유효성을 확인해 볼 `체널 보기` 권한은 있나요?",
                ),
            )
            .await;
            None
        }
    }
}

pub fn extract_voice_options(
    options: &[CommandDataOption],
) -> (Option<String>, Option<i64>, Option<i64>) {
    let name = options
        .iter()
        .find(|opt| opt.name == "name")
        .and_then(|opt| opt.value.as_str())
        .map(|s| s.to_string());
    let user_limit = options
        .iter()
        .find(|opt| opt.name == "user_limit")
        .and_then(|opt| opt.value.as_i64());
    let bitrate = options
        .iter()
        .find(|opt| opt.name == "bitrate")
        .and_then(|opt| opt.value.as_i64());
    (name, user_limit, bitrate)
}

pub fn format_placeholders(
    template: &str,
    context: &std::collections::HashMap<&str, String>,
) -> String {
    PLACEHOLDER_RE
        .replace_all(template, |caps: &regex::Captures| {
            let key = &caps[1];
            context
                .get(key)
                .cloned()
                .unwrap_or_else(|| caps[0].to_string())
        })
        .to_string()
}

pub struct LogConfigF {
    pub title: String,
    pub description: String,
    pub color: u32,
}

pub fn resolve_log_config(event: VoiceEventType, config: &LogConfig) -> LogConfigF {
    let (default_title, default_desc, default_color) = match event {
        VoiceEventType::Add => ("+ ADD", "### <#{c}> : <t:{t}:T>", 0xf7c8e0),
        VoiceEventType::Destroy => ("- DEL", "### <#{o}> : <t:{t}:T>", 0x330c20),
        VoiceEventType::Join => ("\\> ENT", "### <#{c}> : <t:{t}:T>", 0xff91c9),
        VoiceEventType::Move => ("= MOV", "### <#{o}> -> <#{c}> : <t:{t}:T>", 0x99657f),
        VoiceEventType::Leave => ("< OUT", "### <#{o}> : <t:{t}:T>", 0xba0962),
    };

    LogConfigF {
        title: config
            .title
            .clone()
            .unwrap_or_else(|| default_title.to_string()),
        description: config
            .description
            .clone()
            .unwrap_or_else(|| default_desc.to_string()),
        color: config.color.unwrap_or(default_color),
    }
}

pub async fn embed_voice(
    http: &Http,
    channel_id: &ChannelId,
    event: VoiceEventType,
    log_config: &LogConfig,
    context: &std::collections::HashMap<&str, String>,
    member: &Option<Member>,
) -> Result<(), Box<serenity::Error>> {
    let logs = resolve_log_config(event, log_config);

    let mut embed = CreateEmbed::new()
        .title(format_placeholders(&logs.title, context))
        .description(format_placeholders(&logs.description, context))
        .color(logs.color);

    if let Some(member) = member {
        let username = member
            .nick
            .clone()
            .unwrap_or_else(|| member.user.name.clone());
        match member.user.avatar {
            Some(hash) => {
                embed = embed.footer(CreateEmbedFooter::new(username).icon_url(format!(
                    "https://cdn.discordapp.com/avatars/{}/{}.png",
                    member.user.id, hash
                )));
            }
            None => {
                embed = embed.footer(
                    CreateEmbedFooter::new(username).icon_url(member.user.default_avatar_url()),
                );
            }
        }
    }

    channel_id
        .send_message(http, CreateMessage::new().embed(embed))
        .await
        .map_err(Box::new)?;

    Ok(())
}
