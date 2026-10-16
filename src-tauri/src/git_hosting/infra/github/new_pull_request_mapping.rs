//! Plain mapping of a new pull request to GitHub's request, and of GitHub's
//! answer to the created pull request.

use super::wire::{CreatePullRequestRequest, CreatedPullRequestResponse};
use crate::git_hosting::domain::new_pull_request::{CreatedPullRequest, NewPullRequest};

/// `head` is a branch of the same repository, so it needs no `owner:` prefix.
pub fn create_pull_request_request(pull_request: &NewPullRequest) -> CreatePullRequestRequest<'_> {
    CreatePullRequestRequest {
        title: &pull_request.title,
        body: &pull_request.body,
        head: &pull_request.head,
        base: &pull_request.base,
        draft: pull_request.draft,
    }
}

pub fn created_pull_request(response: CreatedPullRequestResponse) -> CreatedPullRequest {
    CreatedPullRequest {
        number: response.number,
        url: response.html_url,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const TITLE: &str = "Amazing new feature";
    const BODY: &str = "Please pull these awesome changes in!";
    const HEAD_BRANCH: &str = "new-topic";
    const BASE_BRANCH: &str = "master";
    const NUMBER: u32 = 1347;
    const URL: &str = "https://github.com/octocat/Hello-World/pull/1347";

    #[test]
    fn sends_the_title_body_branches_and_draft_flag() {
        let pull_request = NewPullRequest {
            title: TITLE.to_string(),
            body: BODY.to_string(),
            head: HEAD_BRANCH.to_string(),
            base: BASE_BRANCH.to_string(),
            draft: true,
        };

        let request = serde_json::to_value(create_pull_request_request(&pull_request)).unwrap();

        assert_eq!(
            request,
            json!({
                "title": TITLE,
                "body": BODY,
                "head": HEAD_BRANCH,
                "base": BASE_BRANCH,
                "draft": true,
            })
        );
    }

    #[test]
    fn reads_the_number_and_link_of_the_created_pull_request() {
        let answer = format!(
            r#"{{
                "id": 1,
                "number": {NUMBER},
                "state": "open",
                "title": "{TITLE}",
                "html_url": "{URL}",
                "url": "https://api.github.com/repos/octocat/Hello-World/pulls/{NUMBER}"
            }}"#
        );

        let created = created_pull_request(serde_json::from_str(&answer).unwrap());

        assert_eq!(
            created,
            CreatedPullRequest {
                number: NUMBER,
                url: URL.to_string(),
            }
        );
    }
}
