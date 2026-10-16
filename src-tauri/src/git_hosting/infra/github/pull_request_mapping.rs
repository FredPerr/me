//! Plain GitHub → domain mapping of pull request lists and their `Link` header.

use super::wire::PullRequestResponse;
use crate::git_hosting::domain::hosted_pull_request::{HostedPullRequest, PullRequestState};

const OPEN: &str = "open";
const NEXT_RELATION: &str = "next";

pub fn pull_request(response: PullRequestResponse) -> HostedPullRequest {
    let state = if response.merged_at.is_some() {
        PullRequestState::Merged
    } else if response.state.eq_ignore_ascii_case(OPEN) {
        PullRequestState::Open
    } else {
        PullRequestState::Closed
    };
    HostedPullRequest {
        number: response.number,
        title: response.title,
        url: response.html_url,
        author: response.user.map(|user| user.login),
        head_branch: response.head.name,
        base_branch: response.base.name,
        draft: response.draft && state == PullRequestState::Open,
        state,
        updated_at: response.updated_at,
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
    const NEXT_URL: &str = "https://api.github.com/repositories/1300192/pulls?page=2";
    const LAST_URL: &str = "https://api.github.com/repositories/1300192/pulls?page=5";

    fn answer(state: &str, draft: bool, merged_at: Option<&str>) -> String {
        let merged_at = merged_at.map_or("null".to_string(), |date| format!(r#""{date}""#));
        format!(
            r#"{{
                "id": 1,
                "number": {NUMBER},
                "state": "{state}",
                "title": "{TITLE}",
                "html_url": "{URL}",
                "user": {{ "login": "{AUTHOR}", "id": 1 }},
                "head": {{ "label": "octocat:{HEAD_BRANCH}", "ref": "{HEAD_BRANCH}", "sha": "6dcb09b" }},
                "base": {{ "label": "octocat:{BASE_BRANCH}", "ref": "{BASE_BRANCH}", "sha": "6dcb09b" }},
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
                draft: false,
                state: PullRequestState::Open,
                updated_at: UPDATED_AT.to_string(),
            }
        );
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
