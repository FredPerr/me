use super::error::{GitHostingError, InputField};

/// A pull request to open from `head` into `base`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPullRequest {
    pub title: String,
    pub body: String,
    pub head: String,
    pub base: String,
    pub draft: bool,
}

impl NewPullRequest {
    /// The same pull request with its title trimmed. The title must not be
    /// blank, and the base branch must be another branch than the head.
    pub fn validated(self) -> Result<Self, GitHostingError> {
        let title = self.title.trim();
        if title.is_empty() {
            return Err(GitHostingError::invalid_input(InputField::Title));
        }
        let base = self.base.trim();
        if base.is_empty() || base == self.head.trim() {
            return Err(GitHostingError::invalid_input(InputField::BaseBranch));
        }
        Ok(Self {
            title: title.to_string(),
            ..self
        })
    }
}

/// The pull request a Git host opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedPullRequest {
    pub number: u32,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TITLE: &str = "Add the login page";
    const PADDED_TITLE: &str = "  Add the login page  ";
    const BLANK: &str = "   ";
    const HEAD: &str = "feature/login";
    const BASE: &str = "main";

    fn new_pull_request(title: &str, head: &str, base: &str) -> NewPullRequest {
        NewPullRequest {
            title: title.to_string(),
            body: String::new(),
            head: head.to_string(),
            base: base.to_string(),
            draft: false,
        }
    }

    #[test]
    fn trims_the_title() {
        let validated = new_pull_request(PADDED_TITLE, HEAD, BASE).validated();

        assert_eq!(validated, Ok(new_pull_request(TITLE, HEAD, BASE)));
    }

    #[test]
    fn a_blank_title_is_refused() {
        assert_eq!(
            new_pull_request(BLANK, HEAD, BASE).validated(),
            Err(GitHostingError::invalid_input(InputField::Title))
        );
    }

    #[test]
    fn merging_a_branch_into_itself_is_refused() {
        assert_eq!(
            new_pull_request(TITLE, HEAD, HEAD).validated(),
            Err(GitHostingError::invalid_input(InputField::BaseBranch))
        );
    }

    #[test]
    fn a_blank_base_branch_is_refused() {
        assert_eq!(
            new_pull_request(TITLE, HEAD, BLANK).validated(),
            Err(GitHostingError::invalid_input(InputField::BaseBranch))
        );
    }
}
