/// Works like the `?` operator on [`StashedResult`] should.
///
/// The [`try2!`] macro works well with [`or_stash`] and [`ErrorStash::ok`]:
///
/// ```
/// # use core::str::FromStr;
/// #[cfg(any(feature = "rust-v1.81", feature = "std"))]
/// use lazy_errors::{prelude::*, Result};
///
/// #[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
/// use lazy_errors::surrogate_error_trait::{prelude::*, Result};
///
/// fn parse_version(s: &str) -> Result<(u32, u32)> {
///     let mut errs = ErrorStash::new(|| "Invalid version");
///
///     // If `parts` does not contain exactly two elements, return right now.
///     let [major, minor]: [_; 2] = try2!(s
///         .split('.')
///         .collect::<Vec<_>>()
///         .try_into()
///         .map_err(|_| Error::from_message("Must have two parts"))
///         .or_stash(&mut errs));
///
///     // If we got exactly two parts, try to parse both of them,
///     // even if the first part already contains an error.
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
///     try2!(errs.ok());
///
///     // If the result above was `Ok`, all `ok()` calls returned `Some`.
///     Ok((major.unwrap(), minor.unwrap()))
/// }
///
/// assert_eq!(parse_version("42.1337").unwrap(), (42, 1337));
///
/// let err = parse_version("-1.-2.-3").unwrap_err();
/// assert_eq!(err.to_string(), "Invalid version: Must have two parts");
///
/// let err = parse_version("-1.-2").unwrap_err();
/// assert_eq!(err.to_string(), "Invalid version (2 errors)");
/// ```
///
/// When the `Try` trait is stabilized, this method will probably be replaced
/// by the `?` operator.
///
/// Before Rust had the `?` operator, that behavior was implemented in
/// the [`try!`] macro. Currently, the `?` operator is being made more
/// generic: When the `Try` trait gets stabilized, we can implement
/// that trait on any of our types and the `?` operator “should just work”.
/// Meanwhile, this macro takes the place of the `?` operator
/// (for [`StashedResult`] only).
///
/// [`ErrorStash::ok`]: crate::ErrorStash::ok
/// [`StashedResult`]: crate::StashedResult
/// [`or_stash`]: crate::OrStash::or_stash
/// [`try2!`]: crate::try2!
#[macro_export]
macro_rules! try2 {
    ($expr:expr $(,)?) => {
        match $expr {
            $crate::StashedResult::Ok(val) => val,
            $crate::StashedResult::Err(errs) => {
                return core::result::Result::Err(errs.take().into());
            }
        }
    };
}
