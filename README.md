[![Roc-Lang][roc_badge]][roc_link]

[roc_badge]: https://img.shields.io/endpoint?url=https%3A%2F%2Fpastebin.com%2Fraw%2FcFzuCCd7
[roc_link]: https://github.com/roc-lang/roc

# basic-cli

A Roc [platform](https://github.com/roc-lang/roc/wiki/Roc-concepts-explained#platform) to work with files, commands, HTTP, TCP, command line arguments,...

:eyes: **examples**:
  - [latest main branch](https://github.com/roc-lang/basic-cli/tree/main/examples)

:book: **documentation**:
  - TBA -- `roc docs` not yet implemented in the new compiler

## Running Locally

This branch requires a Roc compiler matching the commit in `.roc-version`.

### Version Requirements

`ci/all_tests.sh` reads `.roc-version`, reuses `roc` on `PATH` when it matches, and otherwise builds the pinned Roc compiler into `roc-src/`.

```sh
cat .roc-version
roc version
```

### Rust Glue

The Rust host ABI is generated from `platform/main.roc` using Roc's `RustGlue.roc` generator:

```sh
./ci/regenerate_glue.sh
./ci/regenerate_glue.sh --check
```

Commit `src/roc_platform_abi.rs` with any platform API change and the matching Rust host updates.
