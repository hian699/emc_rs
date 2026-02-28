pub mod messagecommand_eval;
pub mod messagecommand_reload;
pub mod slashcommand_eval;
pub mod slashcommand_reload;

use serenity::all::CreateCommand;

pub fn register() -> Vec<CreateCommand> {
    vec![
        slashcommand_reload::register(),
        slashcommand_eval::register(),
    ]
}
