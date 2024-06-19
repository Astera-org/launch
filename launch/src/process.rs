use std::{ffi::OsStr, fmt, io, num::NonZeroI32, process};

use log::debug;

pub struct Command(process::Command);

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Command {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        Self(process::Command::new(program))
    }

    pub fn args<'a, I>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = &'a OsStr>,
    {
        self.0.args(args);
        self
    }

    pub fn try_status(mut self) -> Result<ExitStatus, Error> {
        if log::log_enabled!(log::Level::Debug) {
            debug!("running `{command:?}`...", command = &self.0);
        }

        match self.0.status() {
            Ok(status) => Ok(ExitStatus {
                command: self,
                status,
            }),
            Err(error) => Err(Error {
                command: self,
                kind: error.into(),
            }),
        }
    }

    pub fn status(self) -> Result<(), Error> {
        self.try_status().and_then(ExitStatus::require_success)
    }

    pub fn try_output(mut self) -> Result<Output, Error> {
        if log::log_enabled!(log::Level::Debug) {
            debug!("capturing `{command:?}`...", command = &self.0);
        }

        match self.0.output() {
            Ok(output) => Ok(Output {
                command: self,
                output,
            }),
            Err(error) => Err(Error {
                command: self,
                kind: error.into(),
            }),
        }
    }

    pub fn output(self) -> Result<Output, Error> {
        self.try_output().and_then(Output::require_success)
    }

    pub fn output_with_input(mut self, input: Vec<u8>) -> Result<Output, Error> {
        if log::log_enabled!(log::Level::Debug) {
            debug!("capturing `{command:?}`...", command = &self.0);
        }

        let mut child = match self
            .0
            .stdin(process::Stdio::piped())
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                return Err(Error {
                    command: self,
                    kind: error.into(),
                })
            }
        };

        let stdin_thread = std::thread::spawn({
            let mut stdin = child.stdin.take().expect("Failed to open stdin");
            move || {
                use std::io::Write;
                stdin.write_all(&input).expect("Failed to write to stdin");
            }
        });

        let output = child.wait_with_output().expect("Failed to read stdout");
        stdin_thread
            .join()
            .expect("Thread writing to stdin panicked");

        Ok(Output {
            command: self,
            output,
        })
    }
}

#[derive(Debug)]
pub struct ExitStatus {
    command: Command,
    status: process::ExitStatus,
}

impl ExitStatus {
    pub fn require_success(self) -> Result<(), Error> {
        let ExitStatus { command, status } = self;
        if status.success() {
            Ok(())
        } else {
            Err(Error {
                command,
                kind: ErrorKind::NonZeroExitStatus(status.code().and_then(NonZeroI32::new)),
            })
        }
    }
}

#[derive(Debug)]
pub struct Output {
    pub command: Command,
    pub output: process::Output,
}

impl Output {
    pub fn require_success(self) -> Result<Output, Error> {
        let Output { command, output } = self;
        if output.status.success() {
            Ok(Output { command, output })
        } else {
            Err(Error {
                command,
                kind: ErrorKind::NonZeroExitStatus(output.status.code().and_then(NonZeroI32::new)),
            })
        }
    }
}

impl std::ops::Deref for Output {
    type Target = process::Output;

    fn deref(&self) -> &Self::Target {
        &self.output
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    NotFound,
    PermissionDenied,
    NonZeroExitStatus(Option<NonZeroI32>),
}

impl From<io::Error> for ErrorKind {
    fn from(value: io::Error) -> Self {
        match value.kind() {
            io::ErrorKind::NotFound => ErrorKind::NotFound,
            io::ErrorKind::PermissionDenied => ErrorKind::PermissionDenied,
            _ => panic!(
                "can not convert `std::io::Error` of kind `{kind:?}` to `ErrorKind`",
                kind = value.kind()
            ),
        }
    }
}

#[derive(Debug)]
pub struct Error {
    pub command: Command,
    pub kind: ErrorKind,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to run `{command:?}`: ",
            command = &self.command.0
        )?;
        match self.kind {
            ErrorKind::NotFound => {
                let program = self.command.0.get_program().to_string_lossy();
                write!(f, "the `{program}` command is required but not available on your system, please install it")
            }
            ErrorKind::PermissionDenied => {
                let program = self.command.0.get_program().to_string_lossy();
                write!(f, "the `{program}` command is available but does not have the right permissions, please make sure the binary is executable")
            }
            ErrorKind::NonZeroExitStatus(code) => {
                if let Some(code) = code {
                    write!(f, "exited with non-zero exit code `{code}`")
                } else {
                    write!(f, "did not run succesfully")
                }
            }
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Creates a new [`Command`] and supplies the provided arguments, if any, while calling
/// [`std::convert::AsRef::as_ref`] on each.
macro_rules! command {
    ($program:expr, $($arg:expr),* $(,)?) => {
        $crate::process::args!($crate::process::Command::new($program), $($arg,)*)
    };
}

/// Calls [`Command::args`] on the provided [`Command`] while calling [`std::convert::AsRef::as_ref`]
/// on each argument.
macro_rules! args {
    ($program:expr, $($arg:expr),+ $(,)?) => {
        $program.args([
            $(($arg).as_ref(),)*
        ])
    }
}

pub(crate) use args;
pub(crate) use command;
