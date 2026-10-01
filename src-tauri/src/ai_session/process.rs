//! Low-level process lifecycle: spawn with piped stdio, stream output, watch
//! for exit, and kill by process group. Platform-specific concerns (process
//! groups, signalling) are isolated here behind small helpers.

use crate::ai_session::mcp_sync::{self, McpSync};
use crate::ai_session::registry::{RunningSession, SessionRegistry, SessionStatus};
use crate::ai_session::{
    adapter::SpawnSpec, OutputEvent, OutputStream, SpawnedSession, StatusEvent, EVENT_OUTPUT,
    EVENT_STATUS,
};
use std::process::Stdio;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

/// Spawns the process described by `spec`, registers it, and starts the
/// streaming + exit-watching background tasks. Returns the session descriptor.
pub async fn spawn(
    app: AppHandle,
    registry: &State<'_, SessionRegistry>,
    adapter_id: String,
    spec: SpawnSpec,
) -> Result<SpawnedSession, String> {
    let session_id = uuid::Uuid::new_v4().to_string();

    let mut command = Command::new(&spec.binary);
    command
        .args(&spec.arguments)
        .current_dir(&spec.working_directory)
        .stdin(if spec.stdin_payload.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(false);

    for (key, value) in &spec.environment {
        command.env(key, value);
    }

    configure_process_group(&mut command);

    let mut child = command
        .spawn()
        .map_err(|error| format!("Failed to spawn '{}': {}", spec.binary.display(), error))?;

    let pid = child.id();

    // Write the prompt to stdin when the adapter delivers it that way, then
    // drop the handle to close stdin so the CLI stops reading.
    if let Some(payload) = &spec.stdin_payload {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(payload.as_bytes()).await;
            let _ = stdin.write_all(b"\n").await;
            let _ = stdin.shutdown().await;
        }
    }

    stream_output(&app, &session_id, &mut child);

    let mcp = McpSync::from_environment();

    // Register the session before announcing it is running so a kill issued the
    // instant the frontend sees the status always finds a tracked pid.
    registry.insert(RunningSession {
        session_id: session_id.clone(),
        pid,
        kill_requested: false,
    });

    emit_status(
        &app,
        StatusEvent {
            session_id: session_id.clone(),
            adapter_id: adapter_id.clone(),
            status: SessionStatus::Running,
            pid,
            exit_code: None,
            error: None,
        },
    );
    report_to_mcp(
        &mcp,
        &session_id,
        &adapter_id,
        SessionStatus::Running,
        &spec.working_directory,
        pid,
        None,
    );

    watch_for_exit(
        app,
        child,
        session_id.clone(),
        adapter_id.clone(),
        spec.working_directory.clone(),
        pid,
        mcp,
    );

    Ok(SpawnedSession {
        session_id,
        adapter_id,
        pid,
        working_directory: spec.working_directory,
    })
}

/// Spawns background tasks that read stdout and stderr line-by-line and emit an
/// [`OutputEvent`] per line. Handles are taken from the child here.
fn stream_output(app: &AppHandle, session_id: &str, child: &mut Child) {
    if let Some(stdout) = child.stdout.take() {
        spawn_line_reader(
            app.clone(),
            session_id.to_string(),
            stdout,
            OutputStream::Stdout,
        );
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_line_reader(
            app.clone(),
            session_id.to_string(),
            stderr,
            OutputStream::Stderr,
        );
    }
}

fn spawn_line_reader<R>(app: AppHandle, session_id: String, reader: R, stream: OutputStream)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let _ = app.emit(
                        EVENT_OUTPUT,
                        OutputEvent {
                            session_id: session_id.clone(),
                            stream,
                            line,
                        },
                    );
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }
    });
}

/// Spawns a task that owns the child, waits for it to exit, then removes it
/// from the registry, emits the terminal status, and reports it to the MCP
/// server. The registry keeps the pid for the whole lifetime so `kill` works
/// right up until the process actually exits.
fn watch_for_exit(
    app: AppHandle,
    mut child: Child,
    session_id: String,
    adapter_id: String,
    working_directory: String,
    pid: Option<u32>,
    mcp: Option<McpSync>,
) {
    tokio::spawn(async move {
        let wait_result = child.wait().await;

        // Remove the session now that it has exited; the removed entry carries
        // whether a kill was requested, which classifies the terminal state.
        let kill_requested = app
            .try_state::<SessionRegistry>()
            .and_then(|registry| registry.remove(&session_id))
            .map(|session| session.kill_requested)
            .unwrap_or(false);

        let (status, exit_code, error) = match wait_result {
            Ok(exit_status) => {
                let code = exit_status.code();
                if kill_requested {
                    (SessionStatus::Killed, code, None)
                } else if exit_status.success() {
                    (SessionStatus::Completed, code, None)
                } else {
                    (
                        SessionStatus::Failed,
                        code,
                        Some(format!("Process exited with status {}", exit_status)),
                    )
                }
            }
            Err(wait_error) => (
                SessionStatus::Failed,
                None,
                Some(format!("Failed to wait for process: {}", wait_error)),
            ),
        };

        emit_status(
            &app,
            StatusEvent {
                session_id: session_id.clone(),
                adapter_id: adapter_id.clone(),
                status,
                pid,
                exit_code,
                error,
            },
        );
        report_to_mcp(
            &mcp,
            &session_id,
            &adapter_id,
            status,
            &working_directory,
            None,
            exit_code,
        );
    });
}

fn emit_status(app: &AppHandle, event: StatusEvent) {
    let _ = app.emit(EVENT_STATUS, event);
}

fn report_to_mcp(
    mcp: &Option<McpSync>,
    session_id: &str,
    adapter_id: &str,
    status: SessionStatus,
    working_directory: &str,
    pid: Option<u32>,
    exit_code: Option<i32>,
) {
    if let Some(client) = mcp.clone() {
        let update = mcp_sync::build_update(
            session_id,
            adapter_id,
            status,
            working_directory,
            pid,
            exit_code,
        );
        tokio::spawn(async move {
            client.report(update).await;
        });
    }
}

// --- Platform-specific process group handling ------------------------------

/// Places the child in its own process group so the whole tree (the CLI plus
/// any helpers it spawns) can be terminated together.
#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    // A new session/process group whose group id equals the child's pid.
    command.process_group(0);
}

#[cfg(windows)]
fn configure_process_group(_command: &mut Command) {
    // On Windows we terminate the tree via `taskkill /T`, which does not require
    // a dedicated process group at spawn time.
}

/// Terminates the process group identified by `pid`.
///
/// On Unix the child leads its own group (see [`configure_process_group`]), so
/// the group id equals the pid; we send `SIGTERM` to the negative pid to reach
/// the whole group. On Windows we delegate to `taskkill /T /F`.
#[cfg(unix)]
pub fn kill_process_group(pid: u32) -> Result<(), String> {
    // Safety: killpg is a libc call with no memory effects; we check the result.
    let result = unsafe { libc::killpg(pid as libc::pid_t, libc::SIGTERM) };
    if result != 0 {
        let errno = std::io::Error::last_os_error();
        // Already-exited groups report ESRCH; treat that as success so kill is
        // idempotent.
        if errno.raw_os_error() == Some(libc::ESRCH) {
            return Ok(());
        }
        return Err(format!(
            "Failed to terminate process group {}: {}",
            pid, errno
        ));
    }
    Ok(())
}

#[cfg(windows)]
pub fn kill_process_group(pid: u32) -> Result<(), String> {
    use std::process::Command as StdCommand;

    let status = StdCommand::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .status()
        .map_err(|error| format!("Failed to invoke taskkill for pid {}: {}", pid, error))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "taskkill exited with status {} for pid {}",
            status, pid
        ))
    }
}
