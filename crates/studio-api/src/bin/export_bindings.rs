//! `export-bindings` — the explicit generator and drift check for the TypeScript DTO contract.
//!
//! ```text
//! cargo run -p studio-api --bin export-bindings -- --check
//! cargo run -p studio-api --bin export-bindings -- --write
//! cargo run -p studio-api --bin export-bindings -- --check --dir <path>
//! ```
//!
//! `--check` renders the contract into an isolated temporary directory, compares it with the
//! binding tree and exits non-zero on drift. It repairs nothing. `--write` is the only deliberate
//! installation path.
//!
//! Argument parsing is total and happens **before** any render or write: exactly one mode is
//! required, and conflicting modes, repeated modes, a repeated `--dir` or a `--dir` without a
//! value are all rejected up front. There is no path from an invalid invocation to the writer.

use std::path::PathBuf;
use std::process::ExitCode;

use studio_api::bindings::{self, BindingsError, Recovery};

const USAGE: &str = "\
export-bindings — generate / verify the TypeScript DTO bindings

USAGE:
    cargo run -p studio-api --bin export-bindings -- <--check | --write> [--dir <path>]

MODES (exactly one required):
    --check   Render the DTO contract to a temporary directory, compare it with the binding tree
              and report sorted missing / changed / obsolete paths. Never writes. Exits 1 on drift.
    --write   Install the rendered contract into the binding tree. The only deliberate write path.

OPTIONS:
    --dir <path>   Operate on this directory instead of app/src/bindings. May be given once.
                   The path may not be empty or start with `-`; write ./-name for such a
                   directory.
    -h, --help     Print this message.
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Check,
    Write,
}

/// A fully-validated invocation. Constructing one is the *only* way to reach the writer.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Invocation {
    mode: Mode,
    dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Parsed {
    Run(Invocation),
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliError {
    NoMode,
    ConflictingModes,
    RepeatedMode(&'static str),
    RepeatedDir,
    MissingDirValue,
    InvalidDirValue(String),
    Unrecognised(String),
}

impl CliError {
    fn message(&self) -> String {
        match self {
            Self::NoMode => "exactly one of --check or --write is required".into(),
            Self::ConflictingModes => "--check and --write are mutually exclusive".into(),
            Self::RepeatedMode(m) => format!("{m} was given more than once"),
            Self::RepeatedDir => "--dir was given more than once".into(),
            Self::MissingDirValue => "--dir requires a path".into(),
            Self::InvalidDirValue(v) if v.is_empty() => "--dir requires a non-empty path".into(),
            Self::InvalidDirValue(v) => format!(
                "--dir value `{v}` looks like an option, not a path; \
                 pass a leading-dash directory as `./{v}`"
            ),
            Self::Unrecognised(a) => format!("unrecognised argument `{a}`"),
        }
    }
}

/// Total argument parsing. Rejects every invalid form before any work is done.
fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Parsed, CliError> {
    let mut mode: Option<Mode> = None;
    let mut dir: Option<PathBuf> = None;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Parsed::Help),
            flag @ ("--check" | "--write") => {
                let requested = if flag == "--check" {
                    Mode::Check
                } else {
                    Mode::Write
                };
                match mode {
                    None => mode = Some(requested),
                    Some(existing) if existing == requested => {
                        return Err(CliError::RepeatedMode(if flag == "--check" {
                            "--check"
                        } else {
                            "--write"
                        }))
                    }
                    Some(_) => return Err(CliError::ConflictingModes),
                }
            }
            "--dir" => {
                if dir.is_some() {
                    return Err(CliError::RepeatedDir);
                }
                match args.next() {
                    None => return Err(CliError::MissingDirValue),
                    // An empty or option-looking value is a mistake, not a directory. Without
                    // this, `--write --dir --check` silently installs into a directory literally
                    // named `--check`. A genuine leading-dash directory is passed as `./--check`.
                    Some(path) if path.is_empty() || path.starts_with('-') => {
                        return Err(CliError::InvalidDirValue(path))
                    }
                    Some(path) => dir = Some(PathBuf::from(path)),
                }
            }
            other => return Err(CliError::Unrecognised(other.to_string())),
        }
    }

    match mode {
        Some(mode) => Ok(Parsed::Run(Invocation { mode, dir })),
        None => Err(CliError::NoMode),
    }
}

fn main() -> ExitCode {
    let invocation = match parse(std::env::args().skip(1)) {
        Ok(Parsed::Help) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Ok(Parsed::Run(invocation)) => invocation,
        Err(e) => {
            eprintln!("{}\n\n{USAGE}", e.message());
            return ExitCode::from(2);
        }
    };

    let dir = invocation.dir.unwrap_or_else(bindings::binding_dir);

    match invocation.mode {
        Mode::Check => match bindings::check(&dir) {
            Ok(report) => {
                println!("{}", dir.display());
                println!("{} export root(s).", bindings::export_root_names().len());
                println!("{report}");
                if !report.unowned.is_empty() {
                    println!(
                        "\npreserved, not generator-owned ({}): {}",
                        report.unowned.len(),
                        report.unowned.join(", ")
                    );
                }
                if report.is_clean() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(e) => {
                eprintln!("check failed: {e}");
                ExitCode::from(2)
            }
        },
        Mode::Write => match bindings::write(&dir) {
            Ok(summary) => {
                println!("{}", dir.display());
                println!(
                    "wrote {}, removed {}, unchanged {}, preserved {}",
                    summary.written.len(),
                    summary.removed.len(),
                    summary.unchanged,
                    summary.preserved.len()
                );
                for name in &summary.written {
                    println!("  + {name}");
                }
                for name in &summary.removed {
                    println!("  - {name}");
                }
                ExitCode::SUCCESS
            }
            // Each failure class gets its own diagnosis. Only the classes that genuinely happen
            // before any mutation say so; an installation failure reports its recovery outcome.
            Err(e) => {
                eprintln!("{e}");
                match e {
                    BindingsError::Render(_)
                    | BindingsError::Io { .. }
                    | BindingsError::Ownership(_)
                    | BindingsError::Preflight(_) => ExitCode::from(2),
                    BindingsError::Install {
                        recovery: Recovery::RolledBack,
                        ..
                    } => ExitCode::from(3),
                    BindingsError::Install {
                        recovery: Recovery::Failed { .. },
                        ..
                    } => ExitCode::from(4),
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<Parsed, CliError> {
        parse(args.iter().map(|s| (*s).to_string()))
    }

    fn run(args: &[&str]) -> Invocation {
        match parse_args(args) {
            Ok(Parsed::Run(inv)) => inv,
            other => panic!("expected a runnable invocation, got {other:?}"),
        }
    }

    #[test]
    fn valid_invocations_select_exactly_one_mode() {
        assert_eq!(run(&["--check"]).mode, Mode::Check);
        assert_eq!(run(&["--write"]).mode, Mode::Write);
        assert_eq!(run(&["--check"]).dir, None);

        let inv = run(&["--write", "--dir", "some/where"]);
        assert_eq!(inv.mode, Mode::Write);
        assert_eq!(inv.dir, Some(PathBuf::from("some/where")));

        // Order does not matter.
        assert_eq!(
            run(&["--dir", "x", "--check"]),
            run(&["--check", "--dir", "x"])
        );
    }

    #[test]
    fn help_never_yields_a_runnable_invocation() {
        for args in [
            vec!["--help"],
            vec!["-h"],
            vec!["--write", "--help"],
            vec!["--help", "--write"],
        ] {
            assert_eq!(parse_args(&args), Ok(Parsed::Help), "{args:?}");
        }
    }

    /// **The safety property:** no invalid invocation can produce an `Invocation`, so none can
    /// reach `bindings::write`. `main` calls the writer only on `Ok(Parsed::Run { mode: Write })`.
    #[test]
    fn invalid_invocations_cannot_reach_the_writer() {
        let cases: &[(&[&str], CliError)] = &[
            (&[], CliError::NoMode),
            (&["--dir", "x"], CliError::NoMode),
            (&["--check", "--write"], CliError::ConflictingModes),
            (&["--write", "--check"], CliError::ConflictingModes),
            (
                &["--write", "--dir", "x", "--check"],
                CliError::ConflictingModes,
            ),
            (&["--check", "--check"], CliError::RepeatedMode("--check")),
            (&["--write", "--write"], CliError::RepeatedMode("--write")),
            (
                &["--check", "--dir", "a", "--dir", "b"],
                CliError::RepeatedDir,
            ),
            (&["--write", "--dir"], CliError::MissingDirValue),
            (&["--dir"], CliError::MissingDirValue),
            (
                &["--write", "--dir", "--check"],
                CliError::InvalidDirValue("--check".into()),
            ),
            (
                &["--check", "--dir", ""],
                CliError::InvalidDirValue(String::new()),
            ),
            (&["--oops"], CliError::Unrecognised("--oops".into())),
            (
                &["--check", "stray"],
                CliError::Unrecognised("stray".into()),
            ),
            (&["--Check"], CliError::Unrecognised("--Check".into())),
        ];

        for (args, expected) in cases {
            let got = parse_args(args);
            assert_eq!(got, Err(expected.clone()), "args {args:?}");
            assert!(
                !matches!(got, Ok(Parsed::Run(_))),
                "invalid args {args:?} produced a runnable invocation"
            );
        }
    }

    /// A mode flag must never be swallowed as a `--dir` value. Both orders are covered: the
    /// mode before `--dir` and the mode after it.
    #[test]
    fn a_mode_flag_is_never_consumed_as_a_directory() {
        for args in [
            ["--write", "--dir", "--check"],
            ["--check", "--dir", "--write"],
            ["--dir", "--check", "--write"],
            ["--dir", "--write", "--check"],
        ] {
            let got = parse_args(&args);
            assert!(
                matches!(got, Err(CliError::InvalidDirValue(_))),
                "args {args:?} were not rejected: {got:?}"
            );
            assert!(
                !matches!(got, Ok(Parsed::Run(_))),
                "args {args:?} reached the writer"
            );
        }
    }

    /// Empty and option-looking `--dir` values are rejected; a real leading-dash directory is
    /// still expressible as `./-name`.
    #[test]
    fn dir_values_are_validated() {
        assert_eq!(
            parse_args(&["--check", "--dir", ""]),
            Err(CliError::InvalidDirValue(String::new()))
        );
        assert_eq!(
            parse_args(&["--check", "--dir", "-x"]),
            Err(CliError::InvalidDirValue("-x".into()))
        );
        assert_eq!(
            run(&["--check", "--dir", "./--check"]).dir,
            Some(PathBuf::from("./--check")),
            "a legitimate dash-leading directory must still be passable"
        );
        assert_eq!(
            run(&["--check", "--dir", "app/src/bindings"]).dir,
            Some(PathBuf::from("app/src/bindings"))
        );
    }

    #[test]
    fn every_cli_error_has_a_message() {
        for e in [
            CliError::NoMode,
            CliError::ConflictingModes,
            CliError::RepeatedMode("--check"),
            CliError::RepeatedDir,
            CliError::MissingDirValue,
            CliError::InvalidDirValue(String::new()),
            CliError::InvalidDirValue("-x".into()),
            CliError::Unrecognised("--x".into()),
        ] {
            assert!(!e.message().is_empty(), "{e:?}");
        }
    }
}
