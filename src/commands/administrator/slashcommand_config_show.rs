use anyhow::Context as _;
use serenity::all::{
    CommandInteraction, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::client::Context;

use crate::get_state;
use crate::utils::access_control::ensure_admin_for_slash;

fn format_role_list(
    ids: &std::collections::HashSet<serenity::all::RoleId>,
    guild: Option<&serenity::all::Guild>,
) -> String {
    let mut items: Vec<String> = ids
        .iter()
        .map(|id| {
            let label = guild
                .and_then(|guild| guild.roles.get(id))
                .map(|role| role.name.clone())
                .unwrap_or_else(|| format!("<@&{}>", id.get()));
            format!("{} ({})", label, id.get())
        })
        .collect();
    items.sort();
    if items.is_empty() {
        "<empty>".to_string()
    } else {
        items.join(", ")
    }
}

fn format_channel_list(
    ids: &std::collections::HashSet<serenity::all::ChannelId>,
    guild: Option<&serenity::all::Guild>,
) -> String {
    let mut items: Vec<String> = ids
        .iter()
        .map(|id| {
            let label = guild
                .and_then(|guild| guild.channels.get(id))
                .map(|channel| format!("#{}", channel.name))
                .unwrap_or_else(|| format!("<#{}>", id.get()));
            format!("{} ({})", label, id.get())
        })
        .collect();
    items.sort();
    if items.is_empty() {
        "<empty>".to_string()
    } else {
        items.join(", ")
    }
}

fn format_optional_channel(
    id: Option<serenity::all::ChannelId>,
    guild: Option<&serenity::all::Guild>,
) -> String {
    id.map(|id| {
        let label = guild
            .and_then(|guild| guild.channels.get(&id))
            .map(|channel| format!("#{}", channel.name))
            .unwrap_or_else(|| format!("<#{}>", id.get()));
        format!("{} ({})", label, id.get())
    })
    .unwrap_or_else(|| "<empty>".to_string())
}

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
    let content = {
        let guild = guild_id.to_guild_cached(&ctx.cache);
        format!(
            "Guild config\nadmin_roles: {}\ndeveloper_roles: {}\nmusic_channels: {}\nprivate_voice_allowed_channels: {}\ntemp_voice_category: {}\ntemp_voice_public_lobby_channel: {}\ntemp_voice_private_lobby_channel: {}\nmod_channel: {}",
            format_role_list(&settings.admin_role_ids, guild.as_deref()),
            format_role_list(&settings.developer_role_ids, guild.as_deref()),
            format_channel_list(&settings.music_text_channel_ids, guild.as_deref()),
            format_channel_list(
                &settings.private_voice_allowed_channel_ids,
                guild.as_deref()
            ),
            format_optional_channel(settings.temp_voice_category_id, guild.as_deref()),
            format_optional_channel(
                settings.temp_voice_public_lobby_channel_id,
                guild.as_deref()
            ),
            format_optional_channel(
                settings.temp_voice_private_lobby_channel_id,
                guild.as_deref()
            ),
            format_optional_channel(settings.mod_channel_id, guild.as_deref())
        )
    };

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content(content),
            ),
        )
        .await?;

    Ok(())
}
