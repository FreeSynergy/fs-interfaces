//! Helpers that build and run synthetic contracts. Test helpers; no pattern of their own.
#![allow(dead_code)]

use super::{FILE, FakeMarks};
use fs_mcp::Contract;

/// One synthetic contract file named like the messenger.
pub fn contract(
    interfaces: Vec<fs_mcp::Interface>,
    structs: Vec<fs_mcp::StructDef>,
) -> Vec<Contract> {
    vec![Contract {
        file: FILE.into(),
        interfaces,
        structs,
        enums: vec![],
    }]
}
/// Generate with the standard block list.
pub fn run(c: &[Contract], marks: &FakeMarks) -> fs_mcp::Catalog {
    use fs_mcp::ToolGenerator;
    fs_mcp::Generator::new(marks, fs_mcp::Blocklist::standard())
        .generate(c)
        .unwrap()
}
/// Marks of class `read` for the given `(owner, item)` pairs.
pub fn read_all(items: &[(&str, &str)]) -> FakeMarks {
    let mut m = FakeMarks::default();
    for (o, i) in items {
        m.put(o, i, fs_mcp::RiskClass::Read);
    }
    m
}
/// Did the catalog produce a tool for this method or member?
pub fn has_tool(c: &fs_mcp::Catalog, owner: &str, item: &str) -> bool {
    c.tools
        .iter()
        .any(|t| t.source == fs_mcp::Target::item(FILE, owner, item))
}
