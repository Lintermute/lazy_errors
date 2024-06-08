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

//! Effortlessly create, group, and nest arbitrary errors,
//! and defer error handling ergonomically.
//!
//! ```
//! use lazy_errors::{prelude::*, Result};
//!
//! fn run() -> Result<()>
//! {
//!     let mut errs = ErrorStash::new(|| "Failed to run application");
//!
//!     write_if_ascii("42").or_stash(&mut errs); // `errs` contains 0 errors
//!     write_if_ascii("❌").or_stash(&mut errs); // `errs` contains 1 error
//!
//!     cleanup().or_stash(&mut errs); // Run cleanup even if there were errors,
//!                                    // but cleanup is allowed to fail as well
//!
//!     errs.into() // `Ok(())` if `errs` was still empty, `Err` otherwise
//! }
//! #
//! # fn write_if_ascii(text: &str) -> Result<()>
//! # {
//! #     if text.is_ascii() {
//! #         // ... write ...
//! #         Ok(())
//! #     } else {
//! #         Err(err!("Input is not ASCII: '{text}'"))
//! #     }
//! # }
//! #
//! # fn cleanup() -> Result<()>
//! # {
//! #     Err(err!("Cleanup failed"))
//! # }
//!
//! let errs = run().unwrap_err();
//! assert_eq!(errs.childs().len(), 2);
//! ```
//!
//! # In a Nutshell
//!
//! `lazy_errors` provides types, traits, and blanket implementations
//! on `Result` that can be used to ergonomically defer error handling.
//! Additionally, `lazy_errors` allows you to easily create ad-hoc errors
//! as well as wrap, group, and nest a wide variety of errors
//! in a single common error type, simplifying your codebase.
//! In that latter regard, `lazy_errors` is similar to `anyhow`/`eyre`,
//! except that its reporting isn't as fancy or detailed (for example,
//! `lazy_errors` tracks source code file name and line numbers instead of
//! providing full `std::backtrace` support).
//! On the other hand, `lazy_errors` uses `#![no_std]` by default but
//! integrates with `std::error::Error` if you enable the `std` feature.
//! `lazy_errors` also supports error types that aren't `Send` or `Sync`
//! and allows you to group and nest errors arbitrarily with minimal effort.
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
//! # Walkthrough
//!
//! `lazy_errors` actually supports any error type as long as it's `Sized`;
//! it doesn't even need to be `Send` or `Sync`. You only need to specify
//! the generic type parameters accordingly, as shown in the example
//! on the bottom of this page. Usually however, you'd want to use the
//! aliased types from the [`prelude`]. When you're using these aliases,
//! errors will be boxed and you can dynamically return groups of errors
//! of differing types from the same function.
//! In the default `#![no_std]` mode, `lazy_errors` can box any error type
//! that implements the [`Reportable`] marker trait; if necessary,
//! you can implement that trait in a single line for your custom types.
//! If you need to handle third-party error types that already implement
//! `std::error::Error` instead, you can enable the `std` feature.
//! When `std` is enabled, all error types from this crate will
//! implement `std::error::Error` as well.
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
//! use lazy_errors::prelude::*;
//!
//! fn main()
//! {
//!     let err = run().unwrap_err();
//!     let printed = format!("{err:#}");
//!     let printed = lazy_errors::replace_line_numbers(&printed);
//!     assert_eq!(printed, indoc::indoc! {"
//!         Failed to run application
//!         - Input is not ASCII: '🙈'
//!           at lazy_errors/src/lib.rs:1234:56
//!           at lazy_errors/src/lib.rs:1234:56
//!         - Input is not ASCII: '🙉'
//!           at lazy_errors/src/lib.rs:1234:56
//!           at lazy_errors/src/lib.rs:1234:56
//!         - Input is not ASCII: '🙊'
//!           at lazy_errors/src/lib.rs:1234:56
//!           at lazy_errors/src/lib.rs:1234:56
//!         - Cleanup failed
//!           at lazy_errors/src/lib.rs:1234:56
//!           at lazy_errors/src/lib.rs:1234:56"});
//! }
//!
//! fn run() -> Result<(), Error>
//! {
//!     let mut stash = ErrorStash::new(|| "Failed to run application");
//!
//!     print_if_ascii("🙈").or_stash(&mut stash);
//!     print_if_ascii("🙉").or_stash(&mut stash);
//!     print_if_ascii("🙊").or_stash(&mut stash);
//!     print_if_ascii("42").or_stash(&mut stash);
//!
//!     cleanup().or_stash(&mut stash); // Runs regardless of earlier errors
//!
//!     stash.into() // `Ok(())` if the stash was still empty
//! }
//!
//! fn print_if_ascii(text: &str) -> Result<(), Error>
//! {
//!     if !text.is_ascii() {
//!         return Err(err!("Input is not ASCII: '{text}'"));
//!     }
//!
//!     println!("{text}");
//!     Ok(())
//! }
//!
//! fn cleanup() -> Result<(), Error>
//! {
//!     Err(err!("Cleanup failed"))
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
//! [`ErrorStash`] and [`StashWithErrors`] behave quite similarly.
//! While both [`ErrorStash`] and [`StashWithErrors`] can take additional
//! errors, a [`StashWithErrors`] is guaranteed to be non-empty.
//! The type system will be aware that there is at least one error.
//! Thus, while [`ErrorStash`] can only be converted into [`Result`],
//! yielding either `Ok(())` or `Err(e)` (where `e` is [`Error`]),
//! this distinction allows converting [`StashWithErrors`] into [`Error`]
//! directly.
//!
//! ```
//! use lazy_errors::prelude::*;
//!
//! fn main()
//! {
//!     let err = run().unwrap_err();
//!     let printed = format!("{err:#}");
//!     let printed = lazy_errors::replace_line_numbers(&printed);
//!     assert_eq!(printed, indoc::indoc! {"
//!         Failed to run application
//!         - Input is not ASCII: '❌'
//!           at lazy_errors/src/lib.rs:1234:56
//!           at lazy_errors/src/lib.rs:1234:56
//!         - Cleanup failed
//!           at lazy_errors/src/lib.rs:1234:56
//!           at lazy_errors/src/lib.rs:1234:56"});
//! }
//!
//! fn run() -> Result<(), Error>
//! {
//!     match write("❌").or_create_stash(|| "Failed to run application") {
//!         Ok(()) => Ok(()),
//!         Err(mut stash) => {
//!             cleanup().or_stash(&mut stash);
//!             return Err(stash.into());
//!         },
//!     }
//! }
//!
//! fn write(text: &str) -> Result<(), Error>
//! {
//!     if !text.is_ascii() {
//!         return Err(err!("Input is not ASCII: '{text}'"));
//!     }
//!     Ok(())
//! }
//!
//! fn cleanup() -> Result<(), Error>
//! {
//!     Err(err!("Cleanup failed"))
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
    doc = r##"
[`into_eyre_result`](IntoEyreResult::into_eyre_result) and
[`into_eyre_report`](IntoEyreReport::into_eyre_report):

```
# use color_eyre::eyre;
use lazy_errors::prelude::*;
use eyre::bail;

fn main()
{
    let err = run().unwrap_err();
    let printed = format!("{err:#}");
    let printed = lazy_errors::replace_line_numbers(&printed);
    assert_eq!(printed, indoc::indoc! {"
        Failed to run
        - Input is not ASCII: '❌'
          at lazy_errors/src/lib.rs:1234:56
          at lazy_errors/src/lib.rs:1234:56
        - Cleanup failed
          at lazy_errors/src/lib.rs:1234:56
          at lazy_errors/src/lib.rs:1234:56"});
}

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
```
"##
)]
//! ### Example: Hierarchies
//!
//! As you might have noticed, [`Error`]s form hierarchies:
//!
//! ```
//! use lazy_errors::prelude::*;
//!
//! fn main()
//! {
//!     let err = first().unwrap_err();
//!     let printed = format!("{err:#}");
//!     let printed = lazy_errors::replace_line_numbers(&printed);
//!     assert_eq!(printed, indoc::indoc! {"
//!         In first(): second() failed
//!         - In second(): third() failed
//!           - In third(): There were errors
//!             - First error
//!               at lazy_errors/src/lib.rs:1234:56
//!             - Second error
//!               at lazy_errors/src/lib.rs:1234:56
//!             at lazy_errors/src/lib.rs:1234:56
//!           at lazy_errors/src/lib.rs:1234:56"});
//! }
//!
//! fn first() -> Result<(), Error>
//! {
//!     let mut stash = ErrorStash::new(|| "In first(): second() failed");
//!     stash.push(second().unwrap_err());
//!     stash.into()
//! }
//!
//! fn second() -> Result<(), Error>
//! {
//!     let mut stash = ErrorStash::new(|| "In second(): third() failed");
//!     stash.push(third().unwrap_err());
//!     stash.into()
//! }
//!
//! fn third() -> Result<(), Error>
//! {
//!     let mut stash = ErrorStash::new(|| "In third(): There were errors");
//!
//!     stash.push("First error");
//!     stash.push("Second error");
//!
//!     stash.into()
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
//! use lazy_errors::{prelude::*, Result};
//!
//! fn main()
//! {
//!     let err = first().unwrap_err();
//!     let printed = format!("{err:#}");
//!     let printed = lazy_errors::replace_line_numbers(&printed);
//!     assert_eq!(printed, indoc::indoc! {"
//!         Something went wrong: In third(): There were errors
//!         - First error
//!           at lazy_errors/src/lib.rs:1234:56
//!         - Second error
//!           at lazy_errors/src/lib.rs:1234:56
//!         at lazy_errors/src/lib.rs:1234:56
//!         at lazy_errors/src/lib.rs:1234:56"});
//! }
//!
//! fn first() -> Result<(), Error>
//! {
//!     second().or_wrap_with(|| "Something went wrong")
//! }
//!
//! fn second() -> Result<()>
//! {
//!     third().or_wrap() // Wrap it “silently”: No message, just file location
//! }
//!
//! fn third() -> Result<()>
//! {
//!     let mut stash = ErrorStash::new(|| "In third(): There were errors");
//!
//!     stash.push("First error");
//!     stash.push("Second error");
//!
//!     stash.into()
//! }
//! ```
//!
//! ### Example: Ad-Hoc Errors
//!
//! The [`err!`] macro allows you to format a string
//! and turn it into an ad-hoc [`Error`] at the same time:
//!
//! ```
//! use lazy_errors::{prelude::*, Result};
//!
//! let pid = 42;
//! let err: Error = err!("Error in process {pid}");
//! ```
//!
//! You'll often find ad-hoc errors to be the leaves in an error tree.
//! However, the error tree can have almost any _inner error type_ as leaf.
//!
//! ### Supported Error Types
//!
//! The [`prelude`] module exports commonly used traits and _aliased_ types.
//! Importing `prelude::*` should set you up for most use-cases.
//! You may also want to import [`lazy_errors::Result`](crate::Result).
//! When you're using the aliased types from the prelude, this crate should
//! support any `Result<_, E>` if `E` implements `Into<`[`Stashable`]`>`.
//! [`Stashable`] is, basically, a `Box<dyn E>`, where `E` is either
//! `std::error::Error` or a similar trait in `#![no_std]` mode.
//! Thus, using the aliased types from the prelude, any error you put into
//! any of the containers defined by this crate will be boxed.
//! The `Into<Box<dyn E>>` trait bound was chosen because it is implemented
//! for a wide range of error types or _“error-like”_ types.
//! Some examples of types that satisfy this constraint are:
//!
//! - [`&str`]
//! - [`String`]
//! - `eyre::Report`
//! - `anyhow::Error`
//! - [`std::error::Error`]
//! - All error types from this crate
//!
//! The primary error type from this crate is [`Error`].
//! You can convert all supported _error-like_ types into [`Error`]
//! by calling [`or_wrap`] or [`or_wrap_with`]:
//!
//! ```
//! use lazy_errors::prelude::*;
//!
//! fn main()
//! {
//!     let err = parent().unwrap_err();
//!     let printed = format!("{err:#}");
//!     let printed = lazy_errors::replace_line_numbers(&printed);
//!     assert_eq!(printed, indoc::indoc! {"
//!         In parent(): child() failed: Arbitrary String
//!         at lazy_errors/src/lib.rs:1234:56"});
//! }
//!
//! fn parent() -> Result<(), Error>
//! {
//!     child().or_wrap_with(|| "In parent(): child() failed")
//! }
//!
//! fn child() -> Result<(), String>
//! {
//!     Err(String::from("Arbitrary String"))
//! }
//! ```
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
//! The concrete type to use for `I` may be chosen by the user arbitrarily.
//! It can be a custom type and does not need to implement any traits
//! or auto traits except [`Sized`].
//! Thus, if the default aliases defined in the prelude
//! do not suit your purpose, you can import the required traits
//! and types manually and define custom aliases, as shown below.
//!
//! ### Example: Custom Error Types
//!
//! Here's a complex example that does not use the [`prelude`]
//! but instead defines its own aliases.
//! These error types have their static type information still present,
//! enabling running recovery logic without having to rely on downcasts
//! at run-time. The example also shows how such custom error types
//! can still be used alongside the boxed error types ([`Stashable`]s)
//! with custom lifetimes.
//!
//! ```
//! use std::str::FromStr;
//!
//! use lazy_errors::{
//!     err,
//!     Error,
//!     ErrorStash,
//!     OrStash,
//!     Result,
//!     Stashable,
//!     StashedResult,
//! };
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
//! // Use `CustomError` as `I` for both `Error` and `ErrorStash`:
//! type ParserError<'a> = Error<CustomError<'a>>;
//! type ParserStash<'a, F, M> = ErrorStash<F, M, CustomError<'a>>;
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
//!     let parser_result = parse_input(input); // Soft errors
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
//! fn parse_input<'a>(input: &[&'a str]) -> Result<(), ParserError<'a>>
//! {
//!     if input.is_empty() {
//!         return Err(Error::wrap(CustomError::EmptyInput));
//!     }
//!
//!     let mut errs = ParserStash::new(|| {
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
//! fn handle_parser_errors(errs: &ParserError) -> Result<()>
//! {
//!     println!("Step #2: Starting...");
//!
//!     for e in errs.childs() {
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
//! fn guess_hex(s: &str) -> Result<u32>
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
//!     at lazy_errors/src/lib.rs:72:52
//!   - Input 'oobar' is not u32
//!     at lazy_errors/src/lib.rs:72:52
//!   - Input '3b' is not u32
//!     at lazy_errors/src/lib.rs:72:52
//!   at lazy_errors/src/lib.rs:43:14
//! - Unsupported input 'oobar': invalid digit found in string
//!   at lazy_errors/src/lib.rs:120:17
//!   at lazy_errors/src/lib.rs:45:18
//! ```
//!
//! [`or_stash`]: crate::OrStash::or_stash
//! [`or_create_stash`]: crate::OrCreateStash::or_create_stash
//! [`or_wrap`]: crate::OrWrap::or_wrap
//! [`or_wrap_with`]: crate::OrWrapWith::or_wrap_with

#![cfg_attr(not(feature = "std"), no_std)]
#[macro_use]
extern crate std;

#[macro_use]
extern crate alloc;

pub mod boxed;
pub mod prelude;

mod err;
mod error;
mod eyre;
mod or_create_stash;
mod or_stash;
mod or_wrap;
mod or_wrap_with;
mod reportable;
mod stash;
mod stashable;
mod try2;

pub use error::{AdHocError, Error, ErrorData, StashedErrors, WrappedError};
#[cfg(feature = "eyre")]
pub use eyre::{IntoEyreReport, IntoEyreResult};
pub use or_create_stash::OrCreateStash;
pub use or_stash::{OrStash, StashedResult};
pub use or_wrap::OrWrap;
pub use or_wrap_with::OrWrapWith;
#[cfg(any(not(feature = "std"), doc))]
pub use reportable::Reportable;
pub use stash::{ErrorStash, StashWithErrors};
pub use stashable::Stashable;

/// Like the `Result<T, E>` we all know, but uses [`prelude::Error`]
/// as default value for `E` if not present.
pub type Result<T, E = prelude::Error> = core::result::Result<T, E>;

use alloc::string::String;

/// Do not use this method.
/// We just need this to be able to use [`assert_eq`] in doctests.
///
/// Replaces parts of the string that maybe are a line number
/// or maybe are a column number with static mock values.
/// Also sneakly changes `\` to `/` because this may be a path separator.
///
/// This function may behave incorrectly in many cases.
/// It's also implemented inefficiently.
/// We just need this to be able to use [`assert_eq`] in doctests.
/// Do not use this method.
#[doc(hidden)]
pub fn replace_line_numbers(text: &str) -> String
{
    use alloc::{format, string::ToString};

    // We need to call this method from the doctests.
    // Using a regex would require us to add the regex crate
    // as dependency in general.
    let mut result = text.to_string();
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

    result = result.replace(".rs::", ".rs:1234:56");

    result.replace('\\', "/")
}
