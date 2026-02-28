use rand::Rng;
use serenity::all::{
    CommandInteraction, CommandOptionType, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::client::Context;

use crate::commands::get_i64_option;

pub fn register() -> CreateCommand {
    CreateCommand::new("random")
        .description("Generate random number in range")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "min", "Minimum value")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "max", "Maximum value")
                .required(true),
        )
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let min = get_i64_option(command, "min").unwrap_or(0);
    let max = get_i64_option(command, "max").unwrap_or(100);

    let content = if min > max {
        "min must be <= max".to_string()
    } else {
        let result = rand::thread_rng().gen_range(min..=max);
        format!("Random result: {result}")
    };

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(content),
            ),
        )
        .await?;
    Ok(())
}
