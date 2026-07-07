# PR Merge Backlog

This file tracks only known issues that block the Zig compiler migration PR or
require an explicit temporary workaround before merge. General release chores,
API wishlist items, and post-merge refactors should not live here.

## Upstream Roc Blockers

- Generate docs from the real platform entrypoint.
  - Blocked by https://github.com/roc-lang/roc/issues/10002.
  - Current workaround: `.github/workflows/release.yml` and
    `.github/workflows/deploy-docs.yml` run `roc docs` against
    `docs/basic-cli.roc`.
  - Required resolution: remove `docs/basic-cli.roc` and generate docs from
    `platform/main.roc` once `roc docs platform/main.roc` can resolve package
    aliases declared by the platform.

- Re-enable the command error end-to-end test.
  - Blocked by https://github.com/roc-lang/roc/issues/10003.
  - Current workaround: `ci/all_tests.sh` keeps `cmd-test` in
    `SKIPPED_EXPECT_NAMES`; `tests/cmd-test.roc` and
    `ci/expect_scripts/cmd-test.exp` remain in the repo but are not active in
    the check/build/expect suite.
  - Required resolution: add `cmd-test` back to `TEST_EXPECT_NAMES` once the Roc
    compiler can build/run this coverage path, or replace it with a smaller
    active regression test that preserves the command error behavior without
    triggering the upstream compiler bug.

## Local PR Decisions

- Decide whether to restore old environment APIs or explicitly drop them.
  - Current coverage: `tests/env.roc` exercises `Env.var!`, `Env.cwd!`,
    `Env.exe_path!`, and `Env.temp_dir!`.
  - Missing old API coverage: `Env.platform!`, `Env.dict!`, and
    `Env.set_cwd!`.
  - Required resolution: restore these APIs/tests, or document that they are
    intentionally removed from the migrated platform API.

- Decide whether to restore old typed env/arg helper APIs or keep the current
  observable behavior only.
  - `examples/env-var.roc` restores the old `LETTERS=a,c,e,j` output using
    `Env.var_str!`, but does not restore typed `Env.decode!`.
  - `examples/command-line-args.roc` restores the old bytes and round-trip
    output using the current `List(OsStr)` argv shape.
  - Required resolution: restore these helper APIs, or document that the current
    examples intentionally preserve behavior without preserving the old API
    surface.
