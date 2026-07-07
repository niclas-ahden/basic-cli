app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.Path

# Demo of basic-cli Path functions

main! : List(OsStr) => Try({}, _)
main! = |_args| {
    path = "path.roc"

    Stdout.line!(
        \\is_file: ${Str.inspect(Path.is_file!(path))}
        \\is_dir: ${Str.inspect(Path.is_dir!(path))}
        \\is_sym_link: ${Str.inspect(Path.is_sym_link!(path))}
        \\display: ${Path.display(path)}
    )?

    Ok({})
}
