# Changelog

This file documents all changes affecting the [semver] version of this project.

## New in this release

### Breaking Changes

- Hide the doctest line number helper better

### Added

- Add `ErrorStash::ok`
- Add Apache 2.0 license (project is now dual-licensed: MIT or Apache 2.0)
- Add contribution info to `README.md`
- Add rustdoc links and badges to `README.md`

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

[`v0.4.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.4.0
[`v0.3.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.3.0
[`v0.2.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.2.0
[`v0.1.1`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.1.1
[`v0.1.0`]: https://github.com/Lintermute/lazy_errors/releases/tag/v0.1.0

[semver]: https://semver.org/spec/v2.0.0.html
