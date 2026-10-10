## Greet a name supplied as a native command-line argument.
app [main!] { pf: platform "https://github.com/niclas-ahden/basic-cli/releases/download/0.29.0/GrSTLEKd6yw8orQ5ByQQVY3Htg59vZYaoHBvQELueiMB.tar.zst" }

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
	match args {
		[first, ..] => OsStr.display(first)
		[] => "friend"
	}
