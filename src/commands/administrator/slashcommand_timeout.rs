use std::time::Duration;

use anyhow::Context as _;
use serenity::all::{
    CommandInteraction, CommandOptionType, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage, EditMember,
};
use serenity::client::Context;

use crate::commands::{get_i64_option, get_user_id_option};
use crate::utils::access_control::ensure_admin_for_slash;

pub fn register() -> CreateCommand {
    CreateCommand::new("timeout")
        .description("Timeout a guild member")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Target user").required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "minutes",
                "Timeout duration in minutes",
            )
            .required(true),
        )
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    if !ensure_admin_for_slash(ctx, command).await? {
        return Ok(());
    }

    let guild_id = command.guild_id.context("Command not used in guild")?;
    let user_id = get_user_id_option(command, "user").context("Missing user option")?;
    let minutes = get_i64_option(command, "minutes").unwrap_or(1).max(1);
    let until = serenity::model::Timestamp::from_unix_timestamp(
        (chrono::Utc::now()
            + chrono::Duration::from_std(Duration::from_secs((minutes as u64) * 60))?)
        .timestamp(),
    )?;

    guild_id
        .edit_member(
            &ctx.http,
            user_id,
            EditMember::new().disable_communication_until(until.to_string()),
        )
        .await?;

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(
                format!("Timed out <@{}> for {} minutes", user_id.get(), minutes),
            )),
        )
        .await?;

    Ok(())
}
