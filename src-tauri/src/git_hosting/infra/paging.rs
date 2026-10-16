//! Following a Git host's paginated lists, shared by the adapters.

use std::future::Future;

use reqwest::Url;

use crate::git_hosting::domain::capped::{Capped, PAGE_LIMIT};
use crate::git_hosting::domain::error::GitHostingError;

const DOT_SEGMENTS: [&str; 2] = [".", ".."];

/// One page of a list and the URL of the next page, if there is one.
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_url: Option<String>,
}

/// Reads up to [`PAGE_LIMIT`] pages starting at `first_url`. The list is
/// `truncated` when a next page remains, or when the next page is not on
/// `api_base`: a token is never sent to a URL another server handed out.
pub async fn follow_pages<T, Fetch, PageFuture>(
    first_url: String,
    api_base: &str,
    mut fetch_page: Fetch,
) -> Result<Capped<T>, GitHostingError>
where
    Fetch: FnMut(String) -> PageFuture,
    PageFuture: Future<Output = Result<Page<T>, GitHostingError>>,
{
    let mut items = Vec::new();
    let mut next_url = Some(first_url);
    for _ in 0..PAGE_LIMIT {
        let Some(url) = next_url.take() else {
            break;
        };
        let page = fetch_page(url).await?;
        items.extend(page.items);
        next_url = page.next_url;
        if let Some(url) = &next_url {
            if !is_under(url, api_base) {
                eprintln!("[git_hosting] next page outside {api_base} not followed");
                break;
            }
        }
    }
    Ok(Capped {
        truncated: next_url.is_some(),
        items,
    })
}

/// Whether `url` is `api_base` itself or a path below it.
pub fn is_under(url: &str, api_base: &str) -> bool {
    url.strip_prefix(api_base)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(['/', '?']))
}

/// `api_base` followed by each segment, percent-encoded, then the query.
/// `.` and `..` would change the path, so a repository named so is not found.
pub fn api_url(
    api_base: &str,
    segments: &[&str],
    query: &[(&str, &str)],
) -> Result<String, GitHostingError> {
    if segments
        .iter()
        .any(|segment| DOT_SEGMENTS.contains(segment))
    {
        return Err(GitHostingError::NotFound);
    }
    let mut url = Url::parse(api_base).map_err(|_| GitHostingError::InvalidResponse)?;
    url.path_segments_mut()
        .map_err(|_| GitHostingError::InvalidResponse)?
        .pop_if_empty()
        .extend(segments);
    if !query.is_empty() {
        url.query_pairs_mut().extend_pairs(query);
    }
    Ok(url.into())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    const API_BASE: &str = "https://api.example.org";
    const FIRST_URL: &str = "https://api.example.org/items?page=1";
    const ITEMS_PER_PAGE: usize = 2;

    fn page_url(page_number: usize) -> String {
        format!("{API_BASE}/items?page={page_number}")
    }

    /// A list of `page_count` pages, each linking to the next.
    async fn read_pages(page_count: usize) -> (Capped<usize>, Vec<String>) {
        read_pages_linking(page_count, |page_number| page_url(page_number + 1)).await
    }

    async fn read_pages_linking(
        page_count: usize,
        next_url: impl Fn(usize) -> String,
    ) -> (Capped<usize>, Vec<String>) {
        let fetched_urls = RefCell::new(Vec::new());
        let capped = follow_pages(FIRST_URL.to_string(), API_BASE, |url| {
            fetched_urls.borrow_mut().push(url);
            let page_number = fetched_urls.borrow().len();
            let page = Page {
                items: vec![page_number; ITEMS_PER_PAGE],
                next_url: (page_number < page_count).then(|| next_url(page_number)),
            };
            async move { Ok(page) }
        })
        .await
        .unwrap();
        (capped, fetched_urls.into_inner())
    }

    #[tokio::test]
    async fn reads_a_single_page() {
        let (capped, fetched_urls) = read_pages(1).await;

        assert_eq!(fetched_urls, vec![FIRST_URL.to_string()]);
        assert_eq!(
            capped,
            Capped {
                items: vec![1; ITEMS_PER_PAGE],
                truncated: false
            }
        );
    }

    #[tokio::test]
    async fn follows_every_next_page() {
        let (capped, fetched_urls) = read_pages(3).await;

        assert_eq!(
            fetched_urls,
            vec![FIRST_URL.to_string(), page_url(2), page_url(3)]
        );
        assert_eq!(capped.items, vec![1, 1, 2, 2, 3, 3]);
        assert!(!capped.truncated);
    }

    #[tokio::test]
    async fn reads_the_page_limit_without_truncating() {
        let (capped, fetched_urls) = read_pages(PAGE_LIMIT).await;

        assert_eq!(fetched_urls.len(), PAGE_LIMIT);
        assert!(!capped.truncated);
    }

    #[tokio::test]
    async fn stops_at_the_page_limit_and_truncates() {
        let (capped, fetched_urls) = read_pages(PAGE_LIMIT + 1).await;

        assert_eq!(fetched_urls.len(), PAGE_LIMIT);
        assert_eq!(capped.items.len(), PAGE_LIMIT * ITEMS_PER_PAGE);
        assert!(capped.truncated);
    }

    #[tokio::test]
    async fn does_not_follow_a_next_page_on_another_host() {
        let (capped, fetched_urls) =
            read_pages_linking(2, |_| "https://api.example.org.evil.test/items".to_string()).await;

        assert_eq!(fetched_urls, vec![FIRST_URL.to_string()]);
        assert!(capped.truncated);
    }

    #[tokio::test]
    async fn a_failing_page_fails_the_list() {
        let result: Result<Capped<usize>, GitHostingError> =
            follow_pages(FIRST_URL.to_string(), API_BASE, |_| async {
                Err(GitHostingError::Network)
            })
            .await;

        assert_eq!(result, Err(GitHostingError::Network));
    }

    #[test]
    fn a_url_below_the_base_is_under_it() {
        assert!(is_under(&page_url(2), API_BASE));
    }

    #[test]
    fn a_look_alike_host_is_not_under_the_base() {
        for url in [
            "https://api.example.org.evil.test/items",
            "https://api.example.org@evil.test/items",
            "http://api.example.org/items",
            "https://evil.test/https://api.example.org/items",
        ] {
            assert!(!is_under(url, API_BASE), "{url}");
        }
    }

    #[test]
    fn encodes_path_segments_and_query() {
        let url = api_url(
            API_BASE,
            &["repos", "acme corp", "a/b?c#d"],
            &[("state", "open")],
        )
        .unwrap();

        assert_eq!(
            url,
            "https://api.example.org/repos/acme%20corp/a%2Fb%3Fc%23d?state=open"
        );
    }

    #[test]
    fn keeps_the_path_of_the_base() {
        let url = api_url("https://api.example.org/2.0", &["user"], &[]).unwrap();

        assert_eq!(url, "https://api.example.org/2.0/user");
    }

    #[test]
    fn refuses_dot_segments() {
        assert_eq!(
            api_url(API_BASE, &["repos", "..", "user"], &[]),
            Err(GitHostingError::NotFound)
        );
    }
}
