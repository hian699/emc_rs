use anyhow::Context as _;
use serenity::all::VoiceState;
use serenity::client::Context;

use crate::get_state;

pub async fn run(ctx: &Context, new_state: &VoiceState) -> anyhow::Result<()> {
    let Some(guild_id) = new_state.guild_id else {
        return Ok(());
    };

    let state = get_state(ctx).await?;
    let Some(queue) = state.music_manager.get_queue(guild_id).await else {
        return Ok(());
    };

    if queue.read().await.is_auto_leave_suppressed() {
        return Ok(());
    }

    let bot_user_id = ctx.cache.current_user().id;
    let (bot_voice_channel, non_bot_count) = {
        let guild = guild_id
            .to_guild_cached(&ctx.cache)
            .context("Guild not in cache")?;
        let bot_voice_channel = guild
            .voice_states
            .get(&bot_user_id)
            .and_then(|v| v.channel_id);

        let non_bot_count = if let Some(channel_id) = bot_voice_channel {
            guild
                .voice_states
                .iter()
                .filter(|(user_id, voice)| {
                    **user_id != bot_user_id && voice.channel_id == Some(channel_id)
                })
                .count()
        } else {
            0
        };

        (bot_voice_channel, non_bot_count)
    };

    let Some(_channel_id) = bot_voice_channel else {
        state.music_manager.delete_queue(guild_id).await;
        return Ok(());
    };

    if non_bot_count == 0 {
        let bot_channel_id = bot_voice_channel;
        queue.write().await.destroy(ctx).await?;
        state.music_manager.delete_queue(guild_id).await;

        // Bot just left — if that channel was a temp voice with no real users left,
        // delete it now. The normal on_temp_voice_channels handler skips bot events,
        // so we have to handle cleanup here.
        if let Some(channel_id) = bot_channel_id {
            let temp_entry = state
                .private_voice_registry
                .read()
                .await
                .get_entry(channel_id);

            if let Some(entry) = temp_entry {
                // Re-check: after bot leaves, is the channel truly empty?
                let still_occupied = guild_id
                    .to_guild_cached(&ctx.cache)
                    .map(|guild| {
                        guild
                            .voice_states
                            .values()
                            .filter(|v| v.channel_id == Some(channel_id))
                            .any(|v| v.user_id != bot_user_id)
                    })
                    .unwrap_or(false);

                if !still_occupied {
                    match channel_id.delete(&ctx.http).await {
                        Ok(_) => {
                            state
                                .private_voice_registry
                                .write()
                                .await
                                .delete_owner(channel_id);

                            let settings = state
                                .settings_repo
                                .get_settings(guild_id)
                                .await
                                .ok();
                            if let Some(mod_channel_id) =
                                settings.and_then(|s| s.mod_channel_id)
                            {
                                let kind_label = match entry.kind {
                                    crate::utils::private_voice_registry::TempVoiceChannelKind::Public => "public",
                                    crate::utils::private_voice_registry::TempVoiceChannelKind::Private => "private",
                                };
                                let _ = mod_channel_id
                                    .say(
                                        &ctx.http,
                                        format!(
                                            "Deleted empty {} temp voice <#{}> (bot auto-left)",
                                            kind_label,
                                            channel_id.get()
                                        ),
                                    )
                                    .await;
                            }
                        }
                        Err(err) => {
                            tracing::warn!(
                                "Failed to delete temp voice {} after bot auto-left: {}",
                                channel_id,
                                err
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
