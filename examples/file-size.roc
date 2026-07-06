app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.File

# To run this example: check the README.md in this folder

main! : List(Str) => Try({}, _)
main! = |_args| {
	file_size = File.size_in_bytes!("LICENSE")?

	Stdout.line!("The size of the LICENSE file is: ${file_size.to_str()} bytes")?

	Ok({})
}
