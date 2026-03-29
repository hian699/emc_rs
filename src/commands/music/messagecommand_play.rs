use anyhow::Context as _;
use serenity::all::{
    CreateActionRow, CreateMessage, CreateSelectMenu, CreateSelectMenuKind, Message,
};
use serenity::client::Context;

use crate::commands::music::playback::{
    build_search_options, enqueue_track, prepare_playback, resolve_direct_track,
    resolve_search_results,
};
use crate::components::select_menu::music_search::format_duration;
use crate::get_state;
use crate::utils::access_control::ensure_music_channel_for_message;
use crate::utils::discord_embed::{info_embed, success_embed, warning_embed};
use crate::utils::music_queue::MusicQueue;

pub async fn run(ctx: &Context, message: &Message, query: &str) -> anyhow::Result<()> {
    if !ensure_music_channel_for_message(ctx, message).await? {
        return Ok(());
    }

    let guild_id = message
        .guild_id
        .context("Message command not used in guild")?;
    let state = get_state(ctx).await?;
    let (queue, voice_channel_id) =
        prepare_playback(ctx, guild_id, message.author.id, message.channel_id).await?;

    if query.starts_with("http://") || query.starts_with("https://") {
        if let Err(err) = MusicQueue::connect(ctx, voice_channel_id).await {
            message
                .channel_id
                .send_message(
                    &ctx.http,
                    CreateMessage::new().embed(warning_embed(
                        "Voice Connect Failed",
                        format!("Cannot connect the bot to your voice channel.\nDetails: {err}"),
                    )),
                )
                .await?;
            return Ok(());
        }

        let item = resolve_direct_track(ctx, guild_id, query, &message.author.tag()).await?;
        if let Err(err) = enqueue_track(ctx, guild_id, &queue, item.clone()).await {
            message
                .channel_id
                .send_message(
                    &ctx.http,
                    CreateMessage::new().embed(warning_embed(
                        "Playback Failed",
                        format!("Failed to start Lavalink playback.\nDetails: {err}"),
                    )),
                )
                .await?;
            return Ok(());
        }

        message
            .channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new().embed(success_embed(
                    "Song Added",
                    format!("**{}** ({})", item.title, format_duration(item.duration_ms)),
                )),
            )
            .await?;

        return Ok(());
    }

    let songs = resolve_search_results(ctx, guild_id, query, &message.author.tag()).await?;

    if songs.is_empty() {
        message
            .channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new().embed(warning_embed("No Results", "No search results found")),
            )
            .await?;
        return Ok(());
    }

    let cache_key = format!("search:{}:{}", guild_id.get(), message.author.id.get());
    state
        .search_cache
        .write()
        .await
        .store_results(cache_key.clone(), songs.clone());

    if let Err(err) = MusicQueue::connect(ctx, voice_channel_id).await {
        message
            .channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new().embed(warning_embed(
                    "Voice Connect Failed",
                    format!("Cannot connect the bot to your voice channel.\nDetails: {err}"),
                )),
            )
            .await?;
        return Ok(());
    }

    let options = build_search_options(&songs);

    let select = CreateSelectMenu::new(
        format!("music-search:{cache_key}"),
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Select a song to add to queue");

    message
        .channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .embed(info_embed(
                    "Search Results",
                    "Select one song from the menu below.",
                ))
                .components(vec![CreateActionRow::SelectMenu(select)]),
        )
        .await?;

    Ok(())
}
