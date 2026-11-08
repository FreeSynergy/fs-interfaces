//! `EmailAddress`: the practical rule of the HTML "valid e-mail address", with international domains.
//!
//! Pattern: Value object — the local part follows the HTML rule (atext characters, dots inside), the
//! domain is a host name with at least one dot, labels of 1 to 63 letters, digits or hyphens, not
//! starting or ending with a hyphen. A Unicode domain is accepted. The domain is mapped as IDNA
//! does (lower case, look-alike forms folded, invisible characters dropped), so `as_str` holds it in
//! Unicode and `ascii` in punycode, and both are the same for every spelling of one domain.

use super::{ContactValue, ValueProblem};

/// A valid e-mail address. Two are equal when the part before the `@` is the same and the domain is
/// the same domain, whatever its case or whether it was typed in Unicode or in punycode.
#[derive(Clone, Debug)]
pub struct EmailAddress {
    text: String,
    ascii: String,
}

impl PartialEq for EmailAddress {
    fn eq(&self, other: &Self) -> bool {
        self.ascii == other.ascii
    }
}

impl Eq for EmailAddress {}

impl std::hash::Hash for EmailAddress {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ascii.hash(state);
    }
}

impl EmailAddress {
    /// The address with the domain in ASCII (`user@xn--bcher-kva.example`).
    #[must_use]
    pub fn ascii(&self) -> &str {
        &self.ascii
    }
}

fn atext(c: char) -> bool {
    c.is_ascii_alphanumeric() || "!#$%&'*+/=?^_`{|}~-".contains(c)
}

fn local_ok(local: &str) -> Result<(), ValueProblem> {
    if local.is_empty() {
        return Err(ValueProblem::EmptyLocal);
    }
    if local.chars().count() > 64 {
        return Err(ValueProblem::LocalTooLong);
    }
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return Err(ValueProblem::LocalCharacter);
    }
    if local.chars().any(|c| c != '.' && !atext(c)) {
        return Err(ValueProblem::LocalCharacter);
    }
    Ok(())
}

fn label_ok(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 63
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn domain_ascii(domain: &str) -> Result<String, ValueProblem> {
    if domain.is_empty() {
        return Err(ValueProblem::EmptyDomain);
    }
    let ascii = idna::domain_to_ascii(domain).map_err(|_| ValueProblem::BadDomain)?;
    if ascii.len() > 253 || !ascii.split('.').all(label_ok) {
        return Err(ValueProblem::BadDomain);
    }
    if !ascii.contains('.') {
        return Err(ValueProblem::DomainWithoutDot);
    }
    Ok(ascii)
}

impl ContactValue for EmailAddress {
    fn parse(typed: &str) -> Result<Self, ValueProblem> {
        let typed = typed.trim();
        if typed.is_empty() {
            return Err(ValueProblem::Empty);
        }
        let mut parts = typed.split('@');
        let (Some(local), Some(domain), rest) = (parts.next(), parts.next(), parts.next()) else {
            return Err(ValueProblem::MissingAt);
        };
        if rest.is_some() {
            return Err(ValueProblem::ManyAt);
        }
        local_ok(local)?;
        let ascii_domain = domain_ascii(domain)?;
        let ascii = format!("{local}@{ascii_domain}");
        if ascii.len() > 254 {
            return Err(ValueProblem::AddressTooLong);
        }
        let (unicode, _) = idna::domain_to_unicode(&ascii_domain);
        Ok(Self {
            text: format!("{local}@{unicode}"),
            ascii,
        })
    }

    fn as_str(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_and_plus_addresses_are_valid() {
        for ok in [
            "ada@example.org",
            "a.b+tag@mail.example.co.uk",
            " x_y@a-b.de ",
        ] {
            assert!(EmailAddress::check(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn each_rule_has_its_own_problem() {
        let cases = [
            ("", ValueProblem::Empty),
            ("ada", ValueProblem::MissingAt),
            ("a@b@c.de", ValueProblem::ManyAt),
            ("@example.org", ValueProblem::EmptyLocal),
            ("ada@", ValueProblem::EmptyDomain),
            ("a b@example.org", ValueProblem::LocalCharacter),
            (".a@example.org", ValueProblem::LocalCharacter),
            ("a..b@example.org", ValueProblem::LocalCharacter),
            ("ada@localhost", ValueProblem::DomainWithoutDot),
            ("ada@-bad.example", ValueProblem::BadDomain),
            ("ada@exa mple.org", ValueProblem::BadDomain),
        ];
        for (typed, problem) in cases {
            assert_eq!(EmailAddress::check(typed), Err(problem), "{typed:?}");
        }
        let long = format!("{}@example.org", "a".repeat(65));
        assert_eq!(EmailAddress::check(&long), Err(ValueProblem::LocalTooLong));
    }

    #[test]
    fn an_international_domain_has_a_unicode_and_an_ascii_form() {
        let a = EmailAddress::parse("ada@bücher.example").unwrap();
        assert_eq!(a.as_str(), "ada@bücher.example");
        assert_eq!(a.ascii(), "ada@xn--bcher-kva.example");
    }

    #[test]
    fn spellings_of_one_domain_are_one_address() {
        let one = EmailAddress::parse("ada@Example.ORG").unwrap();
        assert_eq!(one, EmailAddress::parse("ada@example.org").unwrap());
        assert_eq!(one.as_str(), "ada@example.org");
        let unicode = EmailAddress::parse("ada@bücher.example").unwrap();
        assert_eq!(
            unicode,
            EmailAddress::parse("ada@xn--bcher-kva.example").unwrap()
        );
        assert_eq!(unicode, EmailAddress::parse("ada@BÜCHER.example").unwrap());
        // Invisible and look-alike characters are mapped away, not kept.
        let soft = EmailAddress::parse("ada@exam\u{00AD}ple.org").unwrap();
        assert_eq!(soft.as_str(), "ada@example.org");
        let wide = EmailAddress::parse("ada@ｅｘａｍｐｌｅ.org").unwrap();
        assert_eq!(wide.as_str(), "ada@example.org");
        // The part before the @ stays as typed, and is not folded.
        assert_ne!(one, EmailAddress::parse("Ada@example.org").unwrap());
    }

    #[test]
    fn the_limits_hold_at_their_edges() {
        let label = |n: usize| "a".repeat(n);
        assert!(EmailAddress::check(&format!("x@{}.org", label(63))).is_ok());
        assert_eq!(
            EmailAddress::check(&format!("x@{}.org", label(64))),
            Err(ValueProblem::BadDomain)
        );
        assert!(EmailAddress::check(&format!("{}@example.org", label(64))).is_ok());
        let domain = format!("{0}.{0}.{0}.{1}", label(63), label(61)); // 253 characters
        assert_eq!(domain.len(), 253);
        assert_eq!(
            EmailAddress::check(&format!("{}@{domain}", label(64))),
            Err(ValueProblem::AddressTooLong)
        );
        // 1 + 1 + 253 is one more than the 254 of RFC 5321; one character less is fine.
        assert_eq!(
            EmailAddress::check(&format!("{}@{domain}", label(1))),
            Err(ValueProblem::AddressTooLong)
        );
        let shorter = format!("{0}.{0}.{0}.{1}", label(63), label(60));
        assert!(EmailAddress::check(&format!("{}@{shorter}", label(1))).is_ok());
    }
}
