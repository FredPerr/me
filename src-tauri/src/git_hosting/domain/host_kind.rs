use serde::{Deserialize, Serialize};

const GITHUB_HOST_NAME: &str = "github.com";
const BITBUCKET_HOST_NAME: &str = "bitbucket.org";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HostKind {
    Github,
    Bitbucket,
}

impl HostKind {
    pub const ALL: [HostKind; 2] = [HostKind::Github, HostKind::Bitbucket];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Bitbucket => "bitbucket",
        }
    }

    pub fn host_name(&self) -> &'static str {
        match self {
            Self::Github => GITHUB_HOST_NAME,
            Self::Bitbucket => BITBUCKET_HOST_NAME,
        }
    }

    /// The Git host whose host name is exactly `host_name`, ignoring ASCII case.
    pub fn from_host_name(host_name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.host_name().eq_ignore_ascii_case(host_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_its_identifier() {
        for kind in HostKind::ALL {
            assert_eq!(
                serde_json::to_string(&kind).unwrap(),
                format!("\"{}\"", kind.as_str())
            );
        }
    }

    #[test]
    fn finds_a_host_by_its_exact_host_name() {
        assert_eq!(
            HostKind::from_host_name(GITHUB_HOST_NAME),
            Some(HostKind::Github)
        );
        assert_eq!(
            HostKind::from_host_name(BITBUCKET_HOST_NAME),
            Some(HostKind::Bitbucket)
        );
        assert_eq!(
            HostKind::from_host_name("GitHub.com"),
            Some(HostKind::Github)
        );
    }

    #[test]
    fn finds_no_host_for_a_similar_host_name() {
        for host_name in ["notgithub.com", "github.com.evil.example", "gitlab.com"] {
            assert_eq!(HostKind::from_host_name(host_name), None, "{host_name}");
        }
    }
}
