use std::collections::VecDeque;
use std::time::Duration;

use anyhow::Context as _;
#[cfg(feature = "lavalink")]
use lavalink_rs::model::track::{TrackData, TrackInfo};
use serenity::all::{ChannelId, GuildId};
use serenity::client::Context;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message as WebSocketMessage;

use crate::get_lavalink_client;
use crate::get_state;
use crate::utils::discord_embed::error_embed;
#[cfg(feature = "lavalink")]
use crate::utils::lavalink_client::try_create_player_context;

const IDLE_DISCONNECT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug)]
pub struct SongItem {
    pub title: String,
    pub url: String,
    pub duration_ms: Option<u64>,
    pub requested_by: String,
    pub lavalink_encoded_track: Option<String>,
}

pub struct MusicQueue {
    pub guild_id: GuildId,
    pub text_channel_id: ChannelId,
    songs: VecDeque<SongItem>,
    current: Option<SongItem>,
    disconnect_timeout: Option<JoinHandle<()>>,
}

impl MusicQueue {
    pub fn constructor(guild_id: GuildId, text_channel_id: ChannelId) -> Self {
        Self {
            guild_id,
            text_channel_id,
            songs: VecDeque::new(),
            current: None,
            disconnect_timeout: None,
        }
    }

    pub async fn connect(_ctx: &Context, _channel_id: ChannelId) -> anyhow::Result<()> {
        #[cfg(feature = "lavalink")]
        {
            let channel = _channel_id
                .to_channel(&_ctx.http)
                .await
                .context("Failed to resolve channel")?;
            let guild_channel = channel.guild().context("Target is not a guild channel")?;
            let bot_user_id = _ctx.cache.current_user().id;

            if guild_channel.kind != serenity::all::ChannelType::Voice
                && guild_channel.kind != serenity::all::ChannelType::Stage
            {
                anyhow::bail!("Target channel is not a voice or stage channel")
            }

            let current_bot_channel_id = guild_channel
                .guild_id
                .to_guild_cached(&_ctx.cache)
                .and_then(|guild| {
                    guild
                        .voice_states
                        .get(&bot_user_id)
                        .and_then(|state| state.channel_id)
                });

            if current_bot_channel_id != Some(_channel_id) {
                let payload = serde_json::json!({
                    "op": 4,
                    "d": {
                        "guild_id": guild_channel.guild_id.get().to_string(),
                        "channel_id": _channel_id.get().to_string(),
                        "self_mute": false,
                        "self_deaf": false
                    }
                });

                _ctx.shard
                    .websocket_message(WebSocketMessage::Text(payload.to_string().into()));

                tokio::time::timeout(Duration::from_secs(5), async {
                    loop {
                        let bot_channel_id = guild_channel
                            .guild_id
                            .to_guild_cached(&_ctx.cache)
                            .and_then(|guild| {
                                guild
                                    .voice_states
                                    .get(&bot_user_id)
                                    .and_then(|state| state.channel_id)
                            });

                        if bot_channel_id == Some(_channel_id) {
                            break;
                        }

                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                })
                .await
                .with_context(|| {
                    format!(
                        "Timed out while waiting for bot voice state update into channel {} after sending gateway VOICE_STATE_UPDATE",
                        _channel_id.get()
                    )
                })?;
            }
        }

        Ok(())
    }

    async fn disconnect_from_voice(ctx: &Context, guild_id: GuildId) -> anyhow::Result<()> {
        #[cfg(feature = "lavalink")]
        if let Some(client) = get_lavalink_client(ctx).await? {
            client
                .delete_player(guild_id)
                .await
                .context("Failed to delete lavalink player")?;
        }

        let payload = serde_json::json!({
            "op": 4,
            "d": {
                "guild_id": guild_id.get().to_string(),
                "channel_id": null,
                "self_mute": false,
                "self_deaf": false
            }
        });

        ctx.shard
            .websocket_message(WebSocketMessage::Text(payload.to_string().into()));

        Ok(())
    }

    fn is_idle(&self) -> bool {
        self.current.is_none() && self.songs.is_empty()
    }

    fn refresh_disconnect_timeout(&mut self, ctx: &Context) {
        if self.is_idle() {
            self.start_disconnect_timeout(ctx, IDLE_DISCONNECT_TIMEOUT);
        } else {
            self.clear_disconnect_timeout();
        }
    }

    pub fn enqueue_song(&mut self, song: SongItem) -> bool {
        self.clear_disconnect_timeout();
        let is_first = self.current.is_none();
        if is_first {
            self.current = Some(song.clone());
        }
        self.songs.push_back(song);

        is_first
    }

    pub async fn sync_lavalink_enqueue(
        _ctx: &Context,
        guild_id: GuildId,
        song: &SongItem,
        play_now: bool,
    ) -> anyhow::Result<()> {
        #[cfg(feature = "lavalink")]
        {
            if let Some(client) = get_lavalink_client(_ctx).await? {
                let encoded = song.lavalink_encoded_track.clone();
                if let Some(encoded) = encoded {
                    let player = if let Some(existing) = client.get_player_context(guild_id) {
                        existing
                    } else {
                        try_create_player_context(&client, guild_id).await?
                    };

                    let track = track_from_song(song.clone(), encoded);
                    player.queue(track.clone())?;
                    if play_now {
                        player.play_now(&track).await?;
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn play(&mut self, _ctx: &Context) -> anyhow::Result<Option<SongItem>> {
        Ok(self.current.clone())
    }

    pub async fn handle_song_end(&mut self, ctx: &Context) -> anyhow::Result<Option<SongItem>> {
        self.songs.pop_front();
        self.current = self.songs.front().cloned();
        self.refresh_disconnect_timeout(ctx);
        self.play(ctx).await
    }

    pub async fn skip(&mut self, ctx: &Context) -> anyhow::Result<()> {
        self.songs.pop_front();
        self.current = self.songs.front().cloned();
        self.refresh_disconnect_timeout(ctx);
        Ok(())
    }

    pub async fn stop(&mut self, ctx: &Context) -> anyhow::Result<()> {
        self.songs.clear();
        self.current = None;
        self.refresh_disconnect_timeout(ctx);
        Ok(())
    }

    pub async fn destroy(&mut self, ctx: &Context) -> anyhow::Result<()> {
        self.clear_disconnect_timeout();
        self.songs.clear();
        self.current = None;
        Self::disconnect_from_voice(ctx, self.guild_id).await
    }

    pub async fn send_error(&self, ctx: &Context, message: &str) -> anyhow::Result<()> {
        self.text_channel_id
            .send_message(
                &ctx.http,
                serenity::all::CreateMessage::new().embed(error_embed("Music Error", message)),
            )
            .await
            .context("Failed to send error message")?;
        Ok(())
    }

    pub fn start_disconnect_timeout(&mut self, ctx: &Context, timeout: Duration) {
        self.clear_disconnect_timeout();
        let ctx = ctx.clone();
        let guild_id = self.guild_id;
        self.disconnect_timeout = Some(tokio::spawn(async move {
            tokio::time::sleep(timeout).await;

            let Ok(state) = get_state(&ctx).await else {
                return;
            };

            let Some(queue) = state.music_manager.get_queue(guild_id).await else {
                return;
            };

            let mut queue = queue.write().await;
            if !queue.is_idle() {
                return;
            }

            queue.disconnect_timeout = None;
            if queue.destroy(&ctx).await.is_ok() {
                drop(queue);
                state.music_manager.delete_queue(guild_id).await;
            }
        }));
    }

    pub fn clear_disconnect_timeout(&mut self) {
        if let Some(handle) = self.disconnect_timeout.take() {
            handle.abort();
        }
    }

    pub fn get_queue_info(&self) -> String {
        let now_playing = self
            .current
            .as_ref()
            .map(|s| format!("{} (req by {})", s.title, s.requested_by))
            .unwrap_or_else(|| "Nothing".to_string());
        format!("Now playing: {now_playing} | Pending: {}", self.songs.len())
    }
}

#[cfg(feature = "lavalink")]
fn track_from_song(song: SongItem, encoded: String) -> TrackData {
    TrackData {
        encoded,
        info: TrackInfo {
            identifier: song.url.clone(),
            is_seekable: true,
            author: song.requested_by,
            length: song.duration_ms.unwrap_or(0),
            is_stream: false,
            position: 0,
            title: song.title,
            uri: Some(song.url),
            artwork_url: None,
            isrc: None,
            source_name: "lavalink".to_string(),
        },
        plugin_info: None,
        user_data: None,
    }
}
