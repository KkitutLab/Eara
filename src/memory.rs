use std::sync::Arc;
use tokio::sync::Mutex;

use crate::handler::Handler;

impl Handler {
    pub async fn add_voice_entry(
        &self,
        guild_id: u64,
        channel_id: u64,
        user_id: u64,
        blacklist: bool,
    ) {
        let mut voice_map = self.voice_data.write().await;

        let voice_vec_mutex = voice_map
            .entry(guild_id)
            .or_insert_with(|| Arc::new(Mutex::new(Vec::new())));

        let mut voice_vec = voice_vec_mutex.lock().await;

        voice_vec.push((channel_id, user_id, blacklist));
    }

    #[allow(dead_code)]
    pub async fn get_voice_entry(
        &self,
        guild_id: u64,
        channel_id: u64,
    ) -> Option<(u64, u64, bool)> {
        let voice_map = self.voice_data.read().await;

        if let Some(voice_vec_mutex) = voice_map.get(&guild_id) {
            let voice_vec = voice_vec_mutex.lock().await;
            // 특정 채널 ID와 일치하는 첫 번째 엔트리를 찾아서 복사본 반환
            voice_vec
                .iter()
                .find(|(tc_id, _, _)| *tc_id == channel_id)
                .cloned()
        } else {
            None
        }
    }

    pub async fn get_voice_entries(&self, guild_id: u64) -> Vec<(u64, u64, bool)> {
        let voice_map = self.voice_data.read().await;

        if let Some(voice_vec_mutex) = voice_map.get(&guild_id) {
            let voice_vec = voice_vec_mutex.lock().await;
            voice_vec.clone()
        } else {
            Vec::new()
        }
    }

    pub async fn remove_voice_entry_by_channel(
        &self,
        guild_id: u64,
        channel_id: u64,
    ) -> Option<bool> {
        let voice_map = self.voice_data.read().await;

        if let Some(voice_vec_mutex) = voice_map.get(&guild_id) {
            let mut voice_vec = voice_vec_mutex.lock().await;

            let removed_item = voice_vec
                .iter()
                .find(|&&(c_id, _, _)| c_id == channel_id)
                .map(|&(_, _, b)| b);

            voice_vec.retain(|&(c_id, _, _)| c_id != channel_id);

            removed_item
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub async fn remove_guild_voice_data(&self, guild_id: u64) {
        let mut voice_map = self.voice_data.write().await;
        voice_map.remove(&guild_id);
    }

    #[allow(dead_code)]
    pub async fn update_voice_entry(
        &self,
        guild_id: u64,
        old_entry: (u64, u64),
        new_entry: (u64, u64),
    ) -> bool {
        let voice_map = self.voice_data.read().await;

        if let Some(voice_vec_mutex) = voice_map.get(&guild_id) {
            let mut voice_vec = voice_vec_mutex.lock().await;

            if let Some(pos) = voice_vec
                .iter()
                .position(|&(x1, x2, _)| (x1, x2) == old_entry)
            {
                let (_, _, flag) = voice_vec[pos];
                voice_vec[pos] = (new_entry.0, new_entry.1, flag);
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}
