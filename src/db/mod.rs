pub mod models;

pub use models::*;

use sqlx::{MySql, MySqlPool, Pool, QueryBuilder, Row};
use tokio::sync::OnceCell;

pub static TOKEN: once_cell::sync::OnceCell<String> = once_cell::sync::OnceCell::new();
pub static DATABASE_URL: once_cell::sync::OnceCell<String> = once_cell::sync::OnceCell::new();

static POOL: OnceCell<MySqlPool> = OnceCell::const_new();

pub async fn get_pool() -> Result<&'static MySqlPool, sqlx::Error> {
    POOL.get_or_try_init(|| async { Pool::<MySql>::connect(DATABASE_URL.get().unwrap()).await })
        .await
}

pub fn get_row_u64(row: &sqlx::mysql::MySqlRow, col: &str) -> Option<u64> {
    row.try_get::<u64, _>(col)
        .or_else(|_| row.try_get::<i64, _>(col).map(|v| v as u64))
        .or_else(|_| row.try_get::<u32, _>(col).map(|v| v as u64))
        .or_else(|_| row.try_get::<i32, _>(col).map(|v| v as u64))
        .ok()
}

pub fn get_row_opt_u64(row: &sqlx::mysql::MySqlRow, col: &str) -> Option<u64> {
    row.try_get::<Option<u64>, _>(col)
        .or_else(|_| {
            row.try_get::<Option<i64>, _>(col)
                .map(|o| o.map(|v| v as u64))
        })
        .or_else(|_| {
            row.try_get::<Option<u32>, _>(col)
                .map(|o| o.map(|v| v as u64))
        })
        .or_else(|_| {
            row.try_get::<Option<i32>, _>(col)
                .map(|o| o.map(|v| v as u64))
        })
        .ok()
        .flatten()
}

pub fn get_row_bool(row: &sqlx::mysql::MySqlRow, col: &str) -> bool {
    row.try_get::<bool, _>(col)
        .or_else(|_| row.try_get::<i8, _>(col).map(|v| v != 0))
        .or_else(|_| row.try_get::<u8, _>(col).map(|v| v != 0))
        .unwrap_or(false)
}

pub fn get_row_opt_u8(row: &sqlx::mysql::MySqlRow, col: &str) -> Option<u8> {
    row.try_get::<Option<u8>, _>(col)
        .or_else(|_| {
            row.try_get::<Option<i8>, _>(col)
                .map(|o| o.map(|v| v as u8))
        })
        .ok()
        .flatten()
}

pub fn get_row_opt_u16(row: &sqlx::mysql::MySqlRow, col: &str) -> Option<u16> {
    row.try_get::<Option<u16>, _>(col)
        .or_else(|_| {
            row.try_get::<Option<i16>, _>(col)
                .map(|o| o.map(|v| v as u16))
        })
        .or_else(|_| {
            row.try_get::<Option<i32>, _>(col)
                .map(|o| o.map(|v| v as u16))
        })
        .ok()
        .flatten()
}

pub fn get_row_opt_u32(row: &sqlx::mysql::MySqlRow, col: &str) -> Option<u32> {
    row.try_get::<Option<u32>, _>(col)
        .or_else(|_| {
            row.try_get::<Option<i32>, _>(col)
                .map(|o| o.map(|v| v as u32))
        })
        .ok()
        .flatten()
}

impl VoiceConfig {
    pub fn from_row(r: &sqlx::mysql::MySqlRow) -> Self {
        Self {
            log_enable: get_row_bool(r, "log_enable"),
            add_enable: get_row_bool(r, "add_enable"),
            log_channel: get_row_opt_u64(r, "log_channel"),
            add_channel: get_row_opt_u64(r, "add_channel"),
            default_name: r.try_get("default_name").ok(),
            default_user_limit: get_row_opt_u8(r, "default_user_limit"),
            default_bitrate: get_row_opt_u16(r, "default_bitrate"),
            create_color: get_row_opt_u32(r, "create_color"),
            create_title: r.try_get("create_title").ok(),
            create_desc: r.try_get("create_desc").ok(),
            destroy_color: get_row_opt_u32(r, "destroy_color"),
            destroy_title: r.try_get("destroy_title").ok(),
            destroy_desc: r.try_get("destroy_desc").ok(),
            join_color: get_row_opt_u32(r, "join_color"),
            join_title: r.try_get("join_title").ok(),
            join_desc: r.try_get("join_desc").ok(),
            move_color: get_row_opt_u32(r, "move_color"),
            move_title: r.try_get("move_title").ok(),
            move_desc: r.try_get("move_desc").ok(),
            leave_color: get_row_opt_u32(r, "leave_color"),
            leave_title: r.try_get("leave_title").ok(),
            leave_desc: r.try_get("leave_desc").ok(),
            log_mode: LogMode::from(get_row_opt_u8(r, "log_mode").unwrap_or(0)),
        }
    }
}

pub async fn get_guild_voice_settings(
    pool: &MySqlPool,
    guild_id: u64,
) -> Result<Option<GuildVoiceSettings>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT log_enable, log_channel, add_enable, add_channel, log_mode FROM voice WHERE server = ?",
    )
    .bind(guild_id)
    .fetch_optional(pool)
    .await?;

    let Some(r) = row else {
        return Ok(None);
    };

    let mode_channel_list = get_log_mode_channel_list(pool, guild_id).await?;

    Ok(Some(GuildVoiceSettings {
        log_enable: get_row_bool(&r, "log_enable"),
        log_channel: get_row_opt_u64(&r, "log_channel"),
        add_enable: get_row_bool(&r, "add_enable"),
        add_channel: get_row_opt_u64(&r, "add_channel"),
        log_mode: LogMode::from(get_row_opt_u8(&r, "log_mode").unwrap_or(0)),
        mode_channel_list,
    }))
}

pub async fn get_all_config(
    pool: &MySqlPool,
    server_id: u64,
) -> Result<Option<AllConfig>, sqlx::Error> {
    let row = sqlx::query("SELECT * FROM voice WHERE server = ?")
        .bind(server_id)
        .fetch_optional(pool)
        .await?;

    let Some(r) = row else {
        return Ok(None);
    };

    let voice = VoiceConfig::from_row(&r);
    let log_mode_channel_list = get_log_mode_channel_list(pool, server_id).await?;

    Ok(Some(AllConfig {
        voice,
        log_mode_channel_list,
    }))
}

#[allow(dead_code)]
pub async fn get_log_enable(pool: &MySqlPool, server_id: u64) -> Result<bool, sqlx::Error> {
    let row = sqlx::query("SELECT log_enable FROM voice WHERE server = ?")
        .bind(server_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.is_some_and(|r| get_row_bool(&r, "log_enable")))
}

#[allow(dead_code)]
pub async fn get_add_enable(pool: &MySqlPool, server_id: u64) -> Result<bool, sqlx::Error> {
    let row = sqlx::query("SELECT add_enable FROM voice WHERE server = ?")
        .bind(server_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.is_some_and(|r| get_row_bool(&r, "add_enable")))
}

pub async fn set_log_enable(
    pool: &MySqlPool,
    server_id: u64,
    enable: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO voice (server, log_enable) VALUES (?, ?)
         ON DUPLICATE KEY UPDATE log_enable = VALUES(log_enable)",
    )
    .bind(server_id)
    .bind(enable as i8)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_add_enable(
    pool: &MySqlPool,
    server_id: u64,
    enable: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO voice (server, add_enable) VALUES (?, ?)
         ON DUPLICATE KEY UPDATE add_enable = VALUES(add_enable)",
    )
    .bind(server_id)
    .bind(enable as i8)
    .execute(pool)
    .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn get_log_channel(pool: &MySqlPool, server_id: u64) -> Result<Option<u64>, sqlx::Error> {
    let row = sqlx::query("SELECT log_channel FROM voice WHERE server = ?")
        .bind(server_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.and_then(|r| get_row_opt_u64(&r, "log_channel")))
}

#[allow(dead_code)]
pub async fn get_add_channel(pool: &MySqlPool, server_id: u64) -> Result<Option<u64>, sqlx::Error> {
    let row = sqlx::query("SELECT add_channel FROM voice WHERE server = ?")
        .bind(server_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.and_then(|r| get_row_opt_u64(&r, "add_channel")))
}

pub async fn set_log_channel(
    pool: &MySqlPool,
    server_id: u64,
    channel_id: Option<u64>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO voice (server, log_channel) VALUES (?, ?)
         ON DUPLICATE KEY UPDATE log_channel = VALUES(log_channel)",
    )
    .bind(server_id)
    .bind(channel_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_add_channel(
    pool: &MySqlPool,
    server_id: u64,
    channel_id: Option<u64>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO voice (server, add_channel) VALUES (?, ?)
         ON DUPLICATE KEY UPDATE add_channel = VALUES(add_channel)",
    )
    .bind(server_id)
    .bind(channel_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_server(pool: &MySqlPool, server_id: u64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("INSERT IGNORE INTO voice (server) VALUES (?)")
        .bind(server_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() == 0)
}

pub async fn delete_server(pool: &MySqlPool, server_id: u64) -> Result<bool, sqlx::Error> {
    sqlx::query("DELETE FROM voice_mode_channel_list WHERE server_id = ?")
        .bind(server_id)
        .execute(pool)
        .await?;

    let result = sqlx::query("DELETE FROM voice WHERE server = ?")
        .bind(server_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn get_voice_defaults(
    pool: &MySqlPool,
    server_id: u64,
) -> Result<VoiceDefaults, sqlx::Error> {
    let row = sqlx::query(
        "SELECT default_name, default_user_limit, default_bitrate FROM voice WHERE server = ?",
    )
    .bind(server_id)
    .fetch_optional(pool)
    .await?;

    Ok(match row {
        Some(r) => VoiceDefaults {
            name: r.try_get("default_name").ok(),
            user_limit: get_row_opt_u8(&r, "default_user_limit"),
            bitrate: get_row_opt_u16(&r, "default_bitrate"),
        },
        None => VoiceDefaults::default(),
    })
}

pub async fn set_voice_defaults(
    pool: &MySqlPool,
    server_id: u64,
    defaults: &VoiceDefaults,
) -> Result<(), sqlx::Error> {
    let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new("UPDATE voice SET ");
    let mut added = false;
    let mut separated = query_builder.separated(", ");

    if let Some(ref name) = defaults.name {
        separated.push("default_name = ");
        separated.push_bind(name);
        added = true;
    }

    if let Some(limit) = defaults.user_limit {
        separated.push("default_user_limit = ");
        separated.push_bind(limit);
        added = true;
    }

    if let Some(bitrate) = defaults.bitrate {
        separated.push("default_bitrate = ");
        separated.push_bind(bitrate);
        added = true;
    }

    if !added {
        return Ok(());
    }

    query_builder.push(" WHERE server = ");
    query_builder.push_bind(server_id);
    query_builder.build().execute(pool).await?;

    Ok(())
}

impl VoiceEventType {
    pub async fn get_log_config(
        self,
        pool: &MySqlPool,
        server_id: u64,
    ) -> Result<Option<LogConfig>, sqlx::Error> {
        let sql = match self {
            VoiceEventType::Add => {
                "SELECT create_title AS title, create_desc AS description, create_color AS color FROM voice WHERE server = ?"
            }
            VoiceEventType::Destroy => {
                "SELECT destroy_title AS title, destroy_desc AS description, destroy_color AS color FROM voice WHERE server = ?"
            }
            VoiceEventType::Join => {
                "SELECT join_title AS title, join_desc AS description, join_color AS color FROM voice WHERE server = ?"
            }
            VoiceEventType::Move => {
                "SELECT move_title AS title, move_desc AS description, move_color AS color FROM voice WHERE server = ?"
            }
            VoiceEventType::Leave => {
                "SELECT leave_title AS title, leave_desc AS description, leave_color AS color FROM voice WHERE server = ?"
            }
        };
        let row = sqlx::query(sql)
            .bind(server_id)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(|r| LogConfig {
            title: r.try_get("title").ok(),
            description: r.try_get("description").ok(),
            color: get_row_opt_u32(&r, "color"),
        }))
    }

    pub async fn set_log_config(
        &self,
        pool: &MySqlPool,
        server_id: u64,
        config: &LogConfig,
    ) -> Result<(), sqlx::Error> {
        let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new("UPDATE voice SET ");
        let mut separated = query_builder.separated(", ");
        let mut added = false;

        if let Some(ref title) = config.title {
            separated.push(format!("{}_title = ", self.as_str()));
            separated.push_bind(title);
            added = true;
        }

        if let Some(ref desc) = config.description {
            separated.push(format!("{}_desc = ", self.as_str()));
            separated.push_bind(desc);
            added = true;
        }

        if let Some(color) = config.color {
            separated.push(format!("{}_color = ", self.as_str()));
            separated.push_bind(color);
            added = true;
        }

        if !added {
            return Ok(());
        }

        query_builder.push(" WHERE server = ");
        query_builder.push_bind(server_id);
        query_builder.build().execute(pool).await?;

        Ok(())
    }
}

pub async fn server_exists(pool: &MySqlPool, server_id: u64) -> Result<bool, sqlx::Error> {
    let row = sqlx::query("SELECT 1 as one FROM voice WHERE server = ? LIMIT 1")
        .bind(server_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}

pub async fn reset_all_voice_configs(pool: &MySqlPool, server_id: u64) -> Result<(), sqlx::Error> {
    let event_types = ["create", "destroy", "join", "move", "leave"];
    let fields = ["title", "desc", "color"];

    let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new("UPDATE voice SET ");
    let mut separated = query_builder.separated(", ");

    for event in &event_types {
        for field in &fields {
            separated.push(format!("{}_{} = NULL", event, field));
        }
    }

    query_builder.push(" WHERE server = ");
    query_builder.push_bind(server_id);
    query_builder.build().execute(pool).await?;

    Ok(())
}

pub async fn get_log_mode_channel_list(
    pool: &MySqlPool,
    server_id: u64,
) -> Result<Vec<u64>, sqlx::Error> {
    let rows = sqlx::query("SELECT channel_id FROM voice_mode_channel_list WHERE server_id = ?")
        .bind(server_id)
        .fetch_all(pool)
        .await?;

    Ok(rows
        .iter()
        .filter_map(|r| get_row_u64(r, "channel_id"))
        .collect())
}

pub async fn add_log_mode_channel_list(
    pool: &MySqlPool,
    server_id: u64,
    channel_id: u64,
) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query(
        "SELECT 1 FROM voice_mode_channel_list WHERE server_id = ? AND channel_id = ? LIMIT 1",
    )
    .bind(server_id)
    .bind(channel_id)
    .fetch_optional(pool)
    .await?
    .is_some();

    if exists {
        return Ok(false);
    }

    sqlx::query("INSERT INTO voice_mode_channel_list (server_id, channel_id) VALUES (?, ?)")
        .bind(server_id)
        .bind(channel_id)
        .execute(pool)
        .await?;

    Ok(true)
}

pub async fn delete_log_mode_channel_list(
    pool: &MySqlPool,
    server_id: u64,
    channel_id: u64,
) -> Result<bool, sqlx::Error> {
    let result =
        sqlx::query("DELETE FROM voice_mode_channel_list WHERE server_id = ? AND channel_id = ?")
            .bind(server_id)
            .bind(channel_id)
            .execute(pool)
            .await?;

    Ok(result.rows_affected() > 0)
}

#[allow(dead_code)]
pub async fn get_log_mode(pool: &MySqlPool, server_id: u64) -> Result<LogMode, sqlx::Error> {
    let row = sqlx::query("SELECT log_mode FROM voice WHERE server = ?")
        .bind(server_id)
        .fetch_optional(pool)
        .await?;

    let val = row
        .and_then(|r| get_row_opt_u8(&r, "log_mode"))
        .unwrap_or(0);
    Ok(LogMode::from(val))
}

pub async fn set_log_mode(
    pool: &MySqlPool,
    server_id: u64,
    mode: LogMode,
) -> Result<bool, sqlx::Error> {
    let mode_val = mode as u8;

    let result = sqlx::query("UPDATE voice SET log_mode = ? WHERE server = ?")
        .bind(mode_val)
        .bind(server_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}
