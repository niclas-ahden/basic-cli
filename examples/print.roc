## Write lines, text, and lists to standard output and standard error.
app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.Stderr

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	# Print a string to stdout
	Stdout.line!("Hello, world!")?

	# Print without a newline
	Stdout.write!("No newline after me.")?

	# Print a string to stderr
	Stderr.line!("Hello, error!")?

	# Print a string to stderr without a newline
	Stderr.write!("Err with no newline after.")?

	# Print a list to stdout
	for str in ["Foo", "Bar", "Baz"] {
		Stdout.line!(str)?
	}

	Ok({})
}
