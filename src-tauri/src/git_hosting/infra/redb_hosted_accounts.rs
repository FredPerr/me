use std::fmt::Display;
use std::sync::Arc;

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::hosted_account::HostedAccount;
use crate::git_hosting::domain::ports::{HostedAccounts, HostedAccountsUnavailable};

/// One record per Git host, keyed by its identifier, holding the account as
/// MessagePack with field names so added fields still read from older records.
const ACCOUNTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("git_hosting_accounts");

fn unavailable(error: impl Display) -> HostedAccountsUnavailable {
    eprintln!("[git_hosting] account storage failed: {error}");
    HostedAccountsUnavailable
}

/// An unreadable record reads as no account, so it is fetched again and replaced.
fn decode(key: &str, bytes: &[u8]) -> Option<HostedAccount> {
    match rmp_serde::from_slice(bytes) {
        Ok(account) => Some(account),
        Err(error) => {
            eprintln!("[git_hosting] account record '{key}' unreadable: {error}");
            None
        }
    }
}

/// Account summaries in the shared redb database.
pub struct RedbHostedAccounts {
    database: Arc<Database>,
}

impl RedbHostedAccounts {
    /// Uses the shared database, creating its `git_hosting_accounts` table when missing.
    pub fn new(database: Arc<Database>) -> Result<Self, HostedAccountsUnavailable> {
        let transaction = database.begin_write().map_err(unavailable)?;
        transaction
            .open_table(ACCOUNTS_TABLE)
            .map_err(unavailable)?;
        transaction.commit().map_err(unavailable)?;
        Ok(Self { database })
    }

    fn write(
        &self,
        apply: impl FnOnce(&mut redb::Table<&str, &[u8]>) -> Result<(), HostedAccountsUnavailable>,
    ) -> Result<(), HostedAccountsUnavailable> {
        let transaction = self.database.begin_write().map_err(unavailable)?;
        {
            let mut table = transaction
                .open_table(ACCOUNTS_TABLE)
                .map_err(unavailable)?;
            apply(&mut table)?;
        }
        transaction.commit().map_err(unavailable)
    }
}

impl HostedAccounts for RedbHostedAccounts {
    fn list(&self) -> Result<Vec<HostedAccount>, HostedAccountsUnavailable> {
        let transaction = self.database.begin_read().map_err(unavailable)?;
        let table = transaction
            .open_table(ACCOUNTS_TABLE)
            .map_err(unavailable)?;
        let mut accounts = Vec::new();
        for entry in table.iter().map_err(unavailable)? {
            let (key, value) = entry.map_err(unavailable)?;
            accounts.extend(decode(key.value(), value.value()));
        }
        Ok(accounts)
    }

    fn find(&self, host: HostKind) -> Result<Option<HostedAccount>, HostedAccountsUnavailable> {
        let transaction = self.database.begin_read().map_err(unavailable)?;
        let table = transaction
            .open_table(ACCOUNTS_TABLE)
            .map_err(unavailable)?;
        let record = table.get(host.as_str()).map_err(unavailable)?;
        Ok(record.and_then(|record| decode(host.as_str(), record.value())))
    }

    fn save(&self, account: &HostedAccount) -> Result<(), HostedAccountsUnavailable> {
        let record = rmp_serde::to_vec_named(account).map_err(unavailable)?;
        self.write(|table| {
            table
                .insert(account.host.as_str(), record.as_slice())
                .map_err(unavailable)?;
            Ok(())
        })
    }

    fn remove(&self, host: HostKind) -> Result<(), HostedAccountsUnavailable> {
        self.write(|table| {
            table.remove(host.as_str()).map_err(unavailable)?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::git_hosting::infra::hosted_accounts_contract::hosted_accounts_contract_tests;
    use crate::storage;

    const LOGIN: &str = "octocat";
    const CORRUPT_BYTES: &[u8] = b"not an account";

    fn accounts_in(directory: &TempDir) -> RedbHostedAccounts {
        RedbHostedAccounts::new(storage::open(directory.path()).unwrap()).unwrap()
    }

    fn with_accounts(test: fn(&dyn HostedAccounts)) {
        let directory = tempfile::tempdir().unwrap();
        test(&accounts_in(&directory));
    }

    hosted_accounts_contract_tests!(with_accounts);

    fn github_account() -> HostedAccount {
        HostedAccount {
            host: HostKind::Github,
            login: LOGIN.to_string(),
            name: None,
            avatar_url: None,
        }
    }

    fn write_raw_record(accounts: &RedbHostedAccounts, key: &str, bytes: &[u8]) {
        accounts
            .write(|table| {
                table.insert(key, bytes).map_err(unavailable)?;
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn reopening_the_database_keeps_the_accounts() {
        let directory = tempfile::tempdir().unwrap();
        accounts_in(&directory).save(&github_account()).unwrap();

        let reopened = accounts_in(&directory);

        assert_eq!(
            reopened.find(HostKind::Github).unwrap(),
            Some(github_account())
        );
    }

    #[test]
    fn stores_the_account_with_field_names() {
        let directory = tempfile::tempdir().unwrap();
        let accounts = accounts_in(&directory);
        let record = rmp_serde::to_vec_named(&json!({ "host": "github", "login": LOGIN })).unwrap();

        write_raw_record(&accounts, HostKind::Github.as_str(), &record);

        assert_eq!(
            accounts.find(HostKind::Github).unwrap(),
            Some(github_account())
        );
    }

    #[test]
    fn an_unreadable_record_reads_as_no_account() {
        let directory = tempfile::tempdir().unwrap();
        let accounts = accounts_in(&directory);

        write_raw_record(&accounts, HostKind::Github.as_str(), CORRUPT_BYTES);

        assert_eq!(accounts.find(HostKind::Github).unwrap(), None);
        assert_eq!(accounts.list().unwrap(), vec![]);
    }
}
