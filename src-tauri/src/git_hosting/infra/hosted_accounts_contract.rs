//! Behaviour every `HostedAccounts` adapter shares. An adapter's tests run it
//! with `hosted_accounts_contract_tests!(with_accounts)`, where `with_accounts`
//! runs a test against a fresh, empty store.

use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::hosted_account::HostedAccount;
use crate::git_hosting::domain::ports::HostedAccounts;

const GITHUB_LOGIN: &str = "octocat";
const OTHER_GITHUB_LOGIN: &str = "hubot";
const BITBUCKET_LOGIN: &str = "alice";
const ACCOUNT_NAME: &str = "Alice Example";
const AVATAR_URL: &str = "https://avatars.example/alice.png";

fn account(host: HostKind, login: &str) -> HostedAccount {
    HostedAccount {
        host,
        login: login.to_string(),
        name: Some(ACCOUNT_NAME.to_string()),
        avatar_url: Some(AVATAR_URL.to_string()),
    }
}

fn account_without_details(host: HostKind, login: &str) -> HostedAccount {
    HostedAccount {
        name: None,
        avatar_url: None,
        ..account(host, login)
    }
}

pub fn lists_nothing_when_empty(accounts: &dyn HostedAccounts) {
    assert_eq!(accounts.list().unwrap(), vec![]);
}

pub fn saves_an_account(accounts: &dyn HostedAccounts) {
    let github = account(HostKind::Github, GITHUB_LOGIN);

    accounts.save(&github).unwrap();

    assert_eq!(accounts.list().unwrap(), vec![github]);
}

pub fn save_replaces_the_account_of_the_same_host(accounts: &dyn HostedAccounts) {
    accounts
        .save(&account(HostKind::Github, GITHUB_LOGIN))
        .unwrap();
    let replacement = account_without_details(HostKind::Github, OTHER_GITHUB_LOGIN);

    accounts.save(&replacement).unwrap();

    assert_eq!(accounts.list().unwrap(), vec![replacement]);
}

pub fn lists_accounts_ordered_by_host(accounts: &dyn HostedAccounts) {
    let github = account(HostKind::Github, GITHUB_LOGIN);
    let bitbucket = account(HostKind::Bitbucket, BITBUCKET_LOGIN);

    accounts.save(&github).unwrap();
    accounts.save(&bitbucket).unwrap();

    assert_eq!(accounts.list().unwrap(), vec![bitbucket, github]);
}

pub fn finds_the_account_of_a_host(accounts: &dyn HostedAccounts) {
    let github = account(HostKind::Github, GITHUB_LOGIN);
    accounts.save(&github).unwrap();
    accounts
        .save(&account(HostKind::Bitbucket, BITBUCKET_LOGIN))
        .unwrap();

    assert_eq!(accounts.find(HostKind::Github).unwrap(), Some(github));
}

pub fn finds_nothing_for_a_host_without_an_account(accounts: &dyn HostedAccounts) {
    accounts
        .save(&account(HostKind::Github, GITHUB_LOGIN))
        .unwrap();

    assert_eq!(accounts.find(HostKind::Bitbucket).unwrap(), None);
}

pub fn removes_the_account_of_a_host(accounts: &dyn HostedAccounts) {
    let bitbucket = account(HostKind::Bitbucket, BITBUCKET_LOGIN);
    accounts
        .save(&account(HostKind::Github, GITHUB_LOGIN))
        .unwrap();
    accounts.save(&bitbucket).unwrap();

    accounts.remove(HostKind::Github).unwrap();

    assert_eq!(accounts.list().unwrap(), vec![bitbucket]);
}

pub fn removing_a_missing_account_succeeds(accounts: &dyn HostedAccounts) {
    accounts.remove(HostKind::Github).unwrap();
    accounts.remove(HostKind::Github).unwrap();

    assert_eq!(accounts.find(HostKind::Github).unwrap(), None);
}

/// Generates one `#[test]` per contract behaviour, each run through
/// `$with_accounts: fn(fn(&dyn HostedAccounts))`.
macro_rules! hosted_accounts_contract_tests {
    ($with_accounts:ident) => {
        $crate::git_hosting::infra::hosted_accounts_contract::hosted_accounts_contract_tests!(
            @tests $with_accounts;
            lists_nothing_when_empty,
            saves_an_account,
            save_replaces_the_account_of_the_same_host,
            lists_accounts_ordered_by_host,
            finds_the_account_of_a_host,
            finds_nothing_for_a_host_without_an_account,
            removes_the_account_of_a_host,
            removing_a_missing_account_succeeds,
        );
    };
    (@tests $with_accounts:ident; $($behaviour:ident),* $(,)?) => {
        mod contract {
            $(
                #[test]
                fn $behaviour() {
                    super::$with_accounts(
                        $crate::git_hosting::infra::hosted_accounts_contract::$behaviour,
                    );
                }
            )*
        }
    };
}

pub(crate) use hosted_accounts_contract_tests;
