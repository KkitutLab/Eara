use std::ops::Deref;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceEventType {
    Add,
    Destroy,
    Join,
    Move,
    Leave,
}

impl VoiceEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            VoiceEventType::Add => "create",
            VoiceEventType::Destroy => "destroy",
            VoiceEventType::Join => "join",
            VoiceEventType::Move => "move",
            VoiceEventType::Leave => "leave",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum LogMode {
    #[default]
    None = 0,
    BlackList = 1,
    WhiteList = 2,
}

impl From<u8> for LogMode {
    fn from(value: u8) -> Self {
        match value {
            1 => LogMode::BlackList,
            2 => LogMode::WhiteList,
            _ => LogMode::None,
        }
    }
}

impl std::fmt::Display for LogMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            LogMode::None => "None",
            LogMode::BlackList => "BlackList",
            LogMode::WhiteList => "WhiteList",
        };
        write!(f, "{}", name)
    }
}

#[derive(Debug, Clone, Default)]
pub struct VoiceDefaults {
    pub name: Option<String>,
    pub user_limit: Option<u8>,
    pub bitrate: Option<u16>,
}

#[derive(Debug, Clone, Default)]
pub struct LogConfig {
    pub title: Option<String>,
    pub description: Option<String>,
    pub color: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct VoiceConfig {
    pub log_enable: bool,
    pub add_enable: bool,
    pub log_channel: Option<u64>,
    pub add_channel: Option<u64>,
    pub default_name: Option<String>,
    pub default_user_limit: Option<u8>,
    pub default_bitrate: Option<u16>,
    pub create_color: Option<u32>,
    pub create_title: Option<String>,
    pub create_desc: Option<String>,
    pub destroy_color: Option<u32>,
    pub destroy_title: Option<String>,
    pub destroy_desc: Option<String>,
    pub join_color: Option<u32>,
    pub join_title: Option<String>,
    pub join_desc: Option<String>,
    pub move_color: Option<u32>,
    pub move_title: Option<String>,
    pub move_desc: Option<String>,
    pub leave_color: Option<u32>,
    pub leave_title: Option<String>,
    pub leave_desc: Option<String>,
    pub log_mode: LogMode,
}

#[derive(Debug, Clone, Default)]
pub struct AllConfig {
    pub voice: VoiceConfig,
    pub log_mode_channel_list: Vec<u64>,
}

impl Deref for AllConfig {
    type Target = VoiceConfig;

    fn deref(&self) -> &Self::Target {
        &self.voice
    }
}

#[derive(Debug, Clone)]
pub struct GuildVoiceSettings {
    pub log_enable: bool,
    pub log_channel: Option<u64>,
    pub add_enable: bool,
    pub add_channel: Option<u64>,
    pub log_mode: LogMode,
    pub mode_channel_list: Vec<u64>,
}
