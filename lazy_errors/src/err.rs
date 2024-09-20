/// Creates an ad-hoc [`Error`](crate::Error)
/// from some message or format string.
///
/// Use this macro if you want to bail early from a function
/// that otherwise may return multiple errors
/// or when your codebase is generally using the
/// return type
#[cfg_attr(
    any(feature = "rust-v1.81", feature = "std"),
    doc = r##"
[`lazy_errors::Result`](crate::Result)
or
[`lazy_errors::surrogate_error_trait::Result`].

 "##
)]
#[cfg_attr(
    not(any(feature = "rust-v1.81", feature = "std")),
    doc = r##"
[`lazy_errors::surrogate_error_trait::Result`]
(or `lazy_errors::Result` if any of
the `rust-v1.81` or
the `std` features is enabled).

 "##
)]
/// [`lazy_errors::surrogate_error_trait::Result`]:
/// crate::surrogate_error_trait::Result
///
/// A guard clause is a typical use case for this macro:
///
/// ```
/// #[cfg(any(feature = "rust-v1.81", feature = "std"))]
/// use lazy_errors::{err, Result};
///
/// #[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
/// use lazy_errors::{err, surrogate_error_trait::Result};
///
/// fn handle_ascii(text: &str) -> Result<()> {
///     if !text.is_ascii() {
///         return Err(err!("Not ASCII: '{text}'"));
///     }
///
///     // ... code that does the actual handling ...
///     todo!()
/// }
///
/// assert!(handle_ascii("🦀").is_err());
/// ```
#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => {{
        extern crate alloc;
        $crate::Error::from_message(alloc::format!($($arg)*))
    }};
}
