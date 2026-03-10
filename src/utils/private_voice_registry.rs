use std::collections::HashMap;

use serenity::all::{ChannelId, UserId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TempVoiceChannelKind {
    Public,
    Private,
}

#[derive(Clone, Copy, Debug)]
pub struct TempVoiceChannelEntry {
    pub owner: UserId,
    pub kind: TempVoiceChannelKind,
}

pub struct PrivateVoiceRegistry {
    channels: HashMap<ChannelId, TempVoiceChannelEntry>,
}

impl PrivateVoiceRegistry {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    pub fn set_channel(
        &mut self,
        channel_id: ChannelId,
        user_id: UserId,
        kind: TempVoiceChannelKind,
    ) {
        self.channels.insert(
            channel_id,
            TempVoiceChannelEntry {
                owner: user_id,
                kind,
            },
        );
    }

    pub fn get_entry(&self, channel_id: ChannelId) -> Option<TempVoiceChannelEntry> {
        self.channels.get(&channel_id).copied()
    }

    pub fn get_owner(&self, channel_id: ChannelId) -> Option<UserId> {
        self.get_entry(channel_id).map(|entry| entry.owner)
    }

    pub fn is_private(&self, channel_id: ChannelId) -> bool {
        matches!(
            self.get_entry(channel_id).map(|entry| entry.kind),
            Some(TempVoiceChannelKind::Private)
        )
    }

    pub fn delete_owner(&mut self, channel_id: ChannelId) {
        self.channels.remove(&channel_id);
    }
}

impl Default for PrivateVoiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
