## Long-lived child processes that cannot outlive this program, with
## [Cmd.spawn_leashed!]. A leashed child, and its whole process tree, is taken
## down when this program ends, including on the deaths that skip cleanup such
## as Ctrl+C, a crash, or `kill -9`. Use it for a server or driver you
## supervise, so a cancelled run never leaves a stray behind.
app [main!] { pf: platform "https://github.com/niclas-ahden/basic-cli/releases/download/0.26.0/EuuihZ91yAY1ANck1QytRBcW2jexEfH6yVmxcPCHEHDz.tar.zst" }

import pf.Cmd
import pf.OsStr exposing [OsStr]
import pf.Stdout

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	# --- talk to a leashed child over pipes ---
	# Default streams are inherited, exactly as spawn!, so ask for pipes.
	cat = Cmd.new_str("cat").stdin(Pipe).stdout(Pipe).spawn_leashed!() ? |e| SpawnFailed(e)
	cat.write!(Str.to_utf8("hello, leash!\n"), 1_000) ? |e| WriteFailed(e)
	cat.close_stdin!() ? |e| CloseStdinFailed(e)
	echoed = read_all!(cat, []) ? |e| ReadFailed(e)
	Stdout.line!("cat echoed: ${Str.from_utf8_lossy(echoed).trim()}")?
	_ = cat.wait!() ? |e| WaitFailed(e)

	# --- a leashed child reports its own exit status ---
	coder = Cmd.new_str("sh").args_str(["-c", "exit 7"]).spawn_leashed!() ? |e| SpawnFailed(e)
	done = coder.wait!() ? |e| WaitFailed(e)
	Stdout.line!("leashed exit: ${Str.inspect(done.status)}")?

	# --- close! takes a running leashed tree down at once ---
	# The child backgrounds a grandchild, so this proves the whole group goes,
	# not just the direct child. `up` on stdout is the readiness signal.
	server =
		Cmd.new_str("sh")
			.args_str(["-c", "sleep 100 & echo up; wait"])
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
	match child.read!(1_024, 1_000)? {
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
		match child.read!(1_024, 1_000)? {
			Stdout(chunk) => read_all_until_newline!(child, acc.concat(chunk))
			Stderr(_) => read_all_until_newline!(child, acc)
			End => Ok(acc)
		}
	}
