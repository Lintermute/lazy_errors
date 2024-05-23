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

//! Exports commonly used traits and _aliased_ types of this crate.
//!
//! You usually don't want to specify the
//! [_inner error type_ `I`](crate::Error#inner-error-type-i) explicitly.
//! Usually, boxing the error into a [`Stashable`](crate::Stashable) is fine.
//! Also, using the `'static` bound for the trait object usually works fine.
//! Thus, this prelude exports type aliases accordingly.
//! If you want to use different inner error types, you can go ahead and use
//! the container and wrapper types from this library directly. In that case,
//! please check out the example in the [crate root documentation][crate].

#[cfg(not(feature = "std"))]
pub use crate::reportable::Reportable;
pub use crate::{
    boxed::*,
    err,
    OrCreateStash,
    OrStash,
    OrWrap,
    OrWrapWith,
    StashedResult,
};
#[cfg(feature = "eyre")]
pub use crate::{IntoEyreReport, IntoEyreResult};
