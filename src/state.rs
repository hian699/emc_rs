use std::sync::Arc;

use tokio::sync::RwLock;

#[cfg(feature = "lavalink")]
use lavalink_rs::prelude::LavalinkClient;

use crate::utils::music_manager::MusicManager;
use crate::utils::private_voice_registry::PrivateVoiceRegistry;
use crate::utils::search_cache::SearchCache;
use crate::utils::settings_repository::SettingsRepository;

pub struct BotState {
    pub settings_repo: Arc<SettingsRepository>,
    pub music_manager: Arc<MusicManager>,
    pub search_cache: Arc<RwLock<SearchCache>>,
    pub private_voice_registry: Arc<RwLock<PrivateVoiceRegistry>>,
    #[cfg(feature = "lavalink")]
    pub lavalink_client: Arc<RwLock<Option<LavalinkClient>>>,
}
