use std::sync::Arc;
#[cfg(feature = "lavalink")]
use std::time::Instant;

use tokio::sync::{Mutex, RwLock};

#[cfg(feature = "lavalink")]
use lavalink_rs::prelude::LavalinkClient;

use crate::utils::music_manager::MusicManager;
use crate::utils::private_voice_registry::PrivateVoiceRegistry;
use crate::utils::search_cache::SearchCache;
use crate::utils::settings_repository::SettingsRepository;

#[cfg(feature = "lavalink")]
#[derive(Default)]
pub struct LavalinkRuntimeState {
    pub client: Option<LavalinkClient>,
    pub retry_after: Option<Instant>,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
}

pub struct BotState {
    pub settings_repo: Arc<SettingsRepository>,
    pub music_manager: Arc<MusicManager>,
    pub search_cache: Arc<RwLock<SearchCache>>,
    pub private_voice_registry: Arc<RwLock<PrivateVoiceRegistry>>,
    #[cfg(feature = "lavalink")]
    pub lavalink_runtime: Arc<RwLock<LavalinkRuntimeState>>,
    #[cfg(feature = "lavalink")]
    pub lavalink_init_lock: Arc<Mutex<()>>,
}
