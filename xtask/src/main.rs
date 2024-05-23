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

//! Tests, builds, runs all lints, and validates dependencies.
//! Runs MIRI tests as well.
//!
//! Several tasks can be skipped,
//! please refer to the [CLI documentation](CiArgs).
//!
//! The implementation of the `xtask` workspace and `cargo xtask`
//! is based on [the blog post “Make Your Own Make” by matklad][MYOM]
//! and the [`xtask` GitHub repo][xtask] by the same author.
//! Additional ideas have been stolen from [Robbepop].
//!
//! [MYOM]: https://matklad.github.io/2018/01/03/make-your-own-make.html
//! [xtask]: https://github.com/matklad/cargo-xtask
//! [Robbepop]: https://github.com/Robbepop

use std::{env, process, process::ExitCode};

use lazy_errors::{prelude::*, Result};

#[derive(clap::Parser)]
#[clap()]
struct Cli
{
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(clap::Subcommand)]
enum CliCommand
{
    /// Runs the CI quality gate on the workspace on your local machine:
    /// compilation, linting, testing, dependency checking, and so on.
    /// Aborts on the first step that fails.
    ///
    /// Steps, in order (some arguments omitted here for brevity):
    /// - `cargo fmt --check`
    /// - `cargo clippy/check` (*)
    /// - `cargo test` (*)
    /// - `cargo doc` (*)
    /// - `cargo build` (*)
    /// - `cargo tarpaulin`
    /// - `cargo miri test`
    /// - `cargo update --locked`
    /// - `cargo audit --deny warnings`
    ///
    /// The steps are run in that order to make steps that are most likely
    /// to fail (or quickest to fix) fail as early as possible.
    /// All steps marked with `(*)` will be run in development mode first.
    /// Then, that sequence will be repeated with the `--release` flag
    /// added to each individual command marked with `(*)` in the list.
    ///
    /// Note that `cargo build` is executed late because we already ran
    /// `cargo check`, which means that `cargo build` should not fail usually.
    ///
    /// Also note that `cargo test` has a nicer CLI than `cargo tarpaulin`.
    /// Furthermore, `cargo tarpaulin` will compile the project a second time,
    /// separately from `cargo test`. The actual execution times of the tests
    /// will be larger as well.
    ///
    /// MIRI tests take a lot of time and will run `cargo clean` before and
    /// after compiling/running the tests. They will be run after the steps
    /// marked with `(*)` to defer the `cargo clean` for as long as possible.
    ///
    /// Finally, when all other steps succeeded, `cargo update --locked`
    /// and `cargo audit` will be run. Since checking dependencies requires
    /// accessing remote servers, we run them last to keep the load on the
    /// Cargo/Rust servers low.
    #[clap(verbatim_doc_comment)]
    Ci(CiArgs),
}

#[derive(clap::Args)]
struct CiArgs
{
    /// Run ignored tests as well when running `cargo test` (and MIRI).
    #[clap(long)]
    include_ignored_tests: bool,

    /// Run ignored tests as well when running `cargo tarpaulin`.
    #[clap(long)]
    include_ignored_tests_in_coverage: bool,

    /// Skip running the rustfmt file formatting check.
    #[clap(long)]
    skip_rustfmt: bool,

    /// Skip running the steps marked with `(*)` in the list above
    /// a second time with the `--release` flag added.
    /// Steps will only be run once (using the dev profile).
    #[clap(long)]
    skip_release_target: bool,

    /// Skip the `cargo build` step. Note: `cargo check` will still be run.
    /// If `cargo check` passed, `cargo build` should usually pass as well.
    #[clap(long)]
    skip_build: bool,

    /// Skip the `cargo tarpaulin` step.
    #[clap(long)]
    skip_tarpaulin: bool,

    /// Skip the `cargo miri test` steps (both dev and release profiles).
    /// You may want to use that flag locally because running the MIRI step
    /// triggers a `cargo clean`.
    #[clap(long)]
    skip_miri: bool,

    /// Skip `cargo update --locked` and `cargo audit`.
    /// This is nice to use on your local machine to keep server load low.
    #[clap(long)]
    skip_dependency_checks: bool,

    /// Skip checks that rely on some external input or on tools that may
    /// change. These checks may fail even if the codebase has not changed.
    /// Use this flag to validate old commits, e.g. with `git bisect`.
    /// When this flag is present, the process will run `cargo check` instead
    /// of `cargo clippy` and will skip the `cargo fmt`, `cargo tarpaulin`,
    /// `cargo update --locked`, and `cargo audit` steps.
    #[clap(long)]
    skip_moving_targets: bool,
}

type CommandLine = Vec<&'static str>;

#[cfg(not(tarpaulin_include))]
fn main() -> ExitCode
{
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err:#}");
            ExitCode::FAILURE
        },
    }
}

#[cfg(not(tarpaulin_include))]
fn run() -> Result<()>
{
    let args = parse_args_from_env()?;
    let tasks = tasklist_from(&args);

    // Make `cargo doc` raise an error if there are any warnings.
    env::set_var("RUSTDOCFLAGS", "-Dwarnings");
    exec_all(&tasks)
}

#[cfg(not(tarpaulin_include))]
fn parse_args_from_env() -> Result<CiArgs>
{
    parse_args(std::env::args_os())
}

fn parse_args<IntoIter, T>(args: IntoIter) -> Result<CiArgs>
where
    IntoIter: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    use clap::Parser;

    let Cli { command } = Cli::try_parse_from(args).or_wrap()?;
    let CliCommand::Ci(args) = command;

    Ok(args)
}

fn tasklist_from(args: &CiArgs) -> Vec<CommandLine>
{
    let mut tasklist = Vec::new();

    if !args.skip_moving_targets && !args.skip_rustfmt {
        tasklist.push(fmt());
    }

    tasklist.extend(compile_and_test(args, false));
    if !args.skip_release_target {
        tasklist.extend(compile_and_test(args, true));
    }

    if !args.skip_moving_targets && !args.skip_miri {
        tasklist.extend(test_miri(args));
    }

    if !args.skip_moving_targets && !args.skip_dependency_checks {
        // Checks if our dependencies are up-to-date and secure.
        // These functions will access the network.
        // These function may produce different results when run again,
        // dependant on upstream changes.
        tasklist.push(update());
        tasklist.push(audit());
    }

    tasklist
}

fn compile_and_test(args: &CiArgs, is_release: bool) -> Vec<CommandLine>
{
    let mut tasklist = Vec::new();

    if !args.skip_moving_targets {
        tasklist.push(clippy(is_release));
    } else {
        tasklist.push(check(is_release));
    }

    tasklist.push(test(args, is_release));

    tasklist.push(doc(is_release));

    if !args.skip_build {
        tasklist.push(build(is_release));
    }

    if !args.skip_moving_targets && !args.skip_tarpaulin {
        tasklist.push(tarpaulin(args, is_release));
    }

    tasklist
}

fn fmt() -> CommandLine
{
    vec!["cargo", "+nightly", "--locked", "fmt", "--check", "--all"]
}

fn clippy(is_release: bool) -> CommandLine
{
    // Clippy seems to use the same arguments as `cargo check`.
    // It looks like there is no way to specify doctests here.

    let mut task = vec![
        "cargo",
        "hack",
        "clippy",
        "--locked",
        "--workspace",
        "--feature-powerset",
        "--optional-deps",
        "--all-targets",
    ];

    if is_release {
        task.push("--release");
    }

    task.extend(&["--", "-Dwarnings"]);

    task
}

fn check(is_release: bool) -> CommandLine
{
    // It looks like there is no way to specify doctests here.

    let mut task = vec![
        "cargo",
        "hack",
        "check",
        "--locked",
        "--workspace",
        "--feature-powerset",
        "--optional-deps",
        "--all-targets",
    ];

    if is_release {
        task.push("--release");
    }

    task
}

fn test(config: &CiArgs, is_release: bool) -> CommandLine
{
    // WARNING: `--all-targets` enables benchmarks and disables doctests.
    let mut task = vec![
        "cargo",
        "hack",
        "test",
        "--locked",
        "--workspace",
        "--feature-powerset",
        "--optional-deps",
    ];

    if is_release {
        task.push("--release");
    }

    if config.include_ignored_tests {
        task.extend(&["--", "--include-ignored"]);
    }

    task
}

fn doc(is_release: bool) -> CommandLine
{
    // Test if documentation builds properly.
    // This is especially useful to detect broken intra doc links.

    let mut task = vec![
        "cargo",
        "hack",
        "doc",
        "--locked",
        "--workspace",
        "--feature-powerset",
        "--optional-deps",
        "--no-deps",
    ];

    if is_release {
        task.push("--release");
    }

    task
}

fn build(is_release: bool) -> CommandLine
{
    let mut task = vec![
        "cargo",
        "hack",
        "build",
        "--locked",
        "--workspace",
        "--feature-powerset",
        "--optional-deps",
        "--all-targets",
        "--exclude=xtask",
    ];

    if is_release {
        task.push("--release");
    }

    task
}

fn tarpaulin(config: &CiArgs, is_release: bool) -> CommandLine
{
    // WARNING: `--all-targets` enables benchmarks and disables doctests.
    let mut task = vec![
        "cargo",
        "tarpaulin",
        "--locked",
        "--workspace",
        "--all-features",
        "--all-targets",
        "--doc",
        "--no-fail-fast",
    ];

    if is_release {
        task.extend(&["--output-dir", "tarpaulin-report-release"]);
        task.push("--release");
    } else {
        task.extend(&["--output-dir", "tarpaulin-report-dev"]);
    }

    if config.include_ignored_tests_in_coverage {
        task.extend(&["--", "--include-ignored"]);
    }

    task
}

fn test_miri(config: &CiArgs) -> [CommandLine; 3]
{
    // Remove (non-)MIRI outputs
    let clean = vec!["cargo", "+nightly", "--locked", "clean"];

    // Note: MIRI args are the same as for `cargo run` and `cargo test`.
    // WARNING: `--all-targets` enables benchmarks and disables doctests.
    let mut test = vec![
        "cargo",
        "+nightly",
        "hack",
        "miri",
        "test",
        "--locked",
        "--workspace",
        "--feature-powerset",
        "--optional-deps",
    ];

    if config.include_ignored_tests {
        test.extend(&["--", "--include-ignored"]);
    }

    [clean.clone(), test, clean]
}

fn update() -> CommandLine
{
    vec!["cargo", "--locked", "update", "--workspace"]
}

fn audit() -> CommandLine
{
    vec!["cargo", "--locked", "audit", "--deny", "warnings"]
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
    let (command, args) = match command_with_args {
        [head, tail @ ..] => (head, tail),
        _ => return Err(err!("No command passed.")),
    };

    eprintln!("Starting '{}'...", command_with_args.join(" "));

    let status_code = process::Command::new(command)
        .args(args)
        .status()
        .map(|status| status.code());

    let result = match status_code {
        Ok(Some(0)) => Ok(()),
        Ok(Some(e)) => Err(err!("Status code was {e}")),
        Ok(None) => Err(err!("No status code (terminated by signal?)")),
        Err(e) => Err(Error::wrap_with(e, "Failed to start process")),
    };

    result.or_wrap_with(|| format!("Failed to run {command_with_args:?}"))
}

#[cfg(test)]
mod tests
{
    use test_case::test_case;

    use super::*;

    #[test_case(
        &["xtask", "ci",
            "--skip-rustfmt",
            "--skip-release-target",
            "--skip-build",
            "--skip-tarpaulin",
            "--skip-miri",
            "--skip-dependency-checks",
            "--skip-moving-targets"],
        &[
            &["cargo", "hack", "check",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--all-targets"],
            &["cargo", "hack", "test",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps"],
            &["cargo", "hack", "doc",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--no-deps"],
        ]; "minimal tasklist")]
    #[test_case(
        &["xtask", "ci"],
        &[
            &["cargo", "+nightly", "--locked", "fmt", "--check", "--all"],

            &["cargo", "hack", "clippy",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--all-targets", "--", "-Dwarnings"],
            &["cargo", "hack", "test",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps"],
            &["cargo", "hack", "doc",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--no-deps"],
            &["cargo", "hack", "build",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--all-targets", "--exclude=xtask"],
            &["cargo", "tarpaulin",
                "--locked", "--workspace",
                "--all-features", "--all-targets", "--doc", "--no-fail-fast",
                "--output-dir", "tarpaulin-report-dev"],

            &["cargo", "hack", "clippy",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--all-targets", "--release", "--", "-Dwarnings"],
            &["cargo", "hack", "test",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--release"],
            &["cargo", "hack", "doc",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--no-deps", "--release"],
            &["cargo", "hack", "build",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--all-targets", "--exclude=xtask", "--release"],
            &["cargo", "tarpaulin",
                "--locked", "--workspace",
                "--all-features", "--all-targets", "--doc", "--no-fail-fast",
                "--output-dir", "tarpaulin-report-release",
                "--release"],

            &["cargo", "+nightly", "--locked", "clean"],
            &["cargo", "+nightly", "hack", "miri", "test",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps"],
            &["cargo", "+nightly", "--locked", "clean"],
            &["cargo", "--locked", "update", "--workspace"],
            &["cargo", "--locked", "audit", "--deny", "warnings"],
        ]; "default tasklist")]
    #[test_case(
        &["xtask", "ci", "--skip-moving-targets"],
        &[
            &["cargo", "hack", "check",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--all-targets"],
            &["cargo", "hack", "test",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps"],
            &["cargo", "hack", "doc",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--no-deps"],
            &["cargo", "hack", "build",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--all-targets", "--exclude=xtask"],

            &["cargo", "hack", "check",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--all-targets", "--release"],
            &["cargo", "hack", "test",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--release"],
            &["cargo", "hack", "doc",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--no-deps", "--release"],
            &["cargo", "hack", "build",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--all-targets", "--exclude=xtask", "--release"],
        ]; "stable tasklist")]
    fn transform_args_to_tasks(
        args: &[&str],
        tasklist: &[&[&str]],
    ) -> Result<()>
    {
        let tasks = tasklist_from(&parse_args(args)?);
        assert_eq!(&tasks, tasklist);
        Ok(())
    }

    #[test_case(
        &["xtask", "ci", "--include-ignored-tests"],
        &[
            &["cargo", "hack", "test",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--", "--include-ignored"],
            &["cargo", "tarpaulin",
                "--locked", "--workspace",
                "--all-features", "--all-targets", "--doc", "--no-fail-fast",
                "--output-dir", "tarpaulin-report-dev"],
            &["cargo", "hack", "test",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--release", "--", "--include-ignored"],
            &["cargo", "tarpaulin",
                "--locked", "--workspace",
                "--all-features", "--all-targets", "--doc", "--no-fail-fast",
                "--output-dir", "tarpaulin-report-release",
                "--release"],
        ]; "can run ignored tests w/o coverage")]
    #[test_case(
        &["xtask", "ci", "--include-ignored-tests-in-coverage"],
        &[
            &["cargo", "hack", "test",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps"],
            &["cargo", "tarpaulin",
                "--locked", "--workspace",
                "--all-features", "--all-targets", "--doc", "--no-fail-fast",
                "--output-dir", "tarpaulin-report-dev",
                "--", "--include-ignored"],
            &["cargo", "hack", "test",
                "--locked", "--workspace",
                "--feature-powerset", "--optional-deps",
                "--release"],
            &["cargo", "tarpaulin",
                "--locked", "--workspace",
                "--all-features", "--all-targets", "--doc", "--no-fail-fast",
                "--output-dir", "tarpaulin-report-release",
                "--release", "--", "--include-ignored"],
        ]; "can run ignored tests only for coverage")]
    fn tasklist_contains(args: &[&str], task_sublist: &[&[&str]])
        -> Result<()>
    {
        let mut tasks = tasklist_from(&parse_args(args)?);
        tasks.retain(|task| task_sublist.contains(&task.as_ref()));
        assert_eq!(&tasks, task_sublist);
        Ok(())
    }

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
