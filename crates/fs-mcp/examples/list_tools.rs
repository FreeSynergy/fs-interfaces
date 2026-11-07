//! Lists what the generator makes of the real contracts: the tool and resource lists as JSON on stdout,
//! and on stderr everything left out, every missing mark and every stale mark.
//!
//! ```text
//! cargo run -p fs-mcp --example list_tools -- schema crates/fs-mcp/marks
//! ```
//! Pattern: none (example). Stage 1 runs nothing: this only prints the lists.

use fs_mcp::{Blocklist, CapnpcSource, ContractSource, Generator, MarkFiles, ToolGenerator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let schema = args.next().unwrap_or_else(|| "schema".into());
    let marks = args.next().unwrap_or_else(|| "crates/fs-mcp/marks".into());
    let contracts = CapnpcSource::new(schema).contracts()?;
    let marks = MarkFiles::from_dir(marks)?;
    let catalog = Generator::new(&marks, Blocklist::standard()).generate(&contracts)?;
    for l in &catalog.left_out {
        eprintln!("left out  {}  {:?}", l.target, l.why);
    }
    for t in &catalog.unmarked {
        eprintln!("no mark   {t}");
    }
    for t in &catalog.stale {
        eprintln!("stale     {t}");
    }
    println!("{}", serde_json::to_string_pretty(&catalog.to_json())?);
    Ok(())
}
