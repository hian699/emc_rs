use std::collections::HashMap;
use std::sync::Arc;

use serenity::all::{ChannelId, GuildId};
use tokio::sync::RwLock;

use crate::utils::music_queue::MusicQueue;

pub struct MusicManager {
    queues: RwLock<HashMap<GuildId, Arc<RwLock<MusicQueue>>>>,
}

impl MusicManager {
    pub fn new() -> Self {
        Self {
            queues: RwLock::new(HashMap::new()),
        }
    }

    pub fn constructor() -> Self {
        Self::new()
    }

    pub async fn get_queue(&self, guild_id: GuildId) -> Option<Arc<RwLock<MusicQueue>>> {
        self.queues.read().await.get(&guild_id).cloned()
    }

    pub async fn has_queue(&self, guild_id: GuildId) -> bool {
        self.queues.read().await.contains_key(&guild_id)
    }

    pub async fn create_queue(
        &self,
        guild_id: GuildId,
        text_channel_id: ChannelId,
    ) -> Arc<RwLock<MusicQueue>> {
        let queue = Arc::new(RwLock::new(MusicQueue::constructor(
            guild_id,
            text_channel_id,
        )));
        self.queues.write().await.insert(guild_id, queue.clone());
        queue
    }

    pub async fn delete_queue(&self, guild_id: GuildId) {
        self.queues.write().await.remove(&guild_id);
    }

    pub async fn get_all_queues(&self) -> Vec<Arc<RwLock<MusicQueue>>> {
        self.queues.read().await.values().cloned().collect()
    }
}

impl Default for MusicManager {
    fn default() -> Self {
        Self::new()
    }
}
