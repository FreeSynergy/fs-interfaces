//! Output rules on the real schemas: administration and provider interfaces, resources, hints.

mod support;

use support::real::real_catalog;

#[test]
fn admin_and_provider_interfaces_have_no_tool_and_no_handle() {
    let c = real_catalog();
    for gone in [
        "groups.delegations",
        "groups.memberships",
        "groups.invitations",
        "people.roles",
        "files.backends",
        "files.sharing",
        "files.blobBackend",
        "groups.groupDirectory",
        "hub.session",
        "places.tileSource",
        "people.people.roles",
        "groups.groups.delegations",
        "groups.groups.memberships",
        "messenger.accounts",
        "messenger.messenger.accounts",
        "events.invitations",
        "events.events.invitations",
        "groups.groups.invitations",
        "files.files.backends",
        "files.files.sharing",
        "places.places.tiles",
        "hub.hub.register",
        "groups.items.removeParent",
        "groups.items.addParent",
        "whiteboard.boardEditor.add",
    ] {
        assert!(
            c.tools
                .iter()
                .all(|t| t.name != gone && !t.name.starts_with(&format!("{gone}."))),
            "{gone} is offered"
        );
    }
}

#[test]
fn every_resource_is_class_read_and_uris_are_unique() {
    let c = real_catalog();
    assert!(
        c.resources
            .iter()
            .all(|r| r.risk == fs_mcp::RiskClass::Read)
    );
    let mut uris: Vec<_> = c.resources.iter().map(|r| &r.uri_template).collect();
    uris.sort();
    uris.dedup();
    assert_eq!(uris.len(), c.resources.len());
}

#[test]
fn nothing_is_read_only_and_every_tool_carries_its_class() {
    let json = real_catalog().to_json();
    for t in json["tools"].as_array().unwrap() {
        assert_eq!(t["annotations"]["readOnlyHint"], false, "{}", t["name"]);
        assert!(t["_meta"]["net.freesynergy/mcp"]["risk"].is_string());
        assert_eq!(
            t["_meta"]["net.freesynergy/mcp"]["needsRelease"], true,
            "reading is not harmless either"
        );
    }
    assert!(!json.to_string().contains("\"readOnlyHint\":true"));
}

#[test]
fn events_and_blobs_are_resources_and_listeners_are_no_tools() {
    let c = real_catalog();
    assert!(c.resources.iter().any(|r| {
        r.uri_template
            .starts_with("fs://messenger/Messenger/events")
    }));
    assert!(
        c.resources
            .iter()
            .any(|r| r.uri_template.starts_with("fs://common/Blob/{handle}/read"))
    );
    assert!(
        c.tools
            .iter()
            .all(|t| !t.name.contains("subscribe") && !t.name.contains("listener"))
    );
    assert!(
        c.tools.iter().all(|t| t.name != "common.blob.read"),
        "bytes are a resource"
    );
}

#[test]
fn every_method_of_accounts_and_event_invitations_is_excluded_one_by_one() {
    let c = real_catalog();
    for (file, iface) in [("messenger", "Accounts"), ("events", "Invitations")] {
        let contracts = support::real::contracts();
        let i = contracts
            .iter()
            .filter(|x| x.file == file)
            .flat_map(|x| &x.interfaces)
            .find(|i| i.name == iface)
            .unwrap();
        assert!(!i.methods.is_empty());
        for m in &i.methods {
            let src = fs_mcp::Target::item(file, iface, &m.name);
            assert!(c.tools.iter().all(|t| t.source != src), "{src} is a tool");
        }
    }
}
