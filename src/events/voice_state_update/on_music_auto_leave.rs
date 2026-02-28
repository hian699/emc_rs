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
        queue.write().await.destroy(ctx).await?;
        state.music_manager.delete_queue(guild_id).await;
    }

    Ok(())
}
