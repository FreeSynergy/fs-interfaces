//! The [`ContractSource`] over `capnp-compiler`.
//!
//! Pattern: Adapter — wraps `capnp-compiler` and `capnp::schema_capnp` behind [`ContractSource`].
//! Trait: [`ContractSource`].

use std::path::{Path, PathBuf};

use capnp::schema_capnp::code_generator_request;
use capnp_compiler::CompileCommand;

use crate::{Contract, ContractSource, Error};

/// All `*.capnp` files of one directory, compiled together.
pub struct CapnpcSource {
    dir: PathBuf,
}

impl CapnpcSource {
    /// Contracts in `dir` (for example the `schema` directory of `fs-interfaces`).
    #[must_use]
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
        }
    }
}

impl ContractSource for CapnpcSource {
    fn contracts(&self) -> Result<Vec<Contract>, Error> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&self.dir)
            .map_err(|e| Error::Compile(format!("{}: {e}", self.dir.display())))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "capnp"))
            .collect();
        files.sort();
        let mut cmd = CompileCommand::new();
        cmd.src_prefix(&self.dir);
        for f in &files {
            cmd.file(f);
        }
        let bytes = cmd
            .run_bytes()
            .map_err(|e| Error::Compile(format!("{e:?}")))?;
        let message = capnp::serialize::read_message(
            &mut &bytes[..],
            capnp::message::ReaderOptions {
                traversal_limit_in_words: None,
                nesting_limit: 128,
            },
        )?;
        let request: code_generator_request::Reader<'_> = message.get_root()?;
        super::nodes::read_request(request)
    }
}
