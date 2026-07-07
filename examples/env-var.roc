app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.Env

# How to read environment variables with Env.var!

main! : List(OsStr) => Try({}, _)
main! = |_args| {
    editor = Env.var!("EDITOR")?
    Stdout.line!("Your favorite editor is ${OsStr.display(editor)}!")?

    letters = Env.var_str!("LETTERS")?
    joined_letters = Str.join_with(Str.split_on(letters, ","), " ")
    Stdout.line!("Your favorite letters are: ${joined_letters}")?

    Ok({})
}
