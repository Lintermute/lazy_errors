/// Creates an ad-hoc [`Error`](crate::Error)
/// from some message or format string.
///
/// Use this macro if you want to bail early from a function
/// that otherwise may return multiple errors
/// or when your codebase is generally using the
/// return type [`Result<_, prelude::Error>`](crate::Result).
///
/// A guard clause is a typical use case for this macro:
///
/// ```
/// use lazy_errors::prelude::*;
///
/// fn handle_ascii(text: &str) -> Result<(), Error>
/// {
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
        $crate::Error::from_message(std::format!($($arg)*))
    }};
}
