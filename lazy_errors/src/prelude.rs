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
