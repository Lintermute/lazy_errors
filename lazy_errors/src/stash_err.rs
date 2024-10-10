use core::marker::PhantomData;

use crate::{OrStash, StashedResult};

/// Adds the [`stash_err`](Self::stash_err) method on
/// [`Iterator<Item = Result<T, E>>`](Iterator)
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
pub trait StashErr<T, E, S, I>: Iterator<Item = Result<T, E>>
where
    E: Into<I>,
{
    /// Turns an [`Iterator<Item = Result<T, E>>`](Iterator)
    /// into an `Iterator<Item = T>`
    /// that will move any `E` item into an error stash
    /// as soon as it is encountered.
    ///
    /// ```
    /// # use core::str::FromStr;
    /// #[cfg(any(feature = "rust-v1.81", feature = "std"))]
    /// use lazy_errors::{prelude::*, Result};
    ///
    /// #[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
    /// use lazy_errors::surrogate_error_trait::{prelude::*, Result};
    ///
    /// fn parse_each_u8(tokens: &[&str]) -> (Vec<u8>, usize) {
    ///     let mut errs = ErrorStash::new(|| "There were one or more errors");
    ///
    ///     let numbers: Vec<u8> = tokens
    ///         .iter()
    ///         .map(|&s| u8::from_str(s))
    ///         .stash_err(&mut errs)
    ///         .collect();
    ///
    ///     let errors = match errs.into_result() {
    ///         Ok(()) => 0,
    ///         Err(e) => e.children().len(),
    ///     };
    ///
    ///     (numbers, errors)
    /// }
    ///
    /// assert_eq!(parse_each_u8(&[]), (vec![], 0));
    /// assert_eq!(parse_each_u8(&["1", "42", "3"]), (vec![1, 42, 3], 0));
    /// assert_eq!(parse_each_u8(&["1", "XX", "3"]), (vec![1, 3], 1));
    /// assert_eq!(parse_each_u8(&["1", "XX", "Y"]), (vec![1], 2));
    /// assert_eq!(parse_each_u8(&["X", "YY", "Z"]), (vec![], 3));
    /// ```
    ///
    /// [`stash_err`] is most useful for chaining another method, such as
    /// [`Iterator::filter`] or [`Iterator::map`],
    /// on the resulting `Iterator<Item = T>` before calling
    /// [`Iterator::collect`], [`Iterator::fold`], or a similar method.
    ///
    /// When using `stash_err` together with `collect`,
    /// there will be no indication of whether
    /// the iterator contained any `Err` items:
    /// all `Err` items will simply be moved into the error stash.
    /// If you don't need to chain any methods between calling
    /// `stash_err` and `collect`, or if
    /// you need `collect` to fail (lazily) if
    /// the iterator contained any `Err` items,
    /// you can call [`try_collect_or_stash`]
    /// on `Iterator<Item = Result<…>>` instead.
    ///
    /// [`stash_err`]: Self::stash_err
    /// [`try_collect_or_stash`]:
    /// crate::TryCollectOrStash::try_collect_or_stash
    fn stash_err(self, stash: &mut S) -> StashErrIter<Self, T, E, S, I>
    where
        Self: Sized,
    {
        StashErrIter {
            iter: self,
            stash,
            _unused: PhantomData,
        }
    }
}

impl<Iter, T, E, S, I> StashErr<T, E, S, I> for Iter
where
    Iter: Iterator<Item = Result<T, E>>,
    E: Into<I>,
{
}

/// An iterator that will turn a sequence of [`Result<T, E>`] items
/// into a sequence of `T` items,
/// moving any `Err` item into the supplied error stash.
///
/// Values of this type can be created by calling [`stash_err`] on
/// [`Iterator<Item = Result<T, E>>`](Iterator).
///
/// [`stash_err`]: StashErr::stash_err
pub struct StashErrIter<'a, Iter, T, E, S, I>
where
    Iter: Iterator<Item = Result<T, E>>,
{
    iter:    Iter,
    stash:   &'a mut S,
    _unused: PhantomData<I>,
}

impl<'a, Iter, T, E, S, I> Iterator for StashErrIter<'a, Iter, T, E, S, I>
where
    Iter: Iterator<Item = Result<T, E>>,
    Iter::Item: OrStash<S, I, T>,
    E: Into<I>,
{
    type Item = T;

    /// Moves all `Err` items of the underlying iterator into the error stash
    /// until an `Ok` value is encountered.
    /// As soon as `Ok(T)` is encountered, `Some(T)` will be returned.
    /// Returns `None` when the underlying iterator returns `None`.
    fn next(&mut self) -> Option<Self::Item> {
        // This method has no `#[track_caller]` annotation.
        // Thus, the backtrace will show the name of this file and
        // the location of this method within that file,
        // instead of the location where `stash_err`
        // (or a method like `collect`) was called.
        // If this method had a `#[track_caller]` annotation,
        // the backtrace would point to internals of the Rust standard library
        // instead of this file, making it even harder to understand.
        loop {
            match self.iter.next() {
                Some(result) => match result.or_stash(self.stash) {
                    StashedResult::Err(_) => continue,
                    StashedResult::Ok(t) => return Some(t),
                },
                None => return None,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use core::str::FromStr;

    use alloc::vec::Vec;

    #[cfg(any(feature = "rust-v1.81", feature = "std"))]
    use crate::prelude::*;

    #[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
    use crate::surrogate_error_trait::prelude::*;

    /// Ensures that all relevant methods have the `#[track_caller]` annotation
    /// and we're not losing the backtrace due to, e.g., calling a closure
    /// as long as feature `closure_track_caller` (#87417) is unstable.
    /// Also ensures that the `#[track_caller]` is missing from methods
    /// that would create misleading backtraces.
    #[test]
    fn stash_err_has_correct_backtrace() {
        let mut errs = ErrorStash::new(|| "There were one or more errors");

        let _: Vec<u8> = vec!["not a number"]
            .into_iter()
            .map(u8::from_str)
            .stash_err(&mut errs)
            .collect();

        let err: Error = errs.into_result().unwrap_err();
        let msg = crate::doctest_line_num_helper(&format!("{err:#}"));
        assert_eq!(&msg, indoc::indoc! {"
            There were one or more errors
            - invalid digit found in string
              at src/stash_err.rs:1234:56"});
    }
}
