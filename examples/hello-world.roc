app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout

main! : List(Str) => Try({}, _)
main! = |_args| {
    Stdout.line!("Hello, World!")?
    Ok({})
}
