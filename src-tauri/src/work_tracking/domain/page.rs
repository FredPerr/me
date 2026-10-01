use serde::Serialize;

use super::error::{InputField, InvalidInputReason, WorkTrackingError};

pub const DEFAULT_PAGE_SIZE: u32 = 50;
pub const MAX_PAGE_SIZE: u32 = 100;
const MAX_CURSOR_LENGTH: usize = 64;

/// Opaque pagination token; its meaning belongs to the provider adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Cursor(String);

impl Cursor {
    /// 1–64 visible ASCII characters.
    pub fn parse(raw: &str) -> Result<Self, WorkTrackingError> {
        let is_valid = (1..=MAX_CURSOR_LENGTH).contains(&raw.len())
            && raw.bytes().all(|byte| byte.is_ascii_graphic());
        if !is_valid {
            return Err(WorkTrackingError::invalid_input(
                InputField::Cursor,
                InvalidInputReason::InvalidFormat,
            ));
        }
        Ok(Self(raw.to_string()))
    }

    pub fn from_numeric(value: u64) -> Self {
        Self(value.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRequest {
    cursor: Option<Cursor>,
    page_size: u32,
}

impl PageRequest {
    pub fn new(cursor: Option<Cursor>, page_size: u32) -> Result<Self, WorkTrackingError> {
        if !(1..=MAX_PAGE_SIZE).contains(&page_size) {
            return Err(WorkTrackingError::invalid_input(
                InputField::PageSize,
                InvalidInputReason::OutOfRange,
            ));
        }
        Ok(Self { cursor, page_size })
    }

    pub fn cursor(&self) -> Option<&Cursor> {
        self.cursor.as_ref()
    }

    pub fn page_size(&self) -> u32 {
        self.page_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_request_bounds() {
        let out_of_range = Err(WorkTrackingError::invalid_input(
            InputField::PageSize,
            InvalidInputReason::OutOfRange,
        ));
        assert_eq!(PageRequest::new(None, 0), out_of_range);
        assert_eq!(PageRequest::new(None, 1).unwrap().page_size(), 1);
        assert_eq!(PageRequest::new(None, 100).unwrap().page_size(), 100);
        assert_eq!(PageRequest::new(None, 101), out_of_range);
        assert!(PageRequest::new(None, DEFAULT_PAGE_SIZE).is_ok());
    }

    #[test]
    fn cursor_rules() {
        let invalid = Err(WorkTrackingError::invalid_input(
            InputField::Cursor,
            InvalidInputReason::InvalidFormat,
        ));
        assert_eq!(Cursor::parse("2").unwrap().as_str(), "2");
        assert!(Cursor::parse(&"a".repeat(64)).is_ok());
        assert_eq!(Cursor::parse(&"a".repeat(65)), invalid);
        assert_eq!(Cursor::parse(""), invalid);
        assert_eq!(Cursor::parse("a b"), invalid);
        assert_eq!(Cursor::parse("é"), invalid);
        assert_eq!(Cursor::parse("a\n"), invalid);
        assert_eq!(Cursor::from_numeric(3).as_str(), "3");
    }

    #[test]
    fn page_request_keeps_cursor() {
        let request = PageRequest::new(Some(Cursor::parse("4").unwrap()), 50).unwrap();
        assert_eq!(request.cursor().map(Cursor::as_str), Some("4"));
    }
}
