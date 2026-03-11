use std::collections::VecDeque;
use std::time::{Duration, Instant};

use anyhow::Context as _;
use tracing::debug;
use serenity::all::{ChannelId, GuildId};
use serenity::client::Context;
use tokio::task::JoinHandle;

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
    /// Set to true after a Lavalink voice player is successfully created.
    /// Used to skip redundant voice handshakes on subsequent connect() calls.
    pub lavalink_player_initialized: bool,
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
            lavalink_player_initialized: false,
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
            let guild_id = guild_channel.guild_id;
            let state = get_state(_ctx).await?;

            if let Some(queue) = state.music_manager.get_queue(guild_id).await {
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

            // If already in the correct channel with an active Lavalink player, skip re-handshake.
            let bot_user_id = _ctx.cache.current_user().id;
            let already_in_channel = guild_id
                .to_guild_cached(&_ctx.cache)
                .and_then(|g| g.voice_states.get(&bot_user_id).and_then(|vs| vs.channel_id))
                == Some(_channel_id);

            if already_in_channel {
                let player_ready = if let Some(q) = state.music_manager.get_queue(guild_id).await {
                    q.read().await.lavalink_player_initialized
                } else {
                    false
                };
                if player_ready {
                    debug!(
                        "[Lavalink] player already initialized for guild {:?}, skipping",
                        guild_id
                    );
                    return Ok(());
                }
            }

            // songbird.join() sends OP4 internally and waits for BOTH VOICE_STATE_UPDATE
            // and VOICE_SERVER_UPDATE to complete before returning — guaranteeing a fresh,
            // non-stale token/endpoint with no race condition.
            let manager = songbird::get(_ctx)
                .await
                .ok_or_else(|| anyhow::anyhow!("Songbird is not registered in the serenity client"))?;

            let (call, join_result) = manager.join(guild_id, _channel_id).await;
            join_result.context("Songbird failed to join voice channel")?;

            // Fresh voice connection info — guaranteed valid after successful join().
            let conn_info = {
                let call_lock = call.lock().await;
                call_lock.current_connection().cloned()
            };
            let conn_info =
                conn_info.ok_or_else(|| anyhow::anyhow!("Songbird has no connection info after join"))?;

            debug!(
                "[Lavalink] songbird joined: guild={:?} endpoint={:?} session={:?} token_len={}",
                guild_id, conn_info.endpoint, conn_info.session_id, conn_info.token.len()
            );

            let lavalink_client = get_lavalink_client(_ctx).await?;
            let Some(client) = lavalink_client else {
                return Ok(());
            };

            // Wait for Lavalink WS Ready event so session_id is populated.
            if let Some(node) = client.nodes.first() {
                tokio::time::timeout(Duration::from_secs(8), async {
                    loop {
                        let sid = node.session_id.load();
                        if sid.parse::<usize>().is_err() {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                })
                .await
                .with_context(|| "Timed out waiting for Lavalink node Ready event")?;
            }

            // PATCH Lavalink player with fresh voice data from songbird.
            let node = client
                .nodes
                .first()
                .ok_or_else(|| anyhow::anyhow!("No Lavalink nodes available"))?;
            let session_id_str = node.session_id.load().to_string();
            // Lavalink expects endpoint without protocol prefix.
            let endpoint = conn_info.endpoint.replace("wss://", "");
            let patch_body = serde_json::json!({
                "voice": {
                    "token": &conn_info.token,
                    "endpoint": &endpoint,
                    "sessionId": &conn_info.session_id,
                }
            });
            debug!(
                "[Lavalink] PATCH /sessions/{}/players/{} body={}",
                session_id_str,
                guild_id.get(),
                serde_json::to_string(&patch_body).unwrap_or_default()
            );
            let uri = node
                .http
                .path_to_uri(
                    &format!("/sessions/{}/players/{}", session_id_str, guild_id.get()),
                    true,
                )
                .map_err(|e| anyhow::anyhow!("Failed to build Lavalink URI: {e}"))?;
            let response = node
                .http
                .raw_request(::http::Method::PATCH, uri, Some(&patch_body))
                .await
                .map_err(|e| anyhow::anyhow!("Lavalink PATCH failed: {e}"))?;
            debug!("[Lavalink] PATCH response: guild={:?} body={}", guild_id, &response);
            if response.contains("\"error\"") {
                anyhow::bail!("Lavalink rejected voice player creation: {}", response);
            }

            if let Some(q) = state.music_manager.get_queue(guild_id).await {
                q.write().await.lavalink_player_initialized = true;
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

        // songbird.remove() sends OP4 channel_id=null and clears its internal state.
        #[cfg(feature = "lavalink")]
        if let Some(manager) = songbird::get(ctx).await {
            let _ = manager.remove(guild_id).await;
        }

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


        let node = client.nodes.first()
            .ok_or_else(|| anyhow::anyhow!("No Lavalink nodes available"))?;
        let session_id_str = node.session_id.load().to_string();
        let uri = node.http.path_to_uri(
            &format!(
                "/sessions/{}/players/{}?noReplace=false",
                session_id_str,
                guild_id.get()
            ),
            true,
        )
        .map_err(|e| anyhow::anyhow!("Failed to build Lavalink player URI: {e}"))?;
        let encoded_clone = encoded.clone();
        let patch_body = serde_json::json!({
            "track": { "encoded": encoded_clone },
            "paused": false
        });
        let response = node.http
            .raw_request(::http::Method::PATCH, uri, Some(&patch_body))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to update Lavalink player for current song: {e}"))?;
        debug!("[Lavalink] play_song_now response for guild {:?}: {}", guild_id, &response);
        if response.contains("\"error\"") {
            anyhow::bail!("Lavalink rejected track playback: {}", response);
        }
        Ok(())
    }


    #[cfg(feature = "lavalink")]
    async fn stop_remote_playback(ctx: &Context, guild_id: GuildId) -> anyhow::Result<()> {
        let Some(client) = get_lavalink_client(ctx).await? else {
            return Ok(());
        };

        let node = client.nodes.first()
            .ok_or_else(|| anyhow::anyhow!("No Lavalink nodes available"))?;
        let session_id_str = node.session_id.load().to_string();
        let uri = node.http.path_to_uri(
            &format!(
                "/sessions/{}/players/{}?noReplace=false",
                session_id_str,
                guild_id.get()
            ),
            true,
        )
        .map_err(|e| anyhow::anyhow!("Failed to build Lavalink stop URI: {e}"))?;
        // null encoded track = stop playback
        let patch_body = serde_json::json!({ "track": { "encoded": null } });
        node.http
            .raw_request(::http::Method::PATCH, uri, Some(&patch_body))
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
        self.lavalink_player_initialized = false;
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

