#[cfg(feature = "lavalink")]
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::Context as _;

#[cfg(feature = "lavalink")]
use lavalink_rs::model::BoxFuture;
#[cfg(feature = "lavalink")]
use lavalink_rs::model::events::{
    Events, Ready, TrackEnd, TrackException, TrackStart, WebSocketClosed,
};
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
use crate::utils::lavalink_runtime::trigger_lavalink_reconnect;

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

        let mut queue = queue.write().await;
        queue.mark_playing();
        queue.clear_disconnect_timeout();
    })
}

#[cfg(feature = "lavalink")]
fn handle_ready(_client: LavalinkClient, session_id: String, event: &Ready) -> BoxFuture<'_, ()> {
    let resumed = event.resumed;
    Box::pin(async move {
        tracing::info!(
            "[Lavalink] Ready session_id={} resumed={}",
            session_id,
            resumed
        );

        let Some(ctx) = lavalink_runtime_context() else {
            return;
        };
        let Ok(state) = get_state(&ctx).await else {
            return;
        };

        for queue in state.music_manager.get_all_queues().await {
            queue.write().await.lavalink_player_initialized = false;
        }
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

#[cfg(feature = "lavalink")]
fn handle_track_exception(
    _client: LavalinkClient,
    _session_id: String,
    event: &TrackException,
) -> BoxFuture<'_, ()> {
    let event = event.clone();
    Box::pin(async move {
        tracing::warn!(
            "[Lavalink] TrackException guild={} track={:?} severity={} cause={} message={}",
            event.guild_id.0,
            event.track.info.title,
            event.exception.severity,
            event.exception.cause,
            event.exception.message,
        );
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

#[cfg(feature = "lavalink")]
fn handle_websocket_closed(
    _client: LavalinkClient,
    _session_id: String,
    event: &WebSocketClosed,
) -> BoxFuture<'_, ()> {
    let event = event.clone();
    Box::pin(async move {
        tracing::warn!(
            "[Lavalink] WebSocketClosed guild={} code={} reason={:?} by_remote={}",
            event.guild_id.0,
            event.code,
            event.reason,
            event.by_remote,
        );

        let Some(ctx) = lavalink_runtime_context() else {
            return;
        };

        trigger_lavalink_reconnect(
            &ctx,
            format!(
                "websocket-closed guild={} code={} remote={}",
                event.guild_id.0, event.code, event.by_remote
            ),
        );
    })
}

#[cfg(feature = "lavalink")]
pub fn lavalink_session_ready(client: &LavalinkClient) -> bool {
    client.nodes.first().is_some_and(|node| {
        let session_id = node.session_id.load();
        let trimmed = session_id.trim();
        !trimmed.is_empty() && trimmed.parse::<usize>().is_err()
    })
}

#[cfg(feature = "lavalink")]
pub async fn wait_for_lavalink_ready(
    client: &LavalinkClient,
    timeout: Duration,
) -> anyhow::Result<String> {
    tokio::time::timeout(timeout, async {
        loop {
            if let Some(node) = client.nodes.first() {
                let session_id = node.session_id.load();
                let trimmed = session_id.trim();
                if !trimmed.is_empty() && trimmed.parse::<usize>().is_err() {
                    return Ok(trimmed.to_string());
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .context("Timed out waiting for Lavalink node Ready event")?
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
    // lookup_host requires "host:port" format; append :80 if no port is present
    let lookup_addr = if host.contains(':') {
        host.to_string()
    } else {
        format!("{host}:80")
    };
    let mut addresses = tokio::time::timeout(Duration::from_secs(3), tokio::net::lookup_host(lookup_addr.as_str()))
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
            ready: Some(handle_ready),
            track_start: Some(handle_track_start),
            track_end: Some(handle_track_end),
            track_exception: Some(handle_track_exception),
            websocket_closed: Some(handle_websocket_closed),
            ..Default::default()
        },
        session_id: None,
    };

    tokio::time::timeout(
        Duration::from_secs(10),
        LavalinkClient::new(
            Events {
                ready: Some(handle_ready),
                track_start: Some(handle_track_start),
                track_end: Some(handle_track_end),
                track_exception: Some(handle_track_exception),
                websocket_closed: Some(handle_websocket_closed),
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

#[cfg(all(test, feature = "lavalink"))]
mod tests {
    use super::extract_lavalink_host;

    #[test]
    fn extracts_host_and_ssl_from_supported_urls() {
        assert_eq!(
            extract_lavalink_host("ws://lavalink:2333").unwrap(),
            ("lavalink:2333".to_string(), Some(false))
        );
        assert_eq!(
            extract_lavalink_host("https://lava.example.com/v4/websocket").unwrap(),
            ("lava.example.com".to_string(), Some(true))
        );
        assert_eq!(
            extract_lavalink_host("lava.example.com:2333").unwrap(),
            ("lava.example.com:2333".to_string(), None)
        );
    }

    #[test]
    fn rejects_empty_lavalink_host() {
        assert!(extract_lavalink_host("   ").is_err());
    }
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
