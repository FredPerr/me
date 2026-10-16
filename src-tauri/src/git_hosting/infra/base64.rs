const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const PADDING: char = '=';

/// Standard base64 (RFC 4648) with padding. Only builds the HTTP Basic
/// credential, so it stays local rather than adding a crate.
pub fn encode(input: &[u8]) -> String {
    let mut encoded = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let first = chunk[0] as u32;
        let second = *chunk.get(1).unwrap_or(&0) as u32;
        let third = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (first << 16) | (second << 8) | third;
        let sextet = |shift: u32| ALPHABET[((triple >> shift) & 0x3F) as usize] as char;
        encoded.push(sextet(18));
        encoded.push(sextet(12));
        encoded.push(if chunk.len() > 1 { sextet(6) } else { PADDING });
        encoded.push(if chunk.len() > 2 { sextet(0) } else { PADDING });
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::encode;

    #[test]
    fn encodes_without_padding_when_the_length_is_a_multiple_of_three() {
        assert_eq!(encode(b"abc"), "YWJj");
    }

    #[test]
    fn pads_a_single_trailing_byte() {
        assert_eq!(encode(b"a"), "YQ==");
    }

    #[test]
    fn pads_two_trailing_bytes() {
        assert_eq!(encode(b"ab"), "YWI=");
    }

    #[test]
    fn encodes_the_basic_credential_form() {
        assert_eq!(encode(b"x-access-token:t"), "eC1hY2Nlc3MtdG9rZW46dA==");
    }
}
