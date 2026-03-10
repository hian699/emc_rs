use anyhow::Context as _;
use serenity::all::{
    CommandInteraction, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    Member, Message, RoleId, UserId,
};
use serenity::client::Context;

use crate::get_state;
use crate::utils::discord_embed::{error_embed, warning_embed};
use crate::utils::guild_settings::GuildSettings;

fn member_has_any_role(member: &Member, required: &[RoleId]) -> bool {
    required
        .iter()
        .any(|role_id| member.roles.contains(role_id))
}

fn owner_ids_from_env() -> Vec<UserId> {
    let mut out = Vec::new();

    for key in ["BOT_OWNER_IDS", "BOT_OWNER_ID", "DISCORD_OWNER_ID"] {
        if let Ok(raw) = std::env::var(key) {
            out.extend(
                raw.split(',')
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .filter_map(|v| v.parse::<u64>().ok())
                    .map(UserId::new),
            );
        }
    }

    out.sort_unstable_by_key(|id| id.get());
    out.dedup_by_key(|id| id.get());
    out
}

fn is_owner_user(user_id: UserId) -> bool {
    owner_ids_from_env().iter().any(|id| *id == user_id)
}

pub async fn ensure_owner_for_slash(
    ctx: &Context,
    command: &CommandInteraction,
) -> anyhow::Result<bool> {
    let owner_ids = owner_ids_from_env();
    if owner_ids.is_empty() || owner_ids.iter().any(|id| *id == command.user.id) {
        return Ok(true);
    }

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .embed(error_embed(
                        "Owner Only",
                        "This command is restricted to bot owner.",
                    )),
            ),
        )
        .await?;

    Ok(false)
}

pub async fn ensure_owner_for_message(ctx: &Context, message: &Message) -> anyhow::Result<bool> {
    let owner_ids = owner_ids_from_env();
    if owner_ids.is_empty() || owner_ids.iter().any(|id| *id == message.author.id) {
        return Ok(true);
    }

    message
        .channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(error_embed(
                "Owner Only",
                "This command is restricted to bot owner.",
            )),
        )
        .await?;

    Ok(false)
}

pub async fn ensure_admin_for_slash(
    ctx: &Context,
    command: &CommandInteraction,
) -> anyhow::Result<bool> {
    if is_owner_user(command.user.id) {
        return Ok(true);
    }

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
                    .embed(error_embed(
                        "Permission Denied",
                        "You do not have permission to use this command.",
                    )),
            ),
        )
        .await?;
    Ok(false)
}

pub async fn ensure_developer_for_slash(
    ctx: &Context,
    command: &CommandInteraction,
) -> anyhow::Result<bool> {
    if is_owner_user(command.user.id) {
        return Ok(true);
    }

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
                    .embed(error_embed(
                        "Developer Role Required",
                        "Developer role is required for this command.",
                    )),
            ),
        )
        .await?;
    Ok(false)
}

pub async fn ensure_developer_for_message(
    ctx: &Context,
    message: &Message,
) -> anyhow::Result<bool> {
    if is_owner_user(message.author.id) {
        return Ok(true);
    }

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
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(error_embed(
                "Developer Role Required",
                "Developer role is required for this command.",
            )),
        )
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
                    .embed(warning_embed(
                        "Wrong Channel",
                        "This command is not allowed in this channel.",
                    )),
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
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(warning_embed(
                "Wrong Channel",
                "This command is not allowed in this channel.",
            )),
        )
        .await?;
    Ok(false)
}
