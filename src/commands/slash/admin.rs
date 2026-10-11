use std::fmt::Write;

use serenity::all::{ChannelType, CommandInteraction, GuildId, Http};
use serenity::prelude::Context;
use sqlx::MySqlPool;

use crate::constants::ADMIN_INIT_COMMAND_TAG;
use crate::db::{
    LogConfig, LogMode, VoiceDefaults, VoiceEventType, add_log_mode_channel_list,
    delete_log_mode_channel_list, delete_server, get_all_config, reset_all_voice_configs,
    set_add_channel, set_add_enable, set_log_channel, set_log_enable, set_log_mode, set_server,
    set_voice_defaults,
};
use crate::utils::{
    extract_voice_options, followup_db_error, followup_embed_error, followup_embed_say, lng_trs,
    validate_guild_channel,
};

async fn report_setting_errors(
    http: &Http,
    command: &CommandInteraction,
    lng: bool,
    error_count: usize,
    errors: &str,
    all_error_msg: (&str, &str),
) {
    let trimmed = errors.trim_end();
    if error_count < 3 {
        let msg = format!(
            "## {}{}{}",
            error_count,
            lng_trs(
                lng,
                " problem(s) occurred!:\n\n",
                "개의 문제가 발생했어요!:\n\n"
            ),
            trimmed
        );
        let _ = followup_embed_say(
            http,
            command,
            lng_trs(lng, "❕ Some errors", "❕ 몇몇 오류"),
            &msg,
            0x91275c,
        )
        .await;
    } else {
        let msg = format!(
            "## {}:\n\n{}",
            lng_trs(lng, all_error_msg.0, all_error_msg.1),
            trimmed
        );
        let _ = followup_embed_error(http, command, lng, &msg).await;
    }
}

pub async fn handle_admin_init(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &MySqlPool,
    guild_id: GuildId,
    lng: bool,
) {
    match set_server(pool, guild_id.get()).await {
        Ok(is_duplication) => {
            if is_duplication {
                let _ = followup_embed_say(
                    &ctx.http,
                    command,
                    lng_trs(lng, "❕ Duplicate", "❕ 중복"),
                    lng_trs(
                        lng,
                        "It's already been initialized!!",
                        "이미 초기화 되었어요!!",
                    ),
                    0xed95c1,
                )
                .await;
            } else {
                let _ = followup_embed_say(
                    &ctx.http,
                    command,
                    lng_trs(lng, "☑️ OK", "☑️ 완료"),
                    lng_trs(
                        lng,
                        "It was initialized normally!",
                        "정상적으로 초기화 되었어요!",
                    ),
                    0xed95c1,
                )
                .await;
            }
        }
        Err(e) => {
            eprintln!("[ERROR] set_server(): {:?}", e);
            let _ = followup_embed_error(
                &ctx.http,
                command,
                lng,
                lng_trs(
                    lng,
                    "an error happened during initialization..",
                    "초기화 도중 오류가 났어요..",
                ),
            )
            .await;
        }
    }
}

pub async fn handle_admin_delete(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &MySqlPool,
    guild_id: GuildId,
    lng: bool,
) {
    if let Err(e) = delete_server(pool, guild_id.get()).await {
        eprintln!("[ERROR] delete_server(): {:?}", e);
        let _ = followup_db_error(&ctx.http, command, lng).await;
    } else {
        let msg = format!(
            "**## Settings have been destroyed!!**\nIf you want to use it again, please use {}.",
            ADMIN_INIT_COMMAND_TAG.as_str()
        );
        let msg_ko = format!(
            "**## 설정이 공중분해됐어요!!**\n만약 다시 사용하시려면 {}를 사용해주세요.",
            ADMIN_INIT_COMMAND_TAG.as_str()
        );
        let _ = followup_embed_say(
            &ctx.http,
            command,
            lng_trs(lng, "💥 OK", "💥 완료"),
            lng_trs(lng, &msg, &msg_ko),
            0xed95c1,
        )
        .await;
    }
}

pub async fn handle_admin_reset(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &MySqlPool,
    guild_id: GuildId,
    lng: bool,
) {
    if let Err(e) = reset_all_voice_configs(pool, guild_id.get()).await {
        eprintln!("[ERROR] reset_all_voice_configs(): {:?}", e);
        let _ = followup_db_error(&ctx.http, command, lng).await;
    } else {
        let _ = followup_embed_say(
            &ctx.http,
            command,
            lng_trs(lng, "☑️ OK", "☑️ 완료"),
            lng_trs(
                lng,
                "It was reset event settings normally!",
                "정상적으로 이벤트 설정을 초기화 하였어요!",
            ),
            0xed95c1,
        )
        .await;
    }
}

pub async fn handle_admin_toggle(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &MySqlPool,
    guild_id: GuildId,
    lng: bool,
) {
    let Some(target) = command
        .data
        .options
        .first()
        .and_then(|opt| opt.value.as_str())
    else {
        let _ = followup_embed_error(
            &ctx.http,
            command,
            lng,
            lng_trs(
                lng,
                "Missing or invalid target option!",
                "대상이 없거나 잘못되었어요!",
            ),
        )
        .await;
        return;
    };

    let Some(enable) = command
        .data
        .options
        .get(1)
        .and_then(|opt| opt.value.as_bool())
    else {
        let _ = followup_embed_error(
            &ctx.http,
            command,
            lng,
            lng_trs(
                lng,
                "Missing or invalid enable option!",
                "활성화 여부가 없거나 잘못되었어요!",
            ),
        )
        .await;
        return;
    };

    let result = match target {
        "log" => set_log_enable(pool, guild_id.get(), enable).await,
        "add" => set_add_enable(pool, guild_id.get(), enable).await,
        _ => {
            let _ = followup_embed_error(
                &ctx.http,
                command,
                lng,
                lng_trs(lng, "Invalid target!!", "올바르지 않은 대상이에요!!"),
            )
            .await;
            return;
        }
    };

    match result {
        Ok(_) => {
            let _ = followup_embed_say(
                &ctx.http,
                command,
                lng_trs(lng, "☑️ OK", "☑️ 완료"),
                lng_trs(
                    lng,
                    &format!(
                        "Feature `{target}` has been {}.",
                        if enable { "enabled" } else { "disabled" }
                    ),
                    &format!(
                        "기능 `{}`이(가) {} 되었어요.",
                        target,
                        if enable { "활성화" } else { "비활성화" }
                    ),
                ),
                0xed95c1,
            )
            .await;
        }
        Err(e) => {
            eprintln!("[ERROR] admin_toggle: {:?}", e);
            let _ = followup_db_error(&ctx.http, command, lng).await;
        }
    }
}

pub async fn handle_admin_log(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &MySqlPool,
    guild_id: GuildId,
    lng: bool,
) {
    let Some(target_channel_id) = command
        .data
        .options
        .first()
        .and_then(|opt| opt.value.as_channel_id())
    else {
        let _ = followup_embed_error(
            &ctx.http,
            command,
            lng,
            lng_trs(
                lng,
                "Missing or invalid Channel option!",
                "체널이 없거나 잘못되었어요!",
            ),
        )
        .await;
        return;
    };

    let channel = match validate_guild_channel(&ctx.http, command, target_channel_id, lng).await {
        Some(ch) => ch,
        None => return,
    };

    if channel.kind == ChannelType::Category {
        let _ = followup_embed_error(
            &ctx.http,
            command,
            lng,
            lng_trs(
                lng,
                "Categories are not allowed!",
                "카테고리는 허용되지 않아요!",
            ),
        )
        .await;
        return;
    }

    if let Some(perms) = command.app_permissions {
        if !perms.send_messages() {
            let _ = followup_embed_error(
                &ctx.http,
                command,
                lng,
                lng_trs(
                    lng,
                    "I don't have permission to send messages in that channel..",
                    "그 채널에 메시지를 보낼 권한이 없어요..",
                ),
            )
            .await;
            return;
        }
    } else {
        let _ = followup_embed_error(
            &ctx.http,
            command,
            lng,
            lng_trs(
                lng,
                "Failed to get bot member..",
                "제 권한 정보를 가져오지 못했어요..",
            ),
        )
        .await;
        return;
    }

    if let Err(e) = set_log_channel(pool, guild_id.get(), Some(target_channel_id.get())).await {
        eprintln!("[ERROR] admin_log: {:?}", e);
        let _ = followup_db_error(&ctx.http, command, lng).await;
    } else {
        let _ = followup_embed_say(
            &ctx.http,
            command,
            lng_trs(lng, "☑️ OK", "☑️ 완료"),
            lng_trs(
                lng,
                "Log channel added normally!",
                "정상적으로 로그 체널을 설정하였어요!",
            ),
            0xed95c1,
        )
        .await;
    }
}

pub async fn handle_admin_add(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &MySqlPool,
    guild_id: GuildId,
    lng: bool,
) {
    let Some(target_channel_id) = command
        .data
        .options
        .first()
        .and_then(|opt| opt.value.as_channel_id())
    else {
        let _ = followup_embed_error(
            &ctx.http,
            command,
            lng,
            lng_trs(
                lng,
                "Missing or invalid Channel option!",
                "체널이 없거나 잘못되었어요!",
            ),
        )
        .await;
        return;
    };

    if validate_guild_channel(&ctx.http, command, target_channel_id, lng)
        .await
        .is_none()
    {
        return;
    }

    if let Err(e) = set_add_channel(pool, guild_id.get(), Some(target_channel_id.get())).await {
        eprintln!("[ERROR] admin_add: {:?}", e);
        let _ = followup_db_error(&ctx.http, command, lng).await;
    } else {
        let _ = followup_embed_say(
            &ctx.http,
            command,
            lng_trs(lng, "☑️ OK", "☑️ 완료"),
            lng_trs(
                lng,
                "Add channel set normally!",
                "정상적으로 추가 체널을 설정하였어요!",
            ),
            0xed95c1,
        )
        .await;
    }
}

pub async fn handle_admin_add_setting(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &MySqlPool,
    guild_id: GuildId,
    lng: bool,
) {
    let mut errors = String::new();
    let mut error_count = 0;
    let mut name_invalid = false;
    let mut user_limit_invalid = false;
    let mut bitrate_invalid = false;

    let (name, user_limit_raw, bitrate_raw) = extract_voice_options(&command.data.options);

    if name.is_none() && user_limit_raw.is_none() && bitrate_raw.is_none() {
        let _ = followup_embed_say(
            &ctx.http,
            command,
            lng_trs(lng, "❕ Empty", "❕ 비어있음"),
            lng_trs(lng, "No value was set!!", "아무런 값도 설정되지 않았어요!!"),
            0x91275c,
        )
        .await;
        return;
    }

    if let Some(ref name_str) = name {
        let len = name_str.chars().count();
        if len < 2 {
            errors.push_str(lng_trs(
                lng,
                "Channel name must be **2 characters or higher**!\n",
                "채널 이름은 **2자 이상**이여야 돼요!\n",
            ));
            name_invalid = true;
            error_count += 1;
        } else if len > 100 {
            errors.push_str(lng_trs(
                lng,
                "Channel names must be **100 characters or less**!\n",
                "채널 이름은 **100자 이하**여야 돼요!\n",
            ));
            name_invalid = true;
            error_count += 1;
        }
    }

    let user_limit = if let Some(v) = user_limit_raw {
        if v < 0 {
            errors.push_str(lng_trs(
                lng,
                "The number of people must be **0 or higher**!\n",
                "인원수는 **0 이상**이어야 돼요!\n",
            ));
            user_limit_invalid = true;
            error_count += 1;
            None
        } else if v > 99 {
            errors.push_str(lng_trs(
                lng,
                "The number of people must be **99 or less**!\n",
                "인원수는 **99 이하**여야 돼요!\n",
            ));
            user_limit_invalid = true;
            error_count += 1;
            None
        } else {
            Some(v as u8)
        }
    } else {
        None
    };

    let bitrate = if let Some(v) = bitrate_raw {
        if v < 8 {
            errors.push_str(lng_trs(
                lng,
                "Bitrate must be **`8` or higher**!\n",
                "비트레이트는 **`8` 이상**이여야 돼요!\n",
            ));
            bitrate_invalid = true;
            error_count += 1;
            None
        } else if v > 384 {
            errors.push_str(lng_trs(
                lng,
                "Bitrate must be **`384` or less**!\n",
                "비트레이트는 **`384` 이하**여야 돼요!\n",
            ));
            bitrate_invalid = true;
            error_count += 1;
            None
        } else {
            Some(v as u16)
        }
    } else {
        None
    };

    let defaults = VoiceDefaults {
        name: if name_invalid { None } else { name },
        user_limit: if user_limit_invalid { None } else { user_limit },
        bitrate: if bitrate_invalid { None } else { bitrate },
    };

    if let Err(e) = set_voice_defaults(pool, guild_id.get(), &defaults).await {
        eprintln!("[ERROR] admin_add_setting: {:?}", e);
        let _ = followup_embed_error(
            &ctx.http,
            command,
            lng,
            lng_trs(
                lng,
                "There was a problem setting default values in the database..",
                "기본 값을 데이터베이스에 설정 중 문제가 발생했어요..",
            ),
        )
        .await;
        return;
    }

    if error_count > 0 {
        report_setting_errors(
            &ctx.http,
            command,
            lng,
            error_count,
            &errors,
            ("Everything has a problem!", "모두 문제가 발생했어요!"),
        )
        .await;
    } else {
        let _ = followup_embed_say(
            &ctx.http,
            command,
            lng_trs(lng, "☑️ OK", "☑️ 완료"),
            lng_trs(
                lng,
                "default values have been set normally!",
                "정상적으로 기본값을 설정하였어요!",
            ),
            0xed95c1,
        )
        .await;
    }
}

pub async fn handle_admin_log_setting(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &MySqlPool,
    guild_id: GuildId,
    lng: bool,
) {
    let options = &command.data.options;

    let event_str = options
        .first()
        .and_then(|opt| opt.value.as_str())
        .unwrap_or_default();

    let event = match event_str {
        "add" => VoiceEventType::Add,
        "destroy" => VoiceEventType::Destroy,
        "join" => VoiceEventType::Join,
        "move" => VoiceEventType::Move,
        "leave" => VoiceEventType::Leave,
        _ => {
            let _ = followup_embed_error(
                &ctx.http,
                command,
                lng,
                lng_trs(lng, "Type is incorrect!", "타입이 잘못되었어요!"),
            )
            .await;
            return;
        }
    };

    let mut errors = String::new();
    let mut error_count = 0;

    let mut title_invalid = false;
    let mut description_invalid = false;
    let mut color_invalid = false;

    let title: Option<String> = options
        .iter()
        .find(|opt| opt.name == "title")
        .and_then(|opt| opt.value.as_str())
        .map(|s| s.to_string());

    let description: Option<String> = options
        .iter()
        .find(|opt| opt.name == "description")
        .and_then(|opt| opt.value.as_str())
        .map(|s| s.to_string());

    let color_raw: Option<&str> = options
        .iter()
        .find(|opt| opt.name == "color")
        .and_then(|opt| opt.value.as_str());

    if title.is_none() && description.is_none() && color_raw.is_none() {
        let _ = followup_embed_say(
            &ctx.http,
            command,
            lng_trs(lng, "❕ Empty", "❕ 비어있음"),
            lng_trs(lng, "No value was set!!", "아무런 값도 설정되지 않았어요!!"),
            0x91275c,
        )
        .await;
        return;
    }

    if let Some(ref title_str) = title {
        let len = title_str.chars().count();
        if len < 1 {
            errors.push_str(lng_trs(
                lng,
                "Title must be at least 1 character!\n",
                "제목은 최소 1자 이상이어야 해요!\n",
            ));
            title_invalid = true;
            error_count += 1;
        } else if len > 16 {
            errors.push_str(lng_trs(
                lng,
                "Title must be 16 characters or less!\n",
                "제목은 16자 이하이어야 해요!\n",
            ));
            title_invalid = true;
            error_count += 1;
        }
    }

    if let Some(ref desc_str) = description {
        let len = desc_str.chars().count();
        if len > 48 {
            errors.push_str(lng_trs(
                lng,
                "Description must be 48 characters or less!\n",
                "설명은 48자 이하이어야 해요!\n",
            ));
            description_invalid = true;
            error_count += 1;
        }
    }

    let color: Option<u32> = if let Some(color_str) = color_raw {
        if color_str.len() != 6 || !color_str.chars().all(|c| c.is_ascii_hexdigit()) {
            errors.push_str(lng_trs(
                lng,
                "Color must be a 6-digit hex RGB value!\n",
                "색상은 6자리 16진수 RGB 값이어야 해요!\n",
            ));
            color_invalid = true;
            error_count += 1;
            None
        } else {
            match u32::from_str_radix(color_str, 16) {
                Ok(val) => Some(val),
                Err(_) => {
                    errors.push_str(lng_trs(
                        lng,
                        "Color parsing error!\n",
                        "색상 변환 중 오류가 발생했어요!\n",
                    ));
                    color_invalid = true;
                    error_count += 1;
                    None
                }
            }
        }
    } else {
        None
    };

    let defaults = LogConfig {
        title: if title_invalid { None } else { title },
        description: if description_invalid {
            None
        } else {
            description
        },
        color: if color_invalid { None } else { color },
    };

    if let Err(e) = event.set_log_config(pool, guild_id.get(), &defaults).await {
        eprintln!("[ERROR] admin_log_setting: {:?}", e);
        let _ = followup_embed_error(
            &ctx.http,
            command,
            lng,
            lng_trs(
                lng,
                "There was a problem setting default values in the database..",
                "기본 값을 데이터베이스에 설정 중 문제가 발생했어요..",
            ),
        )
        .await;
        return;
    }

    if error_count > 0 {
        report_setting_errors(
            &ctx.http,
            command,
            lng,
            error_count,
            &errors,
            ("Multiple errors occurred!", "여러 오류가 발생했어요!"),
        )
        .await;
    } else {
        let _ = followup_embed_say(
            &ctx.http,
            command,
            lng_trs(lng, "☑️ OK", "☑️ 완료"),
            lng_trs(
                lng,
                "Event values have been set normally!",
                "정상적으로 이벤트값을 설정하였어요!",
            ),
            0xed95c1,
        )
        .await;
    }
}

pub async fn handle_admin_info(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &MySqlPool,
    guild_id: GuildId,
    lng: bool,
) {
    match get_all_config(pool, guild_id.get()).await {
        Ok(Some(config)) => {
            let mut desc = String::new();

            let _ = writeln!(desc, "```");
            let _ = writeln!(
                desc,
                "+------------------------+----------------------------+"
            );
            let _ = writeln!(
                desc,
                "| Field                  | Value                      |"
            );
            let _ = writeln!(
                desc,
                "+------------------------+----------------------------+"
            );

            macro_rules! write_field {
                ($name:expr, $val:expr) => {
                    let val_str = match $val {
                        Some(ref v) => format!("{}", v),
                        None => "null".to_string(),
                    };
                    let _ = writeln!(desc, "| {:<22} | {:<26} |", $name, val_str);
                };
            }
            macro_rules! write_field_str {
                ($name:expr, $val:expr) => {
                    let val_str = match $val {
                        Some(ref v) => v.clone(),
                        None => "null".to_string(),
                    };
                    let _ = writeln!(desc, "| {:<22} | {:<26} |", $name, val_str);
                };
            }

            write_field!("Log Enabled", Some(config.log_enable));
            write_field!("Add Enabled", Some(config.add_enable));
            write_field!("Log Channel", config.log_channel);
            write_field!("Add Channel", config.add_channel);
            write_field_str!("Default Name", config.default_name);
            write_field!("User Limit", config.default_user_limit);
            write_field!("Bitrate", config.default_bitrate);
            write_field_str!("Log Add Title", config.create_title);
            write_field_str!("Log Add Desc", config.create_desc);
            write_field!("Log Add Color", config.create_color);
            write_field_str!("Log Destroy Title", config.destroy_title);
            write_field_str!("Log Destroy Desc", config.destroy_desc);
            write_field!("Log Destroy Color", config.destroy_color);
            write_field_str!("Log Join Title", config.join_title);
            write_field_str!("Log Join Desc", config.join_desc);
            write_field!("Log Join Color", config.join_color);
            write_field_str!("Log Move Title", config.move_title);
            write_field_str!("Log Move Desc", config.move_desc);
            write_field!("Log Move Color", config.move_color);
            write_field_str!("Log Leave Title", config.leave_title);
            write_field_str!("Log Leave Desc", config.leave_desc);
            write_field!("Log Leave Color", config.leave_color);
            write_field!("Log Mode", Some(config.log_mode.to_string()));

            let log_mode_channel_list_ids: Vec<u64> = config.log_mode_channel_list;
            let log_mode_channel_list_display = if log_mode_channel_list_ids.is_empty() {
                vec!["None".to_string()]
            } else {
                log_mode_channel_list_ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
            };

            for (i, line) in log_mode_channel_list_display.iter().enumerate() {
                let label = if i == 0 {
                    "Log Mode Channel List"
                } else {
                    "\""
                };
                let _ = writeln!(desc, "| {:<22} | {:<26} |", label, line);
            }

            let _ = writeln!(
                desc,
                "+------------------------+----------------------------+"
            );
            let _ = writeln!(desc, "```");

            let _ = followup_embed_say(
                &ctx.http,
                command,
                lng_trs(lng, "📄 Info", "📄 정보"),
                &desc,
                0xed95c1,
            )
            .await;
        }
        Ok(None) => {
            let _ = followup_embed_error(
                &ctx.http,
                command,
                lng,
                lng_trs(
                    lng,
                    "This server has no config yet!!",
                    "이 서버는 아직 설정된 정보가 없어요!!",
                ),
            )
            .await;
        }
        Err(e) => {
            eprintln!("[ERROR] get_all_config(): {:?}", e);
            let _ = followup_db_error(&ctx.http, command, lng).await;
        }
    }
}

pub async fn handle_admin_log_mode_channel_list(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &MySqlPool,
    guild_id: GuildId,
    lng: bool,
) {
    let Some(add) = command
        .data
        .options
        .iter()
        .find(|opt| opt.name == "add")
        .and_then(|opt| opt.value.as_bool())
    else {
        let _ = followup_embed_error(
            &ctx.http,
            command,
            lng,
            lng_trs(lng, "Missing 'add' option!", "'add' 옵션이 없어요!"),
        )
        .await;
        return;
    };

    let Some(target_channel_id) = command
        .data
        .options
        .iter()
        .find(|opt| opt.name == "channel")
        .and_then(|opt| opt.value.as_channel_id())
    else {
        let _ = followup_embed_error(
            &ctx.http,
            command,
            lng,
            lng_trs(
                lng,
                "Missing or invalid Channel option!",
                "체널이 없거나 잘못되었어요!",
            ),
        )
        .await;
        return;
    };

    if validate_guild_channel(&ctx.http, command, target_channel_id, lng)
        .await
        .is_none()
    {
        return;
    }

    let result = if add {
        add_log_mode_channel_list(pool, guild_id.get(), target_channel_id.get()).await
    } else {
        delete_log_mode_channel_list(pool, guild_id.get(), target_channel_id.get()).await
    };

    match result {
        Ok(true) => {
            let msg = if add {
                lng_trs(lng, "Channel added to list!", "리스트에 채널을 추가했어요!")
            } else {
                lng_trs(
                    lng,
                    "Channel removed from list!",
                    "리스트에서 채널을 제거했어요!",
                )
            };

            let _ = followup_embed_say(
                &ctx.http,
                command,
                lng_trs(lng, "☑️ OK", "☑️ 완료"),
                msg,
                0xed95c1,
            )
            .await;
        }
        Ok(false) => {
            let msg = if add {
                lng_trs(lng, "Already in list!", "이미 리스트에 있어요!")
            } else {
                lng_trs(lng, "Channel is not in list!", "리스트에 없는 채널이에요!")
            };

            let _ = followup_embed_error(&ctx.http, command, lng, msg).await;
        }
        Err(e) => {
            eprintln!("[ERROR] admin_log_mode_channel_list: {:?}", e);
            let _ = followup_embed_error(
                &ctx.http,
                command,
                lng,
                lng_trs(
                    lng,
                    "Database connection error.",
                    "데이터베이스 연결에 문제가 발생했어요.",
                ),
            )
            .await;
        }
    }
}

pub async fn handle_admin_log_mode(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &MySqlPool,
    guild_id: GuildId,
    lng: bool,
) {
    let mode_str = command
        .data
        .options
        .iter()
        .find(|opt| opt.name == "mode")
        .and_then(|opt| opt.value.as_str())
        .unwrap_or("none");

    let mode = match mode_str {
        "blacklist" => LogMode::BlackList,
        "whitelist" => LogMode::WhiteList,
        _ => LogMode::None,
    };

    match set_log_mode(pool, guild_id.get(), mode).await {
        Ok(_) => {
            let msg = lng_trs(
                lng,
                format!("Log mode set to {}!", mode),
                format!("로그 모드를 {}(으)로 설정했어요!", mode),
            );

            let _ = followup_embed_say(
                &ctx.http,
                command,
                lng_trs(lng, "☑️ OK", "☑️ 완료"),
                &msg,
                0xed95c1,
            )
            .await;
        }
        Err(e) => {
            eprintln!("[ERROR] admin_log_mode: {:?}", e);
            let _ = followup_embed_error(
                &ctx.http,
                command,
                lng,
                lng_trs(
                    lng,
                    "Database connection error.",
                    "데이터베이스 연결에 문제가 발생했어요.",
                ),
            )
            .await;
        }
    }
}
