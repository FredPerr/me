use serde::{Deserialize, Serialize};

use super::host_kind::HostKind;

/// The account signed in to a Git host, at most one per host. Stored with
/// field names so added fields still read from older records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedAccount {
    pub host: HostKind,
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

/// A Git host with a stored secret, and its account when it could be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedInHost {
    pub host: HostKind,
    pub account: Option<HostedAccount>,
}
