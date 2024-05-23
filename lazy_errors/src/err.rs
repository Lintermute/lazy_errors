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
