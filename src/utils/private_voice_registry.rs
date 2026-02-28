use std::collections::HashMap;

use serenity::all::{ChannelId, UserId};

pub struct PrivateVoiceRegistry {
    owners: HashMap<ChannelId, UserId>,
}

impl PrivateVoiceRegistry {
    pub fn new() -> Self {
        Self {
            owners: HashMap::new(),
        }
    }

    pub fn set_owner(&mut self, channel_id: ChannelId, user_id: UserId) {
        self.owners.insert(channel_id, user_id);
    }

    pub fn get_owner(&self, channel_id: ChannelId) -> Option<UserId> {
        self.owners.get(&channel_id).copied()
    }

    pub fn delete_owner(&mut self, channel_id: ChannelId) {
        self.owners.remove(&channel_id);
    }
}

impl Default for PrivateVoiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
