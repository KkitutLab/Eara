pub mod interaction;
pub mod ready;
pub mod voice;

use serenity::all::{Interaction, Ready, VoiceState};
use serenity::async_trait;
use serenity::prelude::{Context, EventHandler, RwLock};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::Mutex;

use crate::db::get_pool;
use crate::macros::er;
use interaction::handle_interaction;
use ready::handle_ready;
use voice::handle_voice_state_update;

pub type VoiceDataStore = Arc<RwLock<HashMap<u64, Arc<Mutex<Vec<(u64, u64, bool)>>>>>>;

pub struct Handler {
    pub is_ready: Arc<AtomicBool>,
    pub voice_data: VoiceDataStore,
}

impl Default for Handler {
    fn default() -> Self {
        Self::new()
    }
}

impl Handler {
    pub fn new() -> Self {
        Self {
            is_ready: Arc::new(AtomicBool::new(false)),
            voice_data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        let pool = er!(get_pool().await, "[ERROR] voice_state_update get_pool()");
        handle_voice_state_update(self, &ctx, &old, &new, pool).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let pool = er!(get_pool().await, "[ERROR] interaction get_pool()");
        handle_interaction(self, &ctx, interaction, pool).await;
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        handle_ready(&ctx, ready, &self.is_ready).await;
    }
}
