use anyhow::Context as _;
use serenity::all::{
    CommandInteraction, CreateInteractionResponse, CreateInteractionResponseMessage, Member,
    Message, RoleId,
};
use serenity::client::Context;

use crate::get_state;
use crate::utils::guild_settings::GuildSettings;

fn member_has_any_role(member: &Member, required: &[RoleId]) -> bool {
    required
        .iter()
        .any(|role_id| member.roles.contains(role_id))
}

pub async fn ensure_admin_for_slash(
    ctx: &Context,
    command: &CommandInteraction,
) -> anyhow::Result<bool> {
    let guild_id = match command.guild_id {
        Some(g) => g,
        None => return Ok(false),
    };
    let state = get_state(ctx).await?;
    let settings = state.settings_repo.get_settings(guild_id).await?;
    let required: Vec<_> = settings.admin_role_ids.iter().copied().collect();
    if required.is_empty() {
        return Ok(true);
    }

    let Some(member) = command.member.as_ref() else {
        return Ok(false);
    };

    if member_has_any_role(member, &required) {
        return Ok(true);
    }

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content("You do not have permission to use this command."),
            ),
        )
        .await?;
    Ok(false)
}

pub async fn ensure_developer_for_slash(
    ctx: &Context,
    command: &CommandInteraction,
) -> anyhow::Result<bool> {
    let guild_id = match command.guild_id {
        Some(g) => g,
        None => return Ok(false),
    };
    let state = get_state(ctx).await?;
    let settings = state.settings_repo.get_settings(guild_id).await?;
    let required: Vec<_> = settings.developer_role_ids.iter().copied().collect();
    if required.is_empty() {
        return Ok(true);
    }

    let Some(member) = command.member.as_ref() else {
        return Ok(false);
    };

    if member_has_any_role(member, &required) {
        return Ok(true);
    }

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content("Developer role is required for this command."),
            ),
        )
        .await?;
    Ok(false)
}

pub async fn ensure_developer_for_message(
    ctx: &Context,
    message: &Message,
) -> anyhow::Result<bool> {
    let guild_id = match message.guild_id {
        Some(g) => g,
        None => return Ok(false),
    };
    let state = get_state(ctx).await?;
    let settings = state.settings_repo.get_settings(guild_id).await?;
    let required: Vec<_> = settings.developer_role_ids.iter().copied().collect();
    if required.is_empty() {
        return Ok(true);
    }

    let member = message
        .member(ctx)
        .await
        .context("Message author is not in guild context")?;
    if member_has_any_role(&member, &required) {
        return Ok(true);
    }

    message
        .channel_id
        .say(&ctx.http, "Developer role is required for this command.")
        .await?;
    Ok(false)
}

pub async fn ensure_music_channel_for_slash(
    ctx: &Context,
    command: &CommandInteraction,
) -> anyhow::Result<bool> {
    let guild_id = match command.guild_id {
        Some(g) => g,
        None => return Ok(false),
    };
    let state = get_state(ctx).await?;
    let settings = state.settings_repo.get_settings(guild_id).await?;
    if settings.allow_music_channel(command.channel_id) {
        return Ok(true);
    }

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content("This command is not allowed in this channel."),
            ),
        )
        .await?;
    Ok(false)
}

pub async fn ensure_music_channel_for_message(
    ctx: &Context,
    message: &Message,
) -> anyhow::Result<bool> {
    let guild_id = match message.guild_id {
        Some(g) => g,
        None => return Ok(false),
    };
    let state = get_state(ctx).await?;
    let settings: GuildSettings = state.settings_repo.get_settings(guild_id).await?;
    if settings.allow_music_channel(message.channel_id) {
        return Ok(true);
    }

    message
        .channel_id
        .say(&ctx.http, "This command is not allowed in this channel.")
        .await?;
    Ok(false)
}
