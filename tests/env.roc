app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Env
import pf.Path
import pf.Stdout

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
    actual_value = Env.var!(OsStr.from_str("BASIC_CLI_ENV_TEST"))?
    expect_bool!(OsStr.display(actual_value) == "hello-from-env-test", "Env.var! did not preserve the test value")?

    entries = Env.dict!()
    expect_bool!(
        contains_text_entry(entries, "BASIC_CLI_ENV_TEST", "hello-from-env-test"),
        "Env.dict! did not contain the ordinary native environment entry",
    )?

    current_platform = Env.platform!()
    match current_platform.arch {
        X86 | X64 | ARM | AARCH64 => {}
        OTHER(name) => fail!("Env.platform! returned unsupported test architecture ${name}")?
    }
    match current_platform.os {
        LINUX | MACOS | WINDOWS => {}
        OTHER(name) => fail!("Env.platform! returned unsupported test OS ${name}")?
    }

    match current_platform.os {
        LINUX | MACOS =>
            expect_bool!(
                contains_entry(
                    entries,
                    OsStr.unix("BASIC_CLI_NON_UTF8"),
                    OsStr.unix_bytes([255, 254]),
                ),
                "Env.dict! did not preserve the non-Unicode Unix value",
            )?
        WINDOWS => {}
        OTHER(_) => {}
    }

    original = Env.cwd!()?
    temporary = Env.temp_dir!()
    Env.set_cwd!(temporary)?
    changed = Env.cwd!()
    restore_result = Env.set_cwd!(original)
    restored = Env.cwd!()
    restore_result?

    changed_path = changed?
    restored_path = restored?
    expect_bool!(Path.to_os_str(changed_path) == Path.to_os_str(temporary), "Env.set_cwd! did not change cwd")?
    expect_bool!(Path.to_os_str(restored_path) == Path.to_os_str(original), "Env.set_cwd! did not restore cwd")?

    invalid = Env.exe_path!()?
    match Env.set_cwd!(invalid) {
        Err(InvalidCwd(_)) => {}
        Ok({}) => fail!("Env.set_cwd! unexpectedly accepted a missing directory")?
    }

    Stdout.line!("All tests passed.")?
    Ok({})
}

contains_entry = |entries, expected_name, expected_value|
    List.any(entries, |(name, value)| name == expected_name and value == expected_value)

contains_text_entry = |entries, expected_name, expected_value|
    List.any(entries, |(name, value)| OsStr.display(name) == expected_name and OsStr.display(value) == expected_value)

expect_bool! = |condition, message|
    if condition {
        Ok({})
    } else {
        fail!(message)
    }

fail! = |message| Err(FailedExpectation(message))
