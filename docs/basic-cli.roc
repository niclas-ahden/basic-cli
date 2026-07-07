app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout

# TODO: remove this wrapper and run `roc docs platform/main.roc` directly when
# https://github.com/roc-lang/roc/issues/10002 is fixed.
main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args| {
    Stdout.line!("basic-cli documentation entrypoint") ? |_| Exit(1)
    Ok({})
}
