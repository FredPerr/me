use super::branch_name::BranchName;

const GITHUB_HOST: &str = "github.com";
const SSH_PREFIX: &str = "git@";
const GIT_SUFFIX: &str = ".git";

/// The URL of a repository's `origin` remote, as configured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteUrl(String);

impl RemoteUrl {
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The web address of the repository: SSH remotes become https and the
    /// `.git` suffix is dropped.
    pub fn web_url(&self) -> String {
        let url = self.0.as_str();
        if let Some(without_prefix) = url.strip_prefix(SSH_PREFIX) {
            let with_slash = without_prefix.replacen(':', "/", 1);
            let trimmed = with_slash.strip_suffix(GIT_SUFFIX).unwrap_or(&with_slash);
            return format!("https://{trimmed}");
        }
        url.strip_suffix(GIT_SUFFIX).unwrap_or(url).to_string()
    }

    pub fn is_github(&self) -> bool {
        self.web_url().contains(GITHUB_HOST)
    }

    /// GitHub's page for opening a pull request from `branch`; `None` for
    /// other hosts.
    pub fn github_compare_url(&self, branch: &BranchName) -> Option<String> {
        self.is_github()
            .then(|| format!("{}/compare/{branch}?expand=1", self.web_url()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn web_url(raw: &str) -> String {
        RemoteUrl::new(raw).web_url()
    }

    #[test]
    fn converts_ssh_url_to_https() {
        assert_eq!(
            web_url("git@github.com:user/repo.git"),
            "https://github.com/user/repo"
        );
    }

    #[test]
    fn converts_ssh_url_without_git_suffix() {
        assert_eq!(
            web_url("git@github.com:user/repo"),
            "https://github.com/user/repo"
        );
    }

    #[test]
    fn strips_git_suffix_from_https_url() {
        assert_eq!(
            web_url("https://github.com/user/repo.git"),
            "https://github.com/user/repo"
        );
    }

    #[test]
    fn leaves_clean_https_url_unchanged() {
        assert_eq!(
            web_url("https://github.com/user/repo"),
            "https://github.com/user/repo"
        );
    }

    #[test]
    fn handles_gitlab_ssh_url() {
        assert_eq!(
            web_url("git@gitlab.com:org/project.git"),
            "https://gitlab.com/org/project"
        );
    }

    #[test]
    fn handles_nested_group_paths() {
        assert_eq!(
            web_url("git@github.com:org/team/repo.git"),
            "https://github.com/org/team/repo"
        );
    }

    #[test]
    fn builds_the_github_compare_url_for_a_branch() {
        let branch = BranchName::parse("feature/login").unwrap();
        assert_eq!(
            RemoteUrl::new("git@github.com:user/repo.git").github_compare_url(&branch),
            Some("https://github.com/user/repo/compare/feature/login?expand=1".to_string())
        );
        assert_eq!(
            RemoteUrl::new("https://bitbucket.org/team/repo.git").github_compare_url(&branch),
            None
        );
    }

    #[test]
    fn recognizes_github_remotes() {
        assert!(RemoteUrl::new("git@github.com:user/repo.git").is_github());
        assert!(!RemoteUrl::new("https://bitbucket.org/team/repo.git").is_github());
    }
}
