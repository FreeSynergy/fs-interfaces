//! Prints every method and union member of the contracts with its parameters, one per line.
//! Pattern: none (example). Used to write and review the mark files.

use fs_mcp::{CapnpcSource, ContractSource};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "schema".into());
    for c in CapnpcSource::new(dir).contracts()? {
        for i in &c.interfaces {
            for m in &i.methods {
                let p: Vec<_> = m.params.iter().map(|f| f.name.as_str()).collect();
                let r: Vec<_> = m.results.iter().map(|f| f.name.as_str()).collect();
                println!(
                    "{}/{}.{} ({}) -> ({}){}",
                    c.file,
                    i.name,
                    m.name,
                    p.join(","),
                    r.join(","),
                    if m.streaming { " STREAM" } else { "" }
                );
            }
        }
    }
    Ok(())
}
