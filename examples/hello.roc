app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	Stdout.line!("Hello from basic-cli!")?
	Ok({})
}
