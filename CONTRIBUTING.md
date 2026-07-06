# Contributing

Thanks for helping improve `basic-cli`.

This branch requires a Roc compiler matching the commit in `.roc-version`.

## Code of Conduct

We are committed to providing a friendly, safe, and welcoming environment for all. See the [Code of Conduct](https://github.com/roc-lang/roc/blob/main/CODE_OF_CONDUCT.md) for details.

## Version Requirements

`ci/all_tests.sh` reads `.roc-version`, reuses `roc` on `PATH` when it matches, and otherwise builds the pinned Roc compiler into `roc-src/`.

```sh
cat .roc-version
roc version
```

## Updating Roc

The pinned Roc compiler commit lives in `.roc-version`. To update it:

1. Update `.roc-version` to the full 40-character Roc commit SHA.
2. Run `./ci/regenerate_glue.sh` to refresh `src/roc_platform_abi.rs`.
3. Run `cargo check` and `./ci/all_tests.sh` to verify the new compiler works.

## Verification

Run the full local check before opening release or CI-facing changes:

```sh
./ci/all_tests.sh
```

The script builds the host, checks and builds every example, checks test apps, and runs expect tests for examples with maintained scripts. When all target host libraries are present, it also bundles the platform, serves it from localhost, and tests examples against that bundle.

For faster local iterations when the platform host is already built:

```sh
NO_BUILD=1 NO_BUNDLE=1 ./ci/all_tests.sh
```

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

Every checked-in example should pass `roc check` and `roc build` with the pinned compiler.

Examples should include a top-level `main!` annotation. When the full platform error row would distract from the example, map low-level errors into a small example-domain error or use `_` for the error type. Prefer postfix `?`, infix `?`, or `??` for effect results instead of ignoring them.

HTTP examples use Roc's builtin `Json` parser directly through `Http.get!`.

Examples that are intentionally kept out of CI while an API or compiler blocker is tracked use the `.todoroc` extension and must include a TODO comment with a GitHub issue link. Rename them back to `.roc` only after they check and build with the pinned compiler.

## Documentation

Generate platform docs from the docs entrypoint:

```sh
ROC_DOCS_URL_ROOT=/basic-cli/main roc docs --output=generated-docs docs/basic-cli.roc
```

To preview generated docs locally:

```sh
cd generated-docs
simple-http-server --nocache --index
```

CI attaches `docs.tar.gz` to each GitHub Release and deploys release folders plus the current `main` docs to GitHub Pages.
