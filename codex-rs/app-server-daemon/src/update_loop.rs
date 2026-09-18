//! Installs updates, validates the server restart, then transfers updater ownership.

#[cfg(unix)]
use std::process::Command as StdCommand;
use std::process::Stdio;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use codex_http_client::ClientRouteClass;
use codex_http_client::HttpClientFactory;
use codex_http_client::RouteAwareClientPool;
use futures::FutureExt;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
#[cfg(unix)]
use tokio::signal::unix::Signal;
#[cfg(unix)]
use tokio::signal::unix::SignalKind;
#[cfg(unix)]
use tokio::signal::unix::signal;
use tokio::time::sleep;

use crate::Daemon;
use crate::RestartIfRunningOutcome;
use crate::RestartMode;
use crate::UpdaterRefreshMode;
use crate::managed_install::ExecutableIdentity;
use crate::managed_install::executable_identity;
use crate::managed_install::resolved_managed_codex_bin;

const INITIAL_UPDATE_DELAY: Duration = Duration::from_secs(5 * 60);
const RESTART_RETRY_INTERVAL: Duration = Duration::from_millis(50);
const UPDATE_INTERVAL: Duration = Duration::from_secs(60 * 60);
#[cfg(unix)]
const INSTALL_URL: &str = "https://chatgpt.com/codex/install.sh";
#[cfg(windows)]
const INSTALL_URL: &str = "https://chatgpt.com/codex/install.ps1";

pub(crate) async fn run(http_client_factory: HttpClientFactory) -> Result<()> {
    #[cfg(unix)]
    let mut terminate =
        signal(SignalKind::terminate()).context("failed to install updater shutdown handler")?;
    #[cfg(windows)]
    let updater = {
        let daemon = Daemon::from_environment()?;
        crate::backend::pid_update_loop_backend(
            daemon.backend_paths(&daemon.load_settings().await?),
        )
    };
    #[cfg(windows)]
    updater.wait_for_ownership().await?;
    #[cfg(windows)]
    let mut terminate = Signal;
    #[cfg(windows)]
    let _installer_job = crate::backend::windows::updater_job()?;
    let running_updater_identity = current_updater_identity().await?;
    #[cfg(windows)]
    updater.mark_ready().await?;
    let http = RouteAwareClientPool::new_without_request_logging(
        http_client_factory,
        ClientRouteClass::Other,
    );
    if sleep_or_terminate(INITIAL_UPDATE_DELAY, &mut terminate).await {
        return Ok(());
    }
    loop {
        // Failed successor cleanup leaves its PID published. The predecessor
        // must stop instead of installing again without ownership.
        #[cfg(windows)]
        updater.wait_for_ownership().await?;
        match update_once(&http, &running_updater_identity, &mut terminate).await {
            Ok(UpdateLoopControl::Continue) | Err(_) => {}
            Ok(UpdateLoopControl::Stop) => return Ok(()),
        }
        if sleep_or_terminate(UPDATE_INTERVAL, &mut terminate).await {
            return Ok(());
        }
    }
}

async fn sleep_or_terminate(duration: Duration, terminate: &mut Signal) -> bool {
    tokio::select! {
        _ = sleep(duration) => false,
        _ = terminate.recv() => true,
    }
}

enum UpdateLoopControl {
    Continue,
    Stop,
}

async fn update_once(
    http: &RouteAwareClientPool,
    running_updater_identity: &ExecutableIdentity,
    terminate: &mut Signal,
) -> Result<UpdateLoopControl> {
    #[cfg(unix)]
    install_latest_standalone(http).await?;
    #[cfg(windows)]
    tokio::select! {
        result = install_latest_standalone(http) => result?,
        _ = terminate.recv() => return Ok(UpdateLoopControl::Stop),
    }

    let daemon = Daemon::from_environment()?;
    let managed_codex_bin = resolved_managed_codex_bin(&daemon.managed_codex_bin).await?;
    let managed_identity = executable_identity(&managed_codex_bin).await?;
    let (restart_mode, updater_refresh_mode) =
        update_modes_for_identities(running_updater_identity, &managed_identity);

    loop {
        if terminate.recv().now_or_never().flatten().is_some() {
            return Ok(UpdateLoopControl::Stop);
        }
        match daemon
            .try_restart_if_running(restart_mode, updater_refresh_mode, &managed_codex_bin)
            .await?
        {
            RestartIfRunningOutcome::Busy => {
                if sleep_or_terminate(RESTART_RETRY_INTERVAL, terminate).await {
                    return Ok(UpdateLoopControl::Stop);
                }
            }
            RestartIfRunningOutcome::Restarted => {
                #[cfg(windows)]
                if updater_refresh_mode == UpdaterRefreshMode::ReexecIfManagedBinaryChanged {
                    return Ok(UpdateLoopControl::Stop);
                }
                return Ok(UpdateLoopControl::Continue);
            }
            RestartIfRunningOutcome::NotRunning
            | RestartIfRunningOutcome::NotReady
            | RestartIfRunningOutcome::AlreadyCurrent => return Ok(UpdateLoopControl::Continue),
        }
    }
}

async fn current_updater_identity() -> Result<ExecutableIdentity> {
    let current_exe =
        std::env::current_exe().context("failed to resolve current updater executable")?;
    executable_identity(&current_exe).await
}

fn update_modes_for_identities(
    running_updater_identity: &ExecutableIdentity,
    managed_identity: &ExecutableIdentity,
) -> (RestartMode, UpdaterRefreshMode) {
    if running_updater_identity == managed_identity {
        (RestartMode::IfVersionChanged, UpdaterRefreshMode::None)
    } else {
        (
            RestartMode::Always,
            UpdaterRefreshMode::ReexecIfManagedBinaryChanged,
        )
    }
}

#[cfg(unix)]
pub(crate) fn reexec_managed_updater(managed_codex_bin: &std::path::Path) -> Result<()> {
    let err = StdCommand::new(managed_codex_bin)
        .args(["app-server", "daemon", "pid-update-loop"])
        .exec();
    Err(err).with_context(|| {
        format!(
            "failed to replace updater with managed Codex binary {}",
            managed_codex_bin.display()
        )
    })
}

async fn install_latest_standalone(http: &impl InstallerHttp) -> Result<()> {
    let script = fetch_installer_script(http).await?;

    #[cfg(unix)]
    let mut command = {
        let mut command = Command::new("/bin/sh");
        command.arg("-s");
        command
    };
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("powershell.exe");
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
        command.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", "try { Invoke-Expression ([Console]::In.ReadToEnd()) } catch { Write-Error $_; exit 1 }"])
            .env("CODEX_NON_INTERACTIVE", "1")
            .kill_on_drop(true);
        command
    };
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to invoke standalone Codex updater")?;
    let mut stdin = child
        .stdin
        .take()
        .context("standalone Codex updater stdin was unavailable")?;
    stdin
        .write_all(&script)
        .await
        .context("failed to pass standalone Codex updater to shell")?;
    drop(stdin);
    let status = child
        .wait()
        .await
        .context("failed to wait for standalone Codex updater")?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("standalone Codex updater exited with status {status}")
    }
}

async fn fetch_installer_script(http: &impl InstallerHttp) -> Result<Vec<u8>> {
    match http.get(INSTALL_URL).await? {
        InstallerResponse::Success(body) => Ok(body),
        InstallerResponse::Unsuccessful { status } => {
            anyhow::bail!("standalone Codex updater request failed with status {status}")
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum InstallerResponse {
    Success(Vec<u8>),
    Unsuccessful { status: u16 },
}

/// HTTP boundary used to download the standalone installer.
///
/// Implementations must issue a GET for the supplied URL, return exact response bytes for a
/// successful status, and report a non-success status without buffering its response body.
trait InstallerHttp: Send + Sync {
    fn get<'a>(
        &'a self,
        url: &'a str,
    ) -> impl std::future::Future<Output = Result<InstallerResponse>> + Send + 'a;
}

impl InstallerHttp for RouteAwareClientPool {
    async fn get(&self, url: &str) -> Result<InstallerResponse> {
        let response = RouteAwareClientPool::get(self, url)
            .send()
            .await
            .context("failed to fetch standalone Codex updater")?;
        if !response.status().is_success() {
            return Ok(InstallerResponse::Unsuccessful {
                status: response.status().as_u16(),
            });
        }
        let body = response
            .bytes()
            .await
            .context("failed to read standalone Codex updater")?
            .to_vec();
        Ok(InstallerResponse::Success(body))
    }
}

#[cfg(test)]
#[path = "update_loop_tests.rs"]
mod tests;

#[cfg(windows)]
struct Signal;

#[cfg(windows)]
impl Signal {
    async fn recv(&mut self) -> Option<()> {
        // An unreadable control path must stop the updater rather than disable shutdown.
        let _ = codex_app_server_transport::daemon_shutdown_signal().await;
        Some(())
    }
}
