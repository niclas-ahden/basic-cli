app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.Stderr
import pf.Env

# How to read environment variables with Env.var!

main! = |_args| {
    result = Env.var!("EDITOR")

    match result {
        Ok(editor) => {
            _ = Stdout.line!("Your favorite editor is ${editor}!")
            Ok({})
        }

        Err(VarNotFound(name)) => {
            _ = Stderr.line!("Env var ${name} is not set.")
            Ok({})
        }
    }
}
