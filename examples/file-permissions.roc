app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.File
import pf.Path

# To run this example: check the README.md in this folder

main! : List(OsStr) => Try({}, _)
main! = |args| {
	file = path_argument(args)?

	is_executable = File.is_executable!(file)?

	is_readable = File.is_readable!(file)?

	is_writable = File.is_writable!(file)?

	Stdout.line!(
		\\${Path.display(file)} file permissions:
		\\    Executable: ${Str.inspect(is_executable)}
		\\    Readable: ${Str.inspect(is_readable)}
		\\    Writable: ${Str.inspect(is_writable)}
		,
	)?

	Ok({})
}

path_argument = |args|
	match args.drop_first(1) {
		[first, ..] => Ok(Path.from_os_str(first))
		[] => Err(MissingPathArgument)
	}
