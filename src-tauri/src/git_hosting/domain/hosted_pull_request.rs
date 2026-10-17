#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestState {
    Open,
    Merged,
    Closed,
}

/// A pull request of a repository on a Git host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedPullRequest {
    pub number: u32,
    pub title: String,
    pub url: String,
    pub author: Option<String>,
    pub head_branch: String,
    pub base_branch: String,
    /// The head repository differs from the base one, so the branch lives in
    /// a fork. Defaults to `false` when the host omits the repositories.
    pub is_fork: bool,
    /// Only ever set while the pull request is open.
    pub draft: bool,
    pub state: PullRequestState,
    pub updated_at: String,
}

/// The pull request updated last, so a re-opened or re-created one wins over
/// older ones of the same branch. A Git host sends every `updated_at` in the
/// same ISO 8601 form, so they compare as text.
pub fn most_recently_updated(
    pull_requests: impl IntoIterator<Item = HostedPullRequest>,
) -> Option<HostedPullRequest> {
    pull_requests.into_iter().reduce(|latest, candidate| {
        if candidate.updated_at > latest.updated_at {
            candidate
        } else {
            latest
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLDER: &str = "2024-04-01T12:00:00Z";
    const NEWER: &str = "2024-05-01T12:00:00Z";
    const OLDER_NUMBER: u32 = 1;
    const NEWER_NUMBER: u32 = 2;
    const SAME_DATE_FIRST_NUMBER: u32 = 3;

    fn pull_request(number: u32, updated_at: &str) -> HostedPullRequest {
        HostedPullRequest {
            number,
            title: String::new(),
            url: String::new(),
            author: None,
            head_branch: String::new(),
            base_branch: String::new(),
            is_fork: false,
            draft: false,
            state: PullRequestState::Open,
            updated_at: updated_at.to_string(),
        }
    }

    #[test]
    fn the_most_recently_updated_pull_request_wins() {
        let latest = most_recently_updated([
            pull_request(OLDER_NUMBER, OLDER),
            pull_request(NEWER_NUMBER, NEWER),
        ]);

        assert_eq!(latest.map(|latest| latest.number), Some(NEWER_NUMBER));
    }

    #[test]
    fn the_first_one_wins_between_equal_dates() {
        let latest = most_recently_updated([
            pull_request(SAME_DATE_FIRST_NUMBER, NEWER),
            pull_request(NEWER_NUMBER, NEWER),
        ]);

        assert_eq!(
            latest.map(|latest| latest.number),
            Some(SAME_DATE_FIRST_NUMBER)
        );
    }

    #[test]
    fn no_pull_request_gives_none() {
        assert_eq!(most_recently_updated(Vec::new()), None);
    }
}
