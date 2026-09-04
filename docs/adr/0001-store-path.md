# ADR-0001: A store path is the hash of what the object is

- Status: accepted
- Date: 2026-09-03
- Scope: this repository only (cross-repo decisions are RFCs in public-software/rfcs)

## Context

`pkg` is the content-addressed, reproducible package manager of the suite and the mechanism every
other repository is distributed by. Everything else in it (the on-disk store, the build cache
daemon, the transparency-log client, the `pub pkg` commands) rests on one question: where does a
built or fetched object live, and what does its location promise? The first crate has to settle
that, and settle it without looking at the incumbents: Nix is LGPL-2.1 and Guix is GPL-3, and
neither their source nor their manuals were consulted (`PROVENANCE.md`). The model is designed
from the content-addressing literature: a node of a Merkle DAG is named by the hash of its payload
and of its children's names, which is what makes the graph acyclic and its names verifiable.

## Decision

1. **A store path is `<hash>-<name>-<version>` under one store directory.** The hash is 52
   symbols; the name and version are there for people and tools that list a store, and they are
   inputs of the hash, not decoration. The absolute path is `<store>/<hash>-<name>-<version>`;
   anything deeper is inside the object. `pub-pkg-store` keeps the basename as `StorePath` and
   the directory as `StoreDir`, so the same object is addressed the same way on every host that
   uses the same store directory.

2. **The hash is SHA-256 in the suite's base-32 alphabet, one spelling per path.** The alphabet
   is Douglas Crockford's 32 symbols (the digits and the letters without `i`, `l`, `o` and `u`)
   in lowercase; 256 bits are 52 symbols with four zero pad bits, most significant bit first.
   Crockford's decoder folds case and aliases the excluded letters; ours refuses all of that,
   because a path is an identity and two spellings of one object would defeat every equality
   check, every index and every signature over a path. Nothing is truncated: a shorter hash would
   need a collision argument the full digest does not.

3. **The hash commits to the store directory, the name, the version, the content digest and the
   references.** The fingerprint is a small text (`FINGERPRINT_HEADER`, then one line each for
   `store:`, `name:`, `version:`, `content:sha256:` and `references:<count>`, then one line per
   reference in order) and the path hash is its SHA-256. Consequences that are the point:
   the same content with the same references yields the same path anywhere; an object copied into
   another store directory is another object (a path never lies about where it is valid); a path
   is a Merkle link to its whole closure, so a signature over a root path covers every dependency;
   and an object cannot reference itself or take part in a cycle, because its name would have to
   contain its own hash. The content digest is the SHA-256 of the object's serialised form; the
   serialisation is the store format's decision, a later ADR, and this crate takes the digest as
   given.

4. **Two small grammars keep the basename unambiguous.** A name is `[a-z0-9][a-z0-9_.+-]*`, at
   most 128 bytes, not ending in a hyphen; a version is `[0-9a-z][0-9a-z.+~]*`, at most 64 bytes.
   A version has no hyphen, so the version is what follows the last hyphen and the name may keep
   its own (`gcc-wrapper-x`); the hash is the first 52 symbols. Lowercase only, so the path is
   the same on case-insensitive file systems; nothing outside ASCII, so it is the same in every
   locale; the whole basename stays under 255 bytes.

5. **SHA-256 is implemented in the crate, from FIPS 180-4.** The RustCrypto `sha2` tree has no
   audit in the Mozilla or Google pool at the versions Cargo resolves today, and an exemption is
   not an audit. The module is the one `identity` ships (same organisation, same licence), a page
   of code with no key and no secret-dependent branching, tested against the standard's vectors.
   When the organisation's vet store covers the RustCrypto tree, both crates swap it for `sha2`
   without an API change.

6. **The closure is a function over a lookup, not over a store.** `closure(roots, lookup)`
   returns every object reachable from the roots, each once, and names a reference the lookup
   cannot answer together with the object that made it. The store, the cache and the transport
   each bring their own lookup.

## Consequences

- Every later crate of `pkg` (the on-disk store, `pubd-cache`, the transparency-log client) takes
  `StorePath`, `StoreDir`, `Digest` and `References` as they are; the fingerprint header carries
  a version, so a change of scheme is a new header and a new hash, never a silent one.
- Tests are vectors and derivations: the FIPS 180-4 digests, the base-32 symbols, a graph of four
  objects; no file system in CI.
- What is deferred, with its shape: the serialisation whose digest is the content digest (a
  canonical archive of a directory tree, the store format ADR); an object whose content must
  contain its own path (a build output with embedded paths) needs a scheme for rewriting or
  self-reference and is out of this model on purpose; the build recipe (the derivation of an
  output from inputs and a builder) is a separate object kind with its own fingerprint.
