# pub-pkg-store

The `store` library of [pkg](https://github.com/public-software/pkg), part of Public Software. Kind: `lib`.

The store path model of the content-addressed package manager: a store directory, a SHA-256 digest
(FIPS 180-4, listed in the repository's `PROVENANCE.md`) printed in the suite's base-32 alphabet, the
package name and version grammars, the derivation of a store path `<hash>-<name>-<version>` from the
fingerprint of what the object is (its store directory, name, version, content digest and references,
so the same content yields the same path and a path is a Merkle link to its closure), the parsers of a
basename and of an absolute path under a store, and the closure of a set of roots over a reference
lookup. Not yet: the serialisation the content digest is taken over, the store on disk, self-referencing
outputs, build recipes (ADR-0001).

```sh
cargo nextest run -p pub-pkg-store
```

Its entry in the repository's `CATALOG.toml`:

```toml
[[component]]
crate     = "pub-pkg-store"
kind      = "lib"
ledger    = "store format"
readiness = "seed"
effort    = 3
specs     = ["fips-180-4"]
provides  = []
requires  = []
```
