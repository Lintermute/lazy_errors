// Copyright (c) 2024 Andreas Waidler
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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

use ci::Ci;
use lazy_errors::{prelude::*, try2, Result};
use version::Version;

type CommandLine = Vec<&'static str>;

#[derive(clap::Parser, Debug, Clone, PartialEq, Hash, Eq)]
enum Xtask
{
    /// Runs the CI quality gate or parts thereof
    /// in the workspace on your local machine.
    #[command(subcommand)]
    Ci(Ci),

    /// Manipulates the `version` attribute in `Cargo.toml` and `Cargo.lock`.
    #[command(subcommand)]
    Version(Version),
}

#[cfg(not(tarpaulin_include))]
fn main() -> ExitCode
{
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err:#}");
            ExitCode::FAILURE
        },
    }
}

#[cfg(not(tarpaulin_include))]
fn run() -> Result<()>
{
    let command = parse_args_from_env()?;

    match command {
        Xtask::Ci(command) => ci::run(&command),
        Xtask::Version(command) => version::run(&command),
    }
}

#[cfg(not(tarpaulin_include))]
fn parse_args_from_env() -> Result<Xtask>
{
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
where L: AsRef<[&'static str]>
{
    for task in tasklist {
        exec(task.as_ref())?;
    }

    Ok(())
}

fn exec(command_with_args: &[&str]) -> Result<()>
{
    exec_impl(command_with_args, false)?;
    Ok(())
}

fn exec_and_capture(command_with_args: &[&str]) -> Result<String>
{
    exec_impl(command_with_args, true)
}

fn exec_impl(command_with_args: &[&str], capture: bool) -> Result<String>
{
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

    let stdout = str_or_stash(&process.stdout, &mut errs);
    let stderr = str_or_stash(&process.stderr, &mut errs);

    let err = |msg: &str| {
        if !capture {
            err!("{msg}")
        } else {
            err!("{msg}\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}")
        }
    };

    let status: Result<()> = match process.status.code() {
        Some(0) => Ok(()),
        Some(c) => Err(err(&format!("Status code was {c}"))),
        None => Err(err("No status code (terminated by signal?)")),
    };

    match status.or_stash(&mut errs) {
        StashedResult::Ok(()) => {
            errs.into_result()?;
            Ok(stdout.to_owned())
        },
        StashedResult::Err(errs) => {
            // TODO: The `Try` trait on `StashedResult` would simplify this.
            // Keep this here as an example how that trait could work.
            let mut swap = StashWithErrors::from("DUMMY", "DUMMY");
            std::mem::swap(&mut swap, errs);
            Err(swap.into())
        },
    }
}

fn str_or_stash<'a, F, M>(
    bytes: &'a [u8],
    errs: &mut ErrorStash<F, M>,
) -> &'a str
where
    F: FnOnce() -> M,
    M: std::fmt::Display,
{
    match str::from_utf8(bytes)
        .map(str::trim)
        .or_wrap_with::<Stashable>(|| {
            "Cannot create string: Invalid byte(s): {bytes}"
        })
        .or_stash(errs)
    {
        StashedResult::Ok(output) => output,
        StashedResult::Err(_) => "",
    }
}

#[cfg(test)]
mod tests
{
    use test_case::test_case;

    use super::*;

    #[test]
    fn exec_is_no_op_if_list_is_empty() -> Result<()>
    {
        let empty: &[&[&str]] = &[];
        exec_all(empty) // no-op
    }

    #[test]
    fn exec_returns_error_if_command_is_empty() -> Result<()>
    {
        let err = exec_all(&[&[]]).unwrap_err();
        assert_eq!(err.to_string(), "No command passed.");
        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn exec_can_invoke_cargo() -> Result<()>
    {
        exec_all(&[&["cargo", "version"]])
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn exec_returns_cargo_version() -> Result<()>
    {
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
    )
    {
        let err = exec_all(commands).unwrap_err();
        let msg = &format!("{err}");

        dbg!(msg, expected_error);
        assert!(msg.starts_with(expected_error));
    }
}
