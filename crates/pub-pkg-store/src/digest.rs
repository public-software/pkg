//! A 32-byte digest, SHA-256 of some bytes, printed in the suite's base-32 alphabet.

use std::fmt;
use std::str::FromStr;

use crate::base32;
use crate::error::{Error, malformed};
use crate::sha256::sha256;

/// A SHA-256 digest: 32 bytes, 52 symbols in the suite's base-32 alphabet.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest([u8; 32]);

impl Digest {
    /// The number of bytes of a digest.
    pub const BYTES: usize = 32;

    /// The number of base-32 symbols a digest prints as.
    pub const SYMBOLS: usize = base32::symbols_for(Self::BYTES);

    /// The SHA-256 digest of `bytes` (FIPS 180-4 §6.2).
    pub fn of(bytes: &[u8]) -> Self {
        Self(sha256(bytes))
    }

    /// A digest from its 32 bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The 32 bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The digest as 52 symbols of the suite's base-32 alphabet.
    pub fn to_base32(&self) -> String {
        base32::encode(&self.0)
    }

    /// The digest as 64 lowercase hexadecimal digits, the form the FIPS vectors are published in.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Parses the base-32 form: exactly 52 symbols of the alphabet, zero pad bits.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let symbols = text.chars().count();
        if symbols != Self::SYMBOLS {
            return Err(malformed(
                "digest",
                format!("the hash is {symbols} symbols, not {}", Self::SYMBOLS),
            ));
        }
        let bytes = base32::decode(text).map_err(|err| match err {
            Error::Malformed { reason, .. } => malformed("digest", reason),
            other => other,
        })?;
        let mut digest = [0u8; 32];
        digest.copy_from_slice(&bytes);
        Ok(Self(digest))
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_base32())
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Digest({})", self.to_base32())
    }
}

impl FromStr for Digest {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_shows_the_base32_form() {
        let digest = Digest::of(b"abc");
        assert_eq!(format!("{digest:?}"), format!("Digest({digest})"));
        assert_eq!(digest.to_string().len(), Digest::SYMBOLS);
    }

    #[test]
    fn a_non_malformed_decode_error_cannot_happen_but_is_passed_through() {
        assert!(Digest::parse(&"0".repeat(52)).is_ok());
        assert_eq!(
            Digest::parse(&"0".repeat(52)).unwrap(),
            Digest::from_bytes([0; 32])
        );
    }
}
