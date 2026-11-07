//! Recognising names and comments that look like secrets. Deliberately broad: a false alarm costs one
//! written release, a miss costs a secret in a model's context.
//!
//! Pattern: none (a pure function over word lists). Trait: none.

const NAME_WORDS: &[&str] = &[
    "secret",
    "secrets",
    "password",
    "passwords",
    "passwd",
    "passphrase",
    "pass",
    "token",
    "tokens",
    "key",
    "keys",
    "credential",
    "credentials",
    "apikey",
    "private",
    "seed",
    "otp",
    "auth",
    "cookie",
];
const DOC_WORDS: &[&str] = &[
    "secret",
    "secrets",
    "password",
    "passphrase",
    "credential",
    "credentials",
];

fn words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            if ch.is_uppercase() && prev_lower {
                out.push(std::mem::take(&mut cur));
            }
            prev_lower = ch.is_lowercase() || ch.is_numeric();
            cur.extend(ch.to_lowercase());
        } else {
            prev_lower = false;
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Does an identifier (`publisherKey`, `AccountSetup`, `inputTokens`) contain a secret word?
#[must_use]
pub fn name_looks_secret(name: &str) -> bool {
    words(name).iter().any(|w| NAME_WORDS.contains(&w.as_str()))
}

/// Does a schema comment say it holds a secret?
#[must_use]
pub fn doc_looks_secret(doc: &str) -> bool {
    words(doc).iter().any(|w| DOC_WORDS.contains(&w.as_str()))
}
