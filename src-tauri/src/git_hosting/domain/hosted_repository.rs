use crate::git::domain::remote_url::RemoteUrl;

use super::host_kind::HostKind;

const URL_SCHEMES: [&str; 3] = ["https://", "http://", "ssh://"];
const SCHEME_SEPARATOR: &str = "://";
const GIT_SUFFIX: &str = ".git";

/// A repository on a supported Git host, read from a git remote. This is the
/// single rule deciding which Git host a remote belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedRepository {
    pub host: HostKind,
    pub owner: String,
    pub name: String,
}

/// Why a remote is not a repository on a supported Git host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedRemote {
    /// Neither an `scheme://` URL nor the SSH `host:path` form.
    UnreadableRemote,
    /// A host other than the supported ones, such as GitLab or a self-hosted server.
    UnsupportedHost,
    /// The path has no owner or no repository name.
    MissingRepository,
}

impl HostedRepository {
    pub fn from_remote(remote: &RemoteUrl) -> Result<Self, UnsupportedRemote> {
        let (host_name, path) =
            split_host_and_path(remote.as_str()).ok_or(UnsupportedRemote::UnreadableRemote)?;
        let host = HostKind::from_host_name(host_name).ok_or(UnsupportedRemote::UnsupportedHost)?;
        let mut segments = path.split('/').filter(|segment| !segment.is_empty());
        let owner = segments.next();
        let name = segments
            .next()
            .map(|segment| segment.strip_suffix(GIT_SUFFIX).unwrap_or(segment))
            .filter(|name| !name.is_empty());
        match (owner, name) {
            (Some(owner), Some(name)) => Ok(Self {
                host,
                owner: owner.to_string(),
                name: name.to_string(),
            }),
            _ => Err(UnsupportedRemote::MissingRepository),
        }
    }
}

/// The host name and path of `scheme://[user@]host[:port]/path` or of the SSH
/// shorthand `[user@]host:path`.
fn split_host_and_path(remote: &str) -> Option<(&str, &str)> {
    let remote = remote.trim();
    if let Some(after_scheme) = URL_SCHEMES
        .iter()
        .find_map(|scheme| remote.strip_prefix(scheme))
    {
        let (authority, path) = after_scheme.split_once('/')?;
        let host_and_port = without_user(authority);
        let host_name = host_and_port
            .split_once(':')
            .map_or(host_and_port, |(host_name, _port)| host_name);
        return Some((host_name, path));
    }
    if remote.contains(SCHEME_SEPARATOR) {
        return None;
    }
    let (authority, path) = remote.split_once(':')?;
    if authority.contains('/') {
        return None;
    }
    Some((without_user(authority), path))
}

fn without_user(authority: &str) -> &str {
    authority
        .rsplit_once('@')
        .map_or(authority, |(_user, host)| host)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GITHUB_OWNER: &str = "FredPerr";
    const BITBUCKET_WORKSPACE: &str = "fredperr";
    const REPOSITORY_NAME: &str = "me";

    fn hosted_repository(remote: &str) -> Result<HostedRepository, UnsupportedRemote> {
        HostedRepository::from_remote(&RemoteUrl::new(remote))
    }

    fn github_repository() -> Result<HostedRepository, UnsupportedRemote> {
        Ok(HostedRepository {
            host: HostKind::Github,
            owner: GITHUB_OWNER.into(),
            name: REPOSITORY_NAME.into(),
        })
    }

    fn bitbucket_repository() -> Result<HostedRepository, UnsupportedRemote> {
        Ok(HostedRepository {
            host: HostKind::Bitbucket,
            owner: BITBUCKET_WORKSPACE.into(),
            name: REPOSITORY_NAME.into(),
        })
    }

    mod github {
        use super::*;

        #[test]
        fn reads_an_https_url() {
            let remote = format!("https://github.com/{GITHUB_OWNER}/{REPOSITORY_NAME}");
            assert_eq!(hosted_repository(&remote), github_repository());
        }

        #[test]
        fn drops_the_git_suffix() {
            let remote = format!("https://github.com/{GITHUB_OWNER}/{REPOSITORY_NAME}.git");
            assert_eq!(hosted_repository(&remote), github_repository());
        }

        #[test]
        fn reads_an_ssh_shorthand_remote() {
            let remote = format!("git@github.com:{GITHUB_OWNER}/{REPOSITORY_NAME}.git");
            assert_eq!(hosted_repository(&remote), github_repository());
        }

        #[test]
        fn drops_path_parts_after_owner_and_name() {
            let remote = format!("https://github.com/{GITHUB_OWNER}/{REPOSITORY_NAME}/tree/main");
            assert_eq!(hosted_repository(&remote), github_repository());
        }

        #[test]
        fn rejects_another_host() {
            let remote = format!("https://gitlab.com/{GITHUB_OWNER}/{REPOSITORY_NAME}");
            assert_eq!(
                hosted_repository(&remote),
                Err(UnsupportedRemote::UnsupportedHost)
            );
        }

        #[test]
        fn rejects_a_url_without_a_repository_name() {
            let remote = format!("https://github.com/{GITHUB_OWNER}");
            assert_eq!(
                hosted_repository(&remote),
                Err(UnsupportedRemote::MissingRepository)
            );
        }
    }

    mod bitbucket {
        use super::*;

        #[test]
        fn reads_an_https_url() {
            let remote = format!("https://bitbucket.org/{BITBUCKET_WORKSPACE}/{REPOSITORY_NAME}");
            assert_eq!(hosted_repository(&remote), bitbucket_repository());
        }

        #[test]
        fn drops_the_git_suffix() {
            let remote =
                format!("https://bitbucket.org/{BITBUCKET_WORKSPACE}/{REPOSITORY_NAME}.git");
            assert_eq!(hosted_repository(&remote), bitbucket_repository());
        }

        #[test]
        fn reads_an_ssh_shorthand_remote() {
            let remote = format!("git@bitbucket.org:{BITBUCKET_WORKSPACE}/{REPOSITORY_NAME}.git");
            assert_eq!(hosted_repository(&remote), bitbucket_repository());
        }

        #[test]
        fn drops_path_parts_after_workspace_and_name() {
            let remote =
                format!("https://bitbucket.org/{BITBUCKET_WORKSPACE}/{REPOSITORY_NAME}/src/main");
            assert_eq!(hosted_repository(&remote), bitbucket_repository());
        }

        #[test]
        fn reads_a_remote_of_another_supported_host_as_that_host() {
            let remote = format!("https://github.com/{BITBUCKET_WORKSPACE}/{REPOSITORY_NAME}");
            assert_eq!(
                hosted_repository(&remote).map(|repository| repository.host),
                Ok(HostKind::Github)
            );
        }

        #[test]
        fn rejects_a_url_without_a_repository_name() {
            let remote = format!("https://bitbucket.org/{BITBUCKET_WORKSPACE}");
            assert_eq!(
                hosted_repository(&remote),
                Err(UnsupportedRemote::MissingRepository)
            );
        }
    }

    #[test]
    fn reads_an_ssh_url() {
        let remote = format!("ssh://git@github.com/{GITHUB_OWNER}/{REPOSITORY_NAME}.git");
        assert_eq!(hosted_repository(&remote), github_repository());
    }

    #[test]
    fn reads_an_ssh_url_with_a_port() {
        let remote = format!("ssh://git@bitbucket.org:22/{BITBUCKET_WORKSPACE}/{REPOSITORY_NAME}");
        assert_eq!(hosted_repository(&remote), bitbucket_repository());
    }

    #[test]
    fn reads_an_http_url() {
        let remote = format!("http://github.com/{GITHUB_OWNER}/{REPOSITORY_NAME}");
        assert_eq!(hosted_repository(&remote), github_repository());
    }

    #[test]
    fn reads_an_https_url_with_a_user() {
        let remote = format!("https://{BITBUCKET_WORKSPACE}@bitbucket.org/{BITBUCKET_WORKSPACE}/{REPOSITORY_NAME}.git");
        assert_eq!(hosted_repository(&remote), bitbucket_repository());
    }

    #[test]
    fn drops_a_trailing_slash() {
        let remote = format!("https://github.com/{GITHUB_OWNER}/{REPOSITORY_NAME}/");
        assert_eq!(hosted_repository(&remote), github_repository());
    }

    #[test]
    fn drops_a_trailing_slash_after_the_git_suffix() {
        let remote = format!("https://github.com/{GITHUB_OWNER}/{REPOSITORY_NAME}.git/");
        assert_eq!(hosted_repository(&remote), github_repository());
    }

    #[test]
    fn matches_the_host_name_exactly() {
        for host_name in [
            "notgithub.com",
            "github.com.example.org",
            "www.bitbucket.org",
        ] {
            let remote = format!("https://{host_name}/{GITHUB_OWNER}/{REPOSITORY_NAME}");
            assert_eq!(
                hosted_repository(&remote),
                Err(UnsupportedRemote::UnsupportedHost),
                "{host_name}"
            );
        }
    }

    #[test]
    fn rejects_a_gitlab_ssh_remote() {
        let remote = format!("git@gitlab.com:{GITHUB_OWNER}/{REPOSITORY_NAME}.git");
        assert_eq!(
            hosted_repository(&remote),
            Err(UnsupportedRemote::UnsupportedHost)
        );
    }

    #[test]
    fn rejects_a_repository_name_that_is_only_the_git_suffix() {
        let remote = format!("https://github.com/{GITHUB_OWNER}/{GIT_SUFFIX}");
        assert_eq!(
            hosted_repository(&remote),
            Err(UnsupportedRemote::MissingRepository)
        );
    }

    #[test]
    fn rejects_a_remote_that_is_not_a_url() {
        for remote in ["", "../local/path", "file:///srv/git/me.git"] {
            assert_eq!(
                hosted_repository(remote),
                Err(UnsupportedRemote::UnreadableRemote),
                "{remote}"
            );
        }
    }
}
