use anyhow::Context as _;
use serenity::all::{
    CommandInteraction, CommandOptionType, CreateActionRow, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption,
};
use serenity::client::Context;

use crate::commands::get_string_option;
use crate::components::select_menu::music_search::format_duration;
use crate::get_lavalink_client;
use crate::get_state;
use crate::utils::access_control::ensure_music_channel_for_slash;
use crate::utils::discord_embed::{info_embed, success_embed, warning_embed};
use crate::utils::lavalink_client::search_tracks;
use crate::utils::music_queue::MusicQueue;
use crate::utils::music_queue::SongItem;
use crate::utils::ytdlp_helper::YtDlpHelper;

pub fn register() -> CreateCommand {
    CreateCommand::new("play")
        .description("Play a song from URL or keyword")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "query", "URL or search keyword")
                .required(true),
        )
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    if !ensure_music_channel_for_slash(ctx, command).await? {
        return Ok(());
    }

    let guild_id = command.guild_id.context("Command not used in guild")?;
    let query = get_string_option(command, "query").context("Missing query")?;
    let state = get_state(ctx).await?;

    let voice_channel_id = guild_id
        .to_guild_cached(&ctx.cache)
        .and_then(|guild| {
            guild
                .voice_states
                .get(&command.user.id)
                .and_then(|state| state.channel_id)
        })
        .context("Join a voice channel first")?;

    let queue = if let Some(q) = state.music_manager.get_queue(guild_id).await {
        q
    } else {
        state
            .music_manager
            .create_queue(guild_id, command.channel_id)
            .await
    };

    if query.starts_with("http://") || query.starts_with("https://") {
        if let Err(err) = MusicQueue::connect(ctx, voice_channel_id).await {
            command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().embed(warning_embed(
                            "Voice Connect Failed",
                            format!(
                                "Cannot connect to your voice channel. Check bot Connect/Speak permissions.\nDetails: {err}"
                            ),
                        )),
                    ),
                )
                .await?;
            return Ok(());
        }

        let item = if let Some(client) = get_lavalink_client(ctx).await? {
            match search_tracks(&client, guild_id, &query).await {
                Ok(tracks) => {
                    let (title, url, duration_ms, encoded) =
                        tracks.into_iter().next().context("No tracks found")?;
                    SongItem {
                        title,
                        url,
                        duration_ms: Some(duration_ms),
                        requested_by: command.user.tag(),
                        lavalink_encoded_track: Some(encoded),
                    }
                }
                Err(_) => {
                    let info = YtDlpHelper::get_video_info(&query).await?;
                    SongItem {
                        title: info.title,
                        url: info.webpage_url,
                        duration_ms: info.duration.map(|d| (d * 1000.0) as u64),
                        requested_by: command.user.tag(),
                        lavalink_encoded_track: None,
                    }
                }
            }
        } else {
            let info = YtDlpHelper::get_video_info(&query).await?;
            SongItem {
                title: info.title,
                url: info.webpage_url,
                duration_ms: info.duration.map(|d| (d * 1000.0) as u64),
                requested_by: command.user.tag(),
                lavalink_encoded_track: None,
            }
        };

        let should_play_now = {
            let mut q = queue.write().await;
            q.enqueue_song(item.clone())
        };
        MusicQueue::sync_lavalink_enqueue(ctx, guild_id, &item, should_play_now).await?;

        command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(success_embed(
                        "Song Added",
                        format!("**{}** ({})", item.title, format_duration(item.duration_ms)),
                    )),
                ),
            )
            .await?;

        return Ok(());
    }

    let songs: Vec<SongItem> = if let Some(client) = get_lavalink_client(ctx).await? {
        match search_tracks(&client, guild_id, &query).await {
            Ok(tracks) => tracks
                .into_iter()
                .map(|(title, url, duration_ms, encoded)| SongItem {
                    title,
                    url,
                    duration_ms: Some(duration_ms),
                    requested_by: command.user.tag(),
                    lavalink_encoded_track: Some(encoded),
                })
                .collect(),
            Err(_) => {
                let results = YtDlpHelper::search(&query).await?;
                results
                    .into_iter()
                    .map(|video| SongItem {
                        title: video.title,
                        url: video.webpage_url,
                        duration_ms: video.duration.map(|d| (d * 1000.0) as u64),
                        requested_by: command.user.tag(),
                        lavalink_encoded_track: None,
                    })
                    .collect()
            }
        }
    } else {
        let results = YtDlpHelper::search(&query).await?;
        results
            .into_iter()
            .map(|video| SongItem {
                title: video.title,
                url: video.webpage_url,
                duration_ms: video.duration.map(|d| (d * 1000.0) as u64),
                requested_by: command.user.tag(),
                lavalink_encoded_track: None,
            })
            .collect()
    };

    if songs.is_empty() {
        command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .embed(warning_embed("No Results", "No search results found")),
                ),
            )
            .await?;
        return Ok(());
    }

    let cache_key = format!("search:{}:{}", guild_id.get(), command.user.id.get());
    state
        .search_cache
        .write()
        .await
        .store_results(cache_key.clone(), songs.clone());

    if let Err(err) = MusicQueue::connect(ctx, voice_channel_id).await {
        command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(warning_embed(
                        "Voice Connect Failed",
                        format!(
                            "Cannot connect to your voice channel. Check bot Connect/Speak permissions.\nDetails: {err}"
                        ),
                    )),
                ),
            )
            .await?;
        return Ok(());
    }

    let options: Vec<CreateSelectMenuOption> = songs
        .iter()
        .take(25)
        .map(|song| {
            CreateSelectMenuOption::new(
                format!("{} ({})", song.title, format_duration(song.duration_ms)),
                song.url.clone(),
            )
        })
        .collect();

    let select = CreateSelectMenu::new(
        format!("music-search:{cache_key}"),
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Select a song to add to queue");

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(info_embed("Search Results", "Select one song from the menu below."))
                    .components(vec![CreateActionRow::SelectMenu(select)]),
            ),
        )
        .await?;

    Ok(())
}
