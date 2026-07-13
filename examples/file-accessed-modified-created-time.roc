app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.File
import pf.Path
import pf.Utc

# To run this example: check the README.md in this folder

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	file = "LICENSE"

	# NOTE: these functions are checked and built in CI, but not run in release
	# smoke tests because normal musl bundle builds do not support them
	# consistently across targets.

	time_modified = Utc.to_millis_since_epoch(File.time_modified!(file)?)
	time_accessed = Utc.to_millis_since_epoch(File.time_accessed!(file)?)
	created_line = match File.time_created!(file) {
		Ok(time_created) =>
			"    Created: ${Utc.to_millis_since_epoch(time_created).to_str()} ms since epoch"

		Err(FileErr(Unsupported)) => "    Created: unsupported"
		Err(err) => Err(err)?
	}

	Stdout.line!(
		\\${Path.display(file)} file time metadata:
		\\    Modified: ${time_modified.to_str()} ms since epoch
		\\    Accessed: ${time_accessed.to_str()} ms since epoch
		\\${created_line}
		,
	)?

	Ok({})
}
