# Contributing

Thanks for helping improve `basic-cli`.

CI uses the current Roc nightly from [`roc-lang/nightlies`](https://github.com/roc-lang/nightlies).
For local work, use any recent `roc` on `PATH`, or run `./ci/install_roc.sh`
to install the same latest nightly style used by CI.

## Code of Conduct

We are committed to providing a friendly, safe, and welcoming environment for all. See the [Code of Conduct](https://github.com/roc-lang/roc/blob/main/CODE_OF_CONDUCT.md) for details.

## Version Requirements

Check the compiler available locally:

```sh
roc version
```

To install the latest nightly locally:

```sh
./ci/install_roc.sh
ROC_BIN_DIR="$(dirname "$(find .roc-bin -type f -name roc | head -1)")"
export PATH="$(pwd)/$ROC_BIN_DIR:$PATH"
```

## Updating Roc Glue

CI intentionally tracks the current nightly, so compiler updates are adopted as
soon as a new nightly is published. If a nightly changes the host ABI:

1. Run `./ci/regenerate_glue.sh` to refresh `src/roc_platform_abi.rs`.
2. Reconcile `src/lib.rs` if generated names or layouts changed.
3. Run `cargo check` and `./ci/all_tests.sh`.

## Verification

Run the full local check before opening release or CI-facing changes:

```sh
./ci/all_tests.sh
```

The script builds the host, checks and builds every example, checks test apps, and runs expect tests for examples with maintained scripts. When all target host libraries are present, it also bundles the platform, serves it from localhost, and tests examples against that bundle.

For faster local iterations when the platform host is already built:

```sh
NO_BUILD=1 ./ci/all_tests.sh
```

The test script always bundles the current platform and temporarily rewrites
the example and standalone-test app headers to use that bundle. Checked-in
examples may therefore keep using the latest published release URL while local
work and pull requests exercise the WIP platform.

## Rust Glue

The Rust host ABI is generated from `platform/main.roc` using Roc's `RustGlue.roc` generator:

```sh
./ci/regenerate_glue.sh
./ci/regenerate_glue.sh --check
```

Commit `src/roc_platform_abi.rs` with any platform API change and the matching Rust host updates.

The script defaults to a sibling `../roc` checkout. Override paths when needed:

```sh
ROC=../roc/zig-out/bin/roc ROC_SRC=../roc ./ci/regenerate_glue.sh
```

Use `ci/regenerate_glue.sh --check` separately when reviewing platform ABI changes; CI intentionally treats the committed Rust glue as the host ABI source of truth.

Do not edit generated glue by hand.

## Examples

Every checked-in example should pass `roc check`, `roc test`, and `roc build`
with the current nightly.

Examples should include a top-level `main!` annotation. When the full platform error row would distract from the example, map low-level errors into a small example-domain error or use `_` for the error type. Prefer postfix `?`, infix `?`, or `??` for effect results instead of ignoring them.

HTTP examples use Roc's builtin `Json` parser directly through `Http.get!`.

Examples that are intentionally kept out of CI while an API or compiler blocker is tracked use the `.todoroc` extension and must include a TODO comment with a GitHub issue link. Rename them back to `.roc` only after they check and build with the current nightly.

## Documentation

Generate platform docs from the temporary docs wrapper:

```sh
ROC_DOCS_URL_ROOT=/basic-cli/main roc docs --output=generated-docs docs/basic-cli.roc
```

The correct entrypoint is `platform/main.roc`, but `roc docs platform/main.roc`
currently fails to resolve package aliases declared by the platform. Track the
upstream fix in [roc-lang/roc#10002](https://github.com/roc-lang/roc/issues/10002)
and remove `docs/basic-cli.roc` when that is fixed.

To preview generated docs locally:

```sh
cd generated-docs
simple-http-server --nocache --index
```

The release workflow attaches `docs.tar.gz`, updates checked-in examples to the
new bundle URL, generates versioned docs under `www/`, and opens a follow-up PR
for those source changes. It also deploys the validated docs immediately. After
the follow-up PR merges, the Pages workflow deploys the committed release docs
plus freshly generated `main` docs.
