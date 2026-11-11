//! The committed generated code must equal what the official compiler (`capnpc-embedded`, capnp
//! 1.4.0 as WASM) produces, and what our own compiler (`capnp-compiler`) produces. This keeps the
//! committed files honest and tests our compiler against the official one on the real schemas.

use std::path::{Path, PathBuf};

const SCHEMAS: [&str; 18] = [
    "common",
    "tasks",
    "messenger",
    "hub",
    "people",
    "groups",
    "files",
    "events",
    "places",
    "texts",
    "surveys",
    "history",
    "resources",
    "whiteboard",
    "streaming",
    "store",
    "types",
    "comments",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn generate(request: &[u8], tag: &str) -> PathBuf {
    let out = std::env::temp_dir().join(format!("fs-interfaces-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    capnpc::codegen::CodeGenerationCommand::new()
        .output_directory(&out)
        .run(request)
        .unwrap();
    out
}

fn assert_same(dir: &Path, who: &str) {
    for s in SCHEMAS {
        let name = format!("{s}_capnp.rs");
        let committed = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join(&name),
        )
        .unwrap();
        let fresh = std::fs::read_to_string(dir.join(&name)).unwrap();
        assert!(
            committed == fresh,
            "{name} differs from the output of {who}"
        );
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn official_compiler_output_is_committed() {
    let schema = root().join("schema");
    let mut cmd = capnpc_embedded::CompileCommand::new();
    for s in SCHEMAS {
        cmd.file(schema.join(format!("{s}.capnp")));
    }
    let request = cmd.src_prefix(&schema).compile().expect("official compile");
    assert_same(&generate(&request, "official"), "capnpc-embedded");
}

#[test]
fn own_compiler_output_is_committed() {
    let schema = root().join("schema");
    let mut cmd = capnp_compiler::CompileCommand::new();
    cmd.src_prefix(&schema);
    for s in SCHEMAS {
        cmd.file(schema.join(format!("{s}.capnp")));
    }
    let request = cmd.run_bytes().expect("own compile");
    assert_same(&generate(&request, "own"), "capnp-compiler");
}
