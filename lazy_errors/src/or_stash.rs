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

use std::fmt::Display;

use crate::{ErrorStash, StashWithErrors};

/// Adds the [`or_stash`](Self::or_stash) method on `Result<_, E>`,
/// if `E` implements [`Into<I>`](crate::Error#inner-error-type-i).
///
/// Do not implement this trait.
/// Importing the trait is sufficient due to blanket implementations.
/// The trait is implemented on `Result<_, E>` if `E` implements `Into<I>`,
/// where `I` is the [_inner error type_](crate::Error#inner-error-type-i),
/// typically [`Stashable`](crate::prelude::Stashable).
pub trait OrStash<S, I, T>
{
    /// If `self` is `Result::Ok(value)`,
    /// returns the `Ok(value)` variant of [`StashedResult`];
    /// if `self` is `Result::Err(e)`,
    /// adds `e` to the provided [`ErrorStash`] or [`StashWithErrors`]
    /// and returns the `Err` variant [`StashedResult`].
    ///
    /// Used to collect an arbitrary number of `Result::Err` occurrences
    /// in an [`ErrorStash`] or a [`StashWithErrors`] list,
    /// deferring error handling to some later point in time,
    /// for example until additional `Err`s or have been collected
    /// or until some cleanup logic has been executed.
    /// At that point, [`StashWithErrors`] can be converted into
    /// [`Error`](crate::Error) or into `eyre::Report`
    /// while [`ErrorStash`] can be converted into
    /// `Result<(), Error>` or `eyre::Result<()>`.
    ///
    /// ```
    /// use lazy_errors::prelude::*;
    ///
    /// fn main()
    /// {
    ///     let err = run().unwrap_err();
    ///     let printed = format!("{err:#}");
    ///     let printed = lazy_errors::replace_line_numbers(&printed);
    ///     assert_eq!(printed, indoc::indoc! {"
    ///         Failed to run application
    ///         - Input is not ASCII: '🙈'
    ///           at lazy_errors/src/or_stash.rs:1234:56
    ///           at lazy_errors/src/or_stash.rs:1234:56
    ///         - Input is not ASCII: '🙉'
    ///           at lazy_errors/src/or_stash.rs:1234:56
    ///           at lazy_errors/src/or_stash.rs:1234:56
    ///         - Input is not ASCII: '🙊'
    ///           at lazy_errors/src/or_stash.rs:1234:56
    ///           at lazy_errors/src/or_stash.rs:1234:56
    ///         - Cleanup failed
    ///           at lazy_errors/src/or_stash.rs:1234:56
    ///           at lazy_errors/src/or_stash.rs:1234:56"});
    /// }
    ///
    /// fn run() -> Result<(), Error>
    /// {
    ///     let mut stash = ErrorStash::new(|| "Failed to run application");
    ///
    ///     print_if_ascii("🙈").or_stash(&mut stash);
    ///     print_if_ascii("🙉").or_stash(&mut stash);
    ///     print_if_ascii("🙊").or_stash(&mut stash);
    ///     print_if_ascii("42").or_stash(&mut stash);
    ///
    ///     cleanup().or_stash(&mut stash); // Runs regardless of errors
    ///
    ///     stash.into() // Ok(()) if the stash is still empty
    /// }
    ///
    /// fn print_if_ascii(text: &str) -> Result<(), Error>
    /// {
    ///     if !text.is_ascii() {
    ///         return Err(err!("Input is not ASCII: '{text}'"));
    ///     }
    ///
    ///     println!("{text}");
    ///     Ok(())
    /// }
    ///
    /// fn cleanup() -> Result<(), Error>
    /// {
    ///     Err(err!("Cleanup failed"))
    /// }
    /// ```
    ///
    /// The [`ErrorStash`] is created manually in the example above.
    /// Before the first `Err` is added, the [`ErrorStash`] is empty.
    /// Converting an empty [`ErrorStash`] to `Result` will produce `Ok(())`.
    /// When [`or_stash`](OrStash::or_stash) is called on `Result::Err(e)`,
    /// `e` will be moved into the [`ErrorStash`]. As soon as there is
    /// at least one error stored in the [`ErrorStash`], converting it
    /// will yield `Result::Err(Error)`.
    /// Sometimes you don't want to create an empty [`ErrorStash`] beforehand.
    /// It's possible to create a non-empty container on-demand by using
    /// [`or_create_stash`].
    ///
    /// [`ErrorStash`]: crate::ErrorStash
    /// [`or_create_stash`]: crate::OrCreateStash::or_create_stash
    fn or_stash(self, stash: &mut S) -> StashedResult<T, StashWithErrors<I>>;
}

/// Similar to [`core::result::Result`] except that this type
/// is deliberately _not_ `#[must_use]` and is designed for
/// `E` to be either [`ErrorStash`] or [`StashWithErrors`].
///
/// The generic parameter `E` is stored as `&mut`. This _should_ allow
/// chaining/nesting of [`or_stash`](OrStash::or_stash) calls
/// in the future.
/// Additionally, storing it as`&mut` _may_ allow us to implement
/// the [`std::ops::Try`] trait in the future, adding support for
/// the `?` operator. This _could_ be achieved by “stealing” ownership,
/// similarly to [`std::mem::take`].
pub enum StashedResult<'s, T, E>
{
    Ok(T),
    Err(&'s mut E),
}

impl<F, M, I, T, E> OrStash<ErrorStash<F, M, I>, I, T> for Result<T, E>
where
    E: Into<I>,
    F: FnOnce() -> M,
    M: Display,
{
    #[track_caller]
    fn or_stash(
        self,
        stash: &mut ErrorStash<F, M, I>,
    ) -> StashedResult<T, StashWithErrors<I>>
    {
        match self {
            Ok(v) => StashedResult::Ok(v),
            Err(err) => {
                stash.push(err);
                match stash {
                    ErrorStash::Empty(_) => unreachable!(),
                    ErrorStash::WithErrors(stash) => StashedResult::Err(stash),
                }
            },
        }
    }
}

impl<I, T, E> OrStash<StashWithErrors<I>, I, T> for Result<T, E>
where E: Into<I>
{
    #[track_caller]
    fn or_stash(
        self,
        stash: &mut StashWithErrors<I>,
    ) -> StashedResult<T, StashWithErrors<I>>
    {
        match self {
            Ok(v) => StashedResult::Ok(v),
            Err(e) => {
                stash.push(e);
                StashedResult::Err(stash)
            },
        }
    }
}
