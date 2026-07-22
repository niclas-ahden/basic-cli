## Greet a name supplied as a native command-line argument.
app [main!] { pf: platform "https://github.com/niclas-ahden/basic-cli/releases/download/0.22.0/7FuVkuWGpyRSLu5w8vfBx82bhqpiVRv1gqQZcrA2H9m9.tar.zst" }

import pf.OsStr
import pf.Stdout

main! : List(OsStr) => Try({}, _)
main! = |args| {

	name : Str
	name = greeting_name(args)

	Stdout.line!("Hello, ${name}, from basic-cli!")?

	Ok({})
}

greeting_name : List(OsStr) -> Str
greeting_name = |args|
	match args.drop_first(1) {
		[first, ..] => OsStr.display(first)
		[] => "friend"
	}
