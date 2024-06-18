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
    /// # use lazy_errors::doctest_line_num_helper as replace_line_numbers;
    /// use lazy_errors::prelude::*;
    ///
    /// fn main()
    /// {
    ///     let err = run().unwrap_err();
    ///     let printed = format!("{err:#}");
    ///     let printed = replace_line_numbers(&printed);
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
    fn or_stash(self, stash: &mut S) -> StashedResult<T, I>;
}

/// Similar to [`core::result::Result`], except that this type
/// is deliberately _not_ `#[must_use]` and
/// the error type is hardcoded as [`StashWithErrors`].
///
/// [`StashedResult`] is returned from [`or_stash`];
/// there should be no need to create values of this type manually.
/// Note that the [`StashWithErrors`] is kept as `&mut` and actually borrows
/// the inner value from the `&mut` [`ErrorStash`] passed to [`or_stash`].
/// Thus, if you want to keep the results of multiple [`or_stash`] calls
/// around at the same time, in order to extract their `Ok(t)` values later,
/// you need to call [`StashedResult::ok()`] on them.
/// Otherwise you'll get ownership-related compilation errors.
/// Check out [`or_stash`] for an example.
///
/// The reason we're keeping a reference to the [`StashWithErrors`] is
/// that it allows you to use the [`try2!`] macro
/// (and will probably allow you use the `?` operator in the future
/// when the `Try` trait is stabilized).
///
/// [`try2!`]: crate::try2!
/// [`or_stash`]: OrStash::or_stash
pub enum StashedResult<'s, T, I>
{
    Ok(T),
    Err(&'s mut StashWithErrors<I>),
}

impl<F, M, I, T, E> OrStash<ErrorStash<F, M, I>, I, T> for Result<T, E>
where
    E: Into<I>,
    F: FnOnce() -> M,
    M: Display,
{
    #[track_caller]
    fn or_stash(self, stash: &mut ErrorStash<F, M, I>) -> StashedResult<T, I>
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
    fn or_stash(self, stash: &mut StashWithErrors<I>) -> StashedResult<T, I>
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

impl<'s, T, E> StashedResult<'s, T, E>
{
    /// Returns `Some(t)` if `self` is `Ok(t)`, `None` otherwise.
    ///
    /// This method is useful to discard the `&mut` borrowing of the
    /// [`ErrorStash`]/[`StashWithErrors`] that was passed as parameter
    /// to [`or_stash`](OrStash::or_stash).
    /// You may need to do this if you have multiple `or_stash` statements
    /// and want to extract the `Ok(T)` result from them later.
    /// For example, the following example would fail to compile
    /// without calling `ok` (due to borrowing `errs` mutably twice):
    ///
    /// ```
    /// # use core::str::FromStr;
    /// use lazy_errors::{prelude::*, Result};
    ///
    /// fn parse_version(major: &str, minor: &str) -> Result<(u32, u32)>
    /// {
    ///     let mut errs = ErrorStash::new(|| "Invalid version number");
    ///
    ///     let major = u32::from_str(major)
    ///         .or_stash(&mut errs)
    ///         .ok();
    ///
    ///     let minor = u32::from_str(minor)
    ///         .or_stash(&mut errs)
    ///         .ok();
    ///
    ///     // Return _all_ errors if major, minor, or both were invalid.
    ///     errs.into_result()?;
    ///
    ///     // If the result above was `Ok`, all `ok()` calls returned `Some`.
    ///     Ok((major.unwrap(), minor.unwrap()))
    /// }
    ///
    /// assert_eq!(parse_version("42", "1337").unwrap(), (42, 1337));
    ///
    /// assert_eq!(
    ///     parse_version("42", "-1")
    ///         .unwrap_err()
    ///         .children()
    ///         .len(),
    ///     1
    /// );
    ///
    /// assert_eq!(
    ///     parse_version("-1", "-1")
    ///         .unwrap_err()
    ///         .children()
    ///         .len(),
    ///     2
    /// );
    /// ```
    pub fn ok(self) -> Option<T>
    {
        match self {
            StashedResult::Ok(t) => Some(t),
            StashedResult::Err(_) => None,
        }
    }
}
