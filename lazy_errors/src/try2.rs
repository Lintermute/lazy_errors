// Copyright (c) 2024 Andreas Waidler
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

/// Works like the `?` operator on [`StashedResult`] should.
///
/// Example:
///
/// ```
/// use lazy_errors::{prelude::*, try2, Result};
/// # use core::str::FromStr;
///
/// fn parse_version(s: &str) -> Result<(u32, u32)>
/// {
///     let mut errs = ErrorStash::new(|| "Invalid version");
///
///     // If `parts` does not contain exactly two elements, return right now.
///     let [major, minor] = try2!(s
///         .split('.')
///         .collect::<Vec<_>>()
///         .try_into()
///         .map_err(|_| { Error::from_message("Must have two parts") })
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
///     errs.into_result()?;
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
/// When the `Try` trait is stabilized, this method will be replaced
/// by the `?` operator.
///
/// Before Rust had the `?` operator, that behavior was implemented in
/// the [`try!`] macro. Currently, the `?` operator is being made more
/// generic: When the `Try` trait gets stabilized, we can implement
/// that trait on any of our types and the `?` operator “should just work”.
/// Meanwhile, this macro takes the place of the `?` ([`StashedResult`] only).
///
/// [`StashedResult`]: crate::StashedResult
#[macro_export]
macro_rules! try2 {
    ($expr:expr $(,)?) => {
        match $expr {
            $crate::StashedResult::Ok(val) => val,
            $crate::StashedResult::Err(errs) => {
                return core::result::Result::Err(errs.take().into());
            },
        }
    };
}
