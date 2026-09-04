# Provenance

This repository is a spec-first cleanroom implementation. Record here what was consulted.

## Specifications used
- FIPS 180-4, Secure Hash Standard (NIST, https://csrc.nist.gov/pubs/fips/180-4/upd1/final; consulted
  2026-09-03): §5.1.1 (padding), §6.2 (SHA-256) and the published example digests, for the SHA-256
  module of `pub-pkg-store` (the module is the one `pub-identity-client` ships, Apache-2.0 OR MIT).
- Douglas Crockford, Base 32 (https://www.crockford.com/base32.html; consulted 2026-09-03): the 32-symbol
  alphabet and the letters it excludes. The suite's alphabet is that alphabet in lowercase; the decoding
  leniencies of the page (case folding, aliases, hyphens) are deliberately not adopted (ADR-0001).

## Behavioural references (cited, not copied)
- IPFS documentation, "Merkle Directed Acyclic Graphs (DAGs)"
  (https://docs.ipfs.tech/concepts/merkle-dag/; consulted 2026-09-03): the general rule that a node's
  identifier is the hash of its payload and of its children's identifiers, and that this makes cycles
  impossible. The documentation is CC-BY-SA-4.0 content (its code is MIT); only the concept was used, no
  text was reproduced and no IPFS format, code or identifier scheme was adopted.

## Copyleft sources
None consulted. Nothing of Nix (LGPL-2.1) or Guix (GPL-3) was opened, and neither the Nix manual nor
Eelco Dolstra's thesis on the Nix store; the store path scheme is designed in `docs/adr/0001-store-path.md`
from the references above. Contributors who have studied GPL/AGPL implementations of this domain do not
author the corresponding modules (two-team rule; see the Charter §09).

## AI assistance
Prompts point at the specifications and conformance suites above, never at copyleft source. Generated code is
reviewed against this list before merge.
