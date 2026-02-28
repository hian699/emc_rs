use anyhow::Context as _;
use serenity::all::{
    ComponentInteraction, ComponentInteractionDataKind, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use serenity::client::Context;

use crate::get_state;

pub fn format_duration(ms: Option<u64>) -> String {
    let Some(total_ms) = ms else {
        return "unknown".to_string();
    };

    let total_sec = total_ms / 1000;
    let min = total_sec / 60;
    let sec = total_sec % 60;
    format!("{min:02}:{sec:02}")
}

pub async fn run(ctx: &Context, interaction: &ComponentInteraction) -> anyhow::Result<()> {
    let guild_id = interaction.guild_id.context("Component not in guild")?;
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

    let mut cache = state.search_cache.write().await;
    let songs = cache.get(cache_key).unwrap_or_default();
    let picked = songs
        .iter()
        .find(|s| s.url == selected)
        .cloned()
        .context("Selected song not found in cache")?;

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
        .enqueue_song(ctx, picked.clone())
        .await?;
    cache.cleanup();

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new().content(format!(
                    "Added **{}** ({})",
                    picked.title,
                    format_duration(picked.duration_ms)
                )),
            ),
        )
        .await?;

    Ok(())
}
