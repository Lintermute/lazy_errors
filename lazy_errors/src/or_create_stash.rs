use crate::StashWithErrors;

/// Adds the [`or_create_stash`](Self::or_create_stash) method
/// on `Result<_, E>`,
/// if `E` implements [`Into<I>`](crate::Error#inner-error-type-i).
///
/// Do not implement this trait.
/// Importing the trait is sufficient due to blanket implementations.
/// The trait is implemented on `Result<_, E>` if `E` implements `Into<I>`,
/// where `I` is the [_inner error type_](crate::Error#inner-error-type-i),
/// typically [`Stashable`](crate::prelude::Stashable).
pub trait OrCreateStash<F, M, T, E>
where
    F: FnOnce() -> M,
    M: std::fmt::Display,
{
    /// If `self` is `Result::Ok(value)`, returns `Result::Ok(value)`;
    /// if `self` is `Result::Err(e)`,  returns `Result::Err(errs)`
    /// where `errs` is a [`StashWithErrors`] that is described by
    /// the provided error summary message and that will contain
    /// `e` as its first element.
    ///
    /// Used to create a list of one or more errors lazily, deferring
    /// error handling. For example, after some error occurred,
    /// you may want to run one or more fallible cleanup steps.
    /// If those cleanup steps return errors as well, you can add them to
    /// the current error list by calling this method.
    /// When you're done, you can return the entire error list in one go.
    ///
    /// ```
    /// use lazy_errors::prelude::*;
    ///
    /// fn main()
    /// {
    ///     assert!(write_or_cleanup("ASCII text").is_ok());
    ///
    ///     let err = write_or_cleanup("❌").unwrap_err();
    ///     let printed = format!("{err:#}");
    ///     let printed = lazy_errors::replace_line_numbers(&printed);
    ///     assert_eq!(printed, indoc::indoc! {"
    ///         Failed to write
    ///         - Input is not ASCII: '❌'
    ///           at lazy_errors/src/or_create_stash.rs:1234:56
    ///           at lazy_errors/src/or_create_stash.rs:1234:56
    ///         - Cleanup failed
    ///           at lazy_errors/src/or_create_stash.rs:1234:56
    ///           at lazy_errors/src/or_create_stash.rs:1234:56"});
    /// }
    ///
    /// fn write_or_cleanup(text: &str) -> Result<(), Error>
    /// {
    ///     match write(text).or_create_stash(|| "Failed to write") {
    ///         Ok(()) => Ok(()),
    ///         Err(mut stash) => {
    ///             write("Recovering...").or_stash(&mut stash);
    ///             cleanup().or_stash(&mut stash);
    ///             return Err(stash.into());
    ///         },
    ///     }
    /// }
    ///
    /// fn write(text: &str) -> Result<(), Error>
    /// {
    ///     if !text.is_ascii() {
    ///         return Err(err!("Input is not ASCII: '{text}'"));
    ///     }
    ///
    ///     Ok(())
    /// }
    ///
    /// fn cleanup() -> Result<(), Error>
    /// {
    ///     Err(err!("Cleanup failed"))
    /// }
    /// ```
    ///
    /// Sometimes you want to create an empty [`ErrorStash`] beforehand,
    /// adding errors (if any) as you go. Please take a look at [`or_stash`]
    /// in that case.
    ///
    /// [`ErrorStash`]: crate::ErrorStash
    /// [`or_stash`]: crate::OrStash::or_stash
    fn or_create_stash<I>(self, f: F) -> Result<T, StashWithErrors<I>>
    where E: Into<I>;
}

impl<F, M, T, E> OrCreateStash<F, M, T, E> for Result<T, E>
where
    F: FnOnce() -> M,
    M: std::fmt::Display,
{
    #[track_caller]
    fn or_create_stash<I>(self, f: F) -> Result<T, StashWithErrors<I>>
    where E: Into<I>
    {
        match self {
            Ok(v) => Ok(v),
            Err(err) => Err(StashWithErrors::from(f(), err)),
        }
    }
}
