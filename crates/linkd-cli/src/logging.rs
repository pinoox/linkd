use std::sync::OnceLock;

use linkd_core::log_path;
use tracing_subscriber::EnvFilter;

static LOG_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

pub fn init() -> anyhow::Result<()> {
    linkd_core::ensure_home()?;

    let file_appender = tracing_appender::rolling::never(linkd_core::linkd_home(), "linkd.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let _ = LOG_GUARD.set(guard);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("linkd=info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    let _ = log_path();
    Ok(())
}
