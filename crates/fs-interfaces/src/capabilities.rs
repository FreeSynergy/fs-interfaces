//! The names of capabilities (ADR 0068): one list, here. A package in the catalog says what it provides, requires and
//! recommends by these names (`store.capnp`, `CapabilityRef`); the store checks a name against this list, and the catalog
//! generator reads the same file (`schema/capabilities.txt`), so there is no second copy. Pattern: Registry (a fixed table).
//!
//! A name that is not in the list is not an error for a reader (a newer catalog may know more): [`is_known`] tells, and the
//! store shows the name as it is. Names are only appended, never renamed or removed.

/// The list as written in `schema/capabilities.txt`, for programs and scripts that read the file themselves.
pub const SOURCE: &str = include_str!("../../../schema/capabilities.txt");

/// A capability name and what it means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capability {
    /// Lower-case dotted name, for example `people.persons`.
    pub name: &'static str,
    /// One line for people.
    pub description: &'static str,
}

/// Names the code of this repository refers to.
pub mod name {
    /// A directory of persons (JSContact).
    pub const PEOPLE_PERSONS: &str = "people.persons";
    /// Persons that can sign in.
    pub const PEOPLE_USERS: &str = "people.users";
    /// Groups, membership, delegation.
    pub const GROUPS: &str = "groups";
    /// Comments on any object.
    pub const COMMENTS: &str = "comments";
    /// Readiness of tasks from their dependencies.
    pub const DEPENDENCY_EVALUATE: &str = "dependency.evaluate";
    /// A storage provider.
    pub const STORAGE_PROVIDER: &str = "storage.provider";
    /// Sign-in with `OpenID` Connect.
    pub const IDENTITY_OIDC: &str = "identity.oidc";
    /// Accounts and credentials.
    pub const IDENTITY_ACCOUNTS: &str = "identity.accounts";
    /// A directory over LDAP.
    pub const IDENTITY_LDAP: &str = "identity.ldap";
    /// Provisioning over SCIM.
    pub const IDENTITY_SCIM: &str = "identity.scim";
    /// System sign-in through PAM.
    pub const IDENTITY_PAM: &str = "identity.pam";
    /// Network access over RADIUS.
    pub const IDENTITY_RADIUS: &str = "identity.radius";
}

/// Every capability of the list, in file order.
pub fn all() -> impl Iterator<Item = Capability> {
    SOURCE.lines().filter_map(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let (name, description) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        Some(Capability {
            name,
            description: description.trim(),
        })
    })
}

/// Whether `name` is on the list.
#[must_use]
pub fn is_known(name: &str) -> bool {
    all().any(|c| c.name == name)
}

/// Whether `name` has the form of a capability name: lower-case parts of `a-z`, digits and `-`, joined by dots, each part
/// starting with a letter. Says nothing about the list; for names a catalog brings that this version does not know.
#[must_use]
pub fn is_well_formed(name: &str) -> bool {
    !name.is_empty()
        && name.split('.').all(|part| {
            part.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
}
