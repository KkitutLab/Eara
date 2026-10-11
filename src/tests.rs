use std::collections::HashMap;

use serenity::all::PremiumTier;

use crate::db::{AllConfig, GuildVoiceSettings, LogConfig, LogMode, VoiceConfig, VoiceEventType};
use crate::handler::Handler;
use crate::utils::{
    extract_voice_options, format_placeholders, get_bitrate_limit, lng_trs, resolve_log_config,
};

#[test]
fn test_lng_trs() {
    assert_eq!(lng_trs(true, "en", "ko"), "ko");
    assert_eq!(lng_trs(false, "en", "ko"), "en");
    assert_eq!(lng_trs(true, 10, 20), 20);
    assert_eq!(lng_trs(false, 10, 20), 10);
}

#[test]
fn test_get_bitrate_limit() {
    assert_eq!(get_bitrate_limit(PremiumTier::Tier1), 128);
    assert_eq!(get_bitrate_limit(PremiumTier::Tier2), 256);
    assert_eq!(get_bitrate_limit(PremiumTier::Tier3), 384);
    assert_eq!(get_bitrate_limit(PremiumTier::Tier0), 96);
}

#[test]
fn test_format_placeholders() {
    let mut context = HashMap::new();
    context.insert("c", "123456789".to_string());
    context.insert("o", "987654321".to_string());
    context.insert("t", "1672531199".to_string());

    let template = "### <#{c}> : <t:{t}:T>";
    let result = format_placeholders(template, &context);
    assert_eq!(result, "### <#123456789> : <t:1672531199:T>");

    let template_move = "### <#{o}> -> <#{c}> : <t:{t}:T>";
    let result_move = format_placeholders(template_move, &context);
    assert_eq!(
        result_move,
        "### <#987654321> -> <#123456789> : <t:1672531199:T>"
    );

    let template_unknown = "hello {x} world";
    let result_unknown = format_placeholders(template_unknown, &context);
    assert_eq!(result_unknown, "hello {x} world");
}

#[test]
fn test_log_mode() {
    assert_eq!(LogMode::from(0), LogMode::None);
    assert_eq!(LogMode::from(1), LogMode::BlackList);
    assert_eq!(LogMode::from(2), LogMode::WhiteList);
    assert_eq!(LogMode::from(99), LogMode::None);

    assert_eq!(format!("{}", LogMode::None), "None");
    assert_eq!(format!("{}", LogMode::BlackList), "BlackList");
    assert_eq!(format!("{}", LogMode::WhiteList), "WhiteList");
}

#[test]
fn test_voice_event_type_as_str() {
    assert_eq!(VoiceEventType::Add.as_str(), "create");
    assert_eq!(VoiceEventType::Destroy.as_str(), "destroy");
    assert_eq!(VoiceEventType::Join.as_str(), "join");
    assert_eq!(VoiceEventType::Move.as_str(), "move");
    assert_eq!(VoiceEventType::Leave.as_str(), "leave");
}

#[tokio::test]
async fn test_handler_voice_memory() {
    let handler = Handler::new();
    let guild_id = 1111;

    assert_eq!(handler.get_voice_entries(guild_id).await.len(), 0);

    handler.add_voice_entry(guild_id, 100, 200, false).await;
    handler.add_voice_entry(guild_id, 101, 201, true).await;

    let entries = handler.get_voice_entries(guild_id).await;
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0], (100, 200, false));
    assert_eq!(entries[1], (101, 201, true));

    let entry = handler.get_voice_entry(guild_id, 100).await;
    assert_eq!(entry, Some((100, 200, false)));

    let removed = handler.remove_voice_entry_by_channel(guild_id, 100).await;
    assert_eq!(removed, Some(false));

    let entries_after_remove = handler.get_voice_entries(guild_id).await;
    assert_eq!(entries_after_remove.len(), 1);
    assert_eq!(entries_after_remove[0], (101, 201, true));

    let updated = handler
        .update_voice_entry(guild_id, (101, 201), (102, 202))
        .await;
    assert!(updated);

    let entries_after_update = handler.get_voice_entries(guild_id).await;
    assert_eq!(entries_after_update[0], (102, 202, true));

    handler.remove_guild_voice_data(guild_id).await;
    assert_eq!(handler.get_voice_entries(guild_id).await.len(), 0);
}

#[test]
fn test_resolve_log_config_defaults() {
    let default_cfg = LogConfig::default();

    let add_f = resolve_log_config(VoiceEventType::Add, &default_cfg);
    assert_eq!(add_f.title, "+ ADD");
    assert_eq!(add_f.description, "### <#{c}> : <t:{t}:T>");
    assert_eq!(add_f.color, 0xf7c8e0);

    let del_f = resolve_log_config(VoiceEventType::Destroy, &default_cfg);
    assert_eq!(del_f.title, "- DEL");
    assert_eq!(del_f.color, 0x330c20);

    let join_f = resolve_log_config(VoiceEventType::Join, &default_cfg);
    assert_eq!(join_f.title, "\\> ENT");
    assert_eq!(join_f.color, 0xff91c9);

    let mov_f = resolve_log_config(VoiceEventType::Move, &default_cfg);
    assert_eq!(mov_f.title, "= MOV");
    assert_eq!(mov_f.color, 0x99657f);

    let leave_f = resolve_log_config(VoiceEventType::Leave, &default_cfg);
    assert_eq!(leave_f.title, "< OUT");
    assert_eq!(leave_f.color, 0xba0962);
}

#[test]
fn test_resolve_log_config_custom() {
    let custom_cfg = LogConfig {
        title: Some("Custom Title".to_string()),
        description: Some("Custom Desc".to_string()),
        color: Some(0x123456),
    };

    let resolved = resolve_log_config(VoiceEventType::Join, &custom_cfg);
    assert_eq!(resolved.title, "Custom Title");
    assert_eq!(resolved.description, "Custom Desc");
    assert_eq!(resolved.color, 0x123456);
}

#[test]
fn test_extract_voice_options_empty() {
    let options = vec![];
    let (name, limit, bitrate) = extract_voice_options(&options);
    assert_eq!(name, None);
    assert_eq!(limit, None);
    assert_eq!(bitrate, None);
}

#[test]
fn test_all_config_deref() {
    let voice = VoiceConfig {
        log_enable: true,
        default_name: Some("Room".to_string()),
        log_mode: LogMode::BlackList,
        ..Default::default()
    };
    let all = AllConfig {
        voice,
        log_mode_channel_list: vec![123, 456],
    };

    assert!(all.log_enable);
    assert_eq!(all.default_name.as_deref(), Some("Room"));
    assert_eq!(all.log_mode, LogMode::BlackList);
    assert_eq!(all.log_mode_channel_list, vec![123, 456]);
}

#[test]
fn test_guild_voice_settings() {
    let settings = GuildVoiceSettings {
        log_enable: true,
        log_channel: Some(100),
        add_enable: false,
        add_channel: None,
        log_mode: LogMode::WhiteList,
        mode_channel_list: vec![200],
    };

    assert!(settings.log_enable);
    assert_eq!(settings.log_channel, Some(100));
    assert!(!settings.add_enable);
    assert_eq!(settings.add_channel, None);
    assert_eq!(settings.log_mode, LogMode::WhiteList);
    assert_eq!(settings.mode_channel_list, vec![200]);
}

#[test]
fn test_admin_init_command_tag_default() {
    use crate::constants::ADMIN_INIT_COMMAND_TAG;
    assert!(!ADMIN_INIT_COMMAND_TAG.is_empty());
}
