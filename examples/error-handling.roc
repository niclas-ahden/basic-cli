app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.File
import pf.Path

# Demonstrates error handling patterns

main! : List(OsStr) => Try({}, _)
main! = |_args| {

	file_name : Path
	file_name = "test-file.txt"

	# Try to read a file that doesn't exist - should error
	match File.read_utf8!("nonexistent-file.txt") {
		Ok(content) => Err(UnexpectedReadSuccess(content))?
		Err(FileErr(NotFound)) => Stdout.line!("Expected error: File not found (NotFound)")?
		Err(FileErr(PermissionDenied)) => Stdout.line!("Error: Permission denied")?
		Err(FileErr(Other(msg))) => Stdout.line!("Error: ${msg}")?
		Err(_) => Stdout.line!("Error: Other file error")?
	}

	file_name.write_utf8!("Hello from error-handling example!") ? |err| FileWriteFailed(err)

	content = file_name.read_utf8!() ? |err| FileReadFailed(err)

	# Cleanup
	file_name.delete!() ? |err| FileDeleteFailed(err)

	Stdout.line!("${file_name.display()} contains: ${content}")?

	Ok({})
}
