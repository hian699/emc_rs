use serenity::all::Message;
use serenity::client::Context;

use crate::utils::access_control::ensure_developer_for_message;

pub async fn run(ctx: &Context, message: &Message) -> anyhow::Result<()> {
    if !ensure_developer_for_message(ctx, message).await? {
        return Ok(());
    }

    message.channel_id.say(&ctx.http, "Reload finished").await?;
    Ok(())
}
