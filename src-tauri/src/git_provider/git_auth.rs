//! Injects a stored provider access token into `git` CLI invocations.
//!
//! Rather than rewriting remote URLs (which would persist credentials into
//! `.git/config`) or touching the on-disk credential helper, we pass the token
//! per-invocation via `git -c http.<url>.extraheader=Authorization: Basic ...`.
//! The header is scoped to the specific remote host so the token is never sent
//! to an unrelated remote.

use super::token_store;
use super::PROVIDER_GITHUB;

const GITHUB_HOST: &str = "github.com";

/// Standard, dependency-free base64 (RFC 4648) encoder. Used only to build the
/// HTTP Basic credential; kept local to avoid pulling in a base64 crate.
fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(input.len().div_ceil(3) * 4);

    for chunk in input.chunks(3) {
        let byte0 = chunk[0] as u32;
        let byte1 = *chunk.get(1).unwrap_or(&0) as u32;
        let byte2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (byte0 << 16) | (byte1 << 8) | byte2;

        encoded.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        encoded.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            ALPHABET[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            ALPHABET[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }

    encoded
}

/// Which provider, if any, owns the given remote URL. Only HTTPS remotes can
/// carry an injected token; SSH remotes authenticate via the user's keys.
fn provider_for_remote(remote_url: &str) -> Option<&'static str> {
    let is_https = remote_url.starts_with("https://") || remote_url.starts_with("http://");
    if is_https && remote_url.contains(GITHUB_HOST) {
        Some(PROVIDER_GITHUB)
    } else {
        None
    }
}

/// Build the `-c http.<host>.extraheader=...` arguments that authenticate a
/// `git` call against `remote_url`, or an empty vec when no token applies
/// (non-GitHub remote, SSH remote, or not connected).
///
/// GitHub accepts a user access token as the HTTP Basic password with any
/// username; we use the conventional `x-access-token`.
pub fn auth_args_for_remote(remote_url: &str) -> Vec<String> {
    let Some(provider_id) = provider_for_remote(remote_url) else {
        return Vec::new();
    };

    let token = match token_store::read_token(provider_id) {
        Ok(Some(token)) => token,
        _ => return Vec::new(),
    };

    let credential = base64_encode(format!("x-access-token:{}", token).as_bytes());
    let host_config = format!("https://{}/", GITHUB_HOST);

    vec![
        "-c".to_string(),
        format!("http.{}.extraheader=Authorization: Basic {}", host_config, credential),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_encodes_without_padding_when_len_is_multiple_of_three() {
        assert_eq!(base64_encode(b"abc"), "YWJj");
    }

    #[test]
    fn base64_pads_a_single_trailing_byte() {
        assert_eq!(base64_encode(b"a"), "YQ==");
    }

    #[test]
    fn base64_pads_two_trailing_bytes() {
        assert_eq!(base64_encode(b"ab"), "YWI=");
    }

    #[test]
    fn base64_encodes_the_basic_credential_form() {
        assert_eq!(base64_encode(b"x-access-token:t"), "eC1hY2Nlc3MtdG9rZW46dA==");
    }

    #[test]
    fn no_auth_args_for_ssh_remote() {
        assert!(auth_args_for_remote("git@github.com:user/repo.git").is_empty());
    }

    #[test]
    fn no_auth_args_for_non_github_https_remote() {
        assert!(auth_args_for_remote("https://gitlab.com/user/repo.git").is_empty());
    }
}
