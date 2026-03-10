use anyhow::Context as _;
use serenity::all::{CreateMessage, Message};
use serenity::client::Context;

use crate::get_state;
use crate::utils::discord_embed::info_embed;

pub async fn run(ctx: &Context, message: &Message) -> anyhow::Result<()> {
    let guild_id = message
        .guild_id
        .context("Message command not used in guild")?;
    let state = get_state(ctx).await?;

    if let Some(queue) = state.music_manager.get_queue(guild_id).await {
        queue.write().await.destroy(ctx).await?;
        state.music_manager.delete_queue(guild_id).await;
    }

    message
        .channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(info_embed("Stopped", "Stopped playback and cleared queue")),
        )
        .await?;
    Ok(())
}
