//! Plain GitHub → domain mapping of pull request lists and their `Link`
//! header, and the query finding a branch's pull requests.

use super::wire::PullRequestResponse;
use crate::git_hosting::domain::hosted_pull_request::{
    most_recently_updated, HostedPullRequest, PullRequestState,
};

const OPEN: &str = "open";
const NEXT_RELATION: &str = "next";

/// The pull requests from `owner:branch` in every state, most recently
/// updated first, on a single page.
pub fn branch_pull_requests_query(owner: &str, branch: &str) -> Vec<(&'static str, String)> {
    vec![
        ("head", format!("{owner}:{branch}")),
        ("state", "all".to_string()),
        ("sort", "updated".to_string()),
        ("direction", "desc".to_string()),
        ("per_page", "100".to_string()),
    ]
}

pub fn branch_pull_request(responses: Vec<PullRequestResponse>) -> Option<HostedPullRequest> {
    most_recently_updated(responses.into_iter().map(pull_request))
}

pub fn pull_request(response: PullRequestResponse) -> HostedPullRequest {
    let state = if response.merged_at.is_some() {
        PullRequestState::Merged
    } else if response.state.eq_ignore_ascii_case(OPEN) {
        PullRequestState::Open
    } else {
        PullRequestState::Closed
    };
    let is_fork = is_fork(&response);
    HostedPullRequest {
        number: response.number,
        title: response.title,
        url: response.html_url,
        author: response.user.map(|user| user.login),
        is_fork,
        head_branch: response.head.name,
        base_branch: response.base.name,
        draft: response.draft && state == PullRequestState::Open,
        state,
        updated_at: response.updated_at,
    }
}

/// A fork only when both repositories are known and differ. The list
/// endpoint omits the head repository for a deleted fork, so a missing id
/// means not a fork rather than a guess.
fn is_fork(response: &PullRequestResponse) -> bool {
    match (&response.head.repo, &response.base.repo) {
        (Some(head), Some(base)) => head.id != base.id,
        _ => false,
    }
}

/// The `rel="next"` URL of a `Link` header such as
/// `<https://api.github.com/...&page=2>; rel="next", <...&page=5>; rel="last"`.
pub fn next_page_url(link_header: Option<&str>) -> Option<String> {
    link_header?.split(',').find_map(|link| {
        let (target, parameters) = link.trim().split_once(';')?;
        let url = target.trim().strip_prefix('<')?.strip_suffix('>')?;
        let is_next = parameters.split(';').any(|parameter| {
            parameter
                .trim()
                .strip_prefix("rel=")
                .map(|relations| relations.trim_matches('"'))
                .is_some_and(|relations| {
                    relations
                        .split_whitespace()
                        .any(|relation| relation.eq_ignore_ascii_case(NEXT_RELATION))
                })
        });
        is_next.then(|| url.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NUMBER: u32 = 1347;
    const TITLE: &str = "Amazing new feature";
    const URL: &str = "https://github.com/octocat/Hello-World/pull/1347";
    const AUTHOR: &str = "octocat";
    const HEAD_BRANCH: &str = "new-topic";
    const BASE_BRANCH: &str = "master";
    const UPDATED_AT: &str = "2011-01-26T19:01:12Z";
    const OLDER_NUMBER: u32 = 1200;
    const OLDER_UPDATED_AT: &str = "2010-12-01T08:00:00Z";
    const NEXT_URL: &str = "https://api.github.com/repositories/1300192/pulls?page=2";
    const LAST_URL: &str = "https://api.github.com/repositories/1300192/pulls?page=5";
    const SAME_REPOSITORY_ID: u64 = 1_300_192;
    const FORK_REPOSITORY_ID: u64 = 42;

    fn answer(state: &str, draft: bool, merged_at: Option<&str>) -> String {
        let repository = format!(r#"{{ "id": {SAME_REPOSITORY_ID} }}"#);
        answer_with_repositories(state, draft, merged_at, &repository, &repository)
    }

    fn answer_with_repositories(
        state: &str,
        draft: bool,
        merged_at: Option<&str>,
        head_repository: &str,
        base_repository: &str,
    ) -> String {
        let merged_at = merged_at.map_or("null".to_string(), |date| format!(r#""{date}""#));
        format!(
            r#"{{
                "id": 1,
                "number": {NUMBER},
                "state": "{state}",
                "title": "{TITLE}",
                "html_url": "{URL}",
                "user": {{ "login": "{AUTHOR}", "id": 1 }},
                "head": {{ "label": "octocat:{HEAD_BRANCH}", "ref": "{HEAD_BRANCH}", "sha": "6dcb09b", "repo": {head_repository} }},
                "base": {{ "label": "octocat:{BASE_BRANCH}", "ref": "{BASE_BRANCH}", "sha": "6dcb09b", "repo": {base_repository} }},
                "draft": {draft},
                "merged_at": {merged_at},
                "updated_at": "{UPDATED_AT}"
            }}"#
        )
    }

    fn mapped(answer: &str) -> HostedPullRequest {
        pull_request(serde_json::from_str(answer).unwrap())
    }

    #[test]
    fn reads_an_open_pull_request() {
        assert_eq!(
            mapped(&answer(OPEN, false, None)),
            HostedPullRequest {
                number: NUMBER,
                title: TITLE.to_string(),
                url: URL.to_string(),
                author: Some(AUTHOR.to_string()),
                head_branch: HEAD_BRANCH.to_string(),
                base_branch: BASE_BRANCH.to_string(),
                is_fork: false,
                draft: false,
                state: PullRequestState::Open,
                updated_at: UPDATED_AT.to_string(),
            }
        );
    }

    #[test]
    fn a_pull_request_from_the_same_repository_is_not_a_fork() {
        assert!(!mapped(&answer(OPEN, false, None)).is_fork);
    }

    #[test]
    fn a_pull_request_from_a_different_repository_is_a_fork() {
        let answer = answer_with_repositories(
            OPEN,
            false,
            None,
            &format!(r#"{{ "id": {FORK_REPOSITORY_ID} }}"#),
            &format!(r#"{{ "id": {SAME_REPOSITORY_ID} }}"#),
        );

        assert!(mapped(&answer).is_fork);
    }

    #[test]
    fn a_pull_request_without_a_head_repository_is_not_a_fork() {
        let answer = answer_with_repositories(
            OPEN,
            false,
            None,
            "null",
            &format!(r#"{{ "id": {SAME_REPOSITORY_ID} }}"#),
        );

        assert!(!mapped(&answer).is_fork);
    }

    #[test]
    fn an_open_draft_is_a_draft() {
        assert!(mapped(&answer(OPEN, true, None)).draft);
    }

    #[test]
    fn a_closed_pull_request_with_a_merge_date_is_merged() {
        let pull_request = mapped(&answer("closed", false, Some(UPDATED_AT)));

        assert_eq!(pull_request.state, PullRequestState::Merged);
    }

    #[test]
    fn a_closed_pull_request_without_a_merge_date_is_closed() {
        let pull_request = mapped(&answer("closed", true, None));

        assert_eq!(pull_request.state, PullRequestState::Closed);
        assert!(!pull_request.draft);
    }

    #[test]
    fn a_pull_request_of_a_deleted_user_has_no_author() {
        let without_user = answer(OPEN, false, None).replace(
            &format!(r#""user": {{ "login": "{AUTHOR}", "id": 1 }}"#),
            r#""user": null"#,
        );

        assert_eq!(mapped(&without_user).author, None);
    }

    #[test]
    fn finds_the_pull_requests_of_a_branch_in_every_state() {
        assert_eq!(
            branch_pull_requests_query("octocat", HEAD_BRANCH),
            vec![
                ("head", format!("octocat:{HEAD_BRANCH}")),
                ("state", "all".to_string()),
                ("sort", "updated".to_string()),
                ("direction", "desc".to_string()),
                ("per_page", "100".to_string()),
            ]
        );
    }

    #[test]
    fn the_most_recently_updated_pull_request_of_a_branch_wins() {
        let older = answer("closed", false, None)
            .replace(&NUMBER.to_string(), &OLDER_NUMBER.to_string())
            .replace(UPDATED_AT, OLDER_UPDATED_AT);
        let answers = format!("[{older}, {}]", answer(OPEN, false, None));

        let found = branch_pull_request(serde_json::from_str(&answers).unwrap());

        assert_eq!(found.map(|found| found.number), Some(NUMBER));
    }

    #[test]
    fn a_branch_without_pull_requests_has_none() {
        assert_eq!(branch_pull_request(Vec::new()), None);
    }

    #[test]
    fn reads_the_next_page_url_of_a_link_header() {
        let header = format!(r#"<{NEXT_URL}>; rel="next", <{LAST_URL}>; rel="last""#);

        assert_eq!(next_page_url(Some(&header)), Some(NEXT_URL.to_string()));
    }

    #[test]
    fn reads_the_next_page_url_when_it_is_not_the_first_link() {
        let header = format!(r#"<{LAST_URL}>; rel="prev", <{NEXT_URL}>; rel="next""#);

        assert_eq!(next_page_url(Some(&header)), Some(NEXT_URL.to_string()));
    }

    #[test]
    fn the_last_page_has_no_next_page() {
        let header = format!(r#"<{NEXT_URL}>; rel="prev", <{LAST_URL}>; rel="first""#);

        assert_eq!(next_page_url(Some(&header)), None);
    }

    #[test]
    fn a_single_page_has_no_link_header() {
        assert_eq!(next_page_url(None), None);
    }
}
