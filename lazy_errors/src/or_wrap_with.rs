use core::fmt::Display;

use crate::Error;

/// Adds the [`or_wrap_with`](Self::or_wrap_with) method on `Result<_, E>`,
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
pub trait OrWrapWith<F, M, T, E>
where
    F: FnOnce() -> M,
    M: Display,
{
    /// If `self` is `Result::Ok(value)`, returns `Result::Ok(value)`;
    /// if `self` is `Result::Err(e1)`, returns `Result::Err(e2)`
    /// where `e2` is an [`Error`] containing a [`WrappedError`]
    /// that will hold the original `e1` value
    /// and annotates it with the message provided by the user.
    ///
    /// This method behaves identically to [`or_wrap`]
    /// except that you can pass some information
    /// about the context of the error.
    ///
    /// ```
    /// # use lazy_errors::doctest_line_num_helper as replace_line_numbers;
    /// #[cfg(any(feature = "rust-v1.81", feature = "std"))]
    /// use lazy_errors::prelude::*;
    ///
    /// #[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
    /// use lazy_errors::surrogate_error_trait::prelude::*;
    ///
    /// fn run(tokens: &[&str]) -> Result<(), Error> {
    ///     all_ascii(tokens).or_wrap_with(|| "Input is not ASCII")
    /// }
    ///
    /// fn all_ascii(tokens: &[&str]) -> Result<(), String> {
    ///     match tokens.iter().find(|s| !s.is_ascii()) {
    ///         None => Ok(()),
    ///         Some(not_ascii) => Err(not_ascii.to_string()),
    ///     }
    /// }
    ///
    /// fn main() {
    ///     assert!(run(&["foo", "bar"]).is_ok());
    ///
    ///     let err = run(&["foo", "❌", "bar"]).unwrap_err();
    ///     let printed = format!("{err:#}");
    ///     let printed = replace_line_numbers(&printed);
    ///     assert_eq!(printed, indoc::indoc! {"
    ///         Input is not ASCII: ❌
    ///         at src/or_wrap_with.rs:1234:56"});
    /// }
    /// ```
    ///
    /// Please take a look at [`or_wrap`] if you do not want to supply
    /// the informative message.
    ///
    /// [`WrappedError`]: crate::WrappedError
    /// [`or_wrap`]: crate::OrWrap::or_wrap
    fn or_wrap_with<I>(self, f: F) -> Result<T, Error<I>>
    where
        E: Into<I>;
}

impl<F, M, T, E> OrWrapWith<F, M, T, E> for Result<T, E>
where
    F: FnOnce() -> M,
    M: Display,
{
    #[track_caller]
    fn or_wrap_with<I>(self, f: F) -> Result<T, Error<I>>
    where
        E: Into<I>,
    {
        match self {
            Ok(t) => Ok(t),
            Err(inner) => Err(Error::wrap_with(inner, f())),
        }
    }
}
