## Prompt for two lines of standard input and print a greeting.
app [main!] { pf: platform "https://github.com/niclas-ahden/basic-cli/releases/download/0.22.0/7FuVkuWGpyRSLu5w8vfBx82bhqpiVRv1gqQZcrA2H9m9.tar.zst" }

import pf.OsStr
import pf.Stdin
import pf.Stdout

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	Stdout.line!("What's your first name?")?
	first = Stdin.line!() ? |_| MissingFirstName

	Stdout.line!("What's your last name?")?
	last = Stdin.line!() ? |_| MissingLastName

	Stdout.line!("Hi, ${first} ${last}! \u(1F44B)")?
	Ok({})
}
