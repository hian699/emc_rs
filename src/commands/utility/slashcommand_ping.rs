use serenity::all::{
    CommandInteraction, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::client::Context;

pub fn register() -> CreateCommand {
    CreateCommand::new("ping").description("Check bot latency")
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content("Pong!"),
            ),
        )
        .await?;
    Ok(())
}
