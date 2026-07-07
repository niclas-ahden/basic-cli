app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout

main! : List(Str) => Try({}, _)
main! = |args| {
    # Skip first arg (executable path), get the remaining args
    match args.drop_first(1) {
        [first_arg, ..] => {
            Stdout.line!("received argument: ${first_arg}")?
            arg_bytes = Str.to_utf8(first_arg)
            Stdout.line!("Unix argument, bytes: ${Str.inspect(arg_bytes)}")?
            round_tripped_arg = Str.from_utf8(arg_bytes)?
            Stdout.line!("back to Arg: ${Str.inspect(round_tripped_arg)}")?
            Ok({})
        }
        [] => Err(MissingArgument)
    }
}
