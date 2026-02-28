use serenity::all::{
    CommandInteraction, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::client::Context;

use crate::utils::access_control::ensure_developer_for_slash;

pub fn register() -> CreateCommand {
    CreateCommand::new("reload").description("Reload command cache")
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    if !ensure_developer_for_slash(ctx, command).await? {
        return Ok(());
    }

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content("Reload finished"),
            ),
        )
        .await?;
    Ok(())
}
