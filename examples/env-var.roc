app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.Env

# How to read environment variables with Env.var!

main! : List(Str) => Try({}, _)
main! = |_args| {
    editor = Env.var!("EDITOR")?
    Stdout.line!("Your favorite editor is ${editor}!")?

    letters = Env.var!("LETTERS")?
    joined_letters = Str.join_with(Str.split_on(letters, ","), " ")
    Stdout.line!("Your favorite letters are: ${joined_letters}")?

    Ok({})
}
