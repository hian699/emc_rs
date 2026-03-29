use std::sync::Arc;

use anyhow::Context as _;
use serenity::all::{ChannelId, CreateSelectMenuOption, GuildId, UserId};
use serenity::client::Context;
use tokio::sync::RwLock;
use tracing::warn;

use crate::components::select_menu::music_search::format_song_option_label;
use crate::get_lavalink_client;
use crate::get_state;
use crate::utils::lavalink_client::search_tracks;
use crate::utils::music_queue::{AUTO_LEAVE_SUPPRESSION_WINDOW, MusicQueue, SongItem};
use crate::utils::ytdlp_helper::{YtDlpHelper, YtDlpVideoInfo};

fn requested_song_from_lavalink(
    requested_by: &str,
    track: (String, String, u64, String),
) -> SongItem {
    let (title, url, duration_ms, encoded) = track;
    SongItem {
        title,
        url,
        duration_ms: Some(duration_ms),
        requested_by: requested_by.to_string(),
        lavalink_encoded_track: Some(encoded),
    }
}

fn requested_song_from_ytdlp(requested_by: &str, video: YtDlpVideoInfo) -> SongItem {
    SongItem {
        title: video.title,
        url: video.webpage_url,
        duration_ms: video.duration.map(|duration| (duration * 1000.0) as u64),
        requested_by: requested_by.to_string(),
        lavalink_encoded_track: None,
    }
}

#[cfg(feature = "lavalink")]
async fn require_lavalink_client(
    ctx: &Context,
) -> anyhow::Result<lavalink_rs::prelude::LavalinkClient> {
    get_lavalink_client(ctx)
        .await?
        .context("Lavalink is not ready. Check server logs and wait for reconnect.")
}

pub async fn prepare_playback(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    text_channel_id: ChannelId,
) -> anyhow::Result<(Arc<RwLock<MusicQueue>>, ChannelId)> {
    let state = get_state(ctx).await?;
    let voice_channel_id = guild_id
        .to_guild_cached(&ctx.cache)
        .and_then(|guild| {
            guild
                .voice_states
                .get(&user_id)
                .and_then(|voice_state| voice_state.channel_id)
        })
        .context("Join a voice channel first")?;

    let queue = state
        .music_manager
        .get_or_create_queue(guild_id, text_channel_id)
        .await;
    queue
        .write()
        .await
        .suppress_auto_leave(AUTO_LEAVE_SUPPRESSION_WINDOW);

    Ok((queue, voice_channel_id))
}

pub async fn resolve_direct_track(
    ctx: &Context,
    guild_id: GuildId,
    query: &str,
    requested_by: &str,
) -> anyhow::Result<SongItem> {
    #[cfg(feature = "lavalink")]
    {
        let client = require_lavalink_client(ctx).await?;
        let track = search_tracks(&client, guild_id, query)
            .await?
            .into_iter()
            .next()
            .context("No tracks found")?;
        return Ok(requested_song_from_lavalink(requested_by, track));
    }

    #[cfg(not(feature = "lavalink"))]
    {
        Ok(requested_song_from_ytdlp(
            requested_by,
            YtDlpHelper::get_video_info(query).await?,
        ))
    }
}

pub async fn resolve_search_results(
    ctx: &Context,
    guild_id: GuildId,
    query: &str,
    requested_by: &str,
) -> anyhow::Result<Vec<SongItem>> {
    #[cfg(feature = "lavalink")]
    {
        let client = require_lavalink_client(ctx).await?;
        return Ok(search_tracks(&client, guild_id, query)
            .await?
            .into_iter()
            .map(|track| requested_song_from_lavalink(requested_by, track))
            .collect());
    }

    #[cfg(not(feature = "lavalink"))]
    {
        Ok(YtDlpHelper::search(query)
            .await?
            .into_iter()
            .map(|video| requested_song_from_ytdlp(requested_by, video))
            .collect())
    }
}

pub async fn enqueue_track(
    ctx: &Context,
    guild_id: GuildId,
    queue: &Arc<RwLock<MusicQueue>>,
    song: SongItem,
) -> anyhow::Result<SongItem> {
    let should_play_now = {
        let mut queue = queue.write().await;
        queue.enqueue_song(song.clone())
    };

    if let Err(err) = MusicQueue::sync_lavalink_enqueue(ctx, guild_id, &song, should_play_now).await
    {
        warn!(
            "[Music] rolling back failed enqueue for guild {}: {}",
            guild_id.get(),
            err
        );
        let mut queue = queue.write().await;
        queue.rollback_enqueue(&song);
        return Err(err);
    }

    Ok(song)
}

pub fn build_search_options(songs: &[SongItem]) -> Vec<CreateSelectMenuOption> {
    songs
        .iter()
        .take(25)
        .map(|song| {
            CreateSelectMenuOption::new(
                format_song_option_label(&song.title, song.duration_ms),
                song.url.clone(),
            )
        })
        .collect()
}
