use std::sync::{Arc, Mutex};

use super::registry::GitHostRegistry;
use super::service::GitHostingService;
use super::test_support::FakeProjectRemotes;
use super::test_support::{
    account, browser_step, BrowserScript, FakeHostTokens, HostScript, RecordingEvents,
    ScriptedBrowserSignIn, ScriptedPoll, BITBUCKET_LOGIN, DEVICE_CODE, GITHUB_LOGIN,
};
use crate::git_hosting::domain::error::{GitHostingError, InputField};
use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::host_secret::HostSecret;
use crate::git_hosting::domain::hosted_account::{HostedAccount, SignedInHost};
use crate::git_hosting::domain::ports::{HostTokens, HostedAccounts};
use crate::git_hosting::domain::sign_in::SignInProgress;
use crate::git_hosting::infra::in_memory_hosted_accounts::InMemoryHostedAccounts;

const GITHUB_TOKEN: &str = "gho_token";
const OTHER_GITHUB_LOGIN: &str = "hubot";
const BITBUCKET_USERNAME: &str = "alice";
const BITBUCKET_TOKEN: &str = "api-token";
const PADDED_USERNAME: &str = "  alice ";
const PADDED_TOKEN: &str = "\tapi-token \n";
const BLANK: &str = "   ";
const SLOWER_INTERVAL_SECONDS: u32 = 10;

fn bitbucket_secret() -> String {
    format!("{BITBUCKET_USERNAME}:{BITBUCKET_TOKEN}")
}

/// The service wired to fakes, keeping a handle on each to inspect it.
struct Harness {
    host_tokens: Arc<FakeHostTokens>,
    accounts: Arc<InMemoryHostedAccounts>,
    events: Arc<RecordingEvents>,
    github: Arc<Mutex<HostScript>>,
    bitbucket: Arc<Mutex<HostScript>>,
    browser: Arc<Mutex<BrowserScript>>,
}

impl Harness {
    fn new() -> Self {
        Self::with_tokens(FakeHostTokens::default())
    }

    fn with_tokens(host_tokens: FakeHostTokens) -> Self {
        Self {
            host_tokens: Arc::new(host_tokens),
            accounts: Arc::new(InMemoryHostedAccounts::default()),
            events: Arc::new(RecordingEvents::default()),
            github: HostScript::shared(Ok(account(HostKind::Github, GITHUB_LOGIN))),
            bitbucket: HostScript::shared(Ok(account(HostKind::Bitbucket, BITBUCKET_LOGIN))),
            browser: BrowserScript::shared(ScriptedPoll::Pending),
        }
    }

    fn signed_in_to_github() -> Self {
        Self::with_tokens(FakeHostTokens::with_secret(HostKind::Github, GITHUB_TOKEN))
    }

    fn signed_in_to_bitbucket() -> Self {
        Self::with_tokens(FakeHostTokens::with_secret(
            HostKind::Bitbucket,
            &bitbucket_secret(),
        ))
    }

    fn service(&self) -> GitHostingService {
        let registry = GitHostRegistry::new()
            .register_host(HostKind::Github, HostScript::factory(&self.github))
            .register_host(HostKind::Bitbucket, HostScript::factory(&self.bitbucket))
            .register_browser_sign_in(
                HostKind::Github,
                Arc::new(ScriptedBrowserSignIn {
                    script: Arc::clone(&self.browser),
                }),
            );
        GitHostingService::new(
            registry,
            self.host_tokens.clone(),
            self.accounts.clone(),
            self.events.clone(),
            Arc::new(FakeProjectRemotes::default()),
        )
    }

    fn answer_github_account(&self, result: Result<HostedAccount, GitHostingError>) {
        self.github.lock().unwrap().account_result = result;
    }

    fn answer_bitbucket_account(&self, result: Result<HostedAccount, GitHostingError>) {
        self.bitbucket.lock().unwrap().account_result = result;
    }

    fn answer_poll(&self, poll: ScriptedPoll) {
        self.browser.lock().unwrap().poll = poll;
    }

    fn github_account_fetches(&self) -> usize {
        self.github.lock().unwrap().account_fetches
    }

    fn bitbucket_account_fetches(&self) -> usize {
        self.bitbucket.lock().unwrap().account_fetches
    }

    fn stored_account(&self, host: HostKind) -> Option<HostedAccount> {
        self.accounts.find(host).unwrap()
    }

    fn store_account(&self, stored: &HostedAccount) {
        self.accounts.save(stored).unwrap();
    }
}

mod list_accounts {
    use super::*;

    #[tokio::test]
    async fn lists_nothing_when_no_host_is_signed_in() {
        let harness = Harness::new();

        let signed_in = harness.service().list_accounts().await.unwrap();

        assert_eq!(signed_in, vec![]);
    }

    #[tokio::test]
    async fn returns_the_stored_account_without_fetching_it() {
        let harness = Harness::signed_in_to_github();
        let stored = account(HostKind::Github, OTHER_GITHUB_LOGIN);
        harness.store_account(&stored);

        let signed_in = harness.service().list_accounts().await.unwrap();

        assert_eq!(
            signed_in,
            vec![SignedInHost {
                host: HostKind::Github,
                account: Some(stored)
            }]
        );
        assert_eq!(harness.github_account_fetches(), 0);
    }

    #[tokio::test]
    async fn fetches_a_missing_account_once_and_stores_it() {
        let harness = Harness::signed_in_to_github();
        let service = harness.service();

        service.list_accounts().await.unwrap();
        let signed_in = service.list_accounts().await.unwrap();

        let fetched = account(HostKind::Github, GITHUB_LOGIN);
        assert_eq!(signed_in[0].account, Some(fetched.clone()));
        assert_eq!(harness.stored_account(HostKind::Github), Some(fetched));
        assert_eq!(harness.github_account_fetches(), 1);
    }

    #[tokio::test]
    async fn builds_the_host_with_the_stored_secret() {
        let harness = Harness::signed_in_to_bitbucket();

        harness.service().list_accounts().await.unwrap();

        assert_eq!(
            harness.bitbucket.lock().unwrap().created_with_secrets,
            vec![bitbucket_secret()]
        );
    }

    #[tokio::test]
    async fn lists_a_host_without_account_when_the_fetch_fails() {
        let harness = Harness::signed_in_to_github();
        harness.answer_github_account(Err(GitHostingError::Unauthorized));

        let signed_in = harness.service().list_accounts().await.unwrap();

        assert_eq!(
            signed_in,
            vec![SignedInHost {
                host: HostKind::Github,
                account: None
            }]
        );
        assert_eq!(harness.stored_account(HostKind::Github), None);
    }

    #[tokio::test]
    async fn omits_a_host_whose_secret_is_gone() {
        let harness = Harness::new();
        harness.store_account(&account(HostKind::Github, GITHUB_LOGIN));

        let signed_in = harness.service().list_accounts().await.unwrap();

        assert_eq!(signed_in, vec![]);
    }

    #[tokio::test]
    async fn reports_an_unavailable_keychain() {
        let harness = Harness::with_tokens(FakeHostTokens::unavailable());

        let result = harness.service().list_accounts().await;

        assert_eq!(result, Err(GitHostingError::Storage));
    }
}

mod connected_host {
    use super::*;

    #[test]
    fn a_host_without_a_secret_is_not_connected() {
        let harness = Harness::new();

        let result = harness.service().connected_host(HostKind::Github);

        assert!(matches!(result, Err(GitHostingError::NotConnected)));
    }
}

mod browser_sign_in {
    use super::*;

    #[tokio::test]
    async fn starting_returns_the_browser_step() {
        let harness = Harness::new();

        let step = harness
            .service()
            .start_browser_sign_in(HostKind::Github)
            .await;

        assert_eq!(step, Ok(browser_step()));
    }

    #[tokio::test]
    async fn bitbucket_has_no_browser_sign_in() {
        let harness = Harness::new();

        let result = harness
            .service()
            .start_browser_sign_in(HostKind::Bitbucket)
            .await;

        assert_eq!(
            result,
            Err(GitHostingError::invalid_input(InputField::Host))
        );
    }

    #[tokio::test]
    async fn an_authorized_poll_saves_the_token_and_the_account() {
        let harness = Harness::new();
        harness.answer_poll(ScriptedPoll::Authorized(GITHUB_TOKEN));

        let progress = harness
            .service()
            .poll_browser_sign_in(HostKind::Github, DEVICE_CODE)
            .await;

        assert_eq!(progress, Ok(SignInProgress::Authorized));
        assert_eq!(
            harness.host_tokens.secret(HostKind::Github).as_deref(),
            Some(GITHUB_TOKEN)
        );
        assert_eq!(
            harness.stored_account(HostKind::Github),
            Some(account(HostKind::Github, GITHUB_LOGIN))
        );
        assert_eq!(harness.events.changed_hosts(), vec![HostKind::Github]);
    }

    #[tokio::test]
    async fn an_authorized_poll_replaces_the_previous_account() {
        let harness = Harness::new();
        harness.store_account(&account(HostKind::Github, OTHER_GITHUB_LOGIN));
        harness.answer_poll(ScriptedPoll::Authorized(GITHUB_TOKEN));

        harness
            .service()
            .poll_browser_sign_in(HostKind::Github, DEVICE_CODE)
            .await
            .unwrap();

        assert_eq!(
            harness.stored_account(HostKind::Github),
            Some(account(HostKind::Github, GITHUB_LOGIN))
        );
    }

    #[tokio::test]
    async fn an_authorized_poll_keeps_the_token_when_the_account_fetch_fails() {
        let harness = Harness::new();
        harness.answer_poll(ScriptedPoll::Authorized(GITHUB_TOKEN));
        harness.answer_github_account(Err(GitHostingError::Network));

        let progress = harness
            .service()
            .poll_browser_sign_in(HostKind::Github, DEVICE_CODE)
            .await;

        assert_eq!(progress, Ok(SignInProgress::Authorized));
        assert!(harness.host_tokens.secret(HostKind::Github).is_some());
        assert_eq!(harness.stored_account(HostKind::Github), None);
        assert_eq!(harness.events.changed_hosts(), vec![HostKind::Github]);
    }

    #[tokio::test]
    async fn polls_with_the_trimmed_device_code() {
        let harness = Harness::new();

        harness
            .service()
            .poll_browser_sign_in(HostKind::Github, &format!(" {DEVICE_CODE} "))
            .await
            .unwrap();

        assert_eq!(
            harness.browser.lock().unwrap().polled_device_codes,
            vec![DEVICE_CODE.to_string()]
        );
    }

    #[tokio::test]
    async fn waiting_answers_save_nothing() {
        let answers = [
            (ScriptedPoll::Pending, SignInProgress::Pending),
            (
                ScriptedPoll::SlowDown(SLOWER_INTERVAL_SECONDS),
                SignInProgress::SlowDown {
                    interval_seconds: SLOWER_INTERVAL_SECONDS,
                },
            ),
            (ScriptedPoll::Expired, SignInProgress::Expired),
            (ScriptedPoll::Denied, SignInProgress::Denied),
        ];
        for (poll, expected) in answers {
            let harness = Harness::new();
            harness.answer_poll(poll);

            let progress = harness
                .service()
                .poll_browser_sign_in(HostKind::Github, DEVICE_CODE)
                .await;

            assert_eq!(progress, Ok(expected));
            assert_eq!(harness.host_tokens.secret(HostKind::Github), None);
            assert_eq!(harness.events.changed_hosts(), vec![]);
        }
    }

    #[tokio::test]
    async fn a_failed_poll_passes_the_error_through() {
        let harness = Harness::new();
        harness.answer_poll(ScriptedPoll::Fails(GitHostingError::Timeout));

        let result = harness
            .service()
            .poll_browser_sign_in(HostKind::Github, DEVICE_CODE)
            .await;

        assert_eq!(result, Err(GitHostingError::Timeout));
    }

    #[tokio::test]
    async fn refuses_an_empty_device_code() {
        let harness = Harness::new();

        let result = harness
            .service()
            .poll_browser_sign_in(HostKind::Github, BLANK)
            .await;

        assert_eq!(
            result,
            Err(GitHostingError::invalid_input(InputField::DeviceCode))
        );
    }
}

mod token_sign_in {
    use super::*;

    #[tokio::test]
    async fn saves_the_trimmed_credential_and_the_account() {
        let harness = Harness::new();

        let signed_in = harness
            .service()
            .sign_in_with_token(HostKind::Bitbucket, PADDED_USERNAME, PADDED_TOKEN)
            .await;

        let expected = account(HostKind::Bitbucket, BITBUCKET_LOGIN);
        assert_eq!(signed_in, Ok(expected.clone()));
        assert_eq!(
            harness.host_tokens.secret(HostKind::Bitbucket),
            Some(bitbucket_secret())
        );
        assert_eq!(harness.stored_account(HostKind::Bitbucket), Some(expected));
        assert_eq!(harness.events.changed_hosts(), vec![HostKind::Bitbucket]);
    }

    #[tokio::test]
    async fn checks_the_candidate_credential_with_the_current_account() {
        let harness = Harness::new();

        harness
            .service()
            .sign_in_with_token(HostKind::Bitbucket, BITBUCKET_USERNAME, BITBUCKET_TOKEN)
            .await
            .unwrap();

        let script = harness.bitbucket.lock().unwrap();
        assert_eq!(script.created_with_secrets, vec![bitbucket_secret()]);
        assert_eq!(script.account_fetches, 1);
    }

    #[tokio::test]
    async fn a_rejected_credential_saves_nothing() {
        let harness = Harness::new();
        harness.answer_bitbucket_account(Err(GitHostingError::Unauthorized));

        let result = harness
            .service()
            .sign_in_with_token(HostKind::Bitbucket, BITBUCKET_USERNAME, BITBUCKET_TOKEN)
            .await;

        assert_eq!(result, Err(GitHostingError::Unauthorized));
        assert_eq!(harness.host_tokens.secret(HostKind::Bitbucket), None);
        assert_eq!(harness.stored_account(HostKind::Bitbucket), None);
        assert_eq!(harness.events.changed_hosts(), vec![]);
    }

    #[tokio::test]
    async fn refuses_an_empty_username_before_checking() {
        let harness = Harness::new();

        let result = harness
            .service()
            .sign_in_with_token(HostKind::Bitbucket, BLANK, BITBUCKET_TOKEN)
            .await;

        assert_eq!(
            result,
            Err(GitHostingError::invalid_input(InputField::Username))
        );
        assert_eq!(harness.bitbucket_account_fetches(), 0);
    }

    #[tokio::test]
    async fn refuses_an_empty_token_before_checking() {
        let harness = Harness::new();

        let result = harness
            .service()
            .sign_in_with_token(HostKind::Bitbucket, BITBUCKET_USERNAME, BLANK)
            .await;

        assert_eq!(
            result,
            Err(GitHostingError::invalid_input(InputField::Token))
        );
        assert_eq!(harness.bitbucket_account_fetches(), 0);
    }

    #[tokio::test]
    async fn github_has_no_token_sign_in() {
        let harness = Harness::new();

        let result = harness
            .service()
            .sign_in_with_token(HostKind::Github, BITBUCKET_USERNAME, BITBUCKET_TOKEN)
            .await;

        assert_eq!(
            result,
            Err(GitHostingError::invalid_input(InputField::Host))
        );
    }
}

mod sign_out {
    use super::*;

    #[tokio::test]
    async fn deletes_the_token_and_the_account() {
        let harness = Harness::signed_in_to_github();
        harness.store_account(&account(HostKind::Github, GITHUB_LOGIN));

        harness.service().sign_out(HostKind::Github).await.unwrap();

        assert_eq!(harness.host_tokens.secret(HostKind::Github), None);
        assert_eq!(harness.stored_account(HostKind::Github), None);
        assert_eq!(harness.events.changed_hosts(), vec![HostKind::Github]);
    }

    #[tokio::test]
    async fn signing_out_twice_is_not_an_error() {
        let harness = Harness::signed_in_to_github();
        let service = harness.service();

        service.sign_out(HostKind::Github).await.unwrap();
        let second = service.sign_out(HostKind::Github).await;

        assert_eq!(second, Ok(()));
        assert_eq!(
            harness.events.changed_hosts(),
            vec![HostKind::Github, HostKind::Github]
        );
    }

    #[tokio::test]
    async fn leaves_the_other_host_signed_in() {
        let harness = Harness::signed_in_to_github();
        harness
            .host_tokens
            .save(
                HostKind::Bitbucket,
                &HostSecret::bitbucket(BITBUCKET_USERNAME, BITBUCKET_TOKEN),
            )
            .unwrap();

        harness.service().sign_out(HostKind::Github).await.unwrap();

        assert_eq!(
            harness.host_tokens.secret(HostKind::Bitbucket),
            Some(bitbucket_secret())
        );
    }
}
