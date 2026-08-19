app [main!] { pf: platform "https://github.com/niclas-ahden/basic-cli/releases/download/0.24.0/2mx1EsQx1HEG7HdbW2CwUpexvmJZW4nSCpjbur5GXyRe.tar.zst" }

import pf.Cmd
import pf.OsStr exposing [OsStr]
import pf.Stdout

# The child process API on Windows, where the process tree is a job object
# rather than a process group.
#
# examples/command-spawn.roc and examples/command-kill-wait.roc cover the same
# ground with Unix commands. This one drives cmd and powershell instead, and
# runs only on Windows.

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	# --- spawn!, then collect the output with wait! ---
	greeter = Cmd.new_str("cmd").args_str(["/c", "echo hello"]).spawn_leashed!()?
	greeted = greeter.wait!()?
	Stdout.line!("echo exit code: ${greeted.exit_code.to_str()}")?
	Stdout.line!("echo said: ${Str.from_utf8_lossy(greeted.stdout).trim()}")?

	# --- write to stdin, close it, read what the child made of it ---
	sorter = Cmd.new_str("sort").spawn_leashed!()?
	sorter.write_stdin!(Str.to_utf8("banana\r\napple\r\ncherry\r\n"))?
	sorter.close_stdin!()?
	sorted = sorter.wait!()?
	Stdout.line!("sort exit code: ${sorted.exit_code.to_str()}")?
	Stdout.line!("sort output: ${Str.from_utf8_lossy(sorted.stdout)}")?

	# --- kill_wait! stops a running child and keeps what it printed ---
	# Reading the marker is the synchronisation point, so no sleeping in the
	# parent. A killed job object reports exit code 1, not the -1 that a signal
	# gives on Unix.
	waiter = Cmd.new_str("powershell")
		.args_str(["-NoProfile", "-Command", "Write-Output ready; Start-Sleep 100"])
		.spawn_leashed!()?
	_marker = waiter.read_stdout!(7)?
	stopped = waiter.kill_wait!()?
	Stdout.line!("killed exit code: ${stopped.exit_code.to_str()}")?

	# --- leashed children get cleaned up en masse ---
	_lingerer = Cmd.new_str("powershell")
		.args_str(["-NoProfile", "-Command", "Start-Sleep 100"])
		.spawn_leashed!()?
	Cmd.kill_leashed!({})?
	Stdout.line!("all leashed children killed")?

	Ok({})
}
