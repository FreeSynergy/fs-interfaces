//! How a program starts its own server. Pattern: Strategy (the program provides it) + Command (`CommandStarter`).

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::Command;

/// How a program starts its own server (the one `fs-<program> serve` runs), detached: it must go on when the caller ends.
/// The program provides it; the closure `|| -> io::Result<()>` and [`CommandStarter`] do.
pub trait ServerStarter {
    /// Starts the server and returns without waiting for it.
    ///
    /// # Errors
    /// The server could not be started.
    fn start(&self) -> io::Result<()>;
}

impl<F: Fn() -> io::Result<()>> ServerStarter for F {
    fn start(&self) -> io::Result<()> {
        self()
    }
}

/// Starts a server by running a command (usually the program's own executable with `serve`), detached from the caller:
/// no input or output, its own process group on Unix.
pub struct CommandStarter {
    program: PathBuf,
    args: Vec<OsString>,
}

impl CommandStarter {
    /// The command `program args...`.
    pub fn new<I, S>(program: impl Into<PathBuf>, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }
}

impl ServerStarter for CommandStarter {
    fn start(&self) -> io::Result<()> {
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut command, 0);
        #[cfg(windows)]
        std::os::windows::process::CommandExt::creation_flags(&mut command, 0x0000_0008); // DETACHED_PROCESS
        // Not waited for: the server outlives this call. (A child that ends early is reaped by the system when we end.)
        command.spawn().map(drop)
    }
}
