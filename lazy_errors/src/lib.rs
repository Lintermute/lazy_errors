#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

//! Effortlessly create, group, and nest arbitrary errors,
//! and defer error handling ergonomically.
//!
//! ```
//! # use core::str::FromStr;
//! #[cfg(feature = "std")]
//! use lazy_errors::{prelude::*, Result};
//!
//! #[cfg(not(feature = "std"))]
//! use lazy_errors::surrogate_error_trait::{prelude::*, Result};
//!
//! fn run(input1: &str, input2: &str) -> Result<()>
//! {
//!     let mut errs = ErrorStash::new(|| "There were one or more errors");
//!
//!     u8::from_str("42").or_stash(&mut errs); // `errs` contains 0 errors
//!     u8::from_str("❌").or_stash(&mut errs); // `errs` contains 1 error
//!     u8::from_str("1337").or_stash(&mut errs); // `errs` contains 2 errors
//!
//!     // `input1` is very important in this example,
//!     // so make sure it has a nice message.
//!     let r: Result<u8> = u8::from_str(input1)
//!         .or_wrap_with(|| format!("Input '{input1}' is invalid"));
//!
//!     // If `input1` is invalid, we don't want to continue
//!     // but return _all_ errors that have occurred so far.
//!     let input1: u8 = try2!(r.or_stash(&mut errs));
//!     println!("input1 = {input1:#X}");
//!
//!     // Continue handling other `Result`s.
//!     u8::from_str(input2).or_stash(&mut errs);
//!
//!     errs.into() // `Ok(())` if `errs` is still empty, `Err` otherwise
//! }
//!
//! fn main()
//! {
//!     let err = run("❓", "❗").unwrap_err();
//!     let n = err.children().len();
//!     eprintln!("Got an error with {n} children.");
//!     eprintln!("---------------------------------------------------------");
//!     eprintln!("{err:#}");
//! }
//! ```
//!
//! Running the example will print:
//!
//! ```text
//! Got an error with 3 children.
//! ---------------------------------------------------------
//! There were one or more errors
//! - invalid digit found in string
//!   at src/main.rs:10:24
//! - number too large to fit in target type
//!   at src/main.rs:11:26
//! - Input '❓' is invalid: invalid digit found in string
//!   at src/main.rs:16:10
//!   at src/main.rs:20:30
//! ```
//!
//! # In a Nutshell
//!
//! `lazy_errors` provides types, traits, and blanket implementations
//! on `Result` that can be used to ergonomically defer error handling.
//! `lazy_errors` allows you to easily create ad-hoc errors
//! as well as wrap a wide variety of errors in a single common error type,
//! simplifying your codebase.
//! In that latter regard, it is similar to `anyhow`/`eyre`,
//! except that its reporting isn't as fancy or detailed (for example,
//! `lazy_errors` tracks source code file name and line numbers instead of
//! providing full `std::backtrace` support).
//! On the other hand, `lazy_errors` adds methods to `Result`
//! that let you continue on failure,
//! deferring returning `Err` results.
//! `lazy_errors` allows you to return two or more errors
//! from functions simultaneously and ergonomically.
//! `lazy_error` also supports nested errors.
//! When you return nested errors from functions,
//! errors will form a tree while “bubbling up”.
//! You can report that error tree the user/developer in its entirety.
//!
//! By default, `lazy_errors` will box your error values (like `anyhow`/`eyre`),
//! which allows you to use different error types in the same `Result` type.
//! However, `lazy_errors` will respect static error type information
//! if you provide it explicitly.
//! If you do so, you can access fields and methods of your error values
//! at run-time without needing downcasts.
//! Both modes of operation can work together, as will be shown
//! in the example on the bottom of the page.
//!
//! While `lazy_error` integrates with `std::error::Error` by default,
//! it also supports `#![no_std]` if you disable the `std` feature.
//! When you define a few simple type aliases,
//! `lazy_errors` easily supports error types that aren't
//! `Sync` or even `Send`.
//!
//! Common reasons to use this crate are:
//!
//! - You want to return an error but run some fallible cleanup logic before.
//! - More generally, you're calling two or more functions that return `Result`,
//!   and want to return an error that wraps all errors that occurred.
//! - You're spawning several parallel activities, wait for their completion,
//!   and want to return all errors that occurred.
//! - You want to aggregate multiple errors before running some reporting or
//!   recovery logic, iterating over all errors collected.
//! - You need to handle errors that don't implement
//!   `std::error::Error`/`Display`/`Debug`/`Send`/`Sync` or other common
//!   traits.
//!
//! # Feature Flags
//!
//! - `std`:
//!   - Support error types that implement `std::error::Error`.
//!   - Implement `std::error::Error` for `lazy_error` error types.
//! - `eyre`: Adds `into_eyre_result` and `into_eyre_report` conversions.
//! - `rust-vN` (where `N` is a Rust version number): Does nothing more than add
//!   support for some error types from `core` and `alloc` that were stabilized
//!   in the respective Rust version.
//!
//! # MSRV
//!
//! The MSRV of `lazy_errors` depends on the set of enabled features:
//!
//! - Rust 1.77 supports all features and combinations thereof.
//! - Rust versions 1.61 .. 1.77 need you to disable all `rust-vN` features
//!   where `N` is greater than the version of your Rust toolchain. For example,
//!   to compile `lazy_errors` on Rust 1.66, you have to disable `rust-v1.77`
//!   and `rust-v1.69`, but not `rust-v1.66`.
//! - `eyre` needs at least Rust 1.65.
//! - Rust versions older than 1.61 are unsupported.
//!
//! # Walkthrough
//!
//! `lazy_errors` can actually support any error type as long as it's `Sized`;
//! it doesn't even need to be `Send` or `Sync`. You only need to specify
//! the generic type parameters accordingly, as will be shown in the example
//! on the bottom of this page. Usually however, you'd want to use the
//! aliased types from the [`prelude`]. When you're using these aliases,
//! errors will be boxed and you can dynamically return groups of errors
//! of differing types from the same function.
//!
//! The `std` feature is enabled by default, making `lazy_error` support
//! third-party error types that implement `std::error::Error`.
//! All error types from this crate will implement `std::error::Error` as well
//! in that case.
//! If you need `#![no_std]` support, you can disable the `std` feature
//! and use the [`surrogate_error_trait::prelude`] instead.
//! If you do so, `lazy_errors` will box any error type that implements the
//! [`surrogate_error_trait::Reportable`] marker trait.
//! If necessary, you can implement that trait for your custom types as well
//! (it's just a single line).
//!
//! While `lazy_errors` works standalone, it's not intended to replace
//! `anyhow` or `eyre`. Instead, this project was started to explore
//! approaches on how to run multiple fallible operations, aggregate
//! their errors (if any), and defer the actual error handling/reporting
//! by returning all of these errors from functions that return `Result`.
//! Generally, `Result<_, Vec<_>>` can be used for this purpose,
//! which is not much different from what `lazy_errors` does internally.
//! However, `lazy_errors` provides “syntactic sugar”
//! to make this approach more ergonomic.
//! Thus, arguably the most useful method in this crate is [`or_stash`].
//!
//! ### Example: `or_stash`
//!
//! [`or_stash`] is arguably the most useful method of this crate.
//! It becomes available on `Result` as soon as you
//! import the [`OrStash`] trait or the [`prelude`].
//! Here's an example:
//!
//! ```
//! # use lazy_errors::doctest_line_num_helper as replace_line_numbers;
//! #[cfg(feature = "std")]
//! use lazy_errors::{prelude::*, Result};
//!
//! #[cfg(not(feature = "std"))]
//! use lazy_errors::surrogate_error_trait::{prelude::*, Result};
//!
//! fn run() -> Result<()>
//! {
//!     let mut stash = ErrorStash::new(|| "Failed to run application");
//!
//!     print_if_ascii("❓").or_stash(&mut stash);
//!     print_if_ascii("❗").or_stash(&mut stash);
//!     print_if_ascii("42").or_stash(&mut stash);
//!
//!     cleanup().or_stash(&mut stash); // Runs regardless of earlier errors
//!
//!     stash.into() // `Ok(())` if the stash was still empty
//! }
//!
//! fn print_if_ascii(text: &str) -> Result<()>
//! {
//!     if !text.is_ascii() {
//!         return Err(err!("Input is not ASCII: '{text}'"));
//!     }
//!
//!     println!("{text}");
//!     Ok(())
//! }
//!
//! fn cleanup() -> Result<()>
//! {
//!     Err(err!("Cleanup failed"))
//! }
//!
//! fn main()
//! {
//!     let err = run().unwrap_err();
//!     let printed = format!("{err:#}");
//!     let printed = replace_line_numbers(&printed);
//!     assert_eq!(printed, indoc::indoc! {"
//!         Failed to run application
//!         - Input is not ASCII: '❓'
//!           at src/lib.rs:1234:56
//!           at src/lib.rs:1234:56
//!         - Input is not ASCII: '❗'
//!           at src/lib.rs:1234:56
//!           at src/lib.rs:1234:56
//!         - Cleanup failed
//!           at src/lib.rs:1234:56
//!           at src/lib.rs:1234:56"});
//! }
//! ```
//!
//! In the example above, `run()` will print `42`, run `cleanup()`,
//! and then return the stashed errors.
//!
//! Note that the [`ErrorStash`] is created manually in the example above.
//! The [`ErrorStash`] is empty before the first error is added.
//! Converting an empty [`ErrorStash`] to [`Result`] will produce `Ok(())`.
//! When [`or_stash`] is called on `Result::Err(e)`,
//! `e` will be moved into the [`ErrorStash`]. As soon as there is
//! at least one error stored in the [`ErrorStash`], converting [`ErrorStash`]
//! into [`Result`] will yield a `Result::Err` that contains an [`Error`],
//! the main error type from this crate.
//!
//! ### Example: `or_create_stash`
//!
//! Sometimes you don't want to create an empty [`ErrorStash`] beforehand.
//! In that case you can call [`or_create_stash`] on `Result`
//! to create a non-empty container on-demand, whenever necessary.
//! When [`or_create_stash`] is called on `Result::Err`, the error
//! will be put into a [`StashWithErrors`] instead of an [`ErrorStash`].
//! [`ErrorStash`] and [`StashWithErrors`] behave similarly.
//! While both [`ErrorStash`] and [`StashWithErrors`] can take additional
//! errors, a [`StashWithErrors`] is guaranteed to be non-empty.
//! The type system will be aware that there is at least one error.
//! Thus, while [`ErrorStash`] can only be converted into [`Result`],
//! yielding either `Ok(())` or `Err(e)` (where `e` is [`Error`]),
//! this distinction allows converting [`StashWithErrors`] into [`Error`]
//! directly.
//!
//! ```
//! # use lazy_errors::doctest_line_num_helper as replace_line_numbers;
//! #[cfg(feature = "std")]
//! use lazy_errors::{prelude::*, Result};
//!
//! #[cfg(not(feature = "std"))]
//! use lazy_errors::surrogate_error_trait::{prelude::*, Result};
//!
//! fn run() -> Result<()>
//! {
//!     match write("❌").or_create_stash(|| "Failed to run application") {
//!         Ok(()) => Ok(()),
//!         Err(mut stash) => {
//!             cleanup().or_stash(&mut stash);
//!             Err(stash.into())
//!         },
//!     }
//! }
//!
//! fn write(text: &str) -> Result<()>
//! {
//!     if !text.is_ascii() {
//!         return Err(err!("Input is not ASCII: '{text}'"));
//!     }
//!     Ok(())
//! }
//!
//! fn cleanup() -> Result<()>
//! {
//!     Err(err!("Cleanup failed"))
//! }
//!
//! fn main()
//! {
//!     let err = run().unwrap_err();
//!     let printed = format!("{err:#}");
//!     let printed = replace_line_numbers(&printed);
//!     assert_eq!(printed, indoc::indoc! {"
//!         Failed to run application
//!         - Input is not ASCII: '❌'
//!           at src/lib.rs:1234:56
//!           at src/lib.rs:1234:56
//!         - Cleanup failed
//!           at src/lib.rs:1234:56
//!           at src/lib.rs:1234:56"});
//! }
//! ```
//!
//! ### Example: `into_eyre_*`
//!
//! [`ErrorStash`] and [`StashWithErrors`] can be converted into
//! [`Result`] and [`Error`], respectively. A similar, albeit lossy,
//! conversion from [`ErrorStash`] and [`StashWithErrors`] exist for
//! `eyre::Result` and `eyre::Error` (i.e. `eyre::Report`), namely
#![cfg_attr(
    not(feature = "eyre"),
    doc = "`into_eyre_result` and `into_eyre_report`."
)]
#![cfg_attr(
    feature = "eyre",
    doc = r##"[`into_eyre_result`](IntoEyreResult::into_eyre_result) and
[`into_eyre_report`](IntoEyreReport::into_eyre_report):

```
# use lazy_errors::doctest_line_num_helper as replace_line_numbers;
use lazy_errors::prelude::*;
use eyre::bail;

fn run() -> Result<(), eyre::Report>
{
    let r = write("❌").or_create_stash::<Stashable>(|| "Failed to run");
    match r {
        Ok(()) => Ok(()),
        Err(mut stash) => {
            cleanup().or_stash(&mut stash);
            bail!(stash.into_eyre_report());
        },
    }
}

fn write(text: &str) -> Result<(), Error>
{
    if !text.is_ascii() {
        return Err(err!("Input is not ASCII: '{text}'"));
    }
    Ok(())
}

fn cleanup() -> Result<(), Error>
{
    Err(err!("Cleanup failed"))
}

fn main()
{
    let err = run().unwrap_err();
    let printed = format!("{err:#}");
    let printed = replace_line_numbers(&printed);
    assert_eq!(printed, indoc::indoc! {"
        Failed to run
        - Input is not ASCII: '❌'
          at src/lib.rs:1234:56
          at src/lib.rs:1234:56
        - Cleanup failed
          at src/lib.rs:1234:56
          at src/lib.rs:1234:56"});
}
```
"##
)]
//! ### Example: Hierarchies
//!
//! As you might have noticed, [`Error`]s form hierarchies:
//!
//! ```
//! # use lazy_errors::doctest_line_num_helper as replace_line_numbers;
//! #[cfg(feature = "std")]
//! use lazy_errors::{prelude::*, Result};
//!
//! #[cfg(not(feature = "std"))]
//! use lazy_errors::surrogate_error_trait::{prelude::*, Result};
//!
//! fn parent() -> Result<()>
//! {
//!     let mut stash = ErrorStash::new(|| "In parent(): child() failed");
//!     stash.push(child().unwrap_err());
//!     stash.into()
//! }
//!
//! fn child() -> Result<()>
//! {
//!     let mut stash = ErrorStash::new(|| "In child(): There were errors");
//!     stash.push("First error");
//!     stash.push("Second error");
//!     stash.into()
//! }
//!
//! fn main()
//! {
//!     let err = parent().unwrap_err();
//!     let printed = format!("{err:#}");
//!     let printed = replace_line_numbers(&printed);
//!     assert_eq!(printed, indoc::indoc! {"
//!         In parent(): child() failed
//!         - In child(): There were errors
//!           - First error
//!             at src/lib.rs:1234:56
//!           - Second error
//!             at src/lib.rs:1234:56
//!           at src/lib.rs:1234:56"});
//! }
//! ```
//!
//! The example above may seem unwieldy. In fact, that example only serves
//! the purpose to illustrate the error hierarchy.
//! In practice, you wouldn't write such code.
//! Instead, you'd probably rely on [`or_wrap`] or [`or_wrap_with`].
//!
//! ### Example: Wrapping
//!
//! You can use [`or_wrap`] or [`or_wrap_with`] to wrap any value
//! that can be converted into the
//! [_inner error type_ of `Error`](Error#inner-error-type-i)
//! or to attach some context to an error:
//!
//! ```
//! # use lazy_errors::doctest_line_num_helper as replace_line_numbers;
//! #[cfg(feature = "std")]
//! use lazy_errors::{prelude::*, Result};
//!
//! #[cfg(not(feature = "std"))]
//! use lazy_errors::surrogate_error_trait::{prelude::*, Result};
//!
//! fn run(s: &str) -> Result<u32>
//! {
//!     parse(s).or_wrap_with(|| format!("Not an u32: '{s}'"))
//! }
//!
//! fn parse(s: &str) -> Result<u32>
//! {
//!     let r: Result<u32, core::num::ParseIntError> = s.parse();
//!
//!     // Wrap the error type “silently”:
//!     // No additional message, just file location and wrapped error type.
//!     r.or_wrap()
//! }
//!
//! fn main()
//! {
//!     let err = run("❌").unwrap_err();
//!     let printed = format!("{err:#}");
//!     let printed = replace_line_numbers(&printed);
//!     assert_eq!(printed, indoc::indoc! {"
//!         Not an u32: '❌': invalid digit found in string
//!         at src/lib.rs:1234:56
//!         at src/lib.rs:1234:56"});
//! }
//! ```
//!
//! ### Example: Ad-Hoc Errors
//!
//! The [`err!`] macro allows you to format a string
//! and turn it into an ad-hoc [`Error`] at the same time:
//!
//! ```
//! #[cfg(feature = "std")]
//! use lazy_errors::prelude::*;
//!
//! #[cfg(not(feature = "std"))]
//! use lazy_errors::surrogate_error_trait::prelude::*;
//!
//! let pid = 42;
//! let err: Error = err!("Error in process {pid}");
//! ```
//!
//! You'll often find ad-hoc errors to be the leaves in an error tree.
//! However, the error tree can have almost any _inner error type_ as leaf.
//!
//! ### Supported Error Types
#![cfg_attr(
    feature = "std",
    doc = r##"

The [`prelude`] module exports commonly used traits and _aliased_ types.
Importing `lazy_errors::prelude::*` should set you up for most use-cases.
You may also want to import [`lazy_errors::Result`](crate::Result).
In `![no_std]` mode or when `core::error::Error` is not available,
you can import the [`surrogate_error_trait::prelude`] instead, and use
the corresponding [`lazy_errors::surrogate_error_trait::Result`].

 "##
)]
#![cfg_attr(
    not(feature = "std"),
    doc = r##"

The [`surrogate_error_trait::prelude`] module exports commonly used traits
and _aliased_ types.
Importing `lazy_errors::surrogate_error_trait::prelude::*` should set you up
for many use-cases.
You may also want to import [`lazy_errors::surrogate_error_trait::Result`].
Consider enabling the `std` feature
which makes the `lazy_errors::prelude::*` available.
Types exported from the “regular” prelude
have better intercompatibility with other crates.

 "##
)]
//! [`lazy_errors::surrogate_error_trait::Result`]:
//! crate::surrogate_error_trait::Result
//!
//! When you're using the aliased types from the prelude, this crate should
//! support any `Result<_, E>` if `E` implements `Into<Stashable>`.
//! [`Stashable`] is, basically, a `Box<dyn E>`, where `E` is either
//! `std::error::Error` or a surrogate trait in `#![no_std]` mode
//! ([`surrogate_error_trait::Reportable`]).
//! Thus, using the aliased types from the prelude, any error you put into
//! any of the containers defined by this crate will be boxed.
//! The `Into<Box<dyn E>>` trait bound was chosen because it is implemented
//! for a wide range of error types or _“error-like”_ types.
//! Some examples of types that satisfy this constraint are:
//!
//! - `&str`
//! - `String`
//! - `anyhow::Error`
//! - `eyre::Report`
//! - `std::error::Error`
//! - All error types from this crate
//!
//! The primary error type from this crate is [`Error`].
//! You can convert all supported _error-like_ types into [`Error`]
//! by calling [`or_wrap`] or [`or_wrap_with`].
//!
//! In other words, this crate supports a wide variety of error types.
//! However, in some cases you might need a different kind of flexibility
//! than that. For example, maybe you don't want to lose static error type
//! information or maybe your error types aren't [`Sync`].
//! In general, this crate should work well with any `Result<_, E>`
//! if `E` implements [`Into<I>`] where `I` is named the
//! [_inner error type_ of `Error`](Error#inner-error-type-i).
//! This crate will store errors as type `I` in its containers, for example
//! in [`ErrorStash`] or in [`Error`]. When you're using the type aliases
//! from the [`prelude`], `I` will always be [`Stashable`].
//! However, you do not need to use [`Stashable`] at all.
//! You can chose the type to use for `I` arbitrarily.
//! It can be a custom type and does not need to implement any traits
//! or auto traits except [`Sized`].
//! Thus, if the default aliases defined in the prelude
//! do not suit your purpose, you can import the required traits
//! and types manually and define custom aliases, as shown in the next example.
//!
//! ### Example: Custom Error Types
//!
//! Here's a complex example that does not use the [`prelude`]
//! but instead defines its own aliases. In the example, `Error<CustomError>`
//! and `ParserErrorStash` don't box their errors. Instead, they have all
//! error type information present statically, which allows you to write
//! recovery logic without having to rely on downcasts at run-time.
//! The example also shows how such custom error types
//! can still be used alongside the boxed error types ([`Stashable`])
//! with custom lifetimes.
//!
//! ```
//! # use std::str::FromStr;
//! use lazy_errors::{err, ErrorStash, OrStash, StashedResult};
//!
//! #[cfg(feature = "std")]
//! use lazy_errors::Stashable;
//!
//! #[cfg(not(feature = "std"))]
//! use lazy_errors::surrogate_error_trait::Stashable;
//!
//! #[derive(thiserror::Error, Debug)]
//! pub enum CustomError<'a>
//! {
//!     #[error("Input is empty")]
//!     EmptyInput,
//!
//!     #[error("Input '{0}' is not u32")]
//!     NotU32(&'a str),
//! }
//!
//! // Use `CustomError` as inner error type `I` for `ErrorStash`:
//! type ParserErrorStash<'a, F, M> = ErrorStash<F, M, CustomError<'a>>;
//!
//! // Allow using `CustomError` as `I` but use `Stashable` by default:
//! pub type Error<I = Stashable<'static>> = lazy_errors::Error<I>;
//!
//! fn main()
//! {
//!     let err = run(&["42", "0xA", "f", "oobar", "3b"]).unwrap_err();
//!     eprintln!("{err:#}");
//! }
//!
//! fn run<'a>(input: &[&'a str]) -> Result<(), Error<Stashable<'a>>>
//! {
//!     let mut errs = ErrorStash::new(|| "Application failed");
//!
//!     let parser_result = parse(input); // Soft errors
//!     if let Err(e) = parser_result {
//!         println!("There were errors.");
//!         println!("Errors will be returned after showing some suggestions.");
//!         let recovery_result = handle_parser_errors(&e); // Hard errors
//!         errs.push(e);
//!         if let Err(e) = recovery_result {
//!             errs.push(e);
//!             return errs.into();
//!         }
//!     }
//!
//!     // ... some related work, such as writing log files ...
//!
//!     errs.into()
//! }
//!
//! fn parse<'a>(input: &[&'a str]) -> Result<(), Error<CustomError<'a>>>
//! {
//!     if input.is_empty() {
//!         return Err(Error::wrap(CustomError::EmptyInput));
//!     }
//!
//!     let mut errs = ParserErrorStash::new(|| {
//!         "Input has correctable or uncorrectable errors"
//!     });
//!
//!     println!("Step #1: Starting...");
//!
//!     let mut parsed = vec![];
//!     for s in input {
//!         println!("Step #1: Trying to parse '{s}'");
//!         // Ignore “soft” errors for now...
//!         if let StashedResult::Ok(k) = parse_u32(s).or_stash(&mut errs) {
//!             parsed.push(k);
//!         }
//!     }
//!
//!     println!(
//!         "Step #1: Done. {} of {} inputs were u32 (decimal or hex): {:?}",
//!         parsed.len(),
//!         input.len(),
//!         parsed
//!     );
//!
//!     errs.into() // Return list of all parser errors, if any
//! }
//!
//! fn handle_parser_errors(errs: &Error<CustomError>) -> Result<(), Error>
//! {
//!     println!("Step #2: Starting...");
//!
//!     for e in errs.children() {
//!         match e {
//!             CustomError::NotU32(input) => guess_hex(input)?,
//!             other => return Err(err!("Internal error: {other}")),
//!         };
//!     }
//!
//!     println!("Step #2: Done");
//!
//!     Ok(())
//! }
//!
//! fn parse_u32(s: &str) -> Result<u32, CustomError>
//! {
//!     s.strip_prefix("0x")
//!         .map(|hex| u32::from_str_radix(hex, 16))
//!         .unwrap_or_else(|| u32::from_str(s))
//!         .map_err(|_| CustomError::NotU32(s))
//! }
//!
//! fn guess_hex(s: &str) -> Result<u32, Error>
//! {
//!     match u32::from_str_radix(s, 16) {
//!         Ok(v) => {
//!             println!("Step #2: '{s}' is not u32. Did you mean '{v:#X}'?");
//!             Ok(v)
//!         },
//!         Err(e) => {
//!             println!("Step #2: '{s}' is not u32. Aborting program.");
//!             Err(err!("Unsupported input '{s}': {e}"))
//!         },
//!     }
//! }
//! ```
//!
//! Running the example above will produce an output similar to this:
//!
//! ```text
//! stdout:
//! Step #1: Starting...
//! Step #1: Trying to parse '42'
//! Step #1: Trying to parse '0xA'
//! Step #1: Trying to parse 'f'
//! Step #1: Trying to parse 'oobar'
//! Step #1: Trying to parse '3b'
//! Step #1: Done. 2 of 5 inputs were u32 (decimal or hex): [42, 10]
//! There were errors.
//! Errors will be returned after showing some suggestions.
//! Step #2: Starting...
//! Step #2: 'f' is not u32. Did you mean '0xF'?
//! Step #2: 'oobar' is not u32. Aborting program.
//!
//! stderr:
//! Application failed
//! - Input has correctable or uncorrectable errors
//!   - Input 'f' is not u32
//!     at src/lib.rs:72:52
//!   - Input 'oobar' is not u32
//!     at src/lib.rs:72:52
//!   - Input '3b' is not u32
//!     at src/lib.rs:72:52
//!   at src/lib.rs:43:14
//! - Unsupported input 'oobar': invalid digit found in string
//!   at src/lib.rs:120:17
//!   at src/lib.rs:45:18
//! ```
//!
//! [`or_stash`]: crate::OrStash::or_stash
//! [`or_create_stash`]: crate::OrCreateStash::or_create_stash
//! [`or_wrap`]: crate::OrWrap::or_wrap
//! [`or_wrap_with`]: crate::OrWrapWith::or_wrap_with
#![cfg_attr(
    feature = "std",
    doc = r##"
[`prelude`]: crate::prelude
[`Stashable`]: crate::Stashable
"##
)]
#![cfg_attr(
    not(feature = "std"),
    doc = r##"
[`prelude`]: crate::surrogate_error_trait::prelude
[`Stashable`]: crate::surrogate_error_trait::Stashable
"##
)]

#[macro_use]
extern crate std;

#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
pub mod prelude;

pub mod surrogate_error_trait;

mod err;
mod error;
mod or_create_stash;
mod or_stash;
mod or_wrap;
mod or_wrap_with;
mod stash;
mod try2;

pub use error::{AdHocError, Error, ErrorData, StashedErrors, WrappedError};
pub use or_create_stash::OrCreateStash;
pub use or_stash::{OrStash, StashedResult};
pub use or_wrap::OrWrap;
pub use or_wrap_with::OrWrapWith;
pub use stash::{ErrorStash, StashWithErrors};
pub use surrogate_error_trait::Reportable;

#[cfg(feature = "eyre")]
mod into_eyre;
#[cfg(feature = "eyre")]
pub use into_eyre::{IntoEyreReport, IntoEyreResult};

/// Alias of the `Result<T, E>` we all know, but uses
/// [`prelude::Error`]
/// as default value for `E` if not specified explicitly.
#[cfg(feature = "std")]
pub type Result<T, E = prelude::Error> = core::result::Result<T, E>;

/// The “default” [_inner error type_ `I`](crate::Error#inner-error-type-i)
/// used by the type aliases from the
/// [crate::prelude]
/// _without_ `'static` lifetime.
///
/// The trait bounds `Send` and `Sync` are present because they are
/// required by some third-party crates. Without `Send` and `Sync`,
/// these crates may not be able to consume error types from this crate,
/// such as [`Error`].
/// Note that you can always simply use a custom inner error type.
/// For example, in your codebase you could define `Stashable` instead
/// as `Box<dyn std::error::Error + 'static>` and set an alias for
/// [`Error<I>`] accordingly.
///
/// [`Error`]: crate::error::Error
/// [`Error<I>`]: crate::error::Error#inner-error-type-i
#[cfg(feature = "std")]
pub type Stashable<'a> =
    alloc::boxed::Box<dyn std::error::Error + Send + Sync + 'a>;

/// ⚠️ Do not use this method! ⚠️
///
/// Replaces parts of the string that maybe are a line number
/// or maybe are a column number with static mock values.
/// Also sneakly changes `\` to `/` because this may be a path separator.
///
/// We just need this method to be able to use [`assert_eq`] in doctests.
/// This function may behave incorrectly in many cases.
/// It's also implemented inefficiently.
/// We just need this to be able to use [`assert_eq`] in doctests.
///
/// ⚠️ Do not use this method! ⚠️
#[doc(hidden)]
pub fn doctest_line_num_helper(text: &str) -> alloc::string::String
{
    // We need to call this method from the doctests.
    // Using a regex would require us to add the regex crate
    // as dependency in general.

    #[allow(clippy::useless_format)] // `use` would break MSRV
    let mut result = format!("{text}");
    loop {
        let result_before = result.clone();
        for i in 0..=9 {
            result = result.replace(&format!(".rs:{i}"), ".rs:");
            result = result.replace(&format!(".rs::{i}"), ".rs::");
        }

        if result == result_before {
            // Nothing to replace anymore.
            break;
        }
    }

    result
        .replace('\\', "/")
        .replace("at lazy_errors/src/", "at src/")
        .replace(".rs::", ".rs:1234:56")
}
