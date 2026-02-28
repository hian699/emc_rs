pub mod administrator;
pub mod developer;
pub mod music;
pub mod user;
pub mod utility;

use anyhow::anyhow;
use serenity::all::{CommandInteraction, CreateCommand, Message, ResolvedValue};
use serenity::client::Context;

pub fn register_slash_commands() -> Vec<CreateCommand> {
    let mut commands = Vec::new();
    commands.extend(utility::register());
    commands.extend(user::register());
    commands.extend(administrator::register());
    commands.extend(developer::register());
    commands.extend(music::register());
    commands
}

pub async fn dispatch_slash(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    match command.data.name.as_str() {
        "ping" => utility::slashcommand_ping::run(ctx, command).await,
        "random" => user::slashcommand_random::run(ctx, command).await,
        "timeout" => administrator::slashcommand_timeout::run(ctx, command).await,
        "deletemessage" => administrator::slashcommand_deletemessage::run(ctx, command).await,
        "security-lockdown" => {
            administrator::slashcommand_security_lockdown::run(ctx, command).await
        }
        "config-set" => administrator::slashcommand_config_set::run(ctx, command).await,
        "config-show" => administrator::slashcommand_config_show::run(ctx, command).await,
        "reload" => developer::slashcommand_reload::run(ctx, command).await,
        "eval" => developer::slashcommand_eval::run(ctx, command).await,
        "play" => music::slashcommand_play::run(ctx, command).await,
        "skip" => music::slashcommand_skip::run(ctx, command).await,
        "stop" => music::slashcommand_stop::run(ctx, command).await,
        _ => Err(anyhow!("Unsupported slash command")),
    }
}

pub async fn dispatch_message(ctx: &Context, message: &Message) -> anyhow::Result<()> {
    let content = message.content.trim();

    if content == "!ping" {
        return utility::messagecommand_ping::run(ctx, message).await;
    }

    if content == "!reload" {
        return developer::messagecommand_reload::run(ctx, message).await;
    }

    if content == "!skip" {
        return music::messagecommand_skip::run(ctx, message).await;
    }

    if content == "!stop" {
        return music::messagecommand_stop::run(ctx, message).await;
    }

    if let Some(query) = content.strip_prefix("!play ") {
        return music::messagecommand_play::run(ctx, message, query).await;
    }

    if let Some(code) = content.strip_prefix("!eval ") {
        return developer::messagecommand_eval::run(ctx, message, code).await;
    }

    Ok(())
}

pub fn get_bool_option(command: &CommandInteraction, name: &str) -> Option<bool> {
    command
        .data
        .options()
        .iter()
        .find(|opt| opt.name == name)
        .and_then(|opt| match opt.value {
            ResolvedValue::Boolean(b) => Some(b),
            _ => None,
        })
}

pub fn get_i64_option(command: &CommandInteraction, name: &str) -> Option<i64> {
    command
        .data
        .options()
        .iter()
        .find(|opt| opt.name == name)
        .and_then(|opt| match opt.value {
            ResolvedValue::Integer(v) => Some(v),
            _ => None,
        })
}

pub fn get_string_option(command: &CommandInteraction, name: &str) -> Option<String> {
    command
        .data
        .options()
        .iter()
        .find(|opt| opt.name == name)
        .and_then(|opt| match &opt.value {
            ResolvedValue::String(v) => Some((*v).to_string()),
            _ => None,
        })
}

pub fn get_user_id_option(
    command: &CommandInteraction,
    name: &str,
) -> Option<serenity::all::UserId> {
    command
        .data
        .options()
        .iter()
        .find(|opt| opt.name == name)
        .and_then(|opt| match opt.value {
            ResolvedValue::User(user, _) => Some(user.id),
            _ => None,
        })
}

pub fn get_role_id_option(
    command: &CommandInteraction,
    name: &str,
) -> Option<serenity::all::RoleId> {
    command
        .data
        .options()
        .iter()
        .find(|opt| opt.name == name)
        .and_then(|opt| match opt.value {
            ResolvedValue::Role(role) => Some(role.id),
            _ => None,
        })
}

pub fn get_channel_id_option(
    command: &CommandInteraction,
    name: &str,
) -> Option<serenity::all::ChannelId> {
    command
        .data
        .options()
        .iter()
        .find(|opt| opt.name == name)
        .and_then(|opt| match opt.value {
            ResolvedValue::Channel(channel) => Some(channel.id),
            _ => None,
        })
}
