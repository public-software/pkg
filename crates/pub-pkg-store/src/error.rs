//! What can go wrong: text of the wrong shape, an absolute path that is not under the store, or a
//! reference the store does not hold.

use std::fmt;

use crate::StorePath;

/// An error of the store path model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The text does not have the shape the model gives it. `what` names the value (`store path`,
    /// `digest`, `name`, `version`, `store directory`, `base-32`), `reason` says what is wrong.
    Malformed {
        /// The value that is malformed.
        what: &'static str,
        /// What is wrong with it.
        reason: String,
    },
    /// An absolute path that does not name an object under the store directory.
    OutsideStore {
        /// The store directory the path was resolved against.
        store: String,
        /// The path as given.
        path: String,
    },
    /// A reference to an object the lookup does not hold (boxed: two store paths would make every
    /// `Result` of the crate large).
    MissingReference(Box<MissingReference>),
}

/// The detail of [`Error::MissingReference`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingReference {
    /// The path that could not be looked up.
    pub path: StorePath,
    /// The object whose references named it, or `None` for a root of the closure.
    pub referrer: Option<StorePath>,
}

impl From<MissingReference> for Error {
    fn from(missing: MissingReference) -> Self {
        Self::MissingReference(Box::new(missing))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed { what, reason } => write!(f, "malformed {what}: {reason}"),
            Self::OutsideStore { store, path } => {
                write!(f, "{path} is not under the store directory {store}")
            }
            Self::MissingReference(missing) => match &missing.referrer {
                Some(referrer) => write!(
                    f,
                    "{} is referenced by {referrer} but is not in the store",
                    missing.path
                ),
                None => write!(f, "{} is not in the store", missing.path),
            },
        }
    }
}

impl std::error::Error for Error {}

/// A [`Error::Malformed`] for `what`.
pub(crate) fn malformed(what: &'static str, reason: impl Into<String>) -> Error {
    Error::Malformed {
        what,
        reason: reason.into(),
    }
}
