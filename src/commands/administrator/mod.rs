pub mod slashcommand_config_set;
pub mod slashcommand_config_show;
pub mod slashcommand_deletemessage;
pub mod slashcommand_security_lockdown;
pub mod slashcommand_timeout;

use serenity::all::CreateCommand;

pub fn register() -> Vec<CreateCommand> {
    vec![
        slashcommand_timeout::register(),
        slashcommand_deletemessage::register(),
        slashcommand_security_lockdown::register(),
        slashcommand_config_set::register(),
        slashcommand_config_show::register(),
    ]
}
