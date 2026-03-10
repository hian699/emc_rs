use std::collections::HashSet;

use serenity::all::{ChannelId, GuildId, RoleId};

#[derive(Clone, Debug)]
pub struct GuildSettings {
    pub guild_id: GuildId,
    pub admin_role_ids: HashSet<RoleId>,
    pub developer_role_ids: HashSet<RoleId>,
    pub music_text_channel_ids: HashSet<ChannelId>,
    pub private_voice_allowed_channel_ids: HashSet<ChannelId>,
    pub temp_voice_category_id: Option<ChannelId>,
    pub temp_voice_public_lobby_channel_id: Option<ChannelId>,
    pub temp_voice_private_lobby_channel_id: Option<ChannelId>,
    pub mod_channel_id: Option<ChannelId>,
}

impl GuildSettings {
    pub fn new(guild_id: GuildId) -> Self {
        Self {
            guild_id,
            admin_role_ids: HashSet::new(),
            developer_role_ids: HashSet::new(),
            music_text_channel_ids: HashSet::new(),
            private_voice_allowed_channel_ids: HashSet::new(),
            temp_voice_category_id: None,
            temp_voice_public_lobby_channel_id: None,
            temp_voice_private_lobby_channel_id: None,
            mod_channel_id: None,
        }
    }

    pub fn allow_music_channel(&self, channel_id: ChannelId) -> bool {
        self.music_text_channel_ids.is_empty() || self.music_text_channel_ids.contains(&channel_id)
    }

    pub fn allow_private_voice_channel(&self, channel_id: ChannelId) -> bool {
        self.private_voice_allowed_channel_ids.is_empty()
            || self.private_voice_allowed_channel_ids.contains(&channel_id)
    }

    pub fn parse_ids_csv(input: &str) -> Vec<u64> {
        input
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<u64>().ok())
            .collect()
    }

    pub fn serialize_role_ids(ids: &HashSet<RoleId>) -> String {
        let mut v: Vec<u64> = ids.iter().map(|id| id.get()).collect();
        v.sort_unstable();
        v.iter().map(u64::to_string).collect::<Vec<_>>().join(",")
    }

    pub fn serialize_channel_ids(ids: &HashSet<ChannelId>) -> String {
        let mut v: Vec<u64> = ids.iter().map(|id| id.get()).collect();
        v.sort_unstable();
        v.iter().map(u64::to_string).collect::<Vec<_>>().join(",")
    }

    pub fn parse_role_ids(input: &str) -> HashSet<RoleId> {
        Self::parse_ids_csv(input)
            .into_iter()
            .map(RoleId::new)
            .collect()
    }

    pub fn parse_channel_ids(input: &str) -> HashSet<ChannelId> {
        Self::parse_ids_csv(input)
            .into_iter()
            .map(ChannelId::new)
            .collect()
    }
}
