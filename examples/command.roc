app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.Cmd

# Different ways to run commands like you do in a terminal.

main! : List(OsStr) => Try({}, _)
main! = |_args| run!()

run! : () => Try({}, _)
run! = || {
	# Simplest way to execute a command (prints to your terminal).
	Cmd.exec!("echo", ["Hello"]) ? |err| ExecEchoFailed(err)

	# To execute and capture the output (stdout and stderr) without inheriting your terminal.
	cmd_output =
		Cmd.new("echo")
			.args(["Hi"])
			.exec_output!() ? |err| ExecOutputEchoFailed(err)

	Stdout.line!("${Str.inspect(cmd_output)}")?

	# To run a command with a controlled environment.
	Cmd.new("/usr/bin/env")
		.args(["-i", "BAZ=DUCK", "FOO=BAR", "XYZ=ABC"])
		.exec_cmd!() ? |err| ExecCmdEnvFailed(err)

	# To execute and just get the exit code (prints to your terminal).
	# Prefer using `exec!` or `exec_cmd!`.
	exit_code =
		Cmd.new("cat")
			.args(["non_existent.txt"])
			.exec_exit_code!() ? |err| ExecExitCodeCatFailed(err)

	Stdout.line!("Exit code: ${exit_code.to_str()}")?

	# To execute and capture the output (stdout and stderr) in the original form as bytes without inheriting your terminal.
	# Prefer using `exec_output!`.
	cmd_output_bytes =
		Cmd.new("echo")
			.args(["Hi"])
			.exec_output_bytes!() ? |err| ExecOutputBytesEchoFailed(err)

	Stdout.line!("${Str.inspect(cmd_output_bytes)}")?

	Ok({})
}
