use alloc::vec::Vec;

use crate::{
    err,
    stash::{EnforceErrors, ErrorSink},
    Error, OrStash, StashedResult,
};

/// Adds the [`try_map_or_stash`](Self::try_map_or_stash) method on
/// [`[T; _]`](array) and
/// [`[Result<T, E>; _]`](array)
/// if `E` implements [`Into<I>`](crate::Error#inner-error-type-i).
///
/// Do not implement this trait.
/// Importing the trait is sufficient due to blanket implementations.
/// The trait is implemented automatically if `E` implements `Into<I>`,
/// where `I` is the [_inner error type_](crate::Error#inner-error-type-i),
/// typically [`prelude::Stashable`].
#[cfg_attr(
    any(feature = "rust-v1.81", feature = "std"),
    doc = r##"

[`prelude::Stashable`]: crate::prelude::Stashable
"##
)]
#[cfg_attr(
    not(any(feature = "rust-v1.81", feature = "std")),
    doc = r##"

[`prelude::Stashable`]: crate::surrogate_error_trait::prelude::Stashable
"##
)]
pub trait TryMapOrStash<T, E, S, I, const N: usize>
where
    E: Into<I>,
{
    /// Counterpart to [`array::try_map`] from the Rust standard library
    /// that will _not_ short-circuit,
    /// but instead move all `Err` elements/results into an error stash.
    ///
    /// This method will touch _all_ elements of arrays
    /// of type `[T; _]` or `[Result<T, E>; _]`,
    /// mapping _each_ `T` or `Ok(T)` via the supplied mapping function.
    /// Each time a `Result::Err` element is encountered
    /// or an element is mapped to a `Result::Err` value,
    /// that error will be put into the supplied error stash.
    /// If there are one or more `Result::Err`s,
    /// this method will return a [`StashedResult::Err`]
    /// wrapping that error stash.
    /// Otherwise, this method will return a [`StashedResult::Ok`]
    /// containing an array of the mapped elements, in order.
    ///
    /// Here's an example using `[T; _]`:
    ///
    /// ```
    /// # use core::str::FromStr;
    /// #[cfg(any(feature = "rust-v1.81", feature = "std"))]
    /// use lazy_errors::{prelude::*, Result};
    ///
    /// #[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
    /// use lazy_errors::surrogate_error_trait::{prelude::*, Result};
    ///
    /// fn parse_each_u8(input: [&str; 2]) -> Result<[u8; 2]> {
    ///     let mut errs = ErrorStash::new(|| "Invalid input");
    ///
    ///     let numbers = input.try_map_or_stash(u8::from_str, &mut errs);
    ///     let numbers: [u8; 2] = try2!(numbers);
    ///     Ok(numbers)
    /// }
    ///
    /// let numbers = parse_each_u8(["42", "0"]).unwrap();
    /// let errors_1 = parse_each_u8(["X", "0"]).unwrap_err();
    /// let errors_2 = parse_each_u8(["X", "Y"]).unwrap_err();
    ///
    /// assert_eq!(numbers, [42, 0]);
    /// assert_eq!(errors_1.children().len(), 1);
    /// assert_eq!(errors_2.children().len(), 2);
    /// ```
    ///
    /// Here's a similar example using `[Result<T, E>; _]` instead:
    ///
    /// ```
    /// # use core::str::FromStr;
    /// #[cfg(any(feature = "rust-v1.81", feature = "std"))]
    /// use lazy_errors::{prelude::*, Result};
    ///
    /// #[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
    /// use lazy_errors::surrogate_error_trait::{prelude::*, Result};
    ///
    /// fn try_parse_each_u8(
    ///     input: [Result<&'static str, &'static str>; 2],
    /// ) -> Result<[u8; 2]> {
    ///     let mut errs = ErrorStash::new(|| "Invalid input");
    ///
    ///     let numbers = input.try_map_or_stash(u8::from_str, &mut errs);
    ///     let numbers: [u8; 2] = try2!(numbers);
    ///     Ok(numbers)
    /// }
    ///
    /// let numbers = try_parse_each_u8([Ok("42"), Ok("0")]).unwrap();
    /// let errors_1 = try_parse_each_u8([Err("42"), Ok("0")]).unwrap_err();
    /// let errors_2 = try_parse_each_u8([Err("42"), Ok("X")]).unwrap_err();
    ///
    /// assert_eq!(numbers, [42, 0]);
    /// assert_eq!(errors_1.children().len(), 1);
    /// assert_eq!(errors_2.children().len(), 2);
    /// ```
    ///
    /// Note that `Err` will only be returned
    /// if the array contains an `Err` element or
    /// if any element of the array gets mapped to an `Err` value.
    /// Errors that have been added to the error stash before
    /// calling `try_map_or_stash` will not be considered.
    /// You can call [`ErrorStash::ok`] if you want to bail
    /// in case of earlier errors as well:
    ///
    /// ```
    /// # use core::str::FromStr;
    /// #[cfg(any(feature = "rust-v1.81", feature = "std"))]
    /// use lazy_errors::{prelude::*, Result};
    ///
    /// #[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
    /// use lazy_errors::surrogate_error_trait::{prelude::*, Result};
    ///
    /// let mut errs = ErrorStash::new(|| "There were one or more errors");
    ///
    /// errs.push("Earlier error"); // Ignored in `try_map_or_stash`
    ///
    /// assert!(matches!(errs.ok(), StashedResult::Err(_)));
    ///
    /// let numbers: [&str; 1] = ["42"];
    /// let numbers = numbers.try_map_or_stash(u8::from_str, &mut errs);
    /// assert!(matches!(&numbers, StashedResult::Ok(_)));
    ///
    /// let numbers1 = numbers.ok().unwrap();
    ///
    /// let numbers: [Result<_>; 1] = [Ok("24")];
    /// let numbers = numbers.try_map_or_stash(u8::from_str, &mut errs);
    /// assert!(matches!(&numbers, StashedResult::Ok(_)));
    ///
    /// let numbers2 = numbers.ok().unwrap();
    ///
    /// assert_eq!(&numbers1, &[42]);
    /// assert_eq!(&numbers2, &[24]);
    ///
    /// assert!(matches!(errs.ok(), StashedResult::Err(_)));
    /// ```
    ///
    /// If you need to map and collect items of an
    /// [`Iterator<Item = Result<T, E>>`](Iterator),
    /// take a look at [`try_collect_or_stash`] and [`stash_err`].
    ///
    /// [`ErrorStash::ok`]: crate::ErrorStash::ok
    /// [`stash_err`]: crate::StashErr::stash_err
    /// [`try_collect_or_stash`]:
    /// crate::TryCollectOrStash::try_collect_or_stash
    fn try_map_or_stash<F, U>(
        self,
        f: F,
        stash: &mut S,
    ) -> StashedResult<[U; N], I>
    where
        F: FnMut(T) -> Result<U, E>;
}

impl<T, E, S, I, const N: usize> TryMapOrStash<T, E, S, I, N> for [T; N]
where
    E: Into<I>,
    S: ErrorSink<E, I>,
    S: EnforceErrors<I>,
    Error<I>: Into<I>,
    S: ErrorSink<Error<I>, I>,
{
    // Note that the `#[track_caller]` annotation on this method does not work
    // as long as `closure_track_caller` (#87417) is unstable.
    #[track_caller]
    fn try_map_or_stash<F, U>(
        self,
        f: F,
        stash: &mut S,
    ) -> StashedResult<[U; N], I>
    where
        F: FnMut(T) -> Result<U, E>,
        Result<U, E>: OrStash<S, I, U>,
    {
        let vec = filter_map_or_stash(self, f, stash);

        if vec.len() != N {
            // The stash "cannot" be empty now... unless in case of
            // weird `std::mem::take` shenanigans or API violations.
            return StashedResult::Err(stash.enforce_errors());
        }

        vec_try_into_or_stash(vec, stash)
    }
}

impl<T, E1, E2, S, I, const N: usize> TryMapOrStash<T, E2, S, I, N>
    for [Result<T, E1>; N]
where
    E1: Into<I>,
    E2: Into<I>,
    S: ErrorSink<E1, I>,
    S: ErrorSink<E2, I>,
    S: EnforceErrors<I>,
    Error<I>: Into<I>,
    S: ErrorSink<Error<I>, I>,
{
    // Note that the `#[track_caller]` annotation on this method does not work
    // as long as `closure_track_caller` (#87417) is unstable.
    #[track_caller]
    fn try_map_or_stash<F, U>(
        self,
        f: F,
        stash: &mut S,
    ) -> StashedResult<[U; N], I>
    where
        F: FnMut(T) -> Result<U, E2>,
        Result<U, E2>: OrStash<S, I, U>,
    {
        let vec = filter_map_ok_or_stash(self, f, stash);

        if vec.len() != N {
            // The stash "cannot" be empty now... unless in case of
            // weird `std::mem::take` shenanigans or API violations.
            return StashedResult::Err(stash.enforce_errors());
        }

        vec_try_into_or_stash(vec, stash)
    }
}

// Note that the `#[track_caller]` annotation on this method does not work
// as long as `closure_track_caller` (#87417) is unstable.
#[track_caller]
fn filter_map_or_stash<T, F, U, E, S, I, const N: usize>(
    array: [T; N],
    mut f: F,
    stash: &mut S,
) -> Vec<U>
where
    F: FnMut(T) -> Result<U, E>,
    Result<U, E>: OrStash<S, I, U>,
{
    array
        .into_iter()
        .filter_map(|t| f(t).or_stash(stash).ok())
        .collect()
}

// Note that the `#[track_caller]` annotation on this method does not work
// as long as `closure_track_caller` (#87417) is unstable.
#[track_caller]
fn filter_map_ok_or_stash<T, E1, F, U, E2, S, I, const N: usize>(
    array: [Result<T, E1>; N],
    mut f: F,
    stash: &mut S,
) -> Vec<U>
where
    E1: Into<I>,
    F: FnMut(T) -> Result<U, E2>,
    Result<U, E2>: OrStash<S, I, U>,
    S: ErrorSink<E1, I>,
{
    array
        .into_iter()
        .filter_map(|r| match r {
            Ok(t) => f(t).or_stash(stash).ok(),
            Err(e) => {
                stash.stash(e);
                None
            }
        })
        .collect()
}

fn vec_try_into_or_stash<T, S, I, const N: usize>(
    vec: Vec<T>,
    stash: &mut S,
) -> StashedResult<[T; N], I>
where
    Result<[T; N], Error<I>>: OrStash<S, I, [T; N]>,
{
    vec.try_into()
        .map_err(|_| err!("INTERNAL ERROR: Failed to convert vector to array"))
        .or_stash(stash)
}
