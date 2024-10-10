# Changelog

This file documents all changes affecting the [semver] version of this project.

## New in this release

### Breaking Changes

- `push` now returns a value instead of none (i.e. `()`)
  - `StashWithErrors::push` returns a `&mut StashWithErrors` to `self`
  - `ErrorStash::push` returns a `&mut StashWithErrors` to
    the wrapped inner `StashWithErrors` value
  - Usually, this update does not require changes to your code,
    except in some cases where you need to drop the return value explicitly,
    for example in `match` statements
- `StashedResult`, when imported from any of the two preludes,
  now has its generic inner error type parameter hardcoded
  as the respective `Stashable` type from that prelude

### Added

- Added `try_collect_or_stash` on `Iterator<Item = Result<T, E>>`,
  which is similar to `try_collect` from the Rust standard library,
  except that it fails lazily (i.e. it does _not_ short-circuit)
  and moves all `Err` items into an error stash
- Added `stash_err` on `Iterator<Item = Result<T, E>>`,
  which turns an `Iterator<Item = Result<T, E>>` into an `Iterator<Item = T>`,
  moving any `E` item into an error stash as soon as it is encountered
- `StashedResult` now implements `Debug` if its type parameters do so too

## [`v0.8.0`] (2024-09-20)

### Breaking Changes

- Uses `core::error::Error` by default now (instead of `std::error::Error`)
  - Adds the `rust-v1.81` feature (enabled by default)
    because `core::error::Error` is stable only since Rust v1.81
  - Disables the `std` feature by default
    because it is not needed anymore since Rust v1.81
  - This is NOT a breaking change if you're using Rust v1.81 or later
  - You also DON'T need to change your code if you've been using `no_std`
    (i.e. types exported via the `surrogate_error_trait` module),
    regardless of the Rust toolchain version you're using
  - If you're using a Rust toolchain older than v1.81, please disable
    the `rust-v1.81` feature and either enable the `std` feature or use
    types from the `surrogate_error_trait` module

## [`v0.7.0`] (2024-07-09)

### Breaking Changes

- Replaces the optional `color_eyre` dependency with `eyre`.
  This fixes the build on some older Rust toolchain versions,
  which broke due to a new version of a transitive dependency.

## [`v0.6.0`] (2024-06-25)

This release comes with a few breaking changes.
By introducing these breaking changes, several unexpected compilation failures
that might have happened in certain edge cases should now be fixed in advance.
For example, you can now compile `lazy_errors` on any Rust version since 1.61
(depending on the set of enabled features).
Additionally, you and/or your dependencies can now use both the
`std` and the `no_std` feature set of `lazy_errors` simultaneously.
Not only have conflicts been resolved, but the different data types
are now compatible as well.

While these breaking changes may require some additional effort now,
they prepare you for either seamlessly switching to `core::error::Error`-based
errors when that Rust version reaches stable, or benefiting from
`lazy_errors` now being backwards compatible in that regard.

Here's the details:

### Breaking Changes

- Enable the `std` feature by default
  - If you have already been using the `std` feature, please _remove_
    the `std` feature declaration from your `Cargo.toml` file
    to make it future-proof
  - `std::error::Error` was moved to `core::error` on nightly.
    When `core::error::Error` is stable, the `std` feature will be removed
    from `lazy_errors` because we won't need to depend on `std` anymore at all.
  - The `error_in_core` feature will probably be part of Rust 1.81
- Require explicit opt-in for `#![no_std]` support
  - `lazy_errors::Reportable` (the surrogate `std::error::Error` trait) and
    associated type aliases are _not_ part of the _regular_ prelude anymore.
    If you need `#![no_std]` support,
    - simply import `lazy_errors::surrogate_error_trait::prelude::*`
      instead of the regular prelude, and
    - disable the `std` feature (`--no-default-features`).
  - This change avoids conflicts between `std` and `no_std` dependents:
    The old implementation of the `std` feature in `lazy_errors`
    violated cargo's “features should be additive” rule.
    When one of your dependencies depended on `lazy_errors` with `std` support,
    and another one depended on `lazy_errors` having `std` disabled,
    the dependencies may have failed to compile in some cases.
  - When `error_in_core` is part of stable Rust, you will be able to
    continue using the surrogate error trait (to support old Rust versions),
    and/or you will be able to set the feature flag that will enable
    `error_in_core` in `lazy_errors`.
- Require inner errors be `Sync` in `no_std` mode as well
  - Previously, these errors types did not need to implement `Sync`
  - Now, `std` and `no_std` mode have identical auto-trait bounds
  - Using the aliased types from the two preludes,
    this will allow you to put `no_std` errors into `std` stashes,
    and vice-versa
  - You can always specify your own aliases if your error types aren't `Sync`

### Added

- Add `ErrorData::children` and mark `ErrorData::childs` as deprecated
- Add the `try2!` macro to the prelude
- Hide “new” error types from `core` and `alloc` behind `rust-v*` feature flags
  (enabled by default)
- Support all Rust versions since Rust 1.77 (all features & combinations)
- Support all Rust versions since Rust 1.61 (by disabling some features)
- Document feature flags in top-level docs
- Clarify MSRV in top-level docs

### Fixed

- Fix and clarify several parts of the documentation

## [`v0.5.0`] (2024-06-11)

### Breaking Changes

- Hide the doctest line number helper better

### Added

- Add `ErrorStash::ok`
- Add Apache 2.0 license (project is now dual-licensed: MIT or Apache 2.0)
- Add contribution info to `README.md`
- Add rustdoc links and badges to `README.md`
- Add better rustdoc examples
- Add new generated README for v0.5.0 with new examples and links

## [`v0.4.0`] (2024-06-08)

### Breaking Changes

- Hardcode `StashWithErrors` as `E` in `StashedResult`

### Added

- Add the `try2!` macro (`?` operator on `StashedResult`)

## [`v0.3.0`] (2024-06-07)

### Added

- Add crate `keywords` and `categories`
- Add StashedResult::ok()

## [`v0.2.0`] (2024-06-07)

### Added

- Add `ErrorStash::into_result()`

## [`v0.1.1`] (2024-05-24)

### Fixed

- Remove `cargo readme` artifacts from `README.md`
- Add truncated parts to `README.md`

## [`v0.1.0`] (2024-05-23)

### Added

- Initial release

[`v0.8.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.8.0
[`v0.7.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.7.0
[`v0.6.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.6.0
[`v0.5.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.5.0
[`v0.4.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.4.0
[`v0.3.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.3.0
[`v0.2.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.2.0
[`v0.1.1`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.1.1
[`v0.1.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.1.0

[semver]: https://semver.org/spec/v2.0.0.html
