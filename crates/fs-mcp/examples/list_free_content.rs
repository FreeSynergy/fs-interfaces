//! Prints, as JSON, every tool with the free-format fields it can carry: `{tool, field, format}` per entry.
//! These tools can transport secrets that somebody wrote into specialist text (notes, links, mail text); the
//! protection is the engine's alone. Pattern: none (example).

use fs_mcp::{Blocklist, CapnpcSource, ContractSource, Generator, MarkFiles, ToolGenerator};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let schema = args.next().unwrap_or_else(|| "schema".into());
    let marks = args.next().unwrap_or_else(|| "crates/fs-mcp/marks".into());
    let contracts = CapnpcSource::new(schema).contracts()?;
    let marks = MarkFiles::from_dir(marks)?;
    let catalog = Generator::new(&marks, Blocklist::standard()).generate(&contracts)?;
    let rows: Vec<_> = catalog
        .tools
        .iter()
        .flat_map(|t| {
            t.free_content
                .iter()
                .map(move |(f, k)| json!({"tool": t.name, "field": f, "format": k}))
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}
