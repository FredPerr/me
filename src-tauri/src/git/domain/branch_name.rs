use std::fmt;

use serde::Serialize;

use super::error::GitError;

const FORBIDDEN_CHARACTERS: &[char] = &[' ', '~', '^', ':', '?', '*', '[', '\\'];
const LOCK_SUFFIX: &str = ".lock";

/// A local branch name that git accepts, following `git check-ref-format --branch`.
/// Project naming rules (prefixes, names from task titles) are not checked here.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct BranchName(String);

impl BranchName {
    pub fn parse(raw: &str) -> Result<Self, GitError> {
        match violation(raw) {
            Some(reason) => Err(GitError::InvalidBranchName {
                name: raw.to_string(),
                reason,
            }),
            None => Ok(Self(raw.to_string())),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BranchName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn violation(raw: &str) -> Option<&'static str> {
    if raw.is_empty() {
        return Some("is empty");
    }
    if raw == "@" {
        return Some("is \"@\"");
    }
    if raw.starts_with('-') {
        return Some("starts with \"-\"");
    }
    if raw.starts_with('/') || raw.ends_with('/') {
        return Some("starts or ends with \"/\"");
    }
    if raw.ends_with('.') {
        return Some("ends with \".\"");
    }
    if raw.contains("..") {
        return Some("contains \"..\"");
    }
    if raw.contains("//") {
        return Some("contains \"//\"");
    }
    if raw.contains("@{") {
        return Some("contains \"@{\"");
    }
    if raw.chars().any(|c| c.is_ascii_control()) {
        return Some("contains a control character");
    }
    if raw.contains(FORBIDDEN_CHARACTERS) {
        return Some("contains a space or one of ~ ^ : ? * [ \\");
    }
    let bad_component = raw
        .split('/')
        .any(|component| component.starts_with('.') || component.ends_with(LOCK_SUFFIX));
    if bad_component {
        return Some("has a part starting with \".\" or ending with \".lock\"");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_valid(raw: &str) -> bool {
        BranchName::parse(raw).is_ok()
    }

    #[test]
    fn accepts_plain_and_prefixed_names() {
        assert!(is_valid("main"));
        assert!(is_valid("feature/login-page"));
        assert!(is_valid("fix/JIRA-12_v2"));
        assert!(is_valid("origin/develop"));
    }

    #[test]
    fn rejects_empty_and_reserved_names() {
        assert!(!is_valid(""));
        assert!(!is_valid("@"));
        assert!(!is_valid("-starts-with-dash"));
    }

    #[test]
    fn rejects_bad_slashes_and_dots() {
        assert!(!is_valid("/feature"));
        assert!(!is_valid("feature/"));
        assert!(!is_valid("feature//login"));
        assert!(!is_valid("feature..login"));
        assert!(!is_valid("feature."));
        assert!(!is_valid("feature/.hidden"));
        assert!(!is_valid("feature/login.lock"));
    }

    #[test]
    fn rejects_forbidden_characters() {
        for raw in [
            "has space",
            "tilde~",
            "caret^",
            "colon:",
            "question?",
            "star*",
            "bracket[",
            "back\\slash",
            "reflog@{1}",
            "tab\tname",
        ] {
            assert!(!is_valid(raw), "{raw} should be rejected");
        }
    }

    #[test]
    fn reports_the_name_and_reason() {
        let error = BranchName::parse("feature..login").unwrap_err();
        assert_eq!(
            error,
            GitError::InvalidBranchName {
                name: "feature..login".to_string(),
                reason: "contains \"..\""
            }
        );
    }
}
