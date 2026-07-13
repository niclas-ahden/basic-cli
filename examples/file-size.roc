app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.File
import pf.Path

# To run this example: check the README.md in this folder

main! : List(OsStr) => Try({}, _)
main! = |args| {
	file = path_argument(args)?
	file_size = File.size_in_bytes!(file)?

	Stdout.line!("${Path.display(file)} is ${file_size.to_str()} bytes")?

	Ok({})
}

path_argument = |args|
	match args.drop_first(1) {
		[first, ..] => Ok(Path.from_os_str(first))
		[] => Err(MissingPathArgument)
	}
