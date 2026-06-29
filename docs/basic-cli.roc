app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout

## Documentation entrypoint for the `basic-cli` platform.
main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args| {
    _ = Stdout.line!("basic-cli documentation entrypoint")
    Ok({})
}
