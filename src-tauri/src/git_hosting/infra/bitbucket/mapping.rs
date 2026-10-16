//! Plain Bitbucket → domain mapping: JSON answers and status codes.

use super::wire::UserResponse;
use crate::git_hosting::domain::error::GitHostingError;
use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::hosted_account::HostedAccount;
use crate::git_hosting::infra::http::retry_after_seconds;

/// The error of a failed answer, `None` on success.
pub fn status_error(status: u16, retry_after: Option<&str>) -> Option<GitHostingError> {
    match status {
        200..=299 => None,
        401 => Some(GitHostingError::Unauthorized),
        403 => Some(GitHostingError::Forbidden),
        404 => Some(GitHostingError::NotFound),
        429 => Some(GitHostingError::RateLimited {
            retry_after_seconds: retry_after_seconds(retry_after),
        }),
        _ => Some(GitHostingError::ProviderError {
            status: Some(status),
        }),
    }
}

/// Bitbucket may leave `username` out; the username the user signed in with
/// is the login then.
pub fn account(user: UserResponse, signed_in_username: &str) -> HostedAccount {
    HostedAccount {
        host: HostKind::Bitbucket,
        login: user
            .username
            .unwrap_or_else(|| signed_in_username.to_string()),
        name: user.display_name,
        avatar_url: user
            .links
            .and_then(|links| links.avatar)
            .and_then(|avatar| avatar.href),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIGNED_IN_USERNAME: &str = "alice-entered";
    const USERNAME: &str = "alice";
    const DISPLAY_NAME: &str = "Alice Example";
    const AVATAR_URL: &str = "https://avatar-management.example/alice.png";
    const RETRY_AFTER_SECONDS: u32 = 30;

    const USER_ANSWER: &str = r#"{
        "type": "user",
        "username": "alice",
        "display_name": "Alice Example",
        "account_id": "557058:0000",
        "links": {
            "avatar": { "href": "https://avatar-management.example/alice.png" },
            "html": { "href": "https://bitbucket.org/alice/" }
        }
    }"#;

    const USER_WITHOUT_USERNAME_ANSWER: &str = r#"{
        "type": "user",
        "display_name": "Alice Example",
        "links": { "avatar": { "href": "https://avatar-management.example/alice.png" } }
    }"#;

    const USER_WITHOUT_LINKS_ANSWER: &str = r#"{ "username": "alice", "display_name": null }"#;

    fn user(answer: &str) -> HostedAccount {
        account(serde_json::from_str(answer).unwrap(), SIGNED_IN_USERNAME)
    }

    #[test]
    fn reads_username_display_name_and_avatar() {
        assert_eq!(
            user(USER_ANSWER),
            HostedAccount {
                host: HostKind::Bitbucket,
                login: USERNAME.to_string(),
                name: Some(DISPLAY_NAME.to_string()),
                avatar_url: Some(AVATAR_URL.to_string()),
            }
        );
    }

    #[test]
    fn falls_back_to_the_signed_in_username() {
        assert_eq!(user(USER_WITHOUT_USERNAME_ANSWER).login, SIGNED_IN_USERNAME);
    }

    #[test]
    fn reads_a_user_without_links_or_name() {
        assert_eq!(
            user(USER_WITHOUT_LINKS_ANSWER),
            HostedAccount {
                host: HostKind::Bitbucket,
                login: USERNAME.to_string(),
                name: None,
                avatar_url: None,
            }
        );
    }

    #[test]
    fn success_is_no_error() {
        assert_eq!(status_error(200, None), None);
    }

    #[test]
    fn unauthorized_forbidden_and_not_found() {
        assert_eq!(status_error(401, None), Some(GitHostingError::Unauthorized));
        assert_eq!(status_error(403, None), Some(GitHostingError::Forbidden));
        assert_eq!(status_error(404, None), Some(GitHostingError::NotFound));
    }

    #[test]
    fn too_many_requests_carries_retry_after() {
        assert_eq!(
            status_error(429, Some(&RETRY_AFTER_SECONDS.to_string())),
            Some(GitHostingError::RateLimited {
                retry_after_seconds: Some(RETRY_AFTER_SECONDS)
            })
        );
        assert_eq!(
            status_error(429, None),
            Some(GitHostingError::RateLimited {
                retry_after_seconds: None
            })
        );
    }

    #[test]
    fn other_statuses_are_provider_errors() {
        for status in [302, 400, 500, 503] {
            assert_eq!(
                status_error(status, None),
                Some(GitHostingError::ProviderError {
                    status: Some(status)
                })
            );
        }
    }
}
