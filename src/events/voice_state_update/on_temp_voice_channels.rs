use anyhow::Context as _;
use serenity::all::{ChannelType, CreateChannel, VoiceState};
use serenity::client::Context;

use crate::get_state;

pub async fn run(
    ctx: &Context,
    old_state: Option<&VoiceState>,
    new_state: &VoiceState,
) -> anyhow::Result<()> {
    let guild_id = new_state
        .guild_id
        .or_else(|| old_state.and_then(|s| s.guild_id))
        .context("Missing guild id in voice state event")?;

    let old_channel_id = old_state.and_then(|s| s.channel_id);
    let new_channel_id = new_state.channel_id;

    let state = get_state(ctx).await?;
    let settings = state.settings_repo.get_settings(guild_id).await?;

    if let (Some(lobby_id), Some(category_id)) = (
        settings.temp_voice_lobby_channel_id,
        settings.temp_voice_category_id,
    ) {
        let joined_lobby = new_channel_id == Some(lobby_id) && old_channel_id != Some(lobby_id);
        if joined_lobby {
            let channel_name = format!("temp-{}", new_state.user_id.get());
            let created_channel = guild_id
                .create_channel(
                    &ctx.http,
                    CreateChannel::new(channel_name)
                        .kind(ChannelType::Voice)
                        .category(category_id),
                )
                .await
                .context("Failed to create temporary voice channel")?;

            guild_id
                .move_member(&ctx.http, new_state.user_id, created_channel.id)
                .await
                .context("Failed to move user to temporary voice channel")?;

            state
                .private_voice_registry
                .write()
                .await
                .set_owner(created_channel.id, new_state.user_id);

            if let Some(mod_channel_id) = settings.mod_channel_id {
                let _ = mod_channel_id
                    .say(
                        &ctx.http,
                        format!(
                            "Created temp voice <#{}> for <@{}>",
                            created_channel.id.get(),
                            new_state.user_id.get()
                        ),
                    )
                    .await;
            }
        }
    }

    if let Some(left_channel_id) = old_channel_id {
        let is_temp_channel = state
            .private_voice_registry
            .read()
            .await
            .get_owner(left_channel_id)
            .is_some();

        if is_temp_channel {
            let is_empty = if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
                guild
                    .voice_states
                    .values()
                    .filter(|voice| voice.channel_id == Some(left_channel_id))
                    .count()
                    == 0
            } else {
                false
            };

            if is_empty {
                let _ = left_channel_id.delete(&ctx.http).await;
                state
                    .private_voice_registry
                    .write()
                    .await
                    .delete_owner(left_channel_id);

                if let Some(mod_channel_id) = settings.mod_channel_id {
                    let _ = mod_channel_id
                        .say(
                            &ctx.http,
                            format!("Deleted empty temp voice <#{}>", left_channel_id.get()),
                        )
                        .await;
                }
            }
        }
    }

    Ok(())
}
