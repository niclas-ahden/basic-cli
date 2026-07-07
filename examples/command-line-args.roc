app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout

main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |args| {
    # Skip first arg (executable path), get the remaining args
    match args.drop_first(1) {
        [first_arg, ..] => {
            Stdout.line!("received argument: ${first_arg}") ? |_| Exit(1)
            arg_bytes = Str.to_utf8(first_arg)
            Stdout.line!("Unix argument, bytes: ${Str.inspect(arg_bytes)}") ? |_| Exit(1)
            round_tripped_arg = Str.from_utf8(arg_bytes) ? |_| Exit(1)
            Stdout.line!("back to Arg: ${Str.inspect(round_tripped_arg)}") ? |_| Exit(1)
            Ok({})
        }
        [] => {
            Stdout.line!("Error: I expected one argument, but got none.") ? |_| Exit(1)
            Err(Exit(1))
        }
    }
}
