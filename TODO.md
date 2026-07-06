# TODO

## Release Readiness

- Get all release-facing workflows green.
  - `.github/workflows/ci.yml`
  - `.github/workflows/release.yml` on pull requests
  - Bundle tests on macOS ARM64, macOS x64, Linux x64, and Linux ARM64
  - Docs generation and `docs.tar.gz` upload

- Publish the release only after the manual Release workflow succeeds for the intended tag.
  - Confirm the platform bundle asset is attached.
  - Confirm `docs.tar.gz` is attached.
  - Confirm GitHub Pages deploys release docs and updates the latest-release redirect.

## API Backlog

- Switch `Path.type!` to receiver-style effect calls when Roc supports it.
  - Tracked upstream: https://github.com/roc-lang/roc/issues/9864
  - Desired app-facing shape: `path.type!()`.
