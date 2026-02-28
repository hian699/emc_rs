use std::collections::VecDeque;
use std::time::Duration;

use anyhow::Context as _;
#[cfg(feature = "lavalink")]
use lavalink_rs::model::track::{TrackData, TrackInfo};
use serenity::all::EditVoiceState;
use serenity::all::{ChannelId, GuildId};
use serenity::client::Context;
use tokio::task::JoinHandle;

use crate::get_lavalink_client;
#[cfg(feature = "lavalink")]
use crate::utils::lavalink_client::try_create_player_context;

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

    pub async fn connect(&mut self, _ctx: &Context, _channel_id: ChannelId) -> anyhow::Result<()> {
        #[cfg(feature = "lavalink")]
        {
            let channel = _channel_id
                .to_channel(&_ctx.http)
                .await
                .context("Failed to resolve channel")?;
            let guild_channel = channel.guild().context("Target is not a guild channel")?;
            guild_channel
                .edit_own_voice_state(&_ctx.http, EditVoiceState::new())
                .await
                .context("Failed to update bot voice state")?;
        }

        Ok(())
    }

    pub async fn enqueue_song(&mut self, _ctx: &Context, song: SongItem) -> anyhow::Result<()> {
        let song_for_track = song.clone();
        let is_first = self.current.is_none();
        if is_first {
            self.current = Some(song.clone());
        }
        self.songs.push_back(song);

        #[cfg(feature = "lavalink")]
        {
            if let Some(client) = get_lavalink_client(_ctx).await? {
                let encoded = song_for_track.lavalink_encoded_track.clone();
                if let Some(encoded) = encoded {
                    let player = if let Some(existing) = client.get_player_context(self.guild_id) {
                        existing
                    } else {
                        try_create_player_context(&client, self.guild_id).await?
                    };

                    let track = track_from_song(song_for_track, encoded);
                    player.queue(track.clone())?;
                    if is_first {
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
        self.play(ctx).await
    }

    pub async fn skip(&mut self) -> anyhow::Result<()> {
        self.songs.pop_front();
        self.current = self.songs.front().cloned();
        Ok(())
    }

    pub async fn stop(&mut self) -> anyhow::Result<()> {
        self.songs.clear();
        self.current = None;
        Ok(())
    }

    pub async fn destroy(&mut self, _ctx: &Context) -> anyhow::Result<()> {
        self.clear_disconnect_timeout();
        self.stop().await
    }

    pub async fn send_error(&self, ctx: &Context, message: &str) -> anyhow::Result<()> {
        self.text_channel_id
            .say(&ctx.http, format!("Music error: {message}"))
            .await
            .context("Failed to send error message")?;
        Ok(())
    }

    pub fn start_disconnect_timeout(&mut self, timeout: Duration) {
        self.clear_disconnect_timeout();
        self.disconnect_timeout = Some(tokio::spawn(async move {
            tokio::time::sleep(timeout).await;
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
