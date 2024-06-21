# Changelog

This file documents all changes affecting the [semver] version of this project.

## New in this release

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

[`v0.5.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.5.0
[`v0.4.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.4.0
[`v0.3.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.3.0
[`v0.2.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.2.0
[`v0.1.1`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.1.1
[`v0.1.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.1.0

[semver]: https://semver.org/spec/v2.0.0.html
