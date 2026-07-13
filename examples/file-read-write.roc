app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.File
import pf.Path

# Demo of File.read_utf8! and File.write_utf8!

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	out_file = "out.txt"

	Stdout.line!("Writing a string to out.txt")?

	File.write_utf8!(out_file, "a string!")?

	contents = File.read_utf8!(out_file)?

	# Cleanup
	File.delete!(out_file)?

	Stdout.line!("I read the file back. Its contents are: \"${contents}\"")?
	Ok({})
}
