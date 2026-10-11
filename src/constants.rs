use once_cell::sync::Lazy;
use serenity::all::ChannelId;

pub static CHANNELID_LOG_SECRET: Lazy<Option<ChannelId>> = Lazy::new(|| {
    std::env::var("LOG_CHANNEL_ID")
        .or_else(|_| std::env::var("STATUS_LOG_CHANNEL_ID"))
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(ChannelId::new)
});

pub static ADMIN_INIT_COMMAND_TAG: Lazy<String> = Lazy::new(|| {
    if let Ok(id) = std::env::var("ADMIN_INIT_COMMAND_ID") {
        let id = id.trim();
        if !id.is_empty() {
            return format!("</admin_init:{}>", id);
        }
    }
    if let Ok(tag) = std::env::var("ADMIN_INIT_COMMAND_TAG") {
        let tag = tag.trim();
        if !tag.is_empty() {
            return tag.to_string();
        }
    }
    "`/admin_init`".to_string()
});
