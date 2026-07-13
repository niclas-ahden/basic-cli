app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Sleep
import pf.Stdout
import pf.Utc

main! : List(OsStr) => Try({}, [Exit(I32), ..])
main! = |_args|
	match run_tests!() {
		Ok({}) => Ok({})
		Err(err) => {
			Stdout.line!("Test run failed: ${Str.inspect(err)}") ? |_| Exit(1)
			Err(Exit(1))
		}
	}

run_tests! : () => Try({}, _)
run_tests! = || {
	test_time_conversion!()?
	test_time_delta!()?

	Stdout.line!("\nAll tests executed.")?
	Ok({})
}

test_time_conversion! : () => Try({}, _)
test_time_conversion! = || {
	now = Utc.now!()

	millis_since_epoch = Utc.to_millis_since_epoch(now)
	Stdout.line!("Current time in milliseconds since epoch: ${millis_since_epoch.to_str()}")?
	err_on_false(millis_since_epoch > 0)?

	time_from_millis = Utc.from_millis_since_epoch(millis_since_epoch)
	Stdout.line!("Time reconstructed from milliseconds: ${Utc.to_iso_8601(time_from_millis)}")?
	err_on_false(Utc.to_iso_8601(time_from_millis) == Utc.to_iso_8601(now))?

	nanos_since_epoch = Utc.to_nanos_since_epoch(now)
	Stdout.line!("Current time in nanoseconds since epoch: ${nanos_since_epoch.to_str()}")?
	err_on_false(nanos_since_epoch >= millis_since_epoch * 1_000_000)?

	time_from_nanos = Utc.from_nanos_since_epoch(nanos_since_epoch)
	Stdout.line!("Time reconstructed from nanoseconds: ${Utc.to_iso_8601(time_from_nanos)}")?
	err_on_false(Utc.to_iso_8601(time_from_nanos) == Utc.to_iso_8601(now))?

	Ok({})
}

test_time_delta! : () => Try({}, _)
test_time_delta! = || {
	Stdout.line!("\nTime delta demonstration:")?

	start = Utc.now!()
	Stdout.line!("Starting time: ${Utc.to_iso_8601(start)}")?

	Sleep.millis!(1500)

	finish = Utc.now!()
	Stdout.line!("Ending time: ${Utc.to_iso_8601(finish)}")?

	err_on_false(Utc.to_millis_since_epoch(finish) > Utc.to_millis_since_epoch(start))?

	delta_millis = Utc.delta_as_millis(start, finish)
	Stdout.line!("Time elapsed: ${delta_millis.to_str()} milliseconds")?

	delta_nanos = Utc.delta_as_nanos(start, finish)
	Stdout.line!("Time elapsed: ${delta_nanos.to_str()} nanoseconds")?

	err_on_false(delta_millis > 0)?
	err_on_false(delta_nanos > 0)?
	err_on_false(delta_nanos >= delta_millis * 1_000_000)?

	calculated_millis = delta_nanos // 1_000_000
	Stdout.line!("Nanoseconds converted to milliseconds: ${calculated_millis.to_str()}.0")?

	difference = 
		if calculated_millis > delta_millis {
			calculated_millis - delta_millis
		} else {
			delta_millis - calculated_millis
		}
	err_on_false(difference < 1)?

	Stdout.line!("Verified: deltaMillis and deltaNanos/1_000_000 match within tolerance")?
	Ok({})
}

err_on_false : Bool -> Try({}, [TestFailed, ..])
err_on_false = |condition|
	if condition {
		Ok({})
	} else {
		Err(TestFailed)
	}
