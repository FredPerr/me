//! The Kiro permission floor on the real file system, ported from the
//! frontend's `ensurePermissionFloor`.
//!
//! Kiro resolves permissions as `deny > ask > allow`, and a `deny` rule cannot
//! be overridden, so this `deny` floor keeps a trusted (`--trust-all-tools`)
//! run on a short leash: destructive shell commands and secret-file access stay
//! blocked whatever the trust flag. The file is written once per working
//! directory and never overwritten, so a user's own edits survive. Any write
//! failure is swallowed so it never blocks a run.

use std::path::Path;

use crate::workspace::domain::ports::PermissionFloor;

const PERMISSIONS_RELATIVE_DIR: &str = ".kiro/settings";
const PERMISSIONS_FILE: &str = "permissions.yaml";

/// The seeded policy, kept identical to the frontend's so a working directory
/// already seeded by the old frontend is left untouched.
const DEFAULT_PERMISSIONS_YAML: &str = r#"# Managed by the app: safety floor for trusted (--trust-all-tools) Kiro runs.
# Kiro resolves permissions as deny > ask > allow, and deny always wins, so
# these rules block dangerous actions even when all tools are trusted.
# Edit freely — this file is only created when absent, never overwritten.
rules:
  - capability: shell
    effect: deny
    match:
      - "rm -rf /*"
      - "rm -rf ~*"
      - "rm -rf .*"
      - "sudo *"
      - "chmod 777*"
      - "chmod -R 777*"
      - "mkfs*"
      - "dd * of=/dev/*"
      - ":(){ :|:& };:"
      - "shutdown*"
      - "reboot*"
      - "git push --force*"
      - "git push -f*"
      - "git push * --force*"
      - "* --no-verify"
  - capability: filesystem
    effect: deny
    match:
      - "**/.env"
      - "**/.env.*"
      - "**/*.pem"
      - "**/*.key"
      - "**/id_rsa*"
      - "**/id_ed25519*"
      - "**/.ssh/**"
      - "**/.aws/credentials"
      - "**/.npmrc"
      - "**/.netrc"
"#;

pub struct FileSystemPermissionFloor;

impl FileSystemPermissionFloor {
    /// Writes the policy when absent, returning whether it wrote a file. Lets
    /// the tests assert the once-only behaviour without reading the file back.
    fn ensure(working_directory: &str) -> std::io::Result<bool> {
        let settings_dir = Path::new(working_directory).join(PERMISSIONS_RELATIVE_DIR);
        let file_path = settings_dir.join(PERMISSIONS_FILE);
        if file_path.exists() {
            return Ok(false);
        }
        std::fs::create_dir_all(&settings_dir)?;
        std::fs::write(&file_path, DEFAULT_PERMISSIONS_YAML)?;
        Ok(true)
    }
}

impl PermissionFloor for FileSystemPermissionFloor {
    fn seed(&self, working_directory: &str) {
        if let Err(error) = Self::ensure(working_directory) {
            eprintln!("[workspace] permission floor in '{working_directory}' not seeded: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn working_directory() -> TempDir {
        TempDir::new().unwrap()
    }

    fn policy_path(directory: &TempDir) -> std::path::PathBuf {
        directory
            .path()
            .join(PERMISSIONS_RELATIVE_DIR)
            .join(PERMISSIONS_FILE)
    }

    #[test]
    fn writes_the_policy_when_absent() {
        let directory = working_directory();

        let wrote = FileSystemPermissionFloor::ensure(directory.path().to_str().unwrap()).unwrap();

        assert!(wrote);
        let written = std::fs::read_to_string(policy_path(&directory)).unwrap();
        assert_eq!(written, DEFAULT_PERMISSIONS_YAML);
    }

    #[test]
    fn leaves_an_existing_policy_untouched() {
        let directory = working_directory();
        let custom = "rules: []\n";
        std::fs::create_dir_all(policy_path(&directory).parent().unwrap()).unwrap();
        std::fs::write(policy_path(&directory), custom).unwrap();

        let wrote = FileSystemPermissionFloor::ensure(directory.path().to_str().unwrap()).unwrap();

        assert!(!wrote);
        assert_eq!(
            std::fs::read_to_string(policy_path(&directory)).unwrap(),
            custom
        );
    }

    #[test]
    fn seed_swallows_a_write_failure() {
        // A path whose parent is a file cannot become a directory; seed must
        // not panic or propagate.
        let directory = working_directory();
        let blocker = directory.path().join("blocker");
        std::fs::write(&blocker, "x").unwrap();

        FileSystemPermissionFloor.seed(blocker.to_str().unwrap());
    }
}
