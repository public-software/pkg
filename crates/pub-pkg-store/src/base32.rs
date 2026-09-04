//! The suite's base-32 alphabet: the 32 symbols of Douglas Crockford's Base 32 (the digits and the
//! letters without `i`, `l`, `o` and `u`), in lowercase, five bits per symbol, most significant
//! bit first, the final symbol padded with zero bits.
//!
//! Unlike Crockford's decoder this one is strict: no uppercase, no aliases for the excluded
//! letters, no hyphens. A store path is an identity and has one spelling.

use crate::error::{Error, malformed};

/// The 32 symbols, in value order.
pub const ALPHABET: &str = "0123456789abcdefghjkmnpqrstvwxyz";

const SYMBOLS: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// The number of symbols that encode `bytes` bytes.
pub const fn symbols_for(bytes: usize) -> usize {
    (bytes * 8).div_ceil(5)
}

/// Encodes `bytes`: every five bits one symbol, most significant bit first; when the bit count is
/// not a multiple of five the last symbol carries zero pad bits in its low positions.
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(symbols_for(bytes.len()));
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    for &byte in bytes {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(char::from(SYMBOLS[((buffer >> bits) & 31) as usize]));
        }
        buffer &= (1 << bits) - 1;
    }
    if bits > 0 {
        out.push(char::from(SYMBOLS[((buffer << (5 - bits)) & 31) as usize]));
    }
    out
}

/// Decodes `text`, refusing a symbol outside [`ALPHABET`], a length no byte string encodes to, and
/// non-zero pad bits.
pub fn decode(text: &str) -> Result<Vec<u8>, Error> {
    let mut values = Vec::with_capacity(text.len());
    for (index, symbol) in text.chars().enumerate() {
        let value = value_of(symbol).ok_or_else(|| {
            malformed(
                "base-32",
                format!(
                    "symbol {} is {symbol:?}, not in the alphabet {ALPHABET}",
                    index + 1
                ),
            )
        })?;
        values.push(value);
    }
    let byte_count = values.len() * 5 / 8;
    if symbols_for(byte_count) != values.len() {
        return Err(malformed(
            "base-32",
            format!(
                "{} symbols is not the length of any encoded byte string",
                values.len()
            ),
        ));
    }
    let mut out = Vec::with_capacity(byte_count);
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    for value in values {
        buffer = (buffer << 5) | u32::from(value);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    if buffer != 0 {
        return Err(malformed(
            "base-32",
            "the final symbol carries non-zero pad bits",
        ));
    }
    Ok(out)
}

/// The value of one symbol, `None` outside the alphabet.
pub(crate) fn value_of(symbol: char) -> Option<u8> {
    u8::try_from(symbol)
        .ok()
        .and_then(|byte| SYMBOLS.iter().position(|&s| s == byte))
        .map(|position| position as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_follow_the_alphabet() {
        for (value, symbol) in ALPHABET.chars().enumerate() {
            assert_eq!(value_of(symbol), Some(value as u8), "{symbol}");
        }
        assert_eq!(value_of('I'), None);
        assert_eq!(value_of('\u{e9}'), None);
    }

    #[test]
    fn five_bytes_are_eight_symbols_without_padding() {
        assert_eq!(encode(&[0, 0, 0, 0, 0]), "00000000");
        assert_eq!(encode(&[0xff; 5]), "zzzzzzzz");
        assert_eq!(decode("zzzzzzzz").unwrap(), [0xff; 5]);
        assert_eq!(symbols_for(32), 52);
    }

    #[test]
    fn the_length_check_names_the_symbol_count() {
        let err = decode("000").unwrap_err();
        assert_eq!(
            err.to_string(),
            "malformed base-32: 3 symbols is not the length of any encoded byte string"
        );
    }
}
