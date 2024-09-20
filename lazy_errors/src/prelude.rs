//! Exports traits and _aliased_ types to support the most common use-cases.
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
//! into a [`lazy_errors::Stashable`].
//! Also, using the `'static` bound for the trait object usually works fine.
//! Thus, `Stashable<'static>` is the [_inner error type_ `I`]
//! for all container type aliases exported by this prelude.
//! We also define and export [`Stashable`] as an alias for
//! `Stashable<'static>` for readability, ergonomics, and maintainability.
//!
//! If you want to use different inner error types, you can go ahead and use
//! the container and wrapper types from this library directly. In that case,
//! please check out [the example in the crate root documentation][CUSTOM].
//!
//! If you're using a Rust version older than v1.81 _and_
//! don't enable the `std` feature, you need to use
//! the [`surrogate_error_trait::prelude`] instead.
//!
//! [`lazy_errors::Error`]: crate::Error
//! [`lazy_errors::ErrorStash`]: crate::ErrorStash
//! [`lazy_errors::Stashable`]: crate::Stashable
//! [`surrogate_error_trait::prelude`]: crate::surrogate_error_trait::prelude
//! [_inner error type_ `I`]: crate::Error#inner-error-type-i
//! [CUSTOM]: crate#example-custom-error-types

pub use crate::{
    err, try2, OrCreateStash, OrStash, OrWrap, OrWrapWith, StashedResult,
};

#[cfg(feature = "eyre")]
pub use crate::{IntoEyreReport, IntoEyreResult};

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
/// importing [`lazy_errors::prelude::*`](module@self).
pub type AdHocError = crate::AdHocError;

/// Type alias for [`crate::Stashable`]
/// to use a `'static` bound for the boxed
/// [_inner error type_ `I`](crate::Error#inner-error-type-i) trait object,
/// as explained in [the module documentation](module@self).
pub type Stashable = crate::Stashable<'static>;
