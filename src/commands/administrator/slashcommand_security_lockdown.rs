use anyhow::Context as _;
use serenity::all::{
    ChannelId, CommandInteraction, CommandOptionType, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage, EditChannel, Permissions,
};
use serenity::client::Context;

use crate::commands::get_bool_option;
use crate::utils::access_control::ensure_admin_for_slash;

pub fn register() -> CreateCommand {
    CreateCommand::new("security-lockdown")
        .description("Lock or unlock a text channel")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "enabled",
                "true=lock, false=unlock",
            )
            .required(true),
        )
}

pub async fn set_lockdown(
    ctx: &Context,
    channel_id: ChannelId,
    enabled: bool,
) -> anyhow::Result<()> {
    let channel = channel_id.to_channel(&ctx.http).await?;
    let guild_channel = channel.guild().context("Not a guild channel")?;
    let mut perms = guild_channel.permission_overwrites.clone();

    if let Some(idx) = perms.iter().position(|ow| matches!(ow.kind, serenity::all::PermissionOverwriteType::Role(role_id) if role_id.get() == guild_channel.guild_id.get())) {
        if enabled {
            perms[idx].deny.insert(Permissions::SEND_MESSAGES);
            perms[idx].allow.remove(Permissions::SEND_MESSAGES);
        } else {
            perms[idx].deny.remove(Permissions::SEND_MESSAGES);
        }
    }

    guild_channel
        .id
        .edit(&ctx.http, EditChannel::new().permissions(perms))
        .await?;

    Ok(())
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    if !ensure_admin_for_slash(ctx, command).await? {
        return Ok(());
    }

    let enabled = get_bool_option(command, "enabled").unwrap_or(true);
    set_lockdown(ctx, command.channel_id, enabled).await?;

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(
                if enabled {
                    "Channel locked"
                } else {
                    "Channel unlocked"
                },
            )),
        )
        .await?;
    Ok(())
}
