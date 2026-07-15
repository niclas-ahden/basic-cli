[![Roc-Lang][roc_badge]][roc_link]

[roc_badge]: https://img.shields.io/endpoint?url=https%3A%2F%2Fpastebin.com%2Fraw%2FcFzuCCd7
[roc_link]: https://github.com/roc-lang/roc

# basic-cli

A Roc [platform](https://github.com/roc-lang/roc/wiki/Roc-concepts-explained#platform) for command-line programs.

`basic-cli` supports command execution, directories, environment variables, files, HTTP, locales, paths, random seeds, sleeping, SQLite, standard input/output/error, TCP, terminal raw mode, and UTC time.

## Examples

The [examples](examples/) directory contains executable, application-shaped CLI
programs for common tasks like reading files, running commands, constructing
URLs, making HTTP requests, working with SQLite, and reading stdin.

The examples are pinned to the [`0.21.0-rc4` release](https://github.com/roc-lang/basic-cli/releases/tag/0.21.0-rc4):

```roc
app [main!] { pf: platform "https://github.com/roc-lang/basic-cli/releases/download/0.21.0-rc4/FvCh4vdqm3nBY6DWEfZ8RuGCVfjuMY43HA8KSNk9qVDn.tar.zst" }
```

To run examples from a local checkout instead, build the platform first and use `"../platform/main.roc"`; see [CONTRIBUTING.md](CONTRIBUTING.md).

HTTP examples use Roc's builtin `Json` parser and encoder directly through
`Http.get!` and `Http.send_json!`.

## Documentation

- [`0.21.0-rc4` release documentation](https://roc-lang.github.io/basic-cli/0.21.0-rc4/)
- [latest main branch](https://roc-lang.github.io/basic-cli/main/)

## Help

Ask questions on [Roc Zulip](https://roc.zulipchat.com), especially in the `#beginners` stream.

## Contributing

Contributor setup, verification, generated glue, and documentation publishing notes live in [CONTRIBUTING.md](CONTRIBUTING.md).
