app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr
import pf.Stdout
import pf.Utc
import pf.Sleep

# Demo Utc and Sleep functions

main! : List(OsStr) => Try({}, _)
main! = |_args| {

	start : U128
	start = Utc.now!()

	Stdout.line!("Started at ${Utc.to_iso_8601(start)}")?

	# 1000 ms = 1 second
	Sleep.millis!(1000)

	finish : U128
	finish = Utc.now!()

	duration_ms = Utc.delta_as_millis(finish, start)
	duration_nanos = Utc.delta_as_nanos(finish, start)

	Stdout.line!("Completed in ${duration_ms.to_str()} ms (${duration_nanos.to_str()} ns)")?

	Ok({})
}
