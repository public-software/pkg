//! The store path model end to end: the base-32 alphabet, the name and version grammars, the
//! derivation of a path from its fingerprint, the parsers and the closure.

use std::collections::BTreeMap;

use pub_pkg_store::{
    Digest, Error, Name, References, StoreDir, StorePath, Version, base32, closure,
};

fn store() -> StoreDir {
    StoreDir::new("/pub/store").unwrap()
}

fn name(s: &str) -> Name {
    Name::new(s).unwrap()
}

fn version(s: &str) -> Version {
    Version::new(s).unwrap()
}

fn hello() -> StorePath {
    StorePath::derive(
        &store(),
        &name("hello"),
        &version("1.0"),
        &Digest::of(b"hello"),
        &References::new(),
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// --- SHA-256 -------------------------------------------------------------------------------------

#[test]
fn digest_matches_the_fips_180_4_vectors() {
    assert_eq!(
        hex(Digest::of(b"").as_bytes()),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        hex(Digest::of(b"abc").as_bytes()),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        hex(Digest::of(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq").as_bytes()),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    assert_eq!(
        Digest::of(b"abc").to_hex(),
        hex(Digest::of(b"abc").as_bytes())
    );
}

// --- base-32 -------------------------------------------------------------------------------------

#[test]
fn the_alphabet_is_crockford_in_lowercase() {
    assert_eq!(base32::ALPHABET, "0123456789abcdefghjkmnpqrstvwxyz");
    assert_eq!(base32::ALPHABET.len(), 32);
    for forbidden in ['i', 'l', 'o', 'u'] {
        assert!(!base32::ALPHABET.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn base32_encodes_bits_most_significant_first() {
    assert_eq!(base32::encode(&[]), "");
    assert_eq!(base32::encode(&[0x00]), "00");
    assert_eq!(base32::encode(&[0xff]), "zw");
    assert_eq!(base32::encode(&[0xff, 0xff]), "zzzg");
    assert_eq!(base32::encode(&[0; 32]).len(), 52);
    assert_eq!(base32::encode(&[0xff; 32]).len(), 52);
}

#[test]
fn base32_round_trips_and_refuses_what_is_not_canonical() {
    let bytes: Vec<u8> = (0..=255u8).collect();
    for len in [0usize, 1, 2, 5, 31, 32, 33, 256] {
        let encoded = base32::encode(&bytes[..len]);
        assert_eq!(encoded.len(), (len * 8).div_ceil(5), "{len}");
        assert_eq!(base32::decode(&encoded).unwrap(), &bytes[..len], "{len}");
    }
    // a wrong length for any byte count
    let err = base32::decode("0").unwrap_err();
    assert!(
        matches!(
            err,
            Error::Malformed {
                what: "base-32",
                ..
            }
        ),
        "{err}"
    );
    // non-zero pad bits: 'x' is 11101, the three low bits are padding after one byte
    let err = base32::decode("zx").unwrap_err();
    assert!(
        matches!(&err, Error::Malformed { what: "base-32", reason } if reason.contains("pad")),
        "{err}"
    );
    // uppercase, the excluded letters and hyphens are not aliases here
    for bad in ["ZW", "zi", "zl", "zo", "zu", "z-w", "z w"] {
        let err = base32::decode(bad).unwrap_err();
        assert!(
            matches!(&err, Error::Malformed { what: "base-32", reason } if reason.contains("alphabet")),
            "{bad}: {err}"
        );
    }
}

#[test]
fn a_digest_prints_as_52_symbols_and_parses_back() {
    let digest = Digest::of(b"hello");
    let text = digest.to_string();
    assert_eq!(text.len(), 52);
    assert_eq!(text, digest.to_base32());
    assert!(text.chars().all(|c| base32::ALPHABET.contains(c)), "{text}");
    assert_eq!(Digest::parse(&text).unwrap(), digest);
    assert_eq!(text.parse::<Digest>().unwrap(), digest);
    assert_eq!(Digest::from_bytes(*digest.as_bytes()), digest);
}

#[test]
fn a_digest_with_a_symbol_outside_the_alphabet_is_refused() {
    let text = Digest::of(b"hello").to_string();
    for bad in ['i', 'l', 'o', 'u', 'A', '-', ' '] {
        let mut wrong = text.clone();
        wrong.replace_range(0..1, &bad.to_string());
        let err = Digest::parse(&wrong).unwrap_err();
        assert!(
            matches!(&err, Error::Malformed { what: "digest", reason } if reason.contains("alphabet")),
            "{bad}: {err}"
        );
    }
    let err = Digest::parse(&text[..51]).unwrap_err();
    assert!(
        matches!(&err, Error::Malformed { what: "digest", reason } if reason.contains("52")),
        "{err}"
    );
    // 256 bits end one bit into the last symbol: only '0' and 'g' are canonical there
    let mut padded = text.clone();
    padded.replace_range(51..52, "1");
    let err = Digest::parse(&padded).unwrap_err();
    assert!(
        matches!(&err, Error::Malformed { what: "digest", reason } if reason.contains("pad")),
        "{err}"
    );
}

// --- names and versions ----------------------------------------------------------------------------

#[test]
fn name_grammar() {
    for ok in [
        "hello",
        "gcc-wrapper",
        "libc++",
        "a",
        "x.y_z",
        "9lives",
        &"n".repeat(128),
    ] {
        assert_eq!(Name::new(ok).unwrap().as_str(), ok);
        assert_eq!(ok.parse::<Name>().unwrap().to_string(), ok);
    }
    for bad in [
        "",
        "-foo",
        "foo-",
        "Foo",
        "foo/bar",
        "foo bar",
        "foo@1",
        ".hidden",
        "_x",
        "a\u{e9}",
        &"n".repeat(129),
    ] {
        let err = Name::new(bad).unwrap_err();
        assert!(
            matches!(err, Error::Malformed { what: "name", .. }),
            "{bad:?}: {err}"
        );
    }
}

#[test]
fn version_grammar() {
    for ok in [
        "1.0",
        "2024a",
        "1.0.0+r3",
        "1.0~rc1",
        "unstable",
        "0",
        &"1".repeat(64),
    ] {
        assert_eq!(Version::new(ok).unwrap().as_str(), ok);
        assert_eq!(ok.parse::<Version>().unwrap().to_string(), ok);
    }
    for bad in [
        "",
        "1.0-rc1",
        "1.0/",
        "V1",
        ".1",
        "+1",
        "~1",
        "1_0",
        "1 0",
        &"1".repeat(65),
    ] {
        let err = Version::new(bad).unwrap_err();
        assert!(
            matches!(
                err,
                Error::Malformed {
                    what: "version",
                    ..
                }
            ),
            "{bad:?}: {err}"
        );
    }
}

#[test]
fn store_directory_is_absolute_and_normalised() {
    assert_eq!(StoreDir::new("/pub/store").unwrap().as_str(), "/pub/store");
    assert_eq!(StoreDir::new("/s").unwrap().as_str(), "/s");
    for bad in [
        "",
        "pub/store",
        "/pub/store/",
        "/",
        "/pub//store",
        "/pub/./store",
        "/pub/../store",
        "/pub/store\0",
    ] {
        let err = StoreDir::new(bad).unwrap_err();
        assert!(
            matches!(
                err,
                Error::Malformed {
                    what: "store directory",
                    ..
                }
            ),
            "{bad:?}: {err}"
        );
    }
}

// --- derivation ------------------------------------------------------------------------------------

#[test]
fn the_same_inputs_yield_the_same_path() {
    let a = hello();
    let b = hello();
    assert_eq!(a, b);
    assert_eq!(a.to_string(), b.to_string());
    assert_eq!(a.name().as_str(), "hello");
    assert_eq!(a.version().as_str(), "1.0");
    assert_eq!(a.to_string(), format!("{}-hello-1.0", a.hash()));
    assert_eq!(a.to_string().len(), 52 + 1 + 5 + 1 + 3);
}

#[test]
fn every_input_of_the_fingerprint_changes_the_path() {
    let base = hello();
    let other_content = StorePath::derive(
        &store(),
        &name("hello"),
        &version("1.0"),
        &Digest::of(b"hello!"),
        &References::new(),
    );
    let other_name = StorePath::derive(
        &store(),
        &name("hallo"),
        &version("1.0"),
        &Digest::of(b"hello"),
        &References::new(),
    );
    let other_version = StorePath::derive(
        &store(),
        &name("hello"),
        &version("1.1"),
        &Digest::of(b"hello"),
        &References::new(),
    );
    let other_store = StorePath::derive(
        &StoreDir::new("/opt/store").unwrap(),
        &name("hello"),
        &version("1.0"),
        &Digest::of(b"hello"),
        &References::new(),
    );
    let mut refs = References::new();
    refs.insert(other_content.clone());
    let with_reference = StorePath::derive(
        &store(),
        &name("hello"),
        &version("1.0"),
        &Digest::of(b"hello"),
        &refs,
    );
    let hashes: Vec<String> = [
        &base,
        &other_content,
        &other_name,
        &other_version,
        &other_store,
        &with_reference,
    ]
    .iter()
    .map(|p| p.hash().to_string())
    .collect();
    for (i, a) in hashes.iter().enumerate() {
        for (j, b) in hashes.iter().enumerate() {
            assert_eq!(a == b, i == j, "{i} vs {j}");
        }
    }
    // the same reference set in another insertion order is the same path
    let dep_a = StorePath::derive(
        &store(),
        &name("a"),
        &version("1"),
        &Digest::of(b"a"),
        &References::new(),
    );
    let dep_b = StorePath::derive(
        &store(),
        &name("b"),
        &version("1"),
        &Digest::of(b"b"),
        &References::new(),
    );
    let mut ab = References::new();
    ab.insert(dep_a.clone());
    ab.insert(dep_b.clone());
    let mut reversed = References::new();
    reversed.insert(dep_b);
    reversed.insert(dep_a);
    assert_eq!(
        StorePath::derive(&store(), &name("c"), &version("1"), &Digest::of(b"c"), &ab),
        StorePath::derive(
            &store(),
            &name("c"),
            &version("1"),
            &Digest::of(b"c"),
            &reversed
        )
    );
}

#[test]
fn the_fingerprint_is_the_documented_text() {
    let dep = hello();
    let mut refs = References::new();
    refs.insert(dep.clone());
    let content = Digest::of(b"world");
    let fingerprint =
        StorePath::fingerprint(&store(), &name("world"), &version("2"), &content, &refs);
    assert_eq!(
        fingerprint,
        format!(
            "pub-pkg-store-path:1\nstore:/pub/store\nname:world\nversion:2\ncontent:sha256:{content}\nreferences:1\n{dep}\n"
        )
    );
    let path = StorePath::derive(&store(), &name("world"), &version("2"), &content, &refs);
    assert_eq!(*path.hash(), Digest::of(fingerprint.as_bytes()));
}

// --- parsing ---------------------------------------------------------------------------------------

#[test]
fn a_basename_parses_back_to_the_same_path() {
    let path = hello();
    let text = path.to_string();
    assert_eq!(StorePath::parse(&text).unwrap(), path);
    assert_eq!(text.parse::<StorePath>().unwrap(), path);
    // a name with hyphens keeps them; the version is what follows the last hyphen
    let hyphenated = StorePath::derive(
        &store(),
        &name("gcc-wrapper-x"),
        &version("14.2"),
        &Digest::of(b"gcc"),
        &References::new(),
    );
    let parsed = StorePath::parse(&hyphenated.to_string()).unwrap();
    assert_eq!(parsed.name().as_str(), "gcc-wrapper-x");
    assert_eq!(parsed.version().as_str(), "14.2");
    assert_eq!(parsed, hyphenated);
}

#[test]
fn a_basename_with_an_invalid_hash_character_is_rejected() {
    let text = hello().to_string();
    for bad in ['i', 'l', 'o', 'u', 'A', '-', '/'] {
        let mut wrong = text.clone();
        wrong.replace_range(3..4, &bad.to_string());
        let err = StorePath::parse(&wrong).unwrap_err();
        assert!(
            matches!(&err, Error::Malformed { what: "store path", reason } if reason.contains("alphabet")),
            "{bad}: {err}"
        );
    }
}

#[test]
fn a_basename_without_a_name_or_a_version_is_rejected() {
    let hash = Digest::of(b"hello").to_string();
    for (bad, needle) in [
        (hash.clone(), "name"),
        (format!("{hash}-"), "name"),
        (format!("{hash}-hello"), "version"),
        (format!("{hash}-hello-"), "version"),
        (format!("{hash}-Hello-1.0"), "name"),
        (format!("{hash}-hello-1.0-"), "version"),
        (format!("{hash}hello-1.0"), "hash"),
        (format!("{}-hello-1.0", &hash[..51]), "hash"),
        (String::new(), "hash"),
    ] {
        let err = StorePath::parse(&bad).unwrap_err();
        assert!(
            matches!(&err, Error::Malformed { what: "store path", reason } if reason.contains(needle)),
            "{bad:?}: {err}"
        );
    }
}

#[test]
fn an_absolute_path_resolves_to_the_object_under_its_store() {
    let path = hello();
    let store = store();
    let absolute = store.path_of(&path);
    assert_eq!(absolute, format!("/pub/store/{path}"));
    assert_eq!(store.parse(&absolute).unwrap(), path);
    assert_eq!(store.parse(&format!("{absolute}/bin/hello")).unwrap(), path);
    for outside in [
        format!("/opt/store/{path}"),
        format!("/pub/storex/{path}"),
        format!("/pub/{path}"),
        path.to_string(),
        "/pub/store".to_string(),
        "/pub/store/".to_string(),
    ] {
        let err = store.parse(&outside).unwrap_err();
        assert!(
            matches!(&err, Error::OutsideStore { store, .. } if store == "/pub/store"),
            "{outside}: {err}"
        );
    }
    let err = store.parse("/pub/store/not-a-store-path").unwrap_err();
    assert!(
        matches!(
            err,
            Error::Malformed {
                what: "store path",
                ..
            }
        ),
        "{err}"
    );
}

// --- closure ---------------------------------------------------------------------------------------

fn leaf(n: &str) -> StorePath {
    StorePath::derive(
        &store(),
        &name(n),
        &version("1"),
        &Digest::of(n.as_bytes()),
        &References::new(),
    )
}

fn graph() -> (BTreeMap<StorePath, References>, [StorePath; 4]) {
    let d = leaf("d");
    let c = StorePath::derive(
        &store(),
        &name("c"),
        &version("1"),
        &Digest::of(b"c"),
        &References::from([d.clone()]),
    );
    let b = StorePath::derive(
        &store(),
        &name("b"),
        &version("1"),
        &Digest::of(b"b"),
        &References::from([d.clone()]),
    );
    let a = StorePath::derive(
        &store(),
        &name("a"),
        &version("1"),
        &Digest::of(b"a"),
        &References::from([b.clone(), c.clone()]),
    );
    let mut objects = BTreeMap::new();
    objects.insert(a.clone(), References::from([b.clone(), c.clone()]));
    objects.insert(b.clone(), References::from([d.clone()]));
    objects.insert(c.clone(), References::from([d.clone()]));
    objects.insert(d.clone(), References::new());
    (objects, [a, b, c, d])
}

#[test]
fn a_closure_includes_every_transitive_reference_once() {
    let (objects, [a, b, c, d]) = graph();
    let lookup = |p: &StorePath| objects.get(p).cloned();
    let all = closure([a.clone()], lookup).unwrap();
    assert_eq!(
        all,
        References::from([a.clone(), b.clone(), c.clone(), d.clone()])
    );
    assert_eq!(all.len(), 4, "d is reached twice and listed once");
    assert_eq!(
        closure([b.clone()], lookup).unwrap(),
        References::from([b.clone(), d.clone()])
    );
    assert_eq!(
        closure([d.clone()], lookup).unwrap(),
        References::from([d.clone()])
    );
    assert_eq!(
        closure([b.clone(), c.clone()], lookup).unwrap(),
        References::from([b, c, d])
    );
    assert!(closure(Vec::new(), lookup).unwrap().is_empty());
}

#[test]
fn a_missing_reference_names_the_path_and_its_referrer() {
    let (mut objects, [a, b, c, d]) = graph();
    objects.remove(&d);
    let err = closure([a.clone()], |p: &StorePath| objects.get(p).cloned()).unwrap_err();
    match err {
        Error::MissingReference(missing) => {
            assert_eq!(missing.path, d);
            assert!(
                missing.referrer == Some(b.clone()) || missing.referrer == Some(c.clone()),
                "{:?}",
                missing.referrer
            );
            let referrer = missing.referrer.clone().unwrap();
            assert_eq!(
                Error::from(*missing).to_string(),
                format!("{d} is referenced by {referrer} but is not in the store")
            );
        }
        other => panic!("{other}"),
    }
    let err = closure([d.clone()], |p: &StorePath| objects.get(p).cloned()).unwrap_err();
    assert!(
        matches!(&err, Error::MissingReference(missing) if missing.path == d && missing.referrer.is_none()),
        "{err}"
    );
    assert_eq!(err.to_string(), format!("{d} is not in the store"));
}

#[test]
fn a_cycle_in_the_lookup_terminates() {
    // the derivation cannot produce one; a lookup that lies still must not hang
    let a = leaf("a");
    let b = leaf("b");
    let objects = BTreeMap::from([
        (a.clone(), References::from([b.clone()])),
        (b.clone(), References::from([a.clone()])),
    ]);
    assert_eq!(
        closure([a.clone()], |p: &StorePath| objects.get(p).cloned()).unwrap(),
        References::from([a, b])
    );
}

#[test]
fn errors_display_their_reason() {
    let err = Name::new("Foo").unwrap_err();
    assert!(err.to_string().starts_with("malformed name: "), "{err}");
    let err = store().parse("/elsewhere/x").unwrap_err();
    assert_eq!(
        err.to_string(),
        "/elsewhere/x is not under the store directory /pub/store"
    );
}
