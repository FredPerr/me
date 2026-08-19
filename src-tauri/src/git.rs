use git2::Repository;
use std::path::Path;

#[tauri::command]
pub async fn get_git_remote_url(path: String) -> Result<Option<String>, String> {
    let repo_path = Path::new(&path);

    if !is_repo_root(repo_path) {
        return Ok(None);
    }

    let repository = match Repository::open(repo_path) {
        Ok(repo) => repo,
        Err(_) => return Ok(None),
    };

    let remote = match repository.find_remote("origin") {
        Ok(remote) => remote,
        Err(_) => return Ok(None),
    };

    let url = remote.url().map(|u| normalize_remote_url(u));
    Ok(url)
}

fn is_repo_root(path: &Path) -> bool {
    path.join(".git").exists()
}

fn normalize_remote_url(url: &str) -> String {
    if url.starts_with("git@") {
        let without_prefix = url.strip_prefix("git@").unwrap_or(url);
        let normalized = without_prefix.replace(':', "/");
        let without_suffix = normalized.strip_suffix(".git").unwrap_or(&normalized);
        format!("https://{}", without_suffix)
    } else if url.ends_with(".git") {
        url.strip_suffix(".git").unwrap_or(url).to_string()
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod normalize_remote_url {
        use super::*;

        #[test]
        fn converts_ssh_url_to_https() {
            let ssh_url = "git@github.com:user/repo.git";

            let result = normalize_remote_url(ssh_url);

            assert_eq!(result, "https://github.com/user/repo");
        }

        #[test]
        fn converts_ssh_url_without_git_suffix() {
            let ssh_url = "git@github.com:user/repo";

            let result = normalize_remote_url(ssh_url);

            assert_eq!(result, "https://github.com/user/repo");
        }

        #[test]
        fn strips_git_suffix_from_https_url() {
            let https_url = "https://github.com/user/repo.git";

            let result = normalize_remote_url(https_url);

            assert_eq!(result, "https://github.com/user/repo");
        }

        #[test]
        fn leaves_clean_https_url_unchanged() {
            let https_url = "https://github.com/user/repo";

            let result = normalize_remote_url(https_url);

            assert_eq!(result, "https://github.com/user/repo");
        }

        #[test]
        fn handles_gitlab_ssh_url() {
            let ssh_url = "git@gitlab.com:org/project.git";

            let result = normalize_remote_url(ssh_url);

            assert_eq!(result, "https://gitlab.com/org/project");
        }

        #[test]
        fn handles_nested_group_paths() {
            let ssh_url = "git@github.com:org/team/repo.git";

            let result = normalize_remote_url(ssh_url);

            assert_eq!(result, "https://github.com/org/team/repo");
        }
    }
}
