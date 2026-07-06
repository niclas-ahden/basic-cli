app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdin
import pf.Stdout

main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args| {
    Stdout.line!("What's your first name?") ? |_| Exit(1)
    first = match Stdin.line!() {
        Ok(line) => line
        Err(_) => ""
    }

    Stdout.line!("What's your last name?") ? |_| Exit(1)
    last = match Stdin.line!() {
        Ok(line) => line
        Err(_) => ""
    }

    Stdout.line!("Hi, ${first} ${last}! \u(1F44B)") ? |_| Exit(1)
    Ok({})
}
