# Contributing

## Code of Conduct

We are committed to providing a friendly, safe and welcoming environment for all. See the [Code of Conduct](https://github.com/roc-lang/roc/blob/main/CODE_OF_CONDUCT.md) for details.

## How to update the Roc version

The pinned Roc compiler commit lives in `.roc-version`. To update it:

1. Update `.roc-version` to the full 40-character Roc commit SHA.
2. Run `./ci/regenerate_glue.sh` to refresh `src/roc_platform_abi.rs`.
3. Run `cargo check` and `./ci/all_tests.sh` to verify the new compiler works.

## Documentation

Generated Roc docs are currently disabled for this migration branch because `roc docs` is not implemented in the new compiler backend yet. Update this section and restore docs CI when that command is available again.

## Regenerating Rust Glue

When the platform API changes in `platform/*.roc`, regenerate the Rust ABI bindings instead of editing them by hand:

```bash
./ci/regenerate_glue.sh
```

The script writes `src/roc_platform_abi.rs` using Roc's `RustGlue.roc` generator. It defaults to a sibling `../roc` checkout, and you can override paths when needed:

```bash
ROC=../roc/zig-out/bin/roc ROC_SRC=../roc ./ci/regenerate_glue.sh
```

To verify the checked-in glue is current without modifying the worktree:

```bash
./ci/regenerate_glue.sh --check
```

Commit the platform API change, the regenerated `src/roc_platform_abi.rs`, and any required Rust host implementation updates together. Do not edit the generated glue file manually.

Run `./ci/all_tests.sh` before opening a release or CI-facing PR.
