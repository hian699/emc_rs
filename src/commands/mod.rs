pub mod administrator;
pub mod developer;
pub mod music;
pub mod user;
pub mod utility;

use anyhow::anyhow;
use serenity::all::{CommandInteraction, CreateCommand, Message, ResolvedValue};
use serenity::client::Context;

use crate::utils::access_control::{
    ensure_developer_for_message, ensure_developer_for_slash, ensure_owner_for_message,
    ensure_owner_for_slash,
};

#[derive(Clone, Copy)]
enum CommandRole {
    Owner,
    Developer,
    User,
}

fn required_slash_role(name: &str) -> Option<CommandRole> {
    match name {
        "timeout" | "deletemessage" | "security-lockdown" | "config-set" | "config-show"
        | "reload" => Some(CommandRole::Owner),
        "eval" => Some(CommandRole::Developer),
        "ping" | "random" | "play" | "skip" | "stop" => Some(CommandRole::User),
        _ => None,
    }
}

fn required_message_role(name: &str) -> Option<CommandRole> {
    match name {
        "!reload" => Some(CommandRole::Owner),
        "!eval" => Some(CommandRole::Developer),
        "!ping" | "!play" | "!skip" | "!stop" => Some(CommandRole::User),
        _ => None,
    }
}

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
    if let Some(role) = required_slash_role(command.data.name.as_str()) {
        let allowed = match role {
            CommandRole::Owner => ensure_owner_for_slash(ctx, command).await?,
            CommandRole::Developer => ensure_developer_for_slash(ctx, command).await?,
            CommandRole::User => true,
        };

        if !allowed {
            return Ok(());
        }
    }

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

    if let Some(name) = content.split_whitespace().next() {
        if let Some(role) = required_message_role(name) {
            let allowed = match role {
                CommandRole::Owner => ensure_owner_for_message(ctx, message).await?,
                CommandRole::Developer => ensure_developer_for_message(ctx, message).await?,
                CommandRole::User => true,
            };

            if !allowed {
                return Ok(());
            }
        }
    }

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
