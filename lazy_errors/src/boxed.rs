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

//! Exports type aliases for all container types of this crate
//! to use a boxed [_inner error type_ `I`](crate::Error#inner-error-type-i),
//! making the API very versatile for the common use case.
//!
//! Usually, boxing any error into a [`Stashable`](crate::Stashable),
//! makes the types of this crate well suited for the common
//! “fire and forget” use case.
//! Also, using the `'static` bound for the trait object usually works fine.
//! Thus, we export these type aliases here accordingly.
//!
//! These aliases are usually imported implicitly via the [crate::prelude].

/// Type alias for [`crate::ErrorStash`]
/// to use a boxed [_inner error type_ `I`](crate::Error#inner-error-type-i),
/// as explained in [the module documentation](crate::boxed).
pub type ErrorStash<F, M> = crate::ErrorStash<F, M, Stashable>;

/// Type alias for [`crate::StashWithErrors`]
/// to use a boxed [_inner error type_ `I`](crate::Error#inner-error-type-i),
/// as explained in [the module documentation](crate::boxed).
pub type StashWithErrors = crate::StashWithErrors<Stashable>;

/// Type alias for [`crate::Error`]
/// to use a boxed [_inner error type_ `I`](crate::Error#inner-error-type-i),
/// as explained in [the module documentation](crate::boxed).
pub type Error = crate::Error<Stashable>;

/// Type alias for [`crate::ErrorData`]
/// to use a boxed [_inner error type_ `I`](crate::Error#inner-error-type-i),
/// as explained in [the module documentation](crate::boxed).
pub type ErrorData = crate::ErrorData<Stashable>;

/// Type alias for [`crate::StashedErrors`]
/// to use a boxed [_inner error type_ `I`](crate::Error#inner-error-type-i),
/// as explained in [the module documentation](crate::boxed).
pub type StashedErrors = crate::StashedErrors<Stashable>;

/// Type alias for [`crate::WrappedError`]
/// to use a boxed [_inner error type_ `I`](crate::Error#inner-error-type-i),
/// as explained in [the module documentation](crate::boxed).
pub type WrappedError = crate::WrappedError<Stashable>;

/// Type alias for [`crate::AdHocError`] to allow [`crate::prelude`]
/// get access to all error types by importing `boxed::*`.
pub type AdHocError = crate::AdHocError;

/// Type alias for [`crate::Stashable`]
/// to use a `'static` bound for the boxed
/// [_inner error type_ `I`](crate::Error#inner-error-type-i) trait object,
/// as explained in [the module documentation](crate::boxed).
pub type Stashable = crate::Stashable<'static>;
