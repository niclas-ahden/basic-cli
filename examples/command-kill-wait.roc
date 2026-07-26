app [main!] { pf: platform "../platform/main.roc" }

import pf.Cmd
import pf.OsStr exposing [OsStr]
import pf.Stdout

# Stopping a child while keeping what it printed, with Cmd.Child.kill_wait!.
#
# Each case below synchronises on the child's own output rather than on a sleep,
# so the results are the same on a fast laptop and a loaded CI runner.

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	# --- output written before the kill survives ---
	# Reading the marker off stderr proves the stdout write already happened,
	# because the child writes them in order.
	talker = grouped!("echo out-before-kill; echo sync >&2; sleep 100")?
	_sync = talker.read_stderr!(5)?
	stopped = talker.kill_wait!()?
	Stdout.line!("killed exit code: ${stopped.exit_code.to_str()}")?
	Stdout.line!("killed stdout: ${Str.from_utf8_lossy(stopped.stdout)}")?

	# --- and so does stderr, with the marker on stdout this time ---
	complainer = grouped!("echo err-before-kill >&2; echo sync; sleep 100")?
	_sync2 = complainer.read_stdout!(5)?
	complained = complainer.kill_wait!()?
	Stdout.line!("killed stderr: ${Str.from_utf8_lossy(complained.stderr)}")?

	# --- a child that beat us to the exit keeps its own exit code ---
	# Reading past end of file is how we know the child is gone without reaping
	# it, and it leaves the buffered output in place for kill_wait!.
	racer = grouped!("echo raced; exit 7")?
	_eof = racer.read_stdout!(1000)
	raced = racer.kill_wait!()?
	Stdout.line!("raced exit code: ${raced.exit_code.to_str()}, stdout: ${Str.from_utf8_lossy(raced.stdout)}")?

	# --- a silent child gives back empty output, not an error ---
	silent = grouped!("sleep 100")?
	quiet = silent.kill_wait!()?
	Stdout.line!("silent: exit ${quiet.exit_code.to_str()}, ${quiet.stdout.len().to_str()} out bytes, ${quiet.stderr.len().to_str()} err bytes")?

	# --- more output than a pipe buffer holds still arrives whole ---
	noisy = grouped!("yes hello | head -c 500000; echo sync >&2; sleep 100")?
	_sync3 = noisy.read_stderr!(5)?
	loud = noisy.kill_wait!()?
	Stdout.line!("noisy: exit ${loud.exit_code.to_str()}, ${loud.stdout.len().to_str()} bytes")?

	# --- a grouped child takes its own children down with it ---
	forker = grouped!("sleep 100 & echo forked; sleep 100")?
	_sync4 = forker.read_stdout!(7)?
	forked = forker.kill_wait!()?
	Stdout.line!("forked: exit ${forked.exit_code.to_str()}, tree is gone")?

	# --- a child that leaves something behind does not stall the kill ---
	# spawn! kills only the child, so the backgrounded sleep lives on holding the
	# pipes it inherited. kill_wait! collects what the child itself wrote and
	# returns rather than waiting for those pipes to close. Anything the survivor
	# writes from here on is lost, which is the price of not waiting for it.
	orphaner = alone!("sleep 5 & echo out-before-kill; echo sync >&2; sleep 5")?
	_sync5 = orphaner.read_stderr!(5)?
	orphaned = orphaner.kill_wait!()?
	Stdout.line!("orphaner: exit ${orphaned.exit_code.to_str()}, stdout: ${Str.from_utf8_lossy(orphaned.stdout)}")?

	# --- a child that was already collected reports an error, it does not hang ---
	reaped = grouped!("exit 0")?
	_ = reaped.wait!()?
	match reaped.kill_wait!() {
		Ok(_) => Stdout.line!("second kill_wait: unexpectedly Ok")?
		Err(_) => Stdout.line!("second kill_wait: Err as expected")?
	}

	Ok({})
}

## Run a shell script in a process group, so killing it kills the whole tree.
grouped! : Str => Try(Cmd.Child, _)
grouped! = |script|
	Cmd.new_str("sh").args_str(["-c", script]).spawn_grouped!()

## Run a shell script on its own, leaving any children it spawns behind.
alone! : Str => Try(Cmd.Child, _)
alone! = |script|
	Cmd.new_str("sh").args_str(["-c", script]).spawn!()
