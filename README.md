# lazy_errors ![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue) [![lazy_errors on crates.io](https://img.shields.io/crates/v/lazy_errors)](https://crates.io/crates/lazy_errors) [![lazy_errors on docs.rs](https://docs.rs/lazy_errors/badge.svg)](https://docs.rs/lazy_errors) [![Source Code Repository](https://img.shields.io/badge/Code-On%20GitHub-blue?logo=GitHub)](https://github.com/Lintermute/lazy_errors)

Effortlessly create, group, and nest arbitrary errors,
and defer error handling ergonomically.

```rust
#[cfg(any(feature = "rust-v1.81", feature = "std"))]
use lazy_errors::{prelude::*, Result};

#[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
use lazy_errors::surrogate_error_trait::{prelude::*, Result};

fn run(input: &[&str]) -> Result<()> {
    let mut errs = ErrorStash::new(|| "There were one or more errors");

    u8::from_str("42").or_stash(&mut errs); // `errs` contains 0 errors
    u8::from_str("1337").or_stash(&mut errs); // `errs` contains 1 errors

    let numbers = input
        .iter()
        .map(|&text| -> Result<u8> {
            u8::from_str(text)
                // Make sure validation produces nicer error messages:
                .or_wrap_with(|| format!("Input '{text}' is invalid"))
        })
        // Fail lazily after collecting all errors:
        .try_collect_or_stash(&mut errs);

    // If any item in `input` is invalid, we don't want to continue
    // but return _all_ errors that have occurred so far.
    let numbers: Vec<u8> = try2!(numbers);

    println!("input = {numbers:?}");

    u8::from_str("-1").or_stash(&mut errs);

    errs.into() // `Ok(())` if `errs` is still empty, `Err` otherwise
}

fn main() {
    let err = run(&["❓", "42", "❗"]).unwrap_err();
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
- number too large to fit in target type
  at src/main.rs:9:26
- Input '❓' is invalid: invalid digit found in string
  at src/main.rs:16:18
  at lazy_errors/src/try_collect_or_stash.rs:148:35
  at lazy_errors/src/stash_err.rs:145:46
- Input '❗' is invalid: invalid digit found in string
  at src/main.rs:16:18
  at lazy_errors/src/try_collect_or_stash.rs:148:35
  at lazy_errors/src/stash_err.rs:145:46
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
`lazy_errors` also supports nested errors.
When you return nested errors from functions,
errors will form a tree while “bubbling up”.
You can report that error tree the user/developer in its entirety.
`lazy_errors` integrates with `core::error::Error`
and is `#![no_std]` by default.

By default, `lazy_errors` will box your error values (like `anyhow`/`eyre`),
which allows you to use different error types in the same `Result` type.
However, `lazy_errors` will respect static error type information
if you provide it explicitly.
If you do so, you can access fields and methods of your error values
at run-time without needing downcasts.
Both modes of operation can work together, as will be shown
in the example on the bottom of the page.
When you define a few simple type aliases,
`lazy_errors` also easily supports custom error types that aren’t
`Sync` or even `Send`.

Common reasons to use the `lazy_errors` crate are:

* You want to return an error but run some fallible cleanup logic before.
* More generally, you’re calling two or more functions that return `Result`,
  and want to return an error that wraps all errors that occurred.
* You’re spawning several parallel activities, wait for their completion,
  and want to return all errors that occurred.
* You want to aggregate multiple errors before running some reporting or
  recovery logic, iterating over all errors collected.
* You need to handle errors that don’t implement
  `core::error::Error`/`Display`/`Debug`/`Send`/`Sync` or other common
  traits.

## Feature Flags

* `std` (*disabled* by default):
  * Support any error type that implements `std::error::Error` (instead of
    `core::error::Error`)
  * Implement `std::error::Error` for `lazy_errors` error types (instead of
    `core::error::Error`)
  * Enable this flag if you’re on Rust v1.80 or older (`core::error::Error`
    was stabilized in Rust v1.81)
* `eyre`: Adds `into_eyre_result` and `into_eyre_report` conversions
* `rust-v$N` (where `$N` is a Rust version number): Add support for error
  types from `core` and `alloc` that were stabilized in the respective Rust
  version.

## MSRV

The MSRV of `lazy_errors` depends on the set of enabled features:

* Rust v1.81 and later supports all features and combinations thereof
* Rust v1.61 .. v1.81 need you to disable all `rust-v$N` features where `$N`
  is greater than the version of your Rust toolchain. For example, to
  compile `lazy_errors` on Rust v1.69, you have to disable `rust-v1.81` and
  `rust-v1.77`, but not `rust-v1.69`.
* `eyre` needs at least Rust v1.65
* Rust versions older than v1.61 are unsupported
* In Rust versions below v1.81, `core::error::Error` is not stable yet. If
  you’re using a Rust version before v1.81, please consider enabling the
  `std` feature to make `lazy_errors` use `std::core::Error` instead.

## Walkthrough

`lazy_errors` can actually support any error type as long as it’s `Sized`;
it doesn’t even need to be `Send` or `Sync`. You only need to specify
the generic type parameters accordingly, as will be shown in the example
on the bottom of this page. Usually however, you’d want to use the
aliased types from the [`prelude`][__link0]. When you’re using these aliases,
errors will be boxed and you can dynamically return groups of errors
of differing types from the same function. When you’re also using
the default feature flags, `lazy_errors` is `#![no_std]` and
integrates with `core::error::Error`. In that case,
`lazy_errors` supports any error type that implements `core::error::Error`,
and all error types from this crate implement `core::error::Error` as well.

In Rust versions below v1.81, `core::error::Error` is not stable yet.
If you’re using an old Rust version, please disable (at least)
the `rust-v1.81` feature and enable the `std` feature instead.
Enabling the `std` feature will make `lazy_errors` use `std::error::Error`
instead of `core::error::Error`. If you’re using an old Rust version and
need `#![no_std]` support nevertheless, please use the types from
the [`surrogate_error_trait::prelude`][__link1] instead of the regular prelude.
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

#### Example: `or_stash` on [`Result`][__link4]

[`or_stash`][__link5] is arguably the most useful method of this crate.
It becomes available on `Result` as soon as you
import the [`OrStash`][__link6] trait or the [`prelude`][__link7].
Here’s an example:

```rust
#[cfg(any(feature = "rust-v1.81", feature = "std"))]
use lazy_errors::{prelude::*, Result};

#[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
use lazy_errors::surrogate_error_trait::{prelude::*, Result};

fn run() -> Result<()> {
    let mut stash = ErrorStash::new(|| "Failed to run application");

    print_if_ascii("❓").or_stash(&mut stash);
    print_if_ascii("❗").or_stash(&mut stash);
    print_if_ascii("42").or_stash(&mut stash);

    cleanup().or_stash(&mut stash); // Runs regardless of earlier errors

    stash.into() // `Ok(())` if the stash was still empty
}

fn print_if_ascii(text: &str) -> Result<()> {
    if !text.is_ascii() {
        return Err(err!("Input is not ASCII: '{text}'"));
    }

    println!("{text}");
    Ok(())
}

fn cleanup() -> Result<()> {
    Err(err!("Cleanup failed"))
}

fn main() {
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

Note that the [`ErrorStash`][__link8] is created manually in the example above.
The [`ErrorStash`][__link9] is empty before the first error is added.
Converting an empty [`ErrorStash`][__link10] to [`Result`][__link11] will produce `Ok(())`.
When [`or_stash`][__link12] is called on `Result::Err(e)`,
`e` will be moved into the [`ErrorStash`][__link13]. As soon as there is
at least one error stored in the [`ErrorStash`][__link14], converting [`ErrorStash`][__link15]
into [`Result`][__link16] will yield a `Result::Err` that contains an [`Error`][__link17],
the main error type from this crate.

#### Example: `or_create_stash` on [`Result`][__link18]

Sometimes you don’t want to create an empty [`ErrorStash`][__link19] beforehand.
In that case you can call [`or_create_stash`][__link20] on `Result`
to create a non-empty container on-demand, whenever necessary.
When [`or_create_stash`][__link21] is called on `Result::Err`, the error
will be put into a [`StashWithErrors`][__link22] instead of an [`ErrorStash`][__link23].
[`ErrorStash`][__link24] and [`StashWithErrors`][__link25] behave similarly.
While both [`ErrorStash`][__link26] and [`StashWithErrors`][__link27] can take additional
errors, a [`StashWithErrors`][__link28] is guaranteed to be non-empty.
The type system will be aware that there is at least one error.
Thus, while [`ErrorStash`][__link29] can only be converted into [`Result`][__link30],
yielding either `Ok(())` or `Err(e)` (where `e` is [`Error`][__link31]),
this distinction allows converting [`StashWithErrors`][__link32] into [`Error`][__link33]
directly.

```rust
#[cfg(any(feature = "rust-v1.81", feature = "std"))]
use lazy_errors::{prelude::*, Result};

#[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
use lazy_errors::surrogate_error_trait::{prelude::*, Result};

fn run() -> Result<()> {
    match write("❌").or_create_stash(|| "Failed to run application") {
        Ok(()) => Ok(()),
        Err(mut stash) => {
            cleanup().or_stash(&mut stash);
            Err(stash.into())
        }
    }
}

fn write(text: &str) -> Result<()> {
    if !text.is_ascii() {
        return Err(err!("Input is not ASCII: '{text}'"));
    }
    Ok(())
}

fn cleanup() -> Result<()> {
    Err(err!("Cleanup failed"))
}

fn main() {
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

#### Example: `stash_err` on [`Iterator`][__link34]

Quite similarly to calling [`or_stash`][__link35] on [`Result`][__link36],
you can call [`stash_err`][__link37] on [`Iterator<Item = Result<T, E>>`][__link38]
to turn it into `Iterator<Item = T>`,
moving any `E` item into an error stash as soon as they are encountered:

```rust
#[cfg(any(feature = "rust-v1.81", feature = "std"))]
use lazy_errors::{prelude::*, Result};

#[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
use lazy_errors::surrogate_error_trait::{prelude::*, Result};

fn parse_input() -> Result<Vec<u8>> {
    let mut errs = ErrorStash::new(|| "Invalid input");

    let input = vec![Ok(1), Err("❓"), Ok(42), Err("❗")];

    let numbers: Vec<u8> = input
        .into_iter()
        .stash_err(&mut errs)
        .collect();

    let err = errs.into_result().unwrap_err();
    let msg = format!("{err}");
    assert_eq!(msg, "Invalid input (2 errors)");

    Ok(numbers)
}

let numbers = parse_input().unwrap();
assert_eq!(&numbers, &[1, 42]);
```

#### Example: `try_collect_or_stash` on [`Iterator`][__link39]

[`try_collect_or_stash`][__link40] is a counterpart to [`Iterator::try_collect`][__link41]
from the Rust standard library that will *not* short-circuit,
but instead move all `Err` items into an error stash.
As explained above,
calling [`stash_err`][__link42] on [`Iterator<Item = Result<…>>`][__link43]
will turn a sequence of `Result<T, E>` into a sequence of `T`.
That method is most useful for
chaining another method on the resulting `Iterator<Item = T>`
before calling [`Iterator::collect`][__link44].
Furthermore, when using `stash_err` together with `collect`,
there will be no indication of whether
the iterator contained any `Err` items:
all `Err` items will simply be moved into the error stash.
If you don’t need to chain any methods between calling
`stash_err` and `collect`, or if
you need `collect` to fail (lazily) if
the iterator contained any `Err` items,
you can call [`try_collect_or_stash`][__link45]
on `Iterator<Item = Result<…>>` instead:

```rust
#[cfg(any(feature = "rust-v1.81", feature = "std"))]
use lazy_errors::{prelude::*, Result};

#[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
use lazy_errors::surrogate_error_trait::{prelude::*, Result};

fn parse_input() -> Result<Vec<u8>> {
    let input = vec![Ok(1), Err("❓"), Ok(42), Err("❗")];

    let mut errs = ErrorStash::new(|| "Invalid input");
    let numbers: Vec<u8> = try2!(input
        .into_iter()
        .try_collect_or_stash(&mut errs));

    unreachable!("try2! will bail due to `Err` items in the iterator")
}

let err = parse_input().unwrap_err();
let msg = format!("{err}");
assert_eq!(msg, "Invalid input (2 errors)");
```

#### Example: `try_map_or_stash` on arrays

[`try_map_or_stash`][__link46] is a counterpart to [`array::try_map`][__link47]
from the Rust standard library that will *not* short-circuit,
but instead move all `Err` elements/results into an error stash.
It will touch *all* elements of arrays
of type `[T; _]` or `[Result<T, E>; _]`,
mapping *each* `T` or `Ok(T)` via the supplied mapping function.
Each time an `Err` element is encountered
or an element is mapped to an `Err` value,
that error will be put into the supplied error stash:

```rust
#[cfg(any(feature = "rust-v1.81", feature = "std"))]
use lazy_errors::{prelude::*, Result};

#[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
use lazy_errors::surrogate_error_trait::{prelude::*, Result};

let mut errs = ErrorStash::new(|| "Invalid input");

let input1: [Result<&str, &str>; 3] = [Ok("1"), Ok("42"), Ok("3")];
let input2: [Result<&str, &str>; 3] = [Ok("1"), Err("42"), Ok("42")];
let input3: [&str; 3] = ["1", "foo", "bar"];

let numbers = input1.try_map_or_stash(u8::from_str, &mut errs);
let numbers = numbers.ok().unwrap();
assert_eq!(numbers, [1, 42, 3]);

let _ = input2.try_map_or_stash(u8::from_str, &mut errs);
let _ = input3.try_map_or_stash(u8::from_str, &mut errs);

let err = errs.into_result().unwrap_err();
let msg = format!("{err}");
assert_eq!(msg, "Invalid input (3 errors)");
```

#### Example: Hierarchies

As you might have noticed, [`Error`][__link48]s form hierarchies:

```rust
#[cfg(any(feature = "rust-v1.81", feature = "std"))]
use lazy_errors::{prelude::*, Result};

#[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
use lazy_errors::surrogate_error_trait::{prelude::*, Result};

fn parent() -> Result<()> {
    let mut stash = ErrorStash::new(|| "In parent(): child() failed");
    stash.push(child().unwrap_err());
    stash.into()
}

fn child() -> Result<()> {
    let mut stash = ErrorStash::new(|| "In child(): There were errors");
    stash.push("First error");
    stash.push("Second error");
    stash.into()
}

fn main() {
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
Instead, you’d probably rely on [`or_wrap`][__link49] or [`or_wrap_with`][__link50].

#### Example: Wrapping on [`Result`][__link51]

You can use [`or_wrap`][__link52] or [`or_wrap_with`][__link53] to wrap any value
that can be converted into the
[*inner error type* of `Error`][__link54]
or to attach some context to an error:

```rust
#[cfg(any(feature = "rust-v1.81", feature = "std"))]
use lazy_errors::{prelude::*, Result};

#[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
use lazy_errors::surrogate_error_trait::{prelude::*, Result};

fn run(s: &str) -> Result<u32> {
    parse(s).or_wrap_with(|| format!("Not an u32: '{s}'"))
}

fn parse(s: &str) -> Result<u32> {
    let r: Result<u32, core::num::ParseIntError> = s.parse();

    // Wrap the error type “silently”:
    // No additional message, just file location and wrapped error type.
    r.or_wrap()
}

fn main() {
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

The [`err!`][__link55] macro allows you to format a string
and turn it into an ad-hoc [`Error`][__link56] at the same time:

```rust
#[cfg(any(feature = "rust-v1.81", feature = "std"))]
use lazy_errors::prelude::*;

#[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
use lazy_errors::surrogate_error_trait::prelude::*;

let pid = 42;
let err: Error = err!("Error in process {pid}");
```

You’ll often find ad-hoc errors to be the leaves in an error tree.
However, the error tree can have almost any
[*inner error type*][__link57] as leaf.

#### Example: `into_eyre_*`

[`ErrorStash`][__link58] and [`StashWithErrors`][__link59] can be converted into
[`Result`][__link60] and [`Error`][__link61], respectively. A similar, albeit lossy,
conversion from [`ErrorStash`][__link62] and [`StashWithErrors`][__link63] exist for
`eyre::Result` and `eyre::Error` (i.e. `eyre::Report`), namely
[`into_eyre_result`][__link64] and
[`into_eyre_report`][__link65]:

```rust
use eyre::bail;
use lazy_errors::prelude::*;

fn run() -> Result<(), eyre::Report> {
    let r = write("❌").or_create_stash::<Stashable>(|| "Failed to run");
    match r {
        Ok(()) => Ok(()),
        Err(mut stash) => {
            cleanup().or_stash(&mut stash);
            bail!(stash.into_eyre_report());
        }
    }
}

fn write(text: &str) -> Result<(), Error> {
    if !text.is_ascii() {
        return Err(err!("Input is not ASCII: '{text}'"));
    }
    Ok(())
}

fn cleanup() -> Result<(), Error> {
    Err(err!("Cleanup failed"))
}

fn main() {
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

#### Supported Error Types

The [`prelude`][__link66] module
exports commonly used traits and *aliased* types.
Importing `lazy_errors::prelude::*`
should set you up for most use-cases.
You may also want to import [`lazy_errors::Result`][__link67].
When `core::error::Error` is not available
(i.e. in `![no_std]` mode before Rust v1.81),
you can import the [`surrogate_error_trait::prelude`][__link68] instead, and use
the corresponding [`lazy_errors::surrogate_error_trait::Result`][__link69].

When you’re using the aliased types from the prelude, this crate should
support any `Result<_, E>` if `E` implements `Into<Stashable>`.
[`Stashable`][__link70] is, basically, a `Box<dyn E>`, where `E` is either
`core::error::Error` (Rust v1.81 or later),
`std::error::Error` (before Rust v1.81 if `std` is enabled),
or a surrogate error trait otherwise
([`surrogate_error_trait::Reportable`][__link71]).
Thus, using the aliased types from the prelude, any error you put into
any of the containers defined by this crate will be boxed.
The `Into<Box<dyn E>>` trait bound was chosen because it is implemented
for a wide range of error types or *“error-like”* types.
Some examples of types that satisfy this constraint are:

* `&str`
* `String`
* `anyhow::Error`
* `eyre::Report`
* `core::error::Error`
* All error types from this crate

The primary error type from this crate is [`Error`][__link72].
You can convert all supported *error-like* types into [`Error`][__link73]
by calling [`or_wrap`][__link74] or [`or_wrap_with`][__link75].

In other words, this crate supports a wide variety of error types.
However, in some cases you might need a different kind of flexibility
than that. For example, maybe you don’t want to lose static error type
information or maybe your error types aren’t [`Sync`][__link76].
In general, this crate should work well with any `Result<_, E>`
if `E` implements [`Into<I>`][__link77] where `I` is named the
[*inner error type* of `Error`][__link78].
This crate will store errors as type `I` in its containers, for example
in [`ErrorStash`][__link79] or in [`Error`][__link80]. When you’re using the type aliases
from the [`prelude`][__link81], `I` will always be [`Stashable`][__link82].
However, you do not need to use [`Stashable`][__link83] at all.
You can chose the type to use for `I` arbitrarily.
It can be a custom type and does not need to implement any traits
or auto traits except [`Sized`][__link84].
Thus, if the default aliases defined in the prelude
do not suit your purpose, you can import the required traits
and types manually and define custom aliases, as shown in the next example.

#### Example: Custom Error Types

Here’s a complex example that does not use the [`prelude`][__link85]
but instead defines its own aliases. In the example, `Error<CustomError>`
and `ParserErrorStash` don’t box their errors. Instead, they have all
error type information present statically, which allows you to write
recovery logic without having to rely on downcasts at run-time.
The example also shows how such custom error types
can still be used alongside the boxed error types ([`Stashable`][__link86])
with custom lifetimes.

```rust
use lazy_errors::{err, ErrorStash, OrStash, StashedResult};

#[cfg(any(feature = "rust-v1.81", feature = "std"))]
use lazy_errors::Stashable;

#[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
use lazy_errors::surrogate_error_trait::Stashable;

#[derive(thiserror::Error, Debug)]
pub enum CustomError<'a> {
    #[error("Input is empty")]
    EmptyInput,

    #[error("Input '{0}' is not u32")]
    NotU32(&'a str),
}

// Use `CustomError` as inner error type `I` for `ErrorStash`:
type ParserErrorStash<'a, F, M> = ErrorStash<F, M, CustomError<'a>>;

// Allow using `CustomError` as `I` but use `Stashable` by default:
pub type Error<I = Stashable<'static>> = lazy_errors::Error<I>;

fn main() {
    let err = run(&["42", "0xA", "f", "oobar", "3b"]).unwrap_err();
    eprintln!("{err:#}");
}

fn run<'a>(input: &[&'a str]) -> Result<(), Error<Stashable<'a>>> {
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

fn parse<'a>(input: &[&'a str]) -> Result<(), Error<CustomError<'a>>> {
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

fn handle_parser_errors(errs: &Error<CustomError>) -> Result<(), Error> {
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

fn parse_u32(s: &str) -> Result<u32, CustomError> {
    s.strip_prefix("0x")
        .map(|hex| u32::from_str_radix(hex, 16))
        .unwrap_or_else(|| u32::from_str(s))
        .map_err(|_| CustomError::NotU32(s))
}

fn guess_hex(s: &str) -> Result<u32, Error> {
    match u32::from_str_radix(s, 16) {
        Ok(v) => {
            println!("Step #2: '{s}' is not u32. Did you mean '{v:#X}'?");
            Ok(v)
        }
        Err(e) => {
            println!("Step #2: '{s}' is not u32. Aborting program.");
            Err(err!("Unsupported input '{s}': {e}"))
        }
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

 [__cargo_doc2readme_dependencies_info]: ggGkYW0BYXSEG9ybpOeDAqGAG9HvJZNoD8WVG9j2ywGL9HOVG66pmD4ift53YXKEG3ebbQQTOIEXG3aroVpsxSS-GwLBNE2sbEOAG85gbCIe6nJgYWSCgmVhcnJhefaCa2xhenlfZXJyb3JzZTAuOS4w
 [__link0]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=prelude
 [__link1]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=surrogate_error_trait::prelude
 [__link10]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=ErrorStash
 [__link11]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/type.Result.html
 [__link12]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=OrStash::or_stash
 [__link13]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=ErrorStash
 [__link14]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=ErrorStash
 [__link15]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=ErrorStash
 [__link16]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/type.Result.html
 [__link17]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=Error
 [__link18]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/type.Result.html
 [__link19]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=ErrorStash
 [__link2]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=surrogate_error_trait::Reportable
 [__link20]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=OrCreateStash::or_create_stash
 [__link21]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=OrCreateStash::or_create_stash
 [__link22]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=StashWithErrors
 [__link23]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=ErrorStash
 [__link24]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=ErrorStash
 [__link25]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=StashWithErrors
 [__link26]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=ErrorStash
 [__link27]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=StashWithErrors
 [__link28]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=StashWithErrors
 [__link29]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=ErrorStash
 [__link3]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=OrStash::or_stash
 [__link30]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/type.Result.html
 [__link31]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=Error
 [__link32]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=StashWithErrors
 [__link33]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=Error
 [__link34]: https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html
 [__link35]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=OrStash::or_stash
 [__link36]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/type.Result.html
 [__link37]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=StashErr::stash_err
 [__link38]: https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html
 [__link39]: https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html
 [__link4]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/type.Result.html
 [__link40]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=TryCollectOrStash::try_collect_or_stash
 [__link41]: https://doc.rust-lang.org/stable/std/?search=iter::Iterator::try_collect
 [__link42]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=StashErr::stash_err
 [__link43]: https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html
 [__link44]: https://doc.rust-lang.org/stable/std/?search=iter::Iterator::collect
 [__link45]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=TryCollectOrStash::try_collect_or_stash
 [__link46]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=TryMapOrStash::try_map_or_stash
 [__link47]: https://docs.rs/array/latest/array/?search=try_map
 [__link48]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=Error
 [__link49]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=OrWrap::or_wrap
 [__link5]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=OrStash::or_stash
 [__link50]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=OrWrapWith::or_wrap_with
 [__link51]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/type.Result.html
 [__link52]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=OrWrap::or_wrap
 [__link53]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=OrWrapWith::or_wrap_with
 [__link54]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/struct.Error.html#inner-error-type-i
 [__link55]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/macro.err.html
 [__link56]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=Error
 [__link57]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/struct.Error.html#inner-error-type-i
 [__link58]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=ErrorStash
 [__link59]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=StashWithErrors
 [__link6]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=OrStash
 [__link60]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/type.Result.html
 [__link61]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=Error
 [__link62]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=ErrorStash
 [__link63]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=StashWithErrors
 [__link64]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=IntoEyreResult::into_eyre_result
 [__link65]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=IntoEyreReport::into_eyre_report
 [__link66]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=prelude
 [__link67]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=Result
 [__link68]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=surrogate_error_trait::prelude
 [__link69]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=surrogate_error_trait::Result
 [__link7]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=prelude
 [__link70]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=Stashable
 [__link71]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=surrogate_error_trait::Reportable
 [__link72]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=Error
 [__link73]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=Error
 [__link74]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=OrWrap::or_wrap
 [__link75]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=OrWrapWith::or_wrap_with
 [__link76]: https://doc.rust-lang.org/stable/std/marker/trait.Sync.html
 [__link77]: https://doc.rust-lang.org/stable/std/convert/trait.Into.html
 [__link78]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/struct.Error.html#inner-error-type-i
 [__link79]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=ErrorStash
 [__link8]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=ErrorStash
 [__link80]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=Error
 [__link81]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=prelude
 [__link82]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=Stashable
 [__link83]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=Stashable
 [__link84]: https://doc.rust-lang.org/stable/std/marker/trait.Sized.html
 [__link85]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=prelude
 [__link86]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=Stashable
 [__link9]: https://docs.rs/lazy_errors/0.10.1/lazy_errors/?search=ErrorStash
