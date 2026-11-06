//! How a program's server starts: its arguments and where its data lives.
//!
//! Pattern: Factory — [`ServerOptions`] from the command line and the environment, the same for every
//! program. The data directory holds people's data, so it is treated like the socket: 0700, and an
//! existing one others can reach is refused.

use std::ffi::OsString;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use super::socket::{check_ancestors, create_private, default_socket};

/// The acting person when none is named: the owner of this device.
pub const LOCAL_OWNER: &str = "https://local.invalid/users/owner";

/// What a program's server was started with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerOptions {
    /// The socket to listen on.
    pub socket: PathBuf,
    /// The data directory. Pass it through [`prepare_data_dir`] and use what that answers.
    pub data: PathBuf,
    /// IRI of the acting person, written to the log. Not a verified identity.
    pub actor: String,
    /// Instance domain for the IRIs of new objects (ADR 0027); `None` = the program's default.
    pub instance: Option<String>,
}

/// `$XDG_DATA_HOME/freesynergy/<program>`, else below `$HOME/.local/share`.
#[must_use]
pub fn default_data_dir(program: &str) -> Option<PathBuf> {
    let set = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    let base = set("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| set("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(base.join("freesynergy").join(program))
}

/// A domain as an IRI can carry it: labels of ASCII letters, digits and inner hyphens, 63 bytes each,
/// 253 in all, without a port. The name ends up in `https://<instance>/...`, so anything else would
/// make every new object's IRI wrong, or point it elsewhere. An international name comes as `xn--`.
fn is_domain(name: &str) -> bool {
    name.len() <= 253
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

/// Create the data directory for this user alone; refuse one that others can reach, or one below a
/// directory that others could swap. Answers the canonical path, the one that was checked and is the
/// one to use.
///
/// # Errors
/// Others have access (`PermissionDenied`), or the file system refuses.
pub fn prepare_data_dir(dir: &Path) -> io::Result<PathBuf> {
    create_private(dir)?;
    let dir = dir.canonicalize()?;
    let meta = std::fs::metadata(&dir)?;
    if meta.uid() != rustix::process::geteuid().as_raw() || meta.mode() & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{} must belong to this user alone (0700)", dir.display()),
        ));
    }
    check_ancestors(&dir)?;
    Ok(dir)
}

impl ServerOptions {
    /// Read `--socket PATH --data DIR --actor IRI --instance DOMAIN`. `Ok(None)` means help was asked for.
    ///
    /// # Errors
    /// An unknown argument, a missing or empty value, an instance that is no domain, or a location
    /// that has no default here.
    pub fn parse(
        program: &str,
        args: impl IntoIterator<Item = OsString>,
    ) -> Result<Option<Self>, String> {
        let (mut socket, mut data, mut actor, mut instance) = (None, None, None, None);
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let name = arg.to_string_lossy().into_owned();
            let slot = match name.as_str() {
                "--socket" => &mut socket,
                "--data" => &mut data,
                "--actor" => &mut actor,
                "--instance" => &mut instance,
                "-h" | "--help" => return Ok(None),
                other => return Err(format!("unknown argument {other}")),
            };
            let value = args
                .next()
                .filter(|v| !v.is_empty() && !v.to_string_lossy().starts_with("--"))
                .ok_or_else(|| format!("{name} needs a value"))?;
            *slot = Some(value);
        }
        let socket = socket
            .map(PathBuf::from)
            .or_else(|| default_socket(program))
            .ok_or("no $XDG_RUNTIME_DIR: pass --socket")?;
        let data = data
            .map(PathBuf::from)
            .or_else(|| default_data_dir(program))
            .ok_or("no $XDG_DATA_HOME or $HOME: pass --data")?;
        let actor = actor.map_or_else(
            || LOCAL_OWNER.to_string(),
            |a| a.to_string_lossy().into_owned(),
        );
        let instance = instance.map(|i| i.to_string_lossy().into_owned());
        if let Some(name) = instance.as_deref().filter(|name| !is_domain(name)) {
            return Err(format!("--instance needs a domain, not {name}"));
        }
        Ok(Some(Self {
            socket,
            data,
            actor,
            instance,
        }))
    }

    /// [`ServerOptions::parse`] of the process arguments. Prints help and exits with 0, or prints the
    /// problem and exits with 2.
    #[must_use]
    pub fn from_env_or_exit(program: &str) -> Self {
        let binary = format!("fs-{program}-server");
        match Self::parse(program, std::env::args_os().skip(1)) {
            Ok(Some(options)) => options,
            Ok(None) => {
                println!(
                    "{binary} [--socket PATH] [--data DIR] [--actor IRI] [--instance DOMAIN]\nDefaults: $XDG_RUNTIME_DIR/freesynergy/{program}.sock, $XDG_DATA_HOME/freesynergy/{program}, the local owner"
                );
                std::process::exit(0);
            }
            Err(problem) => {
                eprintln!("{binary}: {problem}");
                std::process::exit(2);
            }
        }
    }
}
