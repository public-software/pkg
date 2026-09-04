//! The closure of a set of store objects: every object reachable through references, each once.

use std::collections::VecDeque;

use crate::error::{Error, MissingReference};
use crate::path::{References, StorePath};

/// The closure of `roots`: the roots and every object reachable from them through `lookup`, which
/// returns an object's references or `None` when the object is not held.
///
/// Breadth-first; each path is looked up once; a lookup that answers `None` is
/// [`Error::MissingReference`] naming the path and the object that referenced it (`None` for a
/// root). A cycle cannot arise from [`StorePath::derive`] (a path commits to its references, so
/// an object cannot name itself), but a lookup that presents one still terminates.
pub fn closure<I, F>(roots: I, mut lookup: F) -> Result<References, Error>
where
    I: IntoIterator<Item = StorePath>,
    F: FnMut(&StorePath) -> Option<References>,
{
    let mut seen = References::new();
    let mut queue: VecDeque<(StorePath, Option<StorePath>)> =
        roots.into_iter().map(|root| (root, None)).collect();
    while let Some((path, referrer)) = queue.pop_front() {
        if seen.contains(&path) {
            continue;
        }
        let references = lookup(&path).ok_or_else(|| {
            Error::from(MissingReference {
                path: path.clone(),
                referrer,
            })
        })?;
        for reference in references {
            if !seen.contains(&reference) {
                queue.push_back((reference, Some(path.clone())));
            }
        }
        seen.insert(path);
    }
    Ok(seen)
}
