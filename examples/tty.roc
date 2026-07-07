app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.Tty

## Raw mode allows you to change the behaviour of the terminal.
## This is useful for running an app like vim or a game in the terminal.

main! : List(OsStr) => Try({}, _)
main! = |_args| {
    Stdout.line!("Tty: enabling raw mode")?
    Tty.enable_raw_mode!()

    Stdout.line!("Tty: disabling raw mode")?
    Tty.disable_raw_mode!()

    Ok({})
}
