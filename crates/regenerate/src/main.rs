//! Regenerates `crates/fs-interfaces/src/*_capnp.rs` from `schema/*.capnp` with our own pure-Rust compiler
//! (`capnp-compiler`, ADR 0040). No C++ toolchain needed. Run from the repository root.
//!
//! ```text
//! cargo run -p regenerate
//! ```

use capnp_compiler::CompileCommand;

const SCHEMAS: [&str; 4] = ["common", "tasks", "messenger", "hub"];

fn main() {
    let mut cmd = CompileCommand::new();
    cmd.src_prefix("schema");
    for s in SCHEMAS {
        cmd.file(format!("schema/{s}.capnp"));
    }
    let request = cmd.run_bytes().expect("schema compilation failed");
    std::fs::create_dir_all("crates/fs-interfaces/src").expect("create src");
    capnpc::codegen::CodeGenerationCommand::new()
        .output_directory("crates/fs-interfaces/src")
        .run(&request[..])
        .expect("code generation failed");
    println!(
        "regenerated {} files in crates/fs-interfaces/src/",
        SCHEMAS.len()
    );
}
