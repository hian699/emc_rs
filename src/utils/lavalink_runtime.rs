#[cfg(feature = "lavalink")]
use std::time::{Duration, Instant};

#[cfg(feature = "lavalink")]
use serenity::all::UserId;
use serenity::client::Context;
#[cfg(feature = "lavalink")]
use tracing::{info, warn};

#[cfg(feature = "lavalink")]
use lavalink_rs::prelude::LavalinkClient;

#[cfg(feature = "lavalink")]
use crate::get_state;
#[cfg(feature = "lavalink")]
use crate::utils::lavalink_client::{
    create_client, lavalink_enabled_from_env, lavalink_session_ready,
};

#[cfg(feature = "lavalink")]
const INITIAL_RETRY_DELAY_SECS: u64 = 5;
#[cfg(feature = "lavalink")]
const MAX_RETRY_DELAY_SECS: u64 = 60;

#[cfg(feature = "lavalink")]
fn next_retry_delay(failure_count: u32) -> Duration {
    let exponent = failure_count.saturating_sub(1).min(4);
    let seconds = INITIAL_RETRY_DELAY_SECS.saturating_mul(1_u64 << exponent);
    Duration::from_secs(seconds.min(MAX_RETRY_DELAY_SECS))
}

#[cfg(feature = "lavalink")]
pub async fn init_lavalink_if_needed(
    ctx: &Context,
    bot_user_id: UserId,
    trigger: &str,
) -> anyhow::Result<Option<LavalinkClient>> {
    if !lavalink_enabled_from_env() {
        return Ok(None);
    }

    let state = get_state(ctx).await?;
    {
        let runtime = state.lavalink_runtime.read().await;
        if let Some(client) = runtime.client.clone() {
            if lavalink_session_ready(&client) {
                return Ok(Some(client));
            }
        }
    }

    let _init_guard = state.lavalink_init_lock.lock().await;
    let mut runtime = state.lavalink_runtime.write().await;

    if let Some(client) = runtime.client.clone() {
        if lavalink_session_ready(&client) {
            return Ok(Some(client));
        }

        warn!("[Lavalink] dropping stale client before re-init (trigger={trigger})");
        runtime.client = None;
    }

    if let Some(retry_after) = runtime.retry_after {
        let now = Instant::now();
        if retry_after > now {
            return Ok(None);
        }
    }

    info!(
        "[Lavalink] initializing client (trigger={trigger}, failures={})",
        runtime.consecutive_failures
    );

    match create_client(bot_user_id).await {
        Ok(client) => {
            runtime.client = Some(client.clone());
            runtime.retry_after = None;
            runtime.consecutive_failures = 0;
            runtime.last_error = None;
            info!("[Lavalink] client ready (trigger={trigger})");
            Ok(Some(client))
        }
        Err(err) => {
            runtime.consecutive_failures = runtime.consecutive_failures.saturating_add(1);
            let delay = next_retry_delay(runtime.consecutive_failures);
            runtime.retry_after = Some(Instant::now() + delay);
            runtime.last_error = Some(err.to_string());
            warn!(
                "[Lavalink] init failed (trigger={trigger}, failures={}, retry_in={}s): {err}",
                runtime.consecutive_failures,
                delay.as_secs()
            );
            Err(err)
        }
    }
}

#[cfg(feature = "lavalink")]
pub async fn get_lavalink_client(ctx: &Context) -> anyhow::Result<Option<LavalinkClient>> {
    if !lavalink_enabled_from_env() {
        return Ok(None);
    }

    let state = get_state(ctx).await?;
    {
        let runtime = state.lavalink_runtime.read().await;
        if let Some(client) = runtime.client.clone() {
            if lavalink_session_ready(&client) {
                return Ok(Some(client));
            }
        }
    }

    let bot_user_id = ctx.cache.current_user().id;
    init_lavalink_if_needed(ctx, bot_user_id, "on-demand").await
}

#[cfg(feature = "lavalink")]
pub async fn invalidate_lavalink_client(
    ctx: &Context,
    reason: impl Into<String>,
) -> anyhow::Result<()> {
    let reason = reason.into();
    let state = get_state(ctx).await?;

    {
        let mut runtime = state.lavalink_runtime.write().await;
        runtime.client = None;
        runtime.last_error = Some(reason.clone());
    }

    for queue in state.music_manager.get_all_queues().await {
        queue.write().await.lavalink_player_initialized = false;
    }

    warn!("[Lavalink] runtime invalidated: {reason}");
    Ok(())
}

#[cfg(feature = "lavalink")]
pub fn trigger_lavalink_reconnect(ctx: &Context, reason: String) {
    let ctx = ctx.clone();
    tokio::spawn(async move {
        if let Err(err) = invalidate_lavalink_client(&ctx, &reason).await {
            warn!("[Lavalink] failed to invalidate client before reconnect: {err}");
        }

        let bot_user_id = ctx.cache.current_user().id;
        if let Err(err) = init_lavalink_if_needed(&ctx, bot_user_id, &reason).await {
            warn!("[Lavalink] reconnect attempt failed: {err}");
        }
    });
}

#[cfg(not(feature = "lavalink"))]
pub async fn init_lavalink_if_needed(
    _ctx: &Context,
    _bot_user_id: serenity::all::UserId,
    _trigger: &str,
) -> anyhow::Result<Option<()>> {
    Ok(None)
}

#[cfg(not(feature = "lavalink"))]
pub async fn get_lavalink_client(_ctx: &Context) -> anyhow::Result<Option<()>> {
    Ok(None)
}

#[cfg(not(feature = "lavalink"))]
pub async fn invalidate_lavalink_client(
    _ctx: &Context,
    _reason: impl Into<String>,
) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(not(feature = "lavalink"))]
pub fn trigger_lavalink_reconnect(_ctx: &Context, _reason: String) {}

#[cfg(all(test, feature = "lavalink"))]
mod tests {
    use super::next_retry_delay;
    use std::time::Duration;

    #[test]
    fn retry_delay_grows_and_caps() {
        assert_eq!(next_retry_delay(1), Duration::from_secs(5));
        assert_eq!(next_retry_delay(2), Duration::from_secs(10));
        assert_eq!(next_retry_delay(3), Duration::from_secs(20));
        assert_eq!(next_retry_delay(4), Duration::from_secs(40));
        assert_eq!(next_retry_delay(5), Duration::from_secs(60));
        assert_eq!(next_retry_delay(8), Duration::from_secs(60));
    }
}
