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
            .map_err(|_| -> Error {
                err!("Invalid number of parts separated by '.'")
            })
            .or_stash(&mut errs));

        let [major, minor, patch]: [u16; 3] = try2!(tokens.try_map_or_stash(
            |token| {
                u16::from_str(token)
                    .map_err(|_| -> Error { err!("Invalid number: '{token}'") })
            },
            &mut errs
        ));

        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

impl FromStr for CustomVersion {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
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
        let Self {
            major,
            minor,
            patch,
        } = self;

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
    crate::exec_and_capture(&["git", "describe", "--dirty"])
        .and_then(|stdout| parse_and_filter(&stdout, &args.accept))
        .and_then(|v| crate::exec(&["cargo", "set-version", &v.to_string()]))
        .or_wrap_with(|| "Failed to set version number based on `git describe`")
}

fn parse_and_filter(
    git_output: &str,
    accept: &[Pattern],
) -> Result<VersionNumber> {
    let version = parse_git_describe_output(git_output)?;

    if !is_accepted(&version, accept) {
        return Err(err!(
            "Version '{version}' does not match any `accept` parameter"
        ));
    }

    Ok(version)
}

fn parse_git_describe_output(output: &str) -> Result<VersionNumber> {
    let output = output.trim();

    let output = match output.strip_prefix('v') {
        Some(remainder) => remainder,
        None => output,
    };

    output.parse()
}

fn is_accepted(version: &VersionNumber, accept: &[Pattern]) -> bool {
    accept.is_empty()
        || accept
            .iter()
            .any(|accept| match accept {
                Pattern::MajorMinorPatch => {
                    matches!(version, VersionNumber::MajorMinorPatch(_))
                }
            })
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;

    fn v(major: u16, minor: u16, patch: u16) -> VersionNumber {
        VersionNumber::MajorMinorPatch(MajorMinorPatch {
            major,
            minor,
            patch,
        })
    }

    fn custom(s: &str) -> VersionNumber {
        VersionNumber::CustomVersion(CustomVersion(s.to_owned()))
    }

    #[test_case("1.2.3", &[], Ok(v(1, 2, 3)))]
    #[test_case("1.2.3", &[Pattern::MajorMinorPatch], Ok(v(1, 2, 3)))]
    #[test_case("v1.2.3", &[], Ok(v(1, 2, 3)))]
    #[test_case("v1.2.3", &[Pattern::MajorMinorPatch], Ok(v(1, 2, 3)))]
    #[test_case("0.5.0-2-ga712af5", &[],
        Ok(custom("0.5.0-2-ga712af5")))]
    #[test_case("0.5.0-2-ga712af5", &[Pattern::MajorMinorPatch],
        Err(String::from(
            "Version '0.5.0-2-ga712af5' does not match any `accept` parameter"
        )))]
    #[test_case("v0.5.0-2-ga712af5", &[],
        Ok(custom("0.5.0-2-ga712af5")))]
    #[test_case("v0.5.0-2-ga712af5", &[Pattern::MajorMinorPatch],
        Err(String::from(
            "Version '0.5.0-2-ga712af5' does not match any `accept` parameter"
        )))]
    fn parse_and_filter(
        input: &str,
        accept: &[Pattern],
        expectation: Result<VersionNumber, String>,
    ) {
        let actual = super::parse_and_filter(input, accept);

        match expectation {
            Ok(v) => assert_eq!(v, actual.unwrap()),
            Err(e) => {
                let actual = actual.unwrap_err();
                dbg!(&actual);
                assert_eq!(actual.to_string(), e);
            }
        }
    }

    #[test]
    fn parse_major_minor_patch_multiple_err() {
        let err = super::MajorMinorPatch::from_str("-1.-2.-3").unwrap_err();
        let msg = format!("{err:#}");
        eprintln!("{}", msg);

        assert!(msg.starts_with("Doesn't match MAJOR.MINOR.PATCH: '-1.-2.-3'"));
        assert!(msg.contains("Invalid number: '-1'"));
        assert!(msg.contains("Invalid number: '-2'"));
        assert!(msg.contains("Invalid number: '-3'"));
    }

    #[test_case("0.0.0", v(0, 0, 0))]
    #[test_case("0.0.7", v(0, 0, 7))]
    #[test_case("0.7.0", v(0, 7, 0))]
    #[test_case("7.0.0", v(7, 0, 0))]
    #[test_case("1.2.3", v(1, 2, 3))]
    #[test_case("v0.0.0", v(0, 0, 0))]
    #[test_case("v0.0.7", v(0, 0, 7))]
    #[test_case("v0.7.0", v(0, 7, 0))]
    #[test_case("v7.0.0", v(7, 0, 0))]
    #[test_case("v1.2.3", v(1, 2, 3))]
    #[test_case("0.5.0-2-ga712af5", custom("0.5.0-2-ga712af5"))]
    #[test_case("v0.5.0-2-ga712af5", custom("0.5.0-2-ga712af5"))]
    #[test_case(" \n  v0.5.0-2-ga712af5 \n  ", custom("0.5.0-2-ga712af5"))]
    #[test_case("abcdef", custom("abcdef"))]
    #[test_case("foobar", custom("foobar"))]
    #[test_case("-1.-2.-3", custom("-1.-2.-3"))]
    fn parse_git_describe_output(input: &str, expectation: VersionNumber) {
        let actual = super::parse_git_describe_output(input).unwrap();
        assert_eq!(actual, expectation);
    }

    #[test_case(""; "empty")]
    #[test_case(" \n\t\r"; "only whitespace")]
    fn parse_git_describe_output_err(input: &str) {
        assert!(super::parse_git_describe_output(input).is_err());
    }

    #[test_case(v(0, 0, 0), "0.0.0")]
    #[test_case(v(0, 0, 7), "0.0.7")]
    #[test_case(v(0, 7, 0), "0.7.0")]
    #[test_case(v(7, 0, 0), "7.0.0")]
    #[test_case(v(1, 2, 3), "1.2.3")]
    #[test_case(custom("0.5.0-2-ga712af5"), "0.5.0-2-ga712af5")]
    #[test_case(custom("v0.5.0-2-ga712af5"), "v0.5.0-2-ga712af5")]
    fn display_version_number(input: VersionNumber, expectation: &str) {
        assert_eq!(&input.to_string(), expectation);
    }

    #[test_case(v(0, 0, 0), &[], true)]
    #[test_case(v(0, 0, 7), &[], true)]
    #[test_case(v(0, 7, 0), &[], true)]
    #[test_case(v(7, 0, 0), &[], true)]
    #[test_case(v(1, 2, 3), &[], true)]
    #[test_case(custom("0.5.0-2-ga712af5"), &[], true)]
    #[test_case(v(0, 0, 0), &[Pattern::MajorMinorPatch], true)]
    #[test_case(v(0, 0, 7), &[Pattern::MajorMinorPatch], true)]
    #[test_case(v(0, 7, 0), &[Pattern::MajorMinorPatch], true)]
    #[test_case(v(7, 0, 0), &[Pattern::MajorMinorPatch], true)]
    #[test_case(v(1, 2, 3), &[Pattern::MajorMinorPatch], true)]
    #[test_case(custom("0.5.0-2-ga712af5"), &[Pattern::MajorMinorPatch], false)]
    #[test_case(
        v(0, 0, 0),
        &[Pattern::MajorMinorPatch, Pattern::MajorMinorPatch],
        true)]
    #[test_case(
        v(0, 0, 7),
        &[Pattern::MajorMinorPatch, Pattern::MajorMinorPatch],
        true)]
    #[test_case(
        v(0, 7, 0),
        &[Pattern::MajorMinorPatch, Pattern::MajorMinorPatch],
        true)]
    #[test_case(
        v(7, 0, 0),
        &[Pattern::MajorMinorPatch, Pattern::MajorMinorPatch],
        true)]
    #[test_case(
        v(1, 2, 3),
        &[Pattern::MajorMinorPatch, Pattern::MajorMinorPatch],
        true)]
    #[test_case(
        custom("0.5.0-2-ga712af5"),
        &[Pattern::MajorMinorPatch, Pattern::MajorMinorPatch],
        false)]
    fn is_accepted(v: VersionNumber, accept: &[Pattern], expectation: bool) {
        let actual = super::is_accepted(&v, accept);
        assert_eq!(actual, expectation);
    }
}
