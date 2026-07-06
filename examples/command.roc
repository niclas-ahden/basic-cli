app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.Cmd

# Different ways to run commands like you do in a terminal.

main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args|
	match run!() {
		Ok(_) => Ok({})
		Err(CommandFailed(message)) => {
			Stdout.line!("Command example failed: ${message}") ? |_| Exit(1)
			Err(Exit(1))
		}
	}

run! : () => Try({}, [CommandFailed(Str)])
run! = || {
	# Simplest way to execute a command (prints to your terminal).
	Cmd.exec!("echo", ["Hello"]) ? |_| CommandFailed("exec echo failed")

	# To execute and capture the output (stdout and stderr) without inheriting your terminal.
	cmd_output =
		Cmd.new("echo")
			.args(["Hi"])
			.exec_output!() ? |_| CommandFailed("exec_output echo failed")

	Stdout.line!("${Str.inspect(cmd_output)}") ? |_| CommandFailed("stdout write failed")

	# To run a command with a controlled environment.
	Cmd.new("/usr/bin/env")
		.args(["-i", "BAZ=DUCK", "FOO=BAR", "XYZ=ABC"])
		.exec_cmd!() ? |_| CommandFailed("exec_cmd env failed")

	# To execute and just get the exit code (prints to your terminal).
	# Prefer using `exec!` or `exec_cmd!`.
	exit_code =
		Cmd.new("cat")
			.args(["non_existent.txt"])
			.exec_exit_code!() ? |_| CommandFailed("exec_exit_code cat failed")

	Stdout.line!("Exit code: ${exit_code.to_str()}") ? |_| CommandFailed("stdout write failed")

	# To execute and capture the output (stdout and stderr) in the original form as bytes without inheriting your terminal.
	# Prefer using `exec_output!`.
	cmd_output_bytes =
		Cmd.new("echo")
			.args(["Hi"])
			.exec_output_bytes!() ? |_| CommandFailed("exec_output_bytes echo failed")

	Stdout.line!("${Str.inspect(cmd_output_bytes)}") ? |_| CommandFailed("stdout write failed")

	Ok({})
}
