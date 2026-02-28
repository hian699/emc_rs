use anyhow::Context as _;
use serenity::all::{
    CommandInteraction, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::client::Context;

use crate::get_state;
use crate::utils::access_control::ensure_admin_for_slash;

pub fn register() -> CreateCommand {
    CreateCommand::new("config-show").description("Show current sqlite guild config")
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    if !ensure_admin_for_slash(ctx, command).await? {
        return Ok(());
    }

    let guild_id = command.guild_id.context("Command not used in guild")?;
    let state = get_state(ctx).await?;
    let settings = state.settings_repo.get_settings(guild_id).await?;

    let mut admin_roles: Vec<String> = settings
        .admin_role_ids
        .iter()
        .map(|id| id.get().to_string())
        .collect();
    admin_roles.sort();
    let mut developer_roles: Vec<String> = settings
        .developer_role_ids
        .iter()
        .map(|id| id.get().to_string())
        .collect();
    developer_roles.sort();
    let mut music_channels: Vec<String> = settings
        .music_text_channel_ids
        .iter()
        .map(|id| id.get().to_string())
        .collect();
    music_channels.sort();
    let mut private_voice_channels: Vec<String> = settings
        .private_voice_allowed_channel_ids
        .iter()
        .map(|id| id.get().to_string())
        .collect();
    private_voice_channels.sort();

    let content = format!(
        "Guild config\nadmin_role_ids: {}\ndeveloper_role_ids: {}\nmusic_text_channel_ids: {}\nprivate_voice_allowed_channel_ids: {}\ntemp_voice_category_id: {}\ntemp_voice_lobby_channel_id: {}\nmod_channel_id: {}",
        if admin_roles.is_empty() {
            "<empty>".to_string()
        } else {
            admin_roles.join(",")
        },
        if developer_roles.is_empty() {
            "<empty>".to_string()
        } else {
            developer_roles.join(",")
        },
        if music_channels.is_empty() {
            "<empty>".to_string()
        } else {
            music_channels.join(",")
        },
        if private_voice_channels.is_empty() {
            "<empty>".to_string()
        } else {
            private_voice_channels.join(",")
        },
        settings
            .temp_voice_category_id
            .map(|id| id.get().to_string())
            .unwrap_or_else(|| "<empty>".to_string()),
        settings
            .temp_voice_lobby_channel_id
            .map(|id| id.get().to_string())
            .unwrap_or_else(|| "<empty>".to_string()),
        settings
            .mod_channel_id
            .map(|id| id.get().to_string())
            .unwrap_or_else(|| "<empty>".to_string())
    );

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content(format!("```txt\n{content}\n```")),
            ),
        )
        .await?;

    Ok(())
}
