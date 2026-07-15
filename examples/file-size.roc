app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr
import pf.Stdout
import pf.Path

# To run this example: check the README.md in this folder

main! : List(OsStr) => Try({}, _)
main! = |args| {
	file = path_argument(args)?
	file_size = file.size_in_bytes!()?

	Stdout.line!("${file.display()} is ${file_size.to_str()} bytes")?

	Ok({})
}

path_argument = |args|
	match args.drop_first(1) {
		[first, ..] => Ok(Path.from_os_str(first))
		[] => Err(MissingPathArgument)
	}
