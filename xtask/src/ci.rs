//! Tests, builds, runs all lints, validates dependencies, and
//! runs MIRI tests as well.
//!
//! Several tasks can be skipped or run individually.
//! Please refer to the [CLI documentation](Ci) for details.

use core::fmt::{self, Display};

use std::env;

use clap::ArgAction;
use lazy_errors::Result;

use crate::CommandLine;

type TaskList = Vec<CommandLine>;

#[derive(clap::Subcommand, Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum Ci {
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
    /// - `cargo upgrades --locked`
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
    /// Finally, when all other steps have succeeded,
    /// `cargo upgrades --locked`, cargo update --locked`, and `cargo audit`
    /// will be run. Since checking dependencies requires accessing remote
    /// servers, we run them last to keep the load on these servers low.
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
    /// This command will run `cargo upgrades --locked`,
    /// `cargo update --locked`, and
    /// `cargo audit --deny warnings`.
    Deps,
}

#[derive(clap::Args, Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub struct AllArgs {
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

    /// Skip `cargo upgrades --locked`, `cargo update --locked`, and
    /// `cargo audit`.
    #[clap(long)]
    skip_dependency_checks: bool,

    /// Skip checks that rely on some external input or on tools that may
    /// change. These checks may fail even if the codebase has not changed.
    ///
    /// Use this flag to validate old commits, e.g. with `git bisect`.
    /// When this flag is present, the process will run `cargo check` instead
    /// of `cargo clippy` and will skip `cargo fmt`, `cargo tarpaulin`,
    /// `cargo miri test`, as well as the dependency checks.
    #[clap(long)]
    skip_moving_targets: bool,
}

#[derive(clap::Args, Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub struct QuickArgs {
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

    /// Skip `cargo upgrades --locked`, `cargo update --locked`, and
    /// `cargo audit`.
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

#[derive(clap::Args, Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub struct CheckArgs {
    /// Rust toolchain version to use (leave blank to use the default
    /// toolchain).
    #[clap(long)]
    rust_version: Option<RustVersion>,

    /// Whether to exclude the `xtask` workspace package.
    #[clap(long)]
    exclude_xtask: bool,

    /// Whether to pass `--release` to cargo or run in `dev` profile.
    #[clap(long)]
    profile: Profile,
}

#[derive(clap::Args, Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub struct TestArgs {
    /// Rust toolchain version to use (leave blank to use the default
    /// toolchain).
    #[clap(long)]
    rust_version: Option<RustVersion>,

    /// Whether to exclude the `xtask` workspace package.
    #[clap(long)]
    exclude_xtask: bool,

    /// Whether to pass `--release` to cargo or run in `dev` profile.
    #[clap(long)]
    profile: Profile,

    /// Run ignored tests as well.
    #[clap(long)]
    include_ignored_tests: bool,
}

#[derive(clap::Args, Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub struct DocsArgs {
    /// Rust toolchain version to use (leave blank to use the default
    /// toolchain).
    #[clap(long)]
    rust_version: Option<RustVersion>,

    /// Whether to pass `--release` to cargo or run in `dev` profile.
    #[clap(long)]
    profile: Profile,
}

#[derive(clap::Args, Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub struct BuildArgs {
    /// Rust toolchain version to use (leave blank to use the default
    /// toolchain).
    #[clap(long)]
    rust_version: Option<RustVersion>,

    /// Whether to exclude the `xtask` workspace package.
    #[clap(long)]
    exclude_xtask: bool,

    #[clap(long)]
    /// Whether to pass `--release` to cargo or run in `dev` profile.
    profile: Profile,
}

#[derive(clap::Args, Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub struct CoverageArgs {
    /// Whether to pass `--release` to cargo or run in `dev` profile.
    #[clap(long)]
    profile: Profile,

    /// Run ignored tests as well.
    #[clap(long)]
    include_ignored_tests: bool,
}

#[derive(clap::Args, Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub struct MiriArgs {
    /// Rust toolchain version to use (leave blank to use the default
    /// toolchain).
    #[clap(long)]
    rust_version: Option<RustVersion>,

    /// Run ignored tests as well.
    #[clap(long)]
    include_ignored_tests: bool,
}

#[derive(clap::ValueEnum, Debug, Copy, Clone, PartialEq, Hash, Eq)]
enum Profile {
    Dev,
    Release,
}

#[derive(clap::ValueEnum, Debug, Copy, Clone, PartialEq, Hash, Eq)]
enum RustVersion {
    #[clap(name = "1.81")]
    V1_81,
    #[clap(name = "1.77")]
    V1_77,
    #[clap(name = "1.69")]
    V1_69,
    #[clap(name = "1.66")]
    V1_66,
    #[clap(name = "1.64")]
    V1_64,
    #[clap(name = "1.61")]
    V1_61,
}

impl CheckArgs {
    fn new(profile: Profile) -> Self {
        Self {
            rust_version: None,
            exclude_xtask: false,
            profile,
        }
    }
}

impl TestArgs {
    fn new(args: &AllArgs, profile: Profile) -> Self {
        Self {
            rust_version: None,
            exclude_xtask: false,
            profile,
            include_ignored_tests: args.include_ignored_tests,
        }
    }
}

impl DocsArgs {
    fn new(profile: Profile) -> Self {
        Self {
            rust_version: None,
            profile,
        }
    }
}

impl BuildArgs {
    fn new(profile: Profile) -> Self {
        Self {
            rust_version: None,
            exclude_xtask: false,
            profile,
        }
    }
}

impl CoverageArgs {
    fn new(args: &AllArgs, profile: Profile) -> Self {
        Self {
            profile,
            include_ignored_tests: args.include_ignored_tests_in_coverage,
        }
    }
}

impl MiriArgs {
    fn new(args: &AllArgs) -> Self {
        Self {
            rust_version: None,
            include_ignored_tests: args.include_ignored_tests,
        }
    }
}

impl From<&QuickArgs> for AllArgs {
    fn from(value: &QuickArgs) -> Self {
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

impl Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Profile::Dev => write!(f, "dev"),
            Profile::Release => write!(f, "release"),
        }
    }
}

pub fn run(command: &Ci) -> Result<()> {
    crate::exec_all(&tasklist_from(command))
}

fn tasklist_from(args: &Ci) -> TaskList {
    match args {
        Ci::All(args) => all(args),
        Ci::Quick(args) => quick(args),
        Ci::Rustfmt => vec![rustfmt()],
        Ci::Clippy(args) => vec![clippy(args)],
        Ci::Test(args) => vec![test(args)],
        Ci::Build(args) => vec![build(args)],
        Ci::Tarpaulin(args) => vec![tarpaulin(args)],
        Ci::Miri(args) => miri(args).into(),
        Ci::Docs(args) => vec![docs(args)],
        Ci::Deps => deps().into(),
    }
}

fn all(args: &AllArgs) -> TaskList {
    let mut tasklist = Vec::new();

    if !args.skip_moving_targets && !args.skip_rustfmt {
        tasklist.push(rustfmt());
    }

    match args.profile {
        Some(profile) => tasklist.extend(compile_and_test(args, profile)),
        None => {
            tasklist.extend(compile_and_test(args, Profile::Dev));
            tasklist.extend(compile_and_test(args, Profile::Release));
        }
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

fn quick(args: &QuickArgs) -> TaskList {
    all(&AllArgs::from(args))
}

fn compile_and_test(args: &AllArgs, profile: Profile) -> TaskList {
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

fn rustfmt() -> CommandLine {
    vec!["cargo", "+nightly", "--locked", "fmt", "--check", "--all"]
}

fn check(args: &CheckArgs) -> CommandLine {
    // It looks like there is no way to specify doctests here.

    let mut task = vec![
        "cargo",
        "hack",
        "check",
        "--locked",
        "--workspace",
        "--all-targets",
    ];

    add_exclude_xtask_flag_maybe(args.exclude_xtask, &mut task);
    add_feature_flags(&args.rust_version, &mut task);
    add_profile_flag_maybe(args.profile, &mut task);

    task
}

fn clippy(args: &CheckArgs) -> CommandLine {
    // Clippy seems to use the same arguments as `cargo check`.
    // It looks like there is no way to specify doctests here.

    let mut task = vec![
        "cargo",
        "hack",
        "clippy",
        "--locked",
        "--workspace",
        "--all-targets",
    ];

    add_exclude_xtask_flag_maybe(args.exclude_xtask, &mut task);
    add_feature_flags(&args.rust_version, &mut task);
    add_profile_flag_maybe(args.profile, &mut task);

    task.extend(&["--", "-Dwarnings"]);

    task
}

fn test(args: &TestArgs) -> CommandLine {
    // WARNING: `--all-targets` enables benchmarks and disables doctests.
    let mut task = vec!["cargo", "hack", "test", "--locked", "--workspace"];

    add_exclude_xtask_flag_maybe(args.exclude_xtask, &mut task);
    add_feature_flags(&args.rust_version, &mut task);
    add_profile_flag_maybe(args.profile, &mut task);

    if args.include_ignored_tests {
        task.extend(&["--", "--include-ignored"]);
    }

    task
}

fn docs(args: &DocsArgs) -> CommandLine {
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
        "--no-deps",
    ];

    add_feature_flags(&args.rust_version, &mut task);
    add_profile_flag_maybe(args.profile, &mut task);

    task
}

fn build(args: &BuildArgs) -> CommandLine {
    let mut task = vec![
        "cargo",
        "hack",
        "build",
        "--locked",
        "--workspace",
        "--all-targets",
    ];

    add_exclude_xtask_flag_maybe(args.exclude_xtask, &mut task);
    add_feature_flags(&args.rust_version, &mut task);
    add_profile_flag_maybe(args.profile, &mut task);

    task
}

fn tarpaulin(args: &CoverageArgs) -> CommandLine {
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
        }
        Profile::Dev => {
            task.extend(&["--output-dir", "tarpaulin-report-dev"]);
        }
    }

    if args.include_ignored_tests {
        task.extend(&["--", "--include-ignored"]);
    }

    task
}

fn miri(args: &MiriArgs) -> [CommandLine; 3] {
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
    ];

    add_feature_flags(&args.rust_version, &mut test);

    if args.include_ignored_tests {
        test.extend(&["--", "--include-ignored"]);
    }

    [clean.clone(), test, clean]
}

fn deps() -> [CommandLine; 3] {
    let upgrades = vec!["cargo", "--locked", "upgrades"];
    let update = vec!["cargo", "--locked", "update"];
    let audit = vec!["cargo", "--locked", "audit", "--deny", "warnings"];

    [upgrades, update, audit]
}

fn add_exclude_xtask_flag_maybe(shall_exclude: bool, task: &mut CommandLine) {
    if shall_exclude {
        task.push("--exclude=xtask");
    }
}

fn add_feature_flags(
    rust_version: &Option<RustVersion>,
    task: &mut CommandLine,
) {
    task.extend(as_feature_flags(rust_version));
}

fn add_profile_flag_maybe(profile: Profile, task: &mut CommandLine) {
    match profile {
        Profile::Release => task.push("--release"),
        Profile::Dev => (),
    }
}

fn as_feature_flags(
    rust_version: &Option<RustVersion>,
) -> &'static [&'static str] {
    match rust_version {
        None => &[
            "--group-features=rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,\
             rust-v1.64",
            "--ignore-unknown-features",
            "--feature-powerset",
            "--optional-deps",
        ],
        Some(RustVersion::V1_81) => &[
            "--version-range=1.81..=1.81",
            "--exclude-features=default",
            "--features=rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
            "--ignore-unknown-features",
            "--feature-powerset",
            "--optional-deps",
        ],
        Some(RustVersion::V1_77) => &[
            "--version-range=1.77..=1.77",
            "--exclude-features=default",
            "--features=rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
            "--ignore-unknown-features",
            "--exclude-features=rust-v1.81",
            "--feature-powerset",
            "--optional-deps",
        ],
        Some(RustVersion::V1_69) => &[
            "--version-range=1.69..=1.69",
            "--exclude-features=default",
            "--features=rust-v1.69,rust-v1.66,rust-v1.64",
            "--ignore-unknown-features",
            "--exclude-features=rust-v1.81,rust-v1.77",
            "--feature-powerset",
            "--optional-deps",
        ],
        Some(RustVersion::V1_66) => &[
            "--version-range=1.66..=1.66",
            "--exclude-features=default",
            "--features=rust-v1.66,rust-v1.64",
            "--ignore-unknown-features",
            "--exclude-features=rust-v1.81,rust-v1.77,rust-v1.69",
            "--feature-powerset",
            "--optional-deps",
        ],
        Some(RustVersion::V1_64) => &[
            "--version-range=1.64..=1.64",
            "--exclude-features=default,eyre",
            "--features=rust-v1.64",
            "--ignore-unknown-features",
            "--exclude-features=rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66",
            "--feature-powerset",
            "--optional-deps",
        ],
        Some(RustVersion::V1_61) => &[
            "--version-range=1.61..=1.61",
            "--exclude-features=default,eyre",
            "--exclude-features=rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,\
             rust-v1.64",
            "--feature-powerset",
            "--optional-deps",
        ],
    }
}

#[cfg(test)]
mod tests {
    use lazy_errors::Result;
    use test_case::test_case;

    use super::*;

    #[test_case(
        &["xtask", "ci", "rustfmt"],
        &[&["cargo", "+nightly", "--locked", "fmt", "--check", "--all"]];
        "`rustfmt` task")]
    #[test_case(
        &["xtask", "ci", "clippy", "--profile=dev"],
        &[
            &[
                "cargo", "hack", "clippy",
                "--locked", "--workspace",
                "--all-targets",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--", "-Dwarnings",
            ]
        ]; "`clippy` task")]
    #[test_case(
        &["xtask", "ci", "test", "--profile=dev"],
        &[
            &[
                "cargo", "hack", "test",
                "--locked", "--workspace",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ]
        ]; "`test` task")]
    #[test_case(
        &["xtask", "ci", "build", "--profile=dev"],
        &[
            &[
                "cargo", "hack", "build",
                "--locked", "--workspace",
                "--all-targets",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ]
        ]; "`build` task (dev)")]
    #[test_case(
        &["xtask", "ci", "build", "--profile=release", "--exclude-xtask"],
        &[
            &[
                "cargo", "hack", "build",
                "--locked", "--workspace",
                "--all-targets",
                "--exclude=xtask",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--release",
            ],
        ]; "`build` task (release, w/o xtask)")]
    #[test_case(
        &["xtask", "ci", "tarpaulin", "--profile=dev"],
        &[
            &[
                "cargo", "tarpaulin",
                "--locked", "--workspace",
                "--all-features", "--all-targets", "--doc", "--no-fail-fast",
                "--output-dir", "tarpaulin-report-dev",
            ],
        ]; "`tarpaulin` task")]
    #[test_case(
        &["xtask", "ci", "miri"],
        &[
            &["cargo", "+nightly", "--locked", "clean"],
            &[
                "cargo", "+nightly", "hack", "miri", "test",
                "--locked", "--workspace",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &["cargo", "+nightly", "--locked", "clean"],
        ]; "`miri` task")]
    #[test_case(
        &["xtask", "ci", "docs", "--profile=dev"],
        &[
            &[
                "cargo", "hack", "doc",
                "--locked", "--workspace",
                "--no-deps",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ]
        ]; "`docs` task")]
    #[test_case(
        &["xtask", "ci", "deps"],
        &[
            &["cargo", "--locked", "upgrades"],
            &["cargo", "--locked", "update"],
            &["cargo", "--locked", "audit", "--deny", "warnings"],
        ]; "`deps` task")]
    #[test_case(
        &["xtask", "ci", "quick"],
        &[
            &["cargo", "+nightly", "--locked", "fmt", "--check", "--all"],
            &[
                "cargo", "hack", "clippy",
                "--locked", "--workspace",
                "--all-targets",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--", "-Dwarnings",
            ],
            &[
                "cargo", "hack", "test",
                "--locked", "--workspace",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &[
                "cargo", "hack", "doc",
                "--locked", "--workspace",
                "--no-deps",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
        ]; "default `quick` tasklist")]
    #[test_case(
        &[
            "xtask", "ci", "quick",
            "--include-ignored-tests",
            "--include-ignored-tests-in-coverage",
            "--skip-rustfmt",
            "--skip-build=false",
            "--skip-tarpaulin=false",
            "--skip-dependency-checks=false",
        ],
        &[
            &[
                "cargo", "hack", "clippy",
                "--locked", "--workspace",
                "--all-targets",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--", "-Dwarnings",
            ],
            &[
                "cargo", "hack", "test",
                "--locked", "--workspace",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--", "--include-ignored",
            ],
            &[
                "cargo", "hack", "doc",
                "--locked", "--workspace",
                "--no-deps",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &[
                "cargo", "hack", "build",
                "--locked", "--workspace",
                "--all-targets",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &[
                "cargo", "tarpaulin",
                "--locked", "--workspace",
                "--all-features", "--all-targets", "--doc", "--no-fail-fast",
                "--output-dir", "tarpaulin-report-dev",
                "--", "--include-ignored",
            ],
            &["cargo", "--locked", "upgrades"],
            &["cargo", "--locked", "update"],
            &["cargo", "--locked", "audit", "--deny", "warnings"],
        ]; "`quick` tasklist with inverted flags (w/o --skip-moving-targets)")]
    #[test_case(
        &["xtask", "ci", "quick", "--skip-moving-targets"],
        &[
            &[
                "cargo", "hack", "check",
                "--locked", "--workspace",
                "--all-targets",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &[
                "cargo", "hack", "test",
                "--locked", "--workspace",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &[
                "cargo", "hack", "doc",
                "--locked", "--workspace",
                "--no-deps",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
        ]; "`quick` tasklist with --skip-moving-targets")]
    #[test_case(
        &[
            "xtask", "ci", "all",
            "--profile=dev",
            "--skip-rustfmt",
            "--skip-build",
            "--skip-tarpaulin",
            "--skip-miri",
            "--skip-dependency-checks",
            "--skip-moving-targets",
        ],
        &[
            &[
                "cargo", "hack", "check",
                "--locked", "--workspace",
                "--all-targets",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &[
                "cargo", "hack", "test",
                "--locked", "--workspace",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &[
                "cargo", "hack", "doc",
                "--locked", "--workspace",
                "--no-deps",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
        ]; "minimal `all` tasklist")]
    #[test_case(
        &["xtask", "ci", "all"],
        &[
            &["cargo", "+nightly", "--locked", "fmt", "--check", "--all"],

            &[
                "cargo", "hack", "clippy",
                "--locked", "--workspace",
                "--all-targets",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--", "-Dwarnings",
            ],
            &[
                "cargo", "hack", "test",
                "--locked", "--workspace",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &[
                "cargo", "hack", "doc",
                "--locked", "--workspace",
                "--no-deps",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &[
                "cargo", "hack", "build",
                "--locked", "--workspace",
                "--all-targets",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &[
                "cargo", "tarpaulin",
                "--locked", "--workspace",
                "--all-features", "--all-targets", "--doc", "--no-fail-fast",
                "--output-dir", "tarpaulin-report-dev",
            ],

            &[
                "cargo", "hack", "clippy",
                "--locked", "--workspace",
                "--all-targets",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--release", "--", "-Dwarnings",
            ],
            &[
                "cargo", "hack", "test",
                "--locked", "--workspace",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--release",
            ],
            &[
                "cargo", "hack", "doc",
                "--locked", "--workspace",
                "--no-deps",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--release",
            ],
            &[
                "cargo", "hack", "build",
                "--locked", "--workspace",
                "--all-targets",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--release",
            ],
            &[
                "cargo", "tarpaulin",
                "--locked", "--workspace",
                "--all-features", "--all-targets", "--doc", "--no-fail-fast",
                "--output-dir", "tarpaulin-report-release",
                "--release",
            ],

            &["cargo", "+nightly", "--locked", "clean"],
            &[
                "cargo", "+nightly", "hack", "miri", "test",
                "--locked", "--workspace",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &["cargo", "+nightly", "--locked", "clean"],
            &["cargo", "--locked", "upgrades"],
            &["cargo", "--locked", "update" ],
            &["cargo", "--locked", "audit", "--deny", "warnings"],
        ]; "default `all` tasklist")]
    #[test_case(
        &["xtask", "ci", "all", "--skip-moving-targets"],
        &[
            &[
                "cargo", "hack", "check",
                "--locked", "--workspace",
                "--all-targets",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &[
                "cargo", "hack", "test",
                "--locked", "--workspace",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &[
                "cargo", "hack", "doc",
                "--locked", "--workspace",
                "--no-deps",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &[
                "cargo", "hack", "build",
                "--locked", "--workspace",
                "--all-targets",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],

            &[
                "cargo", "hack", "check",
                "--locked", "--workspace",
                "--all-targets",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--release",
            ],
            &[
                "cargo", "hack", "test",
                "--locked", "--workspace",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--release",
            ],
            &[
                "cargo", "hack", "doc",
                "--locked", "--workspace",
                "--no-deps",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--release",
            ],
            &[
                "cargo", "hack", "build",
                "--locked", "--workspace",
                "--all-targets",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--release",
            ],
        ]; "stable `all` tasklist")]
    fn transform_args_to_tasks(
        args: &[&str],
        tasklist: &[&[&str]],
    ) -> Result<()> {
        let tasks = tasklist_from(&parse_ci_args(args)?);
        assert_eq!(&tasks, tasklist);
        Ok(())
    }

    #[test_case(
        &["xtask", "ci", "all", "--include-ignored-tests"],
        &[
            &[
                "cargo", "hack", "test",
                "--locked", "--workspace",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--", "--include-ignored",
            ],
            &[
                "cargo", "tarpaulin",
                "--locked", "--workspace",
                "--all-features", "--all-targets", "--doc", "--no-fail-fast",
                "--output-dir", "tarpaulin-report-dev",
            ],
            &[
                "cargo", "hack", "test",
                "--locked", "--workspace",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--release", "--", "--include-ignored",
            ],
            &[
                "cargo", "tarpaulin",
                "--locked", "--workspace",
                "--all-features", "--all-targets", "--doc", "--no-fail-fast",
                "--output-dir", "tarpaulin-report-release",
                "--release",
            ],
        ]; "can run ignored tests w/o coverage")]
    #[test_case(
        &["xtask", "ci", "all", "--include-ignored-tests-in-coverage"],
        &[
            &[
                "cargo", "hack", "test",
                "--locked", "--workspace",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
            ],
            &[
                "cargo", "tarpaulin",
                "--locked", "--workspace",
                "--all-features", "--all-targets", "--doc", "--no-fail-fast",
                "--output-dir", "tarpaulin-report-dev",
                "--", "--include-ignored",
            ],
            &[
                "cargo", "hack", "test",
                "--locked", "--workspace",
                "--group-features=\
                   rust-v1.81,rust-v1.77,rust-v1.69,rust-v1.66,rust-v1.64",
                "--ignore-unknown-features",
                "--feature-powerset",
                "--optional-deps",
                "--release",
            ],
            &[
                "cargo", "tarpaulin",
                "--locked", "--workspace",
                "--all-features", "--all-targets", "--doc", "--no-fail-fast",
                "--output-dir", "tarpaulin-report-release",
                "--release", "--", "--include-ignored",
            ],
        ]; "can run ignored tests only for coverage")]
    fn tasklist_contains(
        args: &[&str],
        task_sublist: &[&[&str]],
    ) -> Result<()> {
        let mut tasks = super::tasklist_from(&parse_ci_args(args)?);
        tasks.retain(|task| task_sublist.contains(&task.as_ref()));
        assert_eq!(&tasks, task_sublist);
        Ok(())
    }

    fn parse_ci_args(args: &[&str]) -> Result<Ci> {
        match crate::parse_args(args)? {
            crate::Xtask::Ci(args) => Ok(args),
            other => panic!("Unexpected args type: {other:?}"),
        }
    }
}
