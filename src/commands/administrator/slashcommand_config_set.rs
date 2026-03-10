use anyhow::Context as _;
use serenity::all::{
    ChannelType, CommandInteraction, CommandOptionType, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::client::Context;

use crate::commands::{get_channel_id_option, get_role_id_option, get_string_option};
use crate::get_state;
use crate::utils::access_control::ensure_admin_for_slash;
use crate::utils::guild_settings::GuildSettings;

async fn require_channel_kind(
    ctx: &Context,
    channel_id: serenity::all::ChannelId,
    expected: ChannelType,
    label: &str,
) -> anyhow::Result<serenity::all::ChannelId> {
    let channel = channel_id.to_channel(&ctx.http).await?;
    let guild_channel = channel.guild().context("Not a guild channel")?;

    if guild_channel.kind != expected {
        anyhow::bail!(
            "{label} must be a {} channel, got {:?}",
            match expected {
                ChannelType::Category => "category",
                ChannelType::Voice => "voice",
                _ => "supported",
            },
            guild_channel.kind
        );
    }

    Ok(channel_id)
}

pub fn register() -> CreateCommand {
    CreateCommand::new("config-set")
        .description("Persist guild config into sqlite")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "key", "Config key")
                .add_string_choice("admin_role_add", "admin_role_add")
                .add_string_choice("developer_role_add", "developer_role_add")
                .add_string_choice("music_channel_add", "music_channel_add")
                .add_string_choice("private_voice_channel_add", "private_voice_channel_add")
                .add_string_choice("temp_voice_category", "temp_voice_category")
                .add_string_choice("temp_voice_public_lobby_channel", "temp_voice_public_lobby_channel")
                .add_string_choice("temp_voice_private_lobby_channel", "temp_voice_private_lobby_channel")
                .add_string_choice("temp_voice_lobby_channel", "temp_voice_lobby_channel")
                .add_string_choice("mod_channel", "mod_channel")
                .add_string_choice("admin_roles_csv", "admin_roles_csv")
                .add_string_choice("developer_roles_csv", "developer_roles_csv")
                .add_string_choice("music_channels_csv", "music_channels_csv")
                .add_string_choice("private_voice_channels_csv", "private_voice_channels_csv")
                .required(true),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::Role,
            "role",
            "Role value for role keys",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::Channel,
            "channel",
            "Channel/category value for channel keys",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::String,
            "csv",
            "CSV IDs for *_csv keys",
        ))
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    if !ensure_admin_for_slash(ctx, command).await? {
        return Ok(());
    }

    let guild_id = command.guild_id.context("Command not used in guild")?;
    let key = get_string_option(command, "key").context("Missing key")?;
    let state = get_state(ctx).await?;
    let mut settings = state.settings_repo.get_settings(guild_id).await?;

    match key.as_str() {
        "admin_role_add" => {
            let role_id = get_role_id_option(command, "role").context("Missing role option")?;
            settings.admin_role_ids.insert(role_id);
        }
        "developer_role_add" => {
            let role_id = get_role_id_option(command, "role").context("Missing role option")?;
            settings.developer_role_ids.insert(role_id);
        }
        "music_channel_add" => {
            let channel_id =
                get_channel_id_option(command, "channel").context("Missing channel option")?;
            settings.music_text_channel_ids.insert(channel_id);
        }
        "private_voice_channel_add" => {
            let channel_id =
                get_channel_id_option(command, "channel").context("Missing channel option")?;
            settings
                .private_voice_allowed_channel_ids
                .insert(channel_id);
        }
        "temp_voice_category" => {
            let channel_id =
                get_channel_id_option(command, "channel").context("Missing channel option")?;
            let channel_id =
                require_channel_kind(ctx, channel_id, ChannelType::Category, "temp_voice_category")
                    .await?;
            settings.temp_voice_category_id = Some(channel_id);
        }
        "temp_voice_public_lobby_channel" => {
            let channel_id =
                get_channel_id_option(command, "channel").context("Missing channel option")?;
            let channel_id = require_channel_kind(
                ctx,
                channel_id,
                ChannelType::Voice,
                "temp_voice_public_lobby_channel",
            )
            .await?;
            settings.temp_voice_public_lobby_channel_id = Some(channel_id);
        }
        "temp_voice_private_lobby_channel" => {
            let channel_id =
                get_channel_id_option(command, "channel").context("Missing channel option")?;
            let channel_id = require_channel_kind(
                ctx,
                channel_id,
                ChannelType::Voice,
                "temp_voice_private_lobby_channel",
            )
            .await?;
            settings.temp_voice_private_lobby_channel_id = Some(channel_id);
        }
        "temp_voice_lobby_channel" => {
            let channel_id =
                get_channel_id_option(command, "channel").context("Missing channel option")?;
            let channel_id = require_channel_kind(
                ctx,
                channel_id,
                ChannelType::Voice,
                "temp_voice_lobby_channel",
            )
            .await?;
            settings.temp_voice_private_lobby_channel_id = Some(channel_id);
        }
        "mod_channel" => {
            let channel_id =
                get_channel_id_option(command, "channel").context("Missing channel option")?;
            settings.mod_channel_id = Some(channel_id);
        }
        "admin_roles_csv" => {
            let csv = get_string_option(command, "csv").context("Missing csv option")?;
            settings.admin_role_ids = GuildSettings::parse_ids_csv(&csv)
                .into_iter()
                .map(serenity::all::RoleId::new)
                .collect();
        }
        "developer_roles_csv" => {
            let csv = get_string_option(command, "csv").context("Missing csv option")?;
            settings.developer_role_ids = GuildSettings::parse_ids_csv(&csv)
                .into_iter()
                .map(serenity::all::RoleId::new)
                .collect();
        }
        "music_channels_csv" => {
            let csv = get_string_option(command, "csv").context("Missing csv option")?;
            settings.music_text_channel_ids = GuildSettings::parse_ids_csv(&csv)
                .into_iter()
                .map(serenity::all::ChannelId::new)
                .collect();
        }
        "private_voice_channels_csv" => {
            let csv = get_string_option(command, "csv").context("Missing csv option")?;
            settings.private_voice_allowed_channel_ids = GuildSettings::parse_ids_csv(&csv)
                .into_iter()
                .map(serenity::all::ChannelId::new)
                .collect();
        }
        _ => anyhow::bail!("Unsupported config key"),
    }

    state.settings_repo.save_settings(&settings).await?;

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content("Config saved to sqlite"),
            ),
        )
        .await?;
    Ok(())
}
