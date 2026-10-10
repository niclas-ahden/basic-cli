## Enable terminal raw mode, read one key, and restore normal behavior.
app [main!] { pf: platform "https://github.com/niclas-ahden/basic-cli/releases/download/0.29.0/GrSTLEKd6yw8orQ5ByQQVY3Htg59vZYaoHBvQELueiMB.tar.zst" }

import pf.OsStr
import pf.Stdout
import pf.Stdin
import pf.Tty

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
