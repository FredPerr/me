//! The result of `suggest_pull_request` and the pure parser that reads it out
//! of the tool's answer.
//!
//! # Output contract
//!
//! The prompt (see [`pull_request_prompt`](super::pull_request_prompt)) asks
//! the tool to answer with a single `Title: <title>` line, a blank line, then
//! the Markdown description. The parser is deliberately lenient about that
//! shape so a slightly-off answer still yields a usable result:
//!
//!   - If the first non-blank line starts with `Title:` (case-insensitively),
//!     its remainder is the title and everything after it is the description.
//!   - Otherwise there is no explicit title: the whole answer is the
//!     description and the title falls back to the context name — exactly the
//!     field the user would have typed by hand.
//!
//! An answer that is blank once trimmed is [`None`]: the caller turns that into
//! the `EmptyAnswer` error.

use super::pull_request_prompt::TITLE_LINE_PREFIX;

/// A written pull request: both halves the frontend needs. The modal keeps a
/// title the user already typed and always fills the description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestedPullRequest {
    pub title: String,
    pub description: String,
}

/// Reads a [`SuggestedPullRequest`] out of the tool's answer, following the
/// output contract above. `context_name` is the title fallback for an answer
/// with no explicit `Title:` line. Returns [`None`] when the answer is blank.
pub fn parse_suggested_pull_request(
    answer: &str,
    context_name: &str,
) -> Option<SuggestedPullRequest> {
    let trimmed = answer.trim();
    if trimmed.is_empty() {
        return None;
    }

    match split_title_line(trimmed) {
        Some((title, description)) => Some(SuggestedPullRequest {
            // A `Title:` line with nothing after it is no better than no title.
            title: non_empty_or(title, context_name),
            description: description.to_string(),
        }),
        None => Some(SuggestedPullRequest {
            title: context_name.trim().to_string(),
            description: trimmed.to_string(),
        }),
    }
}

/// Splits a `Title: ...` first line from the rest, when the answer opens with
/// one. Returns the title text and the description (the remainder, with its
/// leading blank line trimmed). `None` when the first line is not a title line.
fn split_title_line(trimmed_answer: &str) -> Option<(&str, &str)> {
    let (first_line, rest) = match trimmed_answer.split_once('\n') {
        Some((first_line, rest)) => (first_line, rest),
        None => (trimmed_answer, ""),
    };
    let title = first_line
        .strip_prefix(TITLE_LINE_PREFIX)
        .or_else(|| strip_prefix_case_insensitive(first_line, TITLE_LINE_PREFIX))?
        .trim();
    Some((title, rest.trim_start()))
}

/// `line` without a leading `prefix` compared case-insensitively, or `None`.
fn strip_prefix_case_insensitive<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    if line.len() >= prefix.len() && line[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&line[prefix.len()..])
    } else {
        None
    }
}

fn non_empty_or(candidate: &str, fallback: &str) -> String {
    if candidate.is_empty() {
        fallback.trim().to_string()
    } else {
        candidate.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTEXT_NAME: &str = "Add login";

    #[test]
    fn reads_the_title_line_and_the_body_after_it() {
        let answer = "Title: Add the login page\n\n- Adds a login form\n- Wires it up";

        let parsed = parse_suggested_pull_request(answer, CONTEXT_NAME).unwrap();

        assert_eq!(parsed.title, "Add the login page");
        assert_eq!(parsed.description, "- Adds a login form\n- Wires it up");
    }

    #[test]
    fn matches_the_title_prefix_case_insensitively() {
        let answer = "title: Add login\n\nBody";

        let parsed = parse_suggested_pull_request(answer, CONTEXT_NAME).unwrap();

        assert_eq!(parsed.title, "Add login");
        assert_eq!(parsed.description, "Body");
    }

    #[test]
    fn falls_back_to_the_context_name_when_there_is_no_title_line() {
        let answer = "- Adds a login form\n- Wires it up";

        let parsed = parse_suggested_pull_request(answer, CONTEXT_NAME).unwrap();

        assert_eq!(parsed.title, CONTEXT_NAME);
        assert_eq!(parsed.description, "- Adds a login form\n- Wires it up");
    }

    #[test]
    fn falls_back_to_the_context_name_for_an_empty_title_line() {
        let answer = "Title:   \n\n- A change";

        let parsed = parse_suggested_pull_request(answer, CONTEXT_NAME).unwrap();

        assert_eq!(parsed.title, CONTEXT_NAME);
        assert_eq!(parsed.description, "- A change");
    }

    #[test]
    fn trims_surrounding_whitespace_before_parsing() {
        let answer = "\n\n  Title: Add login\n\nBody\n\n";

        let parsed = parse_suggested_pull_request(answer, CONTEXT_NAME).unwrap();

        assert_eq!(parsed.title, "Add login");
        assert_eq!(parsed.description, "Body");
    }

    #[test]
    fn a_title_only_answer_keeps_an_empty_description() {
        let answer = "Title: Add login";

        let parsed = parse_suggested_pull_request(answer, CONTEXT_NAME).unwrap();

        assert_eq!(parsed.title, "Add login");
        assert_eq!(parsed.description, "");
    }

    #[test]
    fn a_blank_answer_is_none() {
        assert_eq!(parse_suggested_pull_request("   \n  ", CONTEXT_NAME), None);
    }
}
