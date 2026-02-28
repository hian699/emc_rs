use serenity::all::{
    CommandInteraction, CommandOptionType, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::client::Context;

use crate::commands::get_string_option;
use crate::utils::access_control::ensure_developer_for_slash;

pub fn register() -> CreateCommand {
    CreateCommand::new("eval")
        .description("Developer echo eval")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "code", "Code snippet")
                .required(true),
        )
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    if !ensure_developer_for_slash(ctx, command).await? {
        return Ok(());
    }

    let code = get_string_option(command, "code").unwrap_or_default();
    let output = format!("```txt\n{code}\n```");

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content(output),
            ),
        )
        .await?;

    Ok(())
}
