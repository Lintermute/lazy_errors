use crate::{
    stash::{EnforceErrors, ErrorSource},
    Error, OrStash, OrWrap, StashErr, StashedResult,
};

/// Adds the [`try_collect_or_stash`](Self::try_collect_or_stash) method on
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
pub trait TryCollectOrStash<T, E, S, I>
where
    E: Into<I>,
{
    /// Counterpart to [`Iterator::try_collect`] from the Rust standard library
    /// that will _not_ short-circuit,
    /// but instead move all `Err` items into an error stash.
    ///
    /// This method evaluates _all_ items in the [`Iterator`].
    /// Each time an `Err` value is encountered,
    /// it will be put into the supplied error stash
    /// and iteration will continue with the next item.
    ///
    /// This method will return a [`StashedResult::Ok`]
    /// containing a collection of all [`Result::Ok`] items.
    /// If there are one or more [`Result::Err`] items,
    /// all of them will be added to the supplied error stash, and
    /// this method will return a [`StashedResult::Err`]
    /// containing that error stash instead.
    ///
    /// ```
    /// # use core::str::FromStr;
    /// #[cfg(any(feature = "rust-v1.81", feature = "std"))]
    /// use lazy_errors::{prelude::*, Result};
    ///
    /// #[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
    /// use lazy_errors::surrogate_error_trait::{prelude::*, Result};
    ///
    /// fn parse_each_u8(tokens: &[&str]) -> Result<Vec<u8>> {
    ///     let mut errs = ErrorStash::new(|| "There were one or more errors");
    ///
    ///     let numbers: StashedResult<Vec<u8>> = tokens
    ///         .iter()
    ///         .map(|&s| u8::from_str(s))
    ///         .try_collect_or_stash(&mut errs);
    ///
    ///     let numbers: Vec<u8> = try2!(numbers);
    ///     Ok(numbers)
    /// }
    ///
    /// let empty = parse_each_u8(&[]).unwrap();
    /// let numbers = parse_each_u8(&["1", "42", "3"]).unwrap();
    /// let errors_1 = parse_each_u8(&["1", "X", "3"]).unwrap_err();
    /// let errors_2 = parse_each_u8(&["1", "X", "Y"]).unwrap_err();
    /// let errors_3 = parse_each_u8(&["X", "Y", "Z"]).unwrap_err();
    ///
    /// assert_eq!(&empty, &[]);
    /// assert_eq!(&numbers, &[1, 42, 3]);
    /// assert_eq!(errors_1.children().len(), 1);
    /// assert_eq!(errors_2.children().len(), 2);
    /// assert_eq!(errors_3.children().len(), 3);
    /// ```
    ///
    /// Note that `Err` will only be returned
    /// if the iterator contains an `Err` element.
    /// Errors that have been added to the error stash before
    /// calling `try_collect_or_stash` will not be considered.
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
    /// errs.push("Earlier error"); // Ignored in `try_collect_or_stash`
    ///
    /// assert!(matches!(errs.ok(), StashedResult::Err(_)));
    ///
    /// let numbers = ["42"]
    ///     .iter()
    ///     .map(|&s| u8::from_str(s))
    ///     .try_collect_or_stash::<Vec<u8>>(&mut errs);
    ///
    /// assert!(matches!(&numbers, StashedResult::Ok(_)));
    ///
    /// let numbers = numbers.ok().unwrap();
    /// assert_eq!(&numbers, &[42]);
    ///
    /// assert!(matches!(errs.ok(), StashedResult::Err(_)));
    /// ```
    ///
    /// If you need to transform an [`Iterator<Item = Result<T, E>>`](Iterator)
    /// into an `Iterator<Item = T>` and
    /// call a method _before_ collecting all `T` items,
    /// take a look at [`stash_err`](crate::StashErr::stash_err).
    ///
    /// If you want to map elements of a fixed-size array in a similar manner,
    /// take a look at [`try_map_or_stash`].
    ///
    /// [`ErrorStash::ok`]: crate::ErrorStash::ok
    /// [`try_map_or_stash`]: crate::TryMapOrStash::try_map_or_stash
    fn try_collect_or_stash<C>(self, stash: &mut S) -> StashedResult<C, I>
    where
        C: FromIterator<T>;
}

impl<Iter, T, E, S, I> TryCollectOrStash<T, E, S, I> for Iter
where
    Iter: Iterator<Item = Result<T, E>>,
    E: Into<I>,
    S: ErrorSource<I>,
    S: EnforceErrors<I>,
    Error<I>: Into<I>,
    Result<T, Error<I>>: OrStash<S, I, T>,
{
    // This method has no `#[track_caller]` annotation
    // because `stash_err` doesn't either.
    // If this method had a `#[track_caller]` annotation,
    // the backtrace would point to internals of the Rust standard library
    // instead of this file, making it even harder to understand.
    fn try_collect_or_stash<C>(self, stash: &mut S) -> StashedResult<C, I>
    where
        C: FromIterator<T>,
        Self: Sized,
    {
        let before = stash.errors().len();

        // Show this method in backtrace even despite `stash_err`
        // not supporting backtraces properly.
        let iter = self.map(|r| r.or_wrap());
        let result = iter.stash_err(stash).collect();

        let after = stash.errors().len();

        if before == after {
            StashedResult::Ok(result)
        } else {
            // The stash "cannot" be empty now... unless in case of
            // weird `std::mem::take` shenanigans or API violations.
            StashedResult::Err(stash.enforce_errors())
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use core::str::FromStr;

    #[cfg(any(feature = "rust-v1.81", feature = "std"))]
    use crate::{prelude::*, Result};

    #[cfg(not(any(feature = "rust-v1.81", feature = "std")))]
    use crate::surrogate_error_trait::{prelude::*, Result};

    /// Tests `try_collect_or_stash` with `StashWithErrors` as parameter.
    ///
    /// All other (doc) tests use `ErrorStash` as parameter instead
    /// because its the more common use-case.
    #[test]
    fn try_collect_or_stash_into_stash_with_errors() -> Result<()> {
        let mut errs = ErrorStash::new(|| "There were one or more errors");
        errs.push("Earlier error"); // Ignored in `try_collect_or_stash`

        let errs: &mut StashWithErrors = match errs.ok() {
            crate::StashedResult::Ok(_) => unreachable!(),
            crate::StashedResult::Err(stash_with_errors) => stash_with_errors,
        };

        let empty: Vec<Result<u8>> = vec![];
        let empty: Vec<u8> = try2!(empty
            .into_iter()
            .try_collect_or_stash(errs));
        assert_eq!(empty, &[]);

        let ok: Vec<Result<u8>> = vec![Ok(42)];
        let ok: Vec<u8> = try2!(ok
            .into_iter()
            .try_collect_or_stash(errs));
        assert_eq!(ok, &[42]);

        let err: Vec<Result<u8>> = vec![Err(err!("not a number"))];
        let err = err
            .into_iter()
            .try_collect_or_stash::<Vec<u8>>(errs);
        assert!(matches!(err, StashedResult::Err(_)));

        Ok(())
    }

    /// Ensures that all relevant methods have the `#[track_caller]` annotation
    /// and we're not losing the backtrace due to, e.g., calling a closure
    /// as long as feature `closure_track_caller` (#87417) is unstable.
    /// Also ensures that the `#[track_caller]` is missing from methods
    /// that would create misleading backtraces.
    #[test]
    fn try_collect_or_stash_has_correct_backtrace() {
        let mut errs = ErrorStash::new(|| "There were one or more errors");

        let _numbers = vec!["not a number"]
            .into_iter()
            .map(u8::from_str)
            .try_collect_or_stash::<Vec<u8>>(&mut errs);

        let err: Error = errs.into_result().unwrap_err();
        let msg = crate::doctest_line_num_helper(&format!("{err:#}"));
        assert_eq!(&msg, indoc::indoc! {"
            There were one or more errors
            - invalid digit found in string
              at src/try_collect_or_stash.rs:1234:56
              at src/stash_err.rs:1234:56"});
    }
}
