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

use crate::Error;

/// Adds the [`or_wrap`](Self::or_wrap) method on `Result<_, E>`,
/// if `E` implements [`Into<I>`](crate::Error#inner-error-type-i).
///
/// Do not implement this trait.
/// Importing the trait is sufficient due to blanket implementations.
/// The trait is implemented on `Result<_, E>` if `E` implements `Into<I>`,
/// where `I` is the [_inner error type_](Error#inner-error-type-i),
/// typically [`Stashable`](crate::prelude::Stashable).
pub trait OrWrap<T, E>
{
    /// If `self` is `Result::Ok(value)`, returns `Result::Ok(value)`;
    /// if `self` is `Result::Err(e1)`, returns `Result::Err(e2)`
    /// where `e2` is an [`Error`] containing a [`WrappedError`]
    /// that will hold the original `e1` value.
    ///
    /// ```
    /// use lazy_errors::prelude::*;
    ///
    /// fn main()
    /// {
    ///     assert!(run(&["foo", "bar"]).is_ok());
    ///
    ///     let err = run(&["foo", "❌", "bar"]).unwrap_err();
    ///     let printed = format!("{err:#}");
    ///     let printed = lazy_errors::replace_line_numbers(&printed);
    ///     assert_eq!(printed, indoc::indoc! {"
    ///         ❌
    ///         at lazy_errors/src/or_wrap.rs:1234:56"});
    /// }
    ///
    /// fn run(tokens: &[&str]) -> Result<(), Error>
    /// {
    ///     all_ascii(tokens).or_wrap()
    /// }
    ///
    /// fn all_ascii(tokens: &[&str]) -> Result<(), String>
    /// {
    ///     match tokens.iter().find(|s| !s.is_ascii()) {
    ///         None => Ok(()),
    ///         Some(not_ascii) => Err(not_ascii.to_string()),
    ///     }
    /// }
    /// ```
    ///
    /// Please take a look at [`or_wrap_with`] if you'd like to
    /// provide some kind of context information when wrapping the error.
    ///
    /// [`WrappedError`]: crate::WrappedError
    /// [`or_wrap_with`]: crate::OrWrapWith::or_wrap_with
    fn or_wrap<I>(self) -> Result<T, Error<I>>
    where E: Into<I>;
}

impl<T, E> OrWrap<T, E> for Result<T, E>
{
    #[track_caller]
    fn or_wrap<I>(self) -> Result<T, Error<I>>
    where E: Into<I>
    {
        match self {
            Ok(t) => Ok(t),
            Err(inner) => Err(Error::wrap(inner)),
        }
    }
}
