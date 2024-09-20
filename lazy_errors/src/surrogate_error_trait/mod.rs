//! Alternatives to types based on
//! `std::error::Error`/`core::error::Error`.
//!
//! When you're using the Rust v1.81 (or later)
//! or when you've enabled the `std` feature,
//! there should be no need to use anything from this module.
//! However,
//! in Rust versions before 1.81, `core::error::Error` is not available.
//! If you don't enable the `std` feature in that case,
//! `std::error::Error` won't be either.
//! Thus, you'd need an alternative to `lazy_errors::Stashable`.
//! [`Reportable`] is a surrogate for `std::error::Error`/`core::error::Error`
//! and [`lazy_errors::surrogate_error_trait::Stashable`] is for [`Reportable`]
//! what `lazy_errors::Stashable` is for `core::error::Error`.
//!
//! It's usually sufficient to import
//! [`lazy_errors::surrogate_error_trait::prelude::*`](prelude) and
//! [`lazy_errors::surrogate_error_trait::Result`](Result).
//!
//! [`lazy_errors::Error`]: crate::Error
//! [`lazy_errors::ErrorStash`]: crate::ErrorStash
//! [`lazy_errors::surrogate_error_trait::Stashable`]:
//! crate::surrogate_error_trait::Stashable

pub mod prelude;

use core::fmt::{Debug, Display};

use alloc::boxed::Box;

use crate::{AdHocError, Error, ErrorData, StashedErrors, WrappedError};

/// Marker trait for types that can be put into [`ErrorStash`]
/// and other containers of this crate
/// when both `std` and `core::error::Error` are not available.
///
/// By default, this trait is referenced in exactly one place: [`Stashable`].
/// By implementing this trait for your custom type, you will be able to
/// put that type into [`ErrorStash`] or other containers
/// (that use the boxed type aliases from the [`prelude`]),
/// without having to specify some static type parameters.
///
/// ```
/// use core::fmt::{Display, Formatter, Result};
/// use lazy_errors::surrogate_error_trait::{prelude::*, Reportable};
///
/// #[derive(Debug)]
/// struct MyType;
///
/// impl Display for MyType
/// {
///     fn fmt(&self, f: &mut Formatter<'_>) -> Result
///     {
///         write!(f, "MyType")
///     }
/// }
///
/// impl Reportable for MyType
/// {
/// }
///
/// let mut errs = ErrorStash::new(|| "Error summary");
/// errs.push(MyType);
/// ```
///
/// If you need a more complex conversion, you could instead implement
/// `From<MyType>` for `Box<dyn Reportable ...>` or for [`Stashable`].
/// As long as `MyType` itself does not implement `Reportable`
/// (there would be a conflicting implementation in that case),
/// implementing `From` will make `lazy_errors` convert your type
/// as expected when put into an [`ErrorStash`].
///
/// ```
/// use core::fmt::{Display, Formatter, Result};
/// use lazy_errors::surrogate_error_trait::{prelude::*, Reportable};
///
/// struct MyExpensiveType;
///
/// impl From<MyExpensiveType> for Stashable
/// {
///     fn from(val: MyExpensiveType) -> Stashable
///     {
///         Box::new(String::from("Summary of data in MyType"))
///         // Drop MyExpensiveType now, e.g. to free memory
///     }
/// }
///
/// let mut errs = ErrorStash::new(|| "Error summary");
/// errs.push(MyExpensiveType);
/// ```
///
/// [`ErrorStash`]: prelude::ErrorStash
/// [`Stashable`]: prelude::Stashable
pub trait Reportable: Display + Debug
{
}

/// Alias of the `Result<T, E>` we all know, but uses
/// [`lazy_errors::surrogate_error_trait::prelude::Error`]
/// as default value for `E` if not specified explitly.
///
/// [`lazy_errors::surrogate_error_trait::prelude::Error`]: prelude::Error
pub type Result<T, E = prelude::Error> = core::result::Result<T, E>;

/// The “default” [_inner error type_ `I`](crate::Error#inner-error-type-i)
/// used by the type aliases from the
/// [`surrogate_error_trait::prelude`](prelude)
/// _without_ `'static` lifetime.
///
/// This type is only used when you're using the type aliases from the
/// [`surrogate_error_trait::prelude`](prelude), which you probably
/// should only do when both `std` and `core::error::Error` are not available.
///
/// When both `std` and the `core::error::Error` trait are not available,
/// we need to fall back on some other trait.
/// We defined the [`Reportable`] trait for that purpose.
/// If you want to use this crate to handle custom error types,
/// you have to implement `Reportable` yourself (it's a one-liner).
///
/// The [`Send`] trait bound
/// [makes errors usable with `thread::spawn` and `task::spawn`][1].
///
/// The [`Sync`] trait bound is present because
/// `Stashable` from the “regular” prelude (`lazy_errors::prelude`)
/// needs the [`Sync`] bound itself.
/// By making the these two types share the same auto-trait bounds,
/// `lazy_errors` can be used identically in `std`/`no_std` configuration.
/// Furthermore, it allows you to put `no_std` errors into `std` stashes,
/// and vice-versa.
///
/// [1]: https://github.com/dtolnay/anyhow/issues/81
#[cfg_attr(
    any(feature = "rust-v1.81", feature = "std"),
    doc = r##"
```
use lazy_errors::prelude as lazy_errors_regular;
use lazy_errors::surrogate_error_trait::prelude as lazy_errors_surrogate;

let regular_error = lazy_errors_regular::Error::from_message("");
let surrogate_error = lazy_errors_surrogate::Error::from_message("");
let mut regular_stash = lazy_errors_regular::ErrorStash::new(|| "");
let mut surrogate_stash = lazy_errors_surrogate::ErrorStash::new(|| "");

regular_stash.push(surrogate_error);
surrogate_stash.push(regular_error);
```
"##
)]
/// Note that you can always define your own type aliases
/// that don't require your error types to be `Sync` or `Send`.
pub type Stashable<'a> =
    alloc::boxed::Box<dyn crate::Reportable + Send + Sync + 'a>;

/// Makes all [`Reportable`]s implement
/// `Into<Box<dyn Reportable>>`,
/// so that they satisfy the `E: Into<I>` constraint used throughout this crate.
impl<'a, E> From<E> for Box<dyn Reportable + 'a>
where E: Reportable + 'a
{
    fn from(val: E) -> Self
    {
        Box::new(val)
    }
}

/// Makes [`Reportable`]s implement
/// `Into<Box<dyn Reportable + Send>>` if possible,
/// so that they satisfy the `E: Into<I>` constraint used throughout this crate.
impl<'a, E> From<E> for Box<dyn Reportable + Send + 'a>
where E: Reportable + Send + 'a
{
    fn from(val: E) -> Self
    {
        Box::new(val)
    }
}

/// Makes [`Reportable`]s implement
/// `Into<Box<dyn Reportable + Sync>>` if possible,
/// so that they satisfy the `E: Into<I>` constraint used throughout this crate.
impl<'a, E> From<E> for Box<dyn Reportable + Sync + 'a>
where E: Reportable + Sync + 'a
{
    fn from(val: E) -> Self
    {
        Box::new(val)
    }
}

/// Makes [`Reportable`]s implement
/// `Into<Box<dyn Reportable + Send + Sync>>` if possible,
/// so that they satisfy the `E: Into<I>` constraint used throughout this crate.
impl<'a, E> From<E> for Box<dyn Reportable + Send + Sync + 'a>
where E: Reportable + Send + Sync + 'a
{
    fn from(val: E) -> Self
    {
        Box::new(val)
    }
}

impl<I> Reportable for Error<I> where I: Display + Debug
{
}

impl<I> Reportable for ErrorData<I> where I: Display + Debug
{
}

impl<I> Reportable for StashedErrors<I> where I: Display + Debug
{
}

impl<I> Reportable for WrappedError<I> where I: Display + Debug
{
}

impl Reportable for AdHocError
{
}

impl Reportable for alloc::string::String
{
}

impl Reportable for &str
{
}

impl Reportable for core::convert::Infallible
{
}

impl Reportable for core::alloc::LayoutError
{
}

impl Reportable for core::array::TryFromSliceError
{
}

impl Reportable for core::cell::BorrowError
{
}

impl Reportable for core::cell::BorrowMutError
{
}

impl Reportable for core::char::CharTryFromError
{
}

impl Reportable for core::char::DecodeUtf16Error
{
}

impl Reportable for core::char::ParseCharError
{
}

impl Reportable for core::char::TryFromCharError
{
}

impl Reportable for alloc::collections::TryReserveError
{
}

#[cfg(feature = "rust-v1.69")]
impl Reportable for core::ffi::FromBytesUntilNulError
{
}

#[cfg(feature = "rust-v1.64")]
impl Reportable for core::ffi::FromBytesWithNulError
{
}

#[cfg(feature = "rust-v1.64")]
impl Reportable for alloc::ffi::FromVecWithNulError
{
}

#[cfg(feature = "rust-v1.64")]
impl Reportable for alloc::ffi::IntoStringError
{
}

#[cfg(feature = "rust-v1.64")]
impl Reportable for alloc::ffi::NulError
{
}

impl Reportable for core::fmt::Error
{
}

#[cfg(feature = "rust-v1.77")]
impl Reportable for core::net::AddrParseError
{
}

impl Reportable for core::num::ParseFloatError
{
}

impl Reportable for core::num::ParseIntError
{
}

impl Reportable for core::num::TryFromIntError
{
}

impl Reportable for core::str::ParseBoolError
{
}

impl Reportable for core::str::Utf8Error
{
}

impl Reportable for alloc::string::FromUtf8Error
{
}

impl Reportable for alloc::string::FromUtf16Error
{
}

#[cfg(feature = "rust-v1.66")]
impl Reportable for core::time::TryFromFloatSecsError
{
}
