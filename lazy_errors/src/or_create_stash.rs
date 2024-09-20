use crate::StashWithErrors;

/// Adds the [`or_create_stash`](Self::or_create_stash) method
/// on `Result<_, E>`,
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
pub trait OrCreateStash<F, M, T, E>
where
    F: FnOnce() -> M,
    M: core::fmt::Display,
{
    /// If `self` is `Result::Ok(value)`, returns `Result::Ok(value)`;
    /// if `self` is `Result::Err(e)`,  returns `Result::Err(errs)`
    /// where `errs` is a [`StashWithErrors`] that is described by
    /// the provided error summary message and that will contain
    /// `e` as its first element.
    ///
    /// Use this method to defer both handling errors as well as
    /// creating an [`ErrorStash`].
    /// In case the `Result` is `Result::Err(e)`, `or_create_stash` will
    /// create a [`StashWithErrors`] that contains `e` as its sole element.
    /// You can turn this stash into [`Error`] later.
    /// Meanwhile, you can run additional fallible functions,
    /// for example fallible cleanup steps.
    /// If those cleanup steps return errors as well, you can add them to
    /// the current error list by calling [`or_stash`].
    /// When you're done, you can return the entire error list in one go.
    ///
    /// ```
    /// # use lazy_errors::doctest_line_num_helper as replace_line_numbers;
    /// #[cfg(any(feature = "rust-v1.81", feature = "std"))]
    /// use lazy_errors::prelude::*;
    ///
    /// #[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
    /// use lazy_errors::surrogate_error_trait::prelude::*;
    ///
    /// fn write_or_cleanup(text: &str) -> Result<(), Error> {
    ///     match write(text).or_create_stash(|| "Failed to write") {
    ///         Ok(()) => Ok(()),
    ///         Err(mut stash) => {
    ///             write("Recovering...").or_stash(&mut stash);
    ///             cleanup().or_stash(&mut stash);
    ///             return Err(stash.into());
    ///         }
    ///     }
    /// }
    ///
    /// fn write(text: &str) -> Result<(), Error> {
    ///     if !text.is_ascii() {
    ///         return Err(err!("Input is not ASCII: '{text}'"));
    ///     }
    ///
    ///     Ok(())
    /// }
    ///
    /// fn cleanup() -> Result<(), Error> {
    ///     Err(err!("Cleanup failed"))
    /// }
    ///
    /// fn main() {
    ///     assert!(write_or_cleanup("ASCII text").is_ok());
    ///
    ///     let err = write_or_cleanup("❌").unwrap_err();
    ///     let printed = format!("{err:#}");
    ///     let printed = replace_line_numbers(&printed);
    ///     assert_eq!(printed, indoc::indoc! {"
    ///         Failed to write
    ///         - Input is not ASCII: '❌'
    ///           at src/or_create_stash.rs:1234:56
    ///           at src/or_create_stash.rs:1234:56
    ///         - Cleanup failed
    ///           at src/or_create_stash.rs:1234:56
    ///           at src/or_create_stash.rs:1234:56"});
    /// }
    /// ```
    ///
    /// Sometimes you want to create an empty [`ErrorStash`] beforehand,
    /// adding errors (if any) as you go.
    /// In that case, please take a look at [`ErrorStash`] and [`or_stash`].
    ///
    /// [`Error`]: crate::Error
    /// [`ErrorStash`]: crate::ErrorStash
    /// [`or_stash`]: crate::OrStash::or_stash
    /// [`or_create_stash`]: Self::or_create_stash
    fn or_create_stash<I>(self, f: F) -> Result<T, StashWithErrors<I>>
    where
        E: Into<I>;
}

impl<F, M, T, E> OrCreateStash<F, M, T, E> for Result<T, E>
where
    F: FnOnce() -> M,
    M: core::fmt::Display,
{
    #[track_caller]
    fn or_create_stash<I>(self, f: F) -> Result<T, StashWithErrors<I>>
    where
        E: Into<I>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(err) => Err(StashWithErrors::from(f(), err)),
        }
    }
}
