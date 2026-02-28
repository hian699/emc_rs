use serenity::all::Message;
use serenity::client::Context;

pub async fn run(ctx: &Context, message: &Message) -> anyhow::Result<()> {
    message.channel_id.say(&ctx.http, "Pong!").await?;
    Ok(())
}
