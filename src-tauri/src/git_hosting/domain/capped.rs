/// How many pages of a Git host's list are read before the list is cut short.
pub const PAGE_LIMIT: usize = 10;

/// A list read from a Git host, `truncated` when more items were left unread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capped<T> {
    pub items: Vec<T>,
    pub truncated: bool,
}
