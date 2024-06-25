use alloc::{
    boxed::Box,
    fmt::{Debug, Display},
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    error::{self, Location},
    Error,
    StashedResult,
};

/// A builder for [`Error`] that keeps a list of errors
/// which may still be empty, along with a message that summarizes
/// all errors that end up in the list.
///
/// The generic type parameter `F` is a function or closure that
/// will create the error summary message lazily.
/// It will be called when the first error is added.
/// The generic type parameter `M` is the result returned from `F`,
/// i.e. the type of the error summary message itself.
/// The generic type parameter `I` is the
/// [_inner error type_ of `Error`](Error#inner-error-type-i).
///
/// Essentially, this type is a builder for something similar to
/// `Result<(), Vec<Error>>`. Errors can be added by calling
/// [`push`] or by calling [`or_stash`] on `Result`.
/// When you're done collecting the errors, the [`ErrorStash`] can be
/// transformed into `Result<(), Error>` (via [`From`]/[`Into`]),
/// where [`Error`] basically wraps a `Vec<E>`
/// along with a message that summarizes all errors in that list.
///
/// ```
/// # use lazy_errors::doctest_line_num_helper as replace_line_numbers;
/// #[cfg(feature = "std")]
/// use lazy_errors::prelude::*;
///
/// #[cfg(not(feature = "std"))]
/// use lazy_errors::surrogate_error_trait::prelude::*;
///
/// let errs = ErrorStash::new(|| "Something went wrong");
/// assert_eq!(&format!("{errs}"), "Stash of 0 errors currently");
/// let r: Result<(), Error> = errs.into();
/// assert!(r.is_ok());
///
/// let mut errs = ErrorStash::new(|| "Something went wrong");
/// errs.push("This is an error message");
/// assert_eq!(&format!("{errs}"), "Stash of 1 errors currently");
///
/// errs.push("Yet another error message");
/// assert_eq!(&format!("{errs}"), "Stash of 2 errors currently");
///
/// let r: Result<(), Error> = errs.into();
/// let err = r.unwrap_err();
///
/// assert_eq!(&format!("{err}"), "Something went wrong (2 errors)");
///
/// let printed = format!("{err:#}");
/// let printed = replace_line_numbers(&printed);
/// assert_eq!(printed, indoc::indoc! {"
///     Something went wrong
///     - This is an error message
///       at src/stash.rs:1234:56
///     - Yet another error message
///       at src/stash.rs:1234:56"});
/// ```
#[cfg_attr(
    feature = "eyre",
    doc = r##"

There's also [`IntoEyreResult`](crate::IntoEyreResult)
which performs a (lossy) conversion to
[`eyre::Result`](color_eyre::eyre::Result).

 "##
)]
/// If you do not want to create an empty [`ErrorStash`] before adding errors,
/// you can use [`or_create_stash`] which will
/// create a [`StashWithErrors`] when an error actually occurs.
///
/// [`or_stash`]: crate::OrStash::or_stash
/// [`or_create_stash`]: crate::OrCreateStash::or_create_stash
/// [`push`]: Self::push
pub enum ErrorStash<F, M, I>
where
    F: FnOnce() -> M,
    M: Display,
{
    Empty(F),
    WithErrors(StashWithErrors<I>),
}

/// A builder for [`Error`] that keeps a list of one or more errors,
/// along with a message that summarizes all errors that end up in the list.
///
/// The generic type parameter `I` is the
/// [_inner error type_ of `Error`](Error#inner-error-type-i).
///
/// This type is similar to [`ErrorStash`] except that an [`ErrorStash`]
/// may be empty. Since [`StashWithErrors`] contains at least one error,
/// guaranteed by the type system at compile time, this type implements
/// `Into<Error>`.
#[cfg_attr(
    feature = "eyre",
    doc = r##"

There's also [`IntoEyreReport`](crate::IntoEyreReport)
which performs a (lossy) conversion to
[`eyre::Report`](color_eyre::eyre::Report).
"##
)]
#[derive(Debug)]
pub struct StashWithErrors<I>
{
    summary:   Box<str>,
    errors:    Vec<I>,
    locations: Vec<Location>,
}

impl<F, M, I> Debug for ErrorStash<F, M, I>
where
    F: FnOnce() -> M,
    M: Display,
    I: Debug,
{
    fn fmt(&self, f: &mut alloc::fmt::Formatter<'_>) -> alloc::fmt::Result
    {
        match self {
            Self::Empty(_) => write!(f, "ErrorStash(Empty)"),
            Self::WithErrors(errs) => {
                write!(f, "ErrorStash(")?;
                Debug::fmt(errs, f)?;
                write!(f, ")")?;
                Ok(())
            },
        }
    }
}

impl<F, M, I> Display for ErrorStash<F, M, I>
where
    F: FnOnce() -> M,
    M: Display,
{
    fn fmt(&self, f: &mut alloc::fmt::Formatter<'_>) -> alloc::fmt::Result
    {
        match self {
            Self::Empty(_) => display::<I>(f, &[]),
            Self::WithErrors(errs) => Display::fmt(errs, f),
        }
    }
}

impl<I> Display for StashWithErrors<I>
{
    fn fmt(&self, f: &mut alloc::fmt::Formatter<'_>) -> alloc::fmt::Result
    {
        display(f, self.errors())
    }
}

impl<F, M, I> From<ErrorStash<F, M, I>> for Result<(), Error<I>>
where
    F: FnOnce() -> M,
    M: Display,
{
    fn from(stash: ErrorStash<F, M, I>) -> Self
    {
        match stash {
            ErrorStash::Empty(_) => Ok(()),
            ErrorStash::WithErrors(stash) => Err(stash.into()),
        }
    }
}

impl<I> From<StashWithErrors<I>> for Error<I>
{
    fn from(stash: StashWithErrors<I>) -> Self
    {
        Error::from_stash(stash.summary, stash.errors, stash.locations)
    }
}

impl<F, M, I> ErrorStash<F, M, I>
where
    F: FnOnce() -> M,
    M: Display,
{
    /// Creates a new [`ErrorStash`] with a “lazy” error summary message
    /// that will be evaluated when the first error (if any) is added
    /// to the stash.
    pub fn new(f: F) -> Self
    {
        Self::Empty(f)
    }

    /// Adds an error into the stash.
    #[track_caller]
    pub fn push<E>(&mut self, err: E)
    where E: Into<I>
    {
        // We need to move out of `&mut self`
        // because we want to call `f()` which is `FnOnce()`.

        let mut swap = Self::WithErrors(StashWithErrors {
            summary:   String::new().into_boxed_str(),
            errors:    vec![],
            locations: vec![],
        });

        core::mem::swap(self, &mut swap);

        let stash_with_errors = match swap {
            ErrorStash::Empty(f) => StashWithErrors::from(f(), err),
            ErrorStash::WithErrors(mut stash) => {
                stash.push(err);
                stash
            },
        };

        *self = ErrorStash::WithErrors(stash_with_errors);
    }

    /// Returns `true` if the stash is empty.
    ///
    /// ```
    /// #[cfg(feature = "std")]
    /// use lazy_errors::prelude::*;
    ///
    /// #[cfg(not(feature = "std"))]
    /// use lazy_errors::surrogate_error_trait::prelude::*;
    ///
    /// let mut errs = ErrorStash::new(|| "Summary message");
    /// assert!(errs.is_empty());
    ///
    /// errs.push("First error");
    /// assert!(!errs.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool
    {
        match self {
            ErrorStash::Empty(_) => true,
            ErrorStash::WithErrors(_) => false,
        }
    }

    /// Returns all errors that have been put into this stash so far.
    ///
    /// ```
    /// type ErrorStash<F, M> = lazy_errors::ErrorStash<F, M, i32>;
    ///
    /// let mut errs = ErrorStash::new(|| "Summary message");
    /// assert_eq!(errs.errors(), &[]);
    ///
    /// errs.push(42);
    /// errs.push(-1);
    /// errs.push(1337);
    /// assert_eq!(errs.errors(), &[42, -1, 1337]);
    /// ```
    ///
    /// Note that this method only returns errors that have been
    /// put into this stash _directly_.
    /// Each of those errors thus may have been created from
    /// an [`ErrorStash`](crate::ErrorStash),
    /// which stored another level of errors.
    /// Such transitive children will _not_ be returned from this method.
    pub fn errors(&self) -> &[I]
    {
        match self {
            ErrorStash::Empty(_) => &[],
            ErrorStash::WithErrors(stash) => stash.errors(),
        }
    }

    /// Returns `Ok(())` if the stash is empty,
    /// otherwise returns [`StashedResult::Err`].
    ///
    /// This method basically allows you to use the `?` operator
    /// (currently implemented in the form of the [`try2!`] macro)
    /// on _all_ prior errors simultaneously.
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// #[cfg(feature = "std")]
    /// use lazy_errors::{prelude::*, Result};
    ///
    /// #[cfg(not(feature = "std"))]
    /// use lazy_errors::surrogate_error_trait::{prelude::*, Result};
    ///
    /// // Always parses two configs, even if the first one contains an error.
    /// // All errors or groups of errors returned from this function
    /// // share the same error summary message.
    /// fn configure(
    ///     path_to_config_a: &str,
    ///     path_to_config_b: &str,
    /// ) -> Result<HashMap<String, String>>
    /// {
    ///     let mut errs = ErrorStash::new(|| "Invalid app config");
    ///
    ///     let config_a = parse_config(path_to_config_a)
    ///         .or_stash(&mut errs)
    ///         .ok();
    ///
    ///     let config_b = parse_config(path_to_config_b)
    ///         .or_stash(&mut errs)
    ///         .ok();
    ///
    ///     // If there was any error, bail out now.
    ///     // If there were no errors, both configs can be unwrapped.
    ///     try2!(errs.ok());
    ///     let config_a = config_a.unwrap();
    ///     let config_b = config_b.unwrap();
    ///
    ///     Ok(try2!(merge(config_a, config_b).or_stash(&mut errs)))
    /// }
    ///
    /// fn parse_config(path: &str) -> Result<HashMap<String, String>>
    /// {
    ///     if path == "bad.cfg" {
    ///         Err(err!("Config file contains an error"))
    ///     } else {
    ///         // ...
    ///         Ok(HashMap::new())
    ///     }
    /// }
    ///
    /// fn merge(
    ///     _a: HashMap<String, String>,
    ///     _b: HashMap<String, String>,
    /// ) -> Result<HashMap<String, String>>
    /// {
    ///     // ...
    ///     Ok(HashMap::new())
    /// }
    ///
    /// let err = configure("bad.cfg", "bad.cfg").unwrap_err();
    /// assert_eq!(err.children().len(), 2);
    ///
    /// let err = configure("good.cfg", "bad.cfg").unwrap_err();
    /// assert_eq!(err.children().len(), 1);
    ///
    /// assert!(configure("good.cfg", "good.cfg").is_ok());
    /// ```
    ///
    /// This method is similar to [`ErrorStash::into_result`] or
    /// `ErrorStash::into`. As opposed to these other methods, however,
    /// [`ok`] does not consume `self`. It only borrows `self` mutably.
    /// This allows you to continue adding errors later,
    /// as soon as you have dropped the [`StashedResult`]
    /// or called [`StashedResult::ok`] to discard the borrowed reference.
    ///
    /// This method enables you to place “barriers” in your code.
    /// Before the “barrier”, you can collect multiple errors.
    /// Then, at some pivotal check, you'll either return all previous errors
    /// or keep going, knowing that no errors have occurred so far.
    ///
    /// [`ErrorData::Stashed`]: crate::ErrorData::Stashed
    /// [`StashedErrors`]: crate::StashedErrors
    /// [`ok`]: Self::ok
    /// [`try2!`]: crate::try2!
    pub fn ok(&mut self) -> StashedResult<(), I>
    {
        match self {
            ErrorStash::Empty(_) => StashedResult::Ok(()),
            ErrorStash::WithErrors(errs) => StashedResult::Err(errs),
        }
    }

    /// Returns `Ok(())` if the stash is empty, otherwise returns an `Err`
    /// containing all errors from this stash.
    ///
    /// You can usually call `into` instead of this method.
    /// This method actually does nothing else besides specifying
    /// the return type. In some cases, Rust cannot figure out
    /// which type you want to convert into.
    /// This method may be more readable than specifying the concrete types:
    ///
    /// ```
    /// # use core::str::FromStr;
    /// #[cfg(feature = "std")]
    /// use lazy_errors::{prelude::*, Result};
    ///
    /// #[cfg(not(feature = "std"))]
    /// use lazy_errors::surrogate_error_trait::{prelude::*, Result};
    ///
    /// fn count_numbers(nums: &[&str]) -> Result<usize>
    /// {
    ///     let mut errs = ErrorStash::new(|| "Something wasn't a number");
    ///
    ///     for n in nums {
    ///         i32::from_str(n).or_stash(&mut errs);
    ///     }
    ///
    ///     // errs.into()?; // Does not compile
    ///     // Result::<()>::from(errs)?; // Works but is hard to read and type
    ///     errs.into_result()?; // Much nicer
    ///
    ///     Ok(nums.len())
    /// }
    ///
    /// assert_eq!(count_numbers(&["42"]).unwrap(), 1);
    /// assert!(count_numbers(&["42", ""]).is_err());
    /// ```
    ///
    /// In case there was at least one error in this stash,
    /// the [`Error`] will hold the [`ErrorData::Stashed`] variant
    /// which contains a [`StashedErrors`] object.
    ///
    /// [`ErrorData::Stashed`]: crate::ErrorData::Stashed
    /// [`StashedErrors`]: crate::StashedErrors
    pub fn into_result(self) -> Result<(), Error<I>>
    {
        self.into()
    }
}

impl<I> StashWithErrors<I>
{
    /// Creates a [`StashWithErrors`] that contains a single error so far;
    /// the supplied message shall summarize
    /// that error and all errors that will be added later.
    #[track_caller]
    pub fn from<M, E>(summary: M, error: E) -> Self
    where
        M: Display,
        E: Into<I>,
    {
        Self {
            summary:   summary.to_string().into(),
            errors:    vec![error.into()],
            locations: vec![error::location()],
        }
    }

    /// Adds an error into the stash.
    #[track_caller]
    pub fn push<E>(&mut self, err: E)
    where E: Into<I>
    {
        self.errors.push(err.into());
        self.locations.push(error::location());
    }

    /// Returns all errors that have been put into this stash so far.
    ///
    /// Note that this method only returns errors that have been
    /// put into this stash _directly_.
    /// Each of those errors thus may have been created from
    /// an [`ErrorStash`](crate::ErrorStash),
    /// which stored another level of errors.
    /// Such transitive children will _not_ be returned from this method.
    pub fn errors(&self) -> &[I]
    {
        &self.errors
    }

    /// ⚠️ Do not use this method! ⚠️
    ///
    /// Returns a [`StashWithErrors`] that's identical to `self`
    /// by replacing the contents of `&mut self` with dummy values.
    ///
    /// Do not call this method. It must only be used for internal purposes.
    /// This method is basically a wrapper for [`core::mem::swap`]
    /// that also handles the `I` type parameter.
    ///
    /// For internal usage only. Even then: Take care when using this method.
    /// Even if you have a `&mut`, you or your callers may not expect
    /// the value to change “that much”.
    /// This method should only be used by the [`try2!`] macro.
    /// When the `Try` trait is stabilized, we can implement it
    /// and remove the [`try2!`] macro and this method.
    ///
    /// ⚠️ Do not use this method! ⚠️
    ///
    /// [`try2!`]: crate::try2!
    #[doc(hidden)]
    pub fn take(&mut self) -> Self
    {
        // The dummy we'll be swapping into `self` should never “leak”,
        // if this type is used correctly.
        // But better print a specific error message in case it does.
        const WARNING: &str = "Internal error: Error info cleared by take()";

        let mut swap_with = Self {
            summary:   WARNING.to_string().into_boxed_str(),
            errors:    vec![],
            locations: vec![],
        };

        core::mem::swap(&mut swap_with, self);
        swap_with
    }
}

fn display<I>(
    f: &mut alloc::fmt::Formatter<'_>,
    errors: &[I],
) -> alloc::fmt::Result
{
    let count = errors.len();
    write!(f, "Stash of {count} errors currently")
}

#[cfg(test)]
mod tests
{
    use crate::{Error, ErrorStash};

    #[test]
    #[cfg(feature = "std")]
    fn stash_debug_fmt_when_empty_std()
    {
        use crate::prelude::Stashable;
        stash_debug_fmt_when_empty::<Stashable>()
    }

    #[test]
    fn stash_debug_fmt_when_empty_surrogate()
    {
        use crate::surrogate_error_trait::prelude::Stashable;
        stash_debug_fmt_when_empty::<Stashable>()
    }

    fn stash_debug_fmt_when_empty<I: std::fmt::Debug>()
    {
        let errs = ErrorStash::<_, _, I>::new(|| "Mock message");

        assert_eq!(format!("{errs:?}"), "ErrorStash(Empty)");
    }

    #[test]
    #[cfg(feature = "std")]
    fn stash_debug_fmt_with_errors_std()
    {
        use crate::prelude::Stashable;
        stash_debug_fmt_with_errors::<Stashable>()
    }

    #[test]
    fn stash_debug_fmt_with_errors_surrogate()
    {
        use crate::surrogate_error_trait::prelude::Stashable;
        stash_debug_fmt_with_errors::<Stashable>()
    }

    #[test]
    #[cfg(feature = "eyre")]
    fn stash_debug_fmt_with_errors_eyre()
    {
        use crate::prelude::ErrorStash;

        let mut errs = ErrorStash::new(|| "Mock message");

        errs.push(color_eyre::eyre::eyre!("Eyre error"));

        let msg = format!("{errs:?}");
        dbg!(&msg);

        assert!(msg.contains("Eyre error"));
        assert!(msg.contains("lazy_errors"));
        assert!(msg.contains("stash.rs"));
    }

    fn stash_debug_fmt_with_errors<'a, I>()
    where
        I: std::fmt::Debug,
        Error<I>: Into<I>,
        &'a str: Into<I>,
    {
        let mut errs = ErrorStash::<_, _, I>::new(|| "Mock message");
        errs.push("First error");
        errs.push(Error::<I>::from_message("Second error"));

        let msg = format!("{errs:?}");
        dbg!(&msg);

        assert!(msg.contains("ErrorStash"));
        assert!(msg.contains("StashWithErrors"));

        assert!(msg.contains("First error"));
        assert!(msg.contains("Second error"));

        assert!(msg.contains("lazy_errors"));
        assert!(msg.contains("stash.rs"));
    }
}
