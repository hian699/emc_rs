mod commands;
mod components;
mod events;
mod state;
mod utils;

use std::env;
use std::sync::Arc;

use anyhow::Context as _;
#[cfg(feature = "lavalink")]
use lavalink_rs::prelude::LavalinkClient;
use serenity::all::{
    Command, GatewayIntents, Interaction, Message, Ready, VoiceServerUpdateEvent, VoiceState,
};
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::prelude::TypeMapKey;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::state::BotState;
use crate::utils::lavalink_client::{create_client, lavalink_enabled_from_env};
use crate::utils::music_manager::MusicManager;
use crate::utils::private_voice_registry::PrivateVoiceRegistry;
use crate::utils::search_cache::SearchCache;
use crate::utils::settings_repository::SettingsRepository;

fn read_discord_token() -> anyhow::Result<String> {
    if let Ok(token) = env::var("DISCORD_TOKEN") {
        if !token.trim().is_empty() {
            return Ok(token);
        }
    }

    if let Ok(token) = env::var("BOT_TOKEN") {
        if !token.trim().is_empty() {
            return Ok(token);
        }
    }

    Err(anyhow::anyhow!(
        "Missing DISCORD_TOKEN (or BOT_TOKEN). Set it in .env or shell before running."
    ))
}

fn normalize_sqlite_url(raw: String) -> String {
    if !raw.starts_with("sqlite:") {
        return raw;
    }

    if raw.contains("mode=") {
        return raw;
    }

    if raw.contains('?') {
        format!("{raw}&mode=rwc")
    } else {
        format!("{raw}?mode=rwc")
    }
}

struct Handler;

struct BotStateKey;

impl TypeMapKey for BotStateKey {
    type Value = Arc<BotState>;
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Connected as {}", ready.user.tag());

        if let Err(err) = init_lavalink_if_needed(&ctx, ready.user.id).await {
            warn!("Lavalink init skipped/failed: {err}");
        }

        let commands = commands::register_slash_commands();
        if let Err(err) = Command::set_global_commands(&ctx.http, commands).await {
            error!("Failed to register slash commands: {err}");
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                if let Err(err) = commands::dispatch_slash(&ctx, &command).await {
                    error!("Slash command failed: {err}");
                }
            }
            Interaction::Component(component) => {
                if let Err(err) = components::dispatch_component(&ctx, &component).await {
                    error!("Component interaction failed: {err}");
                }
            }
            _ => {}
        }
    }

    async fn message(&self, ctx: Context, message: Message) {
        if message.author.bot {
            return;
        }

        if let Err(err) = commands::dispatch_message(&ctx, &message).await {
            error!("Message command failed: {err}");
        }
    }

    async fn voice_server_update(&self, ctx: Context, event: VoiceServerUpdateEvent) {
        #[cfg(feature = "lavalink")]
        if let (Some(guild_id), Ok(Some(client))) =
            (event.guild_id, get_lavalink_client(&ctx).await)
        {
            client.handle_voice_server_update(guild_id, event.token, event.endpoint);
        }
    }

    async fn voice_state_update(&self, ctx: Context, _old: Option<VoiceState>, new: VoiceState) {
        if let Err(err) =
            events::voice_state_update::on_temp_voice_channels::run(&ctx, _old.as_ref(), &new).await
        {
            error!("Temp voice handler failed: {err}");
        }

        #[cfg(feature = "lavalink")]
        if let (Some(guild_id), Ok(Some(client))) = (new.guild_id, get_lavalink_client(&ctx).await)
        {
            client.handle_voice_state_update(
                guild_id,
                new.channel_id,
                new.user_id,
                new.session_id.clone(),
            );
        }

        if let Err(err) = events::voice_state_update::on_music_auto_leave::run(&ctx, &new).await {
            error!("VoiceStateUpdate handler failed: {err}");
        }
    }
}

async fn load_state() -> anyhow::Result<Arc<BotState>> {
    let database_url = normalize_sqlite_url(
        env::var("SQLITE_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://bot_config.db?mode=rwc".to_string()),
    );
    let settings_repo = Arc::new(SettingsRepository::new(&database_url).await?);

    Ok(Arc::new(BotState {
        settings_repo,
        music_manager: Arc::new(MusicManager::new()),
        search_cache: Arc::new(RwLock::new(SearchCache::new())),
        private_voice_registry: Arc::new(RwLock::new(PrivateVoiceRegistry::new())),
        #[cfg(feature = "lavalink")]
        lavalink_client: Arc::new(RwLock::new(None::<LavalinkClient>)),
    }))
}

#[cfg(feature = "lavalink")]
async fn init_lavalink_if_needed(
    ctx: &Context,
    bot_user_id: serenity::all::UserId,
) -> anyhow::Result<()> {
    if !lavalink_enabled_from_env() {
        return Ok(());
    }

    let state = get_state(ctx).await?;
    let mut slot = state.lavalink_client.write().await;
    if slot.is_none() {
        let client = create_client(bot_user_id).await?;
        *slot = Some(client);
    }
    Ok(())
}

#[cfg(not(feature = "lavalink"))]
async fn init_lavalink_if_needed(
    _ctx: &Context,
    _bot_user_id: serenity::all::UserId,
) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(feature = "lavalink")]
pub async fn get_lavalink_client(ctx: &Context) -> anyhow::Result<Option<LavalinkClient>> {
    let state = get_state(ctx).await?;
    Ok(state.lavalink_client.read().await.clone())
}

#[cfg(not(feature = "lavalink"))]
pub async fn get_lavalink_client(_ctx: &Context) -> anyhow::Result<Option<()>> {
    Ok(None)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    let token = read_discord_token()?;

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;

    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .await
        .context("Failed to create Discord client")?;

    {
        let mut data = client.data.write().await;
        data.insert::<BotStateKey>(load_state().await?);
    }

    client
        .start()
        .await
        .context("Discord client exited with error")
}

pub async fn get_state(ctx: &Context) -> anyhow::Result<Arc<BotState>> {
    let data = ctx.data.read().await;
    data.get::<BotStateKey>()
        .cloned()
        .context("Bot state is not initialized")
}
