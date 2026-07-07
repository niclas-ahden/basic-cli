# TODO

## Release Readiness

- Get all release-facing workflows green.
  - `.github/workflows/ci.yml`
  - `.github/workflows/release.yml` on pull requests
  - Bundle tests on macOS ARM64, macOS x64, Linux x64, and Linux ARM64
  - Docs generation and `docs.tar.gz` upload
    - Temporary docs wrapper is tracked by https://github.com/roc-lang/roc/issues/10002.
      Remove `docs/basic-cli.roc` and generate docs from `platform/main.roc` when fixed.

- Publish the release only after the manual Release workflow succeeds for the intended tag.
  - Confirm the platform bundle asset is attached.
  - Confirm `docs.tar.gz` is attached.
  - Confirm GitHub Pages deploys release docs and updates the latest-release redirect.

## API Backlog

- Switch `Path.type!` to receiver-style effect calls when Roc supports it.
  - Tracked upstream: https://github.com/roc-lang/roc/issues/9864
  - Desired app-facing shape: `path.type!()`.

## Host Refactor

- Split the single Rust host implementation into smaller modules or crates after
  the Zig compiler migration settles.
  - Keep `src/roc_platform_abi.rs` generated and committed as the ABI boundary.
  - Prefer grouping handwritten host code by Roc module (`cmd`, `file`, `http`,
    `sqlite`, and so on) before considering separate crates.

## Test Coverage

- Restore runtime execution for `tests/cmd-test.roc` after command-test runtime
  blockers are fixed.
  - The maintained `examples/command.roc` expect test covers the stable command
    happy-path and non-zero-exit behavior today.
  - `cmd-test.roc` keeps checking the broader command-error assertions, but the
    current nightly cannot build the optimized coverage path due to
    https://github.com/roc-lang/roc/issues/10003 and still segfaults after
    dev/direct runs complete their assertions.
