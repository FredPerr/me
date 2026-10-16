use std::path::Path;
use std::process::Command;

use crate::git::domain::error::GitError;
use crate::git::domain::ports::RemoteAuth;

/// Runs a `git` subcommand in `folder`. Prompts are disabled so a missing
/// credential fails instead of hanging; `auth` is injected per call and never
/// written to git config.
pub fn run(
    folder: &Path,
    operation: &str,
    args: &[&str],
    auth: Option<&RemoteAuth>,
) -> Result<GitOutput, GitError> {
    let mut command = Command::new("git");
    if let Some(auth) = auth {
        command.arg("-c").arg(format!(
            "http.{}.extraheader=Authorization: {}",
            auth.scope, auth.authorization
        ));
    }
    let output = command
        .args(args)
        .current_dir(folder)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|error| GitError::io(format!("run git {operation}"), error))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        Ok(GitOutput { stdout, stderr })
    } else {
        Err(classify_failure(operation, stderr))
    }
}

pub struct GitOutput {
    pub stdout: String,
    /// git reports progress here even when it succeeds.
    pub stderr: String,
}

fn classify_failure(operation: &str, stderr: String) -> GitError {
    let lowered = stderr.to_lowercase();
    let matches_any = |needles: &[&str]| needles.iter().any(|needle| lowered.contains(needle));
    let operation = operation.to_string();
    let output = stderr;
    if matches_any(&[
        "authentication failed",
        "could not read username",
        "terminal prompts disabled",
        "permission denied (publickey)",
    ]) {
        GitError::AuthenticationFailed { operation, output }
    } else if matches_any(&[
        "could not resolve host",
        "unable to access",
        "could not connect",
        "connection timed out",
    ]) {
        GitError::Network { operation, output }
    } else if matches_any(&["not possible to fast-forward", "non-fast-forward"]) {
        GitError::NotFastForward { operation, output }
    } else {
        GitError::CommandFailed { operation, output }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_authentication_failures() {
        let error = classify_failure(
            "push",
            "fatal: could not read Username for 'https://github.com': terminal prompts disabled"
                .into(),
        );
        assert!(matches!(error, GitError::AuthenticationFailed { .. }));
    }

    #[test]
    fn recognizes_network_failures() {
        let error = classify_failure(
            "fetch",
            "fatal: unable to access 'https://x/': Could not resolve host: x".into(),
        );
        assert!(matches!(error, GitError::Network { .. }));
    }

    #[test]
    fn keeps_the_output_of_unrecognized_failures() {
        let error = classify_failure("worktree add", "fatal: something odd".into());
        assert_eq!(
            error,
            GitError::CommandFailed {
                operation: "worktree add".into(),
                output: "fatal: something odd".into()
            }
        );
    }
}
