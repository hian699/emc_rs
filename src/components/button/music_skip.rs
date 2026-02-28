use anyhow::Context as _;
use serenity::all::{
    ComponentInteraction, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::client::Context;

use crate::get_state;

pub async fn run(ctx: &Context, interaction: &ComponentInteraction) -> anyhow::Result<()> {
    let guild_id = interaction.guild_id.context("Component not in guild")?;
    let state = get_state(ctx).await?;

    if let Some(queue) = state.music_manager.get_queue(guild_id).await {
        queue.write().await.skip().await?;
    }

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new().content("Skipped current song"),
            ),
        )
        .await?;

    Ok(())
}
