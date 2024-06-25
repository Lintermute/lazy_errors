# lazy_errors ![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue) [![lazy_errors on crates.io](https://img.shields.io/crates/v/lazy_errors)](https://crates.io/crates/lazy_errors) [![lazy_errors on docs.rs](https://docs.rs/lazy_errors/badge.svg)](https://docs.rs/lazy_errors) [![Source Code Repository](https://img.shields.io/badge/Code-On%20GitHub-blue?logo=GitHub)](https://github.com/Lintermute/lazy_errors)

Effortlessly create, group, and nest arbitrary errors,
and defer error handling ergonomically.

```rust
#[cfg(feature = "std")]
use lazy_errors::{prelude::*, Result};

#[cfg(not(feature = "std"))]
use lazy_errors::surrogate_error_trait::{prelude::*, Result};

fn run(input1: &str, input2: &str) -> Result<()>
{
    let mut errs = ErrorStash::new(|| "There were one or more errors");

    u8::from_str("42").or_stash(&mut errs); // `errs` contains 0 errors
    u8::from_str("❌").or_stash(&mut errs); // `errs` contains 1 error
    u8::from_str("1337").or_stash(&mut errs); // `errs` contains 2 errors

    // `input1` is very important in this example,
    // so make sure it has a nice message.
    let r: Result<u8> = u8::from_str(input1)
        .or_wrap_with(|| format!("Input '{input1}' is invalid"));

    // If `input1` is invalid, we don't want to continue
    // but return _all_ errors that have occurred so far.
    let input1: u8 = try2!(r.or_stash(&mut errs));
    println!("input1 = {input1:#X}");

    // Continue handling other `Result`s.
    u8::from_str(input2).or_stash(&mut errs);

    errs.into() // `Ok(())` if `errs` is still empty, `Err` otherwise
}

fn main()
{
    let err = run("❓", "❗").unwrap_err();
    let n = err.children().len();
    eprintln!("Got an error with {n} children.");
    eprintln!("---------------------------------------------------------");
    eprintln!("{err:#}");
}
```

Running the example will print:

```text
Got an error with 3 children.
---------------------------------------------------------
There were one or more errors
- invalid digit found in string
  at src/main.rs:10:24
- number too large to fit in target type
  at src/main.rs:11:26
- Input '❓' is invalid: invalid digit found in string
  at src/main.rs:16:10
  at src/main.rs:20:30
```

## In a Nutshell

`lazy_errors` provides types, traits, and blanket implementations
on `Result` that can be used to ergonomically defer error handling.
`lazy_errors` allows you to easily create ad-hoc errors
as well as wrap a wide variety of errors in a single common error type,
simplifying your codebase.
In that latter regard, it is similar to `anyhow`/`eyre`,
except that its reporting isn’t as fancy or detailed (for example,
`lazy_errors` tracks source code file name and line numbers instead of
providing full `std::backtrace` support).
On the other hand, `lazy_errors` adds methods to `Result`
that let you continue on failure,
deferring returning `Err` results.
`lazy_errors` allows you to return two or more errors
from functions simultaneously and ergonomically.
`lazy_error` also supports nested errors.
When you return nested errors from functions,
errors will form a tree while “bubbling up”.
You can report that error tree the user/developer in its entirety.

By default, `lazy_errors` will box your error values (like `anyhow`/`eyre`),
which allows you to use different error types in the same `Result` type.
However, `lazy_errors` will respect static error type information
if you provide it explicitly.
If you do so, you can access fields and methods of your error values
at run-time without needing downcasts.
Both modes of operation can work together, as will be shown
in the example on the bottom of the page.

While `lazy_error` integrates with `std::error::Error` by default,
it also supports `#![no_std]` if you disable the `std` feature.
When you define a few simple type aliases,
`lazy_errors` easily supports error types that aren’t
`Sync` or even `Send`.

Common reasons to use this crate are:

* You want to return an error but run some fallible cleanup logic before.
* More generally, you’re calling two or more functions that return `Result`,
  and want to return an error that wraps all errors that occurred.
* You’re spawning several parallel activities, wait for their completion,
  and want to return all errors that occurred.
* You want to aggregate multiple errors before running some reporting or
  recovery logic, iterating over all errors collected.
* You need to handle errors that don’t implement
  `std::error::Error`/`Display`/`Debug`/`Send`/`Sync` or other common
  traits.

## Feature Flags

* `std`:
  * Support error types that implement `std::error::Error`.
  * Implement `std::error::Error` for `lazy_error` error types.
* `eyre`: Adds `into_eyre_result` and `into_eyre_report` conversions.
* `rust-vN` (where `N` is a Rust version number): Does nothing more than add
  support for some error types from `core` and `alloc` that were stabilized
  in the respective Rust version.

## MSRV

The MSRV of `lazy_errors` depends on the set of enabled features:

* Rust 1.77 supports all features and combinations thereof.
* Rust versions 1.61 .. 1.77 need you to disable all `rust-vN` features
  where `N` is greater than the version of your Rust toolchain. For example,
  to compile `lazy_errors` on Rust 1.66, you have to disable `rust-v1.77`
  and `rust-v1.69`, but not `rust-v1.66`.
* `eyre` needs at least Rust 1.65.
* Rust versions older than 1.61 are unsupported.

## Walkthrough

`lazy_errors` can actually support any error type as long as it’s `Sized`;
it doesn’t even need to be `Send` or `Sync`. You only need to specify
the generic type parameters accordingly, as will be shown in the example
on the bottom of this page. Usually however, you’d want to use the
aliased types from the [`prelude`][__link0]. When you’re using these aliases,
errors will be boxed and you can dynamically return groups of errors
of differing types from the same function.

The `std` feature is enabled by default, making `lazy_error` support
third-party error types that implement `std::error::Error`.
All error types from this crate will implement `std::error::Error` as well
in that case.
If you need `#![no_std]` support, you can disable the `std` feature
and use the [`surrogate_error_trait::prelude`][__link1] instead.
If you do so, `lazy_errors` will box any error type that implements the
[`surrogate_error_trait::Reportable`][__link2] marker trait.
If necessary, you can implement that trait for your custom types as well
(it’s just a single line).

While `lazy_errors` works standalone, it’s not intended to replace
`anyhow` or `eyre`. Instead, this project was started to explore
approaches on how to run multiple fallible operations, aggregate
their errors (if any), and defer the actual error handling/reporting
by returning all of these errors from functions that return `Result`.
Generally, `Result<_, Vec<_>>` can be used for this purpose,
which is not much different from what `lazy_errors` does internally.
However, `lazy_errors` provides “syntactic sugar”
to make this approach more ergonomic.
Thus, arguably the most useful method in this crate is [`or_stash`][__link3].

#### Example: `or_stash`

[`or_stash`][__link4] is arguably the most useful method of this crate.
It becomes available on `Result` as soon as you
import the [`OrStash`][__link5] trait or the [`prelude`][__link6].
Here’s an example:

```rust
#[cfg(feature = "std")]
use lazy_errors::{prelude::*, Result};

#[cfg(not(feature = "std"))]
use lazy_errors::surrogate_error_trait::{prelude::*, Result};

fn run() -> Result<()>
{
    let mut stash = ErrorStash::new(|| "Failed to run application");

    print_if_ascii("❓").or_stash(&mut stash);
    print_if_ascii("❗").or_stash(&mut stash);
    print_if_ascii("42").or_stash(&mut stash);

    cleanup().or_stash(&mut stash); // Runs regardless of earlier errors

    stash.into() // `Ok(())` if the stash was still empty
}

fn print_if_ascii(text: &str) -> Result<()>
{
    if !text.is_ascii() {
        return Err(err!("Input is not ASCII: '{text}'"));
    }

    println!("{text}");
    Ok(())
}

fn cleanup() -> Result<()>
{
    Err(err!("Cleanup failed"))
}

fn main()
{
    let err = run().unwrap_err();
    let printed = format!("{err:#}");
    let printed = replace_line_numbers(&printed);
    assert_eq!(printed, indoc::indoc! {"
        Failed to run application
        - Input is not ASCII: '❓'
          at src/lib.rs:1234:56
          at src/lib.rs:1234:56
        - Input is not ASCII: '❗'
          at src/lib.rs:1234:56
          at src/lib.rs:1234:56
        - Cleanup failed
          at src/lib.rs:1234:56
          at src/lib.rs:1234:56"});
}
```

In the example above, `run()` will print `42`, run `cleanup()`,
and then return the stashed errors.

Note that the [`ErrorStash`][__link7] is created manually in the example above.
The [`ErrorStash`][__link8] is empty before the first error is added.
Converting an empty [`ErrorStash`][__link9] to [`Result`][__link10] will produce `Ok(())`.
When [`or_stash`][__link11] is called on `Result::Err(e)`,
`e` will be moved into the [`ErrorStash`][__link12]. As soon as there is
at least one error stored in the [`ErrorStash`][__link13], converting [`ErrorStash`][__link14]
into [`Result`][__link15] will yield a `Result::Err` that contains an [`Error`][__link16],
the main error type from this crate.

#### Example: `or_create_stash`

Sometimes you don’t want to create an empty [`ErrorStash`][__link17] beforehand.
In that case you can call [`or_create_stash`][__link18] on `Result`
to create a non-empty container on-demand, whenever necessary.
When [`or_create_stash`][__link19] is called on `Result::Err`, the error
will be put into a [`StashWithErrors`][__link20] instead of an [`ErrorStash`][__link21].
[`ErrorStash`][__link22] and [`StashWithErrors`][__link23] behave similarly.
While both [`ErrorStash`][__link24] and [`StashWithErrors`][__link25] can take additional
errors, a [`StashWithErrors`][__link26] is guaranteed to be non-empty.
The type system will be aware that there is at least one error.
Thus, while [`ErrorStash`][__link27] can only be converted into [`Result`][__link28],
yielding either `Ok(())` or `Err(e)` (where `e` is [`Error`][__link29]),
this distinction allows converting [`StashWithErrors`][__link30] into [`Error`][__link31]
directly.

```rust
#[cfg(feature = "std")]
use lazy_errors::{prelude::*, Result};

#[cfg(not(feature = "std"))]
use lazy_errors::surrogate_error_trait::{prelude::*, Result};

fn run() -> Result<()>
{
    match write("❌").or_create_stash(|| "Failed to run application") {
        Ok(()) => Ok(()),
        Err(mut stash) => {
            cleanup().or_stash(&mut stash);
            Err(stash.into())
        },
    }
}

fn write(text: &str) -> Result<()>
{
    if !text.is_ascii() {
        return Err(err!("Input is not ASCII: '{text}'"));
    }
    Ok(())
}

fn cleanup() -> Result<()>
{
    Err(err!("Cleanup failed"))
}

fn main()
{
    let err = run().unwrap_err();
    let printed = format!("{err:#}");
    let printed = replace_line_numbers(&printed);
    assert_eq!(printed, indoc::indoc! {"
        Failed to run application
        - Input is not ASCII: '❌'
          at src/lib.rs:1234:56
          at src/lib.rs:1234:56
        - Cleanup failed
          at src/lib.rs:1234:56
          at src/lib.rs:1234:56"});
}
```

#### Example: `into_eyre_*`

[`ErrorStash`][__link32] and [`StashWithErrors`][__link33] can be converted into
[`Result`][__link34] and [`Error`][__link35], respectively. A similar, albeit lossy,
conversion from [`ErrorStash`][__link36] and [`StashWithErrors`][__link37] exist for
`eyre::Result` and `eyre::Error` (i.e. `eyre::Report`), namely
[`into_eyre_result`][__link38] and
[`into_eyre_report`][__link39]:

```rust
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

#### Example: Hierarchies

As you might have noticed, [`Error`][__link40]s form hierarchies:

```rust
#[cfg(feature = "std")]
use lazy_errors::{prelude::*, Result};

#[cfg(not(feature = "std"))]
use lazy_errors::surrogate_error_trait::{prelude::*, Result};

fn parent() -> Result<()>
{
    let mut stash = ErrorStash::new(|| "In parent(): child() failed");
    stash.push(child().unwrap_err());
    stash.into()
}

fn child() -> Result<()>
{
    let mut stash = ErrorStash::new(|| "In child(): There were errors");
    stash.push("First error");
    stash.push("Second error");
    stash.into()
}

fn main()
{
    let err = parent().unwrap_err();
    let printed = format!("{err:#}");
    let printed = replace_line_numbers(&printed);
    assert_eq!(printed, indoc::indoc! {"
        In parent(): child() failed
        - In child(): There were errors
          - First error
            at src/lib.rs:1234:56
          - Second error
            at src/lib.rs:1234:56
          at src/lib.rs:1234:56"});
}
```

The example above may seem unwieldy. In fact, that example only serves
the purpose to illustrate the error hierarchy.
In practice, you wouldn’t write such code.
Instead, you’d probably rely on [`or_wrap`][__link41] or [`or_wrap_with`][__link42].

#### Example: Wrapping

You can use [`or_wrap`][__link43] or [`or_wrap_with`][__link44] to wrap any value
that can be converted into the
[*inner error type* of `Error`][__link45]
or to attach some context to an error:

```rust
#[cfg(feature = "std")]
use lazy_errors::{prelude::*, Result};

#[cfg(not(feature = "std"))]
use lazy_errors::surrogate_error_trait::{prelude::*, Result};

fn run(s: &str) -> Result<u32>
{
    parse(s).or_wrap_with(|| format!("Not an u32: '{s}'"))
}

fn parse(s: &str) -> Result<u32>
{
    let r: Result<u32, core::num::ParseIntError> = s.parse();

    // Wrap the error type “silently”:
    // No additional message, just file location and wrapped error type.
    r.or_wrap()
}

fn main()
{
    let err = run("❌").unwrap_err();
    let printed = format!("{err:#}");
    let printed = replace_line_numbers(&printed);
    assert_eq!(printed, indoc::indoc! {"
        Not an u32: '❌': invalid digit found in string
        at src/lib.rs:1234:56
        at src/lib.rs:1234:56"});
}
```

#### Example: Ad-Hoc Errors

The [`err!`][__link46] macro allows you to format a string
and turn it into an ad-hoc [`Error`][__link47] at the same time:

```rust
#[cfg(feature = "std")]
use lazy_errors::prelude::*;

#[cfg(not(feature = "std"))]
use lazy_errors::surrogate_error_trait::prelude::*;

let pid = 42;
let err: Error = err!("Error in process {pid}");
```

You’ll often find ad-hoc errors to be the leaves in an error tree.
However, the error tree can have almost any *inner error type* as leaf.

#### Supported Error Types

The [`prelude`][__link48] module exports commonly used traits and *aliased* types.
Importing `lazy_errors::prelude::*` should set you up for most use-cases.
You may also want to import [`lazy_errors::Result`][__link49].
In `![no_std]` mode or when `core::error::Error` is not available,
you can import the [`surrogate_error_trait::prelude`][__link50] instead, and use
the corresponding [`lazy_errors::surrogate_error_trait::Result`][__link51].

When you’re using the aliased types from the prelude, this crate should
support any `Result<_, E>` if `E` implements `Into<Stashable>`.
[`Stashable`][__link52] is, basically, a `Box<dyn E>`, where `E` is either
`std::error::Error` or a surrogate trait in `#![no_std]` mode
([`surrogate_error_trait::Reportable`][__link53]).
Thus, using the aliased types from the prelude, any error you put into
any of the containers defined by this crate will be boxed.
The `Into<Box<dyn E>>` trait bound was chosen because it is implemented
for a wide range of error types or *“error-like”* types.
Some examples of types that satisfy this constraint are:

* `&str`
* `String`
* `anyhow::Error`
* `eyre::Report`
* `std::error::Error`
* All error types from this crate

The primary error type from this crate is [`Error`][__link54].
You can convert all supported *error-like* types into [`Error`][__link55]
by calling [`or_wrap`][__link56] or [`or_wrap_with`][__link57].

In other words, this crate supports a wide variety of error types.
However, in some cases you might need a different kind of flexibility
than that. For example, maybe you don’t want to lose static error type
information or maybe your error types aren’t [`Sync`][__link58].
In general, this crate should work well with any `Result<_, E>`
if `E` implements [`Into<I>`][__link59] where `I` is named the
[*inner error type* of `Error`][__link60].
This crate will store errors as type `I` in its containers, for example
in [`ErrorStash`][__link61] or in [`Error`][__link62]. When you’re using the type aliases
from the [`prelude`][__link63], `I` will always be [`Stashable`][__link64].
However, you do not need to use [`Stashable`][__link65] at all.
You can chose the type to use for `I` arbitrarily.
It can be a custom type and does not need to implement any traits
or auto traits except [`Sized`][__link66].
Thus, if the default aliases defined in the prelude
do not suit your purpose, you can import the required traits
and types manually and define custom aliases, as shown in the next example.

#### Example: Custom Error Types

Here’s a complex example that does not use the [`prelude`][__link67]
but instead defines its own aliases. In the example, `Error<CustomError>`
and `ParserErrorStash` don’t box their errors. Instead, they have all
error type information present statically, which allows you to write
recovery logic without having to rely on downcasts at run-time.
The example also shows how such custom error types
can still be used alongside the boxed error types ([`Stashable`][__link68])
with custom lifetimes.

```rust
use lazy_errors::{err, ErrorStash, OrStash, StashedResult};

#[cfg(feature = "std")]
use lazy_errors::Stashable;

#[cfg(not(feature = "std"))]
use lazy_errors::surrogate_error_trait::Stashable;

#[derive(thiserror::Error, Debug)]
pub enum CustomError<'a>
{
    #[error("Input is empty")]
    EmptyInput,

    #[error("Input '{0}' is not u32")]
    NotU32(&'a str),
}

// Use `CustomError` as inner error type `I` for `ErrorStash`:
type ParserErrorStash<'a, F, M> = ErrorStash<F, M, CustomError<'a>>;

// Allow using `CustomError` as `I` but use `Stashable` by default:
pub type Error<I = Stashable<'static>> = lazy_errors::Error<I>;

fn main()
{
    let err = run(&["42", "0xA", "f", "oobar", "3b"]).unwrap_err();
    eprintln!("{err:#}");
}

fn run<'a>(input: &[&'a str]) -> Result<(), Error<Stashable<'a>>>
{
    let mut errs = ErrorStash::new(|| "Application failed");

    let parser_result = parse(input); // Soft errors
    if let Err(e) = parser_result {
        println!("There were errors.");
        println!("Errors will be returned after showing some suggestions.");
        let recovery_result = handle_parser_errors(&e); // Hard errors
        errs.push(e);
        if let Err(e) = recovery_result {
            errs.push(e);
            return errs.into();
        }
    }

    // ... some related work, such as writing log files ...

    errs.into()
}

fn parse<'a>(input: &[&'a str]) -> Result<(), Error<CustomError<'a>>>
{
    if input.is_empty() {
        return Err(Error::wrap(CustomError::EmptyInput));
    }

    let mut errs = ParserErrorStash::new(|| {
        "Input has correctable or uncorrectable errors"
    });

    println!("Step #1: Starting...");

    let mut parsed = vec![];
    for s in input {
        println!("Step #1: Trying to parse '{s}'");
        // Ignore “soft” errors for now...
        if let StashedResult::Ok(k) = parse_u32(s).or_stash(&mut errs) {
            parsed.push(k);
        }
    }

    println!(
        "Step #1: Done. {} of {} inputs were u32 (decimal or hex): {:?}",
        parsed.len(),
        input.len(),
        parsed
    );

    errs.into() // Return list of all parser errors, if any
}

fn handle_parser_errors(errs: &Error<CustomError>) -> Result<(), Error>
{
    println!("Step #2: Starting...");

    for e in errs.children() {
        match e {
            CustomError::NotU32(input) => guess_hex(input)?,
            other => return Err(err!("Internal error: {other}")),
        };
    }

    println!("Step #2: Done");

    Ok(())
}

fn parse_u32(s: &str) -> Result<u32, CustomError>
{
    s.strip_prefix("0x")
        .map(|hex| u32::from_str_radix(hex, 16))
        .unwrap_or_else(|| u32::from_str(s))
        .map_err(|_| CustomError::NotU32(s))
}

fn guess_hex(s: &str) -> Result<u32, Error>
{
    match u32::from_str_radix(s, 16) {
        Ok(v) => {
            println!("Step #2: '{s}' is not u32. Did you mean '{v:#X}'?");
            Ok(v)
        },
        Err(e) => {
            println!("Step #2: '{s}' is not u32. Aborting program.");
            Err(err!("Unsupported input '{s}': {e}"))
        },
    }
}
```

Running the example above will produce an output similar to this:

```text
stdout:
Step #1: Starting...
Step #1: Trying to parse '42'
Step #1: Trying to parse '0xA'
Step #1: Trying to parse 'f'
Step #1: Trying to parse 'oobar'
Step #1: Trying to parse '3b'
Step #1: Done. 2 of 5 inputs were u32 (decimal or hex): [42, 10]
There were errors.
Errors will be returned after showing some suggestions.
Step #2: Starting...
Step #2: 'f' is not u32. Did you mean '0xF'?
Step #2: 'oobar' is not u32. Aborting program.

stderr:
Application failed
- Input has correctable or uncorrectable errors
  - Input 'f' is not u32
    at src/lib.rs:72:52
  - Input 'oobar' is not u32
    at src/lib.rs:72:52
  - Input '3b' is not u32
    at src/lib.rs:72:52
  at src/lib.rs:43:14
- Unsupported input 'oobar': invalid digit found in string
  at src/lib.rs:120:17
  at src/lib.rs:45:18
```


## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

 [__cargo_doc2readme_dependencies_info]: ggGkYW0BYXSEG9ybpOeDAqGAG9HvJZNoD8WVG9j2ywGL9HOVG66pmD4ift53YXKEGz3EF3GiOPUnG3K2S9-ZbrqdG4PFyD6yzY-BGxakgkVxTUrRYWSBgmtsYXp5X2Vycm9yc2UwLjYuMA
 [__link0]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=prelude
 [__link1]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=surrogate_error_trait::prelude
 [__link10]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/type.Result.html
 [__link11]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=OrStash::or_stash
 [__link12]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=ErrorStash
 [__link13]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=ErrorStash
 [__link14]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=ErrorStash
 [__link15]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/type.Result.html
 [__link16]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=Error
 [__link17]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=ErrorStash
 [__link18]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=OrCreateStash::or_create_stash
 [__link19]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=OrCreateStash::or_create_stash
 [__link2]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=surrogate_error_trait::Reportable
 [__link20]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=StashWithErrors
 [__link21]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=ErrorStash
 [__link22]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=ErrorStash
 [__link23]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=StashWithErrors
 [__link24]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=ErrorStash
 [__link25]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=StashWithErrors
 [__link26]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=StashWithErrors
 [__link27]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=ErrorStash
 [__link28]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/type.Result.html
 [__link29]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=Error
 [__link3]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=OrStash::or_stash
 [__link30]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=StashWithErrors
 [__link31]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=Error
 [__link32]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=ErrorStash
 [__link33]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=StashWithErrors
 [__link34]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/type.Result.html
 [__link35]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=Error
 [__link36]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=ErrorStash
 [__link37]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=StashWithErrors
 [__link38]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=IntoEyreResult::into_eyre_result
 [__link39]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=IntoEyreReport::into_eyre_report
 [__link4]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=OrStash::or_stash
 [__link40]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=Error
 [__link41]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=OrWrap::or_wrap
 [__link42]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=OrWrapWith::or_wrap_with
 [__link43]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=OrWrap::or_wrap
 [__link44]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=OrWrapWith::or_wrap_with
 [__link45]: Error#inner-error-type-i
 [__link46]: `err!`
 [__link47]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=Error
 [__link48]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=prelude
 [__link49]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=Result
 [__link5]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=OrStash
 [__link50]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=surrogate_error_trait::prelude
 [__link51]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=surrogate_error_trait::Result
 [__link52]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=Stashable
 [__link53]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=surrogate_error_trait::Reportable
 [__link54]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=Error
 [__link55]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=Error
 [__link56]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=OrWrap::or_wrap
 [__link57]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=OrWrapWith::or_wrap_with
 [__link58]: https://doc.rust-lang.org/stable/std/marker/trait.Sync.html
 [__link59]: https://doc.rust-lang.org/stable/std/convert/trait.Into.html
 [__link6]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=prelude
 [__link60]: Error#inner-error-type-i
 [__link61]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=ErrorStash
 [__link62]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=Error
 [__link63]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=prelude
 [__link64]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=Stashable
 [__link65]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=Stashable
 [__link66]: https://doc.rust-lang.org/stable/std/marker/trait.Sized.html
 [__link67]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=prelude
 [__link68]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=Stashable
 [__link7]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=ErrorStash
 [__link8]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=ErrorStash
 [__link9]: https://docs.rs/lazy_errors/0.6.0/lazy_errors/?search=ErrorStash
