app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdin
import pf.Stdout
import pf.Stderr
import pf.IOErr exposing [IOErr]

# To run this example: check the README.md in this folder

main! : List(Str) => Try({}, [EndOfFile, StdinErr(IOErr), StderrErr(IOErr), StdoutErr(IOErr), ..])
main! = |_args| {
    data = Stdin.bytes!({})?
    Stderr.write_bytes!(data)?
    Stdout.write_bytes!(data)?
    Ok({})
}
