use std::time::Duration;

use anyhow::Context as _;

#[cfg(feature = "lavalink")]
use lavalink_rs::prelude::{LavalinkClient, NodeBuilder, NodeDistributionStrategy, UserId};
#[cfg(feature = "lavalink")]
use lavalink_rs::{model::events::Events, model::track::TrackLoadData};
use serenity::all::GuildId;

pub fn lavalink_enabled_from_env() -> bool {
    read_non_empty_env("LAVALINK_HOST").is_ok() && read_non_empty_env("LAVALINK_PASSWORD").is_ok()
}

fn read_non_empty_env(key: &str) -> anyhow::Result<String> {
    let value = std::env::var(key).with_context(|| format!("Missing {key}"))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("{key} is set but empty")
    }
    Ok(trimmed.to_string())
}

fn extract_lavalink_host(raw: &str) -> anyhow::Result<(String, Option<bool>)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        anyhow::bail!("LAVALINK_HOST is empty")
    }

    let (rest, inferred_ssl) = if let Some(value) = trimmed.strip_prefix("ws://") {
        (value, Some(false))
    } else if let Some(value) = trimmed.strip_prefix("wss://") {
        (value, Some(true))
    } else if let Some(value) = trimmed.strip_prefix("http://") {
        (value, Some(false))
    } else if let Some(value) = trimmed.strip_prefix("https://") {
        (value, Some(true))
    } else {
        (trimmed, None)
    };

    let without_path = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .trim();

    if without_path.is_empty() {
        anyhow::bail!(
            "Invalid LAVALINK_HOST '{raw}'. Expected host:port, for example 'lavalink:2333'"
        )
    }

    Ok((without_path.to_string(), inferred_ssl))
}

#[cfg(feature = "lavalink")]
pub async fn create_client(bot_user_id: serenity::all::UserId) -> anyhow::Result<LavalinkClient> {
    let host_raw = read_non_empty_env("LAVALINK_HOST")?;
    let (host, inferred_ssl) = extract_lavalink_host(&host_raw)?;
    let password = read_non_empty_env("LAVALINK_PASSWORD")?;
    let is_ssl = std::env::var("LAVALINK_SSL")
        .ok()
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or_else(|| inferred_ssl.unwrap_or(false));

    let node = NodeBuilder {
        hostname: host,
        is_ssl,
        password,
        user_id: UserId::from(bot_user_id),
        events: Events::default(),
        session_id: None,
    };

    Ok(LavalinkClient::new(
        Events::default(),
        vec![node],
        NodeDistributionStrategy::new(),
    )
    .await)
}

#[cfg(not(feature = "lavalink"))]
pub async fn create_client(_bot_user_id: serenity::all::UserId) -> anyhow::Result<()> {
    anyhow::bail!("Lavalink feature is disabled")
}

#[cfg(feature = "lavalink")]
pub async fn search_tracks(
    client: &LavalinkClient,
    guild_id: GuildId,
    query: &str,
) -> anyhow::Result<Vec<(String, String, u64, String)>> {
    let identifier = if query.starts_with("http://") || query.starts_with("https://") {
        query.to_string()
    } else {
        format!("ytsearch:{query}")
    };

    let loaded = client
        .load_tracks(guild_id, &identifier)
        .await
        .context("Failed to load tracks from lavalink")?;

    let mut out = Vec::new();
    if let Some(data) = loaded.data {
        match data {
            TrackLoadData::Track(track) => {
                let title = track.info.title.clone();
                let uri = track.info.uri.clone().unwrap_or_default();
                let length = track.info.length;
                out.push((title, uri, length, track.encoded));
            }
            TrackLoadData::Search(tracks) => {
                for track in tracks.into_iter().take(10) {
                    let title = track.info.title.clone();
                    let uri = track.info.uri.clone().unwrap_or_default();
                    let length = track.info.length;
                    out.push((title, uri, length, track.encoded));
                }
            }
            TrackLoadData::Playlist(playlist) => {
                for track in playlist.tracks.into_iter().take(10) {
                    let title = track.info.title.clone();
                    let uri = track.info.uri.clone().unwrap_or_default();
                    let length = track.info.length;
                    out.push((title, uri, length, track.encoded));
                }
            }
            TrackLoadData::Error(err) => {
                anyhow::bail!("Lavalink track load error: {}", err.message)
            }
        }
    }

    Ok(out)
}

#[cfg(feature = "lavalink")]
pub async fn try_create_player_context(
    client: &LavalinkClient,
    guild_id: GuildId,
) -> anyhow::Result<lavalink_rs::prelude::PlayerContext> {
    let info = client
        .get_connection_info(guild_id, Duration::from_secs(8))
        .await
        .context("Missing voice connection info from Discord events")?;
    let player = client
        .create_player_context(guild_id, info)
        .await
        .context("Failed to create lavalink player context")?;
    Ok(player)
}
