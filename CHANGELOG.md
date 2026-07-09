<h1 align="center">
    <img width="90px" height="auto" src="https://raw.githubusercontent.com/jamesgober/jamesgober/main/media/icons/hexagon-3.svg" alt="Triple Hexagon">
    <br><b>CHANGELOG</b>
</h1>
<p>
  All notable changes to <code>numeric-lang</code> will be documented in this file. The format is based on <a href="https://keepachangelog.com/en/1.1.0/">Keep a Changelog</a>,
  and this project adheres to <a href="https://semver.org/spec/v2.0.0.html/">Semantic Versioning</a>.
</p>

---

## [Unreleased]

### Added

### Changed

### Fixed

### Security

---

## [1.0.0] - 2026-07-08

API freeze. The public surface — `parse`, `Number` (with its variants and
methods), and `ParseNumericError` — is now stable and frozen under Semantic
Versioning. No code changes from 0.2.0; this release records the stability
promise and the completion of the roadmap.

### Changed

- `docs/API.md` marked stable with a documented SemVer promise: the accepted
  grammar, the `serde` representation of `Number`, and the MSRV are all part of
  the `1.x` contract. `ParseNumericError` remains `#[non_exhaustive]` so new
  variants stay additive.

---

## [0.2.0] - 2026-07-08

The core release: the crate now parses numeric literals. Integers in every
radix with digit separators, and decimal floats with correct rounding. This is
the hard part of the roadmap and it is not deferred. The public surface is one
function and three types; it is documented, property-tested, and benchmarked.

### Added

- `parse(&str) -> Result<Number, ParseNumericError>` — the single entry point.
  Detects the radix from a `0b` / `0o` / `0x` prefix (case-insensitive),
  accepts `_` digit separators strictly between digits, and classifies a
  literal as a float when it carries a `.` or a decimal exponent. Total: never
  panics on any input.
- `Number` enum — `Int(u128)` or `Float(f64)` — with `is_int`, `is_float`,
  `as_u128`, and `as_f64` accessors.
- `ParseNumericError` — a `#[non_exhaustive]` error enum carrying byte offsets
  where applicable, implementing `Display` and `core::error::Error`.
- Integer parsing accumulates into `u128` with checked arithmetic (exact
  overflow detection) in a single allocation-free pass; float parsing delegates
  correctly-rounded conversion to the standard library and only layers
  separator handling on top, parsing in place when no separator is present.
- Optional `serde` support (`Serialize` / `Deserialize`) for `Number` behind
  the `serde` feature.
- Property tests (`tests/proptests.rs`): integer radix round-trip, exact `f64`
  round-trip, separator transparency, and totality across arbitrary input.
- Integration tests (`tests/integration.rs`) and Criterion benchmarks
  (`benches/bench.rs`) covering each parse path.
- `docs/API.md` documenting the full public surface with runnable examples.

### Changed

- Manifest `keywords` and `categories` were unquoted bare identifiers (the
  crate did not parse as valid TOML); they are now proper string arrays.
- `clippy.toml` MSRV aligned with `Cargo.toml` at 1.85 (was 1.87).

### Fixed

- `deny.toml` header referenced the wrong crate name.
- README linked a `dev/DIRECTIVES.md` that is not part of the repository; it
  now points at `REPS.md`.

---

## [0.1.0] - 2026-06-18

Initial scaffold and repository bootstrap. No domain logic yet &mdash; this release establishes the structure, tooling, and quality gates the implementation will be built on.

### Added

- `Cargo.toml` with crate metadata, Rust 2024 edition, MSRV 1.85.
- Dual `Apache-2.0 OR MIT` license files.
- `README.md`, `CHANGELOG.md`, and a documentation skeleton.
- `REPS.md` compliance baseline.
- `.github/workflows/ci.yml` CI matrix; `deny.toml`, `clippy.toml`, `rustfmt.toml`.
- `dev/DIRECTIVES.md` and `dev/ROADMAP.md` (committed engineering standards + plan).

[Unreleased]: https://github.com/jamesgober/numeric-lang/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/jamesgober/numeric-lang/compare/v0.2.0...v1.0.0
[0.2.0]: https://github.com/jamesgober/numeric-lang/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jamesgober/numeric-lang/releases/tag/v0.1.0
