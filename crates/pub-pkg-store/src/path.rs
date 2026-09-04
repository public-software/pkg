//! The store directory and the store path: how an object's location is derived from what it is.

use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use crate::digest::Digest;
use crate::error::{Error, malformed};
use crate::name::{Name, Version};

/// The references of a store object: the store paths it points at, ordered.
pub type References = BTreeSet<StorePath>;

/// The first line of every fingerprint; a change of the scheme changes it.
pub const FINGERPRINT_HEADER: &str = "pub-pkg-store-path:1";

/// The directory every store object lives in: an absolute, normalised path without a trailing
/// slash (`/pub/store`). It is part of every fingerprint, so an object moved to another store
/// directory is another object.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StoreDir(String);

impl StoreDir {
    /// Checks `text`: absolute, not the root, no trailing slash, no empty, `.` or `..` component,
    /// no NUL.
    pub fn new(text: &str) -> Result<Self, Error> {
        const WHAT: &str = "store directory";
        if text.is_empty() {
            return Err(malformed(WHAT, "empty"));
        }
        let Some(rest) = text.strip_prefix('/') else {
            return Err(malformed(WHAT, format!("{text:?} is not absolute")));
        };
        if rest.is_empty() {
            return Err(malformed(WHAT, "the root directory cannot be the store"));
        }
        if text.contains('\0') {
            return Err(malformed(WHAT, "contains a NUL byte"));
        }
        if text.ends_with('/') {
            return Err(malformed(WHAT, format!("{text:?} ends with a slash")));
        }
        for component in rest.split('/') {
            match component {
                "" => return Err(malformed(WHAT, format!("{text:?} has an empty component"))),
                "." | ".." => {
                    return Err(malformed(
                        WHAT,
                        format!("{text:?} has a {component:?} component"),
                    ));
                }
                _ => {}
            }
        }
        Ok(Self(text.to_owned()))
    }

    /// The directory as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The absolute path of `path` in this store: `<store>/<hash>-<name>-<version>`.
    pub fn path_of(&self, path: &StorePath) -> String {
        format!("{}/{path}", self.0)
    }

    /// The store object an absolute path names: the path must be under this directory, and the
    /// first component after it is the object's basename (anything deeper is inside the object).
    pub fn parse(&self, absolute: &str) -> Result<StorePath, Error> {
        let outside = || Error::OutsideStore {
            store: self.0.clone(),
            path: absolute.to_owned(),
        };
        let rest = absolute
            .strip_prefix(self.0.as_str())
            .and_then(|rest| rest.strip_prefix('/'))
            .ok_or_else(outside)?;
        let basename = rest.split('/').next().unwrap_or_default();
        if basename.is_empty() {
            return Err(outside());
        }
        StorePath::parse(basename)
    }
}

impl fmt::Display for StoreDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for StoreDir {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

/// A store path: the basename `<hash>-<name>-<version>` of one store object, where the hash is
/// the SHA-256 of the object's [fingerprint](StorePath::fingerprint) in the suite's base-32
/// alphabet.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StorePath {
    hash: Digest,
    name: Name,
    version: Version,
}

impl StorePath {
    /// Derives the path of an object from what it is: the store it lives in, its name and version,
    /// the digest of its content and the set of objects it references.
    pub fn derive(
        store: &StoreDir,
        name: &Name,
        version: &Version,
        content: &Digest,
        references: &References,
    ) -> Self {
        let fingerprint = Self::fingerprint(store, name, version, content, references);
        Self {
            hash: Digest::of(fingerprint.as_bytes()),
            name: name.clone(),
            version: version.clone(),
        }
    }

    /// The text the path hash is the SHA-256 of: one line each for the header, the store
    /// directory, the name, the version, the content digest and the reference count, then one
    /// line per reference in order, every line newline-terminated.
    ///
    /// ```text
    /// pub-pkg-store-path:1
    /// store:/pub/store
    /// name:hello
    /// version:2.1
    /// content:sha256:<52 symbols>
    /// references:1
    /// <hash>-libc-1.0
    /// ```
    pub fn fingerprint(
        store: &StoreDir,
        name: &Name,
        version: &Version,
        content: &Digest,
        references: &References,
    ) -> String {
        let mut text = format!(
            "{FINGERPRINT_HEADER}\nstore:{store}\nname:{name}\nversion:{version}\ncontent:sha256:{content}\nreferences:{}\n",
            references.len()
        );
        for reference in references {
            text.push_str(&reference.to_string());
            text.push('\n');
        }
        text
    }

    /// A path from its parts, as read from a store rather than derived.
    pub fn from_parts(hash: Digest, name: Name, version: Version) -> Self {
        Self {
            hash,
            name,
            version,
        }
    }

    /// Parses a basename `<hash>-<name>-<version>`: 52 symbols of the alphabet, a hyphen, the
    /// name, a hyphen, the version (the part after the last hyphen).
    pub fn parse(text: &str) -> Result<Self, Error> {
        const WHAT: &str = "store path";
        if !text.is_ascii() {
            return Err(malformed(WHAT, "contains a non-ASCII character"));
        }
        if text.len() < Digest::SYMBOLS {
            return Err(malformed(
                WHAT,
                format!(
                    "the hash is {} symbols, not {}",
                    text.len(),
                    Digest::SYMBOLS
                ),
            ));
        }
        let (hash, rest) = text.split_at(Digest::SYMBOLS);
        let hash = Digest::parse(hash).map_err(|err| match err {
            Error::Malformed { reason, .. } => malformed(WHAT, format!("hash: {reason}")),
            other => other,
        })?;
        if rest.is_empty() {
            return Err(malformed(WHAT, "no name after the hash"));
        }
        let Some(rest) = rest.strip_prefix('-') else {
            return Err(malformed(WHAT, "no hyphen after the hash"));
        };
        if rest.is_empty() {
            return Err(malformed(WHAT, "no name after the hash"));
        }
        let Some((name, version)) = rest.rsplit_once('-') else {
            return Err(malformed(WHAT, "no version after the name"));
        };
        if version.is_empty() {
            return Err(malformed(WHAT, "no version after the name"));
        }
        if name.is_empty() {
            return Err(malformed(WHAT, "no name after the hash"));
        }
        let name = Name::new(name).map_err(|err| match err {
            Error::Malformed { reason, .. } => malformed(WHAT, format!("name: {reason}")),
            other => other,
        })?;
        let version = Version::new(version).map_err(|err| match err {
            Error::Malformed { reason, .. } => malformed(WHAT, format!("version: {reason}")),
            other => other,
        })?;
        Ok(Self {
            hash,
            name,
            version,
        })
    }

    /// The path hash.
    pub fn hash(&self) -> &Digest {
        &self.hash
    }

    /// The package name.
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// The package version.
    pub fn version(&self) -> &Version {
        &self.version
    }
}

impl fmt::Display for StorePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}-{}", self.hash, self.name, self.version)
    }
}

impl FromStr for StorePath {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_parts_prints_like_a_derived_path() {
        let path = StorePath::from_parts(
            Digest::of(b"x"),
            Name::new("x").unwrap(),
            Version::new("1").unwrap(),
        );
        assert_eq!(path.to_string(), format!("{}-x-1", Digest::of(b"x")));
        assert_eq!(StorePath::parse(&path.to_string()).unwrap(), path);
    }

    #[test]
    fn a_non_ascii_basename_is_refused_before_the_hash_is_read() {
        let err = StorePath::parse(&format!("{}-\u{e9}t\u{e9}-1", Digest::of(b"x"))).unwrap_err();
        assert_eq!(
            err.to_string(),
            "malformed store path: contains a non-ASCII character"
        );
    }
}
