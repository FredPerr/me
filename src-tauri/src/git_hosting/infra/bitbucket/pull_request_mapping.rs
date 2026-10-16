//! Plain Bitbucket → domain mapping of pull request lists, and the query
//! finding a branch's pull requests.

use super::wire::{PullRequestAuthor, PullRequestResponse};
use crate::git_hosting::domain::hosted_pull_request::{
    most_recently_updated, HostedPullRequest, PullRequestState,
};

const OPEN: &str = "OPEN";
const MERGED: &str = "MERGED";
const EVERY_STATE: [&str; 4] = [OPEN, MERGED, "DECLINED", "SUPERSEDED"];

/// The pull requests from `branch` in every state, most recently updated
/// first. Bitbucket only lists open ones unless each state is asked for.
pub fn branch_pull_requests_query(branch: &str) -> Vec<(&'static str, String)> {
    let mut query = vec![("q", source_branch_filter(branch))];
    query.extend(EVERY_STATE.map(|state| ("state", state.to_string())));
    query.push(("sort", "-updated_on".to_string()));
    query.push(("pagelen", "50".to_string()));
    query
}

/// A Bitbucket query language filter; `"` and `\` are escaped inside its
/// quoted string.
pub fn source_branch_filter(branch: &str) -> String {
    let escaped = branch.replace('\\', "\\\\").replace('"', "\\\"");
    format!(r#"source.branch.name="{escaped}""#)
}

pub fn branch_pull_request(responses: Vec<PullRequestResponse>) -> Option<HostedPullRequest> {
    most_recently_updated(responses.into_iter().map(pull_request))
}

/// Bitbucket's states are OPEN, MERGED, DECLINED and SUPERSEDED; the last
/// two are closed without merging.
pub fn pull_request(response: PullRequestResponse) -> HostedPullRequest {
    let state = if response.state.eq_ignore_ascii_case(OPEN) {
        PullRequestState::Open
    } else if response.state.eq_ignore_ascii_case(MERGED) {
        PullRequestState::Merged
    } else {
        PullRequestState::Closed
    };
    HostedPullRequest {
        number: response.id,
        title: response.title,
        url: response
            .links
            .and_then(|links| links.html)
            .and_then(|link| link.href)
            .unwrap_or_default(),
        author: response.author.and_then(author_name),
        head_branch: response.source.branch.name,
        base_branch: response.destination.branch.name,
        draft: response.draft && state == PullRequestState::Open,
        state,
        updated_at: response.updated_on,
    }
}

fn author_name(author: PullRequestAuthor) -> Option<String> {
    let non_empty = |name: Option<String>| name.filter(|name| !name.trim().is_empty());
    non_empty(author.nickname).or_else(|| non_empty(author.display_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_hosting::infra::bitbucket::wire::PullRequestPage;

    const NUMBER: u32 = 42;
    const TITLE: &str = "Add the login page";
    const URL: &str = "https://bitbucket.org/acme/api/pull-requests/42";
    const NICKNAME: &str = "alice";
    const DISPLAY_NAME: &str = "Alice Example";
    const HEAD_BRANCH: &str = "feature/login";
    const BASE_BRANCH: &str = "main";
    const UPDATED_ON: &str = "2024-05-01T12:00:00.000000+00:00";
    const OLDER_NUMBER: u32 = 17;
    const OLDER_UPDATED_ON: &str = "2024-03-01T09:30:00.000000+00:00";
    const NEXT_URL: &str =
        "https://api.bitbucket.org/2.0/repositories/acme/api/pullrequests?state=OPEN&page=2";

    fn answer(state: &str, draft: bool, author: &str) -> String {
        format!(
            r#"{{
                "type": "pullrequest",
                "id": {NUMBER},
                "title": "{TITLE}",
                "state": "{state}",
                "draft": {draft},
                "author": {author},
                "source": {{ "branch": {{ "name": "{HEAD_BRANCH}" }}, "commit": {{ "hash": "abc" }} }},
                "destination": {{ "branch": {{ "name": "{BASE_BRANCH}" }} }},
                "updated_on": "{UPDATED_ON}",
                "links": {{ "html": {{ "href": "{URL}" }} }}
            }}"#
        )
    }

    fn author(nickname: &str, display_name: &str) -> String {
        format!(r#"{{ "nickname": "{nickname}", "display_name": "{display_name}" }}"#)
    }

    fn open_answer() -> String {
        answer(OPEN, false, &author(NICKNAME, DISPLAY_NAME))
    }

    fn mapped(answer: &str) -> HostedPullRequest {
        pull_request(serde_json::from_str(answer).unwrap())
    }

    #[test]
    fn reads_an_open_pull_request() {
        assert_eq!(
            mapped(&open_answer()),
            HostedPullRequest {
                number: NUMBER,
                title: TITLE.to_string(),
                url: URL.to_string(),
                author: Some(NICKNAME.to_string()),
                head_branch: HEAD_BRANCH.to_string(),
                base_branch: BASE_BRANCH.to_string(),
                draft: false,
                state: PullRequestState::Open,
                updated_at: UPDATED_ON.to_string(),
            }
        );
    }

    #[test]
    fn reads_each_state() {
        for (state, expected) in [
            (OPEN, PullRequestState::Open),
            (MERGED, PullRequestState::Merged),
            ("DECLINED", PullRequestState::Closed),
            ("SUPERSEDED", PullRequestState::Closed),
        ] {
            let author = author(NICKNAME, DISPLAY_NAME);

            assert_eq!(
                mapped(&answer(state, false, &author)).state,
                expected,
                "{state}"
            );
        }
    }

    #[test]
    fn an_open_draft_is_a_draft() {
        assert!(mapped(&answer(OPEN, true, &author(NICKNAME, DISPLAY_NAME))).draft);
    }

    #[test]
    fn a_merged_draft_is_no_draft() {
        assert!(!mapped(&answer(MERGED, true, &author(NICKNAME, DISPLAY_NAME))).draft);
    }

    #[test]
    fn falls_back_to_the_display_name_without_a_nickname() {
        let without_nickname = answer(OPEN, false, &author("", DISPLAY_NAME));

        assert_eq!(
            mapped(&without_nickname).author,
            Some(DISPLAY_NAME.to_string())
        );
    }

    #[test]
    fn a_pull_request_without_author_has_none() {
        assert_eq!(mapped(&answer(OPEN, false, "null")).author, None);
    }

    #[test]
    fn finds_the_pull_requests_of_a_branch_in_every_state() {
        assert_eq!(
            branch_pull_requests_query(HEAD_BRANCH),
            vec![
                ("q", format!(r#"source.branch.name="{HEAD_BRANCH}""#)),
                ("state", OPEN.to_string()),
                ("state", MERGED.to_string()),
                ("state", "DECLINED".to_string()),
                ("state", "SUPERSEDED".to_string()),
                ("sort", "-updated_on".to_string()),
                ("pagelen", "50".to_string()),
            ]
        );
    }

    #[test]
    fn escapes_quotes_and_backslashes_in_the_branch_filter() {
        assert_eq!(
            source_branch_filter(r#"fix/"quoted"\path"#),
            r#"source.branch.name="fix/\"quoted\"\\path""#
        );
    }

    #[test]
    fn the_most_recently_updated_pull_request_of_a_branch_wins() {
        let older = answer("DECLINED", false, "null")
            .replace(&NUMBER.to_string(), &OLDER_NUMBER.to_string())
            .replace(UPDATED_ON, OLDER_UPDATED_ON);
        let page = format!(r#"{{ "values": [{older}, {}] }}"#, open_answer());
        let page: PullRequestPage = serde_json::from_str(&page).unwrap();

        let found = branch_pull_request(page.values);

        assert_eq!(found.map(|found| found.number), Some(NUMBER));
    }

    #[test]
    fn reads_the_next_page_url() {
        let page = format!(
            r#"{{ "values": [{}], "next": "{NEXT_URL}" }}"#,
            open_answer()
        );

        let page: PullRequestPage = serde_json::from_str(&page).unwrap();

        assert_eq!(page.values.len(), 1);
        assert_eq!(page.next, Some(NEXT_URL.to_string()));
    }

    #[test]
    fn the_last_page_has_no_next_page() {
        let page: PullRequestPage = serde_json::from_str(r#"{ "values": [] }"#).unwrap();

        assert_eq!(page.next, None);
    }
}
