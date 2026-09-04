//! The package name and version that make a store path readable: two small grammars, chosen so
//! that a basename `<hash>-<name>-<version>` splits without ambiguity (a version has no hyphen).

use std::fmt;
use std::str::FromStr;

use crate::error::{Error, malformed};

/// A package name: `[a-z0-9][a-z0-9_.+-]*`, at most 128 bytes, not ending in a hyphen.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name(String);

impl Name {
    /// The longest name, in bytes.
    pub const MAX_LEN: usize = 128;

    /// Checks `text` against the grammar.
    pub fn new(text: &str) -> Result<Self, Error> {
        check(
            "name",
            text,
            Self::MAX_LEN,
            |c| c.is_ascii_lowercase() || c.is_ascii_digit(),
            |c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '+' | '-'),
            "[a-z0-9][a-z0-9_.+-]*",
        )?;
        if text.ends_with('-') {
            return Err(malformed("name", "ends with a hyphen"));
        }
        Ok(Self(text.to_owned()))
    }

    /// The name as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A package version: `[0-9a-z][0-9a-z.+~]*`, at most 64 bytes. No hyphen, so the version is what
/// follows the last hyphen of a basename.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(String);

impl Version {
    /// The longest version, in bytes.
    pub const MAX_LEN: usize = 64;

    /// Checks `text` against the grammar.
    pub fn new(text: &str) -> Result<Self, Error> {
        check(
            "version",
            text,
            Self::MAX_LEN,
            |c| c.is_ascii_lowercase() || c.is_ascii_digit(),
            |c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '+' | '~'),
            "[0-9a-z][0-9a-z.+~]*",
        )?;
        Ok(Self(text.to_owned()))
    }

    /// The version as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn check(
    what: &'static str,
    text: &str,
    max_len: usize,
    first: impl Fn(char) -> bool,
    rest: impl Fn(char) -> bool,
    grammar: &str,
) -> Result<(), Error> {
    let mut chars = text.chars();
    let Some(head) = chars.next() else {
        return Err(malformed(what, "empty"));
    };
    if text.len() > max_len {
        return Err(malformed(
            what,
            format!("{} bytes, longer than {max_len}", text.len()),
        ));
    }
    if !first(head) {
        return Err(malformed(
            what,
            format!("starts with {head:?}; a {what} is {grammar}"),
        ));
    }
    if let Some(bad) = chars.find(|&c| !rest(c)) {
        return Err(malformed(
            what,
            format!("contains {bad:?}; a {what} is {grammar}"),
        ));
    }
    Ok(())
}

macro_rules! text_impls {
    ($type:ident) => {
        impl fmt::Display for $type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl FromStr for $type {
            type Err = Error;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                Self::new(text)
            }
        }

        impl AsRef<str> for $type {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

text_impls!(Name);
text_impls!(Version);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasons_name_the_offending_character() {
        assert_eq!(
            Name::new("Foo").unwrap_err().to_string(),
            "malformed name: starts with 'F'; a name is [a-z0-9][a-z0-9_.+-]*"
        );
        assert_eq!(
            Name::new("foo/bar").unwrap_err().to_string(),
            "malformed name: contains '/'; a name is [a-z0-9][a-z0-9_.+-]*"
        );
        assert_eq!(
            Version::new("1.0-rc1").unwrap_err().to_string(),
            "malformed version: contains '-'; a version is [0-9a-z][0-9a-z.+~]*"
        );
        assert_eq!(
            Version::new("").unwrap_err().to_string(),
            "malformed version: empty"
        );
        assert_eq!(
            Name::new(&"n".repeat(129)).unwrap_err().to_string(),
            "malformed name: 129 bytes, longer than 128"
        );
    }
}
