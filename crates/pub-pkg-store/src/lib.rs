//! `pub-pkg-store` — the store path model of [`pkg`](https://github.com/public-software/pkg), the
//! content-addressed package manager of Public Software.
//!
//! A store object is a directory or file under one store directory (`/pub/store`), named by what
//! it is: its path is `<hash>-<name>-<version>` where the hash is the SHA-256 of a fingerprint
//! over the store directory, the name, the version, the digest of the content and the set of
//! store paths the object references, written in the suite's base-32 alphabet. The same content
//! with the same references yields the same path anywhere; a path is a Merkle link to its closure,
//! so an object cannot reference itself and the reference graph has no cycles. The scheme is
//! ADR-0001 of the repository; the on-disk format, the build cache and the transparency log are
//! later crates that build on these types.
//!
//! ```
//! use std::collections::BTreeMap;
//! use pub_pkg_store::{Digest, Name, References, StoreDir, StorePath, Version, closure};
//!
//! let store = StoreDir::new("/pub/store")?;
//! let libc = StorePath::derive(
//!     &store, &Name::new("libc")?, &Version::new("1.0")?,
//!     &Digest::of(b"the serialised content of libc"), &References::new(),
//! );
//! let hello = StorePath::derive(
//!     &store, &Name::new("hello")?, &Version::new("2.1")?,
//!     &Digest::of(b"the serialised content of hello"), &References::from([libc.clone()]),
//! );
//! assert_eq!(hello.to_string().len(), 52 + "-hello-2.1".len());
//! assert_eq!(store.parse(&store.path_of(&hello))?, hello);
//!
//! let objects = BTreeMap::from([
//!     (libc.clone(), References::new()),
//!     (hello.clone(), References::from([libc.clone()])),
//! ]);
//! let needed = closure([hello.clone()], |path| objects.get(path).cloned())?;
//! assert_eq!(needed, References::from([libc, hello]));
//! # Ok::<(), pub_pkg_store::Error>(())
//! ```

#![forbid(unsafe_code)]

pub mod base32;
mod closure;
mod digest;
mod error;
mod name;
mod path;
mod sha256;

pub use closure::closure;
pub use digest::Digest;
pub use error::{Error, MissingReference};
pub use name::{Name, Version};
pub use path::{FINGERPRINT_HEADER, References, StoreDir, StorePath};

/// The crate's name, as `CATALOG.toml` and crates.io know it.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// The crate's version, as Cargo knows it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_follows_the_naming_rule() {
        assert_eq!(NAME, "pub-pkg-store");
        assert!(NAME.starts_with("pub-pkg-"));
    }

    #[test]
    fn version_is_semver_shaped() {
        assert_eq!(VERSION.split('.').count(), 3, "{VERSION}");
    }
}
