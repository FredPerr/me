//! Plain mapping of a new pull request to Bitbucket's request, and of
//! Bitbucket's answer to the created pull request.

use super::wire::{
    BranchEndpointRequest, BranchNameRequest, CreatePullRequestRequest, CreatedPullRequestResponse,
};
use crate::git_hosting::domain::error::GitHostingError;
use crate::git_hosting::domain::new_pull_request::{CreatedPullRequest, NewPullRequest};

/// The source branch is kept on merge: the app's worktree still uses it.
pub fn create_pull_request_request(pull_request: &NewPullRequest) -> CreatePullRequestRequest<'_> {
    CreatePullRequestRequest {
        title: &pull_request.title,
        description: &pull_request.body,
        source: branch_endpoint(&pull_request.head),
        destination: branch_endpoint(&pull_request.base),
        close_source_branch: false,
        draft: pull_request.draft,
    }
}

fn branch_endpoint(name: &str) -> BranchEndpointRequest<'_> {
    BranchEndpointRequest {
        branch: BranchNameRequest { name },
    }
}

/// A created pull request without a web link cannot be opened, so the answer
/// is unreadable.
pub fn created_pull_request(
    response: CreatedPullRequestResponse,
) -> Result<CreatedPullRequest, GitHostingError> {
    let url = response
        .links
        .and_then(|links| links.html)
        .and_then(|link| link.href)
        .filter(|href| !href.is_empty())
        .ok_or(GitHostingError::InvalidResponse)?;
    Ok(CreatedPullRequest {
        number: response.id,
        url,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const TITLE: &str = "Add the login page";
    const DESCRIPTION: &str = "Adds a login page.";
    const HEAD_BRANCH: &str = "feature/login";
    const BASE_BRANCH: &str = "main";
    const NUMBER: u32 = 42;
    const URL: &str = "https://bitbucket.org/acme/api/pull-requests/42";

    fn new_pull_request(draft: bool) -> NewPullRequest {
        NewPullRequest {
            title: TITLE.to_string(),
            body: DESCRIPTION.to_string(),
            head: HEAD_BRANCH.to_string(),
            base: BASE_BRANCH.to_string(),
            draft,
        }
    }

    fn mapped(answer: &str) -> Result<CreatedPullRequest, GitHostingError> {
        created_pull_request(serde_json::from_str(answer).unwrap())
    }

    #[test]
    fn sends_the_branches_and_keeps_the_source_branch() {
        let pull_request = new_pull_request(true);

        let request = serde_json::to_value(create_pull_request_request(&pull_request)).unwrap();

        assert_eq!(
            request,
            json!({
                "title": TITLE,
                "description": DESCRIPTION,
                "source": { "branch": { "name": HEAD_BRANCH } },
                "destination": { "branch": { "name": BASE_BRANCH } },
                "close_source_branch": false,
                "draft": true,
            })
        );
    }

    #[test]
    fn reads_the_number_and_link_of_the_created_pull_request() {
        let answer = format!(
            r#"{{
                "type": "pullrequest",
                "id": {NUMBER},
                "title": "{TITLE}",
                "state": "OPEN",
                "links": {{
                    "self": {{ "href": "https://api.bitbucket.org/2.0/repositories/acme/api/pullrequests/{NUMBER}" }},
                    "html": {{ "href": "{URL}" }}
                }}
            }}"#
        );

        assert_eq!(
            mapped(&answer),
            Ok(CreatedPullRequest {
                number: NUMBER,
                url: URL.to_string(),
            })
        );
    }

    #[test]
    fn an_answer_without_a_web_link_is_unreadable() {
        let answer = format!(r#"{{ "id": {NUMBER}, "links": {{}} }}"#);

        assert_eq!(mapped(&answer), Err(GitHostingError::InvalidResponse));
    }
}
