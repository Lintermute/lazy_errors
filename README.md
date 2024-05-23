# lazy_errors

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
except that its reporting isn't as fancy or detailed (for example,
`lazy_errors` tracks source code file name and line numbers instead of
providing full `std::backtrace` support).
On the other hand, `lazy_errors` uses `#![no_std]` by default but
integrates with `std::error::Error` if you enable the `std` feature.
`lazy_errors` also supports error types that aren't `Send` or `Sync`
and allows you to group and nest errors arbitrarily with minimal effort.

Common reasons to use this crate are:

- You want to return an error but run some fallible cleanup logic before.
- More generally, you're calling two or more functions that return `Result`,
  and want to return an error that wraps all errors that occurred.
- You're spawning several parallel activities, wait for their completion,
  and want to return all errors that occurred.
- You want to aggregate multiple errors before running some reporting or
  recovery logic, iterating over all errors collected.
- You need to handle errors that don't implement
  `std::error::Error`/`Display`/`Debug`/`Send`/`Sync` or other common
  traits.

## Walkthrough

`lazy_errors` actually supports any error type as long as it's `Sized`;
it doesn't even need to be `Send` or `Sync`. You only need to specify
the generic type parameters accordingly, as shown in the example
on the bottom of this page. Usually however, you'd want to use the
aliased types from the `prelude`. When you're using these aliases,
errors will be boxed and you can dynamically return groups of errors
of differing types from the same function.
In the default `#![no_std]` mode, `lazy_errors` can box any error type
that implements the `Reportable` marker trait; if necessary,
you can implement that trait in a single line for your custom types.
If you need to handle third-party error types that already implement
`std::error::Error` instead, you can enable the `std` feature.
When `std` is enabled, all error types from this crate will
implement `std::error::Error` as well.

While `lazy_errors` works standalone, it's not intended to replace
`anyhow` or `eyre`. Instead, this project was started to explore
approaches on how to run multiple fallible operations, aggregate
their errors (if any), and defer the actual error handling/reporting
by returning all of these errors from functions that return `Result`.
Generally, `Result<_, Vec<_>>` can be used for this purpose,
which is not much different from what `lazy_errors` does internally.
However, `lazy_errors` provides “syntactic sugar”
to make this approach more ergonomic.
Thus, arguably the most useful method in this crate is `or_stash`.

#### Example: `or_stash`

`or_stash` is arguably the most useful method of this crate.
It becomes available on `Result` as soon as you
import the `OrStash` trait or the `prelude`.
Here's an example:

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

Note that the `ErrorStash` is created manually in the example above.
The `ErrorStash` is empty before the first error is added.
Converting an empty `ErrorStash` to `Result` will produce `Ok(())`.
When `or_stash` is called on `Result::Err(e)`,
`e` will be moved into the `ErrorStash`. As soon as there is
at least one error stored in the `ErrorStash`, converting `ErrorStash`
into `Result` will yield a `Result::Err` that contains an `Error`,
the main error type from this crate.

#### Example: `or_create_stash`

Sometimes you don't want to create an empty `ErrorStash` beforehand.
In that case you can call `or_create_stash` on `Result`
to create a non-empty container on-demand, whenever necessary.
When `or_create_stash` is called on `Result::Err`, the error
will be put into a `StashWithErrors` instead of an `ErrorStash`.
`ErrorStash` and `StashWithErrors` behave quite similarly.
While both `ErrorStash` and `StashWithErrors` can take additional
errors, a `StashWithErrors` is guaranteed to be non-empty.
The type system will be aware that there is at least one error.
Thus, while `ErrorStash` can only be converted into `Result`,
yielding either `Ok(())` or `Err(e)` (where `e` is `Error`),
this distinction allows converting `StashWithErrors` into `Error`
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

`ErrorStash` and `StashWithErrors` can be converted into
`Result` and `Error`, respectively. A similar, albeit lossy,
conversion from `ErrorStash` and `StashWithErrors` exist for
`eyre::Result` and `eyre::Error` (i.e. `eyre::Report`), namely

License: MIT
