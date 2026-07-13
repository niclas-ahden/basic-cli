# Release Candidate Blockers

This file tracks only work that must be resolved before publishing a release
candidate from the Zig compiler migration. It was last reviewed against PR #413
at `49005f4` on 2026-07-13.

An upstream compiler issue is not automatically an RC blocker when a tested,
explicit workaround exists. Conversely, a green build step is not sufficient:
the downloaded bundle, release tag, public API contract, and supported target
matrix must all be verified.

P0/P1 indicates resolution order; every unchecked item below blocks the RC.

## P0: Product Contract and Critical Coverage

- [ ] Define and publish the migrated public API contract.
  - This PR is not a drop-in compiler-only migration. It changes public effect
    signatures and types, adopts shared `http` and `path` package types, adds
    `OsStr` for native strings and argv, removes the old `Arg` module, and drops
    environment APIs.
  - Explicit unresolved environment gaps are `Env.platform!`, `Env.dict!`,
    `Env.set_cwd!`, and typed `Env.decode!`. Current tests cover only `Env.var!`,
    `Env.cwd!`, `Env.exe_path!`, and `Env.temp_dir!`.
  - Generated docs expose the ABI-facing `InternalSqlite` module as public API.
    All 13 `Url` entries, all four `InternalSqlite` types, and 10 of 49 `Sqlite`
    entries have no prose. Shared `Request`, `Response`, and package `Path`
    types also render without links in direct platform docs, whose package title
    is the generic `main` rather than `basic-cli`.
  - Required resolution: inventory the old versus migrated exposed API, restore
    anything required for the RC contract, decide whether `InternalSqlite`
    belongs in the public surface, and document every intentional breaking
    change with its replacement or migration path. Release notes must not
    describe this as only an implementation/compiler migration.

- [ ] Restore active end-to-end coverage for command error behavior.
  - `tests/cmd-test.roc` and `ci/expect_scripts/cmd-test.exp` exist, but
    `ci/all_tests.sh` still skips even checking the app. This leaves
    missing-executable, invalid-UTF-8 output, and several structured error
    mappings outside the release gate.
  - The original compiler blocker, https://github.com/roc-lang/roc/issues/10003,
    is closed as no longer reproducing. With `release-fast-ec04debe`, the
    optimized app builds and all runtime assertions pass, but Roc emits eight
    `UNCONDITIONAL CONDITION` warnings and returns a nonzero build status.
  - Required resolution: eliminate or upstream the warning diagnostics, then
    re-enable `cmd-test` and its expect script on the supported Unix runners.
    Windows must at least check and build the app; its assertions use Unix
    commands.

## P1: Reproducibility and Approval

- [ ] Freeze and record the remaining toolchain used for the RC.
  - CI and release jobs still resolve the mutable `nightly-new-compiler` alias.
  - Rust and Cargo are pinned to 1.82.0 by `rust-toolchain.toml` and the workflow
    setup steps. Both Rust workspaces commit `Cargo.lock`, and build commands use
    `--locked` so CI cannot silently update dependency resolution.
  - `src/roc_platform_abi.rs` is committed and imported by the Rust host, so
    every Cargo host build compiles against the checked-in ABI.
  - Required resolution: use an immutable Roc nightly identifier for the RC
    build and all artifact tests, and record the Roc, Zig, and Rust versions in
    the release metadata.

- [ ] Complete review on the exact commit proposed for the RC.
  - PR #413 is currently a draft and GitHub reports `REVIEW_REQUIRED`.
  - Required resolution: rerun the full release matrix after the final blocker
    fixes, obtain maintainer approval, and only then remove draft status/cut the
    RC. Any code change after approval must rerun the applicable gates.

## Tracked Work That Does Not Block the RC

- Add a CI glue drift check that obtains `RustGlue.roc` from the source revision
  corresponding to the published nightly, regenerates `src/roc_platform_abi.rs`,
  and fails on a diff. The committed glue remains the host build input until
  then.
- Restore continuous testing of the latest published release after the
  migration. This is valuable post-release drift detection, but the RC is gated
  by a clean downloaded-bundle test at the pinned RC toolchain.
