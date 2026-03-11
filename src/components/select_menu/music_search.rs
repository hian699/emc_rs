use anyhow::Context as _;
use serenity::all::{
    ComponentInteraction, ComponentInteractionDataKind, CreateInteractionResponse,
    EditInteractionResponse,
};
use serenity::client::Context;

use crate::get_state;
use crate::utils::discord_embed::{success_embed, warning_embed};
use crate::utils::music_queue::{MusicQueue, AUTO_LEAVE_SUPPRESSION_WINDOW};

pub fn format_duration(ms: Option<u64>) -> String {
    let Some(total_ms) = ms else {
        return "unknown".to_string();
    };

    let total_sec = total_ms / 1000;
    let min = total_sec / 60;
    let sec = total_sec % 60;
    format!("{min:02}:{sec:02}")
}

pub fn format_song_option_label(title: &str, duration_ms: Option<u64>) -> String {
    const MAX_LABEL_CHARS: usize = 100;
    let suffix = format!(" ({})", format_duration(duration_ms));
    let suffix_len = suffix.chars().count();
    let max_title_len = MAX_LABEL_CHARS.saturating_sub(suffix_len);

    let safe_title = if title.chars().count() <= max_title_len {
        title.to_string()
    } else if max_title_len <= 3 {
        "...".chars().take(max_title_len).collect()
    } else {
        let mut truncated: String = title.chars().take(max_title_len - 3).collect();
        truncated.push_str("...");
        truncated
    };

    format!("{safe_title}{suffix}")
}

pub async fn run(ctx: &Context, interaction: &ComponentInteraction) -> anyhow::Result<()> {
    let guild_id = interaction.guild_id.context("Component not in guild")?;

    interaction
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await?;

    let state = get_state(ctx).await?;
    let cache_key = interaction
        .data
        .custom_id
        .strip_prefix("music-search:")
        .context("Invalid music search component id")?;

    let selected = match &interaction.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => values.first().cloned(),
        _ => None,
    }
    .context("No selected value")?;

    let picked = {
        let cache = state.search_cache.read().await;
        let songs = cache.get(cache_key).unwrap_or_default();
        songs
            .iter()
            .find(|s| s.url == selected)
            .cloned()
            .context("Selected song not found in cache")?
    };

    let queue = if let Some(q) = state.music_manager.get_queue(guild_id).await {
        q
    } else {
        state
            .music_manager
            .create_queue(guild_id, interaction.channel_id)
            .await
    };

    queue
        .write()
        .await
        .suppress_auto_leave(AUTO_LEAVE_SUPPRESSION_WINDOW);

    // Ensure the bot is connected to the user's current voice channel.
    // This is needed because the user may have taken time to pick from the dropdown
    // and the bot may have disconnected, or no Lavalink player exists yet.
    let voice_channel_id = guild_id
        .to_guild_cached(&ctx.cache)
        .and_then(|guild| {
            guild
                .voice_states
                .get(&interaction.user.id)
                .and_then(|vs| vs.channel_id)
        });

    if let Some(channel_id) = voice_channel_id {
        if let Err(err) = MusicQueue::connect(ctx, channel_id).await {
            interaction
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new()
                        .embed(warning_embed(
                            "Voice Connect Failed",
                            format!("Cannot connect to your voice channel.\nDetails: {err}"),
                        ))
                        .components(vec![]),
                )
                .await?;
            return Ok(());
        }
    }

    let should_play_now = {
        let mut q = queue.write().await;
        q.enqueue_song(picked.clone())
    };
    if let Err(err) = MusicQueue::sync_lavalink_enqueue(ctx, guild_id, &picked, should_play_now).await
    {
        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().embed(warning_embed(
                    "Playback Failed",
                    format!("Failed to start Lavalink playback.\nDetails: {err}"),
                )),
            )
            .await?;
        return Ok(());
    }

    state.search_cache.write().await.cleanup();

    interaction
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new()
                .embed(success_embed(
                    "Song Added",
                    format!(
                        "**{}** ({})",
                        picked.title,
                        format_duration(picked.duration_ms)
                    ),
                ))
                .components(vec![]),
        )
        .await?;

    Ok(())
}
