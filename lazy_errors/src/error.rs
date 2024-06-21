use core::{
    fmt::{Debug, Display},
    ops::Deref,
};

use alloc::{boxed::Box, format, string::ToString};

pub type Location = &'static core::panic::Location<'static>;

/// The primary error type to use when using this crate.
///
/// [`Error`] is a [`Box`] that contains an [`ErrorData`] enum variant.
/// The enum wraps all kinds of errors
/// that you may want to return from functions.
/// [`Error`] boxes that enum to avoid introducing overhead on the
/// happy paths of functions returning `Result<_, Error>`.
/// You can access the [`ErrorData`] variants
/// in [`Error`] via [`Deref`], [`AsRef`], or [`From`]:
///
/// ```
/// # use core::str::FromStr;
/// #[cfg(feature = "std")]
/// use lazy_errors::prelude::*;
///
/// #[cfg(not(feature = "std"))]
/// use lazy_errors::surrogate_error_trait::prelude::*;
///
/// let err: Error = u32::from_str("").or_wrap().unwrap_err();
///
/// let deref: &ErrorData = &*err;
/// let asref: &ErrorData = err.as_ref();
/// let converted: ErrorData = err.into();
///
/// let err: WrappedError = match converted {
///     ErrorData::Wrapped(err) => err,
///     _ => unreachable!(),
/// };
/// ```
///
/// ### Inner Error Type `I`
///
/// The generic type parameter `I` is the _inner error type_.
/// It enables us to support a wide range of use cases.
/// In almost all trait implementations and method signatures
/// in this crate, errors will have the generic type parameter
/// `E: Into<I>`. This trait bound allows us to work with both
/// boxed errors as well as custom error types.
///
/// #### `Stashable`: Boxed Errors
///
/// Most of the time you will be using the _aliased_ re-export of [`Error`]
/// and other containers from the [`prelude`].
/// In that case, `I` will be [`prelude::Stashable`].
/// If the `std` feature is enabled, `prelude::Stashable` is an alias for
/// `Box<dyn std::error::Error + Send + Sync + 'static>`.
/// This trait bound was chosen because
/// `Into<Box<dyn std::error::Error + Send + Sync + 'static>>`
/// is implemented for many types via blanket implementations
/// in `std` and crates such as `anyhow` or `eyre`.
///
/// Some examples of types that satisfy this constraint are:
///
/// - `&str`
/// - `String`
/// - `anyhow::Error`
/// - `eyre::Report`
/// - `std::error::Error`
/// - All error types from this crate
///
/// Additionally, this trait bound makes `Error<I>` satisfy
/// `std::error::Error + Send + Sync + 'static` as well,
/// so it can be consumed by other crates
/// that support errors in general.
///
/// In `#![no_std]` builds, `std::error::Error` is not available.
/// Without the `error_in_core` feature, [`core::error::Error`] isn't either.
/// For these situations, `lazy_errors` comes with a surrogate error trait:
/// [`Reportable`]. `lazy_errors` implements [`Reportable`]
/// for error types in `core` and `alloc` as well as for `&str` and `String`.
/// `lazy_errors` also defines
/// [`surrogate_error_trait::prelude::Stashable`] as an alias for
/// `Box<dyn Reportable + Send + 'static>`
/// and exports additional type aliases in [`surrogate_error_trait::prelude`]
/// that allow you to use `lazy_errors` in `std`/`no_std` modes in the same way,
/// just by importing different preludes.
///
/// FYI, the [`Send`] trait bound
/// [makes errors usable with `thread::spawn` and `task::spawn`][1].
///
/// #### Using Custom Error Types
///
/// Usually, the inner error type `I` is [`prelude::Stashable`].
/// Nevertheless, you can choose the specific type to use for `I` arbitrarily:
///
/// ```
/// use lazy_errors::Error;
///
/// struct CustomError;
/// let error: Error<CustomError> = Error::wrap(CustomError);
/// ```
///
/// Note that you won't be able to print or debug-print errors
/// if the inner error type does not implement [`Display`]/[`Debug`].
/// On the other hand, such error types are completely unsupported by `eyre`:
///
/// ```compile_fail
/// # use color_eyre::eyre;
/// use eyre::Error;
///
/// struct CustomError;
/// let error: Error = Error::new(CustomError);
/// ```
///
/// `lazy_errors` should support any custom inner error type
/// as long as it is [`Sized`].
/// You can even choose a custom type to use for `I`
/// that doesn't implement [`Send`] or [`Sync`]:
///
/// ```
/// # extern crate alloc;
/// use alloc::rc::Rc;
/// use lazy_errors::Error;
///
/// struct NeitherSendNorSync(Rc<usize>);
/// let inner = NeitherSendNorSync(Rc::new(42));
/// let error: Error<NeitherSendNorSync> = Error::wrap(inner);
/// ```
///
/// Even if you implemented `Debug`, `Display`, and `std::error::Error`,
/// `eyre` still won't support your custom error type if it isn't
/// `Send + Sync + 'static`:
///
/// ```compile_fail
/// # extern crate alloc;
/// # use color_eyre::eyre;
/// use alloc::rc::Rc;
/// use eyre::eyre;
///
/// #[derive(Debug)]
/// struct NeitherSendNorSync(Rc<usize>);
///
/// impl std::fmt::Display for NeitherSendNorSync
/// {
///     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
///     {
///         let deref = &self.0;
///         write!(f, "{deref}")
///     }
/// }
///
/// impl std::error::Error for NeitherSendNorSync {}
///
/// let inner = NeitherSendNorSync(Rc::new(42));
/// let report = eyre!(inner);
/// ```
///
/// [1]: https://github.com/dtolnay/anyhow/issues/81
/// [`Reportable`]: crate::Reportable
/// [`surrogate_error_trait::prelude`]: crate::surrogate_error_trait::prelude
/// [`surrogate_error_trait::prelude::Stashable`]:
/// crate::surrogate_error_trait::prelude::Stashable
#[cfg_attr(
    feature = "std",
    doc = r##"
[`prelude`]: crate::prelude
[`prelude::Stashable`]: crate::prelude::Stashable
"##
)]
#[cfg_attr(
    not(feature = "std"),
    doc = r##"
[`prelude`]: crate::surrogate_error_trait::prelude
[`prelude::Stashable`]: crate::surrogate_error_trait::prelude::Stashable
"##
)]
#[derive(Debug)]
pub struct Error<I>(pub Box<ErrorData<I>>);

/// Enum of all kinds of errors that you may want to return
/// from functions when using this crate.
///
/// The main reason to use this crate is to return an arbitrary number
/// of errors from functions, i.e. `Result<_, StashedErrors>`,
/// where [`StashedErrors`] is basically a `Vec<E>`. At run-time,
/// however, you may want to return some other error,
/// for example when a guard clause evaluates to `false`
/// or when a preliminary check evaluated to `Err`.
/// In those cases, you can return an ad-hoc error
/// or wrap that other error. This enum distinguishes these three cases.
/// Since [`Error`] transparently wraps [`ErrorData`],
/// you can thus return `Result<_, Error>` in all of these cases.
#[derive(Debug)]
pub enum ErrorData<I>
{
    Stashed(StashedErrors<I>),
    Wrapped(WrappedError<I>),
    AdHoc(AdHocError),
}

/// A list of one or more errors along with a message
/// that summarizes all errors in the list.
///
/// Most of the time this type is used only internally.
///
/// Values of this type get created (internally)
/// by converting a (non-empty) [`ErrorStash`] into `Result`, or
/// by converting a [`StashWithErrors`] into [`Error`].
/// The error summary message will be set according to the parameter passed to
/// [`ErrorStash::new`] (or [`or_create_stash`], respectively).
///
/// [`ErrorStash`]: crate::ErrorStash
/// [`ErrorStash::new`]: crate::ErrorStash::new
/// [`StashWithErrors`]: crate::StashWithErrors
/// [`or_create_stash`]: crate::OrCreateStash::or_create_stash
#[derive(Debug)]
pub struct StashedErrors<I>
{
    /// Summarizes all errors in the list.
    summary: Box<str>,

    /// Guaranteed to contain at least one element.
    errors: Box<[I]>,

    /// Guaranteed to contain one element dedicated to each `errors` entry.
    locations: Box<[Location]>,
}

/// Wraps exactly one (custom or third-party) error, along with
/// an optional message that informs users or developers about
/// the context of the error.
///
/// Most of the time this type is used only internally.
///
/// Values of this type get created internally
/// by [`or_wrap`] and [`or_wrap_with`].
/// `WrappedError` can wrap any value that can be converted into the
/// [_inner error type_ `I`](Error#inner-error-type-i).
///
/// You can print and pretty-print the error,
/// regardless of whether the optional context message is present or not:
///
/// ```
/// # use lazy_errors::doctest_line_num_helper as replace_line_numbers;
/// # use core::str::FromStr;
/// #[cfg(feature = "std")]
/// use lazy_errors::prelude::*;
///
/// #[cfg(not(feature = "std"))]
/// use lazy_errors::surrogate_error_trait::prelude::*;
///
/// let err1: Error = u32::from_str("❌")
///     .or_wrap()
///     .unwrap_err();
///
/// let err2: Error = u32::from_str("❌")
///     .or_wrap_with(|| "Not an u32")
///     .unwrap_err();
///
/// let printed1 = format!("{err1}");
/// let printed2 = format!("{err2}");
/// assert_eq!(printed1, "invalid digit found in string");
/// assert_eq!(printed2, "Not an u32: invalid digit found in string");
///
/// let printed1 = format!("{err1:#}");
/// let printed1 = replace_line_numbers(&printed1);
/// assert_eq!(printed1, indoc::indoc! {"
///     invalid digit found in string
///     at lazy_errors/src/error.rs:1234:56"});
///
/// let printed2 = format!("{err2:#}");
/// let printed2 = replace_line_numbers(&printed2);
/// assert_eq!(printed2, indoc::indoc! {"
///     Not an u32: invalid digit found in string
///     at lazy_errors/src/error.rs:1234:56"});
/// ```
///
/// [`or_wrap`]: crate::OrWrap::or_wrap
/// [`or_wrap_with`]: crate::OrWrapWith::or_wrap_with
#[derive(Debug)]
pub struct WrappedError<I>
{
    context:  Option<Box<str>>,
    inner:    I,
    location: Location,
}

/// A single, “one of a kind” [`Error`], created from an ad-hoc error message,
/// with source location information that gets added implicitly
/// when a value of this type is constructed.
///
/// Most of the time this type is used only internally.
///
/// Values of this type get created internally
/// when the [`err!`](crate::err!) macro or
/// when [`Error::from_message`] are called.
///
/// `AdHocError` can be printed and supports “pretty-printing” as well:
///
/// ```
/// # use lazy_errors::doctest_line_num_helper as replace_line_numbers;
/// use lazy_errors::AdHocError;
///
/// let err = AdHocError::from_message("Something went wrong");
///
/// let printed = format!("{err}");
/// assert_eq!(printed, "Something went wrong");
///
/// let printed = format!("{err:#}");
/// let printed = replace_line_numbers(&printed);
/// assert_eq!(printed, indoc::indoc! {"
///     Something went wrong
///     at lazy_errors/src/error.rs:1234:56"});
/// ```
#[derive(Debug)]
pub struct AdHocError
{
    message:  Box<str>,
    location: Location,
}

impl<I> From<ErrorData<I>> for Error<I>
{
    fn from(value: ErrorData<I>) -> Self
    {
        Self(Box::new(value))
    }
}

impl<I> Deref for Error<I>
{
    type Target = ErrorData<I>;

    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}

impl<I> AsRef<ErrorData<I>> for Error<I>
{
    fn as_ref(&self) -> &ErrorData<I>
    {
        self.deref()
    }
}

impl<I> From<Error<I>> for ErrorData<I>
{
    fn from(value: Error<I>) -> Self
    {
        *value.0
    }
}

#[cfg(feature = "std")]
impl<I: Display + Debug> std::error::Error for Error<I>
{
}

#[cfg(feature = "std")]
impl<I: Display + Debug> std::error::Error for ErrorData<I>
{
}

#[cfg(feature = "std")]
impl<I: Display + Debug> std::error::Error for StashedErrors<I>
{
}

#[cfg(feature = "std")]
impl<I: Display + Debug> std::error::Error for WrappedError<I>
{
}

#[cfg(feature = "std")]
impl std::error::Error for AdHocError
{
}

impl<I: Display> Display for Error<I>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        Display::fmt(&self.0, f)
    }
}

impl<I: Display> Display for ErrorData<I>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        let i: &dyn Display = match self {
            Self::AdHoc(err) => err,
            Self::Stashed(errs) => errs,
            Self::Wrapped(inner) => inner,
        };

        if !f.alternate() {
            // `#` in format string
            write!(f, "{i}")
        } else {
            write!(f, "{i:#}")
        }
    }
}

impl<I: Display> Display for StashedErrors<I>
{
    /// Without additional flags, the output will be kept to a single line.
    /// To print the output as a list, pass the `#` “pretty-printing” sign.
    /// Doing so will also add source location information:
    ///
    /// ```
    /// # use lazy_errors::doctest_line_num_helper as replace_line_numbers;
    /// #[cfg(feature = "std")]
    /// use lazy_errors::prelude::*;
    ///
    /// #[cfg(not(feature = "std"))]
    /// use lazy_errors::surrogate_error_trait::prelude::*;
    ///
    /// let mut errs = ErrorStash::new(|| "Summary");
    /// errs.push("Foo");
    /// errs.push("Bar");
    ///
    /// let res: Result<(), Error> = errs.into();
    /// let err = res.unwrap_err();
    ///
    /// let printed = format!("{err}");
    /// assert_eq!(&printed, "Summary (2 errors)");
    ///
    /// let printed = format!("{err:#}");
    /// let printed = replace_line_numbers(&printed);
    /// assert_eq!(printed, indoc::indoc! {"
    ///     Summary
    ///     - Foo
    ///       at lazy_errors/src/error.rs:1234:56
    ///     - Bar
    ///       at lazy_errors/src/error.rs:1234:56"});
    /// ```
    ///
    /// When there is only a single error in a group, that error's output
    /// will be printed in the same line along with the “group” summary
    /// when printing the “short” form (without the “pretty-print” flag).
    ///
    /// ```
    /// # use lazy_errors::doctest_line_num_helper as replace_line_numbers;
    /// #[cfg(feature = "std")]
    /// use lazy_errors::{prelude::*, Result};
    ///
    /// #[cfg(not(feature = "std"))]
    /// use lazy_errors::surrogate_error_trait::{prelude::*, Result};
    ///
    /// fn run() -> Result<()>
    /// {
    ///     let mut stash = ErrorStash::new(|| "Parent failed");
    ///     stash.push(parent().unwrap_err());
    ///     stash.into()
    /// }
    ///
    /// fn parent() -> Result<()>
    /// {
    ///     let mut stash = ErrorStash::new(|| "Child failed");
    ///     stash.push(child().unwrap_err());
    ///     stash.into()
    /// }
    ///
    /// fn child() -> Result<(), &'static str>
    /// {
    ///     Err("Root cause")
    /// }
    ///
    /// let err = run().unwrap_err();
    ///
    /// let printed = format!("{err}");
    /// assert_eq!(printed, "Parent failed: Child failed: Root cause");
    ///
    /// let printed = format!("{err:#}");
    /// let printed = replace_line_numbers(&printed);
    /// assert_eq!(printed, indoc::indoc! {"
    ///     Parent failed
    ///     - Child failed
    ///       - Root cause
    ///         at lazy_errors/src/error.rs:1234:56
    ///       at lazy_errors/src/error.rs:1234:56"});
    /// ```
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        // TODO: Limit recursion depth for multiple sequences of
        // “groups” that each only consist of a single element.
        // Downcasting requires `'a: 'static`. Find an alternative.
        // `request_ref` from feature `error_generic_member_access`?
        // Maybe use the `f.precision()` format flag?

        // TODO: Print the source location in the same line as the error
        // when pretty-printing the list:
        // `format!("{indent}- {error} ({location})")`
        // This requires us to check whether `error` is of a type
        // defined in this crate and then handle it accordingly.
        // This will only work with casting; see comment above.

        let errors = self.errors.as_ref();
        let locations = self.locations.as_ref();
        let summary = &self.summary;
        let is_pretty = f.alternate(); // `#` in format string

        match (errors, locations, is_pretty) {
            ([], ..) => write!(f, "{summary}: 0 errors"),
            (_, [], ..) => write!(f, "{summary}: 0 source locations"),
            ([e], _, false) => write!(f, "{summary}: {e}"),
            (errs, _, false) => {
                write!(f, "{summary} ({} errors)", errs.len())
            },
            (errs, locs, true) => {
                write!(f, "{summary}")?;
                display_list_of_children(f, errs, locs)
            },
        }
    }
}

impl<I: Display> Display for WrappedError<I>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        let err = &self.inner;
        let loc = self.location;
        let is_pretty = f.alternate(); // `#` in format string

        match (&self.context, is_pretty) {
            (None, false) => write!(f, "{err}"),
            (None, true) => {
                write!(f, "{err:#}")?;

                // Note that the error may have printed its location already
                // in case it's an error type from our crate. In that case
                // we'd end up with duplicate locations. This is fine
                // as long as we're printing one location per line.
                display_location(f, "", loc)
            },
            (Some(context), false) => {
                // Refer to the note about recursion depth in `StashedErrors`.
                write!(f, "{context}: {err}")
            },
            (Some(context), true) => {
                // Refer to the note about recursion depth in `StashedErrors`.
                write!(f, "{context}: {err:#}")?;
                display_location(f, "", loc)
            },
        }
    }
}

impl Display for AdHocError
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        let is_pretty = f.alternate(); // `#` in format string
        if !is_pretty {
            write!(f, "{}", self.message)
        } else {
            writeln!(f, "{}", self.message)?;
            write!(f, "at {}", self.location)
        }
    }
}

impl<I> Error<I>
{
    /// Creates an [`AdHocError`] variant of [`Error`] from a message.
    #[track_caller]
    pub fn from_message<M: Display>(msg: M) -> Self
    {
        ErrorData::from_message(msg).into()
    }

    /// Creates a [`StashedErrors`] variant of [`Error`].
    pub fn from_stash<M, E, L>(summary: M, errors: E, locations: L) -> Self
    where
        M: Display,
        E: Into<Box<[I]>>,
        L: Into<Box<[Location]>>,
    {
        ErrorData::from_stash(summary, errors, locations).into()
    }

    /// Creates a [`WrappedError`] variant of [`Error`]
    /// from something that can be turned into an
    /// [_inner error type_ `I`](Error#inner-error-type-i).
    #[track_caller]
    pub fn wrap<E>(err: E) -> Self
    where E: Into<I>
    {
        ErrorData::wrap(err).into()
    }

    /// Creates a [`WrappedError`] variant of [`Error`]
    /// from something that can be turned into an
    /// [_inner error type_ `I`](Error#inner-error-type-i)
    /// and annotates it with an informative message.
    #[track_caller]
    pub fn wrap_with<E, M>(err: E, msg: M) -> Self
    where
        E: Into<I>,
        M: Display,
    {
        ErrorData::wrap_with(err, msg).into()
    }
}

impl<I> ErrorData<I>
{
    /// Creates an [`AdHocError`] variant of [`Error`] from a message.
    #[track_caller]
    pub fn from_message<M: Display>(msg: M) -> Self
    {
        let err = AdHocError::from_message(msg.to_string());
        Self::AdHoc(err)
    }

    /// Creates a [`StashedErrors`] variant of [`Error`].
    pub fn from_stash<M, E, L>(summary: M, errors: E, locations: L) -> Self
    where
        M: Display,
        E: Into<Box<[I]>>,
        L: Into<Box<[Location]>>,
    {
        let err = StashedErrors::from(summary, errors, locations);
        Self::Stashed(err)
    }

    /// Creates a [`WrappedError`] variant of [`Error`]
    /// from something that can be turned into an
    /// [_inner error type_ `I`](Error#inner-error-type-i).
    #[track_caller]
    pub fn wrap<E>(err: E) -> Self
    where E: Into<I>
    {
        Self::Wrapped(WrappedError::wrap(err))
    }

    /// Creates a [`WrappedError`] variant of [`Error`]
    /// from something that can be turned into an
    /// [_inner error type_ `I`](Error#inner-error-type-i)
    /// and annotates it with an informative message.
    #[track_caller]
    pub fn wrap_with<E, M>(err: E, msg: M) -> Self
    where
        E: Into<I>,
        M: Display,
    {
        Self::Wrapped(WrappedError::wrap_with(err, msg))
    }

    /// Deprecated method that was renamed to
    /// [`children`](Self::children).
    #[deprecated(since = "0.6.0", note = "renamed to `children`")]
    pub fn childs(&self) -> &[I]
    {
        self.children()
    }

    /// Returns all errors that are direct children of this error.
    ///
    /// ```
    /// #[cfg(feature = "std")]
    /// use lazy_errors::prelude::*;
    ///
    /// #[cfg(not(feature = "std"))]
    /// use lazy_errors::surrogate_error_trait::prelude::*;
    ///
    /// let err = Error::from_message("Something went wrong");
    /// assert!(err.children().is_empty());
    ///
    /// let err = Error::wrap("A thing went wrong");
    /// let [e] = err.children() else { unreachable!() };
    /// assert_eq!(&format!("{e}"), "A thing went wrong");
    ///
    /// let mut err = ErrorStash::new(|| "One or more things went wrong");
    /// err.push("An error");
    /// err.push("Another error");
    ///
    /// let r: Result<(), Error> = err.into();
    /// let err = r.unwrap_err();
    /// let [e1, e2] = err.children() else {
    ///     unreachable!()
    /// };
    ///
    /// assert_eq!(&format!("{e1}"), "An error");
    /// assert_eq!(&format!("{e2}"), "Another error");
    /// ```
    ///
    /// Note that this method only returns _direct_ children.
    /// Each of those errors thus may have been created from
    /// an [`ErrorStash`](crate::ErrorStash),
    /// which stored another level of errors.
    /// Such transitive children will _not_ be returned from this method.
    pub fn children(&self) -> &[I]
    {
        match self {
            Self::AdHoc(_) => &[],
            Self::Wrapped(err) => core::slice::from_ref(err.inner()),
            Self::Stashed(errs) => errs.errors(),
        }
    }
}

impl<I> StashedErrors<I>
{
    pub fn from<M, E, L>(summary: M, errors: E, locations: L) -> Self
    where
        M: Display,
        E: Into<Box<[I]>>,
        L: Into<Box<[Location]>>,
    {
        Self {
            summary:   summary.to_string().into_boxed_str(),
            errors:    errors.into(),
            locations: locations.into(),
        }
    }

    pub fn errors(&self) -> &[I]
    {
        &self.errors
    }
}

impl<I> WrappedError<I>
{
    /// Creates a [`WrappedError`]
    /// from something that can be turned into an
    /// [_inner error type_ `I`](Error#inner-error-type-i).
    #[track_caller]
    pub fn wrap<E>(err: E) -> Self
    where E: Into<I>
    {
        Self {
            context:  None,
            inner:    err.into(),
            location: location(),
        }
    }

    /// Creates a [`WrappedError`]
    /// from something that can be turned into an
    /// [_inner error type_ `I`](Error#inner-error-type-i)
    /// and annotates it with an informative message.
    #[track_caller]
    pub fn wrap_with<E, M>(err: E, msg: M) -> Self
    where
        E: Into<I>,
        M: Display,
    {
        Self {
            context:  Some(msg.to_string().into_boxed_str()),
            inner:    err.into(),
            location: location(),
        }
    }

    /// Return the error that was wrapped.
    pub fn inner(&self) -> &I
    {
        &self.inner
    }
}

impl AdHocError
{
    /// Creates an [`AdHocError`] from a message.
    #[track_caller]
    pub fn from_message<M: Display>(msg: M) -> Self
    {
        Self {
            message:  msg.to_string().into_boxed_str(),
            location: location(),
        }
    }
}

#[track_caller]
pub fn location() -> Location
{
    core::panic::Location::caller()
}

fn display_list_of_children<I: Display>(
    f: &mut core::fmt::Formatter<'_>,
    errs: &[I],
    locs: &[Location],
) -> core::fmt::Result
{
    for (e, l) in errs.iter().zip(locs) {
        display_multiline(f, &e)?;
        display_location(f, "  ", l)?;
    }
    Ok(())
}

fn display_multiline<I: Display>(
    f: &mut core::fmt::Formatter<'_>,
    err: &I,
) -> core::fmt::Result
{
    let mut prefix = "- ";
    for line in format!("{err:#}").lines() {
        writeln!(f)?;
        write!(f, "{prefix}{line}")?;
        prefix = "  ";
    }
    Ok(())
}

fn display_location(
    f: &mut core::fmt::Formatter<'_>,
    indent: &str,
    location: Location,
) -> core::fmt::Result
{
    writeln!(f)?;
    write!(f, "{indent}at {location}")
}

#[cfg(test)]
mod tests
{
    #[test]
    #[cfg(feature = "std")]
    fn error_is_small_std()
    {
        use crate::prelude::Stashable;
        assert_small::<super::Error<Stashable>>();
    }

    #[test]
    fn error_is_small_surrogate()
    {
        use crate::surrogate_error_trait::prelude::Stashable;
        assert_small::<super::Error<Stashable>>();
    }

    fn assert_small<T>()
    {
        use core::mem::size_of;
        assert_eq!(size_of::<T>(), size_of::<usize>());
    }
}
