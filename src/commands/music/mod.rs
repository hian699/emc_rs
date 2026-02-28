pub mod messagecommand_play;
pub mod messagecommand_skip;
pub mod messagecommand_stop;
pub mod slashcommand_play;
pub mod slashcommand_skip;
pub mod slashcommand_stop;

use serenity::all::CreateCommand;

pub fn register() -> Vec<CreateCommand> {
    vec![
        slashcommand_play::register(),
        slashcommand_skip::register(),
        slashcommand_stop::register(),
    ]
}
