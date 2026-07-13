# Release Candidate Blockers

This file tracks only work that must be resolved before publishing a release
candidate from the Zig compiler migration. It was last reviewed against PR #413
at `f7e9162` on 2026-07-13.

An upstream compiler issue is not automatically an RC blocker when a tested,
explicit workaround exists. Conversely, a green build step is not sufficient:
the downloaded bundle, release tag, public API contract, and supported target
matrix must all be verified.

P0/P1 indicates resolution order; every unchecked item below blocks the RC.

## P0: Artifact and Release Integrity

- [ ] Make every release-facing check green at the intended RC commit.
  - Local and CI tests now build a WIP package and rewrite both `examples/*.roc`
    and `tests/*.roc` to consume it. Release tests download the uploaded package
    and use the same artifact-only path.
  - Linux x64 and Windows x64 CI pass at `f7e9162`. The Windows test builds,
    bundles, links, and runs an app using the packaged `host.lib` and Windows SDK
    import libraries:
    https://github.com/roc-lang/basic-cli/actions/runs/29216873253.
  - macOS ARM64 and macOS x64 CI are waiting for runner capacity. The release
    workflow's native Windows host build passes; release-bundle assembly and its
    four-platform downloaded-bundle matrix are still pending:
    https://github.com/roc-lang/basic-cli/actions/runs/29216873260.
  - Required resolution: make the remaining macOS CI jobs and all downloaded
    release-bundle tests pass on macOS ARM64, macOS x64, Linux x64, and Windows
    x64 at the final RC commit.

## P0: Product Contract and Critical Coverage

- [ ] Define and publish the migrated public API contract.
  - This PR is not a drop-in compiler-only migration. It changes public effect
    signatures and types, adopts shared `http` and `path` package types, adds
    `OsStr` for native strings and argv, removes the old `Arg` module, and drops
    environment APIs.
  - Explicit unresolved environment gaps are `Env.platform!`, `Env.dict!`,
    `Env.set_cwd!`, and typed `Env.decode!`. Current tests cover only `Env.var!`,
    `Env.cwd!`, `Env.exe_path!`, and `Env.temp_dir!`.
  - Required resolution: inventory the old versus migrated exposed API, restore
    anything required for the RC contract, and document every intentional
    breaking change with its replacement or migration path. Release notes must
    not describe this as only an implementation/compiler migration.

- [ ] Restore active end-to-end coverage for command error behavior.
  - `tests/cmd-test.roc` and `ci/expect_scripts/cmd-test.exp` exist, but
    `ci/all_tests.sh` skips even checking the app because the current compiler
    crashes on it. This leaves missing-executable, invalid-UTF-8 output, and
    several structured error mappings outside the release gate.
  - Tracked upstream: https://github.com/roc-lang/roc/issues/10003.
  - Required resolution: re-enable `cmd-test`, or split it into smaller active
    regression apps that preserve the critical error-path assertions without
    triggering the compiler bug. The upstream compiler fix itself is not
    required if equivalent coverage is green on all supported targets.

## P1: Reproducibility and Approval

- [ ] Freeze and record the toolchain used for the RC.
  - CI and release jobs resolve the mutable `nightly-new-compiler` alias, while
    the committed `src/roc_platform_abi.rs` is compiler-generated and is not
    regenerated or checked in CI. Rust is also selected as mutable `stable`.
  - Required resolution: use an immutable Roc nightly identifier for the RC
    build and all artifact tests, verify the committed glue with that compiler,
    and record the Roc, Zig, and Rust versions in the release metadata. Prefer
    pinning Rust as well so the host archive can be reproduced.

- [ ] Complete review on the exact commit proposed for the RC.
  - PR #413 is currently a draft and GitHub reports `REVIEW_REQUIRED`.
  - Required resolution: rerun the full release matrix after the final blocker
    fixes, obtain maintainer approval, and only then remove draft status/cut the
    RC. Any code change after approval must rerun the applicable gates.

## Tracked Work That Does Not Block the RC

- Generate docs directly from `platform/main.roc` after
  https://github.com/roc-lang/roc/issues/10002 is fixed. The temporary
  `docs/basic-cli.roc` wrapper is explicit, and the current docs build succeeds.
- Switch `Path.type!` to receiver-style effect calls after
  https://github.com/roc-lang/roc/issues/9864 is fixed.
- Restore continuous testing of the latest published release after the
  migration. This is valuable post-release drift detection, but the RC is gated
  by a clean downloaded-bundle test at the pinned RC toolchain.
