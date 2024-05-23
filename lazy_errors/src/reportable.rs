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

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;
#[cfg(any(not(feature = "std"), doc))]
use core::fmt::{Debug, Display};

#[cfg(not(feature = "std"))]
use crate::{AdHocError, Error, ErrorData, StashedErrors, WrappedError};

/// Marker trait for types that can be put into [`ErrorStash`]
/// and other containers of this crate in `#![no_std]` builds.
///
/// This trait is only used in  `#![no_std]` mode. In that case,
/// it is referenced in exactly one place: [`Stashable`].
/// When you implement this trait for your custom type, you can
/// put that type into [`ErrorStash`] or other containers
/// (that use the boxed type aliases from the [`crate::prelude`]),
/// without having to specify some static type parameters.
///
/// [`ErrorStash`]: crate::ErrorStash
/// [`Stashable`]: crate::Stashable
#[cfg_attr(
    not(feature = "std"),
    doc = r##"
```

use core::fmt::{Display, Formatter, Result};

use lazy_errors::prelude::*;

#[derive(Debug)]
struct MyType;

impl Display for MyType
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result
    {
        write!(f, "MyType")
    }
}

impl lazy_errors::Reportable for MyType
{
}

let mut errs = ErrorStash::new(|| "Error summary");
errs.push(MyType);
```

"##
)]
/// If you need a more complex conversion, you could instead
/// implement `From<MyType>` for `Box<dyn Reportable ...>` or for `Stashable`.
/// As long as `MyType` itself does not implement `Reportable`
/// (there would be a conflicting implementation in that case),
/// implementing `From` will make `lazy_errors` convert your type
/// as expected when put into an [`ErrorStash`].
#[cfg_attr(
    not(feature = "std"),
    doc = r##"
```

use core::fmt::{Display, Formatter, Result};

use lazy_errors::prelude::*;

struct MyExpensiveType;

impl From<MyExpensiveType> for Stashable
{
    fn from(val: MyExpensiveType) -> Stashable
    {
        Box::new(String::from("Summary of data in MyType"))
        // Drop MyExpensiveType now, e.g. to free memory
    }
}

let mut errs = ErrorStash::new(|| "Error summary");
errs.push(MyExpensiveType);
```

"##
)]
#[cfg(any(not(feature = "std"), doc))]
pub trait Reportable: Display + Debug
{
}

/// Makes all [`Reportable`]s implement `Into<Box<dyn Reportable>>` so that
/// they satisfy the `E: Into<I>` constraint used throughout this crate.
#[cfg(not(feature = "std"))]
impl<'a, E> From<E> for Box<dyn Reportable + 'a>
where E: Reportable + 'a
{
    fn from(val: E) -> Self
    {
        Box::new(val)
    }
}

/// Makes all [`Reportable`]s implement `Into<Box<dyn Reportable>>` so that
/// they satisfy the `E: Into<I>` constraint used throughout this crate.
#[cfg(not(feature = "std"))]
impl<'a, E> From<E> for Box<dyn Reportable + Send + 'a>
where E: Reportable + Send + 'a
{
    fn from(val: E) -> Self
    {
        Box::new(val)
    }
}

/// Makes all [`Reportable`]s implement `Into<Box<dyn Reportable>>` so that
/// they satisfy the `E: Into<I>` constraint used throughout this crate.
#[cfg(not(feature = "std"))]
impl<'a, E> From<E> for Box<dyn Reportable + Sync + 'a>
where E: Reportable + Sync + 'a
{
    fn from(val: E) -> Self
    {
        Box::new(val)
    }
}

/// Makes all [`Reportable`]s implement `Into<Box<dyn Reportable>>` so that
/// they satisfy the `E: Into<I>` constraint used throughout this crate.
#[cfg(not(feature = "std"))]
impl<'a, E> From<E> for Box<dyn Reportable + Send + Sync + 'a>
where E: Reportable + Send + Sync + 'a
{
    fn from(val: E) -> Self
    {
        Box::new(val)
    }
}

#[cfg(not(feature = "std"))]
impl<I> Reportable for Error<I> where I: Display + Debug
{
}

#[cfg(not(feature = "std"))]
impl<I> Reportable for ErrorData<I> where I: Display + Debug
{
}

#[cfg(not(feature = "std"))]
impl<I> Reportable for StashedErrors<I> where I: Display + Debug
{
}

#[cfg(not(feature = "std"))]
impl<I> Reportable for WrappedError<I> where I: Display + Debug
{
}

#[cfg(not(feature = "std"))]
impl Reportable for AdHocError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for alloc::string::String
{
}

#[cfg(not(feature = "std"))]
impl Reportable for &str
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::convert::Infallible
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::alloc::LayoutError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::array::TryFromSliceError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::cell::BorrowError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::cell::BorrowMutError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::char::CharTryFromError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::char::DecodeUtf16Error
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::char::ParseCharError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::char::TryFromCharError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for alloc::collections::TryReserveError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::ffi::FromBytesUntilNulError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::ffi::FromBytesWithNulError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for alloc::ffi::FromVecWithNulError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for alloc::ffi::IntoStringError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for alloc::ffi::NulError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::fmt::Error
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::net::AddrParseError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::num::ParseFloatError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::num::ParseIntError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::num::TryFromIntError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::str::ParseBoolError
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::str::Utf8Error
{
}

#[cfg(not(feature = "std"))]
impl Reportable for alloc::string::FromUtf8Error
{
}

#[cfg(not(feature = "std"))]
impl Reportable for alloc::string::FromUtf16Error
{
}

#[cfg(not(feature = "std"))]
impl Reportable for core::time::TryFromFloatSecsError
{
}
