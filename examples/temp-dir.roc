## Print the operating system's default temporary directory.
app [main!] { pf: platform "https://github.com/niclas-ahden/basic-cli/releases/download/0.22.1/DobkAk7zNyqAgqh2Riaj5c5DtWtKhd5iVYE5RFa6izcd.tar.zst" }

import pf.OsStr
import pf.Stdout
import pf.Env
import pf.Path

main! : List(OsStr) => Try({}, _)
main! = |_args| {

	temp_dir_path : Path
	temp_dir_path = Env.temp_dir!()

	Stdout.line!("The temp dir path is ${temp_dir_path.display()}")?

	Ok({})
}
