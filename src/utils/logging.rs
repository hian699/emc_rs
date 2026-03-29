use tracing_subscriber::EnvFilter;

const DEFAULT_LOG_FILTER: &str = "info,emc_rs=debug,lavalink_rs=info,serenity=warn,songbird=warn";

pub fn init_logging() -> anyhow::Result<()> {
    let env_filter =
        EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new(DEFAULT_LOG_FILTER))?;

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_names(true)
        .compact()
        .init();

    Ok(())
}
