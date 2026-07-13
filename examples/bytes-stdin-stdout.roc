app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdin
import pf.Stdout
import pf.Stderr

# To run this example: check the README.md in this folder

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	data = Stdin.read_to_end!()?
	Stdout.write_bytes!(data)?
	Stderr.line!("Copied ${data.len().to_str()} bytes from stdin to stdout.")?
	Ok({})
}
