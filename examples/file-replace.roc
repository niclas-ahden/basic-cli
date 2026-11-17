## Replace every occurrence of a substring in a UTF-8 file in place.
app [main!] { pf: platform "https://github.com/niclas-ahden/basic-cli/releases/download/0.32.0/3MnQnCrtfe6XS5qx58MnAGU6iWk8i1N24MYfoejcWeax.tar.zst" }

import pf.OsStr
import pf.Stdout
import pf.Path

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	file : Path
	file = "greeting.txt"

	file.write_utf8!("Hello, World! Hello, Roc!")?

	# Replaces both occurrences of "Hello", not just the first.
	file.replace_utf8!("Hello", "Goodbye")?

	contents = file.read_utf8!()?

	# Cleanup
	file.delete!()?

	Stdout.line!("After replacing: \"${contents}\"")?

	Ok({})
}
