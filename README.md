# lazy_errors ![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue) [![lazy_errors on crates.io](https://img.shields.io/crates/v/lazy_errors)](https://crates.io/crates/lazy_errors) [![lazy_errors on docs.rs](https://docs.rs/lazy_errors/badge.svg)](https://docs.rs/lazy_errors) [![Source Code Repository](https://img.shields.io/badge/Code-On%20GitHub-blue?logo=GitHub)](https://github.com/Lintermute/lazy_errors)

Effortlessly create, group, and nest arbitrary errors,
and defer error handling ergonomically.

```rust
use lazy_errors::{prelude::*, Result};

fn run() -> Result<()>
{
    let mut errs = ErrorStash::new(|| "Failed to run application");

    write_if_ascii("42").or_stash(&mut errs); // `errs` contains 0 errors
    write_if_ascii("❌").or_stash(&mut errs); // `errs` contains 1 error

    cleanup().or_stash(&mut errs); // Run cleanup even if there were errors,
                                   // but cleanup is allowed to fail as well

    errs.into() // `Ok(())` if `errs` was still empty, `Err` otherwise
}

let errs = run().unwrap_err();
assert_eq!(errs.childs().len(), 2);
```

## In a Nutshell

`lazy_errors` provides types, traits, and blanket implementations
on `Result` that can be used to ergonomically defer error handling.
Additionally, `lazy_errors` allows you to easily create ad-hoc errors
as well as wrap, group, and nest a wide variety of errors
in a single common error type, simplifying your codebase.
In that latter regard, `lazy_errors` is similar to `anyhow`/`eyre`,
except that its reporting isn’t as fancy or detailed (for example,
`lazy_errors` tracks source code file name and line numbers instead of
providing full `std::backtrace` support).
On the other hand, `lazy_errors` uses `#![no_std]` by default but
integrates with `std::error::Error` if you enable the `std` feature.
`lazy_errors` also supports error types that aren’t `Send` or `Sync`
and allows you to group and nest errors arbitrarily with minimal effort.

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

## Walkthrough

`lazy_errors` actually supports any error type as long as it’s `Sized`;
it doesn’t even need to be `Send` or `Sync`. You only need to specify
the generic type parameters accordingly, as shown in the example
on the bottom of this page. Usually however, you’d want to use the
aliased types from the [`prelude`][__link0]. When you’re using these aliases,
errors will be boxed and you can dynamically return groups of errors
of differing types from the same function.
In the default `#![no_std]` mode, `lazy_errors` can box any error type
that implements the [`Reportable`][__link1] marker trait; if necessary,
you can implement that trait in a single line for your custom types.
If you need to handle third-party error types that already implement
`std::error::Error` instead, you can enable the `std` feature.
When `std` is enabled, all error types from this crate will
implement `std::error::Error` as well.

While `lazy_errors` works standalone, it’s not intended to replace
`anyhow` or `eyre`. Instead, this project was started to explore
approaches on how to run multiple fallible operations, aggregate
their errors (if any), and defer the actual error handling/reporting
by returning all of these errors from functions that return `Result`.
Generally, `Result<_, Vec<_>>` can be used for this purpose,
which is not much different from what `lazy_errors` does internally.
However, `lazy_errors` provides “syntactic sugar”
to make this approach more ergonomic.
Thus, arguably the most useful method in this crate is [`or_stash`][__link2].

#### Example: `or_stash`

[`or_stash`][__link3] is arguably the most useful method of this crate.
It becomes available on `Result` as soon as you
import the [`OrStash`][__link4] trait or the [`prelude`][__link5].
Here’s an example:

```rust
use lazy_errors::prelude::*;

fn main()
{
    let err = run().unwrap_err();
    let printed = format!("{err:#}");
    let printed = lazy_errors::replace_line_numbers(&printed);
    assert_eq!(printed, indoc::indoc! {"
        Failed to run application
        - Input is not ASCII: '🙈'
          at lazy_errors/src/lib.rs:1234:56
          at lazy_errors/src/lib.rs:1234:56
        - Input is not ASCII: '🙉'
          at lazy_errors/src/lib.rs:1234:56
          at lazy_errors/src/lib.rs:1234:56
        - Input is not ASCII: '🙊'
          at lazy_errors/src/lib.rs:1234:56
          at lazy_errors/src/lib.rs:1234:56
        - Cleanup failed
          at lazy_errors/src/lib.rs:1234:56
          at lazy_errors/src/lib.rs:1234:56"});
}

fn run() -> Result<(), Error>
{
    let mut stash = ErrorStash::new(|| "Failed to run application");

    print_if_ascii("🙈").or_stash(&mut stash);
    print_if_ascii("🙉").or_stash(&mut stash);
    print_if_ascii("🙊").or_stash(&mut stash);
    print_if_ascii("42").or_stash(&mut stash);

    cleanup().or_stash(&mut stash); // Runs regardless of earlier errors

    stash.into() // `Ok(())` if the stash was still empty
}

fn print_if_ascii(text: &str) -> Result<(), Error>
{
    if !text.is_ascii() {
        return Err(err!("Input is not ASCII: '{text}'"));
    }

    println!("{text}");
    Ok(())
}

fn cleanup() -> Result<(), Error>
{
    Err(err!("Cleanup failed"))
}
```

In the example above, `run()` will print `42`, run `cleanup()`,
and then return the stashed errors.

Note that the [`ErrorStash`][__link6] is created manually in the example above.
The [`ErrorStash`][__link7] is empty before the first error is added.
Converting an empty [`ErrorStash`][__link8] to [`Result`][__link9] will produce `Ok(())`.
When [`or_stash`][__link10] is called on `Result::Err(e)`,
`e` will be moved into the [`ErrorStash`][__link11]. As soon as there is
at least one error stored in the [`ErrorStash`][__link12], converting [`ErrorStash`][__link13]
into [`Result`][__link14] will yield a `Result::Err` that contains an [`Error`][__link15],
the main error type from this crate.

#### Example: `or_create_stash`

Sometimes you don’t want to create an empty [`ErrorStash`][__link16] beforehand.
In that case you can call [`or_create_stash`][__link17] on `Result`
to create a non-empty container on-demand, whenever necessary.
When [`or_create_stash`][__link18] is called on `Result::Err`, the error
will be put into a [`StashWithErrors`][__link19] instead of an [`ErrorStash`][__link20].
[`ErrorStash`][__link21] and [`StashWithErrors`][__link22] behave quite similarly.
While both [`ErrorStash`][__link23] and [`StashWithErrors`][__link24] can take additional
errors, a [`StashWithErrors`][__link25] is guaranteed to be non-empty.
The type system will be aware that there is at least one error.
Thus, while [`ErrorStash`][__link26] can only be converted into [`Result`][__link27],
yielding either `Ok(())` or `Err(e)` (where `e` is [`Error`][__link28]),
this distinction allows converting [`StashWithErrors`][__link29] into [`Error`][__link30]
directly.

```rust
use lazy_errors::prelude::*;

fn main()
{
    let err = run().unwrap_err();
    let printed = format!("{err:#}");
    let printed = lazy_errors::replace_line_numbers(&printed);
    assert_eq!(printed, indoc::indoc! {"
        Failed to run application
        - Input is not ASCII: '❌'
          at lazy_errors/src/lib.rs:1234:56
          at lazy_errors/src/lib.rs:1234:56
        - Cleanup failed
          at lazy_errors/src/lib.rs:1234:56
          at lazy_errors/src/lib.rs:1234:56"});
}

fn run() -> Result<(), Error>
{
    match write("❌").or_create_stash(|| "Failed to run application") {
        Ok(()) => Ok(()),
        Err(mut stash) => {
            cleanup().or_stash(&mut stash);
            return Err(stash.into());
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

#### Example: `into_eyre_*`

[`ErrorStash`][__link31] and [`StashWithErrors`][__link32] can be converted into
[`Result`][__link33] and [`Error`][__link34], respectively. A similar, albeit lossy,
conversion from [`ErrorStash`][__link35] and [`StashWithErrors`][__link36] exist for
`eyre::Result` and `eyre::Error` (i.e. `eyre::Report`), namely
[`into_eyre_result`][__link37] and
[`into_eyre_report`][__link38]:

```rust
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

#### Example: Hierarchies

As you might have noticed, [`Error`][__link39]s form hierarchies:

```rust
use lazy_errors::prelude::*;

fn main()
{
    let err = first().unwrap_err();
    let printed = format!("{err:#}");
    let printed = lazy_errors::replace_line_numbers(&printed);
    assert_eq!(printed, indoc::indoc! {"
        In first(): second() failed
        - In second(): third() failed
          - In third(): There were errors
            - First error
              at lazy_errors/src/lib.rs:1234:56
            - Second error
              at lazy_errors/src/lib.rs:1234:56
            at lazy_errors/src/lib.rs:1234:56
          at lazy_errors/src/lib.rs:1234:56"});
}

fn first() -> Result<(), Error>
{
    let mut stash = ErrorStash::new(|| "In first(): second() failed");
    stash.push(second().unwrap_err());
    stash.into()
}

fn second() -> Result<(), Error>
{
    let mut stash = ErrorStash::new(|| "In second(): third() failed");
    stash.push(third().unwrap_err());
    stash.into()
}

fn third() -> Result<(), Error>
{
    let mut stash = ErrorStash::new(|| "In third(): There were errors");

    stash.push("First error");
    stash.push("Second error");

    stash.into()
}
```

The example above may seem unwieldy. In fact, that example only serves
the purpose to illustrate the error hierarchy.
In practice, you wouldn’t write such code.
Instead, you’d probably rely on [`or_wrap`][__link40] or [`or_wrap_with`][__link41].

#### Example: Wrapping

You can use [`or_wrap`][__link42] or [`or_wrap_with`][__link43] to wrap any value
that can be converted into the
[*inner error type* of `Error`][__link44]
or to attach some context to an error:

```rust
use lazy_errors::{prelude::*, Result};

fn main()
{
    let err = first().unwrap_err();
    let printed = format!("{err:#}");
    let printed = lazy_errors::replace_line_numbers(&printed);
    assert_eq!(printed, indoc::indoc! {"
        Something went wrong: In third(): There were errors
        - First error
          at lazy_errors/src/lib.rs:1234:56
        - Second error
          at lazy_errors/src/lib.rs:1234:56
        at lazy_errors/src/lib.rs:1234:56
        at lazy_errors/src/lib.rs:1234:56"});
}

fn first() -> Result<(), Error>
{
    second().or_wrap_with(|| "Something went wrong")
}

fn second() -> Result<()>
{
    third().or_wrap() // Wrap it “silently”: No message, just file location
}

fn third() -> Result<()>
{
    let mut stash = ErrorStash::new(|| "In third(): There were errors");

    stash.push("First error");
    stash.push("Second error");

    stash.into()
}
```

#### Example: Ad-Hoc Errors

The [`err!`][__link45] macro allows you to format a string
and turn it into an ad-hoc [`Error`][__link46] at the same time:

```rust
use lazy_errors::{prelude::*, Result};

let pid = 42;
let err: Error = err!("Error in process {pid}");
```

You’ll often find ad-hoc errors to be the leaves in an error tree.
However, the error tree can have almost any *inner error type* as leaf.

#### Supported Error Types

The [`prelude`][__link47] module exports commonly used traits and *aliased* types.
Importing `prelude::*` should set you up for most use-cases.
You may also want to import [`lazy_errors::Result`][__link48].
When you’re using the aliased types from the prelude, this crate should
support any `Result<_, E>` if `E` implements `Into<`[`Stashable`][__link49]`>`.
[`Stashable`][__link50] is, basically, a `Box<dyn E>`, where `E` is either
`std::error::Error` or a similar trait in `#![no_std]` mode.
Thus, using the aliased types from the prelude, any error you put into
any of the containers defined by this crate will be boxed.
The `Into<Box<dyn E>>` trait bound was chosen because it is implemented
for a wide range of error types or *“error-like”* types.
Some examples of types that satisfy this constraint are:

* [`&str`][__link51]
* [`String`][__link52]
* `eyre::Report`
* `anyhow::Error`
* [`std::error::Error`][__link53]
* All error types from this crate

The primary error type from this crate is [`Error`][__link54].
You can convert all supported *error-like* types into [`Error`][__link55]
by calling [`or_wrap`][__link56] or [`or_wrap_with`][__link57]:

```rust
use lazy_errors::prelude::*;

fn main()
{
    let err = parent().unwrap_err();
    let printed = format!("{err:#}");
    let printed = lazy_errors::replace_line_numbers(&printed);
    assert_eq!(printed, indoc::indoc! {"
        In parent(): child() failed: Arbitrary String
        at lazy_errors/src/lib.rs:1234:56"});
}

fn parent() -> Result<(), Error>
{
    child().or_wrap_with(|| "In parent(): child() failed")
}

fn child() -> Result<(), String>
{
    Err(String::from("Arbitrary String"))
}
```

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
The concrete type to use for `I` may be chosen by the user arbitrarily.
It can be a custom type and does not need to implement any traits
or auto traits except [`Sized`][__link66].
Thus, if the default aliases defined in the prelude
do not suit your purpose, you can import the required traits
and types manually and define custom aliases, as shown below.

#### Example: Custom Error Types

Here’s a complex example that does not use the [`prelude`][__link67]
but instead defines its own aliases.
These error types have their static type information still present,
enabling running recovery logic without having to rely on downcasts
at run-time. The example also shows how such custom error types
can still be used alongside the boxed error types ([`Stashable`][__link68]s)
with custom lifetimes.

```rust
use std::str::FromStr;

use lazy_errors::{
    err,
    Error,
    ErrorStash,
    OrStash,
    Result,
    Stashable,
    StashedResult,
};

#[derive(thiserror::Error, Debug)]
pub enum CustomError<'a>
{
    #[error("Input is empty")]
    EmptyInput,

    #[error("Input '{0}' is not u32")]
    NotU32(&'a str),
}

// Use `CustomError` as `I` for both `Error` and `ErrorStash`:
type ParserError<'a> = Error<CustomError<'a>>;
type ParserStash<'a, F, M> = ErrorStash<F, M, CustomError<'a>>;

fn main()
{
    let err = run(&["42", "0xA", "f", "oobar", "3b"]).unwrap_err();
    eprintln!("{err:#}");
}

fn run<'a>(input: &[&'a str]) -> Result<(), Error<Stashable<'a>>>
{
    let mut errs = ErrorStash::new(|| "Application failed");

    let parser_result = parse_input(input); // Soft errors
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

fn parse_input<'a>(input: &[&'a str]) -> Result<(), ParserError<'a>>
{
    if input.is_empty() {
        return Err(Error::wrap(CustomError::EmptyInput));
    }

    let mut errs = ParserStash::new(|| {
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

fn handle_parser_errors(errs: &ParserError) -> Result<()>
{
    println!("Step #2: Starting...");

    for e in errs.childs() {
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

fn guess_hex(s: &str) -> Result<u32>
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
    at lazy_errors/src/lib.rs:72:52
  - Input 'oobar' is not u32
    at lazy_errors/src/lib.rs:72:52
  - Input '3b' is not u32
    at lazy_errors/src/lib.rs:72:52
  at lazy_errors/src/lib.rs:43:14
- Unsupported input 'oobar': invalid digit found in string
  at lazy_errors/src/lib.rs:120:17
  at lazy_errors/src/lib.rs:45:18
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

 [__cargo_doc2readme_dependencies_info]: ggGkYW0BYXSEG9ybpOeDAqGAG9HvJZNoD8WVG9j2ywGL9HOVG66pmD4ift53YXKEG1UdOg9mvdfeG50zSyeHMjByG3rkMmEK2-80Gwrd-I5UmUIaYWSCgmpSZXBvcnRhYmxl9oJrbGF6eV9lcnJvcnNlMC40LjA
 [__link0]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/prelude/index.html
 [__link1]: https://crates.io/crates/Reportable
 [__link10]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=OrStash::or_stash
 [__link11]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=ErrorStash
 [__link12]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=ErrorStash
 [__link13]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=ErrorStash
 [__link14]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/type.Result.html
 [__link15]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=Error
 [__link16]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=ErrorStash
 [__link17]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=OrCreateStash::or_create_stash
 [__link18]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=OrCreateStash::or_create_stash
 [__link19]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=StashWithErrors
 [__link2]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=OrStash::or_stash
 [__link20]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=ErrorStash
 [__link21]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=ErrorStash
 [__link22]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=StashWithErrors
 [__link23]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=ErrorStash
 [__link24]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=StashWithErrors
 [__link25]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=StashWithErrors
 [__link26]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=ErrorStash
 [__link27]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/type.Result.html
 [__link28]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=Error
 [__link29]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=StashWithErrors
 [__link3]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=OrStash::or_stash
 [__link30]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=Error
 [__link31]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=ErrorStash
 [__link32]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=StashWithErrors
 [__link33]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/type.Result.html
 [__link34]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=Error
 [__link35]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=ErrorStash
 [__link36]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=StashWithErrors
 [__link37]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=IntoEyreResult::into_eyre_result
 [__link38]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=IntoEyreReport::into_eyre_report
 [__link39]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=Error
 [__link4]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=OrStash
 [__link40]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=OrWrap::or_wrap
 [__link41]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=OrWrapWith::or_wrap_with
 [__link42]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=OrWrap::or_wrap
 [__link43]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=OrWrapWith::or_wrap_with
 [__link44]: Error#inner-error-type-i
 [__link45]: `err!`
 [__link46]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=Error
 [__link47]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/prelude/index.html
 [__link48]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=Result
 [__link49]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=Stashable
 [__link5]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/prelude/index.html
 [__link50]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=Stashable
 [__link51]: `&str`
 [__link52]: https://doc.rust-lang.org/stable/alloc/?search=string::String
 [__link53]: https://doc.rust-lang.org/stable/std/?search=error::Error
 [__link54]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=Error
 [__link55]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=Error
 [__link56]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=OrWrap::or_wrap
 [__link57]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=OrWrapWith::or_wrap_with
 [__link58]: https://doc.rust-lang.org/stable/std/marker/trait.Sync.html
 [__link59]: https://doc.rust-lang.org/stable/std/convert/trait.Into.html
 [__link6]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=ErrorStash
 [__link60]: Error#inner-error-type-i
 [__link61]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=ErrorStash
 [__link62]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=Error
 [__link63]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/prelude/index.html
 [__link64]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=Stashable
 [__link65]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=Stashable
 [__link66]: https://doc.rust-lang.org/stable/std/marker/trait.Sized.html
 [__link67]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/prelude/index.html
 [__link68]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=Stashable
 [__link7]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=ErrorStash
 [__link8]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/?search=ErrorStash
 [__link9]: https://docs.rs/lazy_errors/0.4.0/lazy_errors/type.Result.html
