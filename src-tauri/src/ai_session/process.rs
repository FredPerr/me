//! Low-level process lifecycle: spawn with piped stdio, stream output, watch
//! for exit, and kill by process group. Platform-specific concerns (process
//! groups, signalling) are isolated here behind small helpers.
//!
//! Output and terminal status are delivered two ways at once: to the frontend
//! over Tauri events (the frontend still consumes them), and to an optional
//! in-process [`AiSessionSink`] an in-Rust caller (the agent-run composition
//! adapter) passes in. The two are independent, so adding the sink did not
//! change the existing frontend behaviour.

use crate::ai_session::registry::{RunningSession, SessionRegistry, SessionStatus};
use crate::ai_session::{
    adapter::SpawnSpec, OutputEvent, OutputStream, SpawnedSession, StatusEvent, EVENT_OUTPUT,
    EVENT_STATUS,
};
use std::process::Stdio;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

/// An in-process observer of one session's output and terminal status, in
/// addition to the Tauri events. The agent-run adapter listens on this to drive
/// `workspace` without the frontend being involved.
#[derive(Debug, Clone)]
pub enum AiSessionEvent {
    /// One line of output from the given stream.
    Line { stream: OutputStream, line: String },
    /// The process reached a terminal state.
    Ended {
        status: SessionStatus,
        exit_code: Option<i32>,
    },
}

/// The channel the in-process events are sent on.
pub type AiSessionSink = mpsc::UnboundedSender<AiSessionEvent>;

/// Spawns the process described by `spec`, registers it, and starts the
/// streaming + exit-watching background tasks. Returns the session descriptor.
pub async fn spawn(
    app: AppHandle,
    registry: &State<'_, SessionRegistry>,
    adapter_id: String,
    spec: SpawnSpec,
) -> Result<SpawnedSession, String> {
    spawn_with_sink(app, registry, adapter_id, spec, None).await
}

/// Like [`spawn`], but also forwards every output line and the terminal status
/// to `sink` when one is given. The Tauri events fire exactly as before either
/// way, so the frontend is unaffected.
pub async fn spawn_with_sink(
    app: AppHandle,
    registry: &State<'_, SessionRegistry>,
    adapter_id: String,
    spec: SpawnSpec,
    sink: Option<AiSessionSink>,
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

    stream_output(&app, &session_id, &mut child, sink.clone());

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

    watch_for_exit(
        app,
        child,
        session_id.clone(),
        adapter_id.clone(),
        pid,
        sink,
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
fn stream_output(
    app: &AppHandle,
    session_id: &str,
    child: &mut Child,
    sink: Option<AiSessionSink>,
) {
    if let Some(stdout) = child.stdout.take() {
        spawn_line_reader(
            app.clone(),
            session_id.to_string(),
            stdout,
            OutputStream::Stdout,
            sink.clone(),
        );
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_line_reader(
            app.clone(),
            session_id.to_string(),
            stderr,
            OutputStream::Stderr,
            sink,
        );
    }
}

fn spawn_line_reader<R>(
    app: AppHandle,
    session_id: String,
    reader: R,
    stream: OutputStream,
    sink: Option<AiSessionSink>,
) where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    if let Some(sink) = &sink {
                        let _ = sink.send(AiSessionEvent::Line {
                            stream,
                            line: line.clone(),
                        });
                    }
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
/// from the registry and emits the terminal status. The registry keeps the pid
/// for the whole lifetime so `kill` works right up until the process actually
/// exits.
fn watch_for_exit(
    app: AppHandle,
    mut child: Child,
    session_id: String,
    adapter_id: String,
    pid: Option<u32>,
    sink: Option<AiSessionSink>,
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

        if let Some(sink) = &sink {
            let _ = sink.send(AiSessionEvent::Ended { status, exit_code });
        }

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
    });
}

fn emit_status(app: &AppHandle, event: StatusEvent) {
    let _ = app.emit(EVENT_STATUS, event);
}

/// Terminates a tracked session by killing its process group, callable from
/// plain Rust (the agent-run adapter) without a Tauri command wrapper. Flags
/// the kill in the registry first so the exit is reported as `killed`.
pub fn kill_session(registry: &SessionRegistry, session_id: &str) -> Result<(), String> {
    let pid = registry
        .mark_kill_requested(session_id)
        .or_else(|| registry.pid_of(session_id))
        .ok_or_else(|| format!("No running session with id '{}'", session_id))?;

    kill_process_group(pid)
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
