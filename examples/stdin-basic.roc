app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdin
import pf.Stdout

main! : List(OsStr) => Try({}, _)
main! = |_args| {
    Stdout.line!("What's your first name?")?
    first = Stdin.line!() ?? ""

    Stdout.line!("What's your last name?")?
    last = Stdin.line!() ?? ""

    Stdout.line!("Hi, ${first} ${last}! \u(1F44B)")?
    Ok({})
}
