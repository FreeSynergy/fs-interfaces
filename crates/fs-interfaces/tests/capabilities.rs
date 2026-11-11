//! The capability list (ADR 0068): one file, read here; the names are well formed, unique and the constants are on it.
use fs_interfaces::capabilities::{self, name};

#[test]
fn the_list_is_well_formed_and_unique() {
    let all: Vec<_> = capabilities::all().collect();
    assert!(all.len() >= 20);
    let mut seen = std::collections::BTreeSet::new();
    for c in &all {
        assert!(capabilities::is_well_formed(c.name), "{}", c.name);
        assert!(!c.description.is_empty(), "{} has no description", c.name);
        assert!(seen.insert(c.name), "{} twice", c.name);
    }
}

#[test]
fn constants_are_on_the_list() {
    for n in [
        name::PEOPLE_PERSONS,
        name::PEOPLE_USERS,
        name::GROUPS,
        name::COMMENTS,
        name::DEPENDENCY_EVALUATE,
        name::STORAGE_PROVIDER,
        name::IDENTITY_OIDC,
        name::IDENTITY_ACCOUNTS,
        name::IDENTITY_LDAP,
        name::IDENTITY_SCIM,
        name::IDENTITY_PAM,
        name::IDENTITY_RADIUS,
    ] {
        assert!(capabilities::is_known(n), "{n}");
    }
    assert!(!capabilities::is_known("people.persons.x"));
    assert!(!capabilities::is_well_formed("People"));
    assert!(!capabilities::is_well_formed("a..b"));
    assert!(!capabilities::is_well_formed(""));
}
