app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.Stderr
import pf.Env

# How to read environment variables with Env.var!

main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args| {
    result = Env.var!("EDITOR")

    match result {
        Ok(editor) => {
            Stdout.line!("Your favorite editor is ${editor}!") ? |_| Exit(1)
            letters_result = Env.var!("LETTERS")
            match letters_result {
                Ok(letters) => {
                    joined_letters = Str.join_with(Str.split_on(letters, ","), " ")
                    Stdout.line!("Your favorite letters are: ${joined_letters}") ? |_| Exit(1)
                    Ok({})
                }

                Err(VarNotFound(name)) => {
                    Stderr.line!("Env var ${name} is not set.") ? |_| Exit(1)
                    Ok({})
                }
            }
        }

        Err(VarNotFound(name)) => {
            Stderr.line!("Env var ${name} is not set.") ? |_| Exit(1)
            Ok({})
        }
    }
}
