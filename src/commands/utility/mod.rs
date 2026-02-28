pub mod messagecommand_ping;
pub mod slashcommand_ping;

use serenity::all::CreateCommand;

pub fn register() -> Vec<CreateCommand> {
    vec![slashcommand_ping::register()]
}
