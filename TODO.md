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

- Keep one expect-based end-to-end behavior test for every shipped example.
  - `ci/all_tests.sh` should fail if a new `examples/*.roc` file does not have
    a matching `ci/expect_scripts/*.exp` file.
  - Restored example coverage:
    - `bytes-stdin-stdout.exp`
    - `error-handling.exp`
    - `file-accessed-modified-created-time.exp`
    - `file-permissions.exp`
    - `file-size.exp`
    - `hello.exp`
    - `print.exp`
    - `sqlite-everything.exp`
    - `stdin-pipe.exp`
    - `temp-dir.exp`
    - `terminal-app-snake.exp`

- Port the old standalone test programs back to the new compiler/platform API
  and re-enable their restored expect scripts.
  - `ci/expect_scripts/http.exp` needs a product decision: either port the old
    deleted `examples/http.roc` external-response coverage into
    `examples/http-client.roc`, or delete it with the rationale that
    `http-client.exp` supersedes it with self-contained local-server coverage.

- Restore runtime execution for `tests/cmd-test.roc` after command-test runtime
  blockers are fixed.
  - Re-enable the restored `ci/expect_scripts/cmd-test.exp` when this runs.
  - The maintained `examples/command.roc` expect test covers the stable command
    happy-path and non-zero-exit behavior today.
  - `cmd-test.roc` keeps checking the broader command-error assertions, but the
    current nightly cannot build the optimized coverage path due to
    https://github.com/roc-lang/roc/issues/10003 and still segfaults after
    dev/direct runs complete their assertions.

- Re-enabled standalone expect coverage:
  - `tests/env.roc` -> `ci/expect_scripts/env.exp`
    - Current coverage includes `Env.var!`, `Env.cwd!`, `Env.exe_path!`, and
      `Env.temp_dir!`.
    - Old removed APIs still need a product/API decision before they can be
      restored: `Env.platform!`, `Env.dict!`, and `Env.set_cwd!`.
  - `tests/file.roc` -> `ci/expect_scripts/file.exp`
  - `tests/path-test.roc` -> `ci/expect_scripts/path-test.exp`
  - `tests/sqlite.roc` -> `ci/expect_scripts/sqlite.exp`
  - `tests/tcp.roc` -> `ci/expect_scripts/tcp.exp`
  - `tests/url.roc` -> `ci/expect_scripts/url.exp`
  - `tests/utc.roc` -> `ci/expect_scripts/utc.exp`
