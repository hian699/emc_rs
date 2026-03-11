use std::collections::VecDeque;
use std::time::{Duration, Instant};

use anyhow::Context as _;
#[cfg(feature = "lavalink")]
use lavalink_rs::model::http::{UpdatePlayer, UpdatePlayerTrack};
#[cfg(feature = "lavalink")]
use lavalink_rs::model::track::{TrackData, TrackInfo};
use serenity::all::{ChannelId, GuildId};
use serenity::client::Context;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message as WebSocketMessage;

use crate::get_lavalink_client;
use crate::get_state;
use crate::utils::discord_embed::error_embed;

const IDLE_DISCONNECT_TIMEOUT: Duration = Duration::from_secs(30);
pub const AUTO_LEAVE_SUPPRESSION_WINDOW: Duration = Duration::from_secs(20);

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
    is_playing: bool,
    disconnect_timeout: Option<JoinHandle<()>>,
    auto_leave_suppressed_until: Option<Instant>,
}

impl MusicQueue {
    pub fn constructor(guild_id: GuildId, text_channel_id: ChannelId) -> Self {
        Self {
            guild_id,
            text_channel_id,
            songs: VecDeque::new(),
            current: None,
            is_playing: false,
            disconnect_timeout: None,
            auto_leave_suppressed_until: None,
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
            let state = get_state(_ctx).await?;

            if let Some(queue) = state.music_manager.get_queue(guild_channel.guild_id).await {
                queue
                    .write()
                    .await
                    .suppress_auto_leave(AUTO_LEAVE_SUPPRESSION_WINDOW);
            }

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

            // Start listening for voice connection info BEFORE sending OP4,
            // so the listener is registered when Discord fires the voice events.
            let lavalink_client = get_lavalink_client(_ctx).await?;
            let guild_id = guild_channel.guild_id;
            let connection_info_fut = if let Some(ref client) = lavalink_client {
                let client = client.clone();
                Some(tokio::spawn(async move {
                    client
                        .get_connection_info(guild_id, Duration::from_secs(10))
                        .await
                }))
            } else {
                None
            };

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

            // Now collect the connection info and create the Lavalink player.
            // By this point the voice events have been sent by Discord and should
            // have been received by the lavalink-rs internal handler.
            if let (Some(client), Some(fut)) = (lavalink_client, connection_info_fut) {
                let connection_info = fut
                    .await
                    .context("get_connection_info task panicked")?
                    .context("Timed out waiting for Discord voice events (VOICE_SERVER_UPDATE / VOICE_STATE_UPDATE). Make sure both events are forwarded to lavalink-rs.")?;

                client
                    .create_player(guild_id, connection_info)
                    .await
                    .context("Failed to create Lavalink voice player")?;
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

    pub fn suppress_auto_leave(&mut self, duration: Duration) {
        let until = Instant::now() + duration;
        self.auto_leave_suppressed_until = Some(
            self.auto_leave_suppressed_until
                .map_or(until, |current| current.max(until)),
        );
    }

    pub fn is_auto_leave_suppressed(&self) -> bool {
        self.auto_leave_suppressed_until
            .is_some_and(|until| until > Instant::now())
    }

    pub fn mark_playing(&mut self) {
        self.is_playing = true;
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
        let should_start = !self.is_playing;
        if self.current.is_none() {
            self.current = Some(song.clone());
        }
        self.songs.push_back(song);

        should_start
    }

    pub async fn sync_lavalink_enqueue(
        _ctx: &Context,
        guild_id: GuildId,
        _song: &SongItem,
        play_now: bool,
    ) -> anyhow::Result<()> {
        let state = get_state(_ctx).await?;
        let mut song_to_start = None;
        if let Some(queue) = state.music_manager.get_queue(guild_id).await {
            let mut queue = queue.write().await;
            queue.suppress_auto_leave(AUTO_LEAVE_SUPPRESSION_WINDOW);
            if play_now {
                song_to_start = queue.current.clone();
            }
        }

        #[cfg(feature = "lavalink")]
        {
            if let Some(song) = song_to_start {
                Self::play_song_now(_ctx, guild_id, &song).await?;
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
        self.is_playing = false;
        self.refresh_disconnect_timeout(ctx);

        if let Some(song) = self.current.clone() {
            Self::play_song_now(ctx, self.guild_id, &song).await?;
        }

        self.play(ctx).await
    }

    pub async fn skip(&mut self, ctx: &Context) -> anyhow::Result<()> {
        self.songs.pop_front();
        self.current = self.songs.front().cloned();
        self.is_playing = false;
        self.refresh_disconnect_timeout(ctx);

        if let Some(song) = self.current.clone() {
            Self::play_song_now(ctx, self.guild_id, &song).await?;
        } else {
            Self::stop_remote_playback(ctx, self.guild_id).await?;
        }

        Ok(())
    }

    pub async fn stop(&mut self, ctx: &Context) -> anyhow::Result<()> {
        self.songs.clear();
        self.current = None;
        self.is_playing = false;
        self.refresh_disconnect_timeout(ctx);
        Self::stop_remote_playback(ctx, self.guild_id).await?;
        Ok(())
    }

    #[cfg(feature = "lavalink")]
    async fn play_song_now(ctx: &Context, guild_id: GuildId, song: &SongItem) -> anyhow::Result<()> {
        let Some(client) = get_lavalink_client(ctx).await? else {
            anyhow::bail!("Lavalink client is not available")
        };

        let Some(encoded) = song.lavalink_encoded_track.clone() else {
            anyhow::bail!("Selected track does not have a Lavalink encoded stream")
        };

        let track = track_from_song(song.clone(), encoded);
        let update = UpdatePlayer {
            track: Some(UpdatePlayerTrack {
                encoded: Some(track.encoded.clone()),
                user_data: track.user_data.clone(),
                ..Default::default()
            }),
            paused: Some(false),
            ..Default::default()
        };

        client
            .update_player(guild_id, &update, false)
            .await
            .context("Failed to update Lavalink player for current song")?;
        Ok(())
    }


    #[cfg(feature = "lavalink")]
    async fn stop_remote_playback(ctx: &Context, guild_id: GuildId) -> anyhow::Result<()> {
        let Some(client) = get_lavalink_client(ctx).await? else {
            return Ok(());
        };

        client
            .update_player(
                guild_id,
                &UpdatePlayer {
                    track: Some(UpdatePlayerTrack::default()),
                    ..Default::default()
                },
                false,
            )
            .await
            .context("Failed to stop Lavalink playback")?;
        Ok(())
    }

    #[cfg(not(feature = "lavalink"))]
    async fn stop_remote_playback(_ctx: &Context, _guild_id: GuildId) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn destroy(&mut self, ctx: &Context) -> anyhow::Result<()> {
        self.clear_disconnect_timeout();
        self.songs.clear();
        self.current = None;
        self.is_playing = false;
        self.auto_leave_suppressed_until = None;
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
