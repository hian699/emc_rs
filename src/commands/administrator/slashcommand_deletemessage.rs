use anyhow::Context as _;
use serenity::all::{
    CommandInteraction, CommandOptionType, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::client::Context;

use crate::commands::get_i64_option;
use crate::utils::access_control::ensure_admin_for_slash;

pub fn register() -> CreateCommand {
    CreateCommand::new("deletemessage")
        .description("Delete recent messages in current channel")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "count", "Number of messages")
                .required(true),
        )
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    if !ensure_admin_for_slash(ctx, command).await? {
        return Ok(());
    }

    let count = get_i64_option(command, "count").unwrap_or(1).clamp(1, 100) as u8;
    let channel = command.channel_id;
    let messages = channel
        .messages(&ctx.http, serenity::all::GetMessages::new().limit(count))
        .await
        .context("Failed to fetch messages")?;

    let ids: Vec<_> = messages.iter().map(|m| m.id).collect();
    channel
        .delete_messages(&ctx.http, ids)
        .await
        .context("Failed to delete messages")?;

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content(format!("Deleted {} messages", messages.len())),
            ),
        )
        .await?;

    Ok(())
}
