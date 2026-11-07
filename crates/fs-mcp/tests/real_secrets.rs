//! Second line of defence on the real schemas: reads the finished JSON, the contracts and the release lists itself
//! and matches by plain substring, so a mistake in the generator's own checks cannot hide here.

mod support;

use fs_mcp::Target;
use support::real::{contracts, real_catalog};

/// Second, independent line of defence: it reads the finished JSON and the release lists itself, and matches by
/// plain substring, so a mistake in the generator's word splitting cannot hide here.
#[test]
fn every_secret_looking_property_has_a_written_release_for_exactly_that_field() {
    const NEEDLES: [&str; 15] = [
        "secret",
        "passw",
        "token",
        "key",
        "credential",
        "otp",
        "auth",
        "cookie",
        "session",
        "seed",
        "private",
        "signature",
        "proof",
        "mnemonic",
        "bearer",
    ];
    let mut releases = Vec::new();
    for e in std::fs::read_dir(support::real::marks_dir()).unwrap() {
        let text = std::fs::read_to_string(e.unwrap().path()).unwrap();
        let table: toml::Table = text.parse().unwrap();
        if let Some(toml::Value::Table(r)) = table.get("release") {
            releases.extend(r.keys().cloned());
        }
    }
    let contracts = support::real::contracts();
    let secretish = |p: &str| NEEDLES.iter().any(|n| p.to_lowercase().contains(n));
    for t in &real_catalog().tools {
        for schema in [&t.input_schema, &t.output_schema] {
            for p in schema["properties"]
                .as_object()
                .into_iter()
                .flat_map(|o| o.keys())
                .filter(|p| secretish(p))
            {
                // A method tool is released for exactly `Iface.method.param`; a union-member tool renames the
                // parameter after the member, so there only the last part can be matched.
                let src = t.source.as_str().split_once('/').map_or("", |(_, r)| r);
                let is_method = contracts.iter().flat_map(|c| &c.interfaces).any(|i| {
                    i.methods
                        .iter()
                        .any(|m| format!("{}.{}", i.name, m.name) == src)
                });
                let ok = if is_method {
                    releases.contains(&format!("{src}.{p}"))
                } else {
                    releases.iter().any(|k| k.ends_with(&format!(".{p}")))
                };
                assert!(ok, "{}: top-level {p} has no release", t.name);
            }
            for (def, body) in schema["$defs"].as_object().into_iter().flatten() {
                let strukt = def.split_once('.').map_or(def.as_str(), |(_, s)| s);
                for p in body["properties"]
                    .as_object()
                    .into_iter()
                    .flat_map(|o| o.keys())
                    .filter(|p| secretish(p))
                {
                    let ok = releases.contains(&format!("{strukt}.{p}"));
                    assert!(ok, "{}: {strukt}.{p} has no release", t.name);
                }
            }
        }
    }
}

/// The parameter name of a union parameter must be judged before the split: independent substring check.
#[test]
fn no_union_member_tool_hides_a_secret_looking_parameter_or_union_name() {
    const NEEDLES: [&str; 8] = [
        "secret",
        "passw",
        "token",
        "key",
        "credential",
        "proof",
        "bearer",
        "cookie",
    ];
    let secretish = |p: &str| NEEDLES.iter().any(|n| p.to_lowercase().contains(n));
    let contracts = contracts();
    let c = real_catalog();
    for (file, i, m, p, s) in contracts.iter().flat_map(|c| {
        c.interfaces.iter().flat_map(move |i| {
            i.methods.iter().flat_map(move |m| {
                m.params.iter().filter_map(move |p| match &p.ty {
                    fs_mcp::TypeRef::Struct(n) => c
                        .structs
                        .iter()
                        .chain(std::iter::empty())
                        .find(|s| s.name == n.name && s.is_pure_union())
                        .map(|s| {
                            (
                                c.file.clone(),
                                i.name.clone(),
                                m.name.clone(),
                                p.name.clone(),
                                s.clone(),
                            )
                        }),
                    _ => None,
                })
            })
        })
    }) {
        if secretish(&p) || secretish(&s.name) {
            for f in &s.fields {
                let target = Target::item(&file, &s.name, &f.name);
                assert!(
                    c.tools.iter().all(|t| t.source != target),
                    "{file}/{i}.{m}({p}) split into {}",
                    f.name
                );
            }
        }
    }
}
