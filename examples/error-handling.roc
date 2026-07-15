## Handle common filesystem errors with tag-pattern matching.
app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr
import pf.Stdout
import pf.Path

main! : List(OsStr) => Try({}, _)
main! = |_args| {

	file_name : Path
	file_name = "test-file.txt"

	# Try to read a file that doesn't exist - should error
	missing_file : Path
	missing_file = "nonexistent-file.txt"
	match missing_file.read_utf8!() {
		Ok(content) => Err(UnexpectedReadSuccess(content))?
		Err(PathErr(NotFound)) => Stdout.line!("Expected error: Path not found (NotFound)")?
		Err(PathErr(PermissionDenied)) => Stdout.line!("Error: Permission denied")?
		Err(PathErr(Other(msg))) => Stdout.line!("Error: ${msg}")?
		Err(_) => Stdout.line!("Error: Other file error")?
	}

	# Filesystem kind mismatches are portable typed errors.
	directory : Path
	directory = "examples"
	match directory.read_bytes!() {
		Err(PathErr(IsADirectory)) => Stdout.line!("Expected error: Path is a directory (IsADirectory)")?
		Ok(_) => Err(UnexpectedDirectoryReadSuccess)?
		Err(err) => Err(UnexpectedDirectoryReadError(err))?
	}

	regular_file : Path
	regular_file = "LICENSE"
	match regular_file.list!() {
		Err(PathErr(NotADirectory)) => Stdout.line!("Expected error: Path is not a directory (NotADirectory)")?
		Ok(_) => Err(UnexpectedFileListSuccess)?
		Err(err) => Err(UnexpectedFileListError(err))?
	}

	file_name.write_utf8!("Hello from error-handling example!") ? |err| FileWriteFailed(err)

	content = file_name.read_utf8!() ? |err| FileReadFailed(err)

	# Cleanup
	file_name.delete!() ? |err| FileDeleteFailed(err)

	Stdout.line!("${file_name.display()} contains: ${content}")?

	Ok({})
}
