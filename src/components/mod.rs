pub mod button;
pub mod select_menu;

use anyhow::anyhow;
use serenity::all::ComponentInteraction;
use serenity::client::Context;

pub async fn dispatch_component(
    ctx: &Context,
    interaction: &ComponentInteraction,
) -> anyhow::Result<()> {
    let custom_id = interaction.data.custom_id.as_str();
    match custom_id {
        "music-skip" => button::music_skip::run(ctx, interaction).await,
        "music-stop" => button::music_stop::run(ctx, interaction).await,
        "music-clear" => button::music_clear::run(ctx, interaction).await,
        "private-voice-invite" => select_menu::private_voice_invite::run(ctx, interaction).await,
        _ if custom_id.starts_with("music-search:") => {
            select_menu::music_search::run(ctx, interaction).await
        }
        _ => Err(anyhow!("Unknown component custom id")),
    }
}
