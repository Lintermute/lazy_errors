#![forbid(unsafe_code)]

//! Helper tool to run the CI pipeline locally (`cargo xtask ci`) or
//! set the version based on `git describe` (`cargo xtask version`).
//!
//! The implementation of the `xtask` workspace and `cargo xtask`
//! is based on [the blog post “Make Your Own Make” by matklad][MYOM]
//! and the [`xtask` GitHub repo][xtask] by the same author.
//! Additional ideas have been stolen from [Robbepop].
//!
//! [MYOM]: https://matklad.github.io/2018/01/03/make-your-own-make.html
//! [xtask]: https://github.com/matklad/cargo-xtask
//! [Robbepop]: https://github.com/Robbepop

mod ci;
mod version;

use core::str;

use std::process::{self, ExitCode, Stdio};

use lazy_errors::{prelude::*, Result};

use ci::Ci;
use version::Version;

type CommandLine = Vec<&'static str>;

#[derive(clap::Parser, Debug, Clone, PartialEq, Hash, Eq)]
enum Xtask {
    /// Runs the CI quality gate or parts thereof
    /// in the workspace on your local machine.
    #[command(subcommand)]
    Ci(Ci),

    /// Manipulates the `version` attribute in `Cargo.toml` and `Cargo.lock`.
    #[command(subcommand)]
    Version(Version),
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let command = parse_args_from_env()?;

    match command {
        Xtask::Ci(command) => ci::run(&command),
        Xtask::Version(command) => version::run(&command),
    }
}

fn parse_args_from_env() -> Result<Xtask> {
    parse_args(std::env::args_os())
}

fn parse_args<IntoIter, T>(args: IntoIter) -> Result<Xtask>
where
    IntoIter: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    use clap::Parser;

    let command = Xtask::try_parse_from(args).or_wrap()?;

    Ok(command)
}

fn exec_all<L>(tasklist: &[L]) -> Result<()>
where
    L: AsRef<[&'static str]>,
{
    for task in tasklist {
        exec(task.as_ref())?;
    }

    Ok(())
}

fn exec(command_with_args: &[&str]) -> Result<()> {
    exec_impl(command_with_args, false)?;
    Ok(())
}

fn exec_and_capture(command_with_args: &[&str]) -> Result<String> {
    exec_impl(command_with_args, true)
}

fn exec_impl(command_with_args: &[&str], capture: bool) -> Result<String> {
    let (command, args) = match command_with_args {
        [head, tail @ ..] => (head, tail),
        _ => return Err(err!("No command passed.")),
    };

    eprintln!("Starting '{}'...", command_with_args.join(" "));

    let mut handle = process::Command::new(command);

    if capture {
        handle
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    }

    let mut errs =
        ErrorStash::new(|| format!("Failed to run {command_with_args:?}"));

    let process = try2!(handle
        .args(args)
        .spawn()
        .or_wrap_with::<Stashable>(|| "Failed to start process")
        .and_then(|process| process.wait_with_output().or_wrap())
        .or_stash(&mut errs));

    match process.status.code() {
        Some(0) => (),
        Some(c) => {
            errs.push(format!("Status code was {c}"));
        }
        None => {
            errs.push("No status code (terminated by signal?)");
        }
    };

    let stdout = str_or_stash(&process.stdout, &mut errs);
    let stderr = str_or_stash(&process.stderr, &mut errs);

    if !errs.is_empty() && capture {
        if !stdout.is_empty() {
            errs.push(format!("STDOUT:\n{stdout}"));
        }

        if !stderr.is_empty() {
            errs.push(format!("STDERR:\n{stderr}"));
        }
    }

    errs.into_result()?;

    Ok(stdout.to_owned())
}

fn str_or_stash<'a, F, M>(
    bytes: &'a [u8],
    errs: &mut ErrorStash<F, M>,
) -> &'a str
where
    F: FnOnce() -> M,
    M: core::fmt::Display,
{
    str::from_utf8(bytes)
        .map(str::trim)
        .or_wrap_with::<Stashable>(|| "Cannot create string: Invalid byte(s)")
        .or_stash(errs)
        .ok()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;

    #[test]
    fn exec_is_no_op_if_list_is_empty() -> Result<()> {
        let empty: &[&[&str]] = &[];
        exec_all(empty) // no-op
    }

    #[test]
    fn exec_returns_error_if_command_is_empty() -> Result<()> {
        let err = exec_all(&[&[]]).unwrap_err();
        assert_eq!(err.to_string(), "No command passed.");
        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn exec_can_invoke_cargo() -> Result<()> {
        exec_all(&[&["cargo", "version"]])
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn exec_returns_cargo_version() -> Result<()> {
        let version = exec_and_capture(&["cargo", "version"])?;
        dbg!(&version);

        // Loosely assert that we got some output from the process.
        assert!(version.starts_with("cargo"));
        assert!(version.contains('.'));

        Ok(())
    }

    #[test_case(
        &[&["unexisting-program"]],
         r#"Failed to run ["unexisting-program"]: Failed to start process: "#)]
    #[test_case(
        &[&["cargo", "unexisting-subcommand"]],
         "Failed to run [\"cargo\", \"unexisting-subcommand\"]: \
             Status code was 101")]
    #[cfg_attr(miri, ignore)]
    fn exec_propagates_process_failure(
        commands: &[&[&'static str]],
        expected_error: &str,
    ) {
        let err = exec_all(commands).unwrap_err();
        let msg = &format!("{err}");

        dbg!(msg, expected_error);
        assert!(msg.starts_with(expected_error));
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn exec_propagates_stderr() {
        let err =
            exec_and_capture(&["cargo", "unexisting-subcommand"]).unwrap_err();

        let msg = &format!("{err:#}");
        dbg!(msg);
        assert!(msg.contains("- STDERR:\n"));
    }
}
