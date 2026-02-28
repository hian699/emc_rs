pub mod slashcommand_random;

use serenity::all::CreateCommand;

pub fn register() -> Vec<CreateCommand> {
    vec![slashcommand_random::register()]
}
