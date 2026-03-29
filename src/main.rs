mod commands;
mod components;
mod events;
mod state;
mod utils;

use std::env;
use std::sync::Arc;
use std::time::Duration;

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

use crate::state::BotState;
use crate::utils::lavalink_client::lavalink_enabled_from_env;
#[cfg(feature = "lavalink")]
use crate::utils::lavalink_client::set_lavalink_runtime_context;
use crate::utils::lavalink_runtime::{
    get_lavalink_client as runtime_get_lavalink_client, init_lavalink_if_needed,
};
use crate::utils::logging::init_logging;
use crate::utils::music_manager::MusicManager;
use crate::utils::private_voice_registry::PrivateVoiceRegistry;
use crate::utils::search_cache::SearchCache;
use crate::utils::settings_repository::SettingsRepository;
#[cfg(feature = "lavalink")]
use songbird::SerenityInit;

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

        #[cfg(feature = "lavalink")]
        set_lavalink_runtime_context(&ctx);

        let lavalink_ctx = ctx.clone();
        let bot_user_id = ready.user.id;
        tokio::spawn(async move {
            let init_result = tokio::time::timeout(
                Duration::from_secs(12),
                init_lavalink_if_needed(&lavalink_ctx, bot_user_id, "ready-event"),
            )
            .await;

            match init_result {
                Ok(Ok(Some(_))) => info!("Lavalink client initialized during ready event"),
                Ok(Ok(None)) => info!("Lavalink init skipped (disabled or retry window active)"),
                Ok(Err(err)) => warn!("Lavalink init skipped/failed: {err}"),
                Err(_) => warn!("Lavalink init timed out after 12s"),
            }
        });

        let commands = commands::register_slash_commands();
        if let Err(err) = Command::set_global_commands(&ctx.http, commands).await {
            error!("Failed to register slash commands: {err}");
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    if let Err(err) = commands::dispatch_slash(&ctx, &command).await {
                        error!("Slash command failed: {err}");
                        // Try to surface the error to the user as an ephemeral reply.
                        // If the interaction was already deferred/responded we must use
                        // edit_response; otherwise use create_response.
                        use crate::utils::discord_embed::error_embed;
                        use serenity::all::{
                            CreateInteractionResponse, CreateInteractionResponseMessage,
                            EditInteractionResponse,
                        };
                        let embed = error_embed("Error", format!("{err}"));
                        let via_edit = command.get_response(&ctx.http).await.is_ok();
                        if via_edit {
                            let _ = command
                                .edit_response(
                                    &ctx.http,
                                    EditInteractionResponse::new().embed(embed),
                                )
                                .await;
                        } else {
                            let _ = command
                                .create_response(
                                    &ctx.http,
                                    CreateInteractionResponse::Message(
                                        CreateInteractionResponseMessage::new()
                                            .ephemeral(true)
                                            .embed(embed),
                                    ),
                                )
                                .await;
                        }
                    }
                });
            }
            Interaction::Component(component) => {
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    if let Err(err) = components::dispatch_component(&ctx, &component).await {
                        error!("Component interaction failed: {err}");
                        use crate::utils::discord_embed::error_embed;
                        use serenity::all::{
                            CreateInteractionResponse, CreateInteractionResponseMessage,
                            EditInteractionResponse,
                        };
                        let embed = error_embed("Error", format!("{err}"));
                        let via_edit = component.get_response(&ctx.http).await.is_ok();
                        if via_edit {
                            let _ = component
                                .edit_response(
                                    &ctx.http,
                                    EditInteractionResponse::new().embed(embed),
                                )
                                .await;
                        } else {
                            let _ = component
                                .create_response(
                                    &ctx.http,
                                    CreateInteractionResponse::Message(
                                        CreateInteractionResponseMessage::new()
                                            .ephemeral(true)
                                            .embed(embed),
                                    ),
                                )
                                .await;
                        }
                    }
                });
            }
            _ => {}
        }
    }

    async fn message(&self, ctx: Context, message: Message) {
        if message.author.bot {
            return;
        }

        let ctx = ctx.clone();
        tokio::spawn(async move {
            if let Err(err) = commands::dispatch_message(&ctx, &message).await {
                error!("Message command failed: {err}");
            }
        });
    }

    async fn voice_server_update(&self, ctx: Context, event: VoiceServerUpdateEvent) {
        #[cfg(feature = "lavalink")]
        {
            let guild_id = event.guild_id;
            tracing::debug!(
                "[Lavalink] voice_server_update: guild={:?} endpoint={:?}",
                guild_id,
                event.endpoint
            );
            if let (Some(guild_id), Ok(Some(client))) = (guild_id, get_lavalink_client(&ctx).await)
            {
                client.handle_voice_server_update(guild_id, event.token, event.endpoint);
                tracing::debug!(
                    "[Lavalink] forwarded voice_server_update for guild {:?}",
                    guild_id
                );
            } else {
                if lavalink_enabled_from_env() {
                    tracing::debug!(
                        "[Lavalink] voice_server_update skipped: guild={:?} (client missing or no guild_id)",
                        guild_id
                    );
                }
            }
        }
    }

    async fn voice_state_update(&self, ctx: Context, _old: Option<VoiceState>, new: VoiceState) {
        if let Err(err) =
            events::voice_state_update::on_temp_voice_channels::run(&ctx, _old.as_ref(), &new).await
        {
            error!("Private temp voice handler failed: {err}");
        }

        #[cfg(feature = "lavalink")]
        {
            let guild_id = new.guild_id;
            let user_id = new.user_id;
            let channel_id = new.channel_id;
            tracing::debug!(
                "[Lavalink] voice_state_update: guild={:?} user={:?} channel={:?}",
                guild_id,
                user_id,
                channel_id
            );
            if let (Some(guild_id), Ok(Some(client))) = (guild_id, get_lavalink_client(&ctx).await)
            {
                client.handle_voice_state_update(
                    guild_id,
                    channel_id,
                    user_id,
                    new.session_id.clone(),
                );
                tracing::debug!(
                    "[Lavalink] forwarded voice_state_update for guild {:?} user {:?}",
                    guild_id,
                    user_id
                );
            } else {
                if lavalink_enabled_from_env() {
                    tracing::debug!(
                        "[Lavalink] voice_state_update skipped: guild={:?} user={:?} (client missing or no guild_id)",
                        guild_id,
                        user_id
                    );
                }
            }
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
        lavalink_runtime: Arc::new(RwLock::new(crate::state::LavalinkRuntimeState::default())),
        #[cfg(feature = "lavalink")]
        lavalink_init_lock: Arc::new(tokio::sync::Mutex::new(())),
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    init_logging()?;

    let token = read_discord_token()?;
    info!(
        "Starting EMC RS bot (lavalink_configured={}, sqlite_configured={})",
        lavalink_enabled_from_env(),
        env::var("SQLITE_DATABASE_URL").is_ok()
    );

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;

    let client_builder = Client::builder(token, intents).event_handler(Handler);
    #[cfg(feature = "lavalink")]
    let client_builder = client_builder.register_songbird();
    let mut client = client_builder
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

#[cfg(feature = "lavalink")]
pub async fn get_lavalink_client(ctx: &Context) -> anyhow::Result<Option<LavalinkClient>> {
    runtime_get_lavalink_client(ctx).await
}

#[cfg(not(feature = "lavalink"))]
pub async fn get_lavalink_client(_ctx: &Context) -> anyhow::Result<Option<()>> {
    Ok(None)
}
