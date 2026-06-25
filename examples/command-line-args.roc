app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout

main! = |args| {
    # Skip first arg (executable path), get the remaining args
    match args.drop_first(1) {
        [first_arg, ..] => {
            _ = Stdout.line!("received argument: ${first_arg}")
            Ok({})
        }
        [] => {
            _ = Stdout.line!("Error: I expected one argument, but got none.")
            Err(Exit(1))
        }
    }
}
