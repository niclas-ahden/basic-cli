app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr
import pf.Stdout
import pf.Stdin
import pf.Tty

## Raw mode allows you to change the behaviour of the terminal.
## This is useful for running an app like vim or a game in the terminal.

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	Stdout.line!("Tty: enabling raw mode")?
	Tty.enable_raw_mode!()

	Stdout.line!("Press one key...")?
	input = Stdin.bytes!()

	Stdout.line!("Tty: disabling raw mode")?
	Tty.disable_raw_mode!()

	bytes = input?
	Stdout.line!("Read ${bytes.len().to_str()} byte(s).")?
	Ok({})
}
