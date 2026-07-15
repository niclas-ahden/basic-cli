import IOErr exposing [IOErr]
import Host
import OsStr exposing [OsStr]

## Build and run child processes with native-safe programs, arguments, and
## environment values.
Cmd :: {
	args : List(OsStr),
	clear_envs : Bool,
	envs : List((OsStr, OsStr)),
	program : OsStr,
}.{

	## Simplest way to execute a command by name with arguments.
	## Stdin, stdout, and stderr are inherited from the parent process.
	##
	## If you want to capture the output, use [exec_output!] instead.
	##
	## ```roc
	## Cmd.exec!("echo", ["hello world"])?
	## ```
	exec! : OsStr, List(OsStr) => Try({}, [ExecFailed({ command : Str, exit_code : I32 }), FailedToGetExitCode({ command : Str, err : IOErr }), ..])
	exec! = |program, arguments| {
		command = "${OsStr.display(program)} ${Str.join_with(arguments.map(OsStr.display), " ")}"

		exit_code = new(program)
			.args(arguments)
			.exec_exit_code!()?

		if exit_code == 0 {
			Ok({})
		} else {
			Err(ExecFailed({ command, exit_code }))
		}
	}

	## Execute a Cmd (using the builder pattern).
	## Stdin, stdout, and stderr are inherited from the parent process.
	##
	## You should prefer using [exec!] instead, only use this if you want to use [env], [envs] or [clear_envs].
	## If you want to capture the output, use [exec_output!] instead.
	##
	## ```roc
	## Cmd.new("cargo")
	##     .arg(["build")
	##     .env("RUST_BACKTRACE", "1")
	##     .exec_cmd!()?
	## ```
	exec_cmd! : Cmd => Try({}, [ExecCmdFailed({ command : Str, exit_code : I32 }), FailedToGetExitCode({ command : Str, err : IOErr }), ..])
	exec_cmd! = |cmd| {
		command = to_str(cmd)
		exit_code = exec_exit_code!(cmd)?

		if exit_code == 0 {
			Ok({})
		} else {
			Err(ExecCmdFailed({ command, exit_code }))
		}
	}

	## Execute command and capture stdout and stderr as UTF-8 strings.
	## Invalid UTF-8 sequences are replaced with the Unicode replacement character.
	##
	## Use [exec_output_bytes!] instead if you want to capture the output in the original form as bytes.
	## [exec_output_bytes!] may also be used for maximum performance, because you may be able to avoid unnecessary UTF-8 conversions.
	##
	## ```roc
	## cmd_output =
	##     Cmd.new("echo")
	##         .args(["Hi"])
	##         .exec_output!()?
	##
	## Stdout.line!("Echo output: ${cmd_output.stdout_utf8}")?
	## ```
	exec_output! : Cmd => Try({ stdout_utf8 : Str, stderr_utf8_lossy : Str }, [StdoutContainsInvalidUtf8({ cmd_str : Str, err : [BadUtf8({ problem : _, index : U64 })] }), NonZeroExitCode({ command : Str, exit_code : I32, stdout_utf8_lossy : Str, stderr_utf8_lossy : Str }), FailedToGetExitCode({ command : Str, err : IOErr }), ..])
	exec_output! = |cmd| {
		cmd_str = to_str(cmd)
		exec_try = Host.cmd_exec_output!(to_host_cmd(cmd))

		match exec_try {
			Ok({ stderr_bytes, stdout_bytes }) => {
				stdout_utf8 = Str.from_utf8(stdout_bytes)
					.map_err(|err| StdoutContainsInvalidUtf8({ cmd_str, err }))?

				stderr_utf8_lossy = Str.from_utf8_lossy(stderr_bytes)

				Ok({ stdout_utf8, stderr_utf8_lossy })
			}

			Err(NonZeroExitCode({ exit_code, stderr_bytes, stdout_bytes })) => {
				stdout_utf8_lossy = Str.from_utf8_lossy(stdout_bytes)
				stderr_utf8_lossy = Str.from_utf8_lossy(stderr_bytes)

				Err(NonZeroExitCode({ command: cmd_str, exit_code, stdout_utf8_lossy, stderr_utf8_lossy }))
			}

			Err(FailedToGetExitCode(err)) => Err(FailedToGetExitCode({ command: cmd_str, err }))
		}
	}

	## Execute command and capture stdout and stderr in the original form as bytes.
	##
	## Use [exec_output!] instead if you want to get the output as UTF-8 strings.
	##
	## ```roc
	## cmd_output =
	##     Cmd.new("echo")
	##         .args(["Hi"])
	##         .exec_output_bytes!()?
	##
	## Stdout.line!("${Str.inspect(cmd_output_bytes)}")? # {stderr_bytes: [], stdout_bytes: [72, 105, 10]}
	## ```
	exec_output_bytes! : Cmd => Try({ stderr_bytes : List(U8), stdout_bytes : List(U8) }, [NonZeroExitCodeB({ exit_code : I32, stdout_bytes : List(U8), stderr_bytes : List(U8) }), FailedToGetExitCodeB(IOErr), ..])
	exec_output_bytes! = |cmd| {
		exec_try = Host.cmd_exec_output!(to_host_cmd(cmd))

		match exec_try {
			Ok({ stderr_bytes, stdout_bytes }) =>
				Ok({ stdout_bytes, stderr_bytes })

			Err(NonZeroExitCode({ exit_code, stderr_bytes, stdout_bytes })) => {
				Err(NonZeroExitCodeB({ exit_code, stdout_bytes, stderr_bytes }))
			}

			Err(FailedToGetExitCode(err)) => {
				Err(FailedToGetExitCodeB(err))
			}
		}
	}

	## Execute a command and return its exit code.
	## Stdin, stdout, and stderr are inherited from the parent process.
	##
	## You should prefer using [exec!] or [exec_cmd!] instead, only use this if you want to take a specific action based on a **specific non-zero exit code**.
	## For example, `roc check` returns exit code 1 if there are errors, and exit code 2 if there are only warnings.
	## So, you could use `exec_exit_code!` to ignore warnings on `roc check`.
	##
	## ```roc
	## exit_code = Cmd.new("cat").arg("non_existent.txt").exec_exit_code!()?
	## ```
	exec_exit_code! : Cmd => Try(I32, [FailedToGetExitCode({ command : Str, err : IOErr }), ..])
	exec_exit_code! = |cmd| {
		command = to_str(cmd)

		match Host.cmd_exec_exit_code!(to_host_cmd(cmd)) {
			Ok(num) => Ok(num)
			Err(io_err) => Err(FailedToGetExitCode({ command, err: io_err }))
		}
	}

	## Create a new command with the given program name. Use a function that starts with `exec_` to execute it.
	##
	## ```roc
	## cmd = Cmd.new("ls")
	## ```
	new : OsStr -> Cmd
	new = |program| {
		args: [],
		clear_envs: Bool.False,
		envs: [],
		program,
	}

	## Create a new command from a Roc string.
	new_str : Str -> Cmd
	new_str = |program| new(OsStr.from_str(program))

	## Add a single argument to the command.
	## ❗ Shell features like variable subsitition (e.g. `$FOO`), glob patterns (e.g. `*.txt`), ... are not available.
	##
	## ```roc
	## cmd = Cmd.new("ls").arg("-l")
	## ```
	arg : Cmd, OsStr -> Cmd
	arg = |cmd, a| {
		..cmd,
		args: cmd.args.append(a),
	}

	## Add a single string argument to the command.
	arg_str : Cmd, Str -> Cmd
	arg_str = |cmd, a| arg(cmd, OsStr.from_str(a))

	## Add multiple arguments to the command.
	## ❗ Shell features like variable subsitition (e.g. `$FOO`), glob patterns (e.g. `*.txt`), ... are not available.
	##
	## ```roc
	## cmd = Cmd.new("ls").args(["-l", "-a"])
	## ```
	args : Cmd, List(OsStr) -> Cmd
	args = |cmd, new_args| {
		..cmd,
		args: cmd.args.concat(new_args),
	}

	## Add multiple string arguments to the command.
	args_str : Cmd, List(Str) -> Cmd
	args_str = |cmd, new_args| args(cmd, new_args.map(OsStr.from_str))

	## Add a single environment variable to the command.
	##
	##
	## ```roc
	## cmd = Cmd.new("env").env("FOO", "bar") # add the environment variable "FOO" with value "bar"
	## ```
	env : Cmd, OsStr, OsStr -> Cmd
	env = |cmd, key, value| {
		{ ..cmd, envs: cmd.envs.append((key, value)) }
	}

	## Add a single string environment variable to the command.
	env_str : Cmd, Str, Str -> Cmd
	env_str = |cmd, key, value| env(cmd, OsStr.from_str(key), OsStr.from_str(value))

	## Add multiple environment variables to the command.
	##
	## ```roc
	## cmd = Cmd.new("env").envs([("FOO", "bar"), ("BAZ", "qux")])
	## ```
	envs : Cmd, List((OsStr, OsStr)) -> Cmd
	envs = |cmd, pairs| { ..cmd, envs: cmd.envs.concat(pairs) }

	## Add multiple string environment variables to the command.
	envs_str : Cmd, List((Str, Str)) -> Cmd
	envs_str = |cmd, pairs| {
		arg_pairs = pairs.map(|(key, value)| (OsStr.from_str(key), OsStr.from_str(value)))
		envs(cmd, arg_pairs)
	}

	## Clear all environment variables before running the command.
	## Only environment variables added via `env` or `envs` will be available.
	## Useful if you want a clean command run that does not behave unexpectedly if the user has some env var set.
	##
	## ```roc
	## cmd =
	##     Cmd.new("env")
	##         .clear_envs()
	##         .env("ONLY_THIS", "visible")
	## ```
	clear_envs : Cmd -> Cmd
	clear_envs = |cmd| { ..cmd, clear_envs: Bool.True }

	## Render a command configuration as a stable, escaped string.
	to_str : Cmd -> Str
	to_str = |cmd|
		"Cmd({ program: ${Str.inspect(cmd.program)}, args: ${Str.inspect(cmd.args)}, envs: ${Str.inspect(cmd.envs)}, clear_envs: ${Str.inspect(cmd.clear_envs)} })"

	## Customize command output for `Str.inspect`.
	to_inspect : Cmd -> Str
	to_inspect = |cmd| to_str(cmd)
}

flatten_arg_pairs : List((OsStr, OsStr)), List(OsStr), U64 -> List(OsStr)
flatten_arg_pairs = |pairs, acc, idx| {
	if idx >= pairs.len() {
		acc
	} else {
		match pairs.get(idx) {
			Ok(pair) =>
				flatten_arg_pairs(pairs, acc.append(pair.0).append(pair.1), idx + 1)
			Err(_) =>
				acc
			}
	}
}

to_host_cmd : Cmd -> Host.Cmd
to_host_cmd = |cmd| {
	args: cmd.args.map(OsStr.to_raw),
	clear_envs: cmd.clear_envs,
	envs: flatten_arg_pairs(cmd.envs, [], 0).map(OsStr.to_raw),
	program: OsStr.to_raw(cmd.program),
}

## Inspection is escaped and includes the full immutable command configuration.
expect {
	cmd = Cmd.new_str("echo\nnext")
		.arg_str("hello world")
		.env_str("NAME", "Roc")
		.clear_envs()

	Str.inspect(cmd) == "Cmd({ program: OsStr.utf8(\"echo\\nnext\"), args: [OsStr.utf8(\"hello world\")], envs: [(OsStr.utf8(\"NAME\"), OsStr.utf8(\"Roc\"))], clear_envs: True })"
}
