app [main!] { pf: platform "https://github.com/niclas-ahden/basic-cli/releases/download/0.22.1/DobkAk7zNyqAgqh2Riaj5c5DtWtKhd5iVYE5RFa6izcd.tar.zst" }

import pf.Cmd
import pf.OsStr exposing [OsStr]
import pf.Sleep
import pf.Stdout

# Interactive child processes: spawn `cat` with piped stdio, talk to it both
# ways, then exercise close_stdin/wait, poll, and kill.

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	# --- spawn! + write/read/kill ---
	child = Cmd.new_str("cat").spawn!()?
	child.write_stdin!(Str.to_utf8("hello, pipes!\n"))?
	echoed = child.read_stdout!(14)?
	Stdout.line!("cat echoed: ${Str.from_utf8_lossy(echoed)}")?
	child.kill!()?

	# --- close_stdin! signals EOF; wait! collects output and exit code ---
	counter = Cmd.new_str("sort").spawn!()?
	counter.write_stdin!(Str.to_utf8("banana\napple\ncherry\n"))?
	counter.close_stdin!()?
	result = counter.wait!()?
	Stdout.line!("sort exit code: ${result.exit_code.to_str()}")?
	Stdout.line!("sort output:\n${Str.from_utf8_lossy(result.stdout)}")?

	# --- poll! on a short-lived grouped child ---
	sleeper = Cmd.new_str("sh").args_str(["-c", "exit 7"]).spawn_grouped!()?
	poll_loop!(sleeper, 0)?

	# --- stderr capture ---
	errorer = Cmd.new_str("sh").args_str(["-c", "echo oops >&2; exit 1"]).spawn!()?
	err_out = errorer.read_stderr!(5)?
	Stdout.line!("stderr said: ${Str.from_utf8_lossy(err_out)}")?
	_ = errorer.wait!()?

	# --- grouped children get killed en masse ---
	_lingerer = Cmd.new_str("sleep").args_str(["100"]).spawn_grouped!()?
	Cmd.kill_grouped!({})?
	Stdout.line!("all grouped children killed")?

	Ok({})
}

poll_loop! : Cmd.Child, U64 => Try({}, _)
poll_loop! = |child, attempts| {
	if attempts > 100 {
		Stdout.line!("gave up polling")
	} else {
		match child.poll!()? {
			Exited(out) =>
				Stdout.line!("poll saw exit code ${out.exit_code.to_str()} after ${attempts.to_str()} polls")
			Running => {
				Sleep.millis!(10)
				poll_loop!(child, attempts + 1)
			}
		}
	}
}
