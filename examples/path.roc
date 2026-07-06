app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.Path

# Demo of basic-cli Path functions

main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args| {
    path = Path.from_str("path.roc")

    Stdout.line!(
        \\is_file: ${Str.inspect(Path.is_file!(path))}
        \\is_dir: ${Str.inspect(Path.is_dir!(path))}
        \\is_sym_link: ${Str.inspect(Path.is_sym_link!(path))}
        \\display: ${Path.display(path)}
    ) ? |_| Exit(1)

    Ok({})
}
