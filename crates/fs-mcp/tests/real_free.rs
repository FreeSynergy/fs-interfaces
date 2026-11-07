//! Free-format fields on the real schemas: each one carries a written mark, and the list is machine-readable.

mod support;

use fs_mcp::{RiskSource, Target, looks_free_format};
use support::real::{contracts, marks, real_catalog};

#[test]
fn every_free_format_field_of_the_contracts_has_a_format_mark() {
    let m = marks();
    let mut missing = Vec::new();
    for c in contracts() {
        for s in &c.structs {
            for f in &s.fields {
                if looks_free_format(&f.name, Some(&f.ty))
                    && m.format(&Target::item(&c.file, &s.name, &f.name)).is_none()
                {
                    missing.push(format!("{}/{}.{}", c.file, s.name, f.name));
                }
            }
        }
        for i in &c.interfaces {
            for me in &i.methods {
                for f in me.params.iter().chain(&me.results) {
                    let t = Target::item(&c.file, &i.name, &format!("{}.{}", me.name, f.name));
                    if looks_free_format(&f.name, Some(&f.ty)) && m.format(&t).is_none() {
                        missing.push(t.to_string());
                    }
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "free fields without a format mark: {missing:?}"
    );
}

#[test]
fn the_tools_carry_their_free_fields_and_the_dangerous_rule_setter_is_out() {
    let c = real_catalog();
    let with: Vec<_> = c
        .tools
        .iter()
        .filter(|t| !t.free_content.is_empty())
        .collect();
    assert!(with.len() > 50, "{}", with.len());
    assert!(
        c.tools.iter().all(|t| t.name != "messenger.rules.set"),
        "silent forwarding of all mail"
    );
    assert!(c.tools.iter().any(|t| {
        t.free_content
            .iter()
            .any(|(p, f)| p.ends_with("bodyText") && !f.is_empty())
    }));
}
