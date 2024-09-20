use core::{
    fmt::{self, Display},
    str::FromStr,
};

use lazy_errors::{prelude::*, Result};

#[derive(clap::Subcommand, Debug, Clone, PartialEq, Hash, Eq)]
pub enum Version {
    /// Extracts the version number from some source
    /// and writes it into the `Cargo.toml` and `Cargo.lock` files.
    Import(ImportArgs),
}

#[derive(clap::Args, Debug, Clone, PartialEq, Hash, Eq)]
pub struct ImportArgs {
    /// Where to import the version number from.
    source: Source,

    /// Whitelists version number formats that are allowed to import.
    ///
    /// If missing, any version number is accepted.
    /// If one or more patterns are present, the version number from `source`
    /// will be imported if it matches at least one of the patterns.
    /// Otherwise, an error will be returned.
    #[clap(long, value_name = "PATTERN")]
    accept: Vec<Pattern>,
}

#[derive(clap::ValueEnum, Debug, Copy, Clone, PartialEq, Hash, Eq)]
enum Source {
    /// Use the string returned from `git describe --dirty` as version number.
    GitDescribe,
}

#[derive(clap::ValueEnum, Debug, Copy, Clone, PartialEq, Hash, Eq)]
enum Pattern {
    /// Matches a “regular” version number,
    /// i.e. `MAJOR.MINOR.PATCH` strings if all parts are decimal numbers.
    MajorMinorPatch,
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
enum VersionNumber {
    MajorMinorPatch(MajorMinorPatch),
    CustomVersion(CustomVersion),
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Hash, Eq, Ord)]
struct MajorMinorPatch {
    major: u16,
    minor: u16,
    patch: u16,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Eq, Ord)]
struct CustomVersion(String);

impl FromStr for VersionNumber {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        if let Ok(v) = MajorMinorPatch::from_str(s) {
            return Ok(VersionNumber::MajorMinorPatch(v));
        }

        Ok(VersionNumber::CustomVersion(s.parse()?))
    }
}

impl FromStr for MajorMinorPatch {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut errs = ErrorStash::new(|| {
            format!("Doesn't match MAJOR.MINOR.PATCH: '{s}'")
        });

        let tokens: [&str; 3] = try2!(s
            .split('.')
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| {
                Error::from_message("Invalid number of parts separated by '.'")
            })
            .or_stash(&mut errs));

        let [major, minor, patch] = tokens.map(|tok| {
            u16::from_str(tok)
                .map_err(|_| -> Error { err!("Not a valid number: '{s}'") })
                .or_stash(&mut errs)
                .ok()
        });

        errs.into_result()?;

        Ok(Self {
            major: major.unwrap(),
            minor: minor.unwrap(),
            patch: patch.unwrap(),
        })
    }
}

impl FromStr for CustomVersion {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let s = s.trim();

        if s.is_empty() {
            return Err(err!("Version number is empty"));
        }

        Ok(Self(s.to_owned()))
    }
}

impl Display for VersionNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionNumber::MajorMinorPatch(v) => Display::fmt(v, f),
            VersionNumber::CustomVersion(v) => Display::fmt(v, f),
        }
    }
}

impl Display for MajorMinorPatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let major = self.major;
        let minor = self.minor;
        let patch = self.patch;

        write!(f, "{major}.{minor}.{patch}")
    }
}

impl Display for CustomVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn run(command: &Version) -> Result<()> {
    match command {
        Version::Import(args) => run_import(args),
    }
}

fn run_import(args: &ImportArgs) -> Result<()> {
    let version = crate::exec_and_capture(&["git", "describe", "--dirty"])?;
    let version = version_from_git_describe(&version)?;

    let is_accepted = args.accept.is_empty()
        || args
            .accept
            .iter()
            .any(|accept| match accept {
                Pattern::MajorMinorPatch => {
                    matches!(version, VersionNumber::MajorMinorPatch(_))
                }
            });

    if !is_accepted {
        return Err(err!(
            "Version '{version}' does not match any `accept` parameter"
        ));
    }

    crate::exec(&["cargo", "set-version", &version.to_string()])
}

fn version_from_git_describe(output: &str) -> Result<VersionNumber> {
    if output.is_empty() {
        return Err(err!("Version number is empty"));
    }

    let output = match output.strip_prefix('v') {
        Some(remainder) => remainder,
        None => output,
    };

    output.parse()
}
