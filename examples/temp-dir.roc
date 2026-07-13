app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.Env
import pf.Path

# To run this example: check the README.md in this folder

# Prints the default temp dir

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	temp_dir_path = Env.temp_dir!()

	Stdout.line!("The temp dir path is ${Path.display(temp_dir_path)}")?
	Ok({})
}
