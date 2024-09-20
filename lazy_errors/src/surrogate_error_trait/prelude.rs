//! Exports traits and _aliased_ types to support the most common use-cases
//! (when neither `core::error::Error` nor `std` are available).
//!
//! In Rust versions before v1.81, `core::error::Error` is not stable.
//! In `#![no_std]` builds before Rust v1.81,
//! `std::error::Error` is not available either.
//! This prelude exports the surrogate error trait [`Reportable`] and
//! type aliases that can be used in these cases.
//! Consider using Rust v1.81 or newer, or (if that's not possible)
//! consider enabling the `std` feature. Doing either makes
//! the “regular” `lazy_errors::prelude::*` available.
//! Types exported from the “regular” prelude
//! are compatible with other crates.
//!
//! When using any container from `lazy_errors`, such as [`lazy_errors::Error`]
//! or [`lazy_errors::ErrorStash`], you usually don't want to specify the
//! [_inner error type_ `I`] explicitly.
//! This prelude exports type aliases for all types that otherwise need
//! the `I` parameter. The specific type used as `I` makes the aliased types
//! from the prelude well suited for the common
//! “just bail out now (or later)” use case.
//!
//! Usually, anything that you want to treat as an error can be boxed
//! into a `lazy_errors::Stashable`.
//! When `core::error::Error` is not available and
//! you don't want to introduce a dependency on `std`,
//! you need an alternative to `lazy_errors::Stashable`.
//! [`Reportable`] is a surrogate for `std::error::Error`/`core::error::Error`.
//! [`lazy_errors::surrogate_error_trait::Stashable`] is for
//! [`Reportable`] what `lazy_errors::Stashable` is for `core::error::Error`.
//! Also, using the `'static` bound for the trait object usually works fine.
//! Thus, `Stashable<'static>` is the [_inner error type_ `I`] for all
//! container type aliases exported by this prelude. We also define and export
//! [`Stashable`] as an alias for `Stashable<'static>` for
//! readability, ergonomics, and maintainability.
//!
//! If you want to use different inner error types, you can go ahead and use
//! the container and wrapper types from this library directly. In that case,
//! please check out [the example in the crate root documentation][CUSTOM].
//!
//! [`lazy_errors::Error`]: crate::Error
//! [`lazy_errors::ErrorStash`]: crate::ErrorStash
//! [`lazy_errors::surrogate_error_trait::Stashable`]:
//! crate::surrogate_error_trait::Stashable
//! [`Reportable`]: crate::surrogate_error_trait::Reportable
//! [_inner error type_ `I`]: crate::Error#inner-error-type-i
//! [CUSTOM]: crate#example-custom-error-types

pub use crate::{
    err,
    try2,
    OrCreateStash,
    OrStash,
    OrWrap,
    OrWrapWith,
    StashedResult,
};

/// Type alias for [`crate::ErrorStash`]
/// to use a boxed [_inner error type_ `I`](crate::Error#inner-error-type-i),
/// as explained in [the module documentation](module@self).
pub type ErrorStash<F, M> = crate::ErrorStash<F, M, Stashable>;

/// Type alias for [`crate::StashWithErrors`]
/// to use a boxed [_inner error type_ `I`](crate::Error#inner-error-type-i),
/// as explained in [the module documentation](module@self).
pub type StashWithErrors = crate::StashWithErrors<Stashable>;

/// Type alias for [`crate::Error`]
/// to use a boxed [_inner error type_ `I`](crate::Error#inner-error-type-i),
/// as explained in [the module documentation](module@self).
pub type Error = crate::Error<Stashable>;

/// Type alias for [`crate::ErrorData`]
/// to use a boxed [_inner error type_ `I`](crate::Error#inner-error-type-i),
/// as explained in [the module documentation](module@self).
pub type ErrorData = crate::ErrorData<Stashable>;

/// Type alias for [`crate::StashedErrors`]
/// to use a boxed [_inner error type_ `I`](crate::Error#inner-error-type-i),
/// as explained in [the module documentation](module@self).
pub type StashedErrors = crate::StashedErrors<Stashable>;

/// Type alias for [`crate::WrappedError`]
/// to use a boxed [_inner error type_ `I`](crate::Error#inner-error-type-i),
/// as explained in [the module documentation](module@self).
pub type WrappedError = crate::WrappedError<Stashable>;

/// Type alias for [`crate::AdHocError`] to get access to all error types by
/// importing [`lazy_errors::surrogate_error_trait::prelude::*`](module@self).
pub type AdHocError = crate::AdHocError;

/// Type alias for [`super::Stashable`]
/// to use a `'static` bound for the boxed
/// [_inner error type_ `I`](crate::Error#inner-error-type-i) trait object,
/// as explained in [the module documentation](module@self).
pub type Stashable = super::Stashable<'static>;
