use anyhow::Context as _;
use serenity::all::{
    ComponentInteraction, ComponentInteractionDataKind, CreateInteractionResponse,
    CreateInteractionResponseMessage, PermissionOverwrite, PermissionOverwriteType, Permissions,
};
use serenity::client::Context;

use crate::get_state;

pub async fn run(ctx: &Context, interaction: &ComponentInteraction) -> anyhow::Result<()> {
    let guild_id = interaction.guild_id.context("Component not in guild")?;
    let state = get_state(ctx).await?;

    let user_id = match &interaction.data.kind {
        ComponentInteractionDataKind::UserSelect { values } => values.first().copied(),
        _ => None,
    }
    .context("No selected user")?;

    let voice_channel_id = guild_id
        .to_guild_cached(&ctx.cache)
        .and_then(|g| {
            g.voice_states
                .get(&interaction.user.id)
                .and_then(|v| v.channel_id)
        })
        .context("You are not in a voice channel")?;

    let owner = state
        .private_voice_registry
        .read()
        .await
        .get_owner(voice_channel_id)
        .context("This voice channel is not a private temp voice")?;

    let settings = state.settings_repo.get_settings(guild_id).await?;
    if !settings.allow_private_voice_channel(voice_channel_id) {
        return Err(anyhow::anyhow!(
            "This private temp voice channel is not allowed by configuration"
        ));
    }

    if owner != interaction.user.id {
        return Err(anyhow::anyhow!("Only owner can invite users"));
    }

    voice_channel_id
        .create_permission(
            &ctx.http,
            PermissionOverwrite {
                allow: Permissions::CONNECT | Permissions::VIEW_CHANNEL,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(user_id),
            },
        )
        .await?;

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new().content(format!(
                    "Invited <@{}> to your private temp voice",
                    user_id.get()
                )),
            ),
        )
        .await?;

    Ok(())
}
