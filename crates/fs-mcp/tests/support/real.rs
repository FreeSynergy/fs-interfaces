//! Loading the real contracts and marks for the tests that guard them.
//! Pattern: none (helpers).
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use fs_mcp::{
    Blocklist, CapnpcSource, Catalog, Contract, ContractSource, Generator, MarkFiles, RiskSource,
    ToolGenerator,
};

pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
pub fn schema_dir() -> PathBuf {
    repo().join("schema")
}
pub fn marks_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("marks")
}
pub fn contracts() -> Vec<Contract> {
    CapnpcSource::new(schema_dir())
        .contracts()
        .expect("the real schemas compile")
}
pub fn marks() -> MarkFiles {
    MarkFiles::from_dir(marks_dir()).expect("the mark files are valid")
}
pub fn generate_with(
    contracts: &[Contract],
    risk: &dyn RiskSource,
    blocklist: Blocklist,
) -> Catalog {
    Generator::new(risk, blocklist)
        .generate(contracts)
        .expect("generation works")
}
pub fn real_catalog() -> Catalog {
    generate_with(&contracts(), &marks(), Blocklist::standard())
}
