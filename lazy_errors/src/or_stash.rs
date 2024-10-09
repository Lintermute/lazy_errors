use crate::{stash::ErrorSink, StashWithErrors};

/// Adds the [`or_stash`](Self::or_stash) method on `Result<_, E>`,
/// if `E` implements [`Into<I>`](crate::Error#inner-error-type-i).
///
/// Do not implement this trait.
/// Importing the trait is sufficient due to blanket implementations.
/// The trait is implemented on `Result<_, E>` if `E` implements `Into<I>`,
/// where `I` is the [_inner error type_](crate::Error#inner-error-type-i),
/// typically [`prelude::Stashable`].
#[cfg_attr(
    any(feature = "rust-v1.81", feature = "std"),
    doc = r##"

[`prelude::Stashable`]: crate::prelude::Stashable
"##
)]
#[cfg_attr(
    not(any(feature = "rust-v1.81", feature = "std")),
    doc = r##"

[`prelude::Stashable`]: crate::surrogate_error_trait::prelude::Stashable
"##
)]
pub trait OrStash<S, I, T> {
    /// If `self` is `Result::Ok(value)`,
    /// returns the `Ok(value)` variant of [`StashedResult`];
    /// if `self` is `Result::Err(e)`,
    /// adds `e` to the provided [`ErrorStash`] or [`StashWithErrors`]
    /// and returns the `Err` variant [`StashedResult`].
    ///
    /// Use this method to collect an arbitrary number
    /// of `Result::Err` occurrences
    /// in an [`ErrorStash`] or a [`StashWithErrors`],
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
    /// #[cfg(any(feature = "rust-v1.81", feature = "std"))]
    /// use lazy_errors::prelude::*;
    ///
    /// #[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
    /// use lazy_errors::surrogate_error_trait::prelude::*;
    ///
    /// fn run() -> Result<(), Error> {
    ///     let mut stash = ErrorStash::new(|| "Failed to run application");
    ///
    ///     print_if_ascii("❓").or_stash(&mut stash);
    ///     print_if_ascii("❗").or_stash(&mut stash);
    ///     print_if_ascii("42").or_stash(&mut stash);
    ///
    ///     cleanup().or_stash(&mut stash); // Runs regardless of errors
    ///
    ///     stash.into() // Ok(()) if the stash is still empty
    /// }
    ///
    /// fn print_if_ascii(text: &str) -> Result<(), Error> {
    ///     if !text.is_ascii() {
    ///         return Err(err!("Input is not ASCII: '{text}'"));
    ///     }
    ///
    ///     println!("{text}");
    ///     Ok(())
    /// }
    ///
    /// fn cleanup() -> Result<(), Error> {
    ///     Err(err!("Cleanup failed"))
    /// }
    ///
    /// fn main() {
    ///     let err = run().unwrap_err();
    ///     let printed = format!("{err:#}");
    ///     let printed = replace_line_numbers(&printed);
    ///     assert_eq!(printed, indoc::indoc! {"
    ///         Failed to run application
    ///         - Input is not ASCII: '❓'
    ///           at src/or_stash.rs:1234:56
    ///           at src/or_stash.rs:1234:56
    ///         - Input is not ASCII: '❗'
    ///           at src/or_stash.rs:1234:56
    ///           at src/or_stash.rs:1234:56
    ///         - Cleanup failed
    ///           at src/or_stash.rs:1234:56
    ///           at src/or_stash.rs:1234:56"});
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
    ///
    /// Sometimes you don't want to create an empty [`ErrorStash`] beforehand.
    /// In that case you can call [`or_create_stash`] on `Result`
    /// to create a non-empty container on-demand, whenever necessary.
    ///
    /// [`ErrorStash`]: crate::ErrorStash
    /// [`or_create_stash`]: crate::OrCreateStash::or_create_stash
    fn or_stash(self, stash: &mut S) -> StashedResult<T, I>;
}

/// Similar to [`core::result::Result`], except that this type
/// is deliberately _not_ `#[must_use]`
/// and the `Err` type is more or less hardcoded.
///
/// Note that the error variant is [`&mut StashWithErrors`][StashWithErrors].
/// When `StashedResult` is returned from [`or_stash`],
/// it actually borrows the inner value from
/// the [`&mut ErrorStash`][crate::ErrorStash]
/// that was passed to [`or_stash`].
/// Thus, if you want to keep the results of multiple [`or_stash`] calls
/// around at the same time, in order to extract their `Ok(t)` values later,
/// you need to call [`StashedResult::ok`] on them.
/// Otherwise you'll get ownership-related compilation errors.
/// Check out [`StashedResult::ok`] for an example.
///
/// The reason we're keeping a reference to the [`StashWithErrors`] is
/// that it allows you to use the [`try2!`] macro
/// (and will probably allow you use the `?` operator in the future
/// when the `Try` trait is stabilized).
///
/// `StashedResult` is returned from [`or_stash`].
/// There should be no need to create values of this type manually.
///
/// [`ErrorStash`]: crate::ErrorStash
/// [`try2!`]: crate::try2!
/// [`or_stash`]: OrStash::or_stash
#[derive(Debug)]
pub enum StashedResult<'s, T, I> {
    Ok(T),
    Err(&'s mut StashWithErrors<I>),
}

impl<T, E, S, I> OrStash<S, I, T> for Result<T, E>
where
    E: Into<I>,
    S: ErrorSink<E, I>,
{
    #[track_caller]
    fn or_stash(self, stash: &mut S) -> StashedResult<T, I> {
        match self {
            Ok(v) => StashedResult::Ok(v),
            Err(err) => StashedResult::Err(stash.stash(err)),
        }
    }
}

impl<'s, T, E> StashedResult<'s, T, E> {
    /// Returns `Some(t)` if `self` is `Ok(t)`, `None` otherwise.
    ///
    /// This method is useful to discard the `&mut` borrowing of the
    /// [`ErrorStash`]/[`StashWithErrors`] that was passed as parameter
    /// to [`or_stash`].
    /// You may need to do this if you have multiple [`or_stash`] statements
    /// and want to extract the `Ok(t)` result from them later.
    /// For example, the following example would fail to compile
    /// without calling `ok` (due to borrowing `errs` mutably twice):
    ///
    /// ```
    /// # use core::str::FromStr;
    /// #[cfg(any(feature = "rust-v1.81", feature = "std"))]
    /// use lazy_errors::{prelude::*, Result};
    ///
    /// #[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
    /// use lazy_errors::surrogate_error_trait::{prelude::*, Result};
    ///
    /// fn parse_version(major: &str, minor: &str) -> Result<(u32, u32)> {
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
    ///
    /// [`ErrorStash`]: crate::ErrorStash
    /// [`or_stash`]: OrStash::or_stash
    pub fn ok(self) -> Option<T> {
        match self {
            StashedResult::Ok(t) => Some(t),
            StashedResult::Err(_) => None,
        }
    }
}
