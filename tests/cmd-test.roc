app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.Cmd
import pf.IOErr exposing [IOErr]

main! : List(OsStr) => Try({}, [Exit(I32)])
main! = |_args|
	match run!() {
		Ok({}) => Ok({})
		Err(FailedExpectation(message)) => {
			Stdout.line!(message) ? |_| Exit(1)
			Err(Exit(1))
		}
		Err(other) => {
			Stdout.line!("Unexpected test error: ${Str.inspect(other)}") ? |_| Exit(1)
			Err(Exit(1))
		}
	}

run! = || {
	check_exec_missing!(Cmd.exec!("blablaXYZ", []))?
	check_exec_nonzero!(Cmd.exec!("sh", ["-c", "exit 7"]))?

	check_exec_cmd_missing!(Cmd.new("blablaXYZ").exec_cmd!())?
	check_exec_cmd_nonzero!(Cmd.new("sh").args(["-c", "exit 7"]).exec_cmd!())?

	check_output_missing!(Cmd.new("blablaXYZ").exec_output!())?
	check_output_nonzero!(
		Cmd.new("sh")
			.args(["-c", "printf out; printf err >&2; exit 7"])
			.exec_output!(),
	)?
	check_output_invalid_utf8!(
		Cmd.new("sh")
			.args(["-c", "printf '\\377\\376'"])
			.exec_output!(),
	)?

	check_output_bytes_missing!(Cmd.new("blablaXYZ").exec_output_bytes!())?
	check_output_bytes_nonzero!(
		Cmd.new("sh")
			.args(["-c", "printf out; printf err >&2; exit 7"])
			.exec_output_bytes!(),
	)?

	check_exit_code_missing!(Cmd.new("blablaXYZ").exec_exit_code!())?
	exit_code = Cmd.new("sh").args(["-c", "exit 7"]).exec_exit_code!()?
	expect_i32!(exit_code, 7)?

	Stdout.line!("All tests passed.")?
	Ok({})
}

check_exec_missing! : Try({}, [ExecFailed({ command : Str, exit_code : I32 }), FailedToGetExitCode({ command : Str, err : IOErr }), ..]) => Try({}, [FailedExpectation(Str), ..])
check_exec_missing! = |result|
	match result {
		Err(FailedToGetExitCode({ command, err: NotFound })) =>
			expect_str!(command, "{ cmd: blablaXYZ, args:  }")
		other => fail!("Cmd.exec! missing command returned ${Str.inspect(other)}")
	}

check_exec_nonzero! : Try({}, [ExecFailed({ command : Str, exit_code : I32 }), FailedToGetExitCode({ command : Str, err : IOErr }), ..]) => Try({}, [FailedExpectation(Str), ..])
check_exec_nonzero! = |result|
	match result {
		Err(ExecFailed({ command, exit_code })) => {
			expect_str!(command, "sh -c exit 7")?
			expect_i32!(exit_code, 7)
		}
		other => fail!("Cmd.exec! non-zero command returned ${Str.inspect(other)}")
	}

check_exec_cmd_missing! : Try({}, [ExecCmdFailed({ command : Str, exit_code : I32 }), FailedToGetExitCode({ command : Str, err : IOErr }), ..]) => Try({}, [FailedExpectation(Str), ..])
check_exec_cmd_missing! = |result|
	match result {
		Err(FailedToGetExitCode({ command, err: NotFound })) =>
			expect_str!(command, "{ cmd: blablaXYZ, args:  }")
		other => fail!("Cmd.exec_cmd! missing command returned ${Str.inspect(other)}")
	}

check_exec_cmd_nonzero! : Try({}, [ExecCmdFailed({ command : Str, exit_code : I32 }), FailedToGetExitCode({ command : Str, err : IOErr }), ..]) => Try({}, [FailedExpectation(Str), ..])
check_exec_cmd_nonzero! = |result|
	match result {
		Err(ExecCmdFailed({ command, exit_code })) => {
			expect_str!(command, "{ cmd: sh, args: -c exit 7 }")?
			expect_i32!(exit_code, 7)
		}
		other => fail!("Cmd.exec_cmd! non-zero command returned ${Str.inspect(other)}")
	}

check_output_missing! : Try({ stdout_utf8 : Str, stderr_utf8_lossy : Str }, [StdoutContainsInvalidUtf8({ cmd_str : Str, err : [BadUtf8({ problem : _, index : U64 })] }), NonZeroExitCode({ command : Str, exit_code : I32, stdout_utf8_lossy : Str, stderr_utf8_lossy : Str }), FailedToGetExitCode({ command : Str, err : IOErr }), ..]) => Try({}, [FailedExpectation(Str), ..])
check_output_missing! = |result|
	match result {
		Err(FailedToGetExitCode({ command, err: NotFound })) =>
			expect_str!(command, "{ cmd: blablaXYZ, args:  }")
		other => fail!("Cmd.exec_output! missing command returned ${Str.inspect(other)}")
	}

check_output_nonzero! : Try({ stdout_utf8 : Str, stderr_utf8_lossy : Str }, [StdoutContainsInvalidUtf8({ cmd_str : Str, err : [BadUtf8({ problem : _, index : U64 })] }), NonZeroExitCode({ command : Str, exit_code : I32, stdout_utf8_lossy : Str, stderr_utf8_lossy : Str }), FailedToGetExitCode({ command : Str, err : IOErr }), ..]) => Try({}, [FailedExpectation(Str), ..])
check_output_nonzero! = |result|
	match result {
		Err(NonZeroExitCode({ exit_code, stdout_utf8_lossy, stderr_utf8_lossy, .. })) => {
			expect_i32!(exit_code, 7)?
			expect_str!(stdout_utf8_lossy, "out")?
			expect_str!(stderr_utf8_lossy, "err")
		}
		other => fail!("Cmd.exec_output! non-zero command returned ${Str.inspect(other)}")
	}

check_output_invalid_utf8! : Try({ stdout_utf8 : Str, stderr_utf8_lossy : Str }, [StdoutContainsInvalidUtf8({ cmd_str : Str, err : [BadUtf8({ problem : _, index : U64 })] }), NonZeroExitCode({ command : Str, exit_code : I32, stdout_utf8_lossy : Str, stderr_utf8_lossy : Str }), FailedToGetExitCode({ command : Str, err : IOErr }), ..]) => Try({}, [FailedExpectation(Str), ..])
check_output_invalid_utf8! = |result|
	match result {
		Err(StdoutContainsInvalidUtf8({ err: BadUtf8({ index, problem: InvalidStartByte }), .. })) =>
			expect_u64!(index, 0)
		other => fail!("Cmd.exec_output! invalid UTF-8 returned ${Str.inspect(other)}")
	}

check_output_bytes_missing! : Try({ stderr_bytes : List(U8), stdout_bytes : List(U8) }, [NonZeroExitCodeB({ exit_code : I32, stdout_bytes : List(U8), stderr_bytes : List(U8) }), FailedToGetExitCodeB(IOErr), ..]) => Try({}, [FailedExpectation(Str), ..])
check_output_bytes_missing! = |result|
	match result {
		Err(FailedToGetExitCodeB(NotFound)) => Ok({})
		other => fail!("Cmd.exec_output_bytes! missing command returned ${Str.inspect(other)}")
	}

check_output_bytes_nonzero! : Try({ stderr_bytes : List(U8), stdout_bytes : List(U8) }, [NonZeroExitCodeB({ exit_code : I32, stdout_bytes : List(U8), stderr_bytes : List(U8) }), FailedToGetExitCodeB(IOErr), ..]) => Try({}, [FailedExpectation(Str), ..])
check_output_bytes_nonzero! = |result|
	match result {
		Err(NonZeroExitCodeB({ exit_code, stdout_bytes, stderr_bytes })) => {
			expect_i32!(exit_code, 7)?
			expect_bytes!(stdout_bytes, [111, 117, 116])?
			expect_bytes!(stderr_bytes, [101, 114, 114])
		}
		other => fail!("Cmd.exec_output_bytes! non-zero command returned ${Str.inspect(other)}")
	}

check_exit_code_missing! : Try(I32, [FailedToGetExitCode({ command : Str, err : IOErr }), ..]) => Try({}, [FailedExpectation(Str), ..])
check_exit_code_missing! = |result|
	match result {
		Err(FailedToGetExitCode({ command, err: NotFound })) =>
			expect_str!(command, "{ cmd: blablaXYZ, args:  }")
		other => fail!("Cmd.exec_exit_code! missing command returned ${Str.inspect(other)}")
	}

expect_str! = |actual, expected|
	if actual == expected {
		Ok({})
	} else {
		fail!("Expected `${expected}`, got `${actual}`")
	}

expect_i32! = |actual, expected|
	if actual == expected {
		Ok({})
	} else {
		fail!("Expected ${I32.to_str(expected)}, got ${I32.to_str(actual)}")
	}

expect_u64! = |actual, expected|
	if actual == expected {
		Ok({})
	} else {
		fail!("Expected ${U64.to_str(expected)}, got ${U64.to_str(actual)}")
	}

expect_bytes! = |actual, expected|
	if actual == expected {
		Ok({})
	} else {
		fail!("Expected ${Str.inspect(expected)}, got ${Str.inspect(actual)}")
	}

fail! = |message| Err(FailedExpectation(message))
