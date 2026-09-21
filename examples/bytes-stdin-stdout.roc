## Copy raw bytes from standard input to standard output and report the total.
app [main!] { pf: platform "https://github.com/niclas-ahden/basic-cli/releases/download/0.26.0/EuuihZ91yAY1ANck1QytRBcW2jexEfH6yVmxcPCHEHDz.tar.zst" }

import pf.OsStr
import pf.Stdin
import pf.Stdout
import pf.Stderr

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	data = Stdin.read_to_end!()?

	Stdout.write_bytes!(data)?

	Stderr.line!("Copied ${data.len().to_str()} bytes from stdin to stdout.")?

	Ok({})
}
