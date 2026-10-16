use unicode_normalization::UnicodeNormalization;

/// Lowercase ASCII letters and digits joined by single hyphens, accents
/// removed: "Réparer l'été" becomes "reparer-l-ete". Same rules as the
/// frontend's former `slugify`.
pub fn slugify(input: &str) -> String {
    let lowercased: String = input
        .nfd()
        .filter(|character| !is_diacritic(*character))
        .collect::<String>()
        .to_lowercase();
    let mut slug = String::with_capacity(lowercased.len());
    let mut pending_hyphen = false;
    for character in lowercased.chars() {
        if character.is_ascii_lowercase() || character.is_ascii_digit() {
            if pending_hyphen && !slug.is_empty() {
                slug.push('-');
            }
            pending_hyphen = false;
            slug.push(character);
        } else {
            pending_hyphen = true;
        }
    }
    slug
}

/// The Combining Diacritical Marks block (U+0300–U+036F), the only marks the
/// former frontend rule removed.
fn is_diacritic(character: char) -> bool {
    ('\u{0300}'..='\u{036f}').contains(&character)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_a_string_to_lowercase() {
        assert_eq!(slugify("HELLO"), "hello");
    }

    #[test]
    fn replaces_spaces_with_a_single_hyphen() {
        assert_eq!(slugify("hello world"), "hello-world");
    }

    #[test]
    fn collapses_multiple_consecutive_spaces_into_a_single_hyphen() {
        assert_eq!(slugify("hello   world"), "hello-world");
    }

    #[test]
    fn trims_leading_and_trailing_whitespace() {
        assert_eq!(slugify("  hello world  "), "hello-world");
    }

    #[test]
    fn removes_diacritics() {
        assert_eq!(slugify("Café"), "cafe");
    }

    #[test]
    fn replaces_punctuation_with_a_hyphen() {
        assert_eq!(slugify("file_name (v2).PDF"), "file-name-v2-pdf");
    }

    #[test]
    fn removes_leading_and_trailing_hyphens_produced_by_symbols() {
        assert_eq!(slugify("--hello world--"), "hello-world");
    }

    #[test]
    fn preserves_numbers() {
        assert_eq!(slugify("Item 42"), "item-42");
    }

    #[test]
    fn returns_an_empty_slug_for_whitespace_only_input() {
        assert_eq!(slugify("   "), "");
    }

    #[test]
    fn turns_a_bracketed_task_name_into_a_branch_slug() {
        assert_eq!(
            slugify("[WAFR] Do something here"),
            "wafr-do-something-here"
        );
    }

    #[test]
    fn strips_diacritics_and_apostrophes_from_task_names() {
        assert_eq!(slugify("Réparer l'été"), "reparer-l-ete");
    }

    #[test]
    fn returns_an_empty_slug_for_empty_input() {
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn replaces_letters_outside_ascii_with_a_hyphen() {
        assert_eq!(slugify("straße 東京 test"), "stra-e-test");
    }
}
