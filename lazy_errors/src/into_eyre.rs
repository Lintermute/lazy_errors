use core::fmt::Display;

use crate::{
    error::{AdHocError, Error, ErrorData, StashedErrors, WrappedError},
    stash::{ErrorStash, StashWithErrors},
};

/// Adds the [`into_eyre_result`](Self::into_eyre_result) method
/// on types that can be converted into `Result<T, E>`,
/// converting them into `Result<T, eyre::Report>` instead.
///
/// Do not implement this trait. Importing the trait is sufficient
/// due to blanket implementations. The trait is implemented on `R`
/// if `R` can be converted into `Result<_, E>` and
/// if `E` implements [`IntoEyreReport`].
pub trait IntoEyreResult<T, I>
where
    I: IntoEyreReport,
{
    /// Lossy conversion to return some type,
    /// for example a list of errors that may be empty,
    /// from functions returning [`eyre::Result`],
    /// i.e. `Result<_, E>` where `E` is [`eyre::Report`]:
    ///
    /// ```
    /// use eyre::WrapErr;
    /// use lazy_errors::prelude::*;
    ///
    /// fn parse(s: &str) -> eyre::Result<i32> {
    ///     use core::str::FromStr;
    ///     i32::from_str(s).wrap_err_with(|| format!("Not an i32: '{s}'"))
    /// }
    ///
    /// fn parse_all() -> eyre::Result<()> {
    ///     let mut stash = ErrorStash::new(|| "Failed to parse");
    ///
    ///     parse("🙈").or_stash(&mut stash);
    ///     parse("🙉").or_stash(&mut stash);
    ///     parse("🙊").or_stash(&mut stash);
    ///
    ///     stash.into_eyre_result()
    /// }
    ///
    /// let err: eyre::Report = parse_all().unwrap_err();
    /// let msg = format!("{err}");
    /// assert!(msg.contains("🙈"));
    /// assert!(msg.contains("🙉"));
    /// assert!(msg.contains("🙊"));
    /// ```
    ///
    /// Note: This method discards information because [`IntoEyreReport`]
    /// flattens the type into a single string
    /// that is then passed to [`eyre::eyre!`].
    ///
    /// In some cases, for example if you're using [`or_create_stash`],
    /// you may want to use [`IntoEyreReport`] instead.
    ///
    /// [`or_create_stash`]:
    /// crate::or_create_stash::OrCreateStash::or_create_stash
    fn into_eyre_result(self) -> Result<T, eyre::Report>;
}

/// Adds the [`into_eyre_report`](Self::into_eyre_report) method
/// on various error and error builder types.
///
/// Do not implement this trait. Importing the trait is sufficient.
/// due to blanket implementations. The trait is implemented on
/// [`StashWithErrors`] and on
/// `E` if `E` implements `core::error::Error + Send + Sync + 'a`.
pub trait IntoEyreReport {
    /// Lossy conversion to return some type, for example
    /// a non-empty list of one or more errors,
    /// from functions returning [`eyre::Result`],
    /// i.e. `Result<_, E>` where `E` is [`eyre::Report`].
    ///
    /// ```
    /// # use lazy_errors::doctest_line_num_helper as replace_line_numbers;
    /// use eyre::{bail, eyre};
    /// use lazy_errors::prelude::*;
    ///
    /// fn adhoc_error() -> eyre::Result<()> {
    ///     let err = Error::from_message("first() failed");
    ///     bail!(err.into_eyre_report());
    /// }
    ///
    /// let err: eyre::Report = adhoc_error().unwrap_err();
    /// let printed = format!("{err}"); // No pretty-printing required
    /// let printed = replace_line_numbers(&printed);
    /// assert_eq!(printed, indoc::indoc! {"
    ///     first() failed
    ///     at src/into_eyre.rs:1234:56"});
    ///
    /// fn wrapped_report() -> eyre::Result<()> {
    ///     let report = eyre!("This is an eyre::Report");
    ///     let err: Error = Error::wrap(report);
    ///     bail!(err.into_eyre_report());
    /// }
    ///
    /// let err: eyre::Report = wrapped_report().unwrap_err();
    /// let printed = format!("{err}"); // No pretty-printing required
    /// let printed = replace_line_numbers(&printed);
    /// assert_eq!(printed, indoc::indoc! {"
    ///     This is an eyre::Report
    ///     at src/into_eyre.rs:1234:56"});
    ///
    /// fn stashed_errors() -> eyre::Result<()> {
    ///     let mut stash = ErrorStash::new(|| "One or more things failed");
    ///
    ///     adhoc_error().or_stash(&mut stash);
    ///     wrapped_report().or_stash(&mut stash);
    ///
    ///     stash.into_eyre_result()
    /// }
    ///
    /// let err: eyre::Report = stashed_errors().unwrap_err();
    /// let printed = format!("{err}"); // No pretty-printing required
    /// let printed = replace_line_numbers(&printed);
    /// assert_eq!(printed, indoc::indoc! {"
    ///     One or more things failed
    ///     - first() failed
    ///       at src/into_eyre.rs:1234:56
    ///       at src/into_eyre.rs:1234:56
    ///     - This is an eyre::Report
    ///       at src/into_eyre.rs:1234:56
    ///       at src/into_eyre.rs:1234:56"});
    /// ```
    ///
    /// Note: This method discards information because it
    /// flattens the type into a single string
    /// that is then passed to [`eyre::eyre!`].
    ///
    /// In some cases, for example if you're using [`or_stash`],
    /// you may want to use [`IntoEyreResult`] instead.
    ///
    /// [`or_stash`]: crate::or_stash::OrStash::or_stash
    fn into_eyre_report(self) -> eyre::Report;
}

impl<F, M, I> IntoEyreResult<(), Error<I>> for ErrorStash<F, M, I>
where
    F: FnOnce() -> M,
    M: Display,
    Error<I>: IntoEyreReport,
{
    #[track_caller]
    fn into_eyre_result(self) -> Result<(), eyre::Report> {
        let result: Result<(), Error<I>> = self.into();
        result.map_err(IntoEyreReport::into_eyre_report)
    }
}

impl<I: Display> IntoEyreReport for StashWithErrors<I> {
    /// Flattens the error hierarchy into a single string
    /// that is then passed to [`eyre::eyre!`].
    ///
    /// TODO: Improve this adapter somehow, if this is even possible.
    /// `color_eyre::Section` adds `Report::error`,
    /// but that method is not suited for our purpose.
    /// Firstly, it takes the error by value.
    /// Secondly, there aren't any accessors for these errors.
    /// Thirdly, these errors are not printed when using `{:?}`,
    /// as opposed to the “regular” error causes added via `wrap_err`.
    /// If we used `Into<Box<dyn core::error::Error + Send + Sync + 'static>>`
    /// and return the [`eyre::Report`] from `main`, eyre would
    /// display the error using the regular, non-pretty-printed
    /// form and we won't see the full list of errors.
    #[track_caller]
    fn into_eyre_report(self) -> eyre::Report {
        let err = Error::<I>::from(self);
        eyre::eyre!(format!("{err:#}"))
    }
}

impl<I: Display> IntoEyreReport for Error<I> {
    fn into_eyre_report(self) -> eyre::Report {
        match self.into() {
            ErrorData::Stashed(inner) => inner.into_eyre_report(),
            ErrorData::Wrapped(inner) => inner.into_eyre_report(),
            ErrorData::AdHoc(inner) => inner.into_eyre_report(),
        }
    }
}

impl<I: Display> IntoEyreReport for StashedErrors<I> {
    fn into_eyre_report(self) -> eyre::Report {
        eyre::eyre!(format!("{self:#}"))
    }
}

impl<I: Display> IntoEyreReport for WrappedError<I> {
    fn into_eyre_report(self) -> eyre::Report {
        eyre::eyre!(format!("{self:#}"))
    }
}

impl IntoEyreReport for AdHocError {
    fn into_eyre_report(self) -> eyre::Report {
        eyre::eyre!(format!("{self:#}"))
    }
}
