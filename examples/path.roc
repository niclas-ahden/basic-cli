## Inspect a path's filename, extension, string representation, and type (file/dir/symlink).
app [main!] { pf: platform "https://github.com/niclas-ahden/basic-cli/releases/download/0.22.0/7FuVkuWGpyRSLu5w8vfBx82bhqpiVRv1gqQZcrA2H9m9.tar.zst" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.Path

main! : List(OsStr) => Try({}, _)
main! = |args| {
	path = path_argument(args)?
	filename = Path.filename(path).map_ok(Path.display) ?? "Path is not a file."
	extension = Path.ext(path).map_ok(Path.display) ?? "<none>"

	Stdout.line!(
		\\Path: ${Path.display(path)}
		\\Debug: ${Str.inspect(path)}
		\\Filename: ${filename}
		\\Extension: ${extension}
		\\Type: ${Str.inspect(Path.type!(path)?)}
		,
	)?

	Ok({})
}

path_argument = |args|
	match args.drop_first(1) {
		[first, ..] => Ok(Path.from_os_str(first))
		[] => Err(MissingPathArgument)
	}
