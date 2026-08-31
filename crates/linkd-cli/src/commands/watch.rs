use linkd_daemon::DaemonService;
use linkd_registry::RegistryStore;

use crate::ui::watch_ui;

pub async fn run() -> anyhow::Result<()> {
    let store = RegistryStore::default();
    let service = DaemonService::new(store);

    tokio::select! {
        res = service.run_foreground() => res.map_err(|e| anyhow::anyhow!(e.to_string())),
        res = tokio::task::spawn_blocking(watch_ui::run_tui) => {
            res??;
            Ok(())
        }
    }
}
