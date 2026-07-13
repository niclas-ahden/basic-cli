app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout

main! : List(OsStr) => Try({}, _)
main! = |args| {
	name = greeting_name(args)
	Stdout.line!("Hello, ${name}, from basic-cli!")?
	Ok({})
}

greeting_name = |args|
	match args.drop_first(1) {
		[first, ..] => OsStr.display(first)
		[] => "friend"
	}
