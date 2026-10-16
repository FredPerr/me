use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::hosted_account::HostedAccount;
use crate::git_hosting::domain::ports::{HostedAccounts, HostedAccountsUnavailable};

/// Accounts kept in memory, ordered by Git host identifier like redb.
#[derive(Default)]
pub struct InMemoryHostedAccounts {
    accounts: Mutex<BTreeMap<&'static str, HostedAccount>>,
}

impl InMemoryHostedAccounts {
    fn accounts(&self) -> MutexGuard<'_, BTreeMap<&'static str, HostedAccount>> {
        // Every write replaces whole accounts, so a poisoned map is still consistent.
        self.accounts.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl HostedAccounts for InMemoryHostedAccounts {
    fn list(&self) -> Result<Vec<HostedAccount>, HostedAccountsUnavailable> {
        Ok(self.accounts().values().cloned().collect())
    }

    fn find(&self, host: HostKind) -> Result<Option<HostedAccount>, HostedAccountsUnavailable> {
        Ok(self.accounts().get(host.as_str()).cloned())
    }

    fn save(&self, account: &HostedAccount) -> Result<(), HostedAccountsUnavailable> {
        self.accounts()
            .insert(account.host.as_str(), account.clone());
        Ok(())
    }

    fn remove(&self, host: HostKind) -> Result<(), HostedAccountsUnavailable> {
        self.accounts().remove(host.as_str());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_hosting::infra::hosted_accounts_contract::hosted_accounts_contract_tests;

    fn with_accounts(test: fn(&dyn HostedAccounts)) {
        test(&InMemoryHostedAccounts::default());
    }

    hosted_accounts_contract_tests!(with_accounts);
}
