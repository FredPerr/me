use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
}

impl Label {
    /// Keeps `color` only when it matches `^#[0-9a-fA-F]{3,8}$`, because the UI
    /// uses it as a CSS colour.
    pub fn new(name: String, color: Option<String>) -> Self {
        Self {
            name,
            color: color.filter(|value| is_hex_color(value)),
        }
    }
}

fn is_hex_color(value: &str) -> bool {
    value.strip_prefix('#').is_some_and(|digits| {
        (3..=8).contains(&digits.len())
            && digits
                .chars()
                .all(|character| character.is_ascii_hexdigit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_hex_colors() {
        for valid in ["#abc", "#A1B2C3", "#12345678"] {
            assert_eq!(
                Label::new("x".into(), Some(valid.into())).color.as_deref(),
                Some(valid)
            );
        }
        for invalid in ["abc", "#ab", "#123456789", "#ggg", "red", "#abc;x", ""] {
            assert_eq!(
                Label::new("x".into(), Some(invalid.into())).color,
                None,
                "{invalid}"
            );
        }
    }
}
