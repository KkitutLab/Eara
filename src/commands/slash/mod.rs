pub mod admin;
pub mod misc;
pub mod setting;

use serenity::all::{
    ChannelType, Command, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage, Http,
    InstallationContext,
};
use sqlx::MySqlPool;

use crate::constants::ADMIN_INIT_COMMAND_TAG;
use crate::db::server_exists;
use crate::handler::Handler;
use crate::utils::{check_permission, defer, followup_db_error, followup_embed_error, lng_trs};

pub async fn register_global_commands(http: &Http) {
    let commands = vec![
        CreateCommand::new("ping")
            .description("Measure the latency.")
            .description_localized("ko", "지연 시간을 측정해요.")
            .integration_types(vec![InstallationContext::Guild]),
        CreateCommand::new("admin_init")
            .description("Prepare for use.")
            .description_localized("ko", "사용을 준비해요.")
            .integration_types(vec![InstallationContext::Guild]),
        CreateCommand::new("admin_delete")
            .description("This will blow all settings. Be careful!")
            .description_localized("ko", "모든 설정을 날려요. 신중하세요!")
            .integration_types(vec![InstallationContext::Guild]),
        CreateCommand::new("admin_reset")
            .description("Resets all voice events settings.")
            .description_localized("ko", "모든 음성 이벤트 설정만 초기화해요.")
            .integration_types(vec![InstallationContext::Guild]),
        CreateCommand::new("admin_toggle")
            .description("Enable or disable bot features.")
            .description_localized("ko", "봇 기능을 활성화하거나 비활성화해요.")
            .integration_types(vec![InstallationContext::Guild])
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "target",
                    "Select feature to enable/disable",
                )
                .description_localized("ko", "활성화 또는 비활성화할 기능")
                .required(true)
                .add_string_choice_localized("voice log", "log", [("ko", "음성 기록")])
                .add_string_choice_localized(
                    "voice create",
                    "add",
                    [("ko", "음성 생성")],
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "enable",
                    "Enable (true) or disable (false)",
                )
                .description_localized("ko", "활성화 (true) 또는 비활성화 (false)")
                .required(true),
            ),
        CreateCommand::new("admin_log")
            .description("Channel to record logs")
            .description_localized("ko", "기록할 채널을 설정해요.")
            .add_option(
                CreateCommandOption::new(CommandOptionType::Channel, "channel", "log channel")
                    .description_localized("ko", "로그 채널")
                    .required(true),
            ),
        CreateCommand::new("admin_add")
            .description("Set the voice channel to create temporary channels under.")
            .description_localized("ko", "임시 음성 채널이 생성될 기준 채널을 설정해요.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Channel,
                    "channel",
                    "The base voice channel",
                )
                .description_localized("ko", "기준이 될 음성 채널")
                .channel_types(vec![ChannelType::Voice, ChannelType::Stage])
                .required(true),
            ),
        CreateCommand::new("admin_add_setting")
            .description("Set default settings for temporary voice channels.")
            .description_localized("ko", "임시 음성 채널의 기본 설정을 지정해요.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "name",
                    "Default name for temporary channels",
                )
                .description_localized("ko", "임시 채널의 기본 이름")
                .required(false)
                .min_length(2)
                .max_length(100),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "user_limit",
                    "Default number of users (0 = unlimited)",
                )
                .description_localized("ko", "기본 인원수 (0은 무제한)")
                .required(false)
                .min_int_value(0)
                .max_int_value(99),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "bitrate",
                    "Default Bitrate in kbps. Above 96 requires server boost level!",
                )
                .description_localized(
                    "ko",
                    "기본 비트레이트 (kbps). 96kbps 초과는 서버 부스트 수준에 따라 달라져요!",
                )
                .required(false)
                .min_int_value(8)
                .max_int_value(384),
            ),
        CreateCommand::new("admin_info")
            .description("View the all settings.")
            .description_localized("ko", "전체 설정을 확인해요."),
        CreateCommand::new("setting")
            .description(
                "Change settings of temporary voice channels. Only the creator can change them!",
            )
            .description_localized(
                "ko",
                "음성 채널의 설정을 변경합니다. 생성자만 변경할 수 있어요!",
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "name", "name for channel")
                    .description_localized("ko", "채널의 이름")
                    .required(false)
                    .min_length(2)
                    .max_length(100),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "user_limit",
                    "number of users (0 = unlimited)",
                )
                .description_localized("ko", "인원수 (0은 무제한)")
                .required(false)
                .min_int_value(0)
                .max_int_value(99),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "bitrate",
                    "Bitrate in kbps (8-384). Above 96 requires server boost level!",
                )
                .description_localized(
                    "ko",
                    "비트레이트 (8~384kbps). 96kbps 초과는 서버 부스트 수준에 따라 달라져요!",
                )
                .required(false)
                .min_int_value(8)
                .max_int_value(384),
            ),
        CreateCommand::new("admin_log_mode_channel_list")
            .description("Add or remove a channel from voice event list")
            .description_localized("ko", "음성 이벤트 리스트에 채널을 추가하거나 제거해요.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "add",
                    "Add or remove from list",
                )
                .description_localized("ko", "리스트에 추가하거나 제거할지 여부예요.")
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::Channel, "channel", "Target channel")
                    .description_localized("ko", "대상 채널이에요.")
                    .channel_types(vec![ChannelType::Voice, ChannelType::Stage])
                    .required(true),
            )
            .integration_types(vec![InstallationContext::Guild]),
        CreateCommand::new("admin_log_mode")
            .description("Set the log mode for a voice channel")
            .description_localized("ko", "음성 채널의 로그 모드를 설정해요.")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "mode", "Select log mode")
                    .description_localized("ko", "로그 모드를 선택하세요.")
                    .add_string_choice("None", "none")
                    .add_string_choice("BlackList", "blacklist")
                    .add_string_choice("WhiteList", "whitelist")
                    .required(true),
            ),
        CreateCommand::new("admin_log_setting")
            .description("Change log display settings")
            .description_localized("ko", "로그 표시 설정을 바꿉니다.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "event",
                    "Log event type (add, destroy, join, move, leave)",
                )
                .description_localized("ko", "로그 이벤트 타입 (add, destroy, join, move, leave)")
                .required(true)
                .add_string_choice("Add", "add")
                .add_string_choice("Destroy", "destroy")
                .add_string_choice("Join", "join")
                .add_string_choice("Move", "move")
                .add_string_choice("Leave", "leave"),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "title",
                    "Title (max 16 chars)",
                )
                .description_localized("ko", "제목 (최대 16자)")
                .required(false)
                .max_length(16),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "description",
                    "Description (max 48 chars)",
                )
                .description_localized("ko", "설명 (최대 48자)")
                .required(false)
                .max_length(48),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "color",
                    "Embed color (6-digit hex RGB)",
                )
                .description_localized("ko", "Embed 색상 (6자리 16진수 RGB)")
                .required(false)
                .min_length(6)
                .max_length(6),
            ),
    ];

    if let Err(err) = Command::set_global_commands(http, commands).await {
        eprintln!("[ERROR] Global Commands register(): {:?}", err);
    }
}

pub async fn dispatch_slash_command(
    handler: &Handler,
    ctx: &Context,
    command: &CommandInteraction,
    pool: &MySqlPool,
) {
    let lng = command.locale.as_str() == "ko";
    let Some(guild_id) = command.guild_id else {
        if let Err(e) = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(lng_trs(
                            lng,
                            "oh... This isn't a server..",
                            "어라... 여긴 서버가 아닌걸요..",
                        ))
                        .ephemeral(true),
                ),
            )
            .await
        {
            eprintln!("[ERROR] Command create_response(): {:?}", e);
        }
        return;
    };

    let Some(ref member) = command.member else {
        if let Err(e) = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(lng_trs(
                            lng,
                            "...Huh? I can't see your information. What's going on?",
                            "...엥? 당산의 정보를 볼 수가 없는데 이게 무슨 일인지..",
                        ))
                        .ephemeral(true),
                ),
            )
            .await
        {
            eprintln!("[ERROR] Command create_response(): {:?}", e);
        }
        return;
    };

    let cmdname = command.data.name.as_str();

    if cmdname == "ping" {
        misc::handle_ping(ctx, command, lng).await;
        return;
    }

    if !defer(&ctx.http, command, true).await {
        return;
    }

    let is_admin = cmdname.starts_with("admin_");
    if is_admin && !check_permission(&ctx.http, member, command, lng).await {
        return;
    }

    if cmdname != "admin_init" {
        match server_exists(pool, guild_id.get()).await {
            Ok(true) => {}
            Ok(false) => {
                let msg_en = format!(
                    "You need to initialize first!\nPlease use {}.",
                    ADMIN_INIT_COMMAND_TAG.as_str()
                );
                let msg_ko = format!(
                    "초기화가 필요해요!\n{} 를 사용해주세요.",
                    ADMIN_INIT_COMMAND_TAG.as_str()
                );
                let _ =
                    followup_embed_error(&ctx.http, command, lng, lng_trs(lng, &msg_en, &msg_ko))
                        .await;
                return;
            }
            Err(e) => {
                eprintln!("[ERROR] server_exists(): {:?}", e);
                let _ = followup_db_error(&ctx.http, command, lng).await;
                return;
            }
        }
    }

    match cmdname {
        "setting" => setting::handle_setting(handler, ctx, command, guild_id, lng).await,
        "admin_init" => admin::handle_admin_init(ctx, command, pool, guild_id, lng).await,
        "admin_delete" => admin::handle_admin_delete(ctx, command, pool, guild_id, lng).await,
        "admin_reset" => admin::handle_admin_reset(ctx, command, pool, guild_id, lng).await,
        "admin_toggle" => admin::handle_admin_toggle(ctx, command, pool, guild_id, lng).await,
        "admin_log" => admin::handle_admin_log(ctx, command, pool, guild_id, lng).await,
        "admin_add" => admin::handle_admin_add(ctx, command, pool, guild_id, lng).await,
        "admin_add_setting" => {
            admin::handle_admin_add_setting(ctx, command, pool, guild_id, lng).await
        }
        "admin_info" => admin::handle_admin_info(ctx, command, pool, guild_id, lng).await,
        "admin_log_mode_channel_list" => {
            admin::handle_admin_log_mode_channel_list(ctx, command, pool, guild_id, lng).await
        }
        "admin_log_mode" => admin::handle_admin_log_mode(ctx, command, pool, guild_id, lng).await,
        "admin_log_setting" => {
            admin::handle_admin_log_setting(ctx, command, pool, guild_id, lng).await
        }
        _ => {
            let _ = followup_embed_error(
                &ctx.http,
                command,
                lng,
                lng_trs(lng, "Command does not exist!", "존재하지 않는 명령어에요!."),
            )
            .await;
        }
    }
}
