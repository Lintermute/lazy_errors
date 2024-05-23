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

use alloc::boxed::Box;

/// The “default” [_inner error type_ `I`](crate::Error#inner-error-type-i)
/// in `#![no_std]` builds.
///
/// This type is only used when you're using
/// the type aliases from the [crate::prelude].
///
/// In `#![no_std]` builds, `std::error::Error` is not available,
/// so we need to fall back on some other trait.
/// We defined the [`Reportable`] trait for that purpose.
/// If you want to use this crate to handle custom error types
/// in `#![no_std]` builds, you have to implement [`Reportable`] yourself
/// (it's a one-liner).
///
/// The [`Send`] trait bound
/// [makes errors usable with `thread::spawn` and `task::spawn`][1].
///
/// [1]: https://github.com/dtolnay/anyhow/issues/81
/// [`Reportable`]: crate::reportable::Reportable
#[cfg(not(feature = "std"))]
pub type Stashable<'a> = Box<dyn crate::Reportable + Send + 'a>;

/// The “default” [_inner error type_ `I`](crate::Error#inner-error-type-i)
/// if the `std` feature is enabled.
///
/// This type is only used when you're using
/// the type aliases from the [crate::prelude].
///
/// The trait bounds `Send` and `Sync` are present because they are
/// required by some third-party crates. Without `Send` and `Sync`,
/// these crates may not be able to consume error types from this crate,
/// such as [`Error`].
/// Note that you can always simply use a custom inner error type.
/// For example, in your codebase you could define `Stashable` instead
/// as `Box<dyn std::error::Error + 'static>` and set an alias for
/// [`Error<I>`] accordingly.
///
/// [`Error`]: crate::error::Error
/// [`Error<I>`]: crate::error::Error#inner-error-type-i
#[cfg(feature = "std")]
pub type Stashable<'a> = Box<dyn std::error::Error + Send + Sync + 'a>;
