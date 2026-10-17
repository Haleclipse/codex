//! @cometix: keep a stopped, CLI-seeded daemon on the same build as the CLI.
//!
//! Upstream npm installs seed a stable daemon that the public updater keeps current.
//! This fork's versions carry a prerelease suffix and the public channel is disabled,
//! so a seeded daemon stays pinned to the CLI build that copied it. When that daemon
//! is stopped and reports a different version than the running CLI, re-seed it from
//! this CLI's package, then drop superseded local copies. A running daemon is left
//! alone because replacing it would interrupt work; `/daemon` handles that case.

use std::path::Path;

use anyhow::Result;

use crate::Daemon;
use crate::client;
use crate::ensure_supported_platform;
use crate::managed_install;
use crate::prepare_install;

const DEDICATED_PACKAGE_ROOT: &str = "packages/app-server-daemon";
const LOCAL_RELEASE_PREFIX: &str = "local-";

/// Returns true when the daemon package was replaced with this CLI's package.
pub async fn sync_stopped_daemon_with_cli(cli_version: &str) -> Result<bool> {
    ensure_supported_platform()?;
    let mut daemon = Daemon::from_environment()?;
    // The TUI keeps its screen up while this runs; route progress to the log.
    daemon.log_diagnostics = true;
    let selected = daemon.current_installation()?;
    if !selected.managed_codex_bin.is_file() {
        // Nothing installed yet; the next start seeds it from this CLI.
        return Ok(false);
    }
    if client::probe(&daemon.socket_path).await.is_ok() {
        return Ok(false);
    }
    let settings = daemon.load_settings().await?;
    if selected
        .running_backend_instance(&settings)
        .await?
        .is_some()
    {
        return Ok(false);
    }
    // `--version` is cheap; only hash and copy the package when the build differs.
    let installed = managed_install::managed_codex_version(&selected.managed_codex_bin)
        .await
        .ok();
    if installed.as_deref() == Some(cli_version) {
        return Ok(false);
    }

    daemon.diagnostic(format_args!(
        "Updating the background server to Codex {cli_version}..."
    ));
    let updated = prepare_install::update_from_cli(|request| Ok(!request.restart_required))
        .await?
        .is_some();
    if updated && let Some(home) = daemon.settings_file.parent().and_then(Path::parent) {
        prune_superseded_local_releases(&home.join(DEDICATED_PACKAGE_ROOT));
    }
    Ok(updated)
}

/// Remove CLI-seeded releases other than the selected one. Best effort: a release
/// that cannot be removed now is retried on the next sync.
fn prune_superseded_local_releases(package_root: &Path) {
    let Ok(current) = package_root.join("current").canonicalize() else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(package_root.join("releases")) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_local_release = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(LOCAL_RELEASE_PREFIX));
        let is_current = path.canonicalize().is_ok_and(|release| release == current);
        if is_local_release
            && !is_current
            && entry.file_type().is_ok_and(|kind| kind.is_dir())
            && let Err(err) = std::fs::remove_dir_all(&path)
        {
            tracing::warn!(
                "failed to remove superseded daemon release {}: {err}",
                path.display()
            );
        }
    }
}
