#[cfg(feature = "lavalink")]
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::Context as _;

#[cfg(feature = "lavalink")]
use lavalink_rs::model::BoxFuture;
#[cfg(feature = "lavalink")]
use lavalink_rs::model::events::{Events, TrackEnd, TrackStart};
#[cfg(feature = "lavalink")]
use lavalink_rs::model::track::TrackLoadData;
#[cfg(feature = "lavalink")]
use lavalink_rs::prelude::{LavalinkClient, NodeBuilder, NodeDistributionStrategy, UserId};
use serenity::all::GuildId;
#[cfg(feature = "lavalink")]
use serenity::client::Context as SerenityContext;

#[cfg(feature = "lavalink")]
use crate::get_state;

#[cfg(feature = "lavalink")]
static LAVALINK_RUNTIME_CONTEXT: OnceLock<SerenityContext> = OnceLock::new();

#[cfg(feature = "lavalink")]
pub fn set_lavalink_runtime_context(ctx: &SerenityContext) {
    let _ = LAVALINK_RUNTIME_CONTEXT.set(ctx.clone());
}

#[cfg(feature = "lavalink")]
fn lavalink_runtime_context() -> Option<SerenityContext> {
    LAVALINK_RUNTIME_CONTEXT.get().cloned()
}

#[cfg(feature = "lavalink")]
fn handle_track_start(
    _client: LavalinkClient,
    _session_id: String,
    event: &TrackStart,
) -> BoxFuture<'_, ()> {
    let event = event.clone();
    Box::pin(async move {
        let Some(ctx) = lavalink_runtime_context() else {
            return;
        };
        let Ok(state) = get_state(&ctx).await else {
            return;
        };
        let Some(queue) = state
            .music_manager
            .get_queue(GuildId::new(event.guild_id.0))
            .await
        else {
            return;
        };

        queue.write().await.clear_disconnect_timeout();
    })
}

#[cfg(feature = "lavalink")]
fn handle_track_end(
    _client: LavalinkClient,
    _session_id: String,
    event: &TrackEnd,
) -> BoxFuture<'_, ()> {
    let event = event.clone();
    Box::pin(async move {
        if !bool::from(event.reason.clone()) {
            return;
        }

        let Some(ctx) = lavalink_runtime_context() else {
            return;
        };
        let Ok(state) = get_state(&ctx).await else {
            return;
        };
        let Some(queue) = state
            .music_manager
            .get_queue(GuildId::new(event.guild_id.0))
            .await
        else {
            return;
        };

        let _ = queue.write().await.handle_song_end(&ctx).await;
    })
}

pub fn lavalink_enabled_from_env() -> bool {
    read_non_empty_env("LAVALINK_HOST").is_ok() && read_lavalink_password_from_env().is_ok()
}

fn read_non_empty_env(key: &str) -> anyhow::Result<String> {
    let value = std::env::var(key).with_context(|| format!("Missing {key}"))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("{key} is set but empty")
    }
    Ok(trimmed.to_string())
}

fn read_first_non_empty_env(keys: &[&str]) -> anyhow::Result<String> {
    for key in keys {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }
    }

    anyhow::bail!("Missing non-empty env in keys: {}", keys.join(", "))
}

fn read_lavalink_password_from_env() -> anyhow::Result<String> {
    read_first_non_empty_env(&["LAVALINK_PASSWORD", "LAVALINK_SERVER_PASSWORD"])
}

#[cfg(feature = "lavalink")]
async fn validate_lavalink_host(host: &str) -> anyhow::Result<()> {
    let mut addresses = tokio::time::timeout(Duration::from_secs(3), tokio::net::lookup_host(host))
        .await
        .with_context(|| format!("Timed out while resolving Lavalink host '{host}'"))?
        .with_context(|| {
            format!(
                "Failed to resolve Lavalink host '{host}'. If bot and Lavalink are not on the same Docker network, set LAVALINK_HOST to a reachable internal domain, public domain, or IP instead of a Docker-only service name"
            )
        })?;

    let _ = addresses.next();

    Ok(())
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
    let password = read_lavalink_password_from_env()?;
    validate_lavalink_host(&host).await?;
    let is_ssl = std::env::var("LAVALINK_SSL")
        .ok()
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or_else(|| inferred_ssl.unwrap_or(false));
    let host_for_error = host.clone();

    let node = NodeBuilder {
        hostname: host,
        is_ssl,
        password,
        user_id: UserId::from(bot_user_id),
        events: Events {
            track_start: Some(handle_track_start),
            track_end: Some(handle_track_end),
            ..Default::default()
        },
        session_id: None,
    };

    tokio::time::timeout(
        Duration::from_secs(10),
        LavalinkClient::new(
            Events {
                track_start: Some(handle_track_start),
                track_end: Some(handle_track_end),
                ..Default::default()
            },
            vec![node],
            NodeDistributionStrategy::new(),
        ),
    )
    .await
    .context("Timed out while connecting to Lavalink")
    .with_context(|| {
        format!(
            "Failed to connect to Lavalink at {host_for_error}. Check LAVALINK_HOST, password, SSL setting, and Docker/Dokploy network reachability"
        )
    })
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

    let loaded = match tokio::time::timeout(
        Duration::from_secs(12),
        client.load_tracks(guild_id, &identifier),
    )
    .await
    {
        Ok(Ok(value)) => value,
        _ => {
            tokio::time::sleep(Duration::from_millis(600)).await;
            tokio::time::timeout(
                Duration::from_secs(12),
                client.load_tracks(guild_id, &identifier),
            )
            .await
            .context("Timed out while loading tracks from Lavalink")?
            .with_context(|| {
                format!("Failed to load tracks from lavalink for identifier: {identifier}")
            })?
        }
    };

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
