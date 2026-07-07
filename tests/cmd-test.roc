app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.Cmd
import pf.IOErr exposing [IOErr]

# Tests command error cases by matching result tags directly. Runtime execution
# is not enabled in CI yet: optimized builds are blocked by
# https://github.com/roc-lang/roc/issues/10003, and dev runs still segfault after
# the assertions complete.

main! : List(Str) => Try({}, [Exit(I32)])
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

run! : () => Try(
    {},
    [
        FailedExpectation(Str),
        FailedToGetExitCode({ command : Str, err : IOErr }),
        StdoutErr(IOErr),
        ..
    ],
)
run! = || {
    test_exec_not_found!()?
    test_exec_nonzero!()?
    test_exec_cmd_not_found!()?
    test_exec_cmd_nonzero!()?
    test_exec_output_not_found!()?
    test_exec_output_nonzero!()?
    test_exec_output_invalid_utf8!()?
    test_exec_output_bytes_not_found!()?
    test_exec_output_bytes_nonzero!()?
    test_exec_exit_code_not_found!()?
    test_exec_exit_code_nonzero_is_ok!()?

    Stdout.line!("All tests passed.")?

    Ok({})
}

test_exec_not_found! = || {
    match Cmd.exec!("blablaXYZ", []) {
        Err(FailedToGetExitCode({ command, err: NotFound })) =>
            expect_str!(command, "{ cmd: blablaXYZ, args:  }")
        other => fail("Cmd.exec! missing command returned ${Str.inspect(other)}"),
    }
}

test_exec_nonzero! = || {
    match Cmd.exec!("cat", ["non_existent.txt"]) {
        Err(ExecFailed({ command, exit_code })) => {
            expect_str!(command, "cat non_existent.txt")?
            expect_i32!(exit_code, 1)
        }
        other => fail("Cmd.exec! non-zero command returned ${Str.inspect(other)}"),
    }
}

test_exec_cmd_not_found! = || {
    result = Cmd.new("blablaXYZ").exec_cmd!()
    match result {
        Err(FailedToGetExitCode({ command, err: NotFound })) =>
            expect_str!(command, "{ cmd: blablaXYZ, args:  }")
        other => fail("Cmd.exec_cmd! missing command returned ${Str.inspect(other)}"),
    }
}

test_exec_cmd_nonzero! = || {
    result = Cmd.new("cat").arg("non_existent.txt").exec_cmd!()
    match result {
        Err(ExecCmdFailed({ command, exit_code })) => {
            expect_str!(command, "{ cmd: cat, args: non_existent.txt }")?
            expect_i32!(exit_code, 1)
        }
        other => fail("Cmd.exec_cmd! non-zero command returned ${Str.inspect(other)}"),
    }
}

test_exec_output_not_found! = || {
    result = Cmd.new("blablaXYZ").exec_output!()
    match result {
        Err(FailedToGetExitCode({ command, err: NotFound })) =>
            expect_str!(command, "{ cmd: blablaXYZ, args:  }")
        other => fail("Cmd.exec_output! missing command returned ${Str.inspect(other)}"),
    }
}

test_exec_output_nonzero! = || {
    result = Cmd.new("cat").arg("non_existent.txt").exec_output!()
    match result {
        Err(NonZeroExitCode({ command, exit_code, stdout_utf8_lossy, stderr_utf8_lossy })) => {
            expect_str!(command, "{ cmd: cat, args: non_existent.txt }")?
            expect_i32!(exit_code, 1)?
            expect_str!(stdout_utf8_lossy, "")?
            expect_str!(stderr_utf8_lossy, "cat: non_existent.txt: No such file or directory\n")
        }
        other => fail("Cmd.exec_output! non-zero command returned ${Str.inspect(other)}"),
    }
}

test_exec_output_invalid_utf8! = || {
    result = Cmd.new("printf").args(["\\377\\376"]).exec_output!()
    match result {
        Err(StdoutContainsInvalidUtf8({ cmd_str, err: BadUtf8({ index, problem: InvalidStartByte }) })) => {
            expect_str!(cmd_str, "{ cmd: printf, args: \\377\\376 }")?
            expect_u64!(index, 0)
        }
        other => fail("Cmd.exec_output! invalid UTF-8 returned ${Str.inspect(other)}"),
    }
}

test_exec_output_bytes_not_found! = || {
    result = Cmd.new("blablaXYZ").exec_output_bytes!()
    match result {
        Err(FailedToGetExitCodeB(NotFound)) => Ok({})
        other => fail("Cmd.exec_output_bytes! missing command returned ${Str.inspect(other)}"),
    }
}

test_exec_output_bytes_nonzero! = || {
    result = Cmd.new("cat").arg("non_existent.txt").exec_output_bytes!()
    match result {
        Err(NonZeroExitCodeB({ exit_code, stdout_bytes, stderr_bytes })) => {
            expect_i32!(exit_code, 1)?
            expect_bytes!(stdout_bytes, [])?
            expect_bytes!(
                stderr_bytes,
                [99, 97, 116, 58, 32, 110, 111, 110, 95, 101, 120, 105, 115, 116, 101, 110, 116, 46, 116, 120, 116, 58, 32, 78, 111, 32, 115, 117, 99, 104, 32, 102, 105, 108, 101, 32, 111, 114, 32, 100, 105, 114, 101, 99, 116, 111, 114, 121, 10],
            )
        }
        other => fail("Cmd.exec_output_bytes! non-zero command returned ${Str.inspect(other)}"),
    }
}

test_exec_exit_code_not_found! = || {
    result = Cmd.new("blablaXYZ").exec_exit_code!()
    match result {
        Err(FailedToGetExitCode({ command, err: NotFound })) =>
            expect_str!(command, "{ cmd: blablaXYZ, args:  }")
        other => fail("Cmd.exec_exit_code! missing command returned ${Str.inspect(other)}"),
    }
}

test_exec_exit_code_nonzero_is_ok! = || {
    exit_code =
        Cmd.new("cat")
            .arg("non_existent.txt")
            .exec_exit_code!()?

    expect_i32!(exit_code, 1)
}

expect_str! : Str, Str => Try({}, [FailedExpectation(Str), FailedToGetExitCode({ command : Str, err : IOErr }), StdoutErr(IOErr), ..])
expect_str! = |actual, expected|
    if actual == expected {
        Ok({})
    } else {
        fail("Expected `${expected}`, got `${actual}`")
    }

expect_i32! : I32, I32 => Try({}, [FailedExpectation(Str), FailedToGetExitCode({ command : Str, err : IOErr }), StdoutErr(IOErr), ..])
expect_i32! = |actual, expected|
    if actual == expected {
        Ok({})
    } else {
        fail("Expected ${I32.to_str(expected)}, got ${I32.to_str(actual)}")
    }

expect_u64! : U64, U64 => Try({}, [FailedExpectation(Str), FailedToGetExitCode({ command : Str, err : IOErr }), StdoutErr(IOErr), ..])
expect_u64! = |actual, expected|
    if actual == expected {
        Ok({})
    } else {
        fail("Expected ${U64.to_str(expected)}, got ${U64.to_str(actual)}")
    }

expect_bytes! : List(U8), List(U8) => Try({}, [FailedExpectation(Str), FailedToGetExitCode({ command : Str, err : IOErr }), StdoutErr(IOErr), ..])
expect_bytes! = |actual, expected|
    if actual == expected {
        Ok({})
    } else {
        fail("Expected ${Str.inspect(expected)}, got ${Str.inspect(actual)}")
    }

fail : Str -> Try({}, [FailedExpectation(Str), FailedToGetExitCode({ command : Str, err : IOErr }), StdoutErr(IOErr), ..])
fail = |message| Err(FailedExpectation(message))
