//! What happens when someone edits a contract and forgets the marks: the edit is made on a copy of the
//! real schemas. Fail-closed is the point. Pattern: none (tests).

mod support;

use std::path::PathBuf;

use fs_mcp::{Blocklist, CapnpcSource, ContractSource, RiskClass, Target, Why};
use support::real::{generate_with, marks, schema_dir};

/// A copy of `schema/` with `extra` appended to `messenger.capnp`, removed again on drop.
struct Edited(PathBuf);

impl Edited {
    fn new(tag: &str, extra: &str) -> Self {
        let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("fs-mcp-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for e in std::fs::read_dir(schema_dir()).unwrap() {
            let p = e.unwrap().path();
            std::fs::copy(&p, dir.join(p.file_name().unwrap())).unwrap();
        }
        let m = dir.join("messenger.capnp");
        let text = std::fs::read_to_string(&m).unwrap();
        std::fs::write(&m, format!("{text}\n{extra}\n")).unwrap();
        Self(dir)
    }
}

impl Drop for Edited {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_new_interface_without_marks_is_listed_and_counts_as_unknown() {
    let e = Edited::new(
        "new-interface",
        "interface Extra { wipe @0 () -> (); look @1 () -> (text :Text); }\ninterface Messenger2 { extra @0 () -> (extra :Extra); }",
    );
    let contracts = CapnpcSource::new(&e.0).contracts().unwrap();
    let c = generate_with(&contracts, &marks(), Blocklist::standard());
    for m in ["wipe", "look"] {
        let t = Target::item("messenger", "Extra", m);
        assert!(c.unmarked.contains(&t), "{t} is not listed as unmarked");
        let tool = c
            .tools
            .iter()
            .find(|x| x.source == t)
            .expect("still a tool, as unknown");
        assert_eq!(tool.risk, RiskClass::Unknown);
        assert!(tool.risk.asks_always());
    }
}

#[test]
fn a_new_guard_interface_never_becomes_a_tool_even_when_marked_read() {
    let e = Edited::new(
        "guard",
        "interface Guard { allow @0 (what :Text) -> (); }\ninterface Holder { guard @0 () -> (guard :Guard); }",
    );
    let contracts = CapnpcSource::new(&e.0).contracts().unwrap();
    let mark_all =
        fs_mcp::MarkFiles::from_sources(&[("messenger", "[Guard]\nallow = \"read\"\n")]).unwrap();
    let c = generate_with(&contracts, &mark_all, Blocklist::standard());
    assert!(
        c.tools
            .iter()
            .all(|t| !t.source.as_str().starts_with("messenger/Guard"))
    );
    assert!(c.left_out.iter().any(
        |l| l.target == Target::owner("messenger", "Guard") && matches!(l.why, Why::Blocked(_))
    ));
}

#[test]
fn a_new_secret_field_is_excluded_until_somebody_releases_it() {
    let extra = "struct Link { url @0 :Text; apiToken @1 :Text; }\ninterface Links { open @0 (link :Link) -> (); }";
    let e = Edited::new("secret", extra);
    let contracts = CapnpcSource::new(&e.0).contracts().unwrap();
    let marked =
        fs_mcp::MarkFiles::from_sources(&[("messenger", "[Links]\nopen = \"read\"\n")]).unwrap();
    let c = generate_with(&contracts, &marked, Blocklist::standard());
    let t = Target::item("messenger", "Links", "open");
    assert!(
        c.tools.iter().all(|x| x.source != t),
        "a secret-looking field reached a tool"
    );
    assert!(c.left_out.iter().any(|l| l.target == t
        && matches!(&l.why, Why::SecretSuspect { path } if path.ends_with("Link.apiToken"))));
    // Released in writing, it comes through.
    let released = fs_mcp::MarkFiles::from_sources(&[(
        "messenger",
        "[Links]\nopen = \"read\"\n[release]\n\"Link.apiToken\" = \"test\"\n",
    )])
    .unwrap();
    let c = generate_with(&contracts, &released, Blocklist::standard());
    assert!(c.tools.iter().any(|x| x.source == t));
}

#[test]
fn a_mark_that_names_a_removed_method_is_reported_as_stale() {
    let stale = fs_mcp::MarkFiles::from_sources(&[("messenger", "[Draft]\nteleport = \"read\"\n")])
        .unwrap();
    let contracts = support::real::contracts();
    let c = generate_with(&contracts, &stale, Blocklist::standard());
    assert_eq!(
        c.stale,
        vec![Target::item("messenger", "Draft", "teleport")]
    );
}

#[test]
fn review_counterexamples_on_a_copy_of_the_real_schemas() {
    let extra = "struct Wrap { struct ToolCallDecision { option @0 :Text; } d @0 :ToolCallDecision; }\n\
        interface LabE { wrapped @0 (w :Wrap) -> (); }\n\
        interface SecretStore { reveal @0 () -> (text :Text); }\n\
        interface LabF { open @0 () -> (store :SecretStore); }\n\
        interface LabG { take @0 (source :C.Blob) -> (); }";
    let e = Edited::new("review2", extra);
    let contracts = CapnpcSource::new(&e.0).contracts().unwrap();
    let c = generate_with(&contracts, &marks(), Blocklist::standard());
    let tool = |o: &str, m: &str| {
        c.tools
            .iter()
            .any(|t| t.source == Target::item("messenger", o, m))
    };
    assert!(
        !tool("LabE", "wrapped"),
        "a nested ToolCallDecision got through"
    );
    assert!(
        !tool("SecretStore", "reveal") && !tool("LabF", "open"),
        "an interface named like a secret got through"
    );
    assert!(
        tool("LabG", "take"),
        "a handle passed in is an ordinary argument"
    );
    assert!(
        c.resources.iter().all(|r| !r.uri_template.contains("LabG")),
        "a handle passed in is no listener"
    );
}
