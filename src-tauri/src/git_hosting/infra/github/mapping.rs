//! Plain GitHub → domain mapping: JSON answers, status codes and rate limits.

use super::wire::{AccessTokenResponse, DeviceCodeResponse, UserResponse};
use crate::git_hosting::domain::error::{GitHostingError, InputField};
use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::host_secret::HostSecret;
use crate::git_hosting::domain::hosted_account::HostedAccount;
use crate::git_hosting::domain::sign_in::{BrowserSignInStep, SignInPoll};
use crate::git_hosting::infra::http::retry_after_seconds;

/// GitHub's documented default when `slow_down` comes without an interval.
const DEFAULT_SLOW_DOWN_INTERVAL_SECONDS: u32 = 5;
const NO_REQUESTS_REMAINING: &str = "0";

/// The rate-limit headers of a GitHub answer.
#[derive(Debug, Default, Clone, Copy)]
pub struct RateLimitHeaders<'answer> {
    pub retry_after: Option<&'answer str>,
    pub remaining: Option<&'answer str>,
    pub reset: Option<&'answer str>,
}

/// The error of a failed answer, `None` on success. GitHub signals a rate
/// limit with 403 or 429 and either `Retry-After` or no remaining requests
/// until `X-RateLimit-Reset` (epoch seconds).
pub fn status_error(
    status: u16,
    rate_limit: RateLimitHeaders<'_>,
    now_epoch_seconds: u64,
) -> Option<GitHostingError> {
    match status {
        200..=299 => None,
        401 => Some(GitHostingError::Unauthorized),
        403 | 429 => Some(rate_limited(rate_limit, now_epoch_seconds).unwrap_or(
            if status == 429 {
                GitHostingError::RateLimited {
                    retry_after_seconds: None,
                }
            } else {
                GitHostingError::Forbidden
            },
        )),
        404 => Some(GitHostingError::NotFound),
        _ => Some(GitHostingError::ProviderError {
            status: Some(status),
        }),
    }
}

fn rate_limited(
    rate_limit: RateLimitHeaders<'_>,
    now_epoch_seconds: u64,
) -> Option<GitHostingError> {
    if rate_limit.retry_after.is_some() {
        return Some(GitHostingError::RateLimited {
            retry_after_seconds: retry_after_seconds(rate_limit.retry_after),
        });
    }
    if rate_limit.remaining.map(str::trim) != Some(NO_REQUESTS_REMAINING) {
        return None;
    }
    let retry_after_seconds = rate_limit
        .reset
        .and_then(|reset| reset.trim().parse::<u64>().ok())
        .map(|reset| reset.saturating_sub(now_epoch_seconds))
        .map(|seconds| u32::try_from(seconds).unwrap_or(u32::MAX));
    Some(GitHostingError::RateLimited {
        retry_after_seconds,
    })
}

pub fn browser_sign_in_step(response: DeviceCodeResponse) -> BrowserSignInStep {
    BrowserSignInStep {
        user_code: response.user_code,
        verification_url: response.verification_uri,
        device_code: response.device_code,
        interval_seconds: response.interval,
        expires_in_seconds: response.expires_in,
    }
}

pub fn sign_in_poll(response: AccessTokenResponse) -> Result<SignInPoll, GitHostingError> {
    if let Some(token) = response.access_token.filter(|token| !token.is_empty()) {
        return Ok(SignInPoll::Authorized(HostSecret::new(token)));
    }
    match response.error.as_deref() {
        Some("authorization_pending") => Ok(SignInPoll::Pending),
        Some("slow_down") => Ok(SignInPoll::SlowDown {
            interval_seconds: response
                .interval
                .unwrap_or(DEFAULT_SLOW_DOWN_INTERVAL_SECONDS),
        }),
        Some("expired_token") => Ok(SignInPoll::Expired),
        Some("access_denied") => Ok(SignInPoll::Denied),
        Some("incorrect_device_code") => {
            Err(GitHostingError::invalid_input(InputField::DeviceCode))
        }
        Some(other) => {
            eprintln!("[git_hosting] github sign-in refused: {other}");
            Err(GitHostingError::ProviderError { status: None })
        }
        None => Err(GitHostingError::InvalidResponse),
    }
}

pub fn account(user: UserResponse) -> HostedAccount {
    HostedAccount {
        host: HostKind::Github,
        login: user.login,
        name: user.name,
        avatar_url: user.avatar_url,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_700_000_000;
    const SECONDS_UNTIL_RESET: u32 = 120;
    const RETRY_AFTER_SECONDS: u32 = 60;
    const TOKEN: &str = "gho_token";
    const LOGIN: &str = "octocat";
    const NAME: &str = "The Octocat";
    const AVATAR_URL: &str = "https://avatars.githubusercontent.com/u/583231?v=4";
    const SLOWER_INTERVAL_SECONDS: u32 = 10;

    const DEVICE_CODE_ANSWER: &str = r#"{
        "device_code": "3584d83530557fdd1f46af8289938c8ef79f9dc5",
        "user_code": "WDJB-MJHT",
        "verification_uri": "https://github.com/login/device",
        "expires_in": 900,
        "interval": 5
    }"#;

    const USER_ANSWER: &str = r#"{
        "login": "octocat",
        "id": 583231,
        "name": "The Octocat",
        "avatar_url": "https://avatars.githubusercontent.com/u/583231?v=4",
        "type": "User"
    }"#;

    const USER_WITHOUT_DETAILS_ANSWER: &str = r#"{ "login": "octocat", "name": null }"#;

    fn reset_in(seconds: u32) -> String {
        (NOW + u64::from(seconds)).to_string()
    }

    fn poll(answer: &str) -> Result<SignInPoll, GitHostingError> {
        sign_in_poll(serde_json::from_str(answer).unwrap())
    }

    fn user(answer: &str) -> HostedAccount {
        account(serde_json::from_str(answer).unwrap())
    }

    mod status_codes {
        use super::*;

        #[test]
        fn success_is_no_error() {
            assert_eq!(status_error(200, RateLimitHeaders::default(), NOW), None);
        }

        #[test]
        fn unauthorized_forbidden_and_not_found() {
            let no_headers = RateLimitHeaders::default();
            assert_eq!(
                status_error(401, no_headers, NOW),
                Some(GitHostingError::Unauthorized)
            );
            assert_eq!(
                status_error(403, no_headers, NOW),
                Some(GitHostingError::Forbidden)
            );
            assert_eq!(
                status_error(404, no_headers, NOW),
                Some(GitHostingError::NotFound)
            );
        }

        #[test]
        fn other_statuses_are_provider_errors() {
            for status in [301, 422, 500, 503] {
                assert_eq!(
                    status_error(status, RateLimitHeaders::default(), NOW),
                    Some(GitHostingError::ProviderError {
                        status: Some(status)
                    })
                );
            }
        }

        #[test]
        fn retry_after_makes_a_rate_limit() {
            let retry_after = RETRY_AFTER_SECONDS.to_string();
            let headers = RateLimitHeaders {
                retry_after: Some(&retry_after),
                ..RateLimitHeaders::default()
            };
            for status in [403, 429] {
                assert_eq!(
                    status_error(status, headers, NOW),
                    Some(GitHostingError::RateLimited {
                        retry_after_seconds: Some(RETRY_AFTER_SECONDS)
                    })
                );
            }
        }

        #[test]
        fn no_remaining_requests_waits_until_the_reset() {
            let reset = reset_in(SECONDS_UNTIL_RESET);
            let headers = RateLimitHeaders {
                remaining: Some(NO_REQUESTS_REMAINING),
                reset: Some(&reset),
                ..RateLimitHeaders::default()
            };
            assert_eq!(
                status_error(403, headers, NOW),
                Some(GitHostingError::RateLimited {
                    retry_after_seconds: Some(SECONDS_UNTIL_RESET)
                })
            );
        }

        #[test]
        fn a_reset_in_the_past_retries_now() {
            let reset = (NOW - 1).to_string();
            let headers = RateLimitHeaders {
                remaining: Some(NO_REQUESTS_REMAINING),
                reset: Some(&reset),
                ..RateLimitHeaders::default()
            };
            assert_eq!(
                status_error(403, headers, NOW),
                Some(GitHostingError::RateLimited {
                    retry_after_seconds: Some(0)
                })
            );
        }

        #[test]
        fn a_forbidden_answer_with_requests_remaining_is_forbidden() {
            let reset = reset_in(SECONDS_UNTIL_RESET);
            let headers = RateLimitHeaders {
                remaining: Some("4999"),
                reset: Some(&reset),
                ..RateLimitHeaders::default()
            };
            assert_eq!(
                status_error(403, headers, NOW),
                Some(GitHostingError::Forbidden)
            );
        }

        #[test]
        fn too_many_requests_without_headers_has_no_retry_time() {
            assert_eq!(
                status_error(429, RateLimitHeaders::default(), NOW),
                Some(GitHostingError::RateLimited {
                    retry_after_seconds: None
                })
            );
        }
    }

    mod device_flow {
        use super::*;

        #[test]
        fn reads_the_browser_step() {
            let step = browser_sign_in_step(serde_json::from_str(DEVICE_CODE_ANSWER).unwrap());

            assert_eq!(
                step,
                BrowserSignInStep {
                    user_code: "WDJB-MJHT".to_string(),
                    verification_url: "https://github.com/login/device".to_string(),
                    device_code: "3584d83530557fdd1f46af8289938c8ef79f9dc5".to_string(),
                    interval_seconds: 5,
                    expires_in_seconds: 900,
                }
            );
        }

        #[test]
        fn an_access_token_is_authorized() {
            let answer =
                format!(r#"{{ "access_token": "{TOKEN}", "token_type": "bearer", "scope": "" }}"#);

            let Ok(SignInPoll::Authorized(secret)) = poll(&answer) else {
                panic!("expected an authorized sign-in");
            };
            assert_eq!(secret.expose(), TOKEN);
        }

        #[test]
        fn authorization_pending_is_pending() {
            let answer = r#"{ "error": "authorization_pending", "error_description": "waiting" }"#;
            assert!(matches!(poll(answer), Ok(SignInPoll::Pending)));
        }

        #[test]
        fn slow_down_carries_the_new_interval() {
            let answer =
                format!(r#"{{ "error": "slow_down", "interval": {SLOWER_INTERVAL_SECONDS} }}"#);
            assert!(matches!(
                poll(&answer),
                Ok(SignInPoll::SlowDown { interval_seconds }) if interval_seconds == SLOWER_INTERVAL_SECONDS
            ));
        }

        #[test]
        fn slow_down_without_an_interval_waits_five_seconds() {
            assert!(matches!(
                poll(r#"{ "error": "slow_down" }"#),
                Ok(SignInPoll::SlowDown { interval_seconds }) if interval_seconds == DEFAULT_SLOW_DOWN_INTERVAL_SECONDS
            ));
        }

        #[test]
        fn expired_token_is_expired() {
            assert!(matches!(
                poll(r#"{ "error": "expired_token" }"#),
                Ok(SignInPoll::Expired)
            ));
        }

        #[test]
        fn access_denied_is_denied() {
            assert!(matches!(
                poll(r#"{ "error": "access_denied" }"#),
                Ok(SignInPoll::Denied)
            ));
        }

        #[test]
        fn an_incorrect_device_code_is_invalid_input() {
            assert!(matches!(
                poll(r#"{ "error": "incorrect_device_code" }"#),
                Err(GitHostingError::InvalidInput {
                    field: InputField::DeviceCode
                })
            ));
        }

        #[test]
        fn another_error_code_is_a_provider_error() {
            assert!(matches!(
                poll(r#"{ "error": "device_flow_disabled" }"#),
                Err(GitHostingError::ProviderError { status: None })
            ));
        }

        #[test]
        fn an_answer_without_token_or_error_is_invalid() {
            assert!(matches!(
                poll(r#"{ "access_token": "" }"#),
                Err(GitHostingError::InvalidResponse)
            ));
        }
    }

    mod user {
        use super::*;

        #[test]
        fn reads_login_name_and_avatar() {
            assert_eq!(
                user(USER_ANSWER),
                HostedAccount {
                    host: HostKind::Github,
                    login: LOGIN.to_string(),
                    name: Some(NAME.to_string()),
                    avatar_url: Some(AVATAR_URL.to_string()),
                }
            );
        }

        #[test]
        fn reads_a_user_without_name_or_avatar() {
            assert_eq!(
                user(USER_WITHOUT_DETAILS_ANSWER),
                HostedAccount {
                    host: HostKind::Github,
                    login: LOGIN.to_string(),
                    name: None,
                    avatar_url: None,
                }
            );
        }
    }
}
