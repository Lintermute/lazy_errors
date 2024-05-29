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
//! please refer to the [CLI documentation](CiCommand).
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

use clap::ArgAction;
use lazy_errors::{prelude::*, Result};

#[derive(clap::Parser, Copy, Clone, PartialEq, Hash, Eq)]
#[clap()]
enum Cli
{
    /// Runs the CI quality gate or parts thereof
    /// in the workspace on your local machine.
    #[command(subcommand)]
    Ci(CiCommand),
}

#[derive(clap::Subcommand, Copy, Clone, PartialEq, Hash, Eq)]
enum CiCommand
{
    /// Runs the entire CI quality gate in the workspace on your local machine.
    ///
    /// Lints, builds, and tests code, documentation, and dependencies.
    /// Aborts on the first step that fails.
    ///
    /// Steps, in order (some arguments omitted here for brevity):
    /// - `cargo fmt --check`
    /// - `cargo check/clippy` (*)
    /// - `cargo test` (*)
    /// - `cargo doc` (*)
    /// - `cargo build` (*)
    /// - `cargo tarpaulin`
    /// - `cargo miri test`
    /// - `cargo upgrades`
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
    /// Finally, when all other steps have succeeded, `cargo upgrades`,
    /// cargo update --locked`, and `cargo audit` will be run.
    /// Since checking dependencies requires accessing remote servers,
    /// we run them last to keep the load on these servers low.
    #[clap(verbatim_doc_comment)]
    All(AllArgs),

    /// Runs a small but powerful subset of the CI quality gate.
    ///
    /// This command is very useful during your day-to-day development.
    /// While it's not as thorough as the complete CI quality gate,
    /// it's a good indicator for whether your changes will pass
    /// the other checks as well.
    ///
    /// Tip: Use in combination with `cargo watch`.
    ///
    /// This command has the same options like `all` but different defaults.
    Quick(QuickArgs),

    /// Runs the `cargo fmt` step of the CI quality gate.
    Rustfmt,

    /// Runs the `cargo clippy` step of the CI quality gate.
    Clippy(CheckArgs),

    /// Runs the `cargo test` step of the CI quality gate.
    Test(TestArgs),

    /// Runs the `cargo doc` step of the CI quality gate.
    Docs(DocsArgs),

    /// Runs the `cargo build` step of the CI quality gate.
    Build(BuildArgs),

    /// Runs the `cargo tarpaulin` step of the CI quality gate.
    Tarpaulin(CoverageArgs),

    /// Runs the `cargo miri test` step of the CI quality gate.
    ///
    /// The `cargo miri test` command is preceded and followed by
    /// `cargo clean` to ensure MIRI is using MIRI compilation output
    /// (and to ensure that you won't accidentally use them later).
    Miri(MiriArgs),

    /// Runs the dependency checks of the CI quality gate.
    ///
    /// This command will run `cargo upgrades`, `cargo update --locked`,
    /// and `cargo audit --deny warnings`.
    Deps,
}

#[derive(clap::Args, Copy, Clone, PartialEq, Hash, Eq)]
struct AllArgs
{
    /// Whether to pass `--release` to cargo or run in `dev` profile.
    ///
    /// If missing, run all steps affected by this flag in `dev` profile first.
    /// After all of those steps have succeeded, all steps in that list
    /// are run a second time, this time in `release` mode.
    #[clap(long)]
    profile: Option<Profile>,

    /// Run ignored tests as well during `cargo test` or `cargo miri test`.
    #[clap(long)]
    include_ignored_tests: bool,

    /// Run ignored tests as well during `cargo tarpaulin`.
    #[clap(long)]
    include_ignored_tests_in_coverage: bool,

    /// Skip running the rustfmt file formatting check.
    #[clap(long)]
    skip_rustfmt: bool,

    /// Skip the `cargo build` step. `check` or `clippy` will still be run.
    #[clap(long)]
    skip_build: bool,

    /// Skip the `cargo tarpaulin` step.
    #[clap(long)]
    skip_tarpaulin: bool,

    /// Skip the `cargo miri test` step.
    #[clap(long)]
    skip_miri: bool,

    /// Skip `cargo upgrades`, `cargo update --locked`, and `cargo audit`.
    #[clap(long)]
    skip_dependency_checks: bool,

    /// Skip checks that rely on some external input or on tools that may
    /// change. These checks may fail even if the codebase has not changed.
    ///
    /// Use this flag to validate old commits, e.g. with `git bisect`.
    /// When this flag is present, the process will run `cargo check` instead
    /// of `cargo clippy` and will skip the `cargo fmt`, `cargo tarpaulin`,
    /// as well as the dependency checks.
    #[clap(long)]
    skip_moving_targets: bool,
}

#[derive(clap::Args, Copy, Clone, PartialEq, Hash, Eq)]
struct QuickArgs
{
    /// Whether to pass `--release` to cargo or run in `dev` profile.
    #[clap(long, default_value_t = Profile::Dev)]
    profile: Profile,

    /// Run ignored tests as well during `cargo test` or `cargo miri test`.
    #[clap(long)]
    include_ignored_tests: bool,

    /// Run ignored tests as well during `cargo tarpaulin`.
    #[clap(long)]
    include_ignored_tests_in_coverage: bool,

    /// Skip running the rustfmt file formatting check.
    #[clap(long)]
    skip_rustfmt: bool,

    /// Skip the `cargo build` step. `check` or `clippy` will still be run.
    ///
    /// If `cargo check` passed, `cargo build` should usually pass as well.
    #[clap(
        long,
        value_name = "BOOL",
        default_missing_value("true"),
        default_value("true"),
        num_args(0..=1),
        require_equals(true),
        action = ArgAction::Set,
    )]
    skip_build: bool,

    /// Skip the `cargo tarpaulin` step.
    #[clap(
        long,
        value_name = "BOOL",
        default_missing_value("true"),
        default_value("true"),
        num_args(0..=1),
        require_equals(true),
        action = ArgAction::Set,
    )]
    skip_tarpaulin: bool,

    /// Skip the `cargo miri test` step.
    ///
    /// You may want to use that flag locally because running the MIRI step
    /// triggers a `cargo clean`.
    #[clap(
        long,
        value_name = "BOOL",
        default_missing_value("true"),
        default_value("true"),
        num_args(0..=1),
        require_equals(true),
        action = ArgAction::Set,
    )]
    skip_miri: bool,

    /// Skip `cargo upgrades`, `cargo update --locked`, and `cargo audit`.
    ///
    /// This is nice to use on your local machine to keep server load low.
    #[clap(
        long,
        value_name = "BOOL",
        default_missing_value("true"),
        default_value("true"),
        num_args(0..=1),
        require_equals(true),
        action = ArgAction::Set,
    )]
    skip_dependency_checks: bool,

    /// Skip checks that rely on some external input or on tools that may
    /// change. These checks may fail even if the codebase has not changed.
    ///
    /// Use this flag to validate old commits, e.g. with `git bisect`.
    /// When this flag is present, the process will run `cargo check` instead
    /// of `cargo clippy` and will skip the `cargo fmt`, `cargo tarpaulin`,
    /// as well as the dependency checks.
    #[clap(long)]
    skip_moving_targets: bool,
}

#[derive(clap::Args, Copy, Clone, PartialEq, Hash, Eq)]
struct CheckArgs
{
    /// Whether to pass `--release` to cargo or run in `dev` profile.
    #[clap(long)]
    profile: Profile,
}

#[derive(clap::Args, Copy, Clone, PartialEq, Hash, Eq)]
struct TestArgs
{
    /// Whether to pass `--release` to cargo or run in `dev` profile.
    #[clap(long)]
    profile: Profile,

    /// Run ignored tests as well.
    #[clap(long)]
    include_ignored_tests: bool,
}

#[derive(clap::Args, Copy, Clone, PartialEq, Hash, Eq)]
struct DocsArgs
{
    /// Whether to pass `--release` to cargo or run in `dev` profile.
    #[clap(long)]
    profile: Profile,
}

#[derive(clap::Args, Copy, Clone, PartialEq, Hash, Eq)]
struct BuildArgs
{
    #[clap(long)]
    /// Whether to pass `--release` to cargo or run in `dev` profile.
    profile: Profile,
}

#[derive(clap::Args, Copy, Clone, PartialEq, Hash, Eq)]
struct CoverageArgs
{
    /// Whether to pass `--release` to cargo or run in `dev` profile.
    #[clap(long)]
    profile: Profile,

    /// Run ignored tests as well.
    #[clap(long)]
    include_ignored_tests: bool,
}

#[derive(clap::Args, Copy, Clone, PartialEq, Hash, Eq)]
struct MiriArgs
{
    /// Run ignored tests as well.
    #[clap(long)]
    include_ignored_tests: bool,
}

#[derive(clap::ValueEnum, Copy, Clone, PartialEq, Hash, Eq)]
enum Profile
{
    Dev,
    Release,
}

type CommandLine = Vec<&'static str>;

impl CheckArgs
{
    fn new(profile: Profile) -> Self
    {
        Self { profile }
    }
}

impl TestArgs
{
    fn new(args: &AllArgs, profile: Profile) -> Self
    {
        Self {
            profile,
            include_ignored_tests: args.include_ignored_tests,
        }
    }
}

impl DocsArgs
{
    fn new(profile: Profile) -> Self
    {
        Self { profile }
    }
}

impl BuildArgs
{
    fn new(profile: Profile) -> Self
    {
        Self { profile }
    }
}

impl CoverageArgs
{
    fn new(args: &AllArgs, profile: Profile) -> Self
    {
        Self {
            profile,
            include_ignored_tests: args.include_ignored_tests_in_coverage,
        }
    }
}

impl MiriArgs
{
    fn new(args: &AllArgs) -> Self
    {
        Self {
            include_ignored_tests: args.include_ignored_tests,
        }
    }
}

impl From<&QuickArgs> for AllArgs
{
    fn from(value: &QuickArgs) -> Self
    {
        Self {
            profile: Some(value.profile),
            include_ignored_tests: value.include_ignored_tests,
            include_ignored_tests_in_coverage: value
                .include_ignored_tests_in_coverage,
            skip_rustfmt: value.skip_rustfmt,
            skip_build: value.skip_build,
            skip_tarpaulin: value.skip_tarpaulin,
            skip_miri: value.skip_miri,
            skip_dependency_checks: value.skip_dependency_checks,
            skip_moving_targets: value.skip_moving_targets,
        }
    }
}

impl std::fmt::Display for Profile
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        match self {
            Profile::Dev => write!(f, "dev"),
            Profile::Release => write!(f, "release"),
        }
    }
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
    let args = parse_args_from_env()?;
    let tasks = tasklist_from(&args);

    exec_all(&tasks)
}

#[cfg(not(tarpaulin_include))]
fn parse_args_from_env() -> Result<CiCommand>
{
    parse_args(std::env::args_os())
}

fn parse_args<IntoIter, T>(args: IntoIter) -> Result<CiCommand>
where
    IntoIter: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    use clap::Parser;

    let Cli::Ci(args) = Cli::try_parse_from(args).or_wrap()?;

    Ok(args)
}

fn tasklist_from(args: &CiCommand) -> Vec<CommandLine>
{
    match args {
        CiCommand::All(args) => all(args),
        CiCommand::Quick(args) => quick(args),
        CiCommand::Rustfmt => vec![rustfmt()],
        CiCommand::Clippy(args) => vec![clippy(args)],
        CiCommand::Test(args) => vec![test(args)],
        CiCommand::Build(args) => vec![build(args)],
        CiCommand::Tarpaulin(args) => vec![tarpaulin(args)],
        CiCommand::Miri(args) => miri(args).into(),
        CiCommand::Docs(args) => vec![docs(args)],
        CiCommand::Deps => deps().into(),
    }
}

fn all(args: &AllArgs) -> Vec<CommandLine>
{
    let mut tasklist = Vec::new();

    if !args.skip_moving_targets && !args.skip_rustfmt {
        tasklist.push(rustfmt());
    }

    match args.profile {
        Some(profile) => tasklist.extend(compile_and_test(args, profile)),
        None => {
            tasklist.extend(compile_and_test(args, Profile::Dev));
            tasklist.extend(compile_and_test(args, Profile::Release));
        },
    }

    if !args.skip_moving_targets {
        if !args.skip_miri {
            tasklist.extend(miri(&MiriArgs::new(args)));
        }

        if !args.skip_dependency_checks {
            // Checks if our dependencies are up-to-date and secure.
            // These functions will access the network.
            // These function may produce different results when run again,
            // dependant on upstream changes.
            tasklist.extend(deps());
        }
    }

    tasklist
}

fn quick(args: &QuickArgs) -> Vec<CommandLine>
{
    all(&AllArgs::from(args))
}

fn compile_and_test(args: &AllArgs, profile: Profile) -> Vec<CommandLine>
{
    let mut tasklist = Vec::new();

    if !args.skip_moving_targets {
        tasklist.push(clippy(&CheckArgs::new(profile)));
    } else {
        tasklist.push(check(&CheckArgs::new(profile)));
    }

    tasklist.push(test(&TestArgs::new(args, profile)));

    tasklist.push(docs(&DocsArgs::new(profile)));

    if !args.skip_build {
        tasklist.push(build(&BuildArgs::new(profile)));
    }

    if !args.skip_moving_targets && !args.skip_tarpaulin {
        tasklist.push(tarpaulin(&CoverageArgs::new(args, profile)));
    }

    tasklist
}

fn rustfmt() -> CommandLine
{
    vec!["cargo", "+nightly", "--locked", "fmt", "--check", "--all"]
}

fn check(args: &CheckArgs) -> CommandLine
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

    match args.profile {
        Profile::Release => task.push("--release"),
        Profile::Dev => (),
    }

    task
}

fn clippy(args: &CheckArgs) -> CommandLine
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

    match args.profile {
        Profile::Release => task.push("--release"),
        Profile::Dev => (),
    }

    task.extend(&["--", "-Dwarnings"]);

    task
}

fn test(args: &TestArgs) -> CommandLine
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

    match args.profile {
        Profile::Release => task.push("--release"),
        Profile::Dev => (),
    }

    if args.include_ignored_tests {
        task.extend(&["--", "--include-ignored"]);
    }

    task
}

fn docs(args: &DocsArgs) -> CommandLine
{
    // Make `cargo doc` raise an error if there are any warnings.
    env::set_var("RUSTDOCFLAGS", "-Dwarnings");

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

    match args.profile {
        Profile::Release => task.push("--release"),
        Profile::Dev => (),
    }

    task
}

fn build(args: &BuildArgs) -> CommandLine
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

    match args.profile {
        Profile::Release => task.push("--release"),
        Profile::Dev => (),
    }

    task
}

fn tarpaulin(args: &CoverageArgs) -> CommandLine
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

    match args.profile {
        Profile::Release => {
            task.extend(&["--output-dir", "tarpaulin-report-release"]);
            task.push("--release");
        },
        Profile::Dev => {
            task.extend(&["--output-dir", "tarpaulin-report-dev"]);
        },
    }

    if args.include_ignored_tests {
        task.extend(&["--", "--include-ignored"]);
    }

    task
}

fn miri(args: &MiriArgs) -> [CommandLine; 3]
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

    if args.include_ignored_tests {
        test.extend(&["--", "--include-ignored"]);
    }

    [clean.clone(), test, clean]
}

fn deps() -> [CommandLine; 3]
{
    let upgrades = vec!["cargo", "upgrades"];
    let update = vec!["cargo", "--locked", "update", "--workspace"];
    let audit = vec!["cargo", "--locked", "audit", "--deny", "warnings"];

    [upgrades, update, audit]
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
        &["xtask", "ci", "all",
            "--profile=dev",
            "--skip-rustfmt",
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
        &["xtask", "ci", "all"],
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
            &["cargo", "upgrades"],
            &["cargo", "--locked", "update", "--workspace"],
            &["cargo", "--locked", "audit", "--deny", "warnings"],
        ]; "default tasklist")]
    #[test_case(
        &["xtask", "ci", "all", "--skip-moving-targets"],
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
        &["xtask", "ci", "all", "--include-ignored-tests"],
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
        &["xtask", "ci", "all", "--include-ignored-tests-in-coverage"],
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
