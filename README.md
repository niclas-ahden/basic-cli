[![Roc-Lang][roc_badge]][roc_link]

[roc_badge]: https://img.shields.io/endpoint?url=https%3A%2F%2Fpastebin.com%2Fraw%2FcFzuCCd7
[roc_link]: https://github.com/roc-lang/roc

# basic-cli

A Roc [platform](https://github.com/roc-lang/roc/wiki/Roc-concepts-explained#platform) for command-line programs.

`basic-cli` supports command execution, directories, environment variables, files, HTTP, locales, paths, random seeds, sleeping, SQLite, standard input/output/error, TCP, terminal raw mode, and UTC time.

## Examples

The [examples](examples/) directory shows small CLI programs for common tasks like reading files, running commands, making HTTP requests, working with SQLite, and reading stdin.

If you want to run an example without building `basic-cli` from source, use a released bundle URL in place of `"../platform/main.roc"`. To run examples from a local checkout, build the platform first; see [CONTRIBUTING.md](CONTRIBUTING.md).

HTTP examples use Roc's builtin `Json` parser directly through `Http.get!`.

## Documentation

- [latest release](https://roc-lang.github.io/basic-cli/)
- [latest main branch](https://roc-lang.github.io/basic-cli/main/)

## Help

Ask questions on [Roc Zulip](https://roc.zulipchat.com), especially in the `#beginners` stream.

## Contributing

Contributor setup, verification, generated glue, and documentation publishing notes live in [CONTRIBUTING.md](CONTRIBUTING.md).
