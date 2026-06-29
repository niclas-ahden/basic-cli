[![Roc-Lang][roc_badge]][roc_link]

[roc_badge]: https://img.shields.io/endpoint?url=https%3A%2F%2Fpastebin.com%2Fraw%2FcFzuCCd7
[roc_link]: https://github.com/roc-lang/roc

# basic-cli

A Roc [platform](https://github.com/roc-lang/roc/wiki/Roc-concepts-explained#platform) for command-line programs.

This migration branch supports command execution, directories, environment variables, files, locales, paths, random seeds, sleeping, standard input/output/error, terminal raw mode, and UTC time. HTTP, TCP, SQLite, and URL helpers from the old API have not been ported to the new compiler backend yet.

:eyes: **examples**:
  - [latest main branch](https://github.com/roc-lang/basic-cli/tree/main/examples)

:book: **documentation**:
  - [latest release](https://roc-lang.github.io/basic-cli/)
  - [latest main branch](https://roc-lang.github.io/basic-cli/main/)

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

### Verification

Run the full local check before opening release or CI-facing changes:

```sh
./ci/all_tests.sh
```

The script checks generated glue, builds the host, checks and builds every example, and runs expect tests for examples with maintained scripts. When all target host libraries are present, it also bundles the platform, serves it from localhost, and tests examples against that bundle.

### Documentation

Generate platform docs from the docs entrypoint:

```sh
ROC_DOCS_URL_ROOT=/basic-cli/main roc docs --output=generated-docs docs/basic-cli.roc
```

CI attaches `docs.tar.gz` to each GitHub Release and deploys release folders plus the current `main` docs to GitHub Pages.
