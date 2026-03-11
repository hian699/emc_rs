use anyhow::Context as _;
use serenity::all::{
    ComponentInteraction, CreateInteractionResponse, EditInteractionResponse,
};
use serenity::client::Context;

use crate::get_state;
use crate::utils::discord_embed::info_embed;

pub async fn run(ctx: &Context, interaction: &ComponentInteraction) -> anyhow::Result<()> {
    let guild_id = interaction.guild_id.context("Component not in guild")?;

    interaction
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await?;

    let state = get_state(ctx).await?;

    if let Some(queue) = state.music_manager.get_queue(guild_id).await {
        queue.write().await.skip(ctx).await?;
    }

    interaction
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().embed(info_embed("Skipped", "Skipped current song")),
        )
        .await?;

    Ok(())
}
