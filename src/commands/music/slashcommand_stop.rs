use anyhow::Context as _;
use serenity::all::{
    CommandInteraction, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::client::Context;

use crate::get_state;

pub fn register() -> CreateCommand {
    CreateCommand::new("stop").description("Stop playback and clear queue")
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let guild_id = command.guild_id.context("Command not used in guild")?;
    let state = get_state(ctx).await?;

    if let Some(queue) = state.music_manager.get_queue(guild_id).await {
        queue.write().await.destroy(ctx).await?;
        state.music_manager.delete_queue(guild_id).await;
    }

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content("Stopped playback"),
            ),
        )
        .await?;
    Ok(())
}
