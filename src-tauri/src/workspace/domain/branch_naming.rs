use std::fmt;

use serde::{Deserialize, Serialize};

use crate::git::domain::branch_name::BranchName;
use crate::git::domain::error::GitError;

use super::error::{PrefixRejection, WorkspaceError};
use super::slug::slugify;

pub const DEFAULT_BRANCH_PREFIXES: [&str; 4] = ["feature", "fix", "chore", "refactor"];

const FORBIDDEN_PREFIX_SEQUENCES: [&str; 3] = ["..", "@{", "//"];
const FORBIDDEN_PREFIX_CHARACTERS: [char; 7] = ['~', '^', ':', '?', '*', '[', '\\'];
const LOCK_SUFFIX: &str = ".lock";

/// The part of a branch name before the slash, such as "feature" in
/// "feature/login-page". Surrounding spaces and slashes are dropped.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct BranchPrefix(String);

impl BranchPrefix {
    pub fn parse(raw: &str) -> Result<Self, WorkspaceError> {
        let prefix = raw.trim().trim_matches('/');
        let reason = if prefix.is_empty() {
            Some(PrefixRejection::Empty)
        } else if is_invalid_prefix(prefix) {
            Some(PrefixRejection::Invalid)
        } else {
            None
        };
        match reason {
            Some(reason) => Err(WorkspaceError::InvalidPrefix {
                prefix: raw.to_string(),
                reason,
            }),
            None => Ok(Self(prefix.to_string())),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BranchPrefix {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn is_invalid_prefix(prefix: &str) -> bool {
    let has_forbidden_character = prefix.chars().any(|character| {
        character <= ' '
            || character == '\u{7f}'
            || FORBIDDEN_PREFIX_CHARACTERS.contains(&character)
    });
    let has_forbidden_sequence = FORBIDDEN_PREFIX_SEQUENCES
        .iter()
        .any(|sequence| prefix.contains(sequence));
    let has_bad_segment = prefix
        .split('/')
        .any(|segment| segment.starts_with('.') || segment.ends_with(LOCK_SUFFIX));
    has_forbidden_character
        || has_forbidden_sequence
        || prefix == "@"
        || prefix.ends_with('.')
        || has_bad_segment
}

/// How a project names its branches: the prefixes it offers and whether a
/// branch may have none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "BranchNamingSettings", into = "BranchNamingSettings")]
pub struct BranchNamingPolicy {
    prefixes: Vec<BranchPrefix>,
    allow_no_prefix: bool,
}

impl BranchNamingPolicy {
    /// Keeps the valid prefixes, normalized, without duplicates, in order.
    pub fn new<S: AsRef<str>>(raw_prefixes: &[S], allow_no_prefix: bool) -> Self {
        let mut prefixes: Vec<BranchPrefix> = Vec::new();
        for raw_prefix in raw_prefixes {
            if let Ok(prefix) = BranchPrefix::parse(raw_prefix.as_ref()) {
                if !prefixes.contains(&prefix) {
                    prefixes.push(prefix);
                }
            }
        }
        Self {
            prefixes,
            allow_no_prefix,
        }
    }

    /// The policy the settings form asks for: blank prefixes are dropped and
    /// duplicates merged, but an invalid prefix is refused, and so is
    /// requiring a prefix without offering one.
    pub fn parse<S: AsRef<str>>(
        raw_prefixes: &[S],
        allow_no_prefix: bool,
    ) -> Result<Self, WorkspaceError> {
        let mut prefixes: Vec<BranchPrefix> = Vec::new();
        for raw_prefix in raw_prefixes {
            let prefix = match BranchPrefix::parse(raw_prefix.as_ref()) {
                Ok(prefix) => prefix,
                Err(WorkspaceError::InvalidPrefix {
                    reason: PrefixRejection::Empty,
                    ..
                }) => continue,
                Err(error) => return Err(error),
            };
            if !prefixes.contains(&prefix) {
                prefixes.push(prefix);
            }
        }
        if !allow_no_prefix && prefixes.is_empty() {
            return Err(WorkspaceError::PrefixRequired);
        }
        Ok(Self {
            prefixes,
            allow_no_prefix,
        })
    }

    pub fn prefixes(&self) -> &[BranchPrefix] {
        &self.prefixes
    }

    /// A policy without prefixes always allows a branch without one.
    pub fn allows_no_prefix(&self) -> bool {
        self.allow_no_prefix || self.prefixes.is_empty()
    }

    pub fn default_prefix(&self) -> Option<&BranchPrefix> {
        if self.allows_no_prefix() {
            None
        } else {
            self.prefixes.first()
        }
    }

    /// The branch name for a task title or context name: its slug, after the
    /// chosen prefix.
    pub fn compose_branch_name(
        &self,
        title: &str,
        prefix: Option<&str>,
    ) -> Result<BranchName, WorkspaceError> {
        match prefix {
            None if !self.allows_no_prefix() => return Err(WorkspaceError::PrefixRequired),
            Some(prefix) if !self.prefixes.iter().any(|known| known.as_str() == prefix) => {
                return Err(WorkspaceError::UnknownPrefix {
                    prefix: prefix.to_string(),
                })
            }
            _ => {}
        }
        let slug = slugify(title);
        if slug.is_empty() {
            return Err(WorkspaceError::EmptyBranchName {
                source: title.to_string(),
            });
        }
        let name = match prefix {
            Some(prefix) => format!("{prefix}/{slug}"),
            None => slug,
        };
        parse_branch_name(&name)
    }

    /// The branch name offered by default: composed with the default prefix.
    pub fn suggest_branch_name(&self, title: &str) -> Result<BranchName, WorkspaceError> {
        let prefix = self.default_prefix().map(BranchPrefix::as_str);
        self.compose_branch_name(title, prefix)
    }
}

impl Default for BranchNamingPolicy {
    fn default() -> Self {
        Self::new(&DEFAULT_BRANCH_PREFIXES, true)
    }
}

/// A branch name that git accepts, with the error in the workspace's terms.
pub fn parse_branch_name(raw: &str) -> Result<BranchName, WorkspaceError> {
    BranchName::parse(raw).map_err(|error| match error {
        GitError::InvalidBranchName { name, reason } => {
            WorkspaceError::InvalidBranchName { name, reason }
        }
        _ => WorkspaceError::InvalidBranchName {
            name: raw.to_string(),
            reason: "is not accepted by git",
        },
    })
}

/// The saved shape of the policy. Missing values fall back to the defaults,
/// and prefixes are normalized again when read.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BranchNamingSettings {
    #[serde(default = "default_prefixes")]
    prefixes: Vec<String>,
    #[serde(default = "allow_no_prefix_by_default")]
    allow_no_prefix: bool,
}

fn default_prefixes() -> Vec<String> {
    DEFAULT_BRANCH_PREFIXES.map(String::from).to_vec()
}

fn allow_no_prefix_by_default() -> bool {
    true
}

impl From<BranchNamingSettings> for BranchNamingPolicy {
    fn from(settings: BranchNamingSettings) -> Self {
        Self::new(&settings.prefixes, settings.allow_no_prefix)
    }
}

impl From<BranchNamingPolicy> for BranchNamingSettings {
    fn from(policy: BranchNamingPolicy) -> Self {
        Self {
            prefixes: policy.prefixes.into_iter().map(|prefix| prefix.0).collect(),
            allow_no_prefix: policy.allow_no_prefix,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TASK_NAME: &str = "[WAFR] Do something here";
    const TASK_SLUG: &str = "wafr-do-something-here";

    fn prefix_names(policy: &BranchNamingPolicy) -> Vec<&str> {
        policy.prefixes().iter().map(BranchPrefix::as_str).collect()
    }

    fn rejection(raw: &str) -> Option<PrefixRejection> {
        match BranchPrefix::parse(raw) {
            Err(WorkspaceError::InvalidPrefix { reason, .. }) => Some(reason),
            _ => None,
        }
    }

    fn read_policy(json: &str) -> BranchNamingPolicy {
        serde_json::from_str(json).unwrap()
    }

    mod parse_prefix {
        use super::*;

        #[test]
        fn normalizes_surrounding_spaces_and_slashes() {
            let cases = [
                ("feature/", "feature"),
                (" chore// ", "chore"),
                ("team/feature", "team/feature"),
                ("Feature", "Feature"),
            ];
            for (raw, expected) in cases {
                assert_eq!(
                    BranchPrefix::parse(raw).unwrap().as_str(),
                    expected,
                    "{raw:?}"
                );
            }
        }

        #[test]
        fn rejects_blank_prefixes_as_empty() {
            for raw in ["", "  ", "/"] {
                assert_eq!(rejection(raw), Some(PrefixRejection::Empty), "{raw:?}");
            }
        }

        #[test]
        fn rejects_prefixes_git_refuses_as_invalid() {
            let invalid = [
                "my prefix",
                "a..b",
                "a~b",
                "a^b",
                "a:b",
                "a?b",
                "a*b",
                "a[b",
                "a\\b",
                "@",
                "a@{b",
                ".hidden",
                "x.lock",
                "x.",
                "a//b",
            ];
            for raw in invalid {
                assert_eq!(rejection(raw), Some(PrefixRejection::Invalid), "{raw:?}");
            }
        }
    }

    mod compose_branch_name {
        use super::*;

        #[test]
        fn returns_the_slug_when_no_prefix_is_chosen() {
            let branch = BranchNamingPolicy::default()
                .compose_branch_name(TASK_NAME, None)
                .unwrap();
            assert_eq!(branch.as_str(), TASK_SLUG);
        }

        #[test]
        fn prepends_the_chosen_prefix() {
            let policy = BranchNamingPolicy::default();
            for prefix in ["feature", "fix"] {
                let branch = policy.compose_branch_name(TASK_NAME, Some(prefix)).unwrap();
                assert_eq!(branch.as_str(), format!("{prefix}/{TASK_SLUG}"));
            }
        }

        #[test]
        fn refuses_a_title_without_letters_or_digits() {
            let result =
                BranchNamingPolicy::default().compose_branch_name("  !!!  ", Some("feature"));
            assert_eq!(
                result,
                Err(WorkspaceError::EmptyBranchName {
                    source: "  !!!  ".to_string()
                })
            );
        }

        #[test]
        fn refuses_no_prefix_when_a_prefix_is_required() {
            let policy = BranchNamingPolicy::new(&["feature"], false);
            assert_eq!(
                policy.compose_branch_name(TASK_NAME, None),
                Err(WorkspaceError::PrefixRequired)
            );
        }

        #[test]
        fn refuses_a_prefix_that_is_not_configured() {
            let result =
                BranchNamingPolicy::default().compose_branch_name(TASK_NAME, Some("hotfix"));
            assert_eq!(
                result,
                Err(WorkspaceError::UnknownPrefix {
                    prefix: "hotfix".to_string()
                })
            );
        }

        #[test]
        fn suggests_a_name_with_the_default_prefix() {
            let policy = BranchNamingPolicy::new(&["fix", "feature"], false);
            let branch = policy.suggest_branch_name(TASK_NAME).unwrap();
            assert_eq!(branch.as_str(), format!("fix/{TASK_SLUG}"));
        }
    }

    #[test]
    fn new_drops_invalid_and_empty_prefixes_normalizes_and_dedupes_keeping_order() {
        let policy = BranchNamingPolicy::new(
            &["fix/", "", "my prefix", "feature", "feature/", " chore "],
            true,
        );
        assert_eq!(prefix_names(&policy), ["fix", "feature", "chore"]);
    }

    mod parse_policy {
        use super::*;

        #[test]
        fn drops_blank_prefixes_and_merges_duplicates_keeping_order() {
            let policy = BranchNamingPolicy::parse(&["fix/", " ", "feature", "fix"], true).unwrap();
            assert_eq!(prefix_names(&policy), ["fix", "feature"]);
        }

        #[test]
        fn refuses_an_invalid_prefix() {
            assert_eq!(
                BranchNamingPolicy::parse(&["feature", "my prefix"], true),
                Err(WorkspaceError::InvalidPrefix {
                    prefix: "my prefix".to_string(),
                    reason: PrefixRejection::Invalid,
                })
            );
        }

        #[test]
        fn refuses_requiring_a_prefix_without_offering_one() {
            assert_eq!(
                BranchNamingPolicy::parse(&[""], false),
                Err(WorkspaceError::PrefixRequired)
            );
        }
    }

    mod default_prefix {
        use super::*;

        #[test]
        fn is_none_when_no_prefix_is_allowed() {
            let policy = BranchNamingPolicy::new(&["feature"], true);
            assert!(policy.allows_no_prefix());
            assert_eq!(policy.default_prefix(), None);
        }

        #[test]
        fn is_the_first_prefix_when_a_prefix_is_required() {
            let policy = BranchNamingPolicy::new(&["fix", "feature"], false);
            assert!(!policy.allows_no_prefix());
            assert_eq!(
                policy.default_prefix().map(BranchPrefix::as_str),
                Some("fix")
            );
        }

        #[test]
        fn no_prefix_is_allowed_when_the_list_is_empty_even_if_disallowed() {
            let policy = BranchNamingPolicy::new::<&str>(&[], false);
            assert!(policy.allows_no_prefix());
            assert_eq!(policy.default_prefix(), None);
        }
    }

    mod serialization {
        use super::*;

        #[test]
        fn uses_the_defaults_when_nothing_is_saved() {
            let policy = read_policy("{}");
            assert_eq!(prefix_names(&policy), DEFAULT_BRANCH_PREFIXES);
            assert!(policy.allows_no_prefix());
        }

        #[test]
        fn keeps_the_default_prefixes_when_only_allow_no_prefix_is_saved() {
            let policy = read_policy(r#"{"allowNoPrefix":false}"#);
            assert_eq!(prefix_names(&policy), DEFAULT_BRANCH_PREFIXES);
            assert!(!policy.allows_no_prefix());
        }

        #[test]
        fn keeps_custom_prefixes_and_allows_no_prefix_by_default() {
            let policy = read_policy(r#"{"prefixes":["ops"]}"#);
            assert_eq!(prefix_names(&policy), ["ops"]);
            assert!(policy.allows_no_prefix());
        }

        #[test]
        fn keeps_an_empty_prefix_list() {
            assert!(read_policy(r#"{"prefixes":[]}"#).prefixes().is_empty());
        }

        #[test]
        fn round_trips_keeping_the_raw_allow_no_prefix_setting() {
            let saved = serde_json::json!({ "prefixes": [], "allowNoPrefix": false });
            let policy: BranchNamingPolicy = serde_json::from_value(saved.clone()).unwrap();
            assert_eq!(serde_json::to_value(policy).unwrap(), saved);
        }

        #[test]
        fn normalizes_prefixes_when_read() {
            let policy = read_policy(r#"{"prefixes":["fix/","my prefix"]}"#);
            assert_eq!(prefix_names(&policy), ["fix"]);
        }
    }
}
