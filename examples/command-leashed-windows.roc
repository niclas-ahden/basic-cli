## Long-lived child processes that cannot outlive this program, with
## [Cmd.spawn_leashed!], driven with Windows commands. On Windows the leash is
## a Job Object that dies with this program, so the whole tree goes on any
## exit, including a crash or an outside TerminateProcess.
## examples/command-leashed.roc covers the same ground with Unix commands;
## this one runs only on Windows.
app [main!] { pf: platform "https://github.com/niclas-ahden/basic-cli/releases/download/0.26.0/EuuihZ91yAY1ANck1QytRBcW2jexEfH6yVmxcPCHEHDz.tar.zst" }

import pf.Cmd
import pf.OsStr exposing [OsStr]
import pf.Stdout

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	# --- a leashed child reports its own exit status and output ---
	# Default streams are inherited, exactly as spawn!, so ask for capture.
	greeter =
		Cmd.new_str("cmd")
			.args_str(["/c", "echo hello"])
			.stdout(Capture)
			.spawn_leashed!() ? |e| SpawnFailed(e)
	greeted = greeter.wait!() ? |e| WaitFailed(e)
	Stdout.line!("echo exit: ${Str.inspect(greeted.status)}")?
	Stdout.line!("echo said: ${Str.from_utf8_lossy(greeted.stdout_bytes).trim()}")?

	# --- talk to a leashed child over pipes ---
	sorter = Cmd.new_str("sort").stdin(Pipe).stdout(Pipe).spawn_leashed!() ? |e| SpawnFailed(e)
	sorter.write!(Str.to_utf8("banana\r\napple\r\ncherry\r\n"), 1_000) ? |e| WriteFailed(e)
	sorter.close_stdin!() ? |e| CloseStdinFailed(e)
	sorted = read_all!(sorter, []) ? |e| ReadFailed(e)
	Stdout.line!("sort said: ${Str.from_utf8_lossy(sorted).trim()}")?
	_ = sorter.wait!() ? |e| WaitFailed(e)

	# --- close! takes a running leashed tree down at once ---
	# The child starts a grandchild, so this proves the whole job goes, not
	# just the direct child. `up` on stdout is the readiness signal.
	server =
		Cmd.new_str("powershell")
			.args_str([
				"-NoProfile",
				"-Command",
				"Start-Process ping -ArgumentList '-n','600','127.0.0.1' -WindowStyle Hidden; Write-Output up; Start-Sleep -Seconds 600",
			])
			.stdout(Pipe)
			.spawn_leashed!() ? |e| SpawnFailed(e)
	_marker = read_all_until_newline!(server, []) ? |e| ReadFailed(e)
	server.close!() ? |e| CloseFailed(e)
	Stdout.line!("leashed tree closed")?

	Ok({})
}

## Drain a child's stdout to EOF, concatenating the chunks. read! hands back
## at most the requested bytes, or End once every write end has closed.
read_all! : Cmd.Child, List(U8) => Try(List(U8), _)
read_all! = |child, acc|
	match child.read!(1_024, 5_000)? {
		Stdout(chunk) => read_all!(child, acc.concat(chunk))
		Stderr(_) => read_all!(child, acc)
		End => Ok(acc)
	}

## Read a child's stdout only up to and including the first newline: enough to
## catch a one-line readiness marker without waiting for the child to exit.
read_all_until_newline! : Cmd.Child, List(U8) => Try(List(U8), _)
read_all_until_newline! = |child, acc|
	if acc.contains(10) {
		Ok(acc)
	} else {
		match child.read!(1_024, 10_000)? {
			Stdout(chunk) => read_all_until_newline!(child, acc.concat(chunk))
			Stderr(_) => read_all_until_newline!(child, acc)
			End => Ok(acc)
		}
	}
