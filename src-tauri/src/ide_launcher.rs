use crate::binary_resolver;
use std::process::Command;

/// Resolves an IDE command to an absolute binary path and launches it with the
/// given project path.
///
/// GUI apps launched from Finder do not inherit the user's interactive shell
/// PATH, so a bare command name like `kiro` cannot be found. This resolves the
/// command to an absolute path first (via [`binary_resolver`]), then spawns the
/// binary directly without a shell (avoiding shell injection on the project
/// path).
pub fn launch(command: &str, path: &str, shell: Option<&str>) -> Result<(), String> {
    let binary = binary_resolver::resolve(command, shell)
        .ok_or_else(|| format!("Could not locate IDE command '{}' on this system", command))?;

    Command::new(&binary)
        .arg(path)
        .spawn()
        .map_err(|error| format!("Failed to launch '{}': {}", binary.display(), error))?;

    Ok(())
}
