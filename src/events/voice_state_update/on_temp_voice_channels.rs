use anyhow::Context as _;
use serenity::all::{
    ChannelType, CreateChannel, PermissionOverwrite, PermissionOverwriteType, Permissions, RoleId,
    VoiceState,
};
use serenity::client::Context;
use tracing::warn;

use crate::get_state;
use crate::utils::private_voice_registry::TempVoiceChannelKind;

fn build_temp_voice_channel_name(kind: TempVoiceChannelKind, new_state: &VoiceState) -> String {
    let owner_name = new_state
        .member
        .as_ref()
        .map(|member| {
            member
                .nick
                .as_deref()
                .unwrap_or(member.user.name.as_str())
                .trim()
                .to_string()
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("User {}", new_state.user_id.get()));

    let suffix = match kind {
        TempVoiceChannelKind::Public => "Voice",
        TempVoiceChannelKind::Private => "Private Voice",
    };
    let mut channel_name = format!("{}'s {}", owner_name, suffix);
    if channel_name.chars().count() > 100 {
        channel_name = channel_name.chars().take(100).collect();
    }
    channel_name
}

fn kind_label(kind: TempVoiceChannelKind) -> &'static str {
    match kind {
        TempVoiceChannelKind::Public => "public",
        TempVoiceChannelKind::Private => "private",
    }
}

pub async fn run(
    ctx: &Context,
    old_state: Option<&VoiceState>,
    new_state: &VoiceState,
) -> anyhow::Result<()> {
    let guild_id = new_state
        .guild_id
        .or_else(|| old_state.and_then(|s| s.guild_id))
        .context("Missing guild id in voice state event")?;

    if new_state.user_id == ctx.cache.current_user().id
        || new_state
            .member
            .as_ref()
            .is_some_and(|member| member.user.bot)
    {
        return Ok(());
    }

    let old_channel_id = old_state.and_then(|s| s.channel_id);
    let new_channel_id = new_state.channel_id;

    let state = get_state(ctx).await?;
    let settings = state.settings_repo.get_settings(guild_id).await?;

    if let Some(category_id) = settings.temp_voice_category_id {
        let joined_public_lobby = settings.temp_voice_public_lobby_channel_id.is_some()
            && new_channel_id == settings.temp_voice_public_lobby_channel_id
            && old_channel_id != settings.temp_voice_public_lobby_channel_id;
        let joined_private_lobby = settings.temp_voice_private_lobby_channel_id.is_some()
            && new_channel_id == settings.temp_voice_private_lobby_channel_id
            && old_channel_id != settings.temp_voice_private_lobby_channel_id;

        let joined_kind = if joined_private_lobby {
            Some(TempVoiceChannelKind::Private)
        } else if joined_public_lobby {
            Some(TempVoiceChannelKind::Public)
        } else {
            None
        };

        if let Some(kind) = joined_kind {
            let channel_name = build_temp_voice_channel_name(kind, new_state);
            let mut builder = CreateChannel::new(channel_name)
                .kind(ChannelType::Voice)
                .category(category_id);

            if kind == TempVoiceChannelKind::Private {
                builder = builder.permissions(vec![
                    PermissionOverwrite {
                        allow: Permissions::VIEW_CHANNEL | Permissions::CONNECT,
                        deny: Permissions::empty(),
                        kind: PermissionOverwriteType::Member(new_state.user_id),
                    },
                    PermissionOverwrite {
                        allow: Permissions::empty(),
                        deny: Permissions::VIEW_CHANNEL | Permissions::CONNECT,
                        kind: PermissionOverwriteType::Role(RoleId::new(guild_id.get())),
                    },
                ]);
            }

            let created_channel = guild_id
                .create_channel(&ctx.http, builder)
                .await
                .with_context(|| {
                    format!("Failed to create {} temp voice channel", kind_label(kind))
                })?;

            let latest_channel_id = guild_id.to_guild_cached(&ctx.cache).and_then(|guild| {
                guild
                    .voice_states
                    .get(&new_state.user_id)
                    .and_then(|voice_state| voice_state.channel_id)
            });

            if latest_channel_id != new_channel_id {
                if let Err(delete_err) = created_channel.delete(&ctx.http).await {
                    warn!(
                        "Failed to clean up {} temp voice {} after stale voice state: {}",
                        kind_label(kind),
                        created_channel.id,
                        delete_err
                    );
                }

                return Ok(());
            }

            if let Err(err) = guild_id
                .move_member(&ctx.http, new_state.user_id, created_channel.id)
                .await
            {
                if let Err(delete_err) = created_channel.delete(&ctx.http).await {
                    warn!(
                        "Failed to clean up {} temp voice {} after move failure: {}",
                        kind_label(kind),
                        created_channel.id,
                        delete_err
                    );
                }

                return Err(err).with_context(|| {
                    format!(
                        "Failed to move user to {} temp voice channel",
                        kind_label(kind)
                    )
                });
            }

            state.private_voice_registry.write().await.set_channel(
                created_channel.id,
                new_state.user_id,
                kind,
            );

            if let Some(mod_channel_id) = settings.mod_channel_id {
                let _ = mod_channel_id
                    .say(
                        &ctx.http,
                        format!(
                            "Created {} temp voice <#{}> for <@{}>",
                            kind_label(kind),
                            created_channel.id.get(),
                            new_state.user_id.get()
                        ),
                    )
                    .await;
            }
        }
    }

    if let Some(left_channel_id) = old_channel_id {
        let temp_voice_entry = state
            .private_voice_registry
            .read()
            .await
            .get_entry(left_channel_id);

        if let Some(entry) = temp_voice_entry {
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
                match left_channel_id.delete(&ctx.http).await {
                    Ok(_) => {
                        state
                            .private_voice_registry
                            .write()
                            .await
                            .delete_owner(left_channel_id);

                        if let Some(mod_channel_id) = settings.mod_channel_id {
                            let _ = mod_channel_id
                                .say(
                                    &ctx.http,
                                    format!(
                                        "Deleted empty {} temp voice <#{}>",
                                        kind_label(entry.kind),
                                        left_channel_id.get()
                                    ),
                                )
                                .await;
                        }
                    }
                    Err(err) => {
                        warn!(
                            "Failed to delete empty {} temp voice {}: {}",
                            kind_label(entry.kind),
                            left_channel_id,
                            err
                        );

                        if let Some(mod_channel_id) = settings.mod_channel_id {
                            let _ = mod_channel_id
                                .say(
                                    &ctx.http,
                                    format!(
                                        "Failed to delete empty {} temp voice <#{}>: {}",
                                        kind_label(entry.kind),
                                        left_channel_id.get(),
                                        err
                                    ),
                                )
                                .await;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
