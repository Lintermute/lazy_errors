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
