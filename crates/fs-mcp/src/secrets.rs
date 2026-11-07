//! Recognising names and comments that look like secrets or like free-form content that could hold one.
//! Deliberately broad: a false alarm costs one written release, a miss costs a secret in a model's context.
//!
//! Pattern: Specification — pure predicates over a name, a comment and a type; no state. Trait: none.

use crate::TypeRef;

const SECRET_WORDS: &[&str] = &[
    "secret",
    "secrets",
    "password",
    "passwords",
    "passwd",
    "passphrase",
    "passcode",
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
    "totp",
    "hotp",
    "oauth",
    "auth",
    "authorization",
    "bearer",
    "cookie",
    "cookies",
    "session",
    "sessions",
    "pin",
    "mnemonic",
    "signature",
    "proof",
];
const DOC_WORDS: &[&str] = &[
    "secret",
    "secrets",
    "password",
    "passwords",
    "passphrase",
    "credential",
    "credentials",
    "token",
    "mnemonic",
];
const FREE_FORM_WORDS: &[&str] = &["json", "config", "proof", "payload", "blob", "data"];

/// Split `IMAPPassword` into `imap`, `password`; `publisherKey` into `publisher`, `key`; `a_b-c` at the signs.
fn words(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    for (i, &ch) in chars.iter().enumerate() {
        if !ch.is_alphanumeric() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            continue;
        }
        let prev_lower = i > 0 && (chars[i - 1].is_lowercase() || chars[i - 1].is_numeric());
        let acronym_end = i > 0
            && chars[i - 1].is_uppercase()
            && ch.is_uppercase()
            && chars.get(i + 1).is_some_and(|n| n.is_lowercase());
        if ch.is_uppercase() && (prev_lower || acronym_end) && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        cur.extend(ch.to_lowercase());
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn has_word(s: &str, list: &[&str]) -> bool {
    words(s).iter().any(|w| list.contains(&w.as_str()))
}

/// Does an identifier (`publisherKey`, `LoginCredentials`, `IMAPPassword`) contain a secret word?
#[must_use]
pub fn name_looks_secret(name: &str) -> bool {
    has_word(name, SECRET_WORDS)
}

/// Does a schema comment say it holds a secret?
#[must_use]
pub fn doc_looks_secret(doc: &str) -> bool {
    has_word(doc, DOC_WORDS)
}

/// Is this a place for content the generator cannot judge: `Data` anywhere in the type, or a name such as
/// `json`, `config`, `proof`, `payload`, `blob`, `data`?
#[must_use]
pub fn looks_free_form(name: &str, ty: Option<&TypeRef>) -> bool {
    fn has_data(t: &TypeRef) -> bool {
        match t {
            TypeRef::Data => true,
            TypeRef::List(inner) => has_data(inner),
            _ => false,
        }
    }
    let text = |t: &TypeRef| match t {
        TypeRef::Text => true,
        TypeRef::List(inner) => **inner == TypeRef::Text,
        _ => false,
    };
    ty.is_some_and(has_data) || (ty.is_some_and(text) && has_word(name, FREE_FORM_WORDS))
}
