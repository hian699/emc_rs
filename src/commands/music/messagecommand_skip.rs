use anyhow::Context as _;
use serenity::all::Message;
use serenity::client::Context;

use crate::get_state;

pub async fn run(ctx: &Context, message: &Message) -> anyhow::Result<()> {
    let guild_id = message
        .guild_id
        .context("Message command not used in guild")?;
    let state = get_state(ctx).await?;

    if let Some(queue) = state.music_manager.get_queue(guild_id).await {
        queue.write().await.skip().await?;
    }

    message
        .channel_id
        .say(&ctx.http, "Skipped current song")
        .await?;
    Ok(())
}
