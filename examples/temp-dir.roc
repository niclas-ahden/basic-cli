## Print the operating system's default temporary directory.
app [main!] { pf: platform "https://github.com/niclas-ahden/basic-cli/releases/download/0.22.0/7FuVkuWGpyRSLu5w8vfBx82bhqpiVRv1gqQZcrA2H9m9.tar.zst" }

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
