use std::path::PathBuf;
use std::process::Command;

/// Resolves an IDE command to an absolute binary path and launches it with the
/// given project path.
///
/// GUI apps launched from Finder do not inherit the user's interactive shell
/// PATH, so a bare command name like `kiro` cannot be found. This resolves the
/// command to an absolute path first, then spawns the binary directly without a
/// shell (avoiding shell injection on the project path).
pub fn launch(command: &str, path: &str, shell: Option<&str>) -> Result<(), String> {
    let binary = resolve_binary(command, shell)
        .ok_or_else(|| format!("Could not locate IDE command '{}' on this system", command))?;

    Command::new(&binary)
        .arg(path)
        .spawn()
        .map_err(|error| format!("Failed to launch '{}': {}", binary.display(), error))?;

    Ok(())
}

/// Resolves a command to an absolute binary path.
///
/// Resolution order:
/// 1. Absolute path or path containing a separator, used as-is when it exists.
/// 2. Well-known bin directories (Homebrew, system, and editor CLI shims).
/// 3. A login shell lookup that loads the user's profile PATH.
fn resolve_binary(command: &str, shell: Option<&str>) -> Option<PathBuf> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }

    if is_explicit_path(trimmed) {
        let candidate = PathBuf::from(trimmed);
        return candidate.is_file().then_some(candidate);
    }

    if let Some(found) = search_known_directories(trimmed) {
        return Some(found);
    }

    resolve_via_login_shell(trimmed, shell)
}

fn is_explicit_path(command: &str) -> bool {
    command.contains('/')
}

fn search_known_directories(command: &str) -> Option<PathBuf> {
    known_bin_directories()
        .into_iter()
        .map(|directory| directory.join(command))
        .find(|candidate| candidate.is_file())
}

fn known_bin_directories() -> Vec<PathBuf> {
    let mut directories = vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ];

    if let Some(home) = dirs::home_dir() {
        directories.push(home.join(".local/bin"));
        directories.push(home.join("bin"));
    }

    directories
}

/// Falls back to the user's login shell to resolve the command using their real
/// profile PATH. This covers editors installed in non-standard locations.
fn resolve_via_login_shell(command: &str, shell: Option<&str>) -> Option<PathBuf> {
    let shell_path = shell
        .map(str::to_string)
        .or_else(|| std::env::var("SHELL").ok())
        .unwrap_or_else(|| "/bin/sh".to_string());

    let output = Command::new(&shell_path)
        .arg("-lc")
        .arg(format!("command -v {}", shell_quote(command)))
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if resolved.is_empty() {
        return None;
    }

    let candidate = PathBuf::from(resolved);
    candidate.is_file().then_some(candidate)
}

/// Wraps a value in single quotes for safe interpolation into a shell command,
/// escaping any embedded single quotes.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn treats_slash_containing_command_as_explicit_path() {
        assert!(is_explicit_path("/usr/local/bin/kiro"));
        assert!(is_explicit_path("./scripts/open"));
    }

    #[test]
    fn treats_bare_command_as_non_path() {
        assert!(!is_explicit_path("kiro"));
        assert!(!is_explicit_path("code"));
    }

    #[test]
    fn known_directories_include_homebrew_and_system() {
        let directories = known_bin_directories();

        assert!(directories.contains(&PathBuf::from("/opt/homebrew/bin")));
        assert!(directories.contains(&PathBuf::from("/usr/local/bin")));
        assert!(directories.contains(&PathBuf::from("/usr/bin")));
    }

    #[test]
    fn shell_quote_wraps_plain_value() {
        assert_eq!(shell_quote("kiro"), "'kiro'");
    }

    #[test]
    fn shell_quote_escapes_single_quotes() {
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
    }

    #[test]
    fn empty_command_resolves_to_none() {
        assert_eq!(resolve_binary("   ", None), None);
    }

    #[test]
    fn absolute_path_to_missing_file_resolves_to_none() {
        assert_eq!(resolve_binary("/definitely/not/here/kiro", None), None);
    }
}
